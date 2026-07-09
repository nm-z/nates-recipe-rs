//! Column-type detector — inference half. Char-level model over a column's raw
//! byte stream → one of six `Kind`s. The architecture is fixed (embed→attn→dense
//! →dense) and its trained weights ship inline as `detector.ogdl`. Runs forward
//! through `recipe_infer` directly; the trainer that produced the weights lives
//! up in the framework crate.

pub const CONTEXT: usize = 256;
pub const VOCAB: usize = 257;
pub const N_CLASS: usize = 6;
pub const KIND_NUMERIC: usize = 0;
pub const KIND_TEMPORAL: usize = 1;
pub const KIND_CATEGORICAL: usize = 2;
pub const KIND_ORDINAL: usize = 3;
pub const KIND_TEXT: usize = 4;
pub const KIND_IMAGE: usize = 5;

pub const EMBED_DIM: usize = 32;
pub const HEADS: usize = 4;

const DETECTOR_OGDL: &str = include_str!("../detector.ogdl");

/// One column → one variable-length byte stream (every cell, newline-delimited),
/// read up to the context window, `id = byte + 1`, PAD(0) to `CONTEXT`.
/// No sampling, no per-cell windowing — the whole stream as far as the context reads.
pub fn tokenize_column(cells: &[&str]) -> Vec<f64> {
	let mut ids = Vec::with_capacity(CONTEXT);
	'outer: for (i, c) in cells.iter().enumerate() {
		if i > 0 {
			ids.push(b'\n' as f64 + 1.0);
			if ids.len() == CONTEXT {
				break;
			}
		}
		for &b in c.as_bytes() {
			ids.push(b as f64 + 1.0);
			if ids.len() == CONTEXT {
				break 'outer;
			}
		}
	}
	ids.resize(CONTEXT, 0.0);
	ids
}

/// Load-time column-type detection for the table path (CSV / dir / zip). Runs the
/// GPU char-level detector at `Data::set` time so the classification stays out of
/// the training run's measured init window; the encoder consumes the result instead
/// of calling `predict_kinds` mid-materialize. Returns, per table group, its name
/// and per-column `(header, kind int)` positional to the group's headers. Image
/// groups carry no feature columns and are omitted.
///
/// The detector sees byte-identical input to the in-encode path: per column, the
/// newline-joined stream of non-missing cells, `tokenize_column`-truncated to
/// `CONTEXT`. A plain CSV takes a streaming PREFIX read that stops as soon as every
/// column has ≥ `CONTEXT` bytes of that stream (or at EOF) — a column with fewer
/// forces reading to EOF, exactly what `tokenize_column` consumes from the full
/// parse. Dir/zip/db reuse the full `load_groups` loader (the same parser), taking
/// correctness over a second, prefix-only dialect.
pub fn detect_kinds(path: &str) -> anyhow::Result<crate::encode::PreKinds> {
	let p = std::path::Path::new(path);
	let ext = p.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
	let plain_csv = !p.is_dir() && !matches!(ext.as_deref(), Some("zip" | "db" | "sqlite"));
	if plain_csv {
		let (headers, cells) = prefix_columns(p)?;
		let non_empty: Vec<Vec<&str>> =
			cells.iter().map(|c| c.iter().map(String::as_str).collect()).collect();
		Ok(vec![(String::new(), kinds_for(&headers, &non_empty)?)])
	} else {
		crate::data::load_groups(path)
			.iter()
			.filter_map(|g| match g {
				crate::data::DirGroup::Table { name, headers, cells, .. } => {
					let non_empty: Vec<Vec<&str>> = (0..headers.len())
						.map(|j| {
							cells
								.iter()
								.map(|r| r.get(j).map_or("", String::as_str))
								.filter(|c| !crate::encode::is_missing(c))
								.collect()
						})
						.collect();
					Some(kinds_for(headers, &non_empty).map(|k| (name.clone(), k)))
				}
				crate::data::DirGroup::Image { .. } => None,
			})
			.collect()
	}
}

/// Non-empty columns → detector kinds; every empty column gets `KIND_NUMERIC` (the
/// branch the encoder takes for it regardless of any prediction). One `(header,
/// kind)` per column, in header order, so the encoder matches positionally and can
/// catch a count/name drift against its full parse.
fn kinds_for(headers: &[String], non_empty: &[Vec<&str>]) -> anyhow::Result<Vec<(String, usize)>> {
	let to_predict: Vec<usize> =
		(0..headers.len()).filter(|&j| !non_empty[j].is_empty()).collect();
	let cols: Vec<Vec<&str>> = to_predict.iter().map(|&j| non_empty[j].clone()).collect();
	let preds = predict_kinds(&cols)?;
	let mut pred: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
	for (i, &j) in to_predict.iter().enumerate() {
		pred.insert(j, preds[i]);
	}
	Ok(headers
		.iter()
		.enumerate()
		.map(|(j, name)| (name.clone(), pred.get(&j).copied().unwrap_or(KIND_NUMERIC)))
		.collect())
}

/// Streaming prefix read of a plain CSV: replicates `read_raw_csv`'s header
/// detection and missing-cell filtering, collecting each column's non-missing cells
/// only until it holds ≥ `CONTEXT` bytes of `tokenize_column` stream (or EOF). The
/// returned prefix is exactly what `tokenize_column` would consume from the full
/// column, so the `CONTEXT`-token vector — and thus the detection — is identical.
fn prefix_columns(path: &std::path::Path) -> anyhow::Result<(Vec<String>, Vec<Vec<String>>)> {
	// One token per byte, plus one '\n' separator between consecutive cells.
	fn take(j: usize, cell: &str, cols: &mut [Vec<String>], tok: &mut [usize], full: &mut usize) {
		if tok[j] >= CONTEXT {
			return;
		}
		tok[j] += if cols[j].is_empty() { cell.len() } else { 1 + cell.len() };
		cols[j].push(cell.to_string());
		if tok[j] >= CONTEXT {
			*full += 1;
		}
	}

	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(false)
		.flexible(true)
		.delimiter(crate::data::sniff_delimiter(path))
		.from_path(path)
		.map_err(|e| anyhow::anyhow!("detect_kinds: failed to open {}: {e}", path.display()))?;
	let mut records = rdr.byte_records();
	let Some(first) = records.next() else {
		return Ok((Vec::new(), Vec::new()));
	};
	let first =
		first.map_err(|e| anyhow::anyhow!("detect_kinds: first record of {}: {e}", path.display()))?;
	let first_cells: Vec<String> =
		first.iter().map(|s| String::from_utf8_lossy(s).into_owned()).collect();
	let w = first_cells.len();
	// Header row iff any first-row cell is a non-number (a header names columns);
	// an all-numeric first row is data, and columns are synthesized col_0..col_{w-1}.
	let headerless = !first_cells.is_empty()
		&& first_cells.iter().all(|c| {
			let t = c.trim();
			!t.is_empty() && t.parse::<f64>().is_ok()
		});
	let headers: Vec<String> = if headerless {
		(0..w).map(|j| format!("col_{j}")).collect()
	} else {
		first_cells.clone()
	};

	let mut cols: Vec<Vec<String>> = vec![Vec::new(); w];
	let mut tok = vec![0usize; w];
	let mut full = 0usize;
	if headerless {
		for (j, cell) in first_cells.iter().enumerate() {
			if !crate::encode::is_missing(cell) {
				take(j, cell, &mut cols, &mut tok, &mut full);
			}
		}
	}
	if full < w {
		for rec in records {
			let rec = rec
				.map_err(|e| anyhow::anyhow!("detect_kinds: record of {}: {e}", path.display()))?;
			for j in 0..w {
				if tok[j] >= CONTEXT {
					continue;
				}
				let cell = rec.get(j).map_or(std::borrow::Cow::Borrowed(""), String::from_utf8_lossy);
				if !crate::encode::is_missing(cell.as_ref()) {
					take(j, cell.as_ref(), &mut cols, &mut tok, &mut full);
				}
			}
			if full >= w {
				break;
			}
		}
	}
	Ok((headers, cols))
}

// Parks the detector's backing slab on every exit — normal return or a panic
// during the forward — so the arena is never left registered-and-live and the
// next call adopts it instead of reclaiming. No HIP calls in park, so drop
// drains nothing. `None` once the slab has already been parked/handed off.
struct ArenaGuard(Option<recipe_infer::GpuBuffer>);

impl Drop for ArenaGuard {
	fn drop(&mut self) {
		if let Some(slab) = self.0.take() {
			recipe_infer::park_run_backing(slab);
		}
	}
}

/// Each column's byte stream → argmax over the six kind logits. Builds the fixed
/// `embed(32,vocab=257) → attn(4) → dense(64,leaky) → dense(6,linear)` stack as
/// `recipe_infer::LayerSpec` values, loads the inline checkpoint into it, and runs
/// a single forward pass. The byte-id stream is the embed input, so no feature
/// scaling and no categorical side-input (`x_cat = None`).
pub fn predict_kinds(columns: &[Vec<&str>]) -> anyhow::Result<Vec<usize>> {
	if columns.is_empty() {
		return Ok(Vec::new());
	}
	let n = columns.len();
	let mut data = Vec::with_capacity(n * CONTEXT);
	for col in columns {
		data.extend(tokenize_column(col));
	}
	let x = ndarray::Array2::from_shape_vec((n, CONTEXT), data).expect("detect: shape");
	let specs = vec![
		recipe_infer::LayerSpec::Embed(EMBED_DIM, Some(VOCAB)),
		recipe_infer::LayerSpec::Attn(HEADS),
		recipe_infer::LayerSpec::Dense(64, recipe_infer::Activation::LeakyRelu),
		recipe_infer::LayerSpec::Dense(N_CLASS, recipe_infer::Activation::Linear),
	];
	// Uniform adopt→forward→park backing for the detector's forward. Weights
	// (resume-composed host image), the 12 scratch constants, and the tokenized
	// input compose ONE init image; build/upload/scratch all bump-carve from
	// one memset-committed slab — no per-buffer pool growth (the fresh-page
	// commit that faults ~30-50% of fresh-process loads).
	let saved = recipe_infer::load_ogdl_str(DETECTOR_OGDL)?;
	let plan = recipe_infer::plan_layer_params(&specs, CONTEXT, 0, VOCAB, &saved, true)
		.map_err(|e| anyhow::anyhow!("detect plan_layer_params: {e}"))?;
	let mut stage = recipe_infer::Stage::new();
	let w_off = stage.push(plan.host());
	let consts_off = stage.push(&recipe_infer::SCRATCH_CONSTS);
	let x_off = stage.push(x.as_slice().expect("detect: x contiguous"));
	let image = stage.into_host();
	let image_floats = image.len();
	// Detector claim (ONE drain): re-arm a parked training backing when present,
	// else claim a fresh arena — the composed image rides the adopt/claim H2D, no
	// standalone upload. Both refusing means nothing is parked and the card cannot
	// hold the footprint: fail clean with the numbers, no pool fallback.
	let est = recipe_infer::vram_estimate(&specs, n, CONTEXT, N_CLASS, VOCAB, 0, true);
	let need = est + est / 2 + (1 << 20);
	let slab = recipe_infer::adopt_run_backing_with_image(need, &image)
		.or_else(|| recipe_infer::claim_device_arena_with_image(&image))
		.ok_or_else(|| {
			anyhow::anyhow!(
				"detect: no device backing — footprint {}, claimable {}",
				recipe_infer::human_bytes(need),
				recipe_infer::human_bytes(recipe_infer::claimable_bytes()),
			)
		})?;
	let base = slab.view(0, image_floats);
	// Park-on-drop: on normal return the slab is parked for the next call to
	// adopt; on an unwind through the forward it is parked (not freed) too.
	let _arena = ArenaGuard(Some(slab));
	let params = plan.materialize(&base, w_off);
	let xbuf = base.view(x_off, n * CONTEXT);
	let consts_view = base.view(consts_off, 12);
	let sc = recipe_infer::Scratch::new_infer(&params, n, &consts_view)?;
	recipe_infer::forward_into(&params, &xbuf, None, n, &sc.acts, &sc)?;
	let last = params.len() - 1;
	// Detector release (ONE drain): enqueue the logits D2H (async, no wait), then
	// the single device_synchronize completes it; finish fans the pin into preds.
	// Scratch (all carves) drops after this drain, so its Drop drains nothing.
	let mut preds = vec![0.0f64; n * N_CLASS];
	let exit = unsafe { recipe_infer::exit_d2h_enqueue(sc.acts[last].ptr_raw(), n * N_CLASS * 8) }
		.map_err(|e| anyhow::anyhow!("detect exit d2h enqueue: {e:?}"))?;
	recipe_infer::device_synchronize().map_err(|e| anyhow::anyhow!("detect release sync: {e:?}"))?;
	exit.finish(&mut preds);
	Ok((0..n)
		.map(|r| {
			let lg = &preds[r * N_CLASS..r * N_CLASS + N_CLASS];
			let mut best = 0;
			for j in 1..N_CLASS {
				if lg[j] > lg[best] {
					best = j;
				}
			}
			best
		})
		.collect())
}

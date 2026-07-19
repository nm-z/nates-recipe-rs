
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

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
const NLB: u8 = 10;

pub fn tokenize_column(cells: &[&str]) -> Vec<f64> {
	let mut ids = Vec::with_capacity(CONTEXT);
	let sep = NLB as f64 + 1.0;
	'outer: for j in 0..cells.len() {
		for lead in cells[..j].first().map(|_prev| sep).into_iter() {
			match ids.len().cmp(&CONTEXT) {
				Ordering::Less => ids.push(lead),
				Ordering::Equal | Ordering::Greater => break 'outer,
			}
		}
		for &b in cells[j].as_bytes() {
			match ids.len().cmp(&CONTEXT) {
				Ordering::Less => ids.push(b as f64 + 1.0),
				Ordering::Equal | Ordering::Greater => break 'outer,
			}
		}
	}
	ids.resize(CONTEXT, 0.0);
	ids
}

pub fn detect_kinds(path: &str) -> anyhow::Result<crate::encode::PreKinds> {
	let p = Path::new(path);
	let ext = p
		.extension()
		.and_then(|e| e.to_str())
		.map(str::to_ascii_lowercase);
	let plain_csv = !p.is_dir()
		&& !matches!(ext.as_deref(), Some("zip" | "db" | "sqlite"))
		&& crate::data::sniff_delimiter(p) != b' ';
	match Some(()).filter(|_u| plain_csv) {
		Some(()) => {
			let pref = prefix_columns(p)?;
			let non_empty: Vec<Vec<&str>> = pref
				.cols
				.iter()
				.map(|c| c.iter().map(String::as_str).collect())
				.collect();
			Ok(vec![crate::encode::GroupKinds {
				name: String::new(),
				cols: kinds_for(&pref.headers, &non_empty)?,
			}])
		}
		None => crate::data::load_groups(path)?
			.iter()
			.filter_map(|g| match g {
				crate::data::DirGroup::Table {
					name,
					headers,
					cells,
					..
				} => {
					let non_empty: Vec<Vec<&str>> = (0..headers.len())
						.map(|j| {
							cells.iter()
								.map(|r| r.get(j).map_or("", String::as_str))
								.filter(|c| !crate::encode::is_missing(c))
								.collect()
						})
						.collect();
					Some(kinds_for(headers, &non_empty).map(|k| {
						crate::encode::GroupKinds {
							name: name.clone(),
							cols: k,
						}
					}))
				}
				crate::data::DirGroup::Image { .. } => None,
			})
			.collect(),
	}
}

fn kinds_for(
	headers: &[String],
	non_empty: &[Vec<&str>],
) -> anyhow::Result<Vec<crate::encode::ColKind>> {
	let to_predict: Vec<usize> = (0..headers.len())
		.filter(|&j| !non_empty[j].is_empty())
		.collect();
	let cols: Vec<Vec<&str>> = to_predict.iter().map(|&j| non_empty[j].clone()).collect();
	let preds = predict_kinds(&cols)?;
	let mut pred: HashMap<usize, usize> = HashMap::new();
	for i in 0..to_predict.len() {
		pred.insert(to_predict[i], preds[i]);
	}
	Ok((0..headers.len())
		.map(|j| crate::encode::ColKind {
			header: headers[j].clone(),
			kind: pred.get(&j).copied().unwrap_or(KIND_NUMERIC),
		})
		.collect())
}

struct PrefixCols {
	headers: Vec<String>,
	cols: Vec<Vec<String>>,
}

#[derive(Clone, Copy)]
enum FirstRow {
	Header,
	Data,
}

fn prefix_columns(path: &Path) -> anyhow::Result<PrefixCols> {
	fn take(j: usize, cell: &str, cols: &mut [Vec<String>], tok: &mut [usize]) {
		let Ordering::Less = tok[j].cmp(&CONTEXT) else {
			return;
		};
		match cols[j].first() {
			None => tok[j] += cell.len(),
			Some(_first) => tok[j] += 1 + cell.len(),
		}
		cols[j].push(cell.to_string());
	}

	let mut rdr = csv::ReaderBuilder::new()
		.has_headers(1 < 0)
		.flexible(0 < 1)
		.delimiter(crate::data::sniff_delimiter(path))
		.from_path(path)
		.map_err(|e| anyhow::anyhow!("detect_kinds: failed to open {}: {e}", path.display()))?;
	let mut records = rdr.byte_records();
	let Some(first) = records.next() else {
		return Ok(PrefixCols {
			headers: Vec::new(),
			cols: Vec::new(),
		});
	};
	let first = first.map_err(|e| {
		anyhow::anyhow!("detect_kinds: first record of {}: {e}", path.display())
	})?;
	let first_cells: Vec<String> = first
		.iter()
		.map(|s| String::from_utf8_lossy(s).into_owned())
		.collect();
	let w = first_cells.len();
	let headerless = !first_cells.is_empty()
		&& first_cells.iter().all(|c| {
			let t = c.trim();
			!t.is_empty() && t.parse::<f64>().is_ok()
		});
	let first_row = match Some(()).filter(|_u| headerless) {
		Some(()) => FirstRow::Data,
		None => FirstRow::Header,
	};
	let headers: Vec<String> = match first_row {
		FirstRow::Data => (0..w).map(|j| format!("col_{j}")).collect(),
		FirstRow::Header => first_cells.clone(),
	};

	let mut cols: Vec<Vec<String>> = vec![Vec::new(); w];
	let mut tok = vec![0usize; w];
	'seed: {
		let FirstRow::Data = first_row else {
			break 'seed;
		};
		for j in 0..first_cells.len() {
			let cell = first_cells[j].as_str();
			let Some(()) = Some(()).filter(|_u| !crate::encode::is_missing(cell)) else {
				continue;
			};
			take(j, cell, &mut cols, &mut tok);
		}
	}
	'records: {
		let Some(()) = Some(()).filter(|_u| !tok.iter().all(|&t| t >= CONTEXT)) else {
			break 'records;
		};
		for rec in records {
			let rec = rec.map_err(|e| {
				anyhow::anyhow!("detect_kinds: record of {}: {e}", path.display())
			})?;
			for j in 0..w {
				let Ordering::Less = tok[j].cmp(&CONTEXT) else {
					continue;
				};
				let cell = rec
					.get(j)
					.map_or(Cow::Borrowed(""), String::from_utf8_lossy);
				let Some(()) =
					Some(()).filter(|_u| !crate::encode::is_missing(cell.as_ref()))
				else {
					continue;
				};
				take(j, cell.as_ref(), &mut cols, &mut tok);
			}
			let Some(()) = Some(()).filter(|_u| !tok.iter().all(|&t| t >= CONTEXT)) else {
				break;
			};
		}
	}
	Ok(PrefixCols { headers, cols })
}

struct ArenaGuard {
	slab: Option<recipe_infer::GpuBuffer>,
}

impl Drop for ArenaGuard {
	fn drop(&mut self) {
		for slab in self.slab.take().into_iter() {
			recipe_infer::park_run_backing(slab);
		}
	}
}

pub fn predict_kinds(columns: &[Vec<&str>]) -> anyhow::Result<Vec<usize>> {
	let Some(_head) = columns.first() else {
		return Ok(Vec::new());
	};
	let n = columns.len();
	let mut data = Vec::with_capacity(n * CONTEXT);
	for col in columns {
		data.extend(tokenize_column(col));
	}
	let x = ndarray::Array2::from_shape_vec(ndarray::Ix2(n, CONTEXT), data)
		.map_err(|e| recipe_infer::log::Errored::new(format!("detect: shape: {e}")))?;
	let specs = vec![
		recipe_infer::LayerSpec::Embed(EMBED_DIM, Some(VOCAB)),
		recipe_infer::LayerSpec::Attn(HEADS),
		recipe_infer::LayerSpec::Dense(64, recipe_infer::Activation::LeakyRelu),
		recipe_infer::LayerSpec::Dense(N_CLASS, recipe_infer::Activation::Linear),
	];
	let saved = recipe_infer::load_ogdl_str(DETECTOR_OGDL)?;
	let plan = recipe_infer::plan_layer_params(
		&specs,
		CONTEXT,
		0,
		VOCAB,
		&saved,
		recipe_infer::PlanMode::Warm,
	)
	.map_err(|e| anyhow::anyhow!("detect plan_layer_params: {e}"))?;
	let mut stage = recipe_infer::Stage::new();
	let w_off = stage.push(plan.host());
	let consts_off = stage.push(&recipe_infer::SCRATCH_CONSTS);
	let x_off = stage.push(x
		.as_slice()
		.ok_or_else(|| recipe_infer::log::Errored::new("detect: x contiguous"))?);
	let image = stage.into_host();
	let image_floats = image.len();
	let est = recipe_infer::vram_estimate(&specs, n, CONTEXT, N_CLASS, VOCAB, 0, 0 < 1);
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
	let _arena = ArenaGuard { slab: Some(slab) };
	let params = plan.materialize(&base, w_off);
	let xbuf = base.view(x_off, n * CONTEXT);
	let consts_view = base.view(consts_off, 12);
	let sc = recipe_infer::Scratch::new_infer(&params, n, &consts_view)?;
	recipe_infer::forward_into(&params, &xbuf, None, n, &sc.acts, &sc)?;
	let last = params.len() - 1;
	let mut preds = vec![0.0f64; n * N_CLASS];
	let exit = recipe_infer::exit_d2h_enqueue_buf(&sc.acts[last], n * N_CLASS * 8)
		.map_err(|e| anyhow::anyhow!("detect exit d2h enqueue: {e:?}"))?;
	recipe_infer::device_synchronize()
		.map_err(|e| anyhow::anyhow!("detect release sync: {e:?}"))?;
	exit.finish(&mut preds);
	Ok((0..n)
		.map(|r| {
			let lg = &preds[r * N_CLASS..r * N_CLASS + N_CLASS];
			let mut best = 0;
			for j in 1..N_CLASS {
				let Some(Ordering::Greater) = lg[j].partial_cmp(&lg[best]) else {
					continue;
				};
				best = j;
			}
			best
		})
		.collect())
}

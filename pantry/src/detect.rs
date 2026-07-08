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
pub fn predict_kinds(columns: &[Vec<&str>]) -> Vec<usize> {
	if columns.is_empty() {
		return Vec::new();
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
	// input compose ONE staged image; build/upload/scratch all bump-carve from
	// one memset-committed slab — no per-buffer pool growth (the fresh-page
	// commit that faults ~30-50% of fresh-process loads).
	let saved = recipe_infer::load_ogdl_str(DETECTOR_OGDL);
	let plan = recipe_infer::plan_layer_params(&specs, CONTEXT, 0, VOCAB, &saved, true)
		.unwrap_or_else(|e| panic!("detect plan_layer_params: {e}"));
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
		.unwrap_or_else(|| {
			panic!(
				"detect: no device backing — footprint {}, claimable {}",
				recipe_infer::human_bytes(need),
				recipe_infer::human_bytes(recipe_infer::claimable_bytes()),
			)
		});
	let base = slab.view(0, image_floats);
	// Park-on-drop: on normal return the slab is parked for the next call to
	// adopt; on an unwind through the forward it is parked (not freed) too.
	let _arena = ArenaGuard(Some(slab));
	let params = plan.materialize(&base, w_off);
	let xbuf = base.view(x_off, n * CONTEXT);
	let consts_view = base.view(consts_off, 12);
	let sc = recipe_infer::Scratch::new_infer(&params, n, &consts_view);
	recipe_infer::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
	let last = params.len() - 1;
	// Detector release (ONE drain): enqueue the logits D2H (async, no wait), then
	// the single device_synchronize completes it; finish fans the pin into preds.
	// Scratch (all carves) drops after this drain, so its Drop drains nothing.
	let mut preds = vec![0.0f64; n * N_CLASS];
	let exit = unsafe { recipe_infer::exit_d2h_enqueue(sc.acts[last].ptr_raw(), n * N_CLASS * 8) }
		.expect("detect exit d2h enqueue");
	recipe_infer::device_synchronize().expect("detect release sync");
	exit.finish(&mut preds);
	(0..n)
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
		.collect()
}

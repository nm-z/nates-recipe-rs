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

// Releases the detector's one-claim slab on every exit (normal return or a
// panic in the GPU calls) so a fault can never leave the arena registered and
// wedge the next claim. `None` when nothing was claimed (an arena was already
// active, or the claim was refused) — Drop is then a no-op.
struct ArenaGuard(Option<recipe_infer::GpuBuffer>);

impl Drop for ArenaGuard {
	fn drop(&mut self) {
		if let Some(slab) = self.0.take() {
			recipe_infer::release_device_arena(slab);
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
	// One-claim arena for the detector's forward. Sized by the same estimator
	// fit uses (input + weights + scratch, forward-only) plus a margin, so
	// build/upload/scratch all bump-carve from one memset-committed slab instead
	// of growing the pool per buffer — the fresh-page memset-commit that faults
	// ~30-50% of fresh-process data loads. Only claim when no arena is already
	// active: a parked training backing already carves reliably, and a second
	// claim asserts. A refused claim (None) falls back to the pool path unchanged.
	let arena = (!recipe_infer::device_arena_active())
		.then(|| {
			let est = recipe_infer::vram_estimate(&specs, n, CONTEXT, N_CLASS, VOCAB, 0, true);
			recipe_infer::claim_device_arena_bytes(est + est / 2 + (1 << 20))
		})
		.flatten();
	let _arena = ArenaGuard(arena);
	let saved = recipe_infer::load_ogdl_str(DETECTOR_OGDL);
	let params = recipe_infer::build_layer_params(&specs, CONTEXT, 0, VOCAB, &saved, true)
		.unwrap_or_else(|e| panic!("detect build_layer_params: {e}"));
	let (xbuf, _, _) = recipe_infer::upload(&x);
	let sc = recipe_infer::Scratch::new(&params, n, true);
	recipe_infer::forward_into(&params, &xbuf, None, n, &sc.acts, &sc);
	let last = params.len() - 1;
	// preds copies to host here; params/xbuf/sc are non-owning carves, so the
	// guard's release (after this scope) frees them with the slab.
	let preds = recipe_infer::download_vec(&sc.acts[last], n * N_CLASS);
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

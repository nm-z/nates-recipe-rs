use crate::execute::ModelInner;
use pantry::encode::Dataset;
use recipe_infer::{LayerSpec, pinned_vocab, vram_estimate};

pub struct CatShape {
	cat_cols: usize,
	text_d: usize,
	vocab: usize,
}

pub fn plan_footprint(model: &ModelInner, ds: &Dataset) -> usize {
	let n = ds.x.nrows();
	let d = ds.x.ncols();
	let k = ds.n_targets.max(1);
	let embed_first = matches!(model.specs.first(), Some(LayerSpec::Embed(..)));
	let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
	let shape = Some(())
		.filter(|_probe| embed_cats)
		.map(|_probe| {
			let n_oh: usize = ds.onehot_groups.iter().map(|g| g.len).sum();
			CatShape {
				cat_cols: d - n_oh,
				text_d: ds.onehot_groups.len(),
				vocab: n_oh,
			}
		})
		.or_else(|| {
			Some(()).filter(|_probe| embed_first).map(|_probe| {
				let tc = ds.text_cols.len();
				let vocab = pinned_vocab(&model.specs).unwrap_or_else(|| {
					ds.x.iter().cloned().fold(0.0f64, f64::max) as usize + 1
				});
				CatShape {
					cat_cols: d - tc,
					text_d: tc,
					vocab,
				}
			})
		})
		.unwrap_or(CatShape {
			cat_cols: 0,
			text_d: d,
			vocab: 0,
		});
	vram_estimate(
		&model.specs,
		n,
		shape.text_d,
		k,
		shape.vocab,
		shape.cat_cols,
		false,
	)
}

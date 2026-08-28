//! Tile RAT: learn contraction extents for this GPU from measured trials.

use recipe::*;

/// The caller owns the distribution. Recipe never sees its limits or its size.
fn trial() -> [u32; 3] {
	let draw = |shift: u32, choices: [u32; 4]| {
		let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
		choices[(nanos.rotate_right(shift) % 4) as usize]
	};
	[draw(0, [128, 256, 384, 512]), draw(7, [32, 48, 64, 96]), draw(13, [16, 32, 48, 64])]
}

/// The caller owns invalid-tile policy. A penalty must sit farther from zero
/// time than any real measurement, so an unusable extent is never preferred.
fn benchmark(workload: [u32; 3], extent: [u32; 3]) -> f64 {
	measure_contraction(workload, extent).unwrap_or(1.0)
}

fn main() {
	let trials = recipe.data(trial);

	let bench = recipe.model().layer(24).tanh().loss(huber);

	let tiles = recipe.model().layer(24).tanh().loss(&bench);

	recipe.train().lr(0.001).epochs(100).seed(17).log(all).rat(benchmark).run(&tiles, &trials);
}

use recipe::*;
#[rustfmt::skip]
fn trial() -> [u32; 3] {
	let draw = |shift: u32, choices: [u32; 4]| {
		let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
		choices[(nanos.rotate_right(shift) % 4) as usize]
	};
	[draw(0, [128, 256, 384, 512]), draw(7, [32, 48, 64, 96]), draw(13, [16, 32, 48, 64])]
}
fn benchmark(workload_mnk: [f64; 3], tile_mnk: [f64; 3]) -> f64 {
	let integers = |values: [f64; 3]| values.map(|value| value as u32);
	match measure_contraction(integers(workload_mnk), integers(tile_mnk)) {
		Some(time) => time,
		None => 999_999_999_999.0,
	}
}
#[rustfmt::skip]
fn main() {
	let trials = recipe.data(trial);

	let bench = recipe.model()
		.layer(24).tanh()
		.loss(huber);

	let tiles = recipe.model()
		.layer(24).tanh().loss(&bench);

	recipe.train()
		.lr(0.001)
		.epochs(100)
		.seed(17)
		.log(all)
		.rat(benchmark)
		.run(&tiles, &trials);
}

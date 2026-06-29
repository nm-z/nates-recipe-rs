// Profiling harness for the three GPU fixes. Reproduces the cookbook's Churn
// scenario (42→64→1, N≈475K) — the exact shape behind the before-baseline:
// zscore variance (Fix 1), backward dW 42×64 + out_dim==1 gemv (Fix 2), and the
// per-epoch metric scalar copies (Fix 3). Bounded epochs so it exits cleanly for
// rocprofv3; the per-kernel grid/occupancy/time are shape-intrinsic, so they
// compare directly against engi/856369_results.db (before).
use recipe::*;

fn main() {
	let nn = Model::new().loss(ce).layer(64).leak().layer(2).lr(0.001);
	let data = Data::load()
		.set("datasets/playground-series-s6e3/train.csv")
		.split(0.8)
		.exclude("id")
		.target("Churn");
	let ep: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(50);
	let train = Train::new().epochs(ep).log([Loss, Accuracy]);
	train.run(&nn, &data);
}

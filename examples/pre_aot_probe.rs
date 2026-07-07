#!/usr/bin/env -S cargo run --release --example pre_aot_probe --
// pre-AOT staged-fit verification (dedicated driver — NOT the cookbook showcase).
// Scenario 1 is byte-for-byte the same config as the known-good staged run, so its
// loss/R² trajectory is the parity oracle. Scenario 2 adds Accuracy to a regression
// model — before the wider ring that forced the pooled path; it must now stay
// staged (ledger init 1 / loop 0 / exit 1) with the same loss/R² plus an accuracy
// column. No .net(), single process.
use recipe::*;

fn hp() -> Data {
	Data::load()
		.set("datasets/house-prices/train.csv")
		.split(0.8)
		.exclude("Id")
		.target("SalePrice")
}

fn main() {
	// Scenario 1 — parity: staged [Loss, R2, hip]. Expect ledger init 1 / loop 0 / exit 1.
	let m1 = Model::new()
		.loss(mse)
		.layer(64)
		.leak()
		.layer(1)
		.lr(0.0001);
	let t1 = Train::new()
		.epochs(20)
		.log([Loss,R2,hip,]);
	eprintln!("\n===== S1 staged [Loss, R2, hip] — parity oracle, expect ledger 1/0/1 =====");
	t1.run((&m1, &hp()));

	// Scenario 2 — coverage: Accuracy on a regression model needs the wider ring.
	// Must be STAGED: same loss/R2 as S1 plus an accuracy column, ledger 1/0/1.
	let m2 = Model::new()
		.loss(mse)
		.layer(64)
		.leak()
		.layer(1)
		.lr(0.0001);
	let t2 = Train::new()
		.epochs(20)
		.log([Loss,R2,Accuracy,hip,]);
	eprintln!("\n===== S2 staged [Loss, R2, Accuracy, hip] — must be staged, ledger 1/0/1 =====");
	t2.run((&m2, &hp()));

	// Scenario 3 — rerun continuity: the SAME model run twice. The second run has
	// no fresh init (its params are live) — it warm-starts from the host weight
	// mirror the first staged exit left, so its loss CONTINUES the first run's
	// (does not reset to the fresh-init value). Both runs must stay staged (1/0/1).
	let m3 = Model::new()
		.loss(mse)
		.layer(64)
		.leak()
		.layer(1)
		.lr(0.0001);
	let d3 = hp();
	eprintln!("\n===== S3a first fit (fresh host-randn init) — expect ledger 1/0/1 =====");
	Train::new()
		.epochs(20)
		.log([Loss, R2, hip])
		.run((&m3, &d3));
	eprintln!("\n===== S3b RERUN (warm-start from run-1 mirror) — loss must continue, ledger 1/0/1 =====");
	Train::new()
		.epochs(20)
		.log([Loss, R2, hip])
		.run((&m3, &d3));
	eprintln!("\n===== PROBE COMPLETE =====");
}

//! The workload the RAT tunes: fill percentage from a VNA sweep.
//!
//! Each sample directory holds one `scan.csv` (1200 sweep points, 8 columns)
//! and one `targets.csv` (a single row), which is exactly Recipe's capture
//! layout, so the directory loads without any preparation.
//!
//! `VNA_DATA` overrides the data root. `VNA_EPOCHS` overrides the epoch count,
//! which is how the RAT drives this same model at a measurement length and at a
//! longer numerical-check length.

use recipe::*;

fn main() {
	let root = std::env::var("VNA_DATA").unwrap_or_else(|_| "/home/nate/Desktop/vna-temp-fill-data-v2".to_owned());
	let epochs = std::env::var("VNA_EPOCHS").map_or(200, |value| value.parse().expect("VNA_EPOCHS must be a number"));

	let data = recipe.data(root.as_str()).target(["fill_percent"]).norm(z_score).split(0.8);

	let model = recipe.model().conv(16, 5).pool(64).gelu().layer(64).gelu().layer(1).loss(mae);

	let report = recipe.train().fp(32).lr(0.001).seed(17).epochs(epochs).run(&model, &data);

	// The RAT reads these four lines. The digest is over prediction bit
	// patterns, so a schedule that changes the trained model changes it, which
	// a scalar loss at a short epoch count does not reliably do.
	let mut digest = 0xcbf29ce484222325_u64;
	for value in report.predictions().iter().copied() {
		for byte in value.to_bits().to_le_bytes() {
			digest = (digest ^ u64::from(byte)).wrapping_mul(0x100000001b3);
		}
	}
	println!("epoch_seconds {:.9}", report.epoch_seconds());
	println!("final_loss_bits {:#018x}", report.final_loss().to_bits());
	println!("prediction_digest {digest:#018x}");
	println!("tile {:?}", report.tile());
}

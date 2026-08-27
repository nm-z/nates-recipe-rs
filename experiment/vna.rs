#![cfg_attr(rustfmt, rustfmt::skip)]
//! Fill percentage from a VNA sweep.
//! Each sample directory holds one scan table and one target row
//!

use recipe::*;

fn main() {
	let data = recipe.data("~/Desktop/vna-temp-fill-data-v2").target(["fill_percent"]).norm(z_score).split(0.8);

	let model = recipe.model().conv(16, 5).pool(64).gelu().layer(64).gelu().layer(1).loss(mae);

	recipe.train().fp(32).lr(0.001).epochs(200).save("vna.ogdl").run(&model, &data);
}

//! Semantic-model save followed by existence-conditional resume.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_save_resume`.

use recipe::*;

const DATASET: &str = "examples/datasets/cookbook/binary.csv";
const FIRST_MODEL: &str = "cookbook-save-resume-first.ogdl";
const SECOND_MODEL: &str = "cookbook-save-resume-second.ogdl";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.save(FIRST_MODEL)
		.run()?;

	recipe.data(DATASET)
		.target("target")
		.norm(z_score)
		.split(0.75);
	recipe.model().layer(8).silu().layer(1).loss(bce);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.0002)
		.resume(FIRST_MODEL)
		.save(SECOND_MODEL)
		.run()?;
	Ok(())
}

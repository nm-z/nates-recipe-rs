use recipe::*;

const DATASET: &str = "examples/datasets/no-show-appointments/KaggleV2-May-2016.csv";

fn main() -> TrainingResult<()> {
	recipe.data(DATASET)
		.target("No-show")
		.exclude(["AppointmentID", "PatientId"])
		.exclude(cond!(Age < 0))
		.norm(z_score)
		.split(0.8);

	recipe.model()
		.layer(128)
		.silu()
		.layer(128)
		.silu()
		.layer(1)
		.loss(bce)
		.grad(clip(1.0));

	recipe.train()
		.optimizer(adamw)
		.epochs(100)
		.lr(0.0001)
		.warmup(5)
		.cos()
		.log([Loss, AuRoc, AuPrc, Brier, CalibrationError])
		.save("model.ogdl")
		.run()?;
	Ok(())
}

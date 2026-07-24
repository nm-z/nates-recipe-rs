use recipe::*;

const DATASET: &str = "examples/datasets/no-show-appointments/KaggleV2-May-2016.csv";

fn main() {
	let data = recipe
		.data(DATASET)
		.target("No-show")
		.exclude(["AppointmentID", "PatientId"])
		.exclude(cond!(Age < 0))
		.split(0.8);

	let model = recipe
		.model()
		.norm(z_score)
		.layer(128)
		.silu()
		.layer(128)
		.silu()
		.layer(1)
		.loss(bce)
		.optimizer(adamw);

	recipe.train()
		.batch_size(2048)
		.epochs(100)
		.lr(0.0001)
		.warmup(5)
		.cosine_decay()
		.gradient_clip(1.0)
		.early_stop(AuPrc, 10)
		.calibrate(TemperatureScaling)
		.log([
			Loss,
			AuRoc,
			AuPrc,
			Brier,
			CalibrationError,
			RecallAt(0.10),
			RecallAt(0.20),
			RecallAt(0.30),
		])
		.run(&data, &model)
		.expect("training failed")
		.save(())
		.expect("saving model.ogdl failed");
}

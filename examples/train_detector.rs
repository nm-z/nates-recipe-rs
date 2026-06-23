use nates_recipe::*;

fn main() {
	// 40% train / 60% test over all hand-labelled real columns.
	let (train, test) = nates_recipe::detect::corpus_split(0xC0FFEE, 0.6);
	eprintln!("split: {} train / {} test columns", train.x.nrows(), test.x.nrows());
	let model = nates_recipe::detect::model();
	let trainer = Train::new().epochs(5000).resume_from("pantry/detector.ogdl").log([Epoch, Loss, Accuracy]);
	trainer.run(&model, &train);
	trainer.save_as([w, b], "pantry/detector.ogdl");
	eprintln!("=== held-out test set (60%) — datatype-detection accuracy ===");
	Train::new().log([Accuracy]).run(&model, &Some(test));
}

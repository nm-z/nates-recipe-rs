// E2E for the image join-key abstraction: CSV column of filenames ⋈ dir of images,
// indexed by filename. Mirrors the cookbook CNN scenario, few epochs, foreground.
use recipe::*;

#[allow(unused_variables)] // cnn/data resolved via the live registry, not textually read
fn main() {
	let cnn = Model::new()
		.loss(ce)
		.conv(32, 3, 1)
		.leak()
		.conv(64, 3, 1)
		.leak()
		.layer(128)
		.leak()
		.layer(36)
		.lr(0.001);
	let data = Data::load()
		.set("datasets/predict-the-handwriting-images/train.csv")
		.set("datasets/predict-the-handwriting-images/train_images/")
		.split(0.8)
		.target("label");
	let train = Train::new().epochs(20).log([Loss, Accuracy]);
	train.run(());
}

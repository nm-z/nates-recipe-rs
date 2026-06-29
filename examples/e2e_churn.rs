// E2E for the binary categorical target: cookbook nn config verbatim — bce loss,
// layer(1).sigmoid() on the binary "Churn" target (now a single class-index column).
use recipe::*;

#[allow(unused_variables)] // nn/data resolved via the live registry, not textually read
fn main() {
	let nn = Model::new()
		.loss(bce)
		.layer(64)
		.leak()
		.layer(1)
		.sigmoid()
		.lr(0.001);
	let data = Data::load()
		.set("datasets/playground-series-s6e3/train.csv")
		.split(0.8)
		.exclude("id")
		.target("Churn");
	let train = Train::new().epochs(20).log([Loss, Accuracy]);
	train.run(());
}

use recipe::*;

const HP: &str = "/home/nate/Desktop/nates-recipe-rs/datasets/house-prices/train.csv";
const save: &str = "model.ogdl";

fn main() {
	let d = Data::load(HP)
		.split(0.8)
		.exclude("Id")
		.target("SalePrice");
	let m = Model::new()
		.layer(64)
		.relu()
		.layer(1)
		.loss(mse)
		.lr(0.1);
	Train::new()
		.epochs(200000)
		.log([Loss, R2, Acc])
		//.resume(save)
		.run(&d, &m)
		.save(save);
}








let bench = recipe.model()
	.loss(mse)
	.layer(24)
	.leak()
	.layer(1)
	.lr(0.001);

let tile = recipe.model()
	.loss(&bench)
	.layer(32)
	.leak()
	.layer(3)
	.lr(0.001);

























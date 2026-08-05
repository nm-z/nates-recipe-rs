use recipe::*;

fn main() {
	let data = recipe
		.data(
			"/home/nate/Desktop/nates-recipe-rs/examples/datasets/uci-bank-semicolon/bank-full.csv",
		)
		.target("y")
		.norm(z_score)
		.split(0.6);
	let model = recipe
		.model()
		.conv(8, 3)
		.gelu()
		.norm(batch)
		.lstm(8)
		.silu()
		.norm(layer)
		.layer(1)
		.loss(focal);
	recipe.train()
		.optimizer(adamw)
		.epochs(1)
		.lr(0.01)
		.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
		.run(&model, &data);
}

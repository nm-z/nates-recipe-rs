//! Copyable one-epoch Recipe training examples.
//!
//! Every example is deliberately written out as the same three declarations:
//! define `data`, define `model`, then compose and execute `train` as one
//! expression. There are no cookbook-only helpers or abstractions to carry into
//! copied code.
//!
//! Native execution requires a current measured profile:
//!
//! ```text
//! cargo run --bin recipe -- probe
//! cargo run --example cookbook
//! ```

use recipe::*;

const SONAR: &str = "examples/datasets/uci-sonar/sonar.all-data";
const IONOSPHERE: &str = "examples/datasets/uci-ionosphere/ionosphere.data";
const WDBC: &str = "examples/datasets/uci-wdbc/wdbc.data";
const SPAMBASE: &str = "examples/datasets/uci-spambase/spambase.data";
const MAGIC: &str = "examples/datasets/uci-magic/magic04.data";
const BANK: &str = "examples/datasets/uci-bank-semicolon/bank.csv";
const AIRFOIL: &str = "examples/datasets/uci-airfoil/airfoil_self_noise.dat";
const ABALONE: &str = "examples/datasets/uci-abalone/abalone.data";
const WINE_QUALITY: &str = "examples/datasets/uci-winequality-semicolon/winequality-red.csv";

fn main() {
	recipe.data(SONAR)
		.target("col61")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.batch(0.5)
		.epochs(1)
		.lr(0.0003).cos()
		.log([Loss, AuPrc])
		.run()
		.save("cookbook-sonar-logistic.ogdl");

	recipe.data(IONOSPHERE)
		.target("col35")
		.norm(min_max)
		.split(0.8);
	recipe.model()
		.layer(32).norm(layer_norm).silu()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.batch(0.25)
		.epochs(1)
		.lr(0.0002).cos()
		.log(Loss)
		.run()
		.save("cookbook-ionosphere-shallow.ogdl");

	recipe.data(WDBC)
		.target("col3")
		.exclude("col1")
		.norm(l2_norm)
		.split(0.8);
	recipe.model()
		.layer(64).silu().norm(layer_norm)
		.layer(8).gelu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.batch(0.2)
		.epochs(1)
		.lr(0.0002).cos()
		.log([Loss])
		.run()
		.save("cookbook-wdbc-regression.ogdl");

	recipe.data(SPAMBASE)
		.target("col58")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.loss(huber)
		.layer(256).norm(batch_norm).silu()
		.layer(128).silu()
		.layer(32).huber()
		.layer(1);
	recipe.train()
		.optimizer(adamw)
		.batch(0.125)
		.epochs(1)
		.lr(0.0001).cos()
		.log(Loss)
		.run()
		.save("cookbook-spambase-pyramid.ogdl");

	recipe.data(MAGIC)
		.target("col11")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(256).gelu()
		.layer(64).relu()
		.layer(1)
		.loss(bce);
	recipe.train()
		.optimizer(adamw)
		.batch(0.1)
		.epochs(1)
		.lr(0.0001).cos()
		.log([Loss, AuPrc])
		.run()
		.save("cookbook-magic-wide.ogdl");

	recipe.data(BANK)
		.target("y")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(32).relu()
		.layer(128).silu()
		.layer(32).log()
		.layer(1)
		.loss(bce).grad(clip(1.0));
	recipe.train()
		.optimizer(adamw)
		.batch(0.1)
		.epochs(1)
		.lr(0.0001).cos()
		.log(Loss)
		.run()
		.save("cookbook-bank-hourglass.ogdl");

	recipe.data(AIRFOIL)
		.target("col6")
		.norm(min_max)
		.split(0.8);
	recipe.model()
		.perc(24).silu()
		.layer(12).huber()
		.layer(1)
		.loss(huber);
	recipe.train()
		.optimizer(adamw)
		.batch(0.2)
		.epochs(1)
		.lr(0.0001).cos()
		.log(Loss)
		.run()
		.save("cookbook-airfoil-perceptron.ogdl");

	recipe.data(ABALONE)
		.target("col9")
		.norm(l2_norm)
		.split(0.8);
	recipe.model()
		.layer(48).cos()
		.layer(12).log()
		.layer(1)
		.loss(mae);
	recipe.train()
		.optimizer(adamw)
		.batch(0.2)
		.epochs(1)
		.lr(0.0002).exp()
		.log([Loss])
		.run()
		.save("cookbook-abalone-regression.ogdl");

	recipe.data(WINE_QUALITY)
		.target("quality")
		.norm(z_score)
		.split(0.8);
	recipe.model()
		.layer(64).gelu().norm(batch_norm)
		.layer(16).silu()
		.layer(1)
		.loss(mse);
	recipe.train()
		.optimizer(adamw)
		.batch(0.25)
		.epochs(1)
		.lr(0.0002).exp()
		.log(Loss)
		.run()
		.save("cookbook-wine-quality-regression.ogdl");
}

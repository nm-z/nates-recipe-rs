use recipe::*;

const D: &str = "/home/nate/Desktop/nates-recipe-rs/examples/datasets";

fn main() {
	let datasets: &[(&str, &[&str])] = &[
		("VNA", &["9_10_24_Hold_02_targets.col1"]),
		("rogii-wellbore-geology-prediction", &["TVT"]),
		("ARC-AGI-2", &["output"]),
		("sandp500", &["close"]),
		("uci-shuttle", &["col10"]),
		("uci-sms-tab", &["col1"]),
		("uci-bank-semicolon", &["y"]),
		("uci-winequality-semicolon", &["quality"]),
		("uci-yeast", &["col10"]),
		("uci-seeds", &["col8"]),
		("uci-ecoli", &["col9"]),
		("uci-airfoil", &["col6"]),
		("uci-adult", &["col15"]),
		("uci-spambase", &["col58"]),
		("uci-satimage", &["col37"]),
		("uci-german-numeric", &["col25"]),
		("uci-wdbc", &["col2"]),
		("uci-optdigits", &["col65"]),
		("uci-magic", &["col11"]),
		("uci-letter", &["col1"]),
		("uci-wine", &["col1"]),
		("uci-sonar", &["col61"]),
		("uci-ionosphere", &["col35"]),
		("uci-glass", &["col11"]),
		("uci-bcw", &["col11"]),
		("uci-abalone", &["col9"]),
		("lendingclub-2007-2011", &["loan_status"]),
		("no-show-appointments", &["No-show"]),
		("wine-quality", &["quality"]),
		("house-prices", &["SalePrice"]),
		("web-traffic-time-series-forecasting", &["Visits"]),
		("march-machine-learning-mania-2026", &["Pred"]),
		("predict-the-handwriting-images", &["label"]),
		("llm-classification-finetuning", &["winner_model_a", "winner_model_b", "winner_tie"]),
		("playground-series-s6e3", &["Churn"]),
	];

	let blocks: &[fn(Model) -> Model] = &[
		|m| m.layer(64),
		|m| m.layer(8).residual([layer(8), relu()]),
		|m| m.conv(8, 3).pool(64),
		|m| m.attn(1),
		|m| m.embed(4, 8).pool(64),
		|m| m.rnn(8),
		|m| m.gru(8),
		|m| m.lstm(8),
		|m| m.perc(24),
		|m| m.kmeans(4),
		|m| m.knn(5),
	];
	let activations: &[fn(Model) -> Model] = &[
		|m| m,
		|m| m.relu(),
		|m| m.gelu(),
		|m| m.silu(),
		|m| m.tanh(),
		|m| m.sigmoid(),
		|m| m.elu(),
		|m| m.selu(),
		|m| m.prelu(),
		|m| m.leak(),
		|m| m.exp(),
		|m| m.ln(),
		|m| m.log(),
		|m| m.cos(),
		|m| m.tan(),
		|m| m.huber(),
	];
	let norms: &[Option<Norm>] = &[None, Some(batch), Some(layer)];
	let losses: &[Loss] = &[mse, mae, huber, bce, ce, focal, rmse];

	for &(name, targets) in datasets {
		let data = recipe.data(format!("{D}/{name}")).target(targets).norm(z_score).split(0.6);
		for block in blocks {
			for activation in activations {
				for normalization in norms {
					for loss in losses {
						let m = activation(block(recipe.model()));
						let m = match normalization {
							Some(n) => m.norm(*n),
							None => m,
						};
						recipe.train()
							.epochs(1)
							.lr(0.01)
							.log([Time, Run, Epoch, R2, Loss, blck, atvn, norm])
							.run(&m.layer(1).loss(*loss), &data);
					}
				}
			}
		}
	}
}

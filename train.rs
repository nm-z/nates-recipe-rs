use recipe::*;

fn main() {
	let data = recipe
		.data("/home/nate/Desktop/D7-data")
		.target("temper1")
		.exclude(["Frequency(Hz)", "sample", "time", "scan", "temperx", "am1", "am2", "max6675", "minivna", "am1rh", "am2rh"])
		.norm(z_score)
		.split(0.2);
	let blocks: &[(&str, fn(Model) -> Model)] = &[
		("layer", |m| m.layer(64)),
		("residual", |m| m.layer(8).residual([layer(8), relu()])),
		("conv", |m| m.conv(8, 3).pool(64)),
		("attn", |m| m.attn(1)),
		("embed", |m| m.embed(4, 8).pool(64)),
		("rnn", |m| m.rnn(8)),
		("gru", |m| m.gru(8)),
		("lstm", |m| m.lstm(8)),
		("perc", |m| m.perc(24)),
		("moe", |m| m.moe(2, [layer(8), layer(8), layer(8), layer(8)])),
		("svm", |m| m.svm([tanh, relu, gelu, sigmoid])),
		("kmeans", |m| m.kmeans(4)),
		("knn", |m| m.knn(5)),
	];
	let activations: &[(&str, fn(Model) -> Model)] = &[
		("linear", |m| m), ("relu", |m| m.relu()), ("gelu", |m| m.gelu()), ("silu", |m| m.silu()),
		("tanh", |m| m.tanh()), ("sigmoid", |m| m.sigmoid()), ("elu", |m| m.elu()), ("selu", |m| m.selu()),
		("prelu", |m| m.prelu()), ("leak", |m| m.leak()), ("exp", |m| m.exp()), ("ln", |m| m.ln()),
		("log", |m| m.log()), ("cos", |m| m.cos()), ("tan", |m| m.tan()), ("huber", |m| m.huber()),
	];
	let normalizations: &[(&str, Option<Norm>)] = &[("none", None), ("batch", Some(batch)), ("layer", Some(layer))];
	let losses: &[(&str, Loss)] = &[
		("mse", mse), ("rmse", rmse), ("huber", huber), ("mae", mae), ("bce", bce), ("ce", ce), ("focal", focal),
	];
	let (mut pass, mut fail) = (0_usize, 0_usize);
	for (bn, blk) in blocks {
		for (an, act) in activations {
			for (nn, nrm) in normalizations {
				for (ln, los) in losses {
					let model = act(blk(recipe.model()));
					let model = match nrm { Some(n) => model.norm(*n), None => model };
					let model = model.layer(1).loss(*los);
					let label = format!("{bn}/{an}/{nn}/{ln}");
					let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
						recipe.train().seed(1).epochs(5).lr(0.01).log([Epoch, R2, blck, atvn, norm]).run(&model, &data)
					}));
					match result {
						Ok(report) => { pass += 1; eprintln!("{label}: R2 {:.4}", report.r2()) }
						Err(_) => { fail += 1; eprintln!("{label}: FAILED") }
					}
				}
			}
		}
	}
	eprintln!("{pass} passed, {fail} failed out of {}", pass + fail);
}

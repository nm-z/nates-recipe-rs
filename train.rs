use recipe::*;

fn main() {
	let ds = "/home/nate/Desktop/nates-recipe-rs/examples/datasets";
	let mut datasets: Vec<(String, Data)> = vec![
		("D7".into(), recipe.data("/home/nate/Desktop/D7-data").target("temper1")
			.exclude(["Frequency(Hz)", "sample", "time", "scan", "temperx", "am1", "am2", "max6675", "minivna", "am1rh", "am2rh"])
			.norm(z_score).split(0.2)),
		("VNA".into(), recipe.data("/home/nate/Desktop/odroid-vna-data").target("temperature")
			.exclude(["Frequency(Hz)"]).norm(z_score).split(0.2)),
		("MAX".into(), recipe.data("/home/nate/Desktop/MAX6675-live-20").target("max6675")
			.exclude(["Frequency(Hz)", "sample", "time", "scan"]).norm(z_score).split(0.2)),
	];
	let mut dirs: Vec<_> = std::fs::read_dir(ds).expect("cannot read datasets dir")
		.filter_map(|e| e.ok()).filter(|e| e.path().is_dir())
		.map(|e| e.file_name().to_string_lossy().to_string()).collect();
	dirs.sort();
	for name in dirs { datasets.push((name.clone(), recipe.data(format!("{ds}/{name}")).norm(z_score).split(0.2))) }
	let blocks: &[(&str, fn(Model) -> Model)] = &[
		("layer", |m| m.layer(8)), ("residual", |m| m.layer(8).residual([layer(8), relu()])),
		("conv", |m| m.conv(4, 3).pool(8)), ("attn", |m| m.attn(1)),
		("embed", |m| m.embed(4, 8).pool(8)), ("rnn", |m| m.rnn(4)),
		("gru", |m| m.gru(4)), ("lstm", |m| m.lstm(4)), ("perc", |m| m.perc(8)),
		("moe", |m| m.moe(2, [layer(4), layer(4), layer(4), layer(4)])),
		("svm", |m| m.svm([tanh, relu, gelu, sigmoid])),
		("kmeans", |m| m.kmeans(4)), ("knn", |m| m.knn(5)),
	];
	let activations: &[(&str, fn(Model) -> Model)] = &[
		("linear", |m| m), ("relu", |m| m.relu()), ("gelu", |m| m.gelu()), ("silu", |m| m.silu()),
		("tanh", |m| m.tanh()), ("sigmoid", |m| m.sigmoid()), ("elu", |m| m.elu()), ("selu", |m| m.selu()),
		("prelu", |m| m.prelu()), ("leak", |m| m.leak()), ("exp", |m| m.exp()), ("ln", |m| m.ln()),
		("log", |m| m.log()), ("cos", |m| m.cos()), ("tan", |m| m.tan()), ("huber", |m| m.huber()),
	];
	let norms: &[(&str, Option<Norm>)] = &[("none", None), ("batch", Some(batch)), ("layer", Some(layer))];
	let losses: &[(&str, Loss)] = &[
		("mse", mse), ("rmse", rmse), ("huber", huber), ("mae", mae), ("bce", bce), ("ce", ce), ("focal", focal),
	];
	let (mut pass, mut fail) = (0_usize, 0_usize);
	for (dn, data) in &datasets { for (bn, blk) in blocks { for (an, act) in activations {
		for (nn, nrm) in norms { for (ln, los) in losses {
			let model = act(blk(recipe.model()));
			let model = match nrm { Some(n) => model.norm(*n), None => model };
			let model = model.layer(1).loss(*los);
			let label = format!("{dn}/{bn}/{an}/{nn}/{ln}");
			match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
				recipe.train().seed(1).epochs(5).lr(0.01).run(&model, data)
			})) {
				Ok(r) => { pass += 1; eprintln!("{label}: R2 {:.4}", r.r2()) }
				Err(_) => { fail += 1; eprintln!("{label}: FAILED") }
			}
		} } }
	} }
	eprintln!("{pass} passed, {fail} failed out of {}", pass + fail);
}

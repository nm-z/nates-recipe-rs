use lightgbm_rs::{Params, predict_proba, train_multiclass};
use ogdl::log::{Errored, Opt, Write, acc, data, epoch, set_opt, time};
use std::time::Instant;

fn load_csv(path: &str) -> Result<(Vec<f64>, Vec<usize>, usize, usize), Errored> {
	let text = std::fs::read_to_string(path).map_err(|e| Errored::new(format!("read_to_string {path}: {e}")))?;
	let mut x = Vec::new();
	let mut y = Vec::new();
	let mut p = 0;
	for line in text.lines() {
		if line.is_empty() {
			continue;
		}
		let vals: Vec<&str> = line.split(',').collect();
		p = vals.len() - 1;
		for v in &vals[..p] {
			x.push(v
				.parse::<f64>()
				.map_err(|e| Errored::new(format!("parse f64 '{v}': {e}")))?);
		}
		y.push(vals[p]
			.parse::<usize>()
			.map_err(|e| Errored::new(format!("parse usize '{}': {e}", vals[p])))?
			- 1);
	}
	let n = y.len();
	Ok((x, y, n, p))
}

fn main() -> Result<(), Errored> {
	set_opt(Opt {
		data: true,
		epoch: true,
		time: true,
		acc: true,
		..Opt::default()
	});
	let args: Vec<String> = std::env::args().collect();
	let n_estimators: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1000);
	let train_csv = args
		.get(2)
		.map(|s| s.as_str())
		.unwrap_or("/tmp/covtype_train.csv");
	let test_csv = args
		.get(3)
		.map(|s| s.as_str())
		.unwrap_or("/tmp/covtype_test.csv");
	let num_leaves: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(63);

	let (x_tr, y_tr, n_tr, p) = load_csv(train_csv)?;
	let (x_te, y_te, n_te, p_te) = load_csv(test_csv)?;
	if p != p_te {
		Write::err(format!("feature count mismatch: {p:?} vs {p_te:?}"))?;
	}

	let n_classes = *y_tr
		.iter()
		.max()
		.ok_or_else(|| Errored::new("empty y_tr"))?
		+ 1;
	Write::line(
		data,
		format!("Covtype: train={n_tr} test={n_te} features={p} classes={n_classes}"),
	);

	let params = Params {
		n_estimators,
		num_leaves,
		max_depth: 0,
		learning_rate: 0.05,
		l2_reg: 1.0,
		min_child_weight: 1.0,
		min_gain_to_split: 0.0,
		n_bins: 255,
		goss_a: 0.2,
		goss_b: 0.1,
		use_efb: false,
		efb_max_conflict: 0.0,
		seed: 42,
	};
	Write::line(
		epoch,
		format!(
			"Config: iters={} leaves={} lr={} goss_a={} goss_b={}",
			params.n_estimators, params.num_leaves, params.learning_rate, params.goss_a, params.goss_b
		),
	);

	let t0 = Instant::now();
	let model = train_multiclass(&x_tr, &y_tr, n_tr, p, n_classes, &params)
		.map_err(|e| Errored::new(format!("train_multiclass: {e}")))?;
	let train_time = t0.elapsed();
	Write::line(
		time,
		format!(
			"Training: {:.2}s ({:.2}ms/iter)",
			train_time.as_secs_f64(),
			train_time.as_secs_f64() * 1000.0 / params.n_estimators as f64
		),
	);

	let t1 = Instant::now();
	let probs = predict_proba(&model, &x_te, n_te).map_err(|e| Errored::new(format!("predict_proba: {e}")))?;
	let predict_time = t1.elapsed();

	let correct = (0..n_te)
		.filter(|&i| {
			(0..n_classes)
				.max_by(|&a, &b| {
					probs[i * n_classes + a]
						.partial_cmp(&probs[i * n_classes + b])
						.unwrap_or_else(|| {
							Write::error(format!("NaN in probs at row {i}"));
							return std::cmp::Ordering::Equal;
						})
				})
				.unwrap_or_else(|| {
					Write::error(format!("empty class range at row {i}"));
					return usize::MAX;
				}) == y_te[i]
		})
		.count();
	let accuracy = correct as f64 / n_te as f64;

	Write::line(
		acc,
		"\n# ── lightgbm-rs results ──────────────────────────────────────",
	);
	Write::line(
		data,
		format!("  dataset:       Covertype (train={n_tr}, test={n_te}, features={p})"),
	);
	Write::line(
		acc,
		format!("  accuracy:      {:.4} ({correct}/{n_te})", accuracy),
	);
	Write::line(
		time,
		format!("  train time:    {:.2}s", train_time.as_secs_f64()),
	);
	Write::line(
		time,
		format!("  predict time:  {:.3}s", predict_time.as_secs_f64()),
	);
	Write::line(
		time,
		format!(
			"  ms/iter:       {:.2}",
			train_time.as_secs_f64() * 1000.0 / params.n_estimators as f64
		),
	);
	Ok(())
}

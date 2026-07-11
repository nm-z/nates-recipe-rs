use recipe::*;

const BANK: &str = "datasets/uci-bank-semicolon/bank-full.csv";
const SEEDS: &str = "datasets/uci-seeds/seeds_dataset.txt";
const WINE: &str = "datasets/uci-wine/wine.data";

fn columns(path: &str) -> recipe::data::RawCsv {
	recipe::data::read_raw_csv(std::path::Path::new(path))
		.unwrap_or_else(|e| panic!("stress: {path}: {e}"))
}

fn main() {
	for (path, want, sep) in [
		(BANK, 17usize, "semicolon"),
		(SEEDS, 8, "tab"),
		(WINE, 14, "comma"),
	] {
		let recipe::data::RawCsv { headers, rows } = columns(path);
		assert_eq!(
			headers.len(),
			want,
			"{sep} sniff failed: {path} parsed {} column(s), want {want}",
			headers.len()
		);
		assert!(!rows.is_empty(), "{path}: no rows parsed");
		eprintln!(
			"\x1b[36mstress\x1b[0m  {sep:<10} {path}  {} cols × {} rows",
			headers.len(),
			rows.len()
		);
	}

	let rows = columns(BANK).rows.len();
	let data = Data::load(BANK).target("y");
	let model = Model::new()
		.loss(bce)
		.layer(64)
		.leak()
		.layer(1)
		.sigmoid()
		.lr(0.001);
	Train::new()
		.epochs(20)
		.log([Loss, Accuracy, hip])
		.run(&data, &model);

	let preds = model.eval(&data);
	assert_eq!(
		preds.len(),
		rows,
		"eval returned {} preds for {rows} rows",
		preds.len()
	);
	assert!(
		preds.iter().all(|p| p.is_finite()),
		"eval produced non-finite predictions"
	);
	eprintln!("\x1b[36mstress\x1b[0m  ok");
}

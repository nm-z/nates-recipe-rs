//! Detector TRAINER — produces `pantry/detector.ogdl`. The labelled corpus, the
//! model builder, and the train/test split all live here, not in the shipped
//! library: training labels exist only for this script. Runtime prediction
//! (the architecture, tokenizer, and inline weights) lives in `pantry::detect`.
//!
//!   cargo run --release --example train_detector

use recipe::data::read_raw_csv;
use recipe::{Accuracy, Dataset, Epoch, Loss, Mat, Model, Train, Vec1, attn, b, ce, embed, w};
use pantry::{
	CONTEXT, EMBED_DIM, HEADS, KIND_CATEGORICAL, KIND_IMAGE, KIND_NUMERIC, KIND_ORDINAL,
	KIND_TEMPORAL, KIND_TEXT, N_CLASS, VOCAB, tokenize_column,
};

fn model() -> Model {
	Model::new()
		.layer(embed(EMBED_DIM).vocab(VOCAB))
		.layer(attn(HEADS))
		.layer(64)
		.leak()
		.layer(N_CLASS)
		.loss(ce)
		.lr(0.002)
}

// ── training corpus: real columns from datasets/, labelled by known schema ───

// Every column across datasets/, hand-labelled by type.
const HP: &str = "datasets/house-prices/train.csv";
const CH: &str = "datasets/playground-series-s6e3/train.csv";
const LLM: &str = "datasets/llm-classification-finetuning/train.csv";
const WINE: &str = "datasets/wine-quality/winequality-red.csv";
const NOSHOW: &str = "datasets/no-show-appointments/KaggleV2-May-2016.csv";
const AMZ: &str = "datasets/amazon-fashion/products.csv";
const FURN: &str = "datasets/furniture/competitors_raw_data.csv";
const COS: &str = "datasets/cosmetics/output.csv";
const HW: &str = "datasets/predict-the-handwriting-images/train.csv";
const MDIR: &str = "datasets/march-machine-learning-mania-2026/";
const OL_ORD: &str = "datasets/olist/olist_orders_dataset.csv";
const OL_REV: &str = "datasets/olist/olist_order_reviews_dataset.csv";
const OL_ITM: &str = "datasets/olist/olist_order_items_dataset.csv";
const OL_CUS: &str = "datasets/olist/olist_customers_dataset.csv";
const LC: &str = "datasets/lendingclub-2007-2011/loan.csv";
const HMP: &str = "datasets/hm-products/handm.csv";
const ADI_G: &str = "datasets/adidas-2026/Adidas_Global.csv";
const ADI_U: &str = "datasets/adidas-2026/US_Adidas.csv";
const NC_W: &str = "datasets/newchic/women.csv";
const NC_M: &str = "datasets/newchic/men.csv";
const NC_S: &str = "datasets/newchic/shoes.csv";
const SV18: &str = "datasets/kaggle-survey-2018/multipleChoiceResponses.csv";
const SV17: &str = "datasets/survey-2017/multipleChoiceResponses.csv";
const NCDIR: &str = "datasets/newchic/";
const NEWCHIC_FILES: &[&str] = &[
	"women", "men", "shoes", "bags", "jewelry", "beauty", "house", "kids", "accessories",
];
const NEWCHIC_IMG: &[&str] = &[
	"variation_0_thumbnail", "variation_0_image", "variation_1_thumbnail", "variation_1_image", "image_url",
];
const STOCK_DIR: &str = "datasets/sandp500/individual_stocks_5yr/individual_stocks_5yr";
const STOCK_N: usize = 40;

const SOURCES: &[(&str, &[&str], usize)] = &[
	(HP, &[
		"Id", "LotFrontage", "LotArea", "YearBuilt", "YearRemodAdd", "MasVnrArea",
		"BsmtFinSF1", "BsmtFinSF2", "BsmtUnfSF", "TotalBsmtSF", "1stFlrSF", "2ndFlrSF",
		"LowQualFinSF", "GrLivArea", "BsmtFullBath", "BsmtHalfBath", "FullBath", "HalfBath",
		"BedroomAbvGr", "KitchenAbvGr", "TotRmsAbvGrd", "Fireplaces", "GarageYrBlt",
		"GarageCars", "GarageArea", "WoodDeckSF", "OpenPorchSF", "EnclosedPorch", "3SsnPorch",
		"ScreenPorch", "PoolArea", "MiscVal", "MoSold", "YrSold", "SalePrice",
	], KIND_NUMERIC),
	(HP, &[
		"LotShape", "LandSlope", "OverallQual", "OverallCond", "ExterQual", "ExterCond",
		"BsmtQual", "BsmtCond", "BsmtExposure", "BsmtFinType1", "BsmtFinType2", "HeatingQC",
		"KitchenQual", "Functional", "FireplaceQu", "GarageFinish", "GarageQual", "GarageCond",
		"PavedDrive", "PoolQC", "Fence",
	], KIND_ORDINAL),
	(HP, &[
		"MSSubClass", "MSZoning", "Street", "Alley", "Utilities", "LotConfig", "LandContour",
		"Neighborhood", "Condition1", "Condition2", "BldgType", "HouseStyle", "RoofStyle",
		"RoofMatl", "Exterior1st", "Exterior2nd", "MasVnrType", "Foundation", "Heating",
		"CentralAir", "Electrical", "GarageType", "MiscFeature", "SaleType", "SaleCondition",
	], KIND_CATEGORICAL),
	(CH, &["id", "tenure", "MonthlyCharges", "TotalCharges"], KIND_NUMERIC),
	(CH, &["Contract"], KIND_ORDINAL),
	(CH, &[
		"gender", "SeniorCitizen", "Partner", "Dependents", "PhoneService", "MultipleLines",
		"InternetService", "OnlineSecurity", "OnlineBackup", "DeviceProtection", "TechSupport",
		"StreamingTV", "StreamingMovies", "PaperlessBilling", "PaymentMethod", "Churn",
	], KIND_CATEGORICAL),
	(LLM, &["id"], KIND_NUMERIC),
	(LLM, &["model_a", "model_b", "winner_model_a", "winner_model_b", "winner_tie"], KIND_CATEGORICAL),
	(LLM, &["prompt", "response_a", "response_b"], KIND_TEXT),
	(WINE, &[
		"fixed acidity", "volatile acidity", "citric acid", "residual sugar", "chlorides",
		"free sulfur dioxide", "total sulfur dioxide", "density", "pH", "sulphates", "alcohol",
	], KIND_NUMERIC),
	(WINE, &["quality"], KIND_ORDINAL),
	(NOSHOW, &["PatientId", "AppointmentID", "Age"], KIND_NUMERIC),
	(NOSHOW, &["ScheduledDay", "AppointmentDay"], KIND_TEMPORAL),
	(NOSHOW, &[
		"Gender", "Neighbourhood", "Scholarship", "Hipertension", "Diabetes", "Alcoholism",
		"Handcap", "SMS_received", "No-show",
	], KIND_CATEGORICAL),
	(AMZ, &["price", "rating"], KIND_NUMERIC),
	(AMZ, &["brand", "category"], KIND_CATEGORICAL),
	(AMZ, &["product_id", "title", "product_url"], KIND_TEXT),
	(AMZ, &["image_url"], KIND_IMAGE),
	(FURN, &["Price", "Rating", "Qty_califications"], KIND_NUMERIC),
	(FURN, &["Category_path"], KIND_CATEGORICAL),
	(FURN, &["Title", "Features_JSON_format", "Item_Url"], KIND_TEXT),
	(FURN, &["Image_Url"], KIND_IMAGE),
	(COS, &["id", "price", "rating"], KIND_NUMERIC),
	(COS, &["brand", "price_sign", "currency", "category", "product_type"], KIND_CATEGORICAL),
	(COS, &["name", "product_link", "website_link", "description", "tag_list", "product_api_url", "product_colors"], KIND_TEXT),
	(COS, &["created_at", "updated_at"], KIND_TEMPORAL),
	(COS, &["image_link", "api_featured_image"], KIND_IMAGE),
	(HW, &["label"], KIND_CATEGORICAL),
	(HW, &["image_id"], KIND_IMAGE),
	("datasets/predict-the-handwriting-images/test.csv", &["image_id"], KIND_IMAGE),
	(OL_ORD, &[
		"order_purchase_timestamp", "order_approved_at", "order_delivered_carrier_date",
		"order_delivered_customer_date", "order_estimated_delivery_date",
	], KIND_TEMPORAL),
	(OL_ORD, &["order_id", "customer_id"], KIND_TEXT),
	(OL_REV, &["review_creation_date", "review_answer_timestamp"], KIND_TEMPORAL),
	(OL_REV, &["review_score"], KIND_ORDINAL),
	(OL_REV, &["review_id", "review_comment_title", "review_comment_message"], KIND_TEXT),
	(OL_ITM, &["shipping_limit_date"], KIND_TEMPORAL),
	(OL_ITM, &["product_id", "seller_id"], KIND_TEXT),
	(OL_CUS, &["customer_unique_id"], KIND_TEXT),
	(LC, &["issue_d", "earliest_cr_line", "last_pymnt_d", "last_credit_pull_d", "next_pymnt_d"], KIND_TEMPORAL),
	(LC, &["grade", "sub_grade", "emp_length", "term"], KIND_ORDINAL),
	(LC, &["emp_title", "url", "desc", "title"], KIND_TEXT),
	(HMP, &["productName", "url", "details", "materials"], KIND_TEXT),
	(ADI_G, &["image_url"], KIND_IMAGE),
	(ADI_G, &["product_name", "sku", "size_labels", "product_url", "canonical_url"], KIND_TEXT),
	(ADI_U, &["image_url"], KIND_IMAGE),
	(ADI_U, &["product_name", "sku"], KIND_TEXT),
	(NC_W, &["name", "url", "model"], KIND_TEXT),
	(NC_M, &["name", "url", "model"], KIND_TEXT),
	(NC_S, &["name", "url"], KIND_TEXT),
	(SV18, &["Q2", "Q4", "Q8", "Q9", "Q23", "Q24", "Q25", "Q43", "Q46"], KIND_ORDINAL),
	(SV17, &[
		"FormalEducation", "Tenure", "TimeSpentStudying", "EmployerSize", "WorkDatasetSize",
		"WorkProductionFrequency", "LearningPlatformUsefulnessArxiv", "LearningPlatformUsefulnessBlogs",
		"LearningPlatformUsefulnessKaggle", "LearningPlatformUsefulnessCourses", "JobSkillImportanceStats",
		"JobSkillImportancePython", "WorkToolsFrequencyPython", "WorkToolsFrequencyExcel",
	], KIND_ORDINAL),
];

// march tables — paths built at runtime from MDIR.
const MARCH: &[(&str, &[&str], usize)] = &[
	("Cities.csv", &["CityID"], KIND_NUMERIC),
	("Cities.csv", &["City"], KIND_TEXT),
	("Cities.csv", &["State"], KIND_CATEGORICAL),
	("Conferences.csv", &["ConfAbbrev"], KIND_CATEGORICAL),
	("Conferences.csv", &["Description"], KIND_TEXT),
	("MTeams.csv", &["TeamID", "FirstD1Season", "LastD1Season"], KIND_NUMERIC),
	("MTeams.csv", &["TeamName"], KIND_TEXT),
	("MSeasons.csv", &["Season"], KIND_NUMERIC),
	("MSeasons.csv", &["DayZero"], KIND_TEMPORAL),
	("MSeasons.csv", &["RegionW", "RegionX", "RegionY", "RegionZ"], KIND_CATEGORICAL),
	("MNCAATourneyDetailedResults.csv", &[
		"Season", "DayNum", "WScore", "LScore", "NumOT", "WFGM", "WFGA", "WFGM3", "WFGA3",
		"WFTM", "WFTA", "WOR", "WDR", "WAst", "WTO", "WStl", "WBlk", "WPF", "LFGM", "LFGA",
		"LFGM3", "LFGA3", "LFTM", "LFTA", "LOR", "LDR", "LAst", "LTO", "LStl", "LBlk", "LPF",
	], KIND_NUMERIC),
	("MNCAATourneyDetailedResults.csv", &["WTeamID", "LTeamID"], KIND_NUMERIC),
	("MNCAATourneyDetailedResults.csv", &["WLoc"], KIND_CATEGORICAL),
	("MNCAATourneySeeds.csv", &["TeamID"], KIND_NUMERIC),
	("MNCAATourneySeeds.csv", &["Seed"], KIND_ORDINAL),
	("MTeamConferences.csv", &["TeamID"], KIND_NUMERIC),
	("MTeamConferences.csv", &["ConfAbbrev"], KIND_CATEGORICAL),
];

fn column_cells(path: &str, col: &str) -> Vec<String> {
	let (headers, rows) = read_raw_csv(std::path::Path::new(path)).expect("read corpus csv");
	let Some(j) = headers.iter().position(|h| h == col) else {
		panic!("corpus: column '{col}' not in {path}");
	};
	rows.iter()
		.filter_map(|r| r.get(j))
		.filter(|v| !v.is_empty())
		.cloned()
		.collect()
}

// ── extra corpus: VNA + UCI dumps in varied delimiters / headerless layouts ──
// The shipped parser only auto-detects all-numeric headerless files; here the
// trainer KNOWS each file's delimiter and whether it has a header, so it passes
// that truth in directly (a headerless file with a categorical column would fool
// any content test). One delimited reader covers comma/semicolon/tab/space.

enum Delim {
	Comma,
	Semicolon,
	Tab,
	Space,
}

fn columns_of(path: &str, d: &Delim, headerless: bool) -> Vec<Vec<String>> {
	let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
	let split = |line: &str| -> Vec<String> {
		let raw: Vec<&str> = match d {
			Delim::Comma => line.split(',').collect(),
			Delim::Semicolon => line.split(';').collect(),
			Delim::Tab => line.split('\t').collect(),
			Delim::Space => line.split_whitespace().collect(),
		};
		raw.iter().map(|s| s.trim().trim_matches('"').trim().to_string()).collect()
	};
	let rows: Vec<Vec<String>> = text.lines().filter(|l| !l.trim().is_empty()).map(split).collect();
	if rows.is_empty() {
		return Vec::new();
	}
	let width = rows.iter().map(Vec::len).max().unwrap_or(0);
	let data = if headerless { &rows[..] } else { &rows[1..] };
	(0..width)
		.map(|j| {
			data.iter()
				.filter_map(|r| r.get(j))
				.filter(|c| !c.is_empty())
				.cloned()
				.collect()
		})
		.collect()
}

/// Expand one column's cells into prefix variants + the full stream, same shape
/// as the in-line corpus builder, so fixture-scale and full-length both appear.
fn push_instances(out: &mut Vec<(Vec<String>, usize)>, cells: Vec<String>, kind: usize) {
	if cells.is_empty() {
		return;
	}
	for take in [4usize, 8, 16] {
		if cells.len() > take {
			out.push((cells[..take].to_vec(), kind));
		}
	}
	out.push((cells, kind));
}

/// Per-column kinds for a `w`-wide file: Numeric by default, with named overrides.
fn kinds(width: usize, overrides: &[(usize, usize)]) -> Vec<(usize, usize)> {
	let mut v: Vec<(usize, usize)> = (0..width).map(|j| (j, KIND_NUMERIC)).collect();
	for &(j, k) in overrides {
		if j < width {
			v[j] = (j, k);
		}
	}
	v
}

fn add_indexed(out: &mut Vec<(Vec<String>, usize)>, path: &str, d: &Delim, headerless: bool, cols: &[(usize, usize)]) {
	let columns = columns_of(path, d, headerless);
	for &(j, kind) in cols {
		if let Some(c) = columns.get(j) {
			push_instances(out, c.clone(), kind);
		}
	}
}

/// Wide all-numeric matrices (VNA predictors, HAR sensor): sample ~k evenly-spaced
/// columns as Numeric. Sampling, not every column, so a 3204-wide file can't swamp
/// the corpus and collapse class balance — logged, never silent.
fn add_sampled_numeric(out: &mut Vec<(Vec<String>, usize)>, path: &str, d: &Delim, headerless: bool, k: usize) {
	let cols = columns_of(path, d, headerless);
	let width = cols.len();
	if width == 0 {
		return;
	}
	let step = (width / k).max(1);
	let mut n = 0;
	for j in (0..width).step_by(step) {
		push_instances(out, cols[j].clone(), KIND_NUMERIC);
		n += 1;
	}
	eprintln!("  {path}: sampled {n} of {width} numeric columns");
}

fn add_new_corpus(out: &mut Vec<(Vec<String>, usize)>) {
	use Delim::{Comma, Semicolon, Space, Tab};
	// VNA — the headerless numeric dumps that motivated this. Targets: one column.
	add_indexed(out, "datasets/VNA/9_10_24_Hold_02_targets.csv", &Comma, true, &[(0, KIND_NUMERIC)]);
	add_indexed(out, "datasets/VNA/sample_targets.csv", &Comma, true, &[(0, KIND_NUMERIC)]);
	add_sampled_numeric(out, "datasets/VNA/sample_predictors.csv", &Comma, true, 64);
	add_sampled_numeric(out, "datasets/VNA/Predictors_2025-04-15_10-43_Hold-2.csv", &Comma, true, 64);

	// UCI comma headerless — numeric attributes + a trailing class/label column.
	// Full columns (not sampled): more data generalized better than a balanced-but-
	// smaller corpus here — the v1 (1722 rows) > v2 (1180 rows) held-out result.
	add_indexed(out, "datasets/uci-wine/wine.data", &Comma, true, &kinds(14, &[(0, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-glass/glass.data", &Comma, true, &kinds(11, &[(10, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-ionosphere/ionosphere.data", &Comma, true, &kinds(35, &[(34, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-sonar/sonar.all-data", &Comma, true, &kinds(61, &[(60, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-abalone/abalone.data", &Comma, true, &kinds(9, &[(0, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-bcw/breast-cancer-wisconsin.data", &Comma, true, &kinds(11, &[(10, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-wdbc/wdbc.data", &Comma, true, &kinds(32, &[(1, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-letter/letter-recognition.data", &Comma, true, &kinds(17, &[(0, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-magic/magic04.data", &Comma, true, &kinds(11, &[(10, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-optdigits/optdigits.tra", &Comma, true, &kinds(65, &[(64, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-spambase/spambase.data", &Comma, true, &kinds(58, &[(57, KIND_CATEGORICAL)]));
	add_indexed(
		out, "datasets/uci-adult/adult.data", &Comma, true,
		&kinds(15, &[
			(1, KIND_CATEGORICAL), (3, KIND_CATEGORICAL), (5, KIND_CATEGORICAL), (6, KIND_CATEGORICAL),
			(7, KIND_CATEGORICAL), (8, KIND_CATEGORICAL), (9, KIND_CATEGORICAL), (13, KIND_CATEGORICAL),
			(14, KIND_CATEGORICAL),
		]),
	);

	// UCI space/tab headerless numeric matrices (+ trailing class).
	add_indexed(out, "datasets/uci-german-numeric/german.data-numeric", &Space, true, &kinds(25, &[]));
	add_indexed(out, "datasets/uci-satimage/sat.trn", &Space, true, &kinds(37, &[(36, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-shuttle/shuttle.trn", &Space, true, &kinds(10, &[(9, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-ecoli/ecoli.data", &Space, true, &kinds(9, &[(0, KIND_TEXT), (8, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-yeast/yeast.data", &Space, true, &kinds(10, &[(0, KIND_TEXT), (9, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-seeds/seeds_dataset.txt", &Tab, true, &kinds(8, &[(7, KIND_CATEGORICAL)]));
	add_indexed(out, "datasets/uci-airfoil/airfoil_self_noise.dat", &Tab, true, &kinds(6, &[]));
	add_sampled_numeric(out, "datasets/uci-har-sensor/UCI HAR Dataset/train/X_train.txt", &Space, true, 64);

	// SMS spam: tab-delimited headerless — label + free text.
	add_indexed(out, "datasets/uci-sms-tab/SMSSpamCollection", &Tab, true, &[(0, KIND_CATEGORICAL), (1, KIND_TEXT)]);

	// Semicolon-delimited WITH headers (quoted fields in bank).
	add_indexed(out, "datasets/uci-winequality-semicolon/winequality-red.csv", &Semicolon, false, &kinds(12, &[(11, KIND_ORDINAL)]));
	add_indexed(out, "datasets/uci-winequality-semicolon/winequality-white.csv", &Semicolon, false, &kinds(12, &[(11, KIND_ORDINAL)]));
	add_indexed(
		out, "datasets/uci-bank-semicolon/bank-full.csv", &Semicolon, false,
		&kinds(17, &[
			(1, KIND_CATEGORICAL), (2, KIND_CATEGORICAL), (3, KIND_ORDINAL), (4, KIND_CATEGORICAL),
			(6, KIND_CATEGORICAL), (7, KIND_CATEGORICAL), (8, KIND_CATEGORICAL), (10, KIND_CATEGORICAL),
			(15, KIND_CATEGORICAL), (16, KIND_CATEGORICAL),
		]),
	);
}

/// Every labelled column across datasets/, as `(byte-stream cells, kind)`.
fn instances() -> Vec<(Vec<String>, usize)> {
	let mut out: Vec<(Vec<String>, usize)> = Vec::new();
	let mut add = |path: &str, cols: &[&str], kind: usize| {
		for col in cols {
			let cells = column_cells(path, col);
			if cells.is_empty() {
				continue;
			}
			// Short prefixes so the detector generalizes to tiny columns whose token
			// stream is mostly PAD — fixtures are ~8-10 cells, but every corpus column
			// is full-length. Each prefix keeps the kind's structure (repeats, decimals,
			// date/path shape, cardinality) intact at fixture scale.
			for take in [4usize, 8, 16] {
				if cells.len() > take {
					out.push((cells[..take].to_vec(), kind));
				}
			}
			out.push((cells, kind));
		}
	};
	for (path, cols, kind) in SOURCES {
		add(path, cols, *kind);
	}
	for (file, cols, kind) in MARCH {
		add(&format!("{MDIR}{file}"), cols, *kind);
	}
	// newchic: same five image-URL columns across every category file.
	for file in NEWCHIC_FILES {
		add(&format!("{NCDIR}{file}.csv"), NEWCHIC_IMG, KIND_IMAGE);
	}
	// s&p500: ISO date column from the first STOCK_N per-ticker files.
	let mut stocks: Vec<std::path::PathBuf> = std::fs::read_dir(STOCK_DIR)
		.expect("stock dir")
		.filter_map(|e| e.ok())
		.map(|e| e.path())
		.filter(|p| p.extension().is_some_and(|x| x == "csv"))
		.collect();
	stocks.sort();
	for p in stocks.into_iter().take(STOCK_N) {
		if let Some(s) = p.to_str() {
			add(s, &["date"], KIND_TEMPORAL);
		}
	}
	drop(add);
	add_new_corpus(&mut out);
	out
}

fn build_dataset(insts: &[(Vec<String>, usize)]) -> Dataset {
	let n = insts.len();
	let mut x = Vec::with_capacity(n * CONTEXT);
	let mut y = vec![0.0f64; n * N_CLASS];
	for (r, (cells, kind)) in insts.iter().enumerate() {
		let refs: Vec<&str> = cells.iter().map(String::as_str).collect();
		x.extend(tokenize_column(&refs));
		y[r * N_CLASS + kind] = 1.0;
	}
	Dataset {
		x: Mat::from_shape_vec((n, CONTEXT), x).expect("corpus matrix"),
		y: Vec1::from_vec(y),
		source: "detector-corpus".to_string(),
		n_targets: N_CLASS,
		has_target: true,
		text_cols: (0..CONTEXT).collect(),
		onehot_groups: Vec::new(),
	}
}

/// Shuffle all labelled columns (seeded) and split into (train, test) by fraction.
/// `test_frac = 0.6` → 40% train / 60% test.
fn corpus_split(seed: u64, test_frac: f64) -> (Dataset, Dataset) {
	use rand::SeedableRng as _;
	use rand::seq::SliceRandom as _;
	let mut insts = instances();
	let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
	insts.shuffle(&mut rng);
	let n_train = ((insts.len() as f64) * (1.0 - test_frac)).round() as usize;
	let (tr, te) = insts.split_at(n_train);
	(build_dataset(tr), build_dataset(te))
}

/// Per-class instance counts (argmax the one-hot targets) — proves the held-out
/// accuracy isn't a degenerate all-Numeric pass.
fn class_balance(tag: &str, ds: &Dataset) {
	let y = ds.y.as_slice().expect("y contiguous");
	let n = y.len() / N_CLASS;
	let mut c = [0usize; N_CLASS];
	for r in 0..n {
		let row = &y[r * N_CLASS..(r + 1) * N_CLASS];
		let mut best = 0;
		for j in 1..N_CLASS {
			if row[j] > row[best] {
				best = j;
			}
		}
		c[best] += 1;
	}
	eprintln!(
		"  {tag}: NUM {} TEMP {} CAT {} ORD {} TEXT {} IMG {}",
		c[KIND_NUMERIC], c[KIND_TEMPORAL], c[KIND_CATEGORICAL], c[KIND_ORDINAL], c[KIND_TEXT], c[KIND_IMAGE]
	);
}

fn main() {
	// 40% train / 60% test over all hand-labelled real columns.
	let (train, test) = corpus_split(0xC0FFEE, 0.6);
	eprintln!("split: {} train / {} test columns", train.x.nrows(), test.x.nrows());
	class_balance("train", &train);
	class_balance("test", &test);

	// Retrain from scratch on the EXPANDED corpus. The prior detector.ogdl (0.987)
	// was a different corpus, so resuming + the best-only save guard would compare
	// against a stale, incomparable score and silently refuse to save the retrained
	// model. Remove it first; resume_from still sets the checkpoint path so the best
	// weights are written during training (it just starts from random, file absent).
	let _ = std::fs::remove_file("pantry/detector.ogdl");
	let model = model();
	let trainer = Train::new().epochs(20000).resume_from("pantry/detector.ogdl").log([Epoch, Loss, Accuracy]);
	trainer.run(&model, &train);
	trainer.save_as([w, b], "pantry/detector.ogdl");
	eprintln!("=== held-out test set (60%) — datatype-detection accuracy ===");
	Train::new().log([Accuracy]).run(&model, &Some(test));
}

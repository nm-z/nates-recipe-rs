use std::{ collections::BTreeSet, env, fmt::Write as _, fs, path::{Path, PathBuf}, process::ExitCode,
	time::{SystemTime, UNIX_EPOCH}, };

const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
const API_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/API.ogdl");

const NORMALIZATIONS: &[&str] = &[".norm(z_score)", ".norm(min_max)", ".norm(l2_norm)"];
const BLOCKS: &[&str] = &[".layer(neurons)", ".perc(count)"];
const BLOCK_WIDTHS: &[usize] = &[8, 12, 16, 24, 32, 48, 64, 96, 128, 256];
const LAYER_NORMALIZATIONS: &[&str] = &[".norm(layer_norm)", ".norm(batch_norm)"];
const ACTIVATIONS: &[&str] = &[
	".relu()", ".gelu()", ".silu()", ".cos()", ".exp()", ".log()", ".ln()", ".huber()", ".tan()",
];
const REGRESSION_LOSSES: &[&str] = &[".loss(mse)", ".loss(mae)", ".loss(huber)"];
const GRADIENT_CLIPS: &[&str] = &["0.25", "0.5", "1.0", "2.0", "5.0"];
const SPLITS: &[&str] = &["0.7", "0.75", "0.8", "0.85", "0.9"];
const LEARNING_RATES: &[&str] = &["0.0001", "0.0002", "0.0003", "0.001", "0.003"];
const SCHEDULES: &[&str] = &[".cos()", ".exp()"];
const LOG_INTERVALS: &[usize] = &[1, 2, 5, 10];
const COMMON_METRICS: &[&str] = &["Loss", "Epoch", "Lr", "Time", "Device"];
const BINARY_METRICS: &[&str] = &["AuRoc", "AuPrc", "Brier", "CalibrationError"];
const CHAIN_WIDTH: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Task { Binary, Regression, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Dataset {
	path: &'static str,
	target: &'static str,
	exclusions: &'static [&'static str],
	task: Task, }

const DATASETS: &[Dataset] = &[ Dataset {
		path: "examples/datasets/house-prices/train.csv",
		target: "SalePrice",
		exclusions: &["Id"],
		task: Task::Regression, }, Dataset {
		path: "examples/datasets/no-show-appointments/KaggleV2-May-2016.csv",
		target: "No-show",
		exclusions: &["AppointmentID", "PatientId"],
		task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-sonar/sonar.all-data",
		target: "col61",
		exclusions: &[], task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-ionosphere/ionosphere.data",
		target: "col35",
		exclusions: &[], task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-wdbc/wdbc.data",
		target: "col2",
		exclusions: &["col1"],
		task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-spambase/spambase.data",
		target: "col58",
		exclusions: &[], task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-magic/magic04.data",
		target: "col11",
		exclusions: &[], task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-bank-semicolon/bank.csv",
		target: "y",
		exclusions: &[], task: Task::Binary, }, Dataset {
		path: "examples/datasets/uci-airfoil/airfoil_self_noise.dat",
		target: "col6",
		exclusions: &[], task: Task::Regression, }, Dataset {
		path: "examples/datasets/uci-abalone/abalone.data",
		target: "col9",
		exclusions: &[], task: Task::Regression, }, Dataset {
		path: "examples/datasets/uci-winequality-semicolon/winequality-red.csv",
		target: "quality",
		exclusions: &[], task: Task::Regression, }, ];

const USAGE: &str = "Generate a random, runnable Recipe API chain in train.rs.\n\
\n\
Usage:\n\
\tcargo run --bin generate-train -- [OPTIONS]\n\
\n\
Options:\n\
\t--seed SEED\tReproduce the exact token sequence\n\
\t--output PATH\tWrite somewhere other than the workspace train.rs\n\
\t--stdout\tPrint the generated source without writing a file\n\
\t-h, --help\tShow this help\n";

#[derive(Debug)]
struct Options { seed: u64, output: Option<PathBuf>, }

#[derive(Debug)]
struct GeneratedSource { source: String, dataset: Dataset, }

#[derive(Debug)]
struct ApiSurface { methods: BTreeSet<String>, }

impl ApiSurface { fn parse(source: &str) -> Result<Self, String> { let methods = source .lines() .map(str::trim)
			.filter(|line| line.starts_with('.'))
			.map(str::to_owned) .collect::<BTreeSet<_>>(); if methods.is_empty() {
			return Err("API.ogdl contains no method declarations".to_owned());
		}
		Ok(Self { methods }) }

	fn require(&self, method: &str) -> Result<(), String> { if self.methods.contains(method) { Ok(()) } else {
			Err(format!("API.ogdl does not declare required token {method}"))
		} }

	fn choose<'a>(&self, random: &mut Random, candidates: &'a [&'a str]) -> Result<&'a str, String> {
		let candidates = candidates .iter() .copied() .filter(|candidate| self.methods.contains(*candidate))
			.collect::<Vec<_>>(); if candidates.is_empty() {
			return Err("API.ogdl contains none of the generator's valid next tokens".to_owned());
		}
		Ok(*random.choose(&candidates)) } }

#[derive(Debug)]
struct Random { state: u64, }

impl Random { const fn new(seed: u64) -> Self { Self { state: seed } }

	fn next(&mut self) -> u64 { self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15); let mut value = self.state;
		value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
		value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb); value ^ (value >> 31) }

	fn index(&mut self, length: usize) -> usize {
		assert!(length != 0, "cannot choose from an empty token set");
		let length = u64::try_from(length).expect("token set length fits u64");
		let rejection_start = u64::MAX - (u64::MAX % length); loop { let value = self.next(); if value < rejection_start {
				return usize::try_from(value % length).expect("selected token index fits usize");
			} } }

	fn choose<'a, T>(&mut self, values: &'a [T]) -> &'a T { &values[self.index(values.len())] }

	fn bool(&mut self) -> bool { self.next() & 1 == 1 } }

fn main() -> ExitCode { match run() { Ok(()) => ExitCode::SUCCESS, Err(error) => {
			eprintln!("generate-train: {error}");
			ExitCode::FAILURE } } }

fn run() -> Result<(), String> { let Some(options) = parse_options(env::args().skip(1))? else {
		print!("{USAGE}");
		return Ok(()); };
	let api_source = fs::read_to_string(API_PATH).map_err(|error| format!("read {API_PATH}: {error}"))?;
	let api = ApiSurface::parse(&api_source)?; let datasets = available_datasets(Path::new(MANIFEST_DIRECTORY));
	let generated = generate_source(&api, options.seed, &datasets)?;

	match options.output { Some(path) => { fs::write(&path, &generated.source)
				.map_err(|error| format!("write generated source {}: {error}", path.display()))?;
			eprintln!(
				"generated {} with seed {} and dataset {}",
				path.display(), options.seed, generated.dataset.path ); }
		None => print!("{}", generated.source),
	}
	Ok(()) }

fn parse_options(arguments: impl IntoIterator<Item = String>) -> Result<Option<Options>, String> { let mut seed = None;
	let mut output = Some(PathBuf::from(MANIFEST_DIRECTORY).join("train.rs"));
	let mut arguments = arguments.into_iter(); while let Some(argument) = arguments.next() { match argument.as_str() {
			"-h" | "--help" => return Ok(None),
			"--seed" => {
				let value = arguments .next()
					.ok_or_else(|| "--seed requires an unsigned integer".to_owned())?;
				seed = Some(parse_seed(&value)?); }
			"--output" => {
				let value = arguments .next()
					.ok_or_else(|| "--output requires a path".to_owned())?;
				if output.is_none() {
					return Err("--output conflicts with --stdout".to_owned());
				}
				output = Some(PathBuf::from(value)); }
			"--stdout" => {
				if output .as_ref()
					.is_some_and(|path| path != &Path::new(MANIFEST_DIRECTORY).join("train.rs"))
				{
					return Err("--stdout conflicts with --output".to_owned());
				}
				output = None; }
			_ if argument.starts_with("--seed=") => {
				seed = Some(parse_seed(&argument["--seed=".len()..])?);
			}
			_ if argument.starts_with("--output=") => {
				if output.is_none() {
					return Err("--output conflicts with --stdout".to_owned());
				}
				output = Some(PathBuf::from(&argument["--output=".len()..]));
			}
			_ => return Err(format!("unknown option {argument:?}\n\n{USAGE}")),
		} }
	Ok(Some(Options { seed: seed.unwrap_or_else(random_seed), output, })) }

fn parse_seed(value: &str) -> Result<u64, String> { value.parse()
		.map_err(|error| format!("invalid seed {value:?}: {error}"))
}

fn random_seed() -> u64 { let time = SystemTime::now() .duration_since(UNIX_EPOCH) .unwrap_or_default()
		.as_nanos() as u64; time ^ u64::from(std::process::id()).rotate_left(23) }

fn available_datasets(root: &Path) -> Vec<Dataset> { DATASETS .iter() .copied()
		.filter(|dataset| root.join(dataset.path).is_file()) .collect() }

fn generate_source(api: &ApiSurface, seed: u64, datasets: &[Dataset]) -> Result<GeneratedSource, String> {
	for required in [
		".data(sources)",
		".target(targets)",
		".split(train_fraction)",
		".model()",
		".layer(neurons)",
		".train()",
		".optimizer(adamw)",
		".epochs(count)",
		".lr(rate)",
		".run()",
	] { api.require(required)?; }
	if datasets.is_empty() {
		return Err("none of the generator's tabular datasets exist".to_owned());
	}

	let mut random = Random::new(seed); let dataset = *random.choose(datasets); let mut data = vec![
		format!(".data({:?})", dataset.path),
		format!(".target({:?})", dataset.target),
	]; if !dataset.exclusions.is_empty() && random.bool() { let exclusion = random.choose(dataset.exclusions);
		data.push(format!(".exclude({exclusion:?})"));
	}
	data.push(api.choose(&mut random, NORMALIZATIONS)?.to_owned());
	data.push(format!(".split({})", random.choose(SPLITS)));

	let hidden_blocks = random.index(4) + 1;
	let mut model = vec![".model()".to_owned()];
	for _ in 0..hidden_blocks { let block = api.choose(&mut random, BLOCKS)?; let width = random.choose(BLOCK_WIDTHS);
		model.push(match block {
			".layer(neurons)" => format!(".layer({width})"),
			".perc(count)" => format!(".perc({width})"),
			_ => unreachable!("the block token came from BLOCKS"),
		}); let activation = api.choose(&mut random, ACTIVATIONS)?.to_owned(); if random.bool() {
			let normalization = api.choose(&mut random, LAYER_NORMALIZATIONS)?.to_owned(); if random.bool() {
				model.push(normalization); model.push(activation); } else { model.push(activation); model.push(normalization); }
		} else { model.push(activation); } }
	model.push(".layer(1)".to_owned());
	model.push(match dataset.task { Task::Binary => {
			api.require(".loss(bce)")?;
			".loss(bce)".to_owned()
		}
		Task::Regression => api.choose(&mut random, REGRESSION_LOSSES)?.to_owned(), }); if random.bool() {
		api.require(".grad(clip: maximum_norm)")?;
		model.push(format!(".grad(clip({}))", random.choose(GRADIENT_CLIPS)));
	}

	let epochs = random.index(20) + 1; let mut train = vec![
		".train()".to_owned(),
		".optimizer(adamw)".to_owned(),
		format!(".epochs({epochs})"),
		format!(".lr({})", random.choose(LEARNING_RATES)),
		api.choose(&mut random, SCHEDULES)?.to_owned(), ]; if epochs > 1 && random.bool() {
		api.require(".warmup(count)")?;
		train.push(format!(".warmup({})", random.index(epochs - 1) + 1));
	}

	let metrics = choose_metrics(&mut random, dataset.task);
	let metric_list = metrics.join(", ");
	let log_token = api.choose( &mut random,
		&[".log([items])", ".log([items]).every(interval)"],
	)?;
	if log_token == ".log([items]).every(interval)" {
		train.push(format!(".log([{metric_list}])"));
		train.push(format!(".every({})", random.choose(LOG_INTERVALS)));
	} else {
		train.push(format!(".log([{metric_list}])"));
	}
	if random.bool() {
		api.require(".plot([items])")?;
		train.push(format!(".plot([{metric_list}])"));
	}
	train.push(".run()".to_owned());

	let mut source = String::new();
	writeln!(&mut source, "#!/usr/bin/env -S recipe run").expect("write to String");
	writeln!(&mut source, "// Generated token by token from API.ogdl.").expect("write to String");
	writeln!( &mut source,
		"// Reproduce with: cargo run --bin generate-train -- --seed {seed}"
	)
	.expect("write to String");
	writeln!(&mut source, "\nuse recipe::*;\n").expect("write to String");
	writeln!(&mut source, "fn main() -> TrainingResult<()> {{").expect("write to String");
	emit_chain(&mut source, &data, ";");
	source.push('\n');
	emit_chain(&mut source, &model, ";");
	source.push('\n');
	emit_chain(&mut source, &train, "?;");
	writeln!(&mut source, "\tOk(())").expect("write to String");
	writeln!(&mut source, "}}").expect("write to String");

	Ok(GeneratedSource { source, dataset }) }

fn choose_metrics(random: &mut Random, task: Task) -> Vec<&'static str> {
	let mut candidates = COMMON_METRICS.to_vec(); if task == Task::Binary { candidates.extend_from_slice(BINARY_METRICS); }
	let count = random.index(candidates.len().min(4)) + 1; let mut metrics = Vec::with_capacity(count); for _ in 0..count {
		metrics.push(candidates.swap_remove(random.index(candidates.len()))); }
	metrics }

fn emit_chain(source: &mut String, methods: &[String], terminator: &str) {
	let single_line = format!("\trecipe{}{terminator}", methods.concat());
	if single_line.chars().count() <= CHAIN_WIDTH {
		writeln!(source, "{single_line}").expect("write to String");
		return; }
	let (first, remainder) = methods .split_first()
		.expect("every generated chain has a root method");
	writeln!(source, "\trecipe{first}").expect("write to String");
	for (index, method) in remainder.iter().enumerate() { let line_terminator = if index + 1 == remainder.len() {
			terminator } else {
			""
		};
		writeln!(source, "\t\t{method}{line_terminator}").expect("write to String");
	} }

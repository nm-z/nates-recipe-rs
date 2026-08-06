use super::*;
pub trait IntoDataSources {
	fn into_data_sources(self) -> Vec<String>; } impl IntoDataSources for &str {
	fn into_data_sources(self) -> Vec<String> { 		vec![self.to_owned()] 	} } impl IntoDataSources for String {
	fn into_data_sources(self) -> Vec<String> { 		vec![self] 	} }
impl<T: Into<String>, const N: usize> IntoDataSources for [T; N] { 	fn into_data_sources(self) -> Vec<String> {
		self.into_iter().map(Into::into).collect() 	} } impl<T: Into<String>> IntoDataSources for Vec<T> {
	fn into_data_sources(self) -> Vec<String> { 		self.into_iter().map(Into::into).collect() 	} }
impl<T: Clone + Into<String>> IntoDataSources for &[T] { 	fn into_data_sources(self) -> Vec<String> {
	self.iter().cloned().map(Into::into).collect() 	} } impl Data {
	pub fn target(mut self, target: impl IntoDataSources) -> Self { 		self.target = target.into_data_sources(); 		self 	}
	pub fn r#in(mut self, names: impl IntoDataSources) -> Self {
		self.routes.push(Route { inputs: names.into_data_sources(), outputs: Vec::new() }); 		self 	}
	pub fn out(mut self, names: impl IntoDataSources) -> Self {
		self.routes.last_mut().unwrap_or_else(|| panic!(".out() requires a preceding .r#in()")).outputs =
			names.into_data_sources();
		self 	}
	pub fn exclude(mut self, names: impl IntoDataSources) -> Self { 		self.exclusions = names.into_data_sources(); 		self
	} 	pub fn set(mut self, source: impl Into<String>) -> Self { 		self.sources.push(source.into()); 		self 	}
	pub const fn norm(mut self, _: ZScore) -> Self { 		self.normalize = true; 		self 	}
	pub const fn split(mut self, fraction: f64) -> Self { 		self.split = fraction; 		self 	} } pub(super) struct Prepared {
	pub(super) samples: Vec<f64>, 	pub(super) targets: Vec<f64>, 	pub(super) rows: usize,
	pub(super) features: usize, 	pub(super) schema: String, } struct Table { 	name: String,
	headers: Vec<String>, 	rows: Vec<Vec<String>>, } enum FeatureType { 	Numeric(&'static str), 	Categorical(Vec<String>),
	Text(usize), }
pub(super) fn prepare(data: &Data) -> Result<&Prepared> { 	match data.prepared.get_or_init(|| prepare_data(data)) {
		Ok(prepared) => Ok(prepared), 		Err(error) => Err(error.clone()), 	} }
fn prepare_data(data: &Data) -> Result<Prepared> { 	let mut paths = Vec::new(); 	for source in &data.sources {
		collect_files(&expand_home(source)?, &mut paths)?; 	}
	paths.sort();
	paths.dedup();
	let mut tables = Vec::new(); 	for path in paths { 		let bytes = fs::read(&path)
			.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
		if path.extension().and_then(|value| value.to_str()).is_some_and(is_table) {
			tables.push(parse_table(&path, &bytes)?); 		} 	}
	tables = merge_partitions(tables, &data.target);
	require(!tables.is_empty(), "data source contains no supported table")?;
	let mut selected = Vec::new(); 	for name in &data.target {
		let mut matches = Vec::new(); 		for (table, value) in tables.iter().enumerate() {
			for (column, header) in value.headers.iter().enumerate() { 				let qualified = format!("{}.{}", value.name, header);
				let numbered = format!("col{}", column + 1);
				let qualified_numbered = format!("{}.{}", value.name, numbered);
				if name == header || name == &qualified || name == &numbered || name == &qualified_numbered {
					matches.push((table, column)); 				} 			} 		}
		require(matches.len() == 1, format!("target {name:?} must identify exactly one feature"))?;
		selected.push(matches[0]); 	}
	let table_index = selected.first().map_or(0, |target| target.0);
	let row_count = tables[table_index].rows.len();
	require(selected.iter().all(|target| tables[target.0].rows.len() == row_count), "target row counts differ")?;
	let fit_rows = ((row_count as f64) * data.split).floor().max(1.0) as usize;
	eprintln!("Feature name:                         Dtype:    Samples:"); 	for value in &tables {
		for (column, header) in value.headers.iter().enumerate() {
			let kind = infer_feature(value, column, fit_rows.min(value.rows.len())); 			let samples =
				value.rows.iter().filter(|row| row.get(column).is_some_and(|item| !item.is_empty())).count();
			eprintln!("{:<37} {:<9} {samples}", format!("{}.{}", value.name, header), kind.name()); 		} 	}
	let mut columns = Vec::new(); 	for (table, value) in tables.iter().enumerate() { 		if value.rows.len() == row_count {
			for (column, header) in value.headers.iter().enumerate() { 				let qualified = format!("{}.{}", value.name, header);
				let excluded = data.exclusions.iter().any(|name| name == header || name == &qualified);
				if !selected.contains(&(table, column)) && !excluded {
					columns.push((table, column, infer_feature(value, column, fit_rows))); 				} 			} 		} 	}
	let features = columns.iter().map(|column| column.2.width()).sum();
	require(features != 0, "dataset has no training features")?; 	let target_categories =
		selected.iter().map(|target| categories(&tables[target.0], target.1, fit_rows)).collect::<Vec<_>>();
	let mut samples = Vec::new();
	let mut targets = Vec::new(); 	for row in 0..row_count {
		let mut encoded = Vec::with_capacity(features); 		let valid = columns.iter().all(|column| {
			tables[column.0].rows[row].get(column.1).is_some_and(|value| encode(value, &column.2, &mut encoded)) 		});
		if valid && selected.is_empty() { 			samples.extend_from_slice(&encoded);
			targets.push(0.0); 		} else if valid { 			for (target, categories) in selected.iter().zip(&target_categories) {
				let value = tables[target.0].rows[row].get(target.1);
				let target = value.and_then(|value| value.parse::<f64>().ok()).or_else(|| {
					value.and_then(|value| categories.iter().position(|category| category == value)) 						.map(|value| value as f64)
				}); 				if let Some(target) = target 					&& target.is_finite() 				{
					samples.extend_from_slice(&encoded);
					targets.push(target); 				} 			} 		} 	}
	let rows = targets.len();
	require(rows != 0, "dataset has no complete training rows")?; 	if data.normalize {
		normalize_samples(&mut samples, features, ((rows as f64) * data.split).floor() as usize)?; 	}
	shuffle(&mut samples, &mut targets, features)?; 	let schema = columns 		.iter() 		.map(|column| {
			format!("{}.{}:{}", tables[column.0].name, tables[column.0].headers[column.1], column.2.width()) 		})
		.collect::<Vec<_>>() 		.join("|") + "->" 		+ &data.target.join("|");
	Ok(Prepared { samples, targets, rows, features, schema }) }
fn normalize_samples(samples: &mut [f64], features: usize, fit: usize) -> Result<()> {
	require(fit != 0, "split must retain normalization rows")?; 	for column in 0..features {
		let mean = (0..fit).map(|row| samples[row * features + column]).sum::<f64>() / fit as f64; 		let variance =
			(0..fit).map(|row| (samples[row * features + column] - mean).powi(2)).sum::<f64>() / fit as f64;
		let scale = if variance == 0.0 { 1.0 } else { variance.sqrt() }; 		for row in 0..samples.len() / features {
			samples[row * features + column] = (samples[row * features + column] - mean) / scale; 		} 	} 	Ok(()) }
fn is_table(extension: &str) -> bool { 	matches!(extension.to_ascii_lowercase().as_str(), "csv" | "tsv" | "txt") }
fn expand_home(source: &str) -> Result<PathBuf> { 	if source == "~" || source.starts_with("~/") {
		let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?;
		return Ok(PathBuf::from(home).join(source.trim_start_matches("~/"))); 	} 	Ok(PathBuf::from(source)) }
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> { 	let metadata = fs::metadata(path)
		.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?; 	if metadata.is_file() {
		files.push(path.to_owned());
		return Ok(()); 	} 	let mut children = fs::read_dir(path)
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?
		.collect::<std::io::Result<Vec<_>>>()
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
	children.sort_by_key(fs::DirEntry::path); 	for child in children {
		collect_files(&child.path(), files)?; 	} 	Ok(()) }
fn target_column(table: &Table, name: &str) -> Option<usize> { 	table.headers.iter().enumerate()
		.position(|(column, header)| name == header || name == format!("col{}", column + 1)) }
fn merge_partitions(mut tables: Vec<Table>, targets: &[String]) -> Vec<Table> {
	if targets.is_empty() || targets.iter().any(|target| target.contains('.')) { 		return tables; 	}
	let members = tables.iter().enumerate().filter_map(|(index, table)| {
		targets.iter().all(|target| target_column(table, target).is_some()).then_some(index) 	}).collect::<Vec<_>>();
	if members.len() < 2 { 		return tables 	} 	let mut headers = Vec::new();
	for &index in &members { 		for header in &tables[index].headers { 			if !headers.contains(header) {
				headers.push(header.clone()) 			} 		} 	} 	let mut rows = Vec::new();
	for index in members { 		let positions = tables[index].headers.iter()
			.map(|header| headers.iter().position(|value| value == header).unwrap()).collect::<Vec<_>>();
		for row in std::mem::take(&mut tables[index].rows) {
			let mut merged = std::iter::repeat_with(String::new).take(headers.len()).collect::<Vec<_>>();
			for (column, value) in row.into_iter().enumerate() { 				merged[positions[column]] = value; 			}
			rows.push(merged); 		} 	} 	vec![Table { name: "data".to_owned(), headers, rows }] }
fn parse_table(path: &Path, bytes: &[u8]) -> Result<Table> {
	let first = bytes.split(|byte| *byte == b'\n').next().unwrap_or_default();
	let delimiter = [b',', b';', b'\t'] 		.into_iter()
		.max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count()) 		.unwrap_or(b',');
	let mut rows = records(bytes, delimiter)?;
	require(!rows.is_empty(), format!("dataset {} is empty", path.display()))?;
	let first = rows.remove(0);
	let headerless = first.iter().all(|value| value.parse::<f64>().is_ok()); 	let headers =
		if headerless { (1..=first.len()).map(|column| format!("col{column}")).collect() } else { first.clone() };
	if headerless { 		rows.insert(0, first); 	}
	let width = headers.len();
	rows.retain(|row| row.len() == width);
	let name = path.file_stem().and_then(|value| value.to_str()).unwrap_or("data").to_owned();
	Ok(Table { name, headers, rows }) } fn records(bytes: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>> {
	let mut rows = Vec::new();
	let mut row = Vec::new();
	let mut field = Vec::new();
	let mut quoted = false;
	let mut index = 0; 	while index < bytes.len() {
		let byte = bytes[index]; 		if byte == b'"' { 			if quoted && bytes.get(index + 1) == Some(&b'"') {
				field.push(byte);
				index += 1; 			} else {
				quoted = !quoted; 			} 		} else if byte == delimiter && !quoted {
			row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
			field = Vec::new(); 		} else if byte == b'\n' && !quoted {
			let value = String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?;
			row.push(value.trim_end_matches('\r').to_owned());
			field = Vec::new(); 			if row.iter().any(|value| !value.is_empty()) {
				rows.push(row); 			}
			row = Vec::new(); 		} else {
			field.push(byte); 		}
		index += 1; 	}
	require(!quoted, "unterminated quoted feature")?; 	if !field.is_empty() || !row.is_empty() {
		row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
		rows.push(row); 	} 	Ok(rows) } fn categories(table: &Table, column: usize, rows: usize) -> Vec<String> { 	table.rows
		.iter() 		.take(rows) 		.filter_map(|row| row.get(column)) 		.filter(|value| !value.is_empty()) 		.cloned()
		.collect::<BTreeSet<_>>() 		.into_iter() 		.collect() }
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
	let values = table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty())
		.collect::<Vec<_>>();
	if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) {
		return FeatureType::Numeric("f64"); 	}
	let categories = categories(table, column, rows); 	if categories.len() < values.len() {
		FeatureType::Categorical(categories) 	} else {
		FeatureType::Text(values.iter().map(|value| value.len()).max().unwrap_or(0)) 	} } impl FeatureType {
	const fn name(&self) -> &'static str { 		match self { 			Self::Numeric(name) => name,
			Self::Categorical(_) => "categoric", 			Self::Text(_) => "string", 		} 	} 	fn width(&self) -> usize { 		match self {
			Self::Numeric(_) => 1, 			Self::Categorical(values) => values.len(), 			Self::Text(width) => *width, 		} 	} }
fn encode(value: &str, kind: &FeatureType, output: &mut Vec<f64>) -> bool { 	if value.is_empty() {
		output.resize(output.len() + kind.width(), 0.0);
		return true;
	} 	match kind {
		FeatureType::Numeric(_) => value.parse::<f64>().is_ok_and(|value| { 			output.push(value); 			value.is_finite() 		}),
		FeatureType::Categorical(categories) => { 			let found = categories.iter().position(|category| category == value);
			output.extend((0..categories.len()).map(|index| f64::from(found == Some(index)))); 			found.is_some() 		}
		FeatureType::Text(width) => {
			output.extend(value.bytes().map(f64::from).chain(std::iter::repeat(0.0)).take(*width)); 			value.len() <= *width 		}
	} } fn shuffle(samples: &mut Vec<f64>, targets: &mut Vec<f64>, features: usize) -> Result<()> {
	let mut seed = env!("RECIPE_RANDOM_SEED") 		.parse::<u64>()
		.map_err(|error| RecipeError::new(format!("invalid random seed: {error}")))?;
	let mut order = (0..targets.len()).collect::<Vec<_>>(); 	for index in (1..order.len()).rev() {
		seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		order.swap(index, (seed as usize) % (index + 1)); 	}
	let old_samples = std::mem::take(samples);
	let old_targets = std::mem::take(targets); 	for row in order {
		samples.extend_from_slice(&old_samples[row * features..(row + 1) * features]);
		targets.push(old_targets[row]); 	} 	Ok(()) }
pub struct Train { 	pub(super) epochs: usize,
	pub(super) learning_rate: f64, 	pub(super) log_metrics: Vec<Metric>, 	pub(super) stop: Option<f64>,
	pub(super) resume: Option<String>, 	pub(super) save: Option<String>, }
impl Train { 	pub const fn stop(mut self, value: f64) -> Self { 		self.stop = Some(value); 		self 	}
	pub const fn optimizer(self, _: Adamw) -> Self { 		self 	} 	pub const fn epochs(mut self, value: usize) -> Self {
		self.epochs = value; 		self 	} 	pub const fn lr(mut self, value: f64) -> Self {
		self.learning_rate = value; 		self 	}
	pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self {
		self.log_metrics = metrics.into(); 		self 	} 	pub fn save(mut self, path: impl Into<String>) -> Self {
		self.save = Some(path.into()); 		self 	} 	pub fn resume(mut self, path: impl Into<String>) -> Self {
		self.resume = Some(path.into()); 		self 	} 	pub fn run(&self, model: &Model, data: &Data) -> TrainingReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) }); 		if INTERRUPTED.load(Ordering::Acquire) {
			std::process::exit(INTERRUPTED_EXIT); 		} 		self.try_run(model, data).unwrap_or_else(|error| panic!("{error}")) 	}
	fn try_run(&self, model: &Model, data: &Data) -> Result<TrainingReport> { 		let backend = device_backend()?;
		let config = Config::load()?;
		require(model.downstream.is_none(), "model-valued loss requires .rat()")?;
		let prepared = prepare(data)?;
		let training_rows = ((prepared.rows as f64) * data.split).floor() as usize;
		require(training_rows != 0 && training_rows <= prepared.rows, "split must select training rows")?;
		let run = RUN.fetch_add(1, Ordering::Relaxed) + 1;
		let mut graph = compile(model, prepared, training_rows, backend, config)?;
		let output = graph.output.elements();
		require(output == 1, "model output width must be one")?; 		if let Some(path) = &self.resume {
			let mut stored = stored_graph(&graph, data);
			bundle::restore(path, &prepared.schema, std::slice::from_mut(&mut stored))?;
			graph = stored.graph;
			eprintln!("resumed: {path}"); 		}
		let samples = &prepared.samples[..training_rows * prepared.features];
		let targets = &prepared.targets[..training_rows];
		let mut tape = DeviceTape::new(&graph, samples, targets, backend)?;
		let dispatched = tape.forward();
		self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape)?;
		let initial_predictions = tape.predictions()?;
		let initial_loss = model_loss(&initial_predictions, targets, model.loss, config.activation[7]);
		let tolerance = self.stop.unwrap_or(0.0);
		require(tolerance.is_finite() && (0.0..=1.0).contains(&tolerance), "stop must be between zero and one")?;
		for epoch in 1..=self.epochs { 			let started = Instant::now();
			let dispatched = tape.epoch(epoch, self.learning_rate, model.loss, tolerance, config, false);
			let (loss, checkpoint) = self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape)?;
			let predictions = tape.predictions()?; 			if checkpoint && let Some(path) = &self.save {
				save_graph(path, &graph, data, &prepared.schema, &tape.weights(true)?)?; 			}
			self.print(model, run, epoch, loss, targets, &predictions, started, checkpoint);
			self.finish_dispatch(Ok(()), &graph, data, &prepared.schema, &tape)?;
		} 		if self.stop.is_some() { 			tape.restore_best()?; 		}
		let dispatched = tape.forward();
		self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape)?;
		let predictions = tape.predictions()?;
		let final_loss = model_loss(&predictions, targets, model.loss, config.activation[7]);
		if let Some(path) = &self.save {
			save_graph(path, &graph, data, &prepared.schema, &tape.weights(self.stop.is_some())?)?;
		} 		Ok(TrainingReport(initial_loss, final_loss, initial_predictions, predictions)) 	}
	fn finish_dispatch<T>(
		&self,
		result: Result<T>,
		graph: &Graph,
		data: &Data,
		schema: &str,
		tape: &DeviceTape,
	) -> Result<T> {
		if INTERRUPTED.load(Ordering::Acquire) {
			if let Some(path) = &self.save {
				save_graph(path, graph, data, schema, &tape.weights(self.stop.is_some())?)?;
			}
			std::process::exit(INTERRUPTED_EXIT)
		}
		result
	}
	fn print( 		&self,
		model: &Model, 		run: u64, 		epoch: usize, 		loss: f64, 		targets: &[f64], 		predictions: &[f64], 		started: Instant,
		checkpoint: bool, 	) { 		if self.log_metrics.is_empty() { 			return; 		}
		let topology = model.description(&self.log_metrics);
		let r2 = coefficient(targets, predictions);
		let time = started.elapsed().as_secs_f64() * 1000.0;
		let mut values = Vec::new();
		let mut topology_printed = false; 		for metric in &self.log_metrics { 			let value = match metric.0 {
				0 => format!("run \x1b[38\x3b2\x3b242\x3b40\x3b60m{run:>5}\x1b[0m"),
				1 => format!("{} \x1b[38\x3b2\x3b0\x3b174\x3b107m{loss:.4}\x1b[0m", model.loss.name()),
				2 => format!("r2 \x1b[38\x3b2\x3b39\x3b125\x3b255m{r2:>7.4}\x1b[0m"),
				3 => format!("time \x1b[38\x3b2\x3b255\x3b194\x3b0m{time:>9.3} ms\x1b[0m"),
				4 => format!("epoch \x1b[38\x3b2\x3b135\x3b90\x3b251m{epoch}\x1b[0m"),
				5..=7 if !topology_printed && !topology.is_empty() => { 					topology_printed = true; 					topology.clone() 				}
				5..=7 => continue, 				_ => unreachable!(), 			};
			values.push(value); 		} 		if checkpoint && self.stop.is_some() {
			values.push("\x1b[1\x3b32m← checkpoint\x1b[0m".to_owned()); 		}
		eprintln!("{}", values.join("  ")); 	} }
pub struct TrainingReport(f64, f64, Vec<f64>, Vec<f64>); impl TrainingReport {
	pub const fn initial_loss(&self) -> f64 { 		self.0 	} 	pub const fn final_loss(&self) -> f64 { 		self.1 	}
	pub fn initial_predictions(&self) -> &[f64] { 		&self.2 	} 	pub fn predictions(&self) -> &[f64] { 		&self.3 	} }
fn model_loss(predictions: &[f64], targets: &[f64], loss: LossFunction, threshold: f64) -> f64 {
	let values = predictions.iter().zip(targets);
	let mut result = values.map(|(prediction, target)| loss.value(*prediction, *target, threshold)).sum::<f64>()
		/ targets.len() as f64; 	if loss.0 == 1 {
		result = result.sqrt(); 	} 	result } fn coefficient(targets: &[f64], predictions: &[f64]) -> f64 {
	let mean = targets.iter().sum::<f64>() / targets.len() as f64;
	let residual = targets.iter().zip(predictions).map(|(target, value)| (target - value).powi(2)).sum::<f64>();
	let total = targets.iter().map(|target| (target - mean).powi(2)).sum::<f64>();
	if total == 0.0 { 0.0 } else { 1.0 - residual / total } }
use std::{
	collections::{BTreeMap, BTreeSet},
	io::{BufRead, BufReader, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

struct Process {
	child: Child,
	input: Option<ChildStdin>,
	output: BufReader<ChildStdout>,
}

impl Process {
	fn spawn(command: &str) -> Result<Self> {
		require(!command.trim().is_empty(), ".every() requires a command")?;
		let mut child = Command::new(command)
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.spawn()
			.map_err(|error| RecipeError::new(format!("cannot start {command:?}: {error}")))?;
		let input = child.stdin.take().ok_or_else(|| RecipeError::new("RAT command stdin is absent"))?;
		let output = child.stdout.take().ok_or_else(|| RecipeError::new("RAT command stdout is absent"))?;
		Ok(Self { child, input: Some(input), output: BufReader::new(output) })
	}

	fn read(&mut self) -> Result<BTreeMap<String, f64>> {
		let mut values = BTreeMap::new();
		loop {
			let mut line = String::new();
			let bytes = self
				.output
				.read_line(&mut line)
				.map_err(|error| RecipeError::new(format!("cannot read RAT command: {error}")))?;
			require(bytes != 0, "RAT command exited before a blank-line frame terminator")?;
			let line = line.trim();
			if line.is_empty() {
				require(!values.is_empty(), "RAT command returned an empty frame")?;
				return Ok(values);
			}
			let Some((name, value)) = line.split_once(char::is_whitespace) else {
				continue;
			};
			let value = value
				.trim()
				.parse::<f64>()
				.map_err(|error| RecipeError::new(format!("RAT value {name:?} is invalid: {error}")))?;
			require(value.is_finite(), format!("RAT value {name:?} must be finite"))?;
			require(values.insert(name.to_owned(), value).is_none(), format!("RAT value {name:?} is duplicated"))?;
		}
	}

	fn write(&mut self, names: &[String], values: &[f64]) -> Result<()> {
		require(names.len() == values.len(), "RAT proposal has the wrong width")?;
		let input = self.input.as_mut().ok_or_else(|| RecipeError::new("RAT command stdin is closed"))?;
		writeln!(input, "proposal").map_err(|error| RecipeError::new(format!("cannot write RAT command: {error}")))?;
		for (name, value) in names.iter().zip(values) {
			writeln!(input, "    {name} {value}")
				.map_err(|error| RecipeError::new(format!("cannot write RAT command: {error}")))?;
		}
		writeln!(input).map_err(|error| RecipeError::new(format!("cannot write RAT command: {error}")))?;
		input.flush().map_err(|error| RecipeError::new(format!("cannot flush RAT command: {error}")))
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		drop(self.input.take());
		let _ = self.child.wait();
	}
}

struct State {
	graphs: Vec<bundle::StoredGraph>,
	tapes: Vec<DeviceTape>,
	steps: Vec<usize>,
	schema: String,
}

pub struct RatTrain<const N: usize> {
	train: Train,
	models: [Model; N],
	command: Option<String>,
	process: Option<Process>,
	context: Option<BTreeMap<String, f64>>,
	state: Option<State>,
}

pub struct RatReport {
	proposal: Vec<f64>,
	prediction: Vec<f64>,
	measurement: Vec<f64>,
}

impl RatReport {
	pub fn proposal(&self) -> &[f64] {
		&self.proposal
	}

	pub fn prediction(&self) -> &[f64] {
		&self.prediction
	}

	pub fn measurement(&self) -> &[f64] {
		&self.measurement
	}
}

fn rat<const N: usize>(train: Train, models: [Model; N]) -> RatTrain<N> {
	RatTrain { train, models, command: None, process: None, context: None, state: None }
}

impl Train {
	pub fn rat(self, proposer: Model, predictor: Model) -> RatTrain<2> {
		rat(self, [proposer, predictor])
	}

	pub fn rats<const N: usize>(self, models: [Model; N]) -> RatTrain<N> {
		rat(self, models)
	}
}

fn values(names: &[String], source: &BTreeMap<String, f64>) -> Result<Vec<f64>> {
	names
		.iter()
		.map(|name| source.get(name).copied().ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is absent"))))
		.collect()
}

fn schema(data: &Data) -> String {
	data.routes
		.iter()
		.map(|route| format!("{}->{}", route.inputs.join("|"), route.outputs.join("|")))
		.chain(std::iter::once(format!("target->{}", data.target.join("|"))))
		.collect::<Vec<_>>()
		.join("/")
}

fn build<const N: usize>(
	models: &[Model; N],
	train: &Train,
	data: &Data,
	backend: Backend,
	config: Config,
) -> Result<State> {
	require(N >= 2, "RAT requires an intermediate model and a predictor")?;
	require(data.routes.len() + 1 == N, "RAT requires one .r#in().out() pair per intermediate model")?;
	require(!data.target.is_empty(), "RAT requires .target()")?;
	let mut available = data.routes[0].inputs.iter().cloned().collect::<BTreeSet<_>>();
	let mut graphs = Vec::with_capacity(N);
	for (index, route) in data.routes.iter().enumerate() {
		if let Some(downstream) = &models[index].downstream {
			require(downstream == &models[index + 1].blocks, "model-valued loss must name the next RAT model")?;
		}
		require(!route.inputs.is_empty() && !route.outputs.is_empty(), "RAT route names must not be empty")?;
		require(route.inputs.iter().all(|name| available.contains(name)), "RAT route input is not yet available")?;
		let sample = vec![
			0.0;
			route.inputs.len()
		];
		let prepared = Prepared {
			samples: sample,
			targets: vec![0.0],
			rows: 1,
			features: route.inputs.len(),
			schema: schema(data),
		};
		let graph = compile_output(&models[index], &prepared, 1, backend, config, route.outputs.len())?;
		graphs.push(bundle::StoredGraph { graph, inputs: route.inputs.clone(), outputs: route.outputs.clone() });
		available.extend(route.outputs.iter().cloned());
	}
	let route = data.routes.last().ok_or_else(|| RecipeError::new("RAT route is absent"))?;
	require(models[N - 1].downstream.is_none(), "the final RAT model requires a scalar loss")?;
	let mut inputs = route.inputs.clone();
	inputs.extend(route.outputs.iter().cloned());
	let prepared = Prepared {
		samples: vec![0.0; inputs.len()],
		targets: vec![0.0; data.target.len()],
		rows: 1,
		features: inputs.len(),
		schema: schema(data),
	};
	let graph = compile_output(&models[N - 1], &prepared, 1, backend, config, data.target.len())?;
	graphs.push(bundle::StoredGraph { graph, inputs, outputs: data.target.clone() });
	let schema = schema(data);
	if let Some(path) = &train.resume {
		bundle::restore(path, &schema, &mut graphs)?;
		eprintln!("resumed: {path}");
	}
	let mut tapes = Vec::with_capacity(N);
	for stored in &graphs {
		let samples = vec![
			0.0;
			stored.inputs.len()
		];
		let targets = vec![
			0.0;
			stored.outputs.len()
		];
		tapes.push(DeviceTape::new(&stored.graph, &samples, &targets, backend)?);
	}
	Ok(State { graphs, tapes, steps: vec![0; N], schema })
}

fn forward(state: &mut State, context: &BTreeMap<String, f64>) -> Result<(BTreeMap<String, f64>, Vec<f64>)> {
	let mut fields = context.clone();
	let mut proposal = Vec::new();
	for index in 0..state.graphs.len() - 1 {
		let sample = values(&state.graphs[index].inputs, &fields)?;
		state.tapes[index].set_samples(&sample)?;
		state.tapes[index].forward()?;
		let output = state.tapes[index].predictions()?;
		proposal.extend_from_slice(&output);
		for (name, value) in state.graphs[index].outputs.iter().cloned().zip(output) {
			fields.insert(name, value);
		}
	}
	Ok((fields, proposal))
}

fn train(
	state: &mut State,
	train: &Train,
	predictor: &Model,
	context: &BTreeMap<String, f64>,
	measurement: &[f64],
	config: Config,
) -> Result<Vec<f64>> {
	let run = RUN.fetch_add(1, Ordering::Relaxed) + 1;
	let mut prediction = Vec::new();
	for epoch in 1..=train.epochs {
		let started = Instant::now();
		let (fields, _) = forward(state, context)?;
		let last = state.graphs.len() - 1;
		let sample = values(&state.graphs[last].inputs, &fields)?;
		state.tapes[last].set_samples(&sample)?;
		state.tapes[last].set_targets(measurement)?;
		state.tapes[last].set_frozen(&state.graphs[last].graph.frozen)?;
		state.steps[last] += 1;
		let (loss, _) = state.tapes[last].epoch(
			state.steps[last],
			train.learning_rate,
			predictor.loss,
			0.0,
			config,
			false,
		)?;
		prediction = state.tapes[last].predictions()?;
		train.print(predictor, run, epoch, loss, measurement, &prediction, started, false);
		let seed = prediction.iter().map(|value| 2.0 * value / prediction.len() as f64).collect::<Vec<_>>();
		state.tapes[last].set_targets(&seed)?;
		let frozen = vec![
			1;
			state.graphs[last].graph.parameters.len()
		];
		state.tapes[last].set_frozen(&frozen)?;
		state.tapes[last].epoch(state.steps[last], train.learning_rate, mse, 0.0, config, true)?;
		let mut gradient = state.graphs[last]
			.inputs
			.iter()
			.cloned()
			.zip(state.tapes[last].input_gradients()?)
			.collect::<BTreeMap<_, _>>();
		for index in (0..last).rev() {
			let seed = state.graphs[index]
				.outputs
				.iter()
				.map(|name| gradient.get(name).copied().unwrap_or(0.0))
				.collect::<Vec<_>>();
			state.tapes[index].set_targets(&seed)?;
			state.tapes[index].set_frozen(&state.graphs[index].graph.frozen)?;
			state.steps[index] += 1;
			state.tapes[index].epoch(state.steps[index], train.learning_rate, mse, 0.0, config, true)?;
			for (name, value) in state.graphs[index]
				.inputs
				.iter()
				.cloned()
				.zip(state.tapes[index].input_gradients()?)
			{
				*gradient.entry(name).or_insert(0.0) += value;
			}
		}
	}
	capture(state)?;
	Ok(prediction)
}

fn capture(state: &mut State) -> Result<()> {
	for (stored, tape) in state.graphs.iter_mut().zip(&state.tapes) {
		stored.graph.parameters = tape.weights(false)?;
	}
	Ok(())
}

impl<const N: usize> RatTrain<N> {
	pub fn every(mut self, command: impl Into<String>) -> Self {
		self.command = Some(command.into());
		self
	}

	pub fn save(mut self, path: impl Into<String>) -> Self {
		self.train.save = Some(path.into());
		self
	}

	pub fn resume(mut self, path: impl Into<String>) -> Self {
		self.train.resume = Some(path.into());
		self
	}

	fn process(&mut self) -> Result<&mut Process> {
		if self.process.is_none() {
			let command = self.command.as_deref().ok_or_else(|| RecipeError::new("RAT requires .every()"))?;
			self.process = Some(Process::spawn(command)?);
		}
		self.process.as_mut().ok_or_else(|| RecipeError::new("RAT command is absent"))
	}

	fn check_interrupt(&mut self, state: Option<&mut State>) -> Result<()> {
		if !INTERRUPTED.load(Ordering::Acquire) {
			return Ok(());
		}
		if let Some(state) = state {
			capture(state)?;
			if let Some(path) = &self.train.save {
				bundle::save(path, &state.schema, &state.graphs)?;
			}
		}
		drop(self.process.take());
		std::process::exit(INTERRUPTED_EXIT)
	}

	pub fn run(&mut self, data: &Data) -> RatReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) });
		self.try_run(data).unwrap_or_else(|error| panic!("{error}"))
	}

	fn try_run(&mut self, data: &Data) -> Result<RatReport> {
		if INTERRUPTED.load(Ordering::Acquire) {
			let mut state = self.state.take();
			self.check_interrupt(state.as_mut())?;
		}
		let context_result = match self.context.take() {
			Some(context) => context,
			None => self.process()?.read()?,
		};
		self.check_interrupt(None)?;
		let context = context_result;
		let backend = device_backend()?;
		let config = Config::load()?;
		if self.state.is_none() {
			self.state = Some(build(&self.models, &self.train, data, backend, config)?);
		}
		let mut state = self.state.take().ok_or_else(|| RecipeError::new("RAT state is absent"))?;
		let proposed = forward(&mut state, &context);
		self.check_interrupt(Some(&mut state))?;
		let (fields, proposal) = proposed?;
		let names = state.graphs[..N - 1]
			.iter()
			.flat_map(|graph| graph.outputs.iter().cloned())
			.collect::<Vec<_>>();
		let written = self.process()?.write(&names, &proposal);
		self.check_interrupt(Some(&mut state))?;
		written?;
		let result = self.process()?.read();
		self.check_interrupt(Some(&mut state))?;
		let result = result?;
		let measurement = values(&data.target, &result)?;
		let trained = train(&mut state, &self.train, &self.models[N - 1], &fields, &measurement, config);
		self.check_interrupt(Some(&mut state))?;
		let prediction = trained?;
		self.context = Some(result);
		if let Some(path) = &self.train.save {
			bundle::save(path, &state.schema, &state.graphs)?;
		}
		self.state = Some(state);
		Ok(RatReport { proposal, prediction, measurement })
	}
}

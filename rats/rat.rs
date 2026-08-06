use super::*;
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

#![cfg_attr(rustfmt, rustfmt::skip)]
//! RAT: B, T and L over an m/n/k lookup table, one slot per contraction node and direction.
//!
//! Tiles reach the runtime through the #378 schedule cache. Recipe rewrites that file when it
//! refuses an extent, which doubles as the validity oracle: a file that survives a run is the
//! file that ran.
//!
//! The workload is an ordinary training script. It runs untouched, is timed from the outside,
//! and is judged by the model it saves.
//!
//! Environment: RECIPE_BIN, RAT_SCRIPT, RAT_MODEL, RAT_LOG, RAT_BUDGET, RAT_REPEATS,
//! RAT_EXPLORE, RAT_HIDDEN, RAT_RATE, RAT_SEED.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The lookup table. 5 x 5 x 5 = 125 cells, which is the action space T chooses from.
const M: [u32; 5] = [16, 32, 64, 128, 256];
const N: [u32; 5] = [4, 8, 16, 32, 64];
const K: [u32; 5] = [8, 16, 32, 64, 128];
const ACTIONS: usize = M.len() * N.len() * K.len();

/// Width of the state vector and of the tile-extent vector the nets consume.
const STATE: usize = 7;
const EXTENT: usize = 3;

/// FNV-1a, for the model fingerprint and for picking each slot's neurons.
const PRIME: u64 = 0x100000001b3;
const OFFSET: u64 = 0xcbf29ce484222325;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Tile {
	m: u32,
	n: u32,
	k: u32,
}

/// One tuneable slot: a contraction node and one of its three directions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
	node: usize,
	direction: usize,
}

fn cell(action: usize) -> Tile {
	let k = action % K.len();
	let n = action / K.len() % N.len();
	let m = action / (K.len() * N.len());

	Tile { m: M[m], n: N[n], k: K[k] }
}

/// Log-scaled, so the nets see ratios rather than magnitudes.
fn scaled(tile: Tile) -> [f64; EXTENT] {
	[
		f64::from(tile.m).log2() / 8.0,
		f64::from(tile.n).log2() / 8.0,
		f64::from(tile.k).log2() / 8.0,
	]
}

/// Everything the nets see is publicly observable: the slot identity and the extent Recipe
/// derived for that slot.
fn state(slot: Slot, nodes: usize, heuristic: Tile) -> [f64; STATE] {
	let extent = scaled(heuristic);

	[
		slot.node as f64 / nodes as f64,
		f64::from(slot.direction == 0),
		f64::from(slot.direction == 1),
		f64::from(slot.direction == 2),
		extent[0],
		extent[1],
		extent[2],
	]
}

fn joined(state: &[f64; STATE], extent: [f64; EXTENT]) -> Vec<f64> {
	state.iter().copied().chain(extent).collect()
}

fn softmax(logits: &[f64]) -> Vec<f64> {
	let peak = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let raw = logits.iter().map(|value| (value - peak).exp()).collect::<Vec<_>>();
	let total = raw.iter().sum::<f64>();

	raw.into_iter().map(|value| value / total).collect()
}

fn digest(bytes: &[u8]) -> u64 {
	let mut hash = OFFSET;

	for byte in bytes {
		hash = (hash ^ u64::from(*byte)).wrapping_mul(PRIME);
	}

	hash
}

struct Random(u64);

impl Random {
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}

	fn unit(&mut self) -> f64 {
		(self.next() >> 11) as f64 / (1_u64 << 53) as f64
	}

	fn symmetric(&mut self) -> f64 {
		self.unit() * 2.0 - 1.0
	}

	fn below(&mut self, bound: usize) -> usize {
		(self.next() % bound as u64) as usize
	}
}

/// One hidden tanh layer.
struct Net {
	inputs: usize,
	units: usize,
	outputs: usize,
	w1: Vec<f64>,
	b1: Vec<f64>,
	w2: Vec<f64>,
	b2: Vec<f64>,
}

impl Net {
	fn new(inputs: usize, units: usize, outputs: usize, random: &mut Random) -> Self {
		let scale = (1.0 / inputs as f64).sqrt();

		Self {
			inputs,
			units,
			outputs,
			w1: (0..inputs * units).map(|_| random.symmetric() * scale).collect(),
			b1: vec![0.0; units],
			w2: (0..units * outputs).map(|_| random.symmetric() * scale).collect(),
			b2: vec![0.0; outputs],
		}
	}

	/// Returns the hidden activations and the output, because backward needs both.
	fn forward(&self, input: &[f64]) -> (Vec<f64>, Vec<f64>) {
		assert_eq!(input.len(), self.inputs, "net input width");

		let mut hidden = vec![0.0; self.units];

		for unit in 0..self.units {
			let mut sum = self.b1[unit];

			for (index, feature) in input.iter().enumerate() {
				sum += self.w1[index * self.units + unit] * feature;
			}

			hidden[unit] = sum.tanh();
		}

		let mut output = vec![0.0; self.outputs];

		for index in 0..self.outputs {
			let mut sum = self.b2[index];

			for (unit, value) in hidden.iter().enumerate() {
				sum += self.w2[unit * self.outputs + index] * value;
			}

			output[index] = sum;
		}

		(hidden, output)
	}

	/// `selected` masks which units this slot may write, which is the update-only-selected-neurons
	/// rule. `frozen` returns the input gradient and writes nothing, so a caller can chain through
	/// a frozen net. Returns the gradient with respect to the input either way.
	fn backward(
		&mut self,
		input: &[f64],
		hidden: &[f64],
		gradient: &[f64],
		rate: f64,
		selected: &[bool],
		frozen: bool,
	) -> Vec<f64> {
		let mut inner = vec![0.0; self.units];

		for unit in 0..self.units {
			let mut sum = 0.0;

			for index in 0..self.outputs {
				sum += self.w2[unit * self.outputs + index] * gradient[index];
			}

			inner[unit] = sum * (1.0 - hidden[unit] * hidden[unit]);
		}

		let mut source = vec![0.0; self.inputs];

		for index in 0..self.inputs {
			let mut sum = 0.0;

			for unit in 0..self.units {
				sum += self.w1[index * self.units + unit] * inner[unit];
			}

			source[index] = sum;
		}

		if frozen {
			return source;
		}

		for index in 0..self.outputs {
			for unit in 0..self.units {
				if selected[unit] {
					self.w2[unit * self.outputs + index] -= rate * gradient[index] * hidden[unit];
				}
			}

			self.b2[index] -= rate * gradient[index];
		}

		for unit in 0..self.units {
			if !selected[unit] {
				continue;
			}

			for (index, feature) in input.iter().enumerate() {
				self.w1[index * self.units + unit] -= rate * inner[unit] * feature;
			}

			self.b1[unit] -= rate * inner[unit];
		}

		source
	}
}

/// L: per slot, the B and T units it may write and every cell it has measured.
struct Lookup {
	b: BTreeMap<Slot, Vec<bool>>,
	t: BTreeMap<Slot, Vec<bool>>,
	measured: BTreeMap<Slot, Vec<(usize, f64)>>,
}

impl Lookup {
	/// A deterministic half of the units, so one slot always writes the same subgraph and
	/// neighbouring slots overlap only partly.
	fn select(slot: Slot, units: usize, salt: u64) -> Vec<bool> {
		let mut seed = OFFSET ^ salt;

		for part in [slot.node as u64, slot.direction as u64] {
			seed = (seed ^ part).wrapping_mul(PRIME);
		}

		(0..units)
			.map(|unit| (seed ^ unit as u64).wrapping_mul(PRIME) >> 60 < 8)
			.collect()
	}

	fn new(slots: &[Slot], units: usize) -> Self {
		let mut lookup = Self {
			b: BTreeMap::new(),
			t: BTreeMap::new(),
			measured: BTreeMap::new(),
		};

		for slot in slots {
			lookup.b.insert(*slot, Self::select(*slot, units, 0x5b));
			lookup.t.insert(*slot, Self::select(*slot, units, 0x7d));
			lookup.measured.insert(*slot, Vec::new());
		}

		lookup
	}

	fn store(&mut self, slot: Slot, action: usize, seconds: f64) {
		self.measured.entry(slot).or_default().push((action, seconds));
	}

	fn seen(&self, slot: Slot, action: usize) -> bool {
		self.measured[&slot].iter().any(|(seen, _)| *seen == action)
	}
}

/// The schedule file #378 reads: a device line, then one line per contraction node carrying the
/// forward, gradient and previous extents.
#[derive(Clone)]
struct Schedule {
	device: String,
	nodes: Vec<(usize, [Tile; 3])>,
}

impl Schedule {
	fn parse(text: &str) -> Self {
		let mut lines = text.lines();
		let device = lines.next().expect("schedule cache is empty").to_owned();

		assert!(device.starts_with("device "), "schedule cache has no device line");

		let mut nodes = Vec::new();

		for line in lines.filter(|line| !line.trim().is_empty()) {
			let fields = line.split_whitespace().collect::<Vec<_>>();

			assert!(fields.len() == 11 && fields[0] == "node", "unexpected schedule line: {line}");

			let node = fields[1].parse().expect("node index");
			let at = |index: usize| fields[index].parse::<u32>().expect("tile extent");

			nodes.push((node, [
				Tile { m: at(2), n: at(3), k: at(4) },
				Tile { m: at(5), n: at(6), k: at(7) },
				Tile { m: at(8), n: at(9), k: at(10) },
			]));
		}

		Self { device, nodes }
	}

	fn render(&self) -> String {
		let mut text = format!("{}\n", self.device);

		for (node, tiles) in &self.nodes {
			let extents = tiles
				.iter()
				.flat_map(|tile| [tile.m, tile.n, tile.k])
				.map(|extent| extent.to_string())
				.collect::<Vec<_>>()
				.join(" ");

			let _ = writeln!(text, "node {node} {extents}");
		}

		text
	}

	fn get(&self, slot: Slot) -> Tile {
		let entry = self.nodes.iter().find(|(node, _)| *node == slot.node).expect("slot node");

		entry.1[slot.direction]
	}

	fn set(&mut self, slot: Slot, tile: Tile) {
		let entry = self.nodes.iter_mut().find(|(node, _)| *node == slot.node).expect("slot node");

		entry.1[slot.direction] = tile;
	}

	fn slots(&self) -> Vec<Slot> {
		let mut slots = Vec::new();

		for (node, _) in &self.nodes {
			for direction in 0..3 {
				slots.push(Slot { node: *node, direction });
			}
		}

		slots
	}
}

struct Measurement {
	seconds: f64,
	model: u64,
	dispatched: bool,
}

struct Bench {
	binary: PathBuf,
	script: PathBuf,
	cache: PathBuf,
	model: PathBuf,
	repeats: usize,
}

impl Bench {
	/// Times the whole invocation. Process startup and data preparation are a constant across
	/// candidates, so they shift every measurement equally rather than favouring one schedule.
	/// The saved model is the numerical fingerprint.
	fn run(&self, debug: bool) -> (f64, u64) {
		let mut command = Command::new(&self.binary);

		command.arg(&self.script);

		if debug {
			command.env("RECIPE_DEBUG", "1");
		} else {
			command.env_remove("RECIPE_DEBUG");
		}

		std::fs::remove_file(&self.model).ok();

		let started = std::time::Instant::now();
		let output = command.output().expect("cannot execute recipe");
		let seconds = started.elapsed().as_secs_f64();

		assert!(
			output.status.success(),
			"workload failed: {}",
			String::from_utf8_lossy(&output.stderr)
		);

		let saved = std::fs::read(&self.model).expect("workload saved no model");

		(seconds, digest(&saved))
	}

	/// Dispatches one assignment and measures it. The written file is read back afterwards:
	/// Recipe rewrites it when it refuses an extent, so a surviving file proves the timed run
	/// used this schedule.
	fn measure(&self, schedule: &Schedule) -> Measurement {
		let written = schedule.render();
		let mut seconds = f64::INFINITY;
		let mut model = 0;

		for _ in 0..self.repeats {
			std::fs::write(&self.cache, &written).expect("cannot write schedule cache");

			let (elapsed, observed) = self.run(false);
			let after = std::fs::read_to_string(&self.cache).expect("cannot read schedule cache");

			if after != written {
				return Measurement { seconds: f64::INFINITY, model: observed, dispatched: false };
			}

			seconds = seconds.min(elapsed);
			model = observed;
		}

		Measurement { seconds, model, dispatched: true }
	}
}

fn variable<T: std::str::FromStr>(name: &str, fallback: T) -> T {
	std::env::var(name).map_or(fallback, |value| value.parse().ok().expect("invalid environment value"))
}

fn path(name: &str, fallback: &str) -> PathBuf {
	PathBuf::from(std::env::var(name).unwrap_or_else(|_| fallback.to_owned()))
}

/// Recipe names the schedule file it used in the debug log, on a cache hit and on a fresh
/// selection alike. Only the training tape ever tunes, so only one path appears.
fn locate(log: &Path) -> PathBuf {
	let text = std::fs::read_to_string(log).expect("cannot read recipe.log");
	let mut found = Vec::new();

	for line in text.lines() {
		let field = if line.starts_with("schedule cache hit") {
			"path="
		} else if line.starts_with("schedule select") {
			"cache="
		} else {
			continue;
		};

		if let Some(value) = line.split_whitespace().find_map(|token| token.strip_prefix(field)) {
			found.push(PathBuf::from(value));
		}
	}

	assert_eq!(found.len(), 1, "expected exactly one schedule cache in the log, found {found:?}");

	found.remove(0)
}

fn main() {
	let budget: usize = variable("RAT_BUDGET", 120);
	let repeats: usize = variable("RAT_REPEATS", 3);
	let explore: f64 = variable("RAT_EXPLORE", 0.25);
	let rate: f64 = variable("RAT_RATE", 0.02);
	let units: usize = variable("RAT_HIDDEN", 24);
	let log = path("RAT_LOG", "recipe.log");
	let mut random = Random(variable("RAT_SEED", 17_u64) | 1);

	// The first run compiles the artifact and names the schedule file in the log. Clearing that
	// file and running again makes the baseline Recipe's own selection on a cold cache, rather
	// than whatever a previous session left behind.
	println!("bootstrap: compiling the workload and reading Recipe's own schedule");

	let bench = Bench {
		binary: path("RECIPE_BIN", "target/release/recipe"),
		script: path("RAT_SCRIPT", "experiment/vna.rs"),
		cache: PathBuf::new(),
		model: path("RAT_MODEL", "vna.ogdl"),
		repeats,
	};

	bench.run(true);

	let cache = locate(&log);

	std::fs::remove_file(&cache).expect("cannot clear schedule cache");

	let bench = Bench { cache, ..bench };

	bench.run(true);

	let text = std::fs::read_to_string(&bench.cache).expect("cannot read schedule cache");
	let heuristic = Schedule::parse(&text);

	println!("bootstrap: cache {}", bench.cache.display());
	println!("bootstrap: {}", heuristic.device);

	let slots = heuristic.slots();
	let mut lookup = Lookup::new(&slots, units);
	let mut b = Net::new(STATE + EXTENT, units, 1, &mut random);
	let mut t = Net::new(STATE, units, ACTIONS, &mut random);

	let base = bench.measure(&heuristic);

	assert!(base.dispatched, "Recipe rejected its own schedule");

	let reference = base.model;
	let mut best = heuristic.clone();
	let mut fastest = base.seconds;

	println!("baseline: {fastest:.3} s, model {reference:#018x}\n");
	println!(
		"{:>4}  {:>4}  {:>3}  {:>5} {:>4} {:>5}  {:>9}  {:>9}  {}",
		"step", "node", "dir", "m", "n", "k", "P", "M", "verdict"
	);

	let mut rejected = 0;
	let mut changed = Vec::new();

	for step in 0..budget {
		let slot = slots[step % slots.len()];
		let state = state(slot, heuristic.nodes.len(), heuristic.get(slot));

		// state -> T -> action, with R exploring the cells T has not learned yet.
		let (proposal, logits) = t.forward(&state);
		let distribution = softmax(&logits);

		let action = if random.unit() < explore {
			random.below(ACTIONS)
		} else {
			(0..ACTIONS)
				.filter(|cell| !lookup.seen(slot, *cell))
				.max_by(|left, right| distribution[*left].total_cmp(&distribution[*right]))
				.unwrap_or(0)
		};

		if lookup.seen(slot, action) {
			continue;
		}

		let tile = cell(action);

		// state + action -> B -> P
		let input = joined(&state, scaled(tile));
		let (hidden, prediction) = b.forward(&input);

		// state + action -> benchmark -> M
		let mut candidate = best.clone();

		candidate.set(slot, tile);

		let measurement = bench.measure(&candidate);
		let measured = if measurement.dispatched { measurement.seconds } else { f64::INFINITY };

		lookup.store(slot, action, measured);
		rejected += usize::from(!measurement.dispatched);

		// difference(P, M) -> backward -> the selected B units. A refused extent is a signal too,
		// priced above anything measurable.
		let target = if measured.is_finite() { measured } else { fastest * 4.0 };
		let error = prediction[0] - target;

		b.backward(&input, &hidden, &[error], rate, &lookup.b[&slot], false);

		// objective(P) -> backward through frozen B -> the selected T units. The softmax forms an
		// expected action, B scores that action, and the gradient of the score reaches T's logits.
		// B is not written here.
		let mut average = [0.0; EXTENT];

		for index in 0..ACTIONS {
			let extent = scaled(cell(index));

			for axis in 0..EXTENT {
				average[axis] += distribution[index] * extent[axis];
			}
		}

		let expected = joined(&state, average);
		let (scored, _) = b.forward(&expected);
		let source = b.backward(&expected, &scored, &[1.0], rate, &lookup.b[&slot], true);

		let mut direct = vec![0.0; ACTIONS];

		for index in 0..ACTIONS {
			let extent = scaled(cell(index));

			for axis in 0..EXTENT {
				direct[index] += source[STATE + axis] * extent[axis];
			}
		}

		let mut mean = 0.0;

		for index in 0..ACTIONS {
			mean += distribution[index] * direct[index];
		}

		let mut gradient = vec![0.0; ACTIONS];

		for index in 0..ACTIONS {
			gradient[index] = distribution[index] * (direct[index] - mean);
		}

		t.backward(&state, &proposal, &gradient, rate, &lookup.t[&slot], false);

		// A schedule that trains to a different model is not a scheduling choice. Recipe's own
		// gate cannot see this, because it compares a loss taken before the reverse pass runs.
		let verdict = if !measurement.dispatched {
			"rejected"
		} else if measurement.model != reference {
			changed.push((slot, tile, measurement.model));
			"changes the trained model"
		} else if measured < fastest {
			best = candidate;
			fastest = measured;
			"accepted"
		} else {
			"slower"
		};

		println!(
			"{step:>4}  {:>4}  {:>3}  {:>5} {:>4} {:>5}  {:>9.3}  {:>9.3}  {verdict}",
			slot.node,
			slot.direction,
			tile.m,
			tile.n,
			tile.k,
			prediction[0],
			if measured.is_finite() { measured } else { f64::NAN }
		);
	}

	std::fs::write(&bench.cache, best.render()).expect("cannot write schedule cache");

	println!("\nselected schedule, written to {}", bench.cache.display());
	print!("{}", best.render());
	println!("\nbaseline {:.3} s, selected {fastest:.3} s", base.seconds);
	println!("{rejected} of {budget} cells rejected by the compiled resource bounds");
	println!("{} cells changed the trained model:", changed.len());

	for (slot, tile, model) in &changed {
		println!(
			"  node {} dir {}  {}x{}x{}  model {model:#018x} against {reference:#018x}",
			slot.node, slot.direction, tile.m, tile.n, tile.k
		);
	}
}

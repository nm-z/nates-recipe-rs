//! The RAT: R/P/M/B/T/L, per contraction node and direction, measured on the
//! real fused epoch of a real model on a real device.
//!
//! This is the design from the issue thread, built as one script on top of the
//! runtime-schedule plumbing in #378. It replaces that PR's selector (an
//! XGBoost regressor plus argmin, which is B alone) with the two-model loop:
//!
//!   state -> L                       lookup state: selected neurons, memory
//!   L + state -> T -> action         tile proposer picks an LUT cell
//!   L + state + action -> B -> P     benchmark surrogate predicts seconds
//!   state + action -> benchmark -> M measured seconds, real epochs
//!   M -> L                           stored
//!   difference(P, M) -> backward     updates the selected B neurons
//!   objective(P)     -> backward     through frozen B, updates selected T neurons
//!
//! T's discrete choice is made differentiable the same way Recipe's own RAT
//! does it: the softmax over LUT cells forms an expected action embedding, B
//! scores that, and the gradient of the score reaches T's logits. The argmax
//! cell is what actually gets benchmarked.
//!
//! Dispatch channel. Recipe has no public tile knob, but #378 reads a schedule
//! cache beside the native artifact before the first epoch, and a cache hit
//! short-circuits its own tuner. Writing that file is therefore a supported way
//! to dispatch an arbitrary assignment, and it doubles as a validity oracle: if
//! an extent violates a compiled resource bound, Recipe rejects the whole file
//! and retunes, overwriting it. A file that survives a run was dispatched.
//!
//! Numerics. #378 accepts a candidate when the epoch loss is unchanged, which
//! is measured before the reverse pass and so cannot see a gradient tile at
//! all. This script instead compares the trained model itself, over a long
//! enough run for a difference to surface, and reports every cell that changes
//! it.
//!
//! Run:
//!   cargo build --release
//!   ./target/release/recipe experiment/rat.rs
//!
//! Environment: RECIPE_BIN, VNA_DATA, RAT_BUDGET, RAT_MEASURE_EPOCHS,
//! RAT_REPEATS, RAT_CHECK_EPOCHS, RAT_EXPLORE, RAT_SEED, RAT_HIDDEN.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------- LUT

const TILE_M: [u32; 5] = [16, 32, 64, 128, 256];
const TILE_N: [u32; 5] = [4, 8, 16, 32, 64];
const TILE_K: [u32; 5] = [8, 16, 32, 64, 128];
const ACTIONS: usize = TILE_M.len() * TILE_N.len() * TILE_K.len();

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Tile {
	m: u32,
	n: u32,
	k: u32,
}

fn action_tile(action: usize) -> Tile {
	let k = action % TILE_K.len();
	let n = (action / TILE_K.len()) % TILE_N.len();
	let m = action / (TILE_K.len() * TILE_N.len());
	Tile { m: TILE_M[m], n: TILE_N[n], k: TILE_K[k] }
}

/// Log-scaled tile extents, so the models see ratios rather than magnitudes.
fn tile_features(tile: Tile) -> [f64; 3] { [f64::from(tile.m).log2() / 8.0, f64::from(tile.n).log2() / 8.0, f64::from(tile.k).log2() / 8.0] }

// ---------------------------------------------------------------- state

const STATE: usize = 7;
const ACTION_FEATURES: usize = 3;

/// One tuneable slot: a contraction node and one of its three directions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Slot {
	node: usize,
	direction: usize,
}

/// What the models see. Everything here is publicly observable: the slot
/// identity and the heuristic extent Recipe derived for it.
fn state_features(slot: Slot, nodes: usize, heuristic: Tile) -> [f64; STATE] {
	let tile = tile_features(heuristic);
	[
		slot.node as f64 / nodes.max(1) as f64,
		f64::from(slot.direction == 0),
		f64::from(slot.direction == 1),
		f64::from(slot.direction == 2),
		tile[0],
		tile[1],
		tile[2],
	]
}

// ---------------------------------------------------------------- nets

/// A one-hidden-layer tanh network. Small on purpose: the point is the loop,
/// not the capacity.
struct Net {
	inputs: usize,
	hidden: usize,
	outputs: usize,
	w1: Vec<f64>,
	b1: Vec<f64>,
	w2: Vec<f64>,
	b2: Vec<f64>,
}

struct Pass {
	hidden: Vec<f64>,
	output: Vec<f64>,
}

impl Net {
	fn new(inputs: usize, hidden: usize, outputs: usize, random: &mut Random) -> Self {
		let scale = (1.0 / inputs as f64).sqrt();
		Self {
			inputs,
			hidden,
			outputs,
			w1: (0..inputs * hidden).map(|_| random.symmetric() * scale).collect(),
			b1: vec![0.0; hidden],
			w2: (0..hidden * outputs).map(|_| random.symmetric() * scale).collect(),
			b2: vec![0.0; outputs],
		}
	}

	fn forward(&self, input: &[f64]) -> Pass {
		assert_eq!(input.len(), self.inputs, "net input width");
		let mut hidden = vec![0.0; self.hidden];
		for (unit, value) in hidden.iter_mut().enumerate() {
			let mut sum = self.b1[unit];
			for (index, feature) in input.iter().enumerate() {
				sum += self.w1[index * self.hidden + unit] * feature;
			}
			*value = sum.tanh();
		}
		let mut output = vec![0.0; self.outputs];
		for (index, value) in output.iter_mut().enumerate() {
			let mut sum = self.b2[index];
			for (unit, activation) in hidden.iter().enumerate() {
				sum += self.w2[unit * self.outputs + index] * activation;
			}
			*value = sum;
		}
		Pass { hidden, output }
	}

	/// Backward pass. `selected` masks which hidden units may be written, which
	/// is the "update only selected neurons" rule. Returns the gradient with
	/// respect to the input, so a caller can chain through a frozen net.
	fn backward(&mut self, input: &[f64], pass: &Pass, grad_output: &[f64], rate: f64, selected: &[bool], frozen: bool) -> Vec<f64> {
		let mut grad_hidden = vec![0.0; self.hidden];
		for unit in 0..self.hidden {
			let mut sum = 0.0;
			for index in 0..self.outputs {
				sum += self.w2[unit * self.outputs + index] * grad_output[index];
			}
			grad_hidden[unit] = sum * (1.0 - pass.hidden[unit] * pass.hidden[unit]);
		}
		let mut grad_input = vec![0.0; self.inputs];
		for index in 0..self.inputs {
			let mut sum = 0.0;
			for unit in 0..self.hidden {
				sum += self.w1[index * self.hidden + unit] * grad_hidden[unit];
			}
			grad_input[index] = sum;
		}
		if frozen {
			return grad_input;
		}
		for index in 0..self.outputs {
			for unit in 0..self.hidden {
				if selected[unit] {
					self.w2[unit * self.outputs + index] -= rate * grad_output[index] * pass.hidden[unit];
				}
			}
			self.b2[index] -= rate * grad_output[index];
		}
		for unit in 0..self.hidden {
			if !selected[unit] {
				continue;
			}
			for (index, feature) in input.iter().enumerate() {
				self.w1[index * self.hidden + unit] -= rate * grad_hidden[unit] * feature;
			}
			self.b1[unit] -= rate * grad_hidden[unit];
		}
		grad_input
	}
}

fn softmax(logits: &[f64]) -> Vec<f64> {
	let peak = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
	let raw = logits.iter().map(|value| (value - peak).exp()).collect::<Vec<_>>();
	let total = raw.iter().sum::<f64>();
	raw.into_iter().map(|value| value / total).collect()
}

// ---------------------------------------------------------------- L

/// Lookup state. One entry per slot: which B and T hidden units this slot is
/// allowed to write, and every measurement taken for it.
struct Lookup {
	selected_b: BTreeMap<Slot, Vec<bool>>,
	selected_t: BTreeMap<Slot, Vec<bool>>,
	measured: BTreeMap<Slot, Vec<(usize, f64)>>,
}

impl Lookup {
	fn new() -> Self { Self { selected_b: BTreeMap::new(), selected_t: BTreeMap::new(), measured: BTreeMap::new() } }

	/// Half the units, chosen by a hash of the slot. Deterministic, so the same
	/// slot always writes the same subgraph, and different slots overlap only
	/// partially.
	fn select(slot: Slot, hidden: usize, salt: u64) -> Vec<bool> {
		let mut hash = 0xcbf29ce484222325_u64 ^ salt;
		for byte in [slot.node as u64, slot.direction as u64] {
			hash = (hash ^ byte).wrapping_mul(0x100000001b3);
		}
		(0..hidden)
			.map(|unit| {
				let mut local = hash;
				local = (local ^ unit as u64).wrapping_mul(0x100000001b3);
				local >> 60 < 8
			})
			.collect()
	}

	fn enter(&mut self, slot: Slot, hidden: usize) {
		self.selected_b.entry(slot).or_insert_with(|| Self::select(slot, hidden, 0x5b));
		self.selected_t.entry(slot).or_insert_with(|| Self::select(slot, hidden, 0x7d));
		self.measured.entry(slot).or_default();
	}

	fn store(&mut self, slot: Slot, action: usize, measured: f64) { self.measured.entry(slot).or_default().push((action, measured)); }

	fn seen(&self, slot: Slot, action: usize) -> bool { self.measured.get(&slot).is_some_and(|rows| rows.iter().any(|(seen, _)| *seen == action)) }
}

// ---------------------------------------------------------------- random

struct Random(u64);

impl Random {
	fn next(&mut self) -> u64 {
		self.0 ^= self.0 << 13;
		self.0 ^= self.0 >> 7;
		self.0 ^= self.0 << 17;
		self.0
	}
	fn unit(&mut self) -> f64 { (self.next() >> 11) as f64 / (1_u64 << 53) as f64 }
	fn symmetric(&mut self) -> f64 { self.unit() * 2.0 - 1.0 }
	fn below(&mut self, bound: usize) -> usize { (self.next() % bound as u64) as usize }
}

// ---------------------------------------------------------------- schedule cache

/// The schedule file #378 reads: a device line, then one line per contraction
/// node holding forward, gradient and previous extents.
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
		let nodes = lines
			.filter(|line| !line.trim().is_empty())
			.map(|line| {
				let fields = line.split_whitespace().collect::<Vec<_>>();
				assert!(fields.len() == 11 && fields[0] == "node", "unexpected schedule line: {line}");
				let node = fields[1].parse().expect("node index");
				let value = |index: usize| fields[index].parse::<u32>().expect("tile extent");
				(node, [
					Tile { m: value(2), n: value(3), k: value(4) },
					Tile { m: value(5), n: value(6), k: value(7) },
					Tile { m: value(8), n: value(9), k: value(10) },
				])
			})
			.collect();
		Self { device, nodes }
	}

	fn render(&self) -> String {
		let mut text = format!("{}\n", self.device);
		for (node, tiles) in &self.nodes {
			let extents = tiles.iter().flat_map(|tile| [tile.m, tile.n, tile.k]).map(|extent| extent.to_string()).collect::<Vec<_>>().join(" ");
			let _ = writeln!(text, "node {node} {extents}");
		}
		text
	}

	fn get(&self, slot: Slot) -> Tile { self.nodes.iter().find(|(node, _)| *node == slot.node).expect("slot node").1[slot.direction] }

	fn set(&mut self, slot: Slot, tile: Tile) {
		let entry = self.nodes.iter_mut().find(|(node, _)| *node == slot.node).expect("slot node");
		entry.1[slot.direction] = tile;
	}

	fn slots(&self) -> Vec<Slot> { self.nodes.iter().flat_map(|(node, _)| (0..3).map(move |direction| Slot { node: *node, direction })).collect() }
}

// ---------------------------------------------------------------- measurement

struct Measurement {
	seconds: f64,
	digest: String,
	dispatched: bool,
}

struct Bench {
	binary: PathBuf,
	script: PathBuf,
	cache: PathBuf,
	repeats: usize,
}

impl Bench {
	fn run(&self, epochs: usize, debug: bool) -> (f64, String) {
		let mut command = Command::new(&self.binary);
		command.arg(&self.script).env("VNA_EPOCHS", epochs.to_string());
		if debug {
			command.env("RECIPE_DEBUG", "1");
		} else {
			command.env_remove("RECIPE_DEBUG");
		}
		let output = command.output().expect("cannot execute recipe");
		assert!(output.status.success(), "workload failed: {}", String::from_utf8_lossy(&output.stderr));
		let text = String::from_utf8_lossy(&output.stdout).into_owned();
		let field = |name: &str| {
			text.lines()
				.find_map(|line| line.strip_prefix(name).map(|rest| rest.trim().to_owned()))
				.unwrap_or_else(|| panic!("workload printed no {name}: {text}"))
		};
		(field("epoch_seconds ").parse().expect("epoch_seconds"), field("prediction_digest "))
	}

	/// Dispatches one assignment and measures it. The written file is read back
	/// afterwards: Recipe rewrites it when it rejects an extent, so a surviving
	/// file is proof the measured epochs actually used this schedule.
	fn measure(&self, schedule: &Schedule, epochs: usize) -> Measurement {
		let written = schedule.render();
		let mut best = f64::INFINITY;
		let mut digest = String::new();
		for _ in 0..self.repeats {
			std::fs::write(&self.cache, &written).expect("cannot write schedule cache");
			let (seconds, observed) = self.run(epochs, false);
			if std::fs::read_to_string(&self.cache).expect("cannot read schedule cache") != written {
				return Measurement { seconds: f64::INFINITY, digest: observed, dispatched: false };
			}
			best = best.min(seconds);
			digest = observed;
		}
		Measurement { seconds: best, digest, dispatched: true }
	}
}

// ---------------------------------------------------------------- main

fn variable<T: std::str::FromStr>(name: &str, fallback: T) -> T {
	std::env::var(name).map_or(fallback, |value| value.parse().ok().expect("invalid environment value"))
}

/// Recipe names the schedule file it used in the debug log, on a cache hit and
/// on a fresh selection alike. Reading it back is exact, and it identifies the
/// training tape specifically: the evaluation tape never tunes and so never
/// appears here.
fn locate_cache(log: &Path) -> PathBuf {
	let text = std::fs::read_to_string(log).expect("cannot read recipe.log; is RECIPE_DEBUG reaching the workload?");
	let found = text
		.lines()
		.filter_map(|line| {
			let field = if line.starts_with("schedule cache hit") {
				line.split_whitespace().find_map(|token| token.strip_prefix("path="))
			} else if line.starts_with("schedule select") {
				line.split_whitespace().find_map(|token| token.strip_prefix("cache="))
			} else {
				None
			};
			field.map(PathBuf::from)
		})
		.collect::<Vec<_>>();
	assert_eq!(found.len(), 1, "expected exactly one schedule cache in the log, found {found:?}");
	found.into_iter().next().expect("schedule cache")
}

fn main() {
	let binary = PathBuf::from(std::env::var("RECIPE_BIN").unwrap_or_else(|_| "target/release/recipe".to_owned()));
	let script = PathBuf::from(std::env::var("RAT_SCRIPT").unwrap_or_else(|_| "experiment/vna.rs".to_owned()));
	let log = PathBuf::from(std::env::var("RAT_LOG").unwrap_or_else(|_| "recipe.log".to_owned()));
	let budget: usize = variable("RAT_BUDGET", 120);
	let measure_epochs: usize = variable("RAT_MEASURE_EPOCHS", 40);
	let check_epochs: usize = variable("RAT_CHECK_EPOCHS", 300);
	let repeats: usize = variable("RAT_REPEATS", 3);
	let explore: f64 = variable("RAT_EXPLORE", 0.25);
	let hidden: usize = variable("RAT_HIDDEN", 24);
	let rate: f64 = variable("RAT_RATE", 0.02);
	let mut random = Random(variable("RAT_SEED", 17_u64) | 1);

	// Bootstrap. Let Recipe compile the artifact and write its own schedule, so
	// the node set, the device line and the heuristic extents all come from the
	// runtime rather than from an assumption here.
	println!("bootstrap: compiling the workload and reading Recipe's own schedule");
	let bench = Bench { binary, script, cache: PathBuf::new(), repeats };
	bench.run(measure_epochs, true);
	let cache = locate_cache(&log);
	let bench = Bench { cache, ..bench };
	let heuristic = Schedule::parse(&std::fs::read_to_string(&bench.cache).expect("cannot read schedule cache"));
	println!("bootstrap: cache {}", bench.cache.display());
	println!("bootstrap: {}", heuristic.device);

	// Reference: what the model trains to under the schedule Recipe picked, at
	// a length where a numerical difference has room to appear.
	let reference = bench.measure(&heuristic, check_epochs);
	assert!(reference.dispatched, "Recipe rejected its own schedule");
	println!("reference: {check_epochs} epochs, digest {}", reference.digest);

	let slots = heuristic.slots();
	let mut lookup = Lookup::new();
	let mut b = Net::new(STATE + ACTION_FEATURES, hidden, 1, &mut random);
	let mut t = Net::new(STATE, hidden, ACTIONS, &mut random);
	for slot in &slots {
		lookup.enter(*slot, hidden);
	}

	let base = bench.measure(&heuristic, measure_epochs);
	assert!(base.dispatched, "Recipe rejected its own schedule");
	let mut best = heuristic.clone();
	let mut best_seconds = base.seconds;
	println!("baseline: {:.6} s over {measure_epochs} epochs\n", best_seconds);

	let mut rejected = Vec::new();
	let mut broke_numerics = Vec::new();

	for step in 0..budget {
		let slot = slots[step % slots.len()];
		let state = state_features(slot, heuristic.nodes.len(), heuristic.get(slot));

		// state -> T -> action, with R exploring the cells T has not learned yet.
		let proposal = t.forward(&state);
		let distribution = softmax(&proposal.output);
		let action = if random.unit() < explore {
			random.below(ACTIONS)
		} else {
			let mut choice = 0;
			for candidate in 0..ACTIONS {
				if distribution[candidate] > distribution[choice] && !lookup.seen(slot, candidate) {
					choice = candidate;
				}
			}
			choice
		};
		if lookup.seen(slot, action) {
			continue;
		}
		let tile = action_tile(action);

		// state + action -> B -> P
		let mut input = state.to_vec();
		input.extend(tile_features(tile));
		let prediction = b.forward(&input);
		let predicted = prediction.output[0];

		// state + action -> benchmark -> M
		let mut candidate = best.clone();
		candidate.set(slot, tile);
		let measurement = bench.measure(&candidate, measure_epochs);
		let measured = if measurement.dispatched { measurement.seconds } else { f64::INFINITY };
		lookup.store(slot, action, measured);
		if !measurement.dispatched {
			rejected.push((slot, tile));
		}

		// difference(P, M) -> backward -> update selected B neurons.
		// A rejected extent is a real signal too: it teaches B that this cell is
		// expensive, using a value well above anything measurable.
		let target = if measured.is_finite() { measured } else { best_seconds * 4.0 };
		let error = predicted - target;
		b.backward(&input, &prediction, &[error], rate, &lookup.selected_b[&slot], false);

		// objective(P) -> backward through frozen B -> update selected T neurons.
		// The softmax forms an expected action, B scores it, and the gradient of
		// that score reaches T's logits. B is not written here.
		let mut expected = state.to_vec();
		expected.extend([0.0; ACTION_FEATURES]);
		for cell in 0..ACTIONS {
			let features = tile_features(action_tile(cell));
			for axis in 0..ACTION_FEATURES {
				expected[STATE + axis] += distribution[cell] * features[axis];
			}
		}
		let scored = b.forward(&expected);
		let grad_expected = b.backward(&expected, &scored, &[1.0], rate, &lookup.selected_b[&slot], true);
		let mut grad_logits = vec![0.0; ACTIONS];
		for cell in 0..ACTIONS {
			let features = tile_features(action_tile(cell));
			let mut direct = 0.0;
			for axis in 0..ACTION_FEATURES {
				direct += grad_expected[STATE + axis] * features[axis];
			}
			grad_logits[cell] = direct;
		}
		let weighted = (0..ACTIONS).map(|cell| distribution[cell] * grad_logits[cell]).sum::<f64>();
		for cell in 0..ACTIONS {
			grad_logits[cell] = distribution[cell] * (grad_logits[cell] - weighted);
		}
		t.backward(&state, &proposal, &grad_logits, rate, &lookup.selected_t[&slot], false);

		let verdict = if !measurement.dispatched {
			"rejected".to_owned()
		} else if measured < best_seconds {
			// Only a winner is worth the long numerical check.
			let checked = bench.measure(&candidate, check_epochs);
			if checked.dispatched && checked.digest == reference.digest {
				best = candidate;
				best_seconds = measured;
				"accepted".to_owned()
			} else {
				broke_numerics.push((slot, tile, checked.digest.clone()));
				"changes the trained model".to_owned()
			}
		} else {
			"slower".to_owned()
		};

		println!(
			"{step:>4}  node {:>2} dir {}  {:>4}x{:<3}x{:<4}  P {:>9.6}  M {:>9.6}  best {:>9.6}  {verdict}",
			slot.node,
			slot.direction,
			tile.m,
			tile.n,
			tile.k,
			predicted,
			if measured.is_finite() { measured } else { f64::NAN },
			best_seconds
		);
	}

	std::fs::write(&bench.cache, best.render()).expect("cannot write schedule cache");
	println!("\nselected schedule, written to {}", bench.cache.display());
	print!("{}", best.render());
	println!("\nbaseline {:.6} s, selected {:.6} s over {measure_epochs} epochs", base.seconds, best_seconds);
	println!("{} cells rejected by the compiled resource bounds", rejected.len());
	println!("{} cells measured faster but changed the trained model:", broke_numerics.len());
	for (slot, tile, digest) in &broke_numerics {
		println!("  node {} dir {}  {}x{}x{}  digest {digest} against reference {}", slot.node, slot.direction, tile.m, tile.n, tile.k, reference.digest);
	}
}

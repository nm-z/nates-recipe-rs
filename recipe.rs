//! Recipe executes one model graph after automatically probing a compiled discrete GPU backend.
//! Attention is three-projection scaled Q/K/V without an output projection.
#![allow(non_upper_case_globals)] use std::{ collections::{BTreeMap, BTreeSet}, error::Error, ffi::{c_char, c_void},
fmt, fs, mem::{size_of, size_of_val}, path::{Path, PathBuf}, ptr, sync::{ Mutex, OnceLock,
atomic::{AtomicBool, AtomicU64, Ordering}, }, time::Instant, };
pub static recipe: Recipe = Recipe;
static RUN: AtomicU64 = AtomicU64::new(0);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static SIGNAL: OnceLock<()> = OnceLock::new();
static SURROGATES: OnceLock<Mutex<BTreeMap<String, (Vec<f64>, Vec<f64>)>>> = OnceLock::new();
extern "C" fn interrupt(_: i32) { INTERRUPTED.store(true, Ordering::Relaxed); } #[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(String); impl RecipeError { fn new(message: impl Into<String>) -> Self { Self(message.into()) } }
impl fmt::Display for RecipeError { fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
formatter.write_str(&self.0) } } impl Error for RecipeError {} pub type Result<T> = std::result::Result<T, RecipeError>;
type Ptr = *mut c_void; #[derive(Clone, Copy, Debug, PartialEq, Eq)] enum Backend { Amd, Nvidia, } pub struct Data {
sources: Vec<String>, target: Vec<String>, exclusions: Vec<String>, normalize: bool, split: f64,
prepared: OnceLock<Result<Prepared>>, } #[derive(Clone, Debug, PartialEq, Eq)] pub enum Residual { Layer(usize), Relu, }
pub const fn layer(width: usize) -> Residual { Residual::Layer(width) } pub const fn relu() -> Residual { Residual::Relu
} #[derive(Clone, Debug)] enum Operation { Layer(usize), Conv(usize, usize), Pool(usize), KMeans(usize), Knn(usize),
Embed(usize, usize), Attention(usize), Rnn(usize), Gru(usize), Lstm(usize), Residual(Vec<Residual>), Perceptron(usize),
} #[derive(Clone, Copy, Debug, PartialEq, Eq)] #[repr(u8)] enum Activation { Linear, Cos, Exp, Log, Ln, Huber, Tan,
Relu, Leak, Sigmoid, Tanh, Selu, Gelu, Silu, Elu, Prelu, } #[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockNormalization { Batch, Layer, } #[derive(Clone, Debug)] struct Block { operation: Operation,
activation: Activation, normalization: Option<BlockNormalization>, } pub struct Model { blocks: Vec<Block>,
loss: LossFunction, } macro_rules! operation_methods {
($(fn $method:ident($($argument:ident: $kind:ty),*) = $operation:expr;)+) => {
$(pub fn $method(self, $($argument: $kind),*) -> Self { self.push($operation) })+ }; } impl Model {
fn push(mut self, operation: Operation) -> Self {
self.blocks.push(Block { operation, activation: Activation::Linear, normalization: None }); self }
fn activate(mut self, activation: Activation) -> Self {
let block = self.blocks.last_mut().unwrap_or_else(|| panic!("activation requires a preceding block"));
if block.normalization.is_some() { panic!("activation must precede normalization"); }
block.activation = activation; self } operation_methods! {
fn layer(width: usize) = Operation::Layer(width);
fn conv(filters: usize, kernel: usize) = Operation::Conv(filters, kernel);
fn pool(size: usize) = Operation::Pool(size);
fn kmeans(clusters: usize) = Operation::KMeans(clusters);
fn knn(neighbors: usize) = Operation::Knn(neighbors);
fn embed(dimensions: usize, vocabulary: usize) = Operation::Embed(dimensions, vocabulary);
fn attn(heads: usize) = Operation::Attention(heads);
fn rnn(width: usize) = Operation::Rnn(width);
fn gru(width: usize) = Operation::Gru(width);
fn lstm(width: usize) = Operation::Lstm(width);
fn perc(width: usize) = Operation::Perceptron(width); }
pub fn residual<const N: usize>(self, parts: [Residual; N]) -> Self { self.push(Operation::Residual(parts.into())) }
pub fn norm(mut self, normalization: Normalization) -> Self {
let block = self.blocks.last_mut().unwrap_or_else(|| panic!("normalization requires a preceding block"));
block.normalization = Some(if normalization as usize == batch as usize { BlockNormalization::Batch } else {
BlockNormalization::Layer }); self } pub const fn loss(mut self, loss: LossFunction) -> Self {
self.loss = loss; self } fn description(&self, metrics: &[Metric]) -> String {
let operation = metrics.iter().any(|metric| metric.0 == 5);
let activation = metrics.iter().any(|metric| metric.0 == 6);
let normalization = metrics.iter().any(|metric| metric.0 == 7); self.blocks .iter() .filter_map(|block| {
let mut names = Vec::new(); if operation {
names.push(block.operation.name()); } if activation && block.activation != Activation::Linear {
names.push(block.activation.name()); } if normalization { block.normalization .map(BlockNormalization::name)
.into_iter() .for_each(|name| names.push(name)); } (!names.is_empty()).then(|| names.join(".")) }) .collect::<Vec<_>>()
.join("/") } } impl Operation { const fn name(&self) -> &'static str { match self { Self::Layer(_) => "layer",
Self::Conv(..) => "conv", Self::Pool(_) => "pool", Self::KMeans(_) => "kmeans", Self::Knn(_) => "knn",
Self::Embed(..) => "embed", Self::Attention(_) => "attn", Self::Rnn(_) => "rnn", Self::Gru(_) => "gru",
Self::Lstm(_) => "lstm", Self::Residual(_) => "residual", Self::Perceptron(_) => "perc", } } } impl Activation {
const fn name(self) -> &'static str { match self { Self::Linear => "linear", Self::Cos => "cos", Self::Exp => "exp",
Self::Log => "log", Self::Ln => "ln", Self::Huber => "huber", Self::Tan => "tan", Self::Relu => "relu",
Self::Leak => "leak", Self::Sigmoid => "sigmoid", Self::Tanh => "tanh", Self::Selu => "selu", Self::Gelu => "gelu",
Self::Silu => "silu", Self::Elu => "elu", Self::Prelu => "prelu", } } } impl BlockNormalization {
const fn name(self) -> &'static str { match self { Self::Batch => "bnorm", Self::Layer => "lnorm", } } }
macro_rules! activations { ($(fn $method:ident = $activation:ident;)+) => {$(impl Model { pub fn $method(self) -> Self {
self.activate(Activation::$activation) } })+}; } activations! {
fn cos = Cos;
fn exp = Exp;
fn log = Log;
fn ln = Ln;
fn huber = Huber;
fn tan = Tan;
fn relu = Relu;
fn leak = Leak;
fn sigmoid = Sigmoid;
fn tanh = Tanh;
fn selu = Selu;
fn gelu = Gelu;
fn silu = Silu;
fn elu = Elu;
fn prelu = Prelu; } pub trait IntoDataSources {
fn into_data_sources(self) -> Vec<String>; } impl IntoDataSources for &str { fn into_data_sources(self) -> Vec<String> {
vec![self.to_owned()] } } impl IntoDataSources for String { fn into_data_sources(self) -> Vec<String> { vec![self] } }
impl<T: Into<String>, const N: usize> IntoDataSources for [T; N] { fn into_data_sources(self) -> Vec<String> {
self.into_iter().map(Into::into).collect() } } impl<T: Into<String>> IntoDataSources for Vec<T> {
fn into_data_sources(self) -> Vec<String> { self.into_iter().map(Into::into).collect() } }
impl<T: Clone + Into<String>> IntoDataSources for &[T] { fn into_data_sources(self) -> Vec<String> {
self.iter().cloned().map(Into::into).collect() } } impl Data {
pub fn target(mut self, target: impl IntoDataSources) -> Self { self.target = target.into_data_sources(); self }
pub fn exclude(mut self, names: impl IntoDataSources) -> Self { self.exclusions = names.into_data_sources(); self }
pub fn set(mut self, source: impl Into<String>) -> Self { self.sources.push(source.into()); self }
pub const fn norm(mut self, _: ZScore) -> Self { self.normalize = true; self }
pub const fn split(mut self, fraction: f64) -> Self { self.split = fraction; self } } struct Prepared {
samples: Vec<f64>, targets: Vec<f64>, rows: usize, features: usize, schema: String, } struct Table { name: String,
headers: Vec<String>, rows: Vec<Vec<String>>, } enum FeatureType { Numeric(&'static str), Categorical(Vec<String>),
Text(usize), } fn prepare(data: &Data) -> Result<&Prepared> { match data.prepared.get_or_init(|| prepare_data(data)) {
Ok(prepared) => Ok(prepared), Err(error) => Err(error.clone()), } } fn prepare_data(data: &Data) -> Result<Prepared> {
let mut paths = Vec::new(); for source in &data.sources {
collect_files(&expand_home(source)?, &mut paths)?; }
paths.sort();
paths.dedup();
let mut tables = Vec::new(); for path in paths { let bytes = fs::read(&path)
.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
if path.extension().and_then(|value| value.to_str()).is_some_and(is_table) { tables.push(parse_table(&path, &bytes)?); }
} require(!tables.is_empty(), "data source contains no supported table")?;
let mut selected = Vec::new(); for name in &data.target {
let mut matches = Vec::new(); for (table, value) in tables.iter().enumerate() {
for (column, header) in value.headers.iter().enumerate() { let qualified = format!("{}.{}", value.name, header);
let numbered = format!("col{}", column + 1);
let qualified_numbered = format!("{}.{}", value.name, numbered);
if name == header || name == &qualified || name == &numbered || name == &qualified_numbered {
matches.push((table, column)); } } }
require(matches.len() == 1, format!("target {name:?} must identify exactly one feature"))?;
selected.push(matches[0]); }
let table_index = selected.first().map_or(0, |target| target.0);
let row_count = tables[table_index].rows.len();
require(selected.iter().all(|target| tables[target.0].rows.len() == row_count), "target row counts differ")?;
let fit_rows = ((row_count as f64) * data.split).floor().max(1.0) as usize;
eprintln!("Feature name:                         Dtype:    Samples:"); for value in &tables {
for (column, header) in value.headers.iter().enumerate() {
let kind = infer_feature(value, column, fit_rows.min(value.rows.len())); let samples =
value.rows.iter().filter(|row| row.get(column).is_some_and(|item| !item.is_empty())).count();
eprintln!("{:<37} {:<9} {samples}", format!("{}.{}", value.name, header), kind.name()); } }
let mut columns = Vec::new(); for (table, value) in tables.iter().enumerate() { if value.rows.len() == row_count {
for (column, header) in value.headers.iter().enumerate() { let qualified = format!("{}.{}", value.name, header);
let excluded = data.exclusions.iter().any(|name| name == header || name == &qualified);
if !selected.contains(&(table, column)) && !excluded {
columns.push((table, column, infer_feature(value, column, fit_rows))); } } } }
let features = columns.iter().map(|column| column.2.width()).sum();
require(features != 0, "dataset has no training features")?; let target_categories =
selected.iter().map(|target| categories(&tables[target.0], target.1, fit_rows)).collect::<Vec<_>>();
let mut samples = Vec::new();
let mut targets = Vec::new(); for row in 0..row_count {
let mut encoded = Vec::with_capacity(features); let valid = columns.iter().all(|column| {
tables[column.0].rows[row].get(column.1).is_some_and(|value| encode(value, &column.2, &mut encoded)) });
if valid && selected.is_empty() { samples.extend_from_slice(&encoded);
targets.push(0.0); } else if valid { for (target, categories) in selected.iter().zip(&target_categories) {
let value = tables[target.0].rows[row].get(target.1);
let target = value.and_then(|value| value.parse::<f64>().ok()).or_else(|| {
value.and_then(|value| categories.iter().position(|category| category == value)) .map(|value| value as f64) });
if let Some(target) = target && target.is_finite() { samples.extend_from_slice(&encoded);
targets.push(target); } } } }
let rows = targets.len();
require(rows != 0, "dataset has no complete training rows")?; if data.normalize {
normalize_samples(&mut samples, features, ((rows as f64) * data.split).floor() as usize)?; }
shuffle(&mut samples, &mut targets, features)?; let schema = columns .iter() .map(|column| {
format!("{}.{}:{}", tables[column.0].name, tables[column.0].headers[column.1], column.2.width()) }) .collect::<Vec<_>>()
.join("|") + "->" + &data.target.join("|"); Ok(Prepared { samples, targets, rows, features, schema }) }
fn normalize_samples(samples: &mut [f64], features: usize, fit: usize) -> Result<()> {
require(fit != 0, "split must retain normalization rows")?; for column in 0..features {
let mean = (0..fit).map(|row| samples[row * features + column]).sum::<f64>() / fit as f64; let variance =
(0..fit).map(|row| (samples[row * features + column] - mean).powi(2)).sum::<f64>() / fit as f64;
let scale = if variance == 0.0 { 1.0 } else { variance.sqrt() }; for row in 0..samples.len() / features {
samples[row * features + column] = (samples[row * features + column] - mean) / scale; } } Ok(()) }
fn is_table(extension: &str) -> bool { matches!(extension.to_ascii_lowercase().as_str(), "csv" | "tsv" | "txt") }
fn expand_home(source: &str) -> Result<PathBuf> { if source == "~" || source.starts_with("~/") {
let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?;
return Ok(PathBuf::from(home).join(source.trim_start_matches("~/"))); } Ok(PathBuf::from(source)) }
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> { let metadata = fs::metadata(path)
.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?; if metadata.is_file() {
files.push(path.to_owned());
return Ok(()); } let mut children = fs::read_dir(path)
.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?
.collect::<std::io::Result<Vec<_>>>()
.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
children.sort_by_key(fs::DirEntry::path); for child in children {
collect_files(&child.path(), files)?; } Ok(()) } fn parse_table(path: &Path, bytes: &[u8]) -> Result<Table> {
let first = bytes.split(|byte| *byte == b'\n').next().unwrap_or_default();
let delimiter = [b',', b';', b'\t'] .into_iter()
.max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count()) .unwrap_or(b',');
let mut rows = records(bytes, delimiter)?;
require(!rows.is_empty(), format!("dataset {} is empty", path.display()))?;
let first = rows.remove(0);
let headerless = first.iter().all(|value| value.parse::<f64>().is_ok()); let headers =
if headerless { (1..=first.len()).map(|column| format!("col{column}")).collect() } else { first.clone() };
if headerless { rows.insert(0, first); }
let width = headers.len();
rows.retain(|row| row.len() == width);
let name = path.file_stem().and_then(|value| value.to_str()).unwrap_or("data").to_owned();
Ok(Table { name, headers, rows }) } fn records(bytes: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>> {
let mut rows = Vec::new();
let mut row = Vec::new();
let mut field = Vec::new();
let mut quoted = false;
let mut index = 0; while index < bytes.len() {
let byte = bytes[index]; if byte == b'"' { if quoted && bytes.get(index + 1) == Some(&b'"') {
field.push(byte);
index += 1; } else {
quoted = !quoted; } } else if byte == delimiter && !quoted {
row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
field = Vec::new(); } else if byte == b'\n' && !quoted {
let value = String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?;
row.push(value.trim_end_matches('\r').to_owned());
field = Vec::new(); if row.iter().any(|value| !value.is_empty()) {
rows.push(row); }
row = Vec::new(); } else {
field.push(byte); }
index += 1; }
require(!quoted, "unterminated quoted feature")?; if !field.is_empty() || !row.is_empty() {
row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
rows.push(row); } Ok(rows) } fn categories(table: &Table, column: usize, rows: usize) -> Vec<String> { table.rows
.iter() .take(rows) .filter_map(|row| row.get(column)) .filter(|value| !value.is_empty()) .cloned()
.collect::<BTreeSet<_>>() .into_iter() .collect() }
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
let values = table.rows.iter().take(rows).filter_map(|row| row.get(column)).collect::<Vec<_>>();
if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) { return FeatureType::Numeric("f64"); }
let categories = categories(table, column, rows); if categories.len() < values.len() {
FeatureType::Categorical(categories) } else {
FeatureType::Text(values.iter().map(|value| value.len()).max().unwrap_or(0)) } } impl FeatureType {
const fn name(&self) -> &'static str { match self { Self::Numeric(name) => name, Self::Categorical(_) => "categoric",
Self::Text(_) => "string", } } fn width(&self) -> usize { match self { Self::Numeric(_) => 1,
Self::Categorical(values) => values.len(), Self::Text(width) => *width, } } }
fn encode(value: &str, kind: &FeatureType, output: &mut Vec<f64>) -> bool { match kind {
FeatureType::Numeric(_) => value.parse::<f64>().is_ok_and(|value| { output.push(value); value.is_finite() }),
FeatureType::Categorical(categories) => { let found = categories.iter().position(|category| category == value);
output.extend((0..categories.len()).map(|index| f64::from(found == Some(index)))); found.is_some() }
FeatureType::Text(width) => { output.extend(value.bytes().map(f64::from).chain(std::iter::repeat(0.0)).take(*width));
value.len() <= *width } } } fn shuffle(samples: &mut Vec<f64>, targets: &mut Vec<f64>, features: usize) -> Result<()> {
let mut seed = env!("RECIPE_RANDOM_SEED") .parse::<u64>()
.map_err(|error| RecipeError::new(format!("invalid random seed: {error}")))?;
let mut order = (0..targets.len()).collect::<Vec<_>>(); for index in (1..order.len()).rev() {
seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
order.swap(index, (seed as usize) % (index + 1)); }
let old_samples = std::mem::take(samples);
let old_targets = std::mem::take(targets); for row in order {
samples.extend_from_slice(&old_samples[row * features..(row + 1) * features]);
targets.push(old_targets[row]); } Ok(()) }
pub struct Recipe;
pub struct Adamw; #[derive(Clone, Copy)]
pub struct LossFunction(u8); #[derive(Clone, Copy)]
pub struct Metric(u8);
pub struct ZScore;
pub type Normalization = fn(usize) -> Residual;
pub type Norm = Normalization;
pub type Loss = LossFunction;
pub const adamw: Adamw = Adamw;
pub const mse: LossFunction = LossFunction(0);
pub const rmse: LossFunction = LossFunction(1);
pub const huber: LossFunction = LossFunction(2);
pub const mae: LossFunction = LossFunction(3);
pub const bce: LossFunction = LossFunction(4);
pub const ce: LossFunction = LossFunction(5);
pub const focal: LossFunction = LossFunction(6);
pub const Run: Metric = Metric(0);
pub const Loss: Metric = Metric(1);
pub const R2: Metric = Metric(2);
pub const Time: Metric = Metric(3);
pub const Epoch: Metric = Metric(4);
pub const blck: Metric = Metric(5);
pub const atvn: Metric = Metric(6);
pub const norm: Metric = Metric(7);
pub const z_score: ZScore = ZScore;
pub const batch: Normalization = batch_marker; const fn batch_marker(_: usize) -> Residual { Residual::Relu }
impl LossFunction { const fn name(self) -> &'static str { match self.0 { 0 => "mse", 1 => "rmse", 2 => "huber",
3 => "mae", 4 => "bce", 5 => "ce", 6 => "focal", _ => unreachable!(), } }
fn value(self, prediction: f64, target: f64, threshold: f64) -> f64 { let difference = prediction - target;
let probability = sigmoid(prediction).clamp(f64::EPSILON, 1.0 - f64::EPSILON); match self.0 {
0 | 1 => difference * difference, 2 => { let absolute = difference.abs(); if absolute <= threshold {
0.5 * difference * difference } else { threshold * (absolute - 0.5 * threshold) } } 3 => difference.abs(),
4 | 5 => -target * probability.ln() - (1.0 - target) * (1.0 - probability).ln(), 6 => {
let correct = if target >= 0.5 { probability } else { 1.0 - probability }; -(1.0 - correct).powi(2) * correct.ln() }
_ => f64::NAN, } } } impl Recipe { pub fn data(&self, sources: impl IntoDataSources) -> Data { Data {
sources: sources.into_data_sources(), target: Vec::new(), exclusions: Vec::new(), normalize: false, split: 1.0,
prepared: OnceLock::new(), } } pub fn model(&self) -> Model { Model { blocks: Vec::new(), loss: mse } }
pub const fn train(&self) -> Train {
Train { epochs: 1, learning_rate: 0.001, log_metrics: Vec::new(), stop: None, resume: None, save: None } }
pub fn export(&self, source: impl AsRef<Path>) -> Result<PathBuf> { let source = source.as_ref(); require(
source.extension().and_then(|value| value.to_str()) == Some("rs"), "export requires a Rust source", )?;
fs::metadata(source) .map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", source.display())))?;
let backend = device_backend()?; let (compiled, extension) = match backend {
Backend::Amd => (option_env!("RECIPE_HSA_CODE_OBJECT"), "hsaco"),
Backend::Nvidia => (option_env!("RECIPE_NV_MODULE"), "cubin"), };
let compiled = compiled.ok_or_else(|| RecipeError::new("selected GPU backend was not compiled"))?;
let output = source.with_extension(extension); fs::copy(compiled, &output)
.map_err(|error| RecipeError::new(format!("cannot export {}: {error}", output.display())))?;
eprintln!("exported: {}", output.display()); Ok(output) } } #[derive(Clone, Copy, Debug)] struct Shape {
channels: usize, length: usize, } impl Shape { fn elements(self) -> usize { self.channels * self.length } }
#[derive(Clone, Copy)] #[repr(i32)] enum Primitive { Contraction = 0, Pool = 2, Gather = 3, Attention = 4, Scan = 5,
Elementwise = 6, Normalize = 8, Estimator = 9, } #[derive(Clone, Copy)] #[repr(i32)] enum ScalarOpcode { Add, Constant,
Parameter, Subtract, Multiply, Divide, Absolute, Exp, Log, Sin = 10, Cos, Tanh, Greater, Surrogate, }
struct ScalarProgram(Vec<f64>); impl ScalarProgram {
fn op(&mut self, opcode: ScalarOpcode, left: f64, right: f64) -> f64 { let result = (self.0.len() / 3) as f64;
self.0.extend([opcode as i32 as f64, left, right]); result } fn constant(&mut self, value: f64) -> f64 {
self.op(ScalarOpcode::Constant, value, 0.0) } fn choose(&mut self, condition: f64, yes: f64, no: f64) -> f64 {
let one = self.constant(1.0);
let inverse = self.op(ScalarOpcode::Subtract, one, condition);
let selected = self.op(ScalarOpcode::Multiply, condition, yes);
let alternative = self.op(ScalarOpcode::Multiply, inverse, no); self.op(ScalarOpcode::Add, selected, alternative) }
fn unary(&mut self, opcode: ScalarOpcode, value: f64) -> f64 { self.op(opcode, value, 0.0) } } #[derive(Clone)]
struct Node { op: Primitive, source: i32, second: i32, input: Shape, output: Shape, offset: usize, parameters: usize,
argument: [f64; 9], program_offset: usize, program_count: usize, } struct Graph { nodes: Vec<Node>,
parameters: Vec<f64>, frozen: Vec<u8>, programs: Vec<f64>, output: Shape, }
fn compile(model: &Model, data: &Prepared, rows: usize, backend: Backend, config: Config) -> Result<Graph> {
require(!model.blocks.is_empty(), "model must contain a block")?; let sequential =
matches!(model.blocks[0].operation, Operation::Conv(..) | Operation::Pool(..) | Operation::Embed(..));
let shape = if sequential { Shape { channels: 1, length: data.features } } else {
Shape { channels: data.features, length: 1 } }; let mut graph = Graph { nodes: Vec::new(), parameters: Vec::new(),
frozen: Vec::new(), programs: Vec::new(), output: shape, }; for block in &model.blocks {
lower_block(&mut graph, block, data, rows, backend, config)?; } if graph.output.elements() != 1 {
let length = graph.output.length;
lower_conv(&mut graph, 1, length)?; }
initialize_graph(&mut graph, config); Ok(graph) } fn lower_block( graph: &mut Graph, block: &Block, data: &Prepared,
rows: usize, backend: Backend, config: Config, ) -> Result<()> { let skip = graph.nodes.len() as i32 - 1;
match &block.operation { Operation::Layer(width) | Operation::Perceptron(width) => lower_project(graph, *width)?,
Operation::Conv(filters, kernel) => lower_conv(graph, *filters, *kernel)?,
Operation::Pool(size) => lower_pool(graph, *size)?,
Operation::Embed(dimensions, vocabulary) => lower_gather(graph, *dimensions, *vocabulary)?,
Operation::Attention(heads) => lower_attention(graph, *heads)?, Operation::Rnn(width) => lower_scan(graph, *width, 1)?,
Operation::Gru(width) => lower_scan(graph, *width, 3)?, Operation::Lstm(width) => lower_scan(graph, *width, 4)?,
Operation::Residual(parts) => lower_residual(graph, parts, skip, config)?, Operation::KMeans(_) | Operation::Knn(_) => {
lower_estimator(graph, &block.operation, data, rows, backend, config)? } } if block.activation != Activation::Linear {
lower_activation(graph, block.activation, config)?; } if let Some(normalization) = block.normalization {
let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?; push_node( graph,
Primitive::Normalize, graph.output, 0, arguments(normalization as u8 as f64, epsilon), -2, )?; }
let elements = checked_mul(rows, graph.output.elements(), "node batch")?;
narrow(elements, "GPU node batch")?; Ok(()) } fn push_node( graph: &mut Graph, op: Primitive, output: Shape,
parameters: usize, argument: [f64; 9], second: i32, ) -> Result<()> {
let source = graph.nodes.len() as i32 - 1;
let offset = graph.parameters.len();
graph.parameters.resize(checked_add(offset, parameters, "model parameters")?, 0.0);
graph.frozen.resize(graph.parameters.len(), 0); graph.nodes.push(Node { op, source, second, input: graph.output, output,
offset, parameters, argument, program_offset: 0, program_count: 0, });
graph.output = output; Ok(()) }
fn push_program(graph: &mut Graph, second: i32, initial: &[f64], program: ScalarProgram) -> Result<()> {
let program_offset = graph.programs.len();
let program_count = program.0.len() / 3;
graph.programs.extend(program.0);
let arguments = [0.0;
9];
push_node(graph, Primitive::Elementwise, graph.output, initial.len(), arguments, second)?;
let node = graph.nodes.last_mut().ok_or_else(|| RecipeError::new("scalar program node is absent"))?;
graph.parameters[node.offset..node.offset + initial.len()].copy_from_slice(initial);
node.program_offset = program_offset;
node.program_count = program_count; Ok(()) }
fn lower_activation(graph: &mut Graph, activation: Activation, config: Config) -> Result<()> {
let mut program = ScalarProgram(Vec::new());
let x = -1.0;
let zero = program.constant(0.0);
let one = program.constant(1.0);
let positive = program.op(ScalarOpcode::Greater, x, zero);
let constant = |program: &mut ScalarProgram, value| program.constant(value); let result = match activation {
Activation::Cos => program.unary(ScalarOpcode::Cos, x), Activation::Exp => program.unary(ScalarOpcode::Exp, x),
Activation::Log | Activation::Ln => { let absolute = program.unary(ScalarOpcode::Absolute, x);
let shifted = program.op(ScalarOpcode::Add, one, absolute);
let magnitude = program.unary(ScalarOpcode::Log, shifted);
let negative = program.op(ScalarOpcode::Subtract, zero, magnitude);
let signed = program.choose(positive, magnitude, negative); if activation == Activation::Log {
let base = constant(&mut program, std::f64::consts::LN_10); program.op(ScalarOpcode::Divide, signed, base) } else {
signed } } Activation::Huber => { let threshold = constant(&mut program, config.activation[7]);
let absolute = program.unary(ScalarOpcode::Absolute, x);
let large = program.op(ScalarOpcode::Greater, absolute, threshold);
let square = program.op(ScalarOpcode::Multiply, x, x);
let half = constant(&mut program, 0.5);
let small = program.op(ScalarOpcode::Multiply, half, square);
let half_threshold = program.op(ScalarOpcode::Multiply, half, threshold);
let excess = program.op(ScalarOpcode::Subtract, absolute, half_threshold);
let tail = program.op(ScalarOpcode::Multiply, threshold, excess); program.choose(large, tail, small) }
Activation::Tan => { let sine = program.unary(ScalarOpcode::Sin, x);
let cosine = program.unary(ScalarOpcode::Cos, x); program.op(ScalarOpcode::Divide, sine, cosine) }
Activation::Relu => program.op(ScalarOpcode::Multiply, positive, x),
Activation::Leak | Activation::Elu | Activation::Selu | Activation::Prelu => { let negative = match activation {
Activation::Leak => { let slope = constant(&mut program, config.activation[0]);
program.op(ScalarOpcode::Multiply, slope, x) } Activation::Prelu => {
let slope = program.op(ScalarOpcode::Parameter, 0.0, 0.0); program.op(ScalarOpcode::Multiply, slope, x) } _ => {
let exponential = program.unary(ScalarOpcode::Exp, x);
let shifted = program.op(ScalarOpcode::Subtract, exponential, one); let alpha = constant( &mut program,
config.activation[usize::from(activation == Activation::Selu) + 2], );
program.op(ScalarOpcode::Multiply, alpha, shifted) } };
let selected = program.choose(positive, x, negative); if activation == Activation::Selu {
let scale = constant(&mut program, config.activation[4]); program.op(ScalarOpcode::Multiply, scale, selected) } else {
selected } } Activation::Sigmoid | Activation::Silu => { let negative = program.op(ScalarOpcode::Subtract, zero, x);
let exponential = program.unary(ScalarOpcode::Exp, negative);
let denominator = program.op(ScalarOpcode::Add, one, exponential);
let sigmoid = program.op(ScalarOpcode::Divide, one, denominator);
if activation == Activation::Silu { program.op(ScalarOpcode::Multiply, x, sigmoid) } else { sigmoid } }
Activation::Tanh => program.unary(ScalarOpcode::Tanh, x), Activation::Gelu => {
let square = program.op(ScalarOpcode::Multiply, x, x);
let cube = program.op(ScalarOpcode::Multiply, square, x);
let cubic = constant(&mut program, config.activation[6]);
let curved = program.op(ScalarOpcode::Multiply, cubic, cube);
let sum = program.op(ScalarOpcode::Add, x, curved);
let scale = constant(&mut program, config.activation[5]);
let argument = program.op(ScalarOpcode::Multiply, scale, sum);
let tanh = program.unary(ScalarOpcode::Tanh, argument);
let shifted = program.op(ScalarOpcode::Add, one, tanh);
let half = constant(&mut program, 0.5);
let half_x = program.op(ScalarOpcode::Multiply, half, x); program.op(ScalarOpcode::Multiply, half_x, shifted) }
Activation::Linear => unreachable!(), };
let initial = if activation == Activation::Prelu { &config.activation[1..2] } else { &[] };
debug_assert_eq!(result as usize + 1, program.0.len() / 3); push_program(graph, -2, initial, program) }
fn lower_project(graph: &mut Graph, channels: usize) -> Result<()> {
require(channels != 0, "layer width must be positive")?;
let parameters = checked_mul(graph.output.channels, channels, "projection parameters")?;
let output = Shape { channels, length: graph.output.length };
push_node(graph, Primitive::Contraction, output, parameters, [0.0; 9], -2) }
fn lower_conv(graph: &mut Graph, filters: usize, kernel: usize) -> Result<()> {
require(filters != 0 && kernel != 0, "convolution dimensions must be positive")?;
require(kernel <= graph.output.length, "convolution kernel exceeds sequence length")?;
let parameters = checked_mul(filters, checked_mul(graph.output.channels, kernel, "convolution window")?, "conv")?;
let output = Shape { channels: filters, length: graph.output.length - kernel + 1 };
push_node(graph, Primitive::Contraction, output, parameters, arguments(kernel as f64, 0.0), -2) }
fn lower_pool(graph: &mut Graph, size: usize) -> Result<()> { require(size != 0, "pool window must be positive")?;
let output = Shape { channels: graph.output.channels, length: graph.output.length.div_ceil(size) };
push_node(graph, Primitive::Pool, output, 0, arguments(size as f64, 0.0), -2) }
fn lower_gather(graph: &mut Graph, dimensions: usize, vocabulary: usize) -> Result<()> {
require(dimensions != 0 && vocabulary != 0, "embedding dimensions must be positive")?;
let parameters = checked_mul(dimensions, vocabulary, "embedding table")?;
let output = Shape { channels: dimensions, length: graph.output.elements() };
push_node(graph, Primitive::Gather, output, parameters, arguments(vocabulary as f64, 0.0), -2) }
fn lower_attention(graph: &mut Graph, heads: usize) -> Result<()> {
require(heads != 0 && graph.output.channels % heads == 0, "attention head partition is invalid")?;
let matrix = checked_mul(graph.output.channels, graph.output.channels, "attention matrix")?; push_node( graph,
Primitive::Attention, graph.output, checked_mul(3, matrix, "QKV")?, arguments(heads as f64, 0.0), -2, ) }
fn lower_scan(graph: &mut Graph, channels: usize, gates: usize) -> Result<()> {
require(channels != 0, "recurrent width must be positive")?;
let input = checked_mul(graph.output.channels, channels, "scan input matrix")?;
let state = checked_mul(channels, channels, "scan state matrix")?;
let stride = checked_add(checked_add(input, state, "scan gate")?, channels, "scan bias")?;
let output = Shape { channels, length: graph.output.length }; push_node( graph, Primitive::Scan, output,
checked_mul(gates, stride, "scan parameters")?, arguments(gates as f64, 0.0), -2, ) }
fn lower_residual(graph: &mut Graph, parts: &[Residual], skip: i32, config: Config) -> Result<()> {
let shape = graph.output;
require(!parts.is_empty(), "residual branch must contain an operation")?; for part in parts { match part {
Residual::Layer(width) => lower_project(graph, *width)?,
Residual::Relu => lower_activation(graph, Activation::Relu, config)?, } } require(
graph.output.channels == shape.channels && graph.output.length == shape.length, "residual shape mismatch", )?;
let mut program = ScalarProgram(Vec::new());
program.op(ScalarOpcode::Add, -1.0, -2.0); push_program(graph, skip, &[], program) }
fn initialize_graph(graph: &mut Graph, config: Config) {
for (weight, frozen) in graph.parameters.iter_mut().zip(&graph.frozen) { if *frozen == 0 { *weight = config.initial; } }
} fn push_frozen( graph: &mut Graph, op: Primitive, input: Shape, output: Shape, values: &[f64], argument: [f64; 9],
source: i32, ) -> Result<()> { let offset = graph.parameters.len();
graph.parameters.extend_from_slice(values);
graph.frozen.resize(graph.parameters.len(), 1); graph.nodes.push(Node { op, source, second: -2, input, output, offset,
parameters: 0, argument, program_offset: 0, program_count: 0, });
graph.output = output; Ok(()) } fn graph_inputs( graph: &Graph, samples: &[f64], targets: &[f64], rows: usize,
backend: Backend, config: Config, ) -> Result<Vec<f64>> { if graph.nodes.is_empty() {
return Ok(samples[..rows * graph.output.elements()].to_vec()); }
let mut tape = DeviceTape::new(graph, samples, &targets[..rows], backend, config)?;
tape.forward()?; tape.predictions() }
fn surrogate_key(operation: &Operation, input: Shape, values: &[f64], targets: &[f64]) -> String {
let mut hash = 0xcbf29ce484222325_u64; for value in values.iter().chain(targets) {
hash = hash.wrapping_mul(0x100000001b3) ^ value.to_bits(); } format!("{operation:?}:{input:?}:{hash:016x}") }
fn fit_surrogate( input: Shape, samples: &[f64], targets: &[f64], hidden: usize, backend: Backend, config: Config,
) -> Result<Vec<f64>> { let mut graph = Graph { nodes: Vec::new(), parameters: Vec::new(), frozen: Vec::new(),
programs: Vec::new(), output: input, };
lower_conv(&mut graph, hidden, input.length)?;
lower_activation(&mut graph, Activation::Tanh, config)?;
lower_project(&mut graph, 1)?;
initialize_graph(&mut graph, config);
let mut tape = DeviceTape::new(&graph, samples, targets, backend, config)?; for epoch in 1..=config.surrogate_epochs {
tape.epoch(epoch, config.surrogate_rate, mse, 0.0, config)?; } tape.weights(false) } fn lower_estimator(
graph: &mut Graph, operation: &Operation, data: &Prepared, rows: usize, backend: Backend, config: Config,
) -> Result<()> { initialize_graph(graph, config);
let input = graph.output;
let source = graph.nodes.len() as i32 - 1;
let raw = &data.samples[..rows * data.features];
let inputs = graph_inputs(graph, raw, &data.targets, rows, backend, config)?;
let key = surrogate_key(operation, input, &inputs, &data.targets[..rows]);
let cache = SURROGATES.get_or_init(|| Mutex::new(BTreeMap::new())); let cached =
cache.lock().map_err(|_| RecipeError::new("estimator surrogate cache is poisoned"))?.get(&key).cloned();
let (state, surrogate) = if let Some(cached) = cached { cached } else { let mut samples = inputs.clone();
samples.extend_from_slice(&inputs);
let mut targets = data.targets[..rows].to_vec();
targets.extend_from_within(..);
let paired = Prepared { samples, targets, rows: rows * 2, features: input.elements(), schema: key.clone() };
let (teacher, state) = estimator_predict(operation, &paired, rows, backend, config, None, true)?;
let hidden = match operation { Operation::KMeans(value) | Operation::Knn(value) => *value, _ => unreachable!(), };
let surrogate = fit_surrogate(input, &inputs, &teacher, hidden, backend, config)?; cache.lock()
.map_err(|_| RecipeError::new("estimator surrogate cache is poisoned"))?
.insert(key, (state.clone(), surrogate.clone())); (state, surrogate)
}; let (kind, width) = match operation { Operation::KMeans(width) => (0.0, *width),
Operation::Knn(width) => (1.0, *width), _ => unreachable!(), };
let mut estimator_arguments = arguments(kind, width as f64);
estimator_arguments[2] = rows as f64; push_frozen( graph, Primitive::Estimator, input, Shape { channels: 1, length: 1 },
&state, estimator_arguments, source, )?;
let teacher = graph.nodes.len() as i32 - 1;
let hidden = width;
let first = checked_mul(hidden, input.elements(), "surrogate input")?;
require(surrogate.len() == first + hidden, "surrogate state has the wrong size")?; push_frozen( graph,
Primitive::Contraction, input, Shape { channels: hidden, length: 1 }, &surrogate[..first],
arguments(input.length as f64, 0.0), source, )?;
lower_activation(graph, Activation::Tanh, config)?;
let surrogate_source = graph.nodes.len() as i32 - 1; push_frozen( graph, Primitive::Contraction,
Shape { channels: hidden, length: 1 }, Shape { channels: 1, length: 1 }, &surrogate[first..], [0.0; 9],
surrogate_source, )?;
let mut program = ScalarProgram(Vec::new());
program.op(ScalarOpcode::Surrogate, -1.0, -2.0); push_program(graph, teacher, &[], program) }
fn arguments(first: f64, second: f64) -> [f64; 9] { [first, second, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }
fn checked_add(left: usize, right: usize, role: &str) -> Result<usize> {
left.checked_add(right).ok_or_else(|| RecipeError::new(format!("{role} overflows"))) }
fn checked_mul(left: usize, right: usize, role: &str) -> Result<usize> {
left.checked_mul(right).ok_or_else(|| RecipeError::new(format!("{role} overflows"))) }
fn require(condition: bool, message: impl Into<String>) -> Result<()> {
condition.then_some(()).ok_or_else(|| RecipeError::new(message)) } fn sigmoid(value: f64) -> f64 {
1.0 / (1.0 + (-value).exp()) } #[derive(Clone, Copy)] struct Config { kmeans_iterations: usize, threads: usize,
surrogate_epochs: usize, surrogate_rate: f64, initial: f64, beta1: f64, beta2: f64, epsilon: f64, decay: f64,
activation: [f64; 8], } impl Config { fn load() -> Result<Self> { Ok(Self {
kmeans_iterations: natural("kmeans iterations", env!("RECIPE_KMEANS_ITERATIONS"))?,
threads: natural("GPU threads", env!("RECIPE_GPU_THREADS"))?,
surrogate_epochs: natural("surrogate epochs", env!("RECIPE_SURROGATE_EPOCHS"))?,
surrogate_rate: number("surrogate rate", env!("RECIPE_SURROGATE_RATE"))?,
initial: number("initial weight", env!("RECIPE_TRAIN_INITIAL_WEIGHT"))?,
beta1: number("AdamW beta1", env!("RECIPE_ADAMW_BETA1"))?, beta2: number("AdamW beta2", env!("RECIPE_ADAMW_BETA2"))?,
epsilon: number("AdamW epsilon", env!("RECIPE_ADAMW_EPSILON"))?,
decay: number("AdamW weight decay", env!("RECIPE_ADAMW_WEIGHT_DECAY"))?, activation: [
number("leak slope", env!("RECIPE_LEAK_SLOPE"))?, number("PReLU slope", env!("RECIPE_PRELU_SLOPE"))?,
number("ELU alpha", env!("RECIPE_ELU_ALPHA"))?, number("SELU alpha", env!("RECIPE_SELU_ALPHA"))?,
number("SELU scale", env!("RECIPE_SELU_SCALE"))?, number("GELU scale", env!("RECIPE_GELU_SCALE"))?,
number("GELU cubic", env!("RECIPE_GELU_CUBIC"))?, number("Huber threshold", env!("RECIPE_HUBER_THRESHOLD"))?, ], }) } }
fn number(name: &str, text: &str) -> Result<f64> {
let value = text.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
(value.is_finite() && value > 0.0) .then_some(value)
.ok_or_else(|| RecipeError::new(format!("{name} must be finite and positive"))) }
fn natural(name: &str, text: &str) -> Result<usize> {
let value = text.parse::<usize>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
require(value != 0, format!("{name} must be positive")).map(|_| value) } pub struct Train { epochs: usize,
learning_rate: f64, log_metrics: Vec<Metric>, stop: Option<f64>, resume: Option<String>, save: Option<String>, }
impl Train { pub const fn stop(mut self, value: f64) -> Self { self.stop = Some(value); self }
pub const fn optimizer(self, _: Adamw) -> Self { self } pub const fn epochs(mut self, value: usize) -> Self {
self.epochs = value; self } pub const fn lr(mut self, value: f64) -> Self {
self.learning_rate = value; self }
pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self {
self.log_metrics = metrics.into(); self } pub fn save(mut self, path: impl Into<String>) -> Self {
self.save = Some(path.into()); self } pub fn resume(mut self, path: impl Into<String>) -> Self {
self.resume = Some(path.into()); self } pub fn run(&self, model: &Model, data: &Data) -> TrainingReport {
self.try_run(model, data).unwrap_or_else(|error| panic!("{error}")) }
fn try_run(&self, model: &Model, data: &Data) -> Result<TrainingReport> { let backend = device_backend()?;
let config = Config::load()?;
let prepared = prepare(data)?;
let training_rows = ((prepared.rows as f64) * data.split).floor() as usize;
require(training_rows != 0 && training_rows <= prepared.rows, "split must select training rows")?;
let run = RUN.fetch_add(1, Ordering::Relaxed) + 1;
let mut graph = compile(model, prepared, training_rows, backend, config)?;
let output = graph.output.elements();
require(output == 1, "model output width must be one")?; if let Some(path) = &self.resume {
graph.parameters = load_weights(path, &graph.parameters, model, &prepared.schema)?;
eprintln!("resumed: {path}"); }
let samples = &prepared.samples[..training_rows * prepared.features];
let targets = &prepared.targets[..training_rows];
let mut tape = DeviceTape::new(&graph, samples, targets, backend, config)?;
tape.forward()?;
let initial_predictions = tape.predictions()?;
let initial_loss = model_loss(&initial_predictions, targets, model.loss, config.activation[7]);
let tolerance = self.stop.unwrap_or(0.0);
require(tolerance.is_finite() && (0.0..=1.0).contains(&tolerance), "stop must be between zero and one")?;
SIGNAL.get_or_init(|| unsafe { signal(2, interrupt);
});
let mut interrupted = false; for epoch in 1..=self.epochs {
let started = Instant::now();
let (loss, checkpoint) = tape.epoch(epoch, self.learning_rate, model.loss, tolerance, config)?;
let predictions = tape.predictions()?; if checkpoint && let Some(path) = &self.save {
save_weights(path, model, &prepared.schema, &tape.weights(true)?)?; }
self.print(model, run, epoch, loss, targets, &predictions, started, checkpoint);
if INTERRUPTED.swap(false, Ordering::Relaxed) { interrupted = true;
break; } } if self.stop.is_some() || interrupted {
tape.restore_best()?; }
tape.forward()?;
let predictions = tape.predictions()?;
let final_loss = model_loss(&predictions, targets, model.loss, config.activation[7]); if let Some(path) = &self.save {
let best = self.stop.is_some() || interrupted;
save_weights(path, model, &prepared.schema, &tape.weights(best)?)?; }
Ok(TrainingReport(initial_loss, final_loss, initial_predictions, predictions)) } fn print( &self, model: &Model,
run: u64, epoch: usize, loss: f64, targets: &[f64], predictions: &[f64], started: Instant, checkpoint: bool, ) {
if self.log_metrics.is_empty() { return; }
let topology = model.description(&self.log_metrics);
let r2 = coefficient(targets, predictions);
let time = started.elapsed().as_secs_f64() * 1000.0;
let mut values = Vec::new();
let mut topology_printed = false; for metric in &self.log_metrics { let value = match metric.0 {
0 => format!("run \x1b[38\x3b2\x3b242\x3b40\x3b60m{run:>5}\x1b[0m"),
1 => format!("{} \x1b[38\x3b2\x3b0\x3b174\x3b107m{loss:.4}\x1b[0m", model.loss.name()),
2 => format!("r2 \x1b[38\x3b2\x3b39\x3b125\x3b255m{r2:>7.4}\x1b[0m"),
3 => format!("time \x1b[38\x3b2\x3b255\x3b194\x3b0m{time:>9.3} ms\x1b[0m"),
4 => format!("epoch \x1b[38\x3b2\x3b135\x3b90\x3b251m{epoch}\x1b[0m"),
5..=7 if !topology_printed && !topology.is_empty() => { topology_printed = true; topology.clone() } 5..=7 => continue,
_ => unreachable!(), };
values.push(value); } if checkpoint && self.stop.is_some() {
values.push("\x1b[1\x3b32m← checkpoint\x1b[0m".to_owned()); }
eprintln!("{}", values.join("  ")); } }
pub struct TrainingReport(f64, f64, Vec<f64>, Vec<f64>); impl TrainingReport { pub const fn initial_loss(&self) -> f64 {
self.0 } pub const fn final_loss(&self) -> f64 { self.1 } pub fn initial_predictions(&self) -> &[f64] { &self.2 }
pub fn predictions(&self) -> &[f64] { &self.3 } }
fn model_loss(predictions: &[f64], targets: &[f64], loss: LossFunction, threshold: f64) -> f64 {
let values = predictions.iter().zip(targets);
let mut result = values.map(|(prediction, target)| loss.value(*prediction, *target, threshold)).sum::<f64>()
/ targets.len() as f64; if loss.0 == 1 {
result = result.sqrt(); } result } fn coefficient(targets: &[f64], predictions: &[f64]) -> f64 {
let mean = targets.iter().sum::<f64>() / targets.len() as f64;
let residual = targets.iter().zip(predictions).map(|(target, value)| (target - value).powi(2)).sum::<f64>();
let total = targets.iter().map(|target| (target - mean).powi(2)).sum::<f64>();
if total == 0.0 { 0.0 } else { 1.0 - residual / total } }
fn save_weights(path: &str, model: &Model, schema: &str, weights: &[f64]) -> Result<()> {
require(path.ends_with(".ogdl"), "save requires an .ogdl model")?;
let weights = weights.iter().map(f64::to_string).collect::<Vec<_>>().join(" ");
let signature = format!("{:?}", model.blocks);
let document = format!("recipe-model\n    schema {schema}\n    model {signature}\n    weights {weights}\n");
fs::write(path, document).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}")))?;
eprintln!("saved: {path}"); Ok(()) }
fn load_weights(path: &str, initial: &[f64], model: &Model, schema: &str) -> Result<Vec<f64>> {
require(path.ends_with(".ogdl"), "resume requires an .ogdl model")?;
if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
save_weights(path, model, schema, initial)?; } let document =
fs::read_to_string(path).map_err(|error| RecipeError::new(format!("cannot resume {path}: {error}")))?;
let stored_schema = document.lines().find_map(|line| line.trim().strip_prefix("schema "));
let stored_model = document.lines().find_map(|line| line.trim().strip_prefix("model ")); let values = document .lines()
.find_map(|line| line.trim().strip_prefix("weights ")) .ok_or_else(|| RecipeError::new("model has no weights"))?;
let weights = values .split_whitespace()
.map(|value| value.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid weight: {error}"))))
.collect::<Result<Vec<_>>>()?;
let signature = format!("{:?}", model.blocks); let matches =
stored_schema == Some(schema) && stored_model == Some(signature.as_str()) && weights.len() == initial.len();
if matches { return Ok(weights); }
eprint!("mismatch: overwrite {path}? Y/n "); std::io::Write::flush(&mut std::io::stderr())
.map_err(|error| RecipeError::new(format!("cannot prompt: {error}")))?;
let mut answer = String::new(); std::io::stdin() .read_line(&mut answer)
.map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?;
require(answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y"), "model mismatch not overwritten")?;
save_weights(path, model, schema, initial)?; Ok(initial.to_vec()) } fn estimator_predict( operation: &Operation,
data: &Prepared, training_rows: usize, backend: Backend, config: Config, state: Option<&[f64]>, exclude_self: bool,
) -> Result<(Vec<f64>, Vec<f64>)> { let test_rows = data.rows - training_rows;
require(test_rows != 0, "estimator split must retain test rows")?;
let (kind, argument, workspace, state_size) = match operation { Operation::KMeans(clusters) => {
require(*clusters != 0 && *clusters <= training_rows, "kmeans cluster count is invalid")?;
(0, *clusters, clusters * data.features + 2 * training_rows, clusters * data.features) } Operation::Knn(neighbors) => {
let maximum = training_rows - usize::from(exclude_self);
require(*neighbors != 0 && *neighbors <= maximum, "knn neighbor count is invalid")?;
(1, *neighbors, 2 * neighbors * test_rows, training_rows * (data.features + 1)) }
_ => return Err(RecipeError::new("operation is not a supported estimator")), };
let gpu = gpu(backend)?; if let Some(state) = state {
require(state.len() == state_size, "saved estimator state has the wrong size")?; }
let mut sample_values = data.samples.clone();
let mut target_values = data.targets[..training_rows].to_vec(); if kind == 1 && let Some(state) = state {
let sample_count = training_rows * data.features;
sample_values = state[..sample_count].to_vec();
sample_values.extend_from_slice(&data.samples[training_rows * data.features..]);
target_values.copy_from_slice(&state[sample_count..]); }
let samples = Buffer::upload(gpu, &sample_values)?;
let targets = Buffer::upload(gpu, &target_values)?;
let output = Buffer::new(gpu, checked_mul(test_rows, size_of::<f64>(), "estimator output")?)?;
let mut workspace_values = vec![0.0;
workspace];
if kind == 0 && let Some(state) = state { workspace_values[..state.len()].copy_from_slice(state); }
let workspace_buffer = Buffer::upload(gpu, &workspace_values)?; let mut call =
[samples.pointer, targets.pointer, output.pointer, workspace_buffer.pointer].map(|value| value as Ptr);
let operation = kind + 2 * usize::from(state.is_some()) + 4 * usize::from(exclude_self); let mut scalars =
[training_rows, test_rows, data.features, operation, argument, config.kmeans_iterations, config.threads]
.map(|value| narrow(value, "estimator argument").map(|value| value as u32)) .into_iter() .collect::<Result<Vec<_>>>()?;
let mut arguments = call.iter_mut().map(|value| value as *mut _ as Ptr).collect::<Vec<_>>();
arguments.extend(scalars.iter_mut().map(|value| value as *mut _ as Ptr));
gpu.launch(gpu.estimate, config.threads as u32, &mut arguments)?;
let predictions = output.download(test_rows)?; let fitted = if kind == 0 { workspace_buffer.download(state_size)?
} else { let mut fitted = sample_values[..training_rows * data.features].to_vec();
fitted.extend(target_values); fitted
}; Ok((predictions, fitted)) } struct DeviceTape { gpu: &'static Gpu, values: Vec<Buffer>, _contexts: Vec<Buffer>,
_adjoints: Vec<Buffer>, samples: Buffer, targets: Buffer, weights: Buffer, frozen: Buffer, best: Buffer,
moments: Buffer, variances: Buffer, gradient: Buffer, metrics: Buffer, best_loss: Buffer, value_pointers: Buffer,
context_pointers: Buffer, adjoint_pointers: Buffer, descriptors: Buffer, arguments: Buffer, rows: u32, nodes: u32,
parameters: u32, threads: u32, output: usize, } impl DeviceTape {
fn new(graph: &Graph, samples: &[f64], targets: &[f64], backend: Backend, config: Config) -> Result<Self> {
let gpu = gpu(backend)?;
let mut descriptors = Vec::new();
let mut arguments = Vec::new();
let mut values = Vec::new();
let mut contexts = Vec::new();
let mut adjoints = Vec::new();
let program_base = checked_mul(graph.nodes.len(), 9, "node arguments")?; for node in &graph.nodes {
descriptors.extend(node_descriptor(node, program_base)?);
arguments.extend(node.argument);
let elements = graph_rows_buffer(node.output, targets.len().max(1))?;
values.push(Buffer::new(gpu, elements)?);
let empty = vec![0_u8;
elements];
adjoints.push(Buffer::upload(gpu, &empty)?);
contexts.push(Buffer::new(gpu, node_context(node, targets.len().max(1))?)?); }
arguments.extend(&graph.programs);
let addresses = |buffers: &[Buffer]| buffers.iter().map(|buffer| buffer.pointer).collect::<Vec<_>>();
let zeros = vec![0.0;
graph.parameters.len().max(1)];
let target_values = if targets.is_empty() { vec![0.0] } else { targets.to_vec() }; Ok(Self { gpu,
value_pointers: Buffer::upload(gpu, &addresses(&values))?,
context_pointers: Buffer::upload(gpu, &addresses(&contexts))?,
adjoint_pointers: Buffer::upload(gpu, &addresses(&adjoints))?, descriptors: Buffer::upload(gpu, &descriptors)?,
arguments: Buffer::upload(gpu, &arguments)?, samples: Buffer::upload(gpu, samples)?,
targets: Buffer::upload(gpu, &target_values)?, weights: Buffer::upload(gpu, &graph.parameters)?,
frozen: Buffer::upload(gpu, if graph.frozen.is_empty() { &[1] } else { &graph.frozen })?,
best: Buffer::upload(gpu, &graph.parameters)?, moments: Buffer::upload(gpu, &zeros)?,
variances: Buffer::upload(gpu, &zeros)?, gradient: Buffer::upload(gpu, &zeros)?,
metrics: Buffer::upload(gpu, &[0.0, 0.0, 0.0])?,
best_loss: Buffer::upload(gpu, &[f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY])?,
rows: narrow(targets.len().max(1), "GPU rows")? as u32, nodes: narrow(graph.nodes.len(), "GPU nodes")? as u32,
parameters: narrow(graph.parameters.len(), "GPU parameters")? as u32,
threads: narrow(config.threads, "GPU threads")? as u32, output: graph.output.elements(), values, _contexts: contexts,
_adjoints: adjoints, }) } fn forward(&mut self) -> Result<()> { let mut arguments = self.forward_arguments();
self.gpu.launch(self.gpu.forward, self.threads, &mut arguments) } fn forward_arguments(&mut self) -> [*mut c_void; 9] {
[ &mut self.samples.pointer as *mut _ as Ptr, &mut self.weights.pointer as *mut _ as Ptr,
&mut self.value_pointers.pointer as *mut _ as Ptr, &mut self.context_pointers.pointer as *mut _ as Ptr,
&mut self.descriptors.pointer as *mut _ as Ptr, &mut self.arguments.pointer as *mut _ as Ptr,
&mut self.rows as *mut _ as Ptr, &mut self.nodes as *mut _ as Ptr, &mut self.threads as *mut _ as Ptr, ] }
fn predictions(&self) -> Result<Vec<f64>> { self.values .last() .ok_or_else(|| RecipeError::new("GPU tape is empty"))?
.download(self.rows as usize * self.output) } fn epoch( &mut self, step: usize, rate: f64, loss: LossFunction,
tolerance: f64, config: Config, ) -> Result<(f64, bool)> { let mut loss = loss.0 as u32;
let mut huber_threshold = config.activation[7];
let mut step = narrow(step, "optimizer step")? as u32;
let mut rate = rate;
let mut beta1 = config.beta1;
let mut beta2 = config.beta2;
let mut beta1_power = beta1.powi(step as i32);
let mut beta2_power = beta2.powi(step as i32);
let mut epsilon = config.epsilon;
let mut decay = config.decay;
let mut tolerance = tolerance; let mut call = [ &mut self.samples.pointer as *mut _ as Ptr,
&mut self.targets.pointer as *mut _ as Ptr, &mut self.weights.pointer as *mut _ as Ptr,
&mut self.frozen.pointer as *mut _ as Ptr, &mut self.best.pointer as *mut _ as Ptr,
&mut self.value_pointers.pointer as *mut _ as Ptr, &mut self.context_pointers.pointer as *mut _ as Ptr,
&mut self.adjoint_pointers.pointer as *mut _ as Ptr, &mut self.descriptors.pointer as *mut _ as Ptr,
&mut self.arguments.pointer as *mut _ as Ptr, &mut self.metrics.pointer as *mut _ as Ptr,
&mut self.gradient.pointer as *mut _ as Ptr, &mut self.moments.pointer as *mut _ as Ptr,
&mut self.variances.pointer as *mut _ as Ptr, &mut self.best_loss.pointer as *mut _ as Ptr,
&mut self.rows as *mut _ as Ptr, &mut self.nodes as *mut _ as Ptr, &mut self.parameters as *mut _ as Ptr,
&mut loss as *mut _ as Ptr, &mut huber_threshold as *mut _ as Ptr, &mut rate as *mut _ as Ptr,
&mut beta1 as *mut _ as Ptr, &mut beta2 as *mut _ as Ptr, &mut beta1_power as *mut _ as Ptr,
&mut beta2_power as *mut _ as Ptr, &mut epsilon as *mut _ as Ptr, &mut decay as *mut _ as Ptr,
&mut tolerance as *mut _ as Ptr, &mut step as *mut _ as Ptr, &mut self.threads as *mut _ as Ptr, ];
self.gpu.launch(self.gpu.epoch, self.threads, &mut call)?;
let metrics = self.metrics.download::<f64>(3)?; Ok((metrics[0], metrics[1] != 0.0)) }
fn weights(&self, best: bool) -> Result<Vec<f64>> { if best { self.best.download(self.parameters as usize) } else {
self.weights.download(self.parameters as usize) } } fn restore_best(&mut self) -> Result<()> {
self.weights = Buffer::upload(self.gpu, &self.weights(true)?)?; Ok(()) } }
fn node_descriptor(node: &Node, program_base: usize) -> Result<[i32; 11]> {
let program_offset = if node.program_count == 0 { 0 } else {
checked_add(program_base, node.program_offset, "scalar program offset")? }; Ok([ node.op as i32, node.source,
node.second, narrow(node.input.channels, "input channels")?, narrow(node.input.length, "input length")?,
narrow(node.output.channels, "output channels")?, narrow(node.output.length, "output length")?,
narrow(node.offset, "weight offset")?, narrow(node.parameters, "parameter count")?,
narrow(program_offset, "program offset")?, narrow(node.program_count, "scalar instruction count")?, ]) }
fn graph_rows_buffer(shape: Shape, rows: usize) -> Result<usize> {
checked_mul(checked_mul(rows, shape.elements(), "node elements")?, size_of::<f64>(), "node bytes") }
fn node_context(node: &Node, rows: usize) -> Result<usize> { let elements = match node.op {
Primitive::Elementwise => checked_mul( 2 * node.program_count,
checked_mul(rows, node.output.elements(), "program batch")?, "program", )?, Primitive::Attention => {
checked_mul(6, checked_mul(rows, node.output.elements(), "attention context")?, "attention")? } Primitive::Scan => {
let state_count = checked_mul(rows, node.output.elements(), "scan batch")?;
let gates = node.argument[0] as usize;
let states = checked_mul(2 * gates + 1, state_count, "scan states")?;
let gradients = checked_mul(rows, node.parameters, "scan gradients")?;
checked_add(states, checked_add(gradients, 2 * rows * node.output.channels, "scan scratch")?, "scan")? }
Primitive::Pool => checked_mul(rows, node.output.elements(), "pool context")?, Primitive::Normalize => {
let groups = node.output.channels.max(checked_mul(rows, node.output.length, "layer groups")?);
checked_mul(4, groups, "normalization context")? } Primitive::Estimator => {
checked_mul(checked_mul(rows, node.argument[1] as usize, "estimator rows")?, 2, "estimator context")? } _ => 1, };
checked_mul(elements.max(1), size_of::<f64>(), "context bytes") } fn narrow(value: usize, role: &str) -> Result<i32> {
i32::try_from(value).map_err(|_| RecipeError::new(format!("{role} exceeds i32"))) } struct Buffer {
runtime: &'static Gpu, pointer: u64, } impl Buffer { fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
let mut pointer = 0;
runtime.status(unsafe { (runtime.allocate)(&mut pointer, bytes) }, "allocation")?; Ok(Self { runtime, pointer }) }
fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
let buffer = Self::new(runtime, size_of_val(values))?; runtime.status(
unsafe { (runtime.upload)(buffer.pointer, values.as_ptr().cast(), size_of_val(values)) }, "upload", )?; Ok(buffer) }
fn download<T: Copy + Default>(&self, count: usize) -> Result<Vec<T>> {
self.runtime.status(unsafe { (self.runtime.synchronize)() }, "synchronization")?;
let mut values = vec![T::default();
count];
self.runtime.status(
unsafe { (self.runtime.download)(values.as_mut_ptr().cast(), self.pointer, size_of_val(&*values)) }, "download", )?;
Ok(values) } } impl Drop for Buffer { fn drop(&mut self) { unsafe { (self.runtime.free)(self.pointer); } } }
struct Gpu { backend: Backend, allocate: unsafe extern "C" fn(*mut u64, usize) -> i32,
free: unsafe extern "C" fn(u64) -> i32, upload: unsafe extern "C" fn(u64, *const c_void, usize) -> i32,
download: unsafe extern "C" fn(Ptr, u64, usize) -> i32, synchronize: unsafe extern "C" fn() -> i32, launch: Launch,
forward: usize, epoch: usize, estimate: usize, }
type Launch = unsafe extern "C" fn(usize, u32, u32, u32, u32, u32, u32, u32, Ptr, *mut Ptr, *mut Ptr) -> i32;
type Init = unsafe extern "C" fn(u32) -> i32;
type Count = unsafe extern "C" fn(*mut i32) -> i32;
type Attribute = unsafe extern "C" fn(*mut i32, i32, i32) -> i32; #[cfg(feature = "amd")]
type Select = unsafe extern "C" fn(i32) -> i32; #[cfg(feature = "nvidia")]
type Device = unsafe extern "C" fn(*mut i32, i32) -> i32; #[cfg(feature = "nvidia")]
type Context = unsafe extern "C" fn(*mut Ptr, u32, i32) -> i32;
type Module = unsafe extern "C" fn(*mut Ptr, *const u8) -> i32;
type Function = unsafe extern "C" fn(*mut usize, Ptr, *const u8) -> i32;
struct Library(Ptr); impl Library { fn open(name: &str) -> Result<Self> {
let name = format!("{name}\0");
let handle = unsafe { dlopen(name.as_ptr().cast(), 2) };
require(!handle.is_null(), format!("cannot load {name:?}"))?; Ok(Self(handle)) }
fn function<F: Copy>(&self, name: &[u8]) -> Result<F> { let pointer = unsafe { dlsym(self.0, name.as_ptr().cast()) };
require(!pointer.is_null(), format!("runtime symbol {:?} is absent", name))?;
Ok(unsafe { std::mem::transmute_copy(&pointer) }) } } impl Gpu {
fn status(&self, status: i32, action: &str) -> Result<()> { (status == 0) .then_some(())
.ok_or_else(|| RecipeError::new(format!("{:?} {action} failed: {status}", self.backend))) }
fn launch(&self, function: usize, threads: u32, arguments: &mut [*mut c_void]) -> Result<()> {
let stream: Ptr = ptr::null_mut();
let extra: *mut Ptr = ptr::null_mut(); let status =
unsafe { (self.launch)(function, 1, 1, 1, threads, 1, 1, 0, stream, arguments.as_mut_ptr(), extra) };
self.status(status, "dispatch") } } static AMD: OnceLock<Result<Gpu>> = OnceLock::new();
static NVIDIA: OnceLock<Result<Gpu>> = OnceLock::new(); fn device_backend() -> Result<Backend> {
let mut failures = Vec::new(); for backend in [Backend::Amd, Backend::Nvidia] { match gpu(backend) {
Ok(_) => return Ok(backend), Err(error) => failures.push(error.to_string()), } }
Err(RecipeError::new(failures.join("; "))) } fn gpu(backend: Backend) -> Result<&'static Gpu> {
let loaded = match backend { Backend::Amd => AMD.get_or_init(load_amd),
Backend::Nvidia => NVIDIA.get_or_init(load_nvidia), }; loaded.as_ref().map_err(Clone::clone) }
fn discrete(backend: Backend, count: i32, mut probe: impl FnMut(i32) -> Result<Option<i32>>) -> Result<i32> { (0..count)
.map(&mut probe) .find_map(|result| result.transpose()) .transpose()?
.ok_or_else(|| RecipeError::new(format!("{backend:?} has no discrete GPU"))) } fn load_amd() -> Result<Gpu> {
#[cfg(not(feature = "amd"))] return Err(RecipeError::new("AMD support is not compiled into this build"));
#[cfg(feature = "amd")] unsafe { const INTEGRATED: i32 = 16;
let runtime = Library::open(env!("RECIPE_HSA_RUNTIME"))?;
let init: Init = runtime.function(b"hipInit\0")?;
let count_devices: Count = runtime.function(b"hipGetDeviceCount\0")?;
let attribute: Attribute = runtime.function(b"hipDeviceGetAttribute\0")?;
let select: Select = runtime.function(b"hipSetDevice\0")?;
let load: Module = runtime.function(b"hipModuleLoad\0")?;
let function: Function = runtime.function(b"hipModuleGetFunction\0")?;
let mut count = 0;
let mut module = ptr::null_mut();
let mut forward = 0;
let mut epoch = 0;
let mut estimate = 0; let gpu = Gpu { backend: Backend::Amd, allocate: runtime.function(b"hipMalloc\0")?,
free: runtime.function(b"hipFree\0")?, upload: runtime.function(b"hipMemcpyHtoD\0")?,
download: runtime.function(b"hipMemcpyDtoH\0")?, synchronize: runtime.function(b"hipDeviceSynchronize\0")?,
launch: runtime.function(b"hipModuleLaunchKernel\0")?, forward: 0, epoch: 0, estimate: 0, };
gpu.status(init(0), "initialization")?;
gpu.status(count_devices(&mut count), "device enumeration")?; let device = discrete(Backend::Amd, count, |device| {
let mut integrated = 0;
gpu.status(attribute(&mut integrated, INTEGRATED, device), "device probe")?; Ok((integrated == 0).then_some(device))
})?;
gpu.status(select(device), "device selection")?;
gpu.status(load(&mut module, concat!(env!("RECIPE_HSA_CODE_OBJECT"), "\0").as_ptr()), "module load")?;
gpu.status(function(&mut forward, module, b"forward_graph\0".as_ptr()), "forward load")?;
gpu.status(function(&mut epoch, module, b"tape_epoch_graph\0".as_ptr()), "epoch load")?;
gpu.status(function(&mut estimate, module, b"estimate_graph\0".as_ptr()), "estimator load")?;
Ok(Gpu { forward, epoch, estimate, ..gpu }) } } fn load_nvidia() -> Result<Gpu> { #[cfg(not(feature = "nvidia"))]
return Err(RecipeError::new("NVIDIA support is not compiled into this build")); #[cfg(feature = "nvidia")] unsafe {
const INTEGRATED: i32 = 18;
let runtime = Library::open(env!("RECIPE_NV_RUNTIME"))?;
let init: Init = runtime.function(b"cuInit\0")?;
let count_devices: Count = runtime.function(b"cuDeviceGetCount\0")?;
let get_device: Device = runtime.function(b"cuDeviceGet\0")?;
let attribute: Attribute = runtime.function(b"cuDeviceGetAttribute\0")?;
let create: Context = runtime.function(b"cuCtxCreate_v2\0")?;
let load: Module = runtime.function(b"cuModuleLoad\0")?;
let function: Function = runtime.function(b"cuModuleGetFunction\0")?;
let mut count = 0;
let mut context = ptr::null_mut();
let mut module = ptr::null_mut();
let mut forward = 0;
let mut epoch = 0;
let mut estimate = 0; let gpu = Gpu { backend: Backend::Nvidia, allocate: runtime.function(b"cuMemAlloc_v2\0")?,
free: runtime.function(b"cuMemFree_v2\0")?, upload: runtime.function(b"cuMemcpyHtoD_v2\0")?,
download: runtime.function(b"cuMemcpyDtoH_v2\0")?, synchronize: runtime.function(b"cuCtxSynchronize\0")?,
launch: runtime.function(b"cuLaunchKernel\0")?, forward: 0, epoch: 0, estimate: 0, };
gpu.status(init(0), "initialization")?;
gpu.status(count_devices(&mut count), "device enumeration")?; let device = discrete(Backend::Nvidia, count, |ordinal| {
let mut device = 0;
let mut integrated = 0;
gpu.status(get_device(&mut device, ordinal), "device enumeration")?;
gpu.status(attribute(&mut integrated, INTEGRATED, device), "device probe")?; Ok((integrated == 0).then_some(device))
})?;
gpu.status(create(&mut context, 0, device), "context creation")?;
gpu.status(load(&mut module, concat!(env!("RECIPE_NV_MODULE"), "\0").as_ptr()), "module load")?;
gpu.status(function(&mut forward, module, b"forward_graph\0".as_ptr()), "forward load")?;
gpu.status(function(&mut epoch, module, b"tape_epoch_graph\0".as_ptr()), "epoch load")?;
gpu.status(function(&mut estimate, module, b"estimate_graph\0".as_ptr()), "estimator load")?;
Ok(Gpu { forward, epoch, estimate, ..gpu }) } } #[link(name = "dl")] unsafe extern "C" {
fn dlopen(name: *const c_char, flags: i32) -> Ptr;
fn dlsym(handle: Ptr, name: *const c_char) -> Ptr;
fn signal(number: i32, handler: extern "C" fn(i32)) -> usize; }

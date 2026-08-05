//! Recipe lowers public tensor declarations to shapes, transfers, calculations, and a roofline schedule, then executes the same calculations on the host.
#![allow(non_upper_case_globals)]
use std::sync::atomic::{AtomicU64, Ordering}; use std::{
	collections::{BTreeMap, BTreeSet},
	error::Error,
	ffi::{c_char, c_void},
	fmt, fs,
	path::{Path, PathBuf},
	ptr,
	sync::OnceLock,
	time::Instant,
}; const BALANCE_BYTES_PER_FLOP: &str = env!("RECIPE_BALANCE_BYTES_PER_FLOP"); static RUN: AtomicU64 = AtomicU64::new(0); #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DType {
	F32 = 0,
	F64 = 1,
	I32 = 2,
	I64 = 3,
}
impl DType {
	const fn bytes(self) -> u64 {
		4 << (self as u8 & 1) }
	const fn is_float(self) -> bool {
		matches!(self, Self::F32 | Self::F64) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Value(usize); pub static recipe: Recipe = Recipe::new(); pub struct Data {
	sources: Vec<String>,
	target: String,
	exclusions: Vec<String>,
	normalize: bool,
	split: f64,
	prepared: OnceLock<Result<Prepared, RecipeError>>,
	device: OnceLock<Result<DeviceData, RecipeError>>,
}
pub trait IntoDataSources {
	fn into_data_sources(self) -> Vec<String>; }
macro_rules! data_sources {
	($( [$($generic:tt)*] $source:ty),+) => {$(impl<$($generic)*> IntoDataSources for $source {
		fn into_data_sources(self) -> Vec<String> { self.into_iter().map(Into::into).collect() } })+}; ($($source:ty),+) => {$(impl IntoDataSources for $source {
		fn into_data_sources(self) -> Vec<String> { vec![self.into()] } })+}; }
data_sources!(&str, String); data_sources!([T: Into<String>, const N: usize] [T; N], [T: Into<String>] Vec<T>); macro_rules! data_methods {
	($receiver:ident; $(fn $method:ident($argument:ident: $type:ty) => $update:expr;)+ $([const] fn $const_method:ident($const_argument:ident: $const_type:ty) => $field:ident = $value:expr;)+) => {impl Data {$(pub fn $method(mut self, $argument: $type) -> Self { let $receiver = &mut self; $update; self })+ $(pub const fn $const_method(mut self, $const_argument: $const_type) -> Self { self.$field = $value; self })+}}; }
data_methods! {
	data; fn target(target: impl Into<String>) => data.target = target.into(); fn exclude(names: impl IntoDataSources) => data.exclusions = names.into_data_sources(); fn set(source: impl Into<String>) => data.sources.push(source.into()); [const] fn norm(_value: ZScore) => normalize = true; [const] fn split(fraction: f64) => split = fraction; }
pub struct Model {
	blocks: Vec<Block>,
	loss: LossFunction,
}
macro_rules! operation_blocks {
	($input:ident, $output:ident, $recurrent:ident; $($variant:ident($($field:ident: $field_type:ty),+) => $method:ident($($argument:ident: $type:ty),*) = ($($value:expr),*); width $width:expr; parameters $parameters:expr; name $name:expr;)+ $(@ $plain:ident($($plain_field:ident: $plain_type:ty),+); width $plain_width:expr; parameters $plain_parameters:expr; name $plain_name:expr;)+) => {
		#[derive(Clone, Debug)] enum OperationBlock { $($variant($($field_type),+),)+ $($plain($($plain_type),+),)+ }
		impl Model {
			$(pub fn $method(self, $($argument: $type),*) -> Self { self.push(OperationBlock::$variant($($value),*)) })+
			fn push(mut self, operation: OperationBlock) -> Self { self.blocks.push(Block { operation, activation: Activation::Linear, normalization: None }); self }
			pub fn residual<const N: usize>(self, parts: [Residual; N]) -> Self { self.push(OperationBlock::Residual(parts.into())) }
			pub const fn loss(mut self, loss: LossFunction) -> Self { self.loss = loss; self }
			fn graph(&self, features: usize) -> Result<TrainingGraph<'_>, RecipeError> { if self.blocks.is_empty() { return Err(RecipeError::new("training model must contain a block")); } let mut widths = vec![features]; let mut shapes = vec![(1_usize, features)]; let mut offsets = Vec::with_capacity(self.blocks.len()); let mut parameters = 0_usize; for block in &self.blocks { let input = widths[widths.len() - 1]; let (channels, length) = shapes[shapes.len() - 1]; let output = match &block.operation { OperationBlock::Conv(filters, kernel) => if *kernel == 0 { 0 } else { filters.saturating_mul(length.checked_sub(*kernel).map_or(0, |positions| positions + 1)) }, OperationBlock::Pool(size) => if *size == 0 { 0 } else { channels.saturating_mul(length.div_ceil(*size)) }, _ => block.operation.output(input)? }; if output == 0 { return Err(RecipeError::new("model block dimensions must be positive")); } offsets.push(parameters); let block_parameters = match &block.operation { OperationBlock::Conv(filters, kernel) => filters.checked_mul(channels).and_then(|value| value.checked_mul(*kernel)), _ => Some(block.operation.parameters(input, output)?) }.ok_or_else(|| RecipeError::new("model parameter count overflow"))?; parameters = parameters.checked_add(block_parameters).ok_or_else(|| RecipeError::new("model parameter count overflow"))?; widths.push(output); shapes.push(match &block.operation { OperationBlock::Conv(filters, kernel) => (*filters, length - kernel + 1), OperationBlock::Pool(size) => (channels, length.div_ceil(*size)), _ => (1, output) }); } if widths.last() != Some(&1) { return Err(RecipeError::new("training model output must contain one value")); } let matrix_parameters = parameters; parameters += self.blocks.iter().filter(|block| block.activation == Activation::PRelu).count(); Ok(TrainingGraph { blocks: &self.blocks, widths, shapes, offsets, matrix_parameters, parameters }) }
			fn signature(&self) -> String { format!("{:?}", self.blocks) }
			fn description(&self, operation: bool, activation: bool, normalization: bool) -> String { self.blocks.iter().filter_map(|block| { let mut parts = Vec::new(); if operation { parts.push(block.operation.name()); } if activation && block.activation != Activation::Linear { parts.push(block.activation.name()); } if normalization && let Some(value) = block.normalization { parts.push(value.name()); } (!parts.is_empty()).then(|| parts.join(".")) }).collect::<Vec<_>>().join("/") } }
		impl OperationBlock { fn name(&self) -> &'static str { match self { $(Self::$variant($($field),+) => $name,)+ $(Self::$plain($($plain_field),+) => $plain_name,)+ } } fn output(&self, $input: usize) -> Result<usize, RecipeError> { let $output = match self { $(Self::$variant($($field),+) => $width,)+ $(Self::$plain($($plain_field),+) => $plain_width,)+ }; ($output != 0).then_some($output).ok_or_else(|| RecipeError::new("model block dimensions must be positive")) } fn parameters(&self, $input: usize, $output: usize) -> Result<usize, RecipeError> { let $recurrent = |width: usize, gates: usize| $input.checked_mul(width).and_then(|a| width.checked_mul(width).and_then(|b| a.checked_add(b))).and_then(|value| value.checked_add(width)).and_then(|value| value.checked_mul(gates)); match self { $(Self::$variant($($field),+) => $parameters,)+ $(Self::$plain($($plain_field),+) => $plain_parameters,)+ }.ok_or_else(|| RecipeError::new("model parameter count overflow")) } } }; }
operation_blocks! {
	input, output, recurrent; Layer(_width: usize) => layer(value: usize) = (value); width *_width; parameters input.checked_mul(output); name "layer"; Conv(_filters: usize, _kernel: usize) => conv(filters: usize, kernel: usize) = (filters, kernel); width _filters.saturating_mul(input.checked_sub(*_kernel).map_or(0, |length| length + 1)); parameters _filters.checked_mul(*_kernel); name "conv"; Pool(_size: usize) => pool(value: usize) = (value); width input.div_ceil(*_size); parameters Some(0); name "pool"; KMeans(_width: usize) => kmeans(value: usize) = (value); width *_width; parameters input.checked_mul(output); name "kmeans"; Embed(_dimensions: usize, _vocabulary: usize) => embed(value: usize) = (value, 0); width input.saturating_mul(*_dimensions); parameters if *_vocabulary != 0 { _dimensions.checked_mul(*_vocabulary) } else { return Err(RecipeError::new("embedding vocabulary must be positive")); }; name "embed"; Attention(_heads: usize) => attn(value: usize) = (value); width input; parameters return Err(RecipeError::new("attention requires genuine Q, K, V lowering")); name "attn"; Rnn(_width: usize) => rnn(value: usize) = (value); width *_width; parameters recurrent(*_width, 1); name "rnn"; Gru(_width: usize) => gru(value: usize) = (value); width *_width; parameters recurrent(*_width, 3); name "gru"; Lstm(_width: usize) => lstm(value: usize) = (value); width *_width; parameters recurrent(*_width, 4); name "lstm"; Forest(_trees: usize, _kind: u8, _depth: usize) => forest(value: usize) = (value, 0, 0); width usize::from(*_trees != 0); parameters input.checked_mul(output); name match *_kind { 1 => "lgbm", 2 => "xgbst", _ => "forest" }; Knn(_neighbors: usize) => knn(value: usize) = (value); width 1; parameters input.checked_mul(output); name "knn"; Perceptron(_width: usize) => perc(value: usize) = (value); width *_width; parameters input.checked_mul(output); name "perc"; @ Residual(_parts: Vec<Residual>); width _parts.iter().filter_map(|part| if let Residual::Layer(width) = part { Some(*width) } else { None }).sum(); parameters input.checked_mul(output); name "residual"; }
#[derive(Clone, Debug)]
struct Block {
	operation: OperationBlock,
	activation: Activation,
	normalization: Option<BlockNormalization>,
}
#[derive(Clone, Copy, Debug)]
enum BlockNormalization {
	Batch,
	Layer,
}
#[derive(Clone, Debug)]
pub enum Residual {
	Layer(usize),
	Relu,
}
macro_rules! residuals {
	($($function:ident($($argument:ident: $type:ty),*) => $value:expr),+) => {$(pub const fn $function($($argument: $type),*) -> Residual { $value })+}; }
residuals!(layer(width: usize) => Residual::Layer(width), relu() => Residual::Relu); macro_rules! activations {
	($($method:ident: $activation:ident = $code:literal),+) => {#[derive(Clone, Copy, Debug, PartialEq, Eq)] #[allow(dead_code)] #[repr(u8)] enum Activation { Linear = 0, LegacySignedLogOnePlus = 5, $($activation = $code),+ } struct ActivationConfig { leak: f64, prelu: f64, elu: f64, selu_alpha: f64, selu_scale: f64, gelu_scale: f64, gelu_cubic: f64 } impl Activation { const fn name(self) -> &'static str { match self { Self::Linear => "linear", Self::LegacySignedLogOnePlus => "log1p", $(Self::$activation => stringify!($method)),+ } } } impl Model {$(pub fn $method(mut self) -> Self { let index = self.blocks.len() - 1; if self.blocks[index].normalization.is_some() { panic!("activation must precede normalization"); } self.blocks[index].activation = Activation::$activation; self })+}}; }
activations!(cos: Cosine = 1, exp: Exponential = 2, log: Logarithm = 3, ln: NaturalLogarithm = 4, huber: Huber = 6, tan: Tangent = 7, relu: Relu = 8, leak: LeakyRelu = 9, sigmoid: Sigmoid = 10, tanh: Tanh = 11, selu: Selu = 12, gelu: Gelu = 13, silu: Silu = 14, elu: Elu = 15, prelu: PRelu = 16); macro_rules! model_mutators {
	($($method:ident($argument:ident: $type:ty) => $pattern:pat => $update:expr;)+) => {impl Model {$(pub fn $method(mut self, $argument: $type) -> Self { if let Some($pattern) = self.blocks.last_mut() { $update; } self })+}}; }
model_mutators! {
	vocab(vocabulary: usize) => Block { operation: OperationBlock::Embed(_, value), .. } => *value = vocabulary; lgbm(depth: usize) => Block { operation: OperationBlock::Forest(_, kind, value), .. } => (*kind, *value) = (1, depth); xgbst(depth: usize) => Block { operation: OperationBlock::Forest(_, kind, value), .. } => (*kind, *value) = (2, depth); norm(normalization: Normalization) => block => block.normalization = Some(if normalization as usize == batch as usize { BlockNormalization::Batch } else { BlockNormalization::Layer }); }
impl BlockNormalization {
	const fn name(self) -> &'static str {
		match self {
			Self::Batch => "bnorm",
			Self::Layer => "lnorm", } }
}
struct TrainingGraph<'a> {
	blocks: &'a [Block],
	widths: Vec<usize>,
	shapes: Vec<(usize, usize)>,
	offsets: Vec<usize>,
	matrix_parameters: usize,
	parameters: usize,
}
struct DevicePass<T, O = T> {
	values: T,
	raw: T,
	derivatives: T,
	operations: T,
	scales: O,
	contexts: O,
}
struct DeviceGraph {
	pass: DevicePass<Vec<DeviceBuffer>, Vec<Option<DeviceBuffer>>>,
	pointers: DevicePass<DeviceBuffer>,
	descriptors: DeviceBuffer,
	parameters: DeviceBuffer,
}
struct EpochBuffers {
	metrics: DeviceBuffer,
	gradient: DeviceBuffer,
	delta: [DeviceBuffer; 2],
	checkpoint: DeviceBuffer,
}
#[derive(Clone, Copy)]
struct AdamwConfig {
	rate: f64,
	beta1: f64,
	beta2: f64,
	epsilon: f64,
	decay: f64,
}
macro_rules! hsa_kernels {
	($($kernel:ident)+) => {
		#[derive(Clone, Copy)] enum Vendor { Amd, Nvidia } struct Hsa { status: i32, vendor: Vendor, api: [usize; 8], $($kernel: usize),+ }
		impl Hsa {
			const fn new(status: i32, vendor: Vendor, api: [usize; 8]) -> Self { Self { status, vendor, api, $($kernel: 0),+ } }
			unsafe fn load_kernels(&mut self, module: *mut c_void) { let get = self.api[6]; let mut status = self.status; $(if status == 0 { let mut value = ptr::null_mut(); status = unsafe { address::<unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *const c_char) -> i32>(get)(&mut value, module, concat!(stringify!($kernel), "\0").as_ptr().cast()) }; self.$kernel = value as usize; })+ self.status = status; }
			unsafe fn allocate(&self, pointer: *mut *mut c_void, bytes: usize) -> i32 { unsafe { match self.vendor { Vendor::Amd => address::<unsafe extern "C" fn(*mut *mut c_void, usize) -> i32>(self.api[0])(pointer, bytes), Vendor::Nvidia => address::<unsafe extern "C" fn(*mut u64, usize) -> i32>(self.api[0])(pointer.cast(), bytes) } } } unsafe fn free(&self, pointer: *mut c_void) -> i32 { unsafe { match self.vendor { Vendor::Amd => address::<unsafe extern "C" fn(*mut c_void) -> i32>(self.api[1])(pointer), Vendor::Nvidia => address::<unsafe extern "C" fn(u64) -> i32>(self.api[1])(pointer as usize as u64) } } }
			unsafe fn upload(&self, destination: *mut c_void, source: *const c_void, bytes: usize) -> i32 { unsafe { match self.vendor { Vendor::Amd => address::<unsafe extern "C" fn(*mut c_void, *const c_void, usize, i32) -> i32>(self.api[2])(destination, source, bytes, 1), Vendor::Nvidia => address::<unsafe extern "C" fn(u64, *const c_void, usize) -> i32>(self.api[2])(destination as usize as u64, source, bytes) } } } unsafe fn download(&self, destination: *mut c_void, source: *const c_void, bytes: usize) -> i32 { unsafe { match self.vendor { Vendor::Amd => address::<unsafe extern "C" fn(*mut c_void, *const c_void, usize, i32) -> i32>(self.api[3])(destination, source, bytes, 2), Vendor::Nvidia => address::<unsafe extern "C" fn(*mut c_void, u64, usize) -> i32>(self.api[3])(destination, source as usize as u64, bytes) } } }
			unsafe fn synchronize(&self) -> i32 { unsafe { address::<unsafe extern "C" fn() -> i32>(self.api[4])() } } unsafe fn module_load(&self, module: *mut *mut c_void, path: *const c_char) -> i32 { unsafe { address::<unsafe extern "C" fn(*mut *mut c_void, *const c_char) -> i32>(self.api[5])(module, path) } } unsafe fn launch(&self, function: *mut c_void, grid: u32, block: u32, arguments: *mut *mut c_void) -> i32 { unsafe { address::<unsafe extern "C" fn(*mut c_void, u32, u32, u32, u32, u32, u32, u32, *mut c_void, *mut *mut c_void, *mut *mut c_void) -> i32>(self.api[7])(function, grid, 1, 1, block, 1, 1, 0, ptr::null_mut(), arguments, ptr::null_mut()) } } } }; }
hsa_kernels! { forward_graph epoch_graph normalize affine initialize set_value metrics }
static HSA: OnceLock<Hsa> = OnceLock::new(); macro_rules! device_buffer {
	() => {
		struct DeviceBuffer {
			pointer: *mut c_void,
			elements: usize, }
		impl DeviceBuffer {
			fn status(status: i32, action: &str) -> Result<(), RecipeError> {
				if status == 0 { Ok(()) } else { Err(RecipeError::new(format!("GPU {action} failed with status {status}"))) } }
			fn allocate(elements: usize, bytes: usize) -> Result<Self, RecipeError> {
				let runtime = hsa()?; let mut pointer = ptr::null_mut(); Self::status(unsafe { runtime.allocate(&mut pointer, bytes) }, "allocation")?; Ok(Self { pointer, elements }) }
			fn new(elements: usize) -> Result<Self, RecipeError> {
				Self::allocate(elements, elements * size_of::<f64>()) }
			fn upload<T>(values: &[T]) -> Result<Self, RecipeError> {
				let buffer = Self::allocate(values.len(), size_of_val(values))?; Self::status(unsafe { hsa()?.upload(buffer.pointer, values.as_ptr().cast(), size_of_val(values)) }, "transfer")?; Ok(buffer) }
			fn download(&self) -> Result<Vec<f64>, RecipeError> {
				let runtime = hsa()?; Self::status(unsafe { runtime.synchronize() }, "synchronization")?; let mut values = vec![0.0; self.elements]; Self::status(unsafe { runtime.download(values.as_mut_ptr().cast(), self.pointer, self.elements * size_of::<f64>()) }, "transfer")?; Ok(values) } } }; }
device_buffer!(); struct DeviceData {
	samples: DeviceBuffer,
	targets: DeviceBuffer,
	target_affine: [f64; 2],
}
impl Drop for DeviceBuffer {
	fn drop(&mut self) {
		if let Some(runtime) = HSA.get() {
			unsafe {
				runtime.free(self.pointer); } } }
}
#[link(name = "dl")]
unsafe extern "C" {
	fn dlopen(name: *const c_char, flags: i32) -> *mut c_void; fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void; fn dlclose(handle: *mut c_void) -> i32; }
unsafe fn dynamic<F>(handle: *mut c_void, name: &[u8]) -> F {
	unsafe { std::mem::transmute_copy(&dlsym(handle, name.as_ptr().cast())) }
}
unsafe fn address<F>(value: usize) -> F {
	unsafe { std::mem::transmute_copy(&value) }
}
macro_rules! hsa_loader {
	($($vendor:ident => $module:expr; [$($name:literal),+])+) => {
		fn hsa() -> Result<&'static Hsa, RecipeError> {
			let runtime = HSA.get_or_init(|| unsafe {
				let mut handle = dlopen(b"libamdhip64.so\0".as_ptr().cast(), 2); let mut status = -1; if !handle.is_null() { status = dynamic::<unsafe extern "C" fn(u32) -> i32>(handle, b"hipInit\0")(0); if status == 0 { status = dynamic::<unsafe extern "C" fn(i32) -> i32>(handle, b"hipSetDevice\0")(0); } }
				let vendor = if status == 0 { Vendor::Amd } else { if !handle.is_null() { dlclose(handle); } handle = dlopen(b"libcuda.so.1\0".as_ptr().cast(), 2); if !handle.is_null() { status = dynamic::<unsafe extern "C" fn(u32) -> i32>(handle, b"cuInit\0")(0); let (mut device, mut context) = (0, ptr::null_mut()); if status == 0 { status = dynamic::<unsafe extern "C" fn(*mut i32, i32) -> i32>(handle, b"cuDeviceGet\0")(&mut device, 0); } if status == 0 { status = dynamic::<unsafe extern "C" fn(*mut *mut c_void, u32, i32) -> i32>(handle, b"cuCtxCreate_v2\0")(&mut context, 0, device); } } Vendor::Nvidia }; let names: [&[u8]; 8] = match vendor { $(Vendor::$vendor => [$($name),+],)+ }; let api = names.map(|name| dlsym(handle, name.as_ptr().cast()) as usize); let mut runtime = Hsa::new(status, vendor, api); let mut module = ptr::null_mut(); if status == 0 { runtime.status = runtime.module_load(&mut module, match vendor { $(Vendor::$vendor => $module,)+ }.as_ptr().cast()); } runtime.load_kernels(module); runtime }); if runtime.status == 0 { Ok(runtime) } else { Err(RecipeError::new(format!("GPU initialization failed with status {}", runtime.status))) } } }; }
hsa_loader! {
	Amd => concat!(env!("RECIPE_HSA_CODE_OBJECT"), "\0"); [b"hipMalloc\0", b"hipFree\0", b"hipMemcpy\0", b"hipMemcpy\0", b"hipDeviceSynchronize\0", b"hipModuleLoad\0", b"hipModuleGetFunction\0", b"hipModuleLaunchKernel\0"]
	Nvidia => concat!(env!("RECIPE_NV_MODULE"), "\0"); [b"cuMemAlloc_v2\0", b"cuMemFree_v2\0", b"cuMemcpyHtoD_v2\0", b"cuMemcpyDtoH_v2\0", b"cuCtxSynchronize\0", b"cuModuleLoad\0", b"cuModuleGetFunction\0", b"cuLaunchKernel\0"]
}
fn hsa_launch(function: usize, count: usize, threads: u32, arguments: &mut [*mut c_void]) -> Result<(), RecipeError> {
	let runtime = hsa()?; let grid = (count as u32).div_ceil(threads); let status = unsafe { runtime.launch(function as *mut c_void, grid, threads, arguments.as_mut_ptr()) }; DeviceBuffer::status(status, "dispatch")
}
macro_rules! gpu_launch {
	($function:expr, $count:expr, $threads:expr; $($name:ident = $value:expr),+ $(,)?) => {{
		let launch_count = $count; let launch_threads = $threads; $(let mut $name = $value;)+
		let mut arguments = [$(&mut $name as *mut _ as *mut c_void),+]; hsa_launch($function, launch_count, launch_threads, &mut arguments) }}; }
fn gpu_normalize(values: &DeviceBuffer, reference: &DeviceBuffer, scales: &DeviceBuffer, rows: usize, width: usize, normalization: BlockNormalization, reverse: bool, threads: u32) -> Result<(), RecipeError> {
	let runtime = hsa()?; let mode = if matches!(normalization, BlockNormalization::Batch) { 1 } else { 2 }; let groups = if mode == 1 { width } else { rows }; gpu_launch!(runtime.normalize, groups, threads; values = values.pointer,
		reference = reference.pointer,
		scales = scales.pointer,
		rows = rows as u32,
		width = width as u32,
		mode = mode,
		reverse = i32::from(reverse),
		epsilon = configured_f64(
			"RECIPE_NORMALIZATION_EPSILON",
			env!("RECIPE_NORMALIZATION_EPSILON"), )?,
		threads = threads, )
}
fn gpu_metrics(predictions: &DeviceBuffer, targets: &DeviceBuffer, rows: usize, loss: LossFunction, threads: u32) -> Result<[f64; 3], RecipeError> {
	let runtime = hsa()?; let metrics = DeviceBuffer::new(3)?; gpu_launch!(runtime.metrics, 1, threads; predictions = predictions.pointer,
		targets = targets.pointer,
		output = metrics.pointer,
		rows = rows as u32,
		loss = loss.0 as i32, )?; let values = metrics.download()?; Ok([values[0], values[1], values[2]])
}
fn gpu_affine(values: &DeviceBuffer, scale: f64, offset: f64, threads: u32) -> Result<(), RecipeError> {
	let runtime = hsa()?; gpu_launch!(runtime.affine, values.elements, threads; count = values.elements as u32,
		values = values.pointer,
		scale = scale,
		offset = offset,
		threads = threads, )
}
fn gpu_initialize(count: usize, seed: u64, scale: f64, prelu: &[(usize, f64)], threads: u32) -> Result<DeviceBuffer, RecipeError> {
	let runtime = hsa()?; let values = DeviceBuffer::new(count)?; gpu_launch!(runtime.initialize, count, threads; buffer = values.pointer,
		count_arg = count as u32,
		seed = seed,
		scale = scale,
		threads = threads, )?; for (index, value) in prelu {
		gpu_launch!(runtime.set_value, 1, threads; buffer = values.pointer,
			index = *index as u32,
			value = *value, )?; }
	Ok(values)
}
impl TrainingGraph<'_> {
	fn prelu(&self, stage: usize) -> Option<usize> {
		(self.blocks[stage].activation == Activation::PRelu).then(|| self.matrix_parameters + self.blocks[..stage].iter().filter(|block| block.activation == Activation::PRelu).count()) }
	fn operation(&self, stage: usize) -> (i32, f64, f64) {
		match &self.blocks[stage].operation {
			OperationBlock::Layer(_) => (0, 1.0, 0.0),
			OperationBlock::Conv(_, kernel) => (1, *kernel as f64, 0.0),
			OperationBlock::Pool(size) => (2, *size as f64, 0.0),
			OperationBlock::KMeans(_) => (3, 1.0, 0.0),
			OperationBlock::Embed(_, vocabulary) => (4, (*vocabulary).max(1) as f64, 0.0),
			OperationBlock::Attention(heads) => (5, *heads as f64, 0.0),
			OperationBlock::Rnn(_) => (6, 1.0, 0.0),
			OperationBlock::Gru(_) => (7, 1.0, 0.0),
			OperationBlock::Lstm(_) => (8, 1.0, 0.0),
			OperationBlock::Forest(trees, _, depth) => (9, *trees as f64, (*depth).max(1) as f64),
			OperationBlock::Knn(neighbors) => (10, *neighbors as f64, 0.0),
			OperationBlock::Residual(_) => (11, 1.0, 0.0),
			OperationBlock::Perceptron(_) => (12, 1.0, 0.0), } }
	fn prepare_gpu(&self, rows: usize) -> Result<DeviceGraph, RecipeError> {
		let mut pass = DevicePass { values: Vec::with_capacity(self.blocks.len()), raw: Vec::with_capacity(self.blocks.len()), derivatives: Vec::with_capacity(self.blocks.len()), operations: Vec::with_capacity(self.blocks.len()), scales: Vec::with_capacity(self.blocks.len()), contexts: Vec::with_capacity(self.blocks.len()) }; let forest = |trees: usize, depth: usize| (depth < i32::BITS as usize).then_some(depth).and_then(|depth| 1_usize.checked_shl(depth as u32)).and_then(|leaves| leaves.checked_mul(3).and_then(|nodes| nodes.checked_sub(2)).and_then(|span| trees.checked_mul(span)).and_then(|nodes| trees.checked_mul(rows).and_then(|assignments| nodes.checked_add(assignments))).and_then(|context| context.checked_add(rows))).ok_or_else(|| RecipeError::new("forest dimensions overflow")); let (mut descriptors, mut parameters) = (Vec::with_capacity(self.blocks.len() * 7), Vec::with_capacity(self.blocks.len() * 2)); for stage in 0..self.blocks.len() {
			let (from, to) = (self.widths[stage], self.widths[stage + 1]); let count = rows * to; let normalization = match self.blocks[stage].normalization {
				None => 0,
				Some(BlockNormalization::Batch) => 1,
				Some(BlockNormalization::Layer) => 2, }; let (operation, parameter, mut secondary) = self.operation(stage); if operation == 1 || operation == 2 {
				secondary = self.shapes[stage].0 as f64; }
			let gates = match operation {
				6 => 1,
				7 => 3,
				8 => 4,
				_ => 0, }; descriptors.extend([from as i32, to as i32, self.offsets[stage] as i32, operation, self.blocks[stage].activation as i32, normalization, self.prelu(stage).map_or(-1, |index| index as i32)]); parameters.extend([parameter, secondary]); pass.values.push(DeviceBuffer::new(count)?); pass.raw.push(DeviceBuffer::new(count)?); pass.derivatives.push(DeviceBuffer::new(count)?); pass.operations.push(DeviceBuffer::new(count)?); pass.scales.push((normalization != 0).then(|| DeviceBuffer::new(2 * if normalization == 1 { to } else { rows })).transpose()?); pass.contexts.push(match operation {
				3 => Some(DeviceBuffer::upload(&vec![0.0; 1 + from * to + count])?),
				9 => Some(DeviceBuffer::new(forest(parameter as usize, secondary as usize)?)?),
				10 => Some(DeviceBuffer::new(rows * (1 + parameter as usize))?),
				6..=8 => Some(DeviceBuffer::new((2 * gates + 4) * count)?),
				_ => None, }); }
		let addresses = |buffers: &[DeviceBuffer]| buffers.iter().map(|buffer| buffer.pointer as usize as u64).collect::<Vec<_>>(); let optional = |buffers: &[Option<DeviceBuffer>]| buffers.iter().map(|buffer| buffer.as_ref().map_or(0, |buffer| buffer.pointer as usize as u64)).collect::<Vec<_>>(); let pointers = DevicePass { values: DeviceBuffer::upload(&addresses(&pass.values))?, raw: DeviceBuffer::upload(&addresses(&pass.raw))?, derivatives: DeviceBuffer::upload(&addresses(&pass.derivatives))?, operations: DeviceBuffer::upload(&addresses(&pass.operations))?, scales: DeviceBuffer::upload(&optional(&pass.scales))?, contexts: DeviceBuffer::upload(&optional(&pass.contexts))? }; Ok(DeviceGraph { pass, pointers, descriptors: DeviceBuffer::upload(&descriptors)?, parameters: DeviceBuffer::upload(&parameters)? }) }
	fn forward_gpu(&self, graph: &DeviceGraph, input: &DeviceBuffer, rows: usize, weights: &DeviceBuffer, config: &DeviceBuffer, threads: u32) -> Result<(), RecipeError> {
		let runtime = hsa()?; gpu_launch!(runtime.forward_graph, threads as usize, threads; input = input.pointer,
			weights = weights.pointer,
			config = config.pointer,
			values = graph.pointers.values.pointer,
			raw = graph.pointers.raw.pointer,
			operations = graph.pointers.operations.pointer,
			derivatives = graph.pointers.derivatives.pointer,
			scales = graph.pointers.scales.pointer,
			contexts = graph.pointers.contexts.pointer,
			descriptors = graph.descriptors.pointer,
			parameters = graph.parameters.pointer,
			rows = rows as u32,
			stages = self.blocks.len() as u32,
			epsilon = configured_f64(
				"RECIPE_NORMALIZATION_EPSILON",
				env!("RECIPE_NORMALIZATION_EPSILON"), )?,
			threads = threads, ) }
	fn epoch_gpu(&self, graph: &DeviceGraph, epoch: &EpochBuffers, input: &DeviceBuffer, targets: &DeviceBuffer, weights: &DeviceBuffer, config: &DeviceBuffer, moments: &DeviceBuffer, variances: &DeviceBuffer, rows: usize, threads: u32, loss: LossFunction, optimizer: AdamwConfig, step: i32, previous_loss: f64, tolerance: f64, checkpoint_enabled: bool) -> Result<[f64; 4], RecipeError> {
		let runtime = hsa()?; gpu_launch!(runtime.epoch_graph, threads as usize, threads; input = input.pointer,
			targets = targets.pointer,
			weights = weights.pointer,
			config = config.pointer,
			values = graph.pointers.values.pointer,
			raw = graph.pointers.raw.pointer,
			operations = graph.pointers.operations.pointer,
			derivatives = graph.pointers.derivatives.pointer,
			scales = graph.pointers.scales.pointer,
			contexts = graph.pointers.contexts.pointer,
			descriptors = graph.descriptors.pointer,
			parameters = graph.parameters.pointer,
			metrics = epoch.metrics.pointer,
			gradient = epoch.gradient.pointer,
			delta_a = epoch.delta[0].pointer,
			delta_b = epoch.delta[1].pointer,
			moments = moments.pointer,
			variances = variances.pointer,
			checkpoint = epoch.checkpoint.pointer,
			rows = rows as u32,
			stages = self.blocks.len() as u32,
			parameter_count = self.parameters as u32,
			loss = loss.0 as i32,
			previous_loss = previous_loss,
			tolerance = tolerance,
			checkpoint_enabled = i32::from(checkpoint_enabled),
			normalization_epsilon = configured_f64(
				"RECIPE_NORMALIZATION_EPSILON",
				env!("RECIPE_NORMALIZATION_EPSILON"), )?,
			rate = optimizer.rate,
			beta1 = optimizer.beta1,
			beta2 = optimizer.beta2,
			optimizer_epsilon = optimizer.epsilon,
			decay = optimizer.decay,
			step = step,
			threads = threads, )?; let values = epoch.metrics.download()?; Ok([values[0], values[1], values[2], values[3]]) }
}
pub struct Train {
	epochs: usize,
	learning_rate: f64,
	log_metrics: Vec<Metric>,
	stop_tolerance: Option<f64>,
	resume: Option<String>,
	save: Option<String>,
}
pub struct Adamw; #[derive(Clone, Copy)]
pub struct LossFunction(u8); #[derive(Clone, Copy)]
pub struct Metric(u8); pub struct ZScore; pub type Normalization = fn(usize) -> Residual; pub const adamw: Adamw = Adamw; pub const mse: LossFunction = LossFunction(0); pub const rmse: LossFunction = LossFunction(1); pub const huber: LossFunction = LossFunction(2); pub const mae: LossFunction = LossFunction(3); pub const bce: LossFunction = LossFunction(4); pub const ce: LossFunction = LossFunction(5); pub const focal: LossFunction = LossFunction(6); pub const Run: Metric = Metric(0); pub const Loss: Metric = Metric(1); pub const R2: Metric = Metric(2); pub const Time: Metric = Metric(3); pub const Epoch: Metric = Metric(4); pub const blck: Metric = Metric(5); pub const atvn: Metric = Metric(6); pub const norm: Metric = Metric(7); pub const z_score: ZScore = ZScore; pub const batch: Normalization = |_| Residual::Relu; pub struct TrainingReport(f64, f64, Vec<f64>); impl TrainingReport {
	pub const fn initial_loss(&self) -> f64 {
		self.0 }
	pub const fn final_loss(&self) -> f64 {
		self.1 }
	pub fn predictions(&self) -> &[f64] {
		&self.2 }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
	Transfer,
	Calculation,
}
impl fmt::Display for Bound {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(match self {
			Self::Transfer => "transfer",
			Self::Calculation => "calculation", }) }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(String); impl RecipeError {
	fn new(message: impl Into<String>) -> Self {
		Self(message.into()) }
}
impl fmt::Display for RecipeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.0) }
}
impl Error for RecipeError {}
#[derive(Clone)]
struct Tensor {
	name: String,
	dtype: DType,
	shape: Vec<u64>,
	data: Option<Vec<f64>>,
	producer: Option<usize>,
}
#[derive(Clone)]
enum Operation {
	Conv1d(Value, Value, Option<Value>, Value, u64),
	Matmul(Value, Value, Value),
	Add(Value, Value, Value),
	Relu(Value, Value),
}
impl Operation {
	fn inputs(&self) -> [Option<Value>; 3] {
		match self {
			Self::Conv1d(input, weights, bias, ..) => [Some(*input), Some(*weights), *bias],
			Self::Matmul(left, right, ..) => [Some(*left), Some(*right), None],
			Self::Add(left, right, ..) => [Some(*left), Some(*right), None],
			Self::Relu(input, ..) => [Some(*input), None, None], } }
	const fn output(&self) -> Value {
		match self {
			Self::Conv1d(_, _, _, output, _) | Self::Matmul(_, _, output) | Self::Add(_, _, output) | Self::Relu(_, output) => *output, } }
}
#[derive(Default)]
pub struct Recipe {
	tensors: Vec<Tensor>,
	operations: Vec<Operation>,
}
impl Recipe {
	#[must_use]
	pub const fn new() -> Self {
		Self { tensors: Vec::new(), operations: Vec::new() } }
	pub fn data(&self, sources: impl IntoDataSources) -> Data {
		Data { sources: sources.into_data_sources(), target: String::new(), exclusions: Vec::new(), normalize: false, split: 1.0, prepared: OnceLock::new(), device: OnceLock::new() } }
	pub fn model(&self) -> Model {
		Model { blocks: Vec::new(), loss: mse } }
	pub const fn train(&self) -> Train {
		Train { epochs: 1, learning_rate: 0.001, log_metrics: Vec::new(), stop_tolerance: None, resume: None, save: None } }
	pub fn tensor(&mut self, name: impl Into<String>, dtype: DType, shape: impl IntoIterator<Item = u64>) -> Result<Value, RecipeError> {
		let name = name.into(); if name.is_empty() {
			return Err(RecipeError::new("tensor name must not be empty")); }
		if self.tensors.iter().any(|tensor| tensor.name == name) {
			return Err(RecipeError::new(format!("tensor {name:?} already exists"))); }
		let shape = shape.into_iter().collect::<Vec<_>>(); elements(&shape)?; let value = Value(self.tensors.len()); self.tensors.push(Tensor { name, dtype, shape, data: None, producer: None }); Ok(value) }
	pub fn write(&mut self, value: Value, data: impl IntoIterator<Item = f64>) -> Result<(), RecipeError> {
		let tensor = self.tensor_mut(value)?; if !tensor.dtype.is_float() {
			return Err(RecipeError::new("host execution currently accepts floating-point tensors")); }
		let data = data.into_iter().map(|value| cast(tensor.dtype, value)).collect::<Vec<_>>(); let expected = usize::try_from(elements(&tensor.shape)?).map_err(|_| RecipeError::new("tensor element count does not fit host memory"))?; if data.len() != expected {
			return Err(RecipeError::new(format!("tensor {:?} requires {expected} values, received {}", tensor.name, data.len()))); }
		tensor.data = Some(data); Ok(()) }
	pub fn conv1d(&mut self, input: Value, weights: Value, bias: Option<Value>, stride: u64) -> Result<Value, RecipeError> {
		if stride == 0 {
			return Err(RecipeError::new("conv1d stride must be positive")); }
		let input_tensor = self.tensor_ref(input)?.clone(); let weights_tensor = self.tensor_ref(weights)?.clone(); if input_tensor.dtype != weights_tensor.dtype || !input_tensor.dtype.is_float() {
			return Err(RecipeError::new("conv1d input and weights must use the same floating-point dtype")); }
		let [n, cin, lin] = rank::<3>(&input_tensor.shape, "conv1d input")?; let [cout, weight_cin, kernel] = rank::<3>(&weights_tensor.shape, "conv1d weights")?; if cin != weight_cin {
			return Err(RecipeError::new(format!("conv1d input has {cin} channels but weights require {weight_cin}"))); }
		if kernel > lin {
			return Err(RecipeError::new("conv1d kernel exceeds input length")); }
		if let Some(bias_value) = bias {
			let bias_tensor = self.tensor_ref(bias_value)?; if bias_tensor.dtype != input_tensor.dtype || bias_tensor.shape != [cout] {
				return Err(RecipeError::new(format!("conv1d bias must have shape [{cout}] and match the input dtype"))); } }
		let lout = (lin - kernel) / stride + 1; let output = self.push_output("output", input_tensor.dtype, vec![n, cout, lout])?; self.push_operation(Operation::Conv1d(input, weights, bias, output, stride)); Ok(output) }
	pub fn matmul(&mut self, left: Value, right: Value) -> Result<Value, RecipeError> {
		let left_tensor = self.tensor_ref(left)?.clone(); let right_tensor = self.tensor_ref(right)?.clone(); if left_tensor.dtype != right_tensor.dtype || !left_tensor.dtype.is_float() {
			return Err(RecipeError::new("matmul operands must use the same floating-point dtype")); }
		let [m, k] = rank::<2>(&left_tensor.shape, "matmul left operand")?; let [right_k, n] = rank::<2>(&right_tensor.shape, "matmul right operand")?; if k != right_k {
			return Err(RecipeError::new(format!("matmul inner dimensions differ: {k} and {right_k}"))); }
		let output = self.push_output("matmul_output", left_tensor.dtype, vec![m, n])?; self.push_operation(Operation::Matmul(left, right, output)); Ok(output) }
	pub fn add(&mut self, left: Value, right: Value) -> Result<Value, RecipeError> {
		let left_tensor = self.tensor_ref(left)?.clone(); let right_tensor = self.tensor_ref(right)?.clone(); if left_tensor.dtype != right_tensor.dtype || left_tensor.shape != right_tensor.shape {
			return Err(RecipeError::new("add operands must have identical dtype and shape")); }
		let output = self.push_output("add_output", left_tensor.dtype, left_tensor.shape)?; self.push_operation(Operation::Add(left, right, output)); Ok(output) }
	pub fn relu(&mut self, input: Value) -> Result<Value, RecipeError> {
		let tensor = self.tensor_ref(input)?.clone(); if !tensor.dtype.is_float() {
			return Err(RecipeError::new("relu requires a floating-point tensor")); }
		let output = self.push_output("relu_output", tensor.dtype, tensor.shape)?; self.push_operation(Operation::Relu(input, output)); Ok(output) }
	pub fn lower(&self, output: Value) -> Result<Lowering, RecipeError> {
		self.tensor_ref(output)?; let operations = self.reachable_operations(output)?; let mut reads = Vec::new(); for operation in &operations {
			for input in operation.inputs().into_iter().flatten() {
				if self.tensor_ref(input)?.producer.is_none() && !reads.contains(&input) {
					reads.push(input); } } }
		let mut calculations = Vec::new(); for operation in &operations {
			calculations.extend(self.lower_operation(operation)?); }
		let transfer_bytes = reads.iter().chain([&output]).try_fold(0_u64, |sum, value| sum.checked_add(self.tensor_bytes(*value)?).ok_or_else(|| RecipeError::new("transfer byte count overflow")))?; let flops = calculations.iter().try_fold(0_u64, |sum, calculation| sum.checked_add(calculation.flops()).ok_or_else(|| RecipeError::new("FLOP count overflow")))?; let balance = BALANCE_BYTES_PER_FLOP.parse::<f64>().map_err(|_| RecipeError::new("configured balance point is invalid"))?; if !balance.is_finite() || balance < 0.0 {
			return Err(RecipeError::new("configured balance point must be finite and nonnegative")); }
		Ok(Lowering { tensors: self.tensors.clone(), operations, reads, output, calculations, transfer_bytes, flops, balance }) }
	pub fn run(&mut self, output: Value) -> Result<Vec<f64>, RecipeError> {
		let operations = self.reachable_operations(output)?; for operation in operations {
			let data = match operation.clone() {
				Operation::Conv1d(input, weights, bias, _, stride) => self.execute_conv1d(input, weights, bias, stride)?,
				Operation::Matmul(left, right, _) => self.execute_matmul(left, right)?,
				Operation::Add(left, right, _) => self.execute_add(left, right)?,
				Operation::Relu(input, _) => self.tensor_data(input)?.iter().map(|value| value.max(0.0)).collect(), }; let output = operation.output(); let dtype = self.tensor_ref(output)?.dtype; self.tensor_mut(output)?.data = Some(data.into_iter().map(|value| cast(dtype, value)).collect()); }
		Ok(self.tensor_data(output)?.to_vec()) }
	fn push_output(&mut self, stem: &str, dtype: DType, shape: Vec<u64>) -> Result<Value, RecipeError> {
		let name = unique_name(&self.tensors, stem); self.tensor(name, dtype, shape) }
	fn push_operation(&mut self, operation: Operation) {
		let index = self.operations.len(); self.tensors[operation.output().0].producer = Some(index); self.operations.push(operation); }
	fn tensor_ref(&self, value: Value) -> Result<&Tensor, RecipeError> {
		self.tensors.get(value.0).ok_or_else(|| RecipeError::new(format!("unknown tensor value {}", value.0))) }
	fn tensor_mut(&mut self, value: Value) -> Result<&mut Tensor, RecipeError> {
		self.tensors.get_mut(value.0).ok_or_else(|| RecipeError::new(format!("unknown tensor value {}", value.0))) }
	fn tensor_bytes(&self, value: Value) -> Result<u64, RecipeError> {
		let tensor = self.tensor_ref(value)?; elements(&tensor.shape)?.checked_mul(tensor.dtype.bytes()).ok_or_else(|| RecipeError::new("tensor byte count overflow")) }
	fn tensor_data(&self, value: Value) -> Result<&[f64], RecipeError> {
		let tensor = self.tensor_ref(value)?; tensor.data.as_deref().ok_or_else(|| RecipeError::new(format!("tensor {:?} has no data", tensor.name))) }
	fn reachable_operations(&self, output: Value) -> Result<Vec<Operation>, RecipeError> {
		let mut indices = Vec::new(); self.visit(output, &mut indices)?; Ok(indices.into_iter().map(|index| self.operations[index].clone()).collect()) }
	fn visit(&self, value: Value, indices: &mut Vec<usize>) -> Result<(), RecipeError> {
		let Some(index) = self.tensor_ref(value)?.producer else {
			return Ok(()); }; if indices.contains(&index) {
			return Ok(()); }
		for input in self.operations[index].inputs().into_iter().flatten() {
			self.visit(input, indices)?; }
		indices.push(index); Ok(()) }
	fn lower_operation(&self, operation: &Operation) -> Result<Vec<Calculation>, RecipeError> {
		match operation {
			Operation::Conv1d(input, weights, bias, _, stride) => {
				let [n, cin, lin] = rank::<3>(&self.tensor_ref(*input)?.shape, "conv1d input")?; let [cout, _, kernel] = rank::<3>(&self.tensor_ref(*weights)?.shape, "conv1d weights")?; let lout = (lin - kernel) / stride + 1; let conv_flops = product(&[n, cout, lout, cin, kernel, 2])?; let mut result = vec![Calculation::Conv1d(n, cin, lin, cout, kernel, *stride, conv_flops)]; if bias.is_some() {
					result.push(Calculation::BiasAdd(n, cout, lout, product(&[n, cout, lout])?)); }
				Ok(result) }
			Operation::Matmul(left, right, _) => {
				let [m, k] = rank::<2>(&self.tensor_ref(*left)?.shape, "matmul left operand")?; let [_, n] = rank::<2>(&self.tensor_ref(*right)?.shape, "matmul right operand")?; Ok(vec![Calculation::Matmul(m, k, n, product(&[m, k, n, 2])?)]) }
			Operation::Add(_, _, output) => Ok(vec![Calculation::Add(elements(&self.tensor_ref(*output)?.shape)?)]),
			Operation::Relu(_, output) => Ok(vec![Calculation::Relu(elements(&self.tensor_ref(*output)?.shape)?)]), } }
	fn execute_conv1d(&self, input: Value, weights: Value, bias: Option<Value>, stride: u64) -> Result<Vec<f64>, RecipeError> {
		let [n, cin, lin] = rank::<3>(&self.tensor_ref(input)?.shape, "conv1d input")?; let [cout, _, kernel] = rank::<3>(&self.tensor_ref(weights)?.shape, "conv1d weights")?; let lout = (lin - kernel) / stride + 1; let input_data = self.tensor_data(input)?; let weights_data = self.tensor_data(weights)?; let bias_data = bias.map(|value| self.tensor_data(value)).transpose()?; let mut output = vec![0.0; usize_extent(product(&[n, cout, lout])?)?]; for batch_index in 0..n {
			for out_channel in 0..cout {
				for position in 0..lout {
					let mut sum = bias_data.map_or(0.0, |values| values[out_channel as usize]); for in_channel in 0..cin {
						for offset in 0..kernel {
							let input_index = ((batch_index * cin + in_channel) * lin + position * stride + offset) as usize; let weight_index = ((out_channel * cin + in_channel) * kernel + offset) as usize; sum += input_data[input_index] * weights_data[weight_index]; } }
					output[((batch_index * cout + out_channel) * lout + position) as usize] = sum; } } }
		Ok(output) }
	fn execute_matmul(&self, left: Value, right: Value) -> Result<Vec<f64>, RecipeError> {
		let [m, k] = rank::<2>(&self.tensor_ref(left)?.shape, "matmul left operand")?; let [_, n] = rank::<2>(&self.tensor_ref(right)?.shape, "matmul right operand")?; let left_data = self.tensor_data(left)?; let right_data = self.tensor_data(right)?; let mut output = vec![0.0; usize_extent(product(&[m, n])?)?]; for row in 0..m {
			for column in 0..n {
				for inner in 0..k {
					output[(row * n + column) as usize] += left_data[(row * k + inner) as usize] * right_data[(inner * n + column) as usize]; } } }
		Ok(output) }
	fn execute_add(&self, left: Value, right: Value) -> Result<Vec<f64>, RecipeError> {
		Ok(self.tensor_data(left)?.iter().zip(self.tensor_data(right)?).map(|(left, right)| left + right).collect()) }
}
impl Train {
	pub const fn stop(mut self, value: f64) -> Self {
		self.stop_tolerance = Some(value); self }
	pub fn resume(mut self, value: impl Into<String>) -> Self {
		self.resume = Some(value.into()); self }
	pub fn save(mut self, value: impl Into<String>) -> Self {
		self.save = Some(value.into()); self }
	pub const fn optimizer(self, _: Adamw) -> Self {
		self }
	pub const fn epochs(mut self, value: usize) -> Self {
		self.epochs = value; self }
	pub const fn lr(mut self, value: f64) -> Self {
		self.learning_rate = value; self }
	pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self {
		self.log_metrics = metrics.into(); self }
	pub fn run(&self, model: &Model, data: &Data) -> TrainingReport {
		let run = RUN.fetch_add(1, Ordering::Relaxed) + 1; match self.try_run(model, data, run) {
			Ok(report) => report,
			Err(error) => panic!("{error}"), } }
	fn try_run(&self, model: &Model, data: &Data, run: u64) -> Result<TrainingReport, RecipeError> {
		if self.resume.is_some() && self.save.is_none() {
			return Err(RecipeError::new("resume requires save")); }
		let prepared = load_training_data(data)?; let (samples, targets, rows, features) = (&prepared.samples, &prepared.targets, prepared.rows, prepared.features); let training_rows = ((rows as f64) * data.split) as usize; if training_rows == 0 || training_rows > rows {
			return Err(RecipeError::new("training split must select at least one row and no more than the dataset")); }
		let graph = model.graph(features)?; let initial_weight = configured_f64("RECIPE_TRAIN_INITIAL_WEIGHT", env!("RECIPE_TRAIN_INITIAL_WEIGHT"))?; let beta1 = configured_f64("RECIPE_TRAIN_ADAMW_BETA1", env!("RECIPE_TRAIN_ADAMW_BETA1"))?; let beta2 = configured_f64("RECIPE_TRAIN_ADAMW_BETA2", env!("RECIPE_TRAIN_ADAMW_BETA2"))?; let epsilon = configured_f64("RECIPE_TRAIN_ADAMW_EPSILON", env!("RECIPE_TRAIN_ADAMW_EPSILON"))?; let decay = configured_f64("RECIPE_TRAIN_ADAMW_WEIGHT_DECAY", env!("RECIPE_TRAIN_ADAMW_WEIGHT_DECAY"))?; let threads = env!("RECIPE_GPU_THREADS").parse::<u32>().map_err(|error| RecipeError::new(format!("invalid RECIPE_GPU_THREADS: {error}")))?; if threads == 0 {
			return Err(RecipeError::new("GPU thread count must be positive")); }
		let device = match data.device.get_or_init(|| {
			let samples = DeviceBuffer::upload(samples)?; let targets = DeviceBuffer::upload(targets)?; let target_affine = if data.normalize {
				let sample_stats = DeviceBuffer::new(2 * features)?; gpu_normalize(&samples, &samples, &sample_stats, rows, features, BlockNormalization::Batch, false, threads)?; let target_stats = DeviceBuffer::new(2)?; gpu_normalize(&targets, &targets, &target_stats, rows, 1, BlockNormalization::Batch, false, threads)?; let values = target_stats.download()?; [values[0], values[1]] } else {
				[0.0, 1.0] }; Ok(DeviceData { samples, targets, target_affine }) }) {
			Ok(device) => device,
			Err(error) => return Err(error.clone()), }; let (samples, targets, [target_offset, target_scale]) = (&device.samples, &device.targets, device.target_affine); let activation_config = activation_config()?; let seed = env!("RECIPE_RANDOM_SEED").parse::<u64>().map_err(|error| RecipeError::new(format!("invalid RECIPE_RANDOM_SEED: {error}")))?; let prelu = (0..graph.blocks.len()).filter_map(|stage| graph.prelu(stage).map(|index| (index, activation_config.prelu))).collect::<Vec<_>>(); let initialized = gpu_initialize(graph.parameters, seed, initial_weight, &prelu, threads)?; let weights = match &self.resume {
			Some(path) => DeviceBuffer::upload(&load_weights(path, &initialized.download()?, model, features, &prepared.schema)?)?,
			None => initialized, }; let (moments, variances, mut initial_loss) = (gpu_initialize(graph.parameters, 0, 0.0, &[], threads)?, gpu_initialize(graph.parameters, 0, 0.0, &[], threads)?, 0.0); let ho_loss_weight = configured_f64("RECIPE_HO_LOSS_WEIGHT", env!("RECIPE_HO_LOSS_WEIGHT"))?; let config = DeviceBuffer::upload(&[activation_config.leak, activation_config.prelu, activation_config.elu, activation_config.selu_alpha, activation_config.selu_scale, activation_config.gelu_scale, activation_config.gelu_cubic, ho_loss_weight, f64::from(prepared.target_categorical)])?; let device_graph = graph.prepare_gpu(training_rows)?; let delta_elements = training_rows * graph.widths.iter().copied().max().ok_or_else(|| RecipeError::new("model has no graph width"))?; let epoch_buffers = EpochBuffers { metrics: DeviceBuffer::new(4)?, gradient: DeviceBuffer::new(graph.parameters)?, delta: [DeviceBuffer::new(delta_elements)?, DeviceBuffer::new(delta_elements)?], checkpoint: DeviceBuffer::new(graph.parameters)? }; let optimizer = AdamwConfig { rate: self.learning_rate, beta1, beta2, epsilon, decay }; let tolerance = self.stop_tolerance.map_or(0.0, |value| value); if !tolerance.is_finite() || !(0.0..=1.0).contains(&tolerance) {
			return Err(RecipeError::new("stop tolerance must be between 0 and 1")); }
		let (mut previous_loss, mut checkpoint_saved, epoch_width) = (f64::NAN, false, self.epochs.to_string().len()); for epoch in 1..=self.epochs {
			let started = Instant::now(); let [loss, _, r2, checkpoint] = graph.epoch_gpu(&device_graph, &epoch_buffers, samples, targets, &weights, &config, &moments, &variances, training_rows, threads, model.loss, optimizer, epoch as i32, previous_loss, tolerance, !checkpoint_saved && tolerance > 0.0)?; let milliseconds = started.elapsed().as_secs_f64() * 1000.0; let checkpointed = checkpoint != 0.0; if checkpointed {
				if let Some(path) = &self.save {
					save_weights(path, model, features, &prepared.schema, &epoch_buffers.checkpoint.download()?)?; }
				checkpoint_saved = true; }
			if !self.log_metrics.is_empty() {
				let topology = model.description(self.log_metrics.iter().any(|metric| metric.0 == 5), self.log_metrics.iter().any(|metric| metric.0 == 6), self.log_metrics.iter().any(|metric| metric.0 == 7)); let (mut output, mut items, mut topology_printed) = (String::new(), 0, false); for metric in &self.log_metrics {
					let (name, value) = match metric.0 {
						0 => ("run", format!("{run:>5}")),
						1 => (model.loss.name(), format!("{loss:.4}")),
						2 => ("r2", format!("{r2:>7.4}")),
						3 => ("time", format!("{milliseconds:>9.3} ms")),
						4 => ("epoch", format!("{epoch:>epoch_width$}")),
						5..=7 if !topology_printed && !topology.is_empty() => {
							topology_printed = true; ("", topology.clone()) }
						5..=7 => continue,
						_ => unreachable!(), }; if items != 0 {
						output.push_str("  "); }
					let color = ["242;40;60", "0;174;107", "39;125;255", "255;194;0", "135;90;251", "255;122;0", "215;46;130"][items % 7]; if name.is_empty() {
						output.push_str(&value); } else {
						output.push_str(name); output.push(' '); output.push_str(&format!("\x1b[38;2;{color}m{value}\x1b[0m")); }
					items += 1; }
				if checkpointed {
					output.push_str("  \x1b[1;32m← checkpoint\x1b[0m"); }
				eprintln!("{output}"); }
			initial_loss = if epoch == 1 { loss } else { initial_loss }; previous_loss = loss; if !loss.is_finite() {
				break; } }
		graph.forward_gpu(&device_graph, samples, training_rows, &weights, &config, threads)?; let normalized = &device_graph.pass.values[graph.blocks.len() - 1]; let [final_loss, _, _] = gpu_metrics(normalized, targets, training_rows, model.loss, threads)?; if !checkpoint_saved && let Some(path) = &self.save {
			save_weights(path, model, features, &prepared.schema, &weights.download()?)?; }
		gpu_affine(normalized, target_scale, target_offset, threads)?; let predictions = normalized.download()?; Ok(TrainingReport(initial_loss, final_loss, predictions)) }
}
impl LossFunction {
	const fn name(self) -> &'static str {
		match self.0 {
			0 => "mse",
			1 => "rmse",
			2 => "huber",
			3 => "mae",
			4 => "bce",
			5 => "ce",
			6 => "focal",
			_ => unreachable!(), } }
}
fn load_weights(path: &str, initial: &[f64], model: &Model, features: usize, schema: &str) -> Result<Vec<f64>, RecipeError> {
	path.ends_with(".ogdl").then_some(()).ok_or_else(|| RecipeError::new("resume requires a semantic .ogdl model"))?; if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
		save_weights(path, model, features, schema, initial)?; }
	let text = fs::read_to_string(path).map_err(|error| RecipeError::new(format!("cannot resume {path}: {error}")))?; let (mut stored_model, mut stored_schema, mut stored_weights) = (None, None, None); for line in text.lines().map(str::trim) {
		if let Some(value) = line.strip_prefix("model ") {
			stored_model = Some(value); }
		if let Some(value) = line.strip_prefix("schema ") {
			stored_schema = Some(value); }
		if let Some(value) = line.strip_prefix("weights ") {
			stored_weights = Some(value); } }
	let mut weights = stored_weights.ok_or_else(|| RecipeError::new("model has no weights"))?.split_whitespace().map(|value| value.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid model weight: {error}")))).collect::<Result<Vec<_>, _>>()?; let signature = model.signature(); if weights.len() != initial.len() || stored_model != Some(signature.as_str()) || stored_schema != Some(schema) {
		eprint!("mismatch: overwrite {path}? Y/n "); std::io::Write::flush(&mut std::io::stderr()).map_err(|error| RecipeError::new(format!("cannot prompt: {error}")))?; let mut answer = String::new(); std::io::stdin().read_line(&mut answer).map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?; if !answer.trim().is_empty() && !answer.trim().eq_ignore_ascii_case("y") {
			return Err(RecipeError::new("model mismatch not overwritten")); }
		weights = initial.to_vec(); save_weights(path, model, features, schema, &weights)?; }
	eprintln!("resumed: {path}"); Ok(weights)
}
fn save_weights(path: &str, model: &Model, features: usize, schema: &str, weights: &[f64]) -> Result<(), RecipeError> {
	path.ends_with(".ogdl").then_some(()).ok_or_else(|| RecipeError::new("save requires a semantic .ogdl model"))?; let weights = weights.iter().map(f64::to_string).collect::<Vec<_>>().join(" "); fs::write(path, format!("recipe-model\n    features {features}\n    schema {schema}\n    model {}\n    weights {weights}\n", model.signature())).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}"))).map(|()| eprintln!("saved: {path}"))
}
struct Prepared {
	samples: Vec<f64>,
	targets: Vec<f64>,
	rows: usize,
	features: usize,
	schema: String,
	target_categorical: bool,
}
struct Table {
	name: String,
	headers: Vec<String>,
	rows: Vec<Vec<String>>,
}
enum FeatureType {
	Numeric(&'static str),
	Categoric(Vec<String>),
	Text(usize),
}
fn load_training_data(data: &Data) -> Result<&Prepared, RecipeError> {
	match data.prepared.get_or_init(|| prepare_data(data)) {
		Ok(prepared) => Ok(prepared),
		Err(error) => Err(error.clone()), }
}
fn prepare_data(data: &Data) -> Result<Prepared, RecipeError> {
	let mut paths = Vec::new(); for source in &data.sources {
		collect_files(&expand_home(source)?, &mut paths)?; }
	paths.sort(); paths.dedup(); let mut tables = Vec::new(); let mut images = BTreeMap::<String, usize>::new(); for path in paths {
		let bytes = fs::read(&path).map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?; if let Some(extension) = image_extension(&path) {
			if bytes.is_empty() {
				return Err(RecipeError::new(format!("image {} is empty", path.display()))); }
			*images.entry(extension.to_owned()).or_default() += 1; continue; }
		tables.push(parse_table(&path, &bytes)?); }
	let mut matches = Vec::new(); for (table, value) in tables.iter().enumerate() {
		for (column, header) in value.headers.iter().enumerate() {
			let qualified = format!("{}.{}", value.name, header); if data.target == *header || data.target == qualified {
				matches.push((table, column)); } } }
	if matches.is_empty() {
		return Err(RecipeError::new(format!("target {:?} is absent", data.target))); }
	if matches.len() != 1 {
		return Err(RecipeError::new(format!("target {:?} is ambiguous, use file.feature", data.target))); }
	let (table_index, target) = matches[0]; let selected = &tables[table_index]; let fit_rows = ((selected.rows.len() as f64) * data.split).floor().max(1.0) as usize; let kinds = selected.headers.iter().enumerate().map(|(column, _)| infer_feature(selected, column, fit_rows)).collect::<Vec<_>>(); eprintln!("Feature name:                         Dtype:    Samples:"); for table in &tables {
		for (column, header) in table.headers.iter().enumerate() {
			let kind = infer_feature(table, column, ((table.rows.len() as f64) * data.split).floor().max(1.0) as usize); eprintln!("{:<37} {:<9} {}", format!("{}.{}", table.name, header), kind_name(&kind), table.rows.iter().filter(|row| row.get(column).is_some_and(|value| !value.is_empty())).count()); } }
	for (extension, count) in images {
		eprintln!("{extension:<37} u8 bytes  {count}"); }
	let columns = selected
		.headers
		.iter()
		.enumerate()
		.filter_map(|(column, header)| {
			let qualified = format!("{}.{}", selected.name, header); (column != target && !data.exclusions.iter().any(|excluded| excluded == header || excluded == &qualified)).then_some(column) })
		.collect::<Vec<_>>(); let features = columns.iter().map(|column| feature_width(&kinds[*column])).sum(); if features == 0 {
		return Err(RecipeError::new("dataset has no training features")); }
	let target_categories = categories(selected, target, fit_rows); let mut samples = Vec::new(); let mut targets = Vec::new(); for row in &selected.rows {
		let mut encoded = Vec::with_capacity(features); let mut valid = true; for column in &columns {
			if let Some(value) = row.get(*column) {
				valid &= encode(value, &kinds[*column], &mut encoded); } else {
				valid = false; } }
		let target_value = row.get(target).and_then(|value| value.parse::<f64>().ok()).or_else(|| row.get(target).and_then(|value| target_categories.iter().position(|category| category == value).map(|index| index as f64))); if valid
			&& let Some(target_value) = target_value
			&& target_value.is_finite()
		{
			samples.extend(encoded); targets.push(target_value); } }
	let rows = targets.len(); if rows == 0 {
		return Err(RecipeError::new("dataset has no complete training rows")); }
	let schema = columns.iter().map(|column| format!("{}.{}:{}:{}", selected.name, selected.headers[*column], kind_name(&kinds[*column]), feature_width(&kinds[*column]))).collect::<Vec<_>>().join("|"); Ok(Prepared { samples, targets, rows, features, schema, target_categorical: matches!(kinds[target], FeatureType::Categoric(_)) })
}
fn expand_home(source: &str) -> Result<PathBuf, RecipeError> {
	if source == "~" || source.starts_with("~/") {
		let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?; return Ok(PathBuf::from(home).join(source.trim_start_matches("~/"))); }
	Ok(PathBuf::from(source))
}
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), RecipeError> {
	let metadata = fs::metadata(path).map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?; if metadata.is_file() {
		files.push(path.to_owned()); return Ok(()); }
	let mut children = fs::read_dir(path).map_err(|error| RecipeError::new(format!("cannot read folder {}: {error}", path.display())))?.collect::<Result<Vec<_>, _>>().map_err(|error| RecipeError::new(format!("cannot read folder {}: {error}", path.display())))?; children.sort_by_key(fs::DirEntry::path); for child in children {
		collect_files(&child.path(), files)?; }
	Ok(())
}
fn parse_table(path: &Path, bytes: &[u8]) -> Result<Table, RecipeError> {
	let first = bytes.split(|byte| *byte == b'\n').next().ok_or_else(|| RecipeError::new("dataset is empty"))?; let delimiter = match [b',', b';', b'\t'].into_iter().max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count()).filter(|delimiter| first.contains(delimiter)) {
		Some(value) => value,
		None => b'\t', }; let mut records = records(bytes, delimiter)?; if records.is_empty() {
		return Err(RecipeError::new(format!("dataset {} is empty", path.display()))); }
	let headers = records.remove(0); let width = headers.len(); records.retain(|row| row.len() == width); let name = path.file_stem().and_then(|name| name.to_str()).ok_or_else(|| RecipeError::new(format!("file {} has no UTF-8 name", path.display())))?.to_owned(); Ok(Table { name, headers, rows: records })
}
fn records(bytes: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>, RecipeError> {
	let mut rows = Vec::new(); let mut row = Vec::new(); let mut field = Vec::new(); let mut quoted = false; let mut index = 0; while index < bytes.len() {
		let byte = bytes[index]; if byte == b'"' {
			if quoted && bytes.get(index + 1) == Some(&b'"') {
				field.push(b'"'); index += 1; } else {
				quoted = !quoted; } } else if byte == delimiter && !quoted {
			row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature text is not UTF-8"))?); field = Vec::new(); } else if byte == b'\n' && !quoted {
			row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature text is not UTF-8"))?.trim_end_matches('\r').to_owned()); field = Vec::new(); if row.iter().any(|value| !value.is_empty()) {
				rows.push(row); }
			row = Vec::new(); } else {
			field.push(byte); }
		index += 1; }
	if quoted {
		return Err(RecipeError::new("unterminated quoted feature")); }
	if !field.is_empty() || !row.is_empty() {
		row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature text is not UTF-8"))?); rows.push(row); }
	Ok(rows)
}
fn categories(table: &Table, column: usize, rows: usize) -> Vec<String> {
	table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty()).cloned().collect::<BTreeSet<_>>().into_iter().collect()
}
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
	let values = table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty()).collect::<Vec<_>>(); if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) {
		return FeatureType::Numeric(numeric_name(&values)); }
	let categories = categories(table, column, rows); if categories.len() < values.len() { FeatureType::Categoric(categories) } else { FeatureType::Text(values.iter().map(|value| value.len()).max().map_or(0, |value| value)) }
}
fn numeric_name(values: &[&String]) -> &'static str {
	let parsed = values.iter().filter_map(|value| value.parse::<f64>().ok()).collect::<Vec<_>>(); if parsed.iter().all(|value| value.fract() == 0.0) {
		let min = parsed.iter().copied().fold(f64::INFINITY, f64::min); let max = parsed.iter().copied().fold(f64::NEG_INFINITY, f64::max); if min >= 0.0 {
			if max <= u8::MAX as f64 {
				"u8" } else if max <= u16::MAX as f64 {
				"u16" } else if max <= u32::MAX as f64 {
				"u32" } else {
				"u64" } } else if min >= i8::MIN as f64 && max <= i8::MAX as f64 {
			"i8" } else if min >= i16::MIN as f64 && max <= i16::MAX as f64 {
			"i16" } else if min >= i32::MIN as f64 && max <= i32::MAX as f64 {
			"i32" } else {
			"i64" } } else if parsed.iter().all(|value| (*value as f32) as f64 == *value) {
		"f32" } else {
		"f64" }
}
fn kind_name(kind: &FeatureType) -> &'static str {
	match kind {
		FeatureType::Numeric(name) => name,
		FeatureType::Categoric(_) => "categoric",
		FeatureType::Text(_) => "string", }
}
fn feature_width(kind: &FeatureType) -> usize {
	match kind {
		FeatureType::Numeric(_) => 1,
		FeatureType::Categoric(values) => values.len(),
		FeatureType::Text(width) => *width, }
}
fn encode(value: &str, kind: &FeatureType, output: &mut Vec<f64>) -> bool {
	match kind {
		FeatureType::Numeric(_) => {
			if let Ok(value) = value.parse::<f64>() {
				output.push(value); value.is_finite() } else {
				false } }
		FeatureType::Categoric(categories) => {
			let found = categories.iter().position(|category| category == value); output.extend((0..categories.len()).map(|index| f64::from(Some(index) == found))); true }
		FeatureType::Text(width) => {
			output.extend(value.as_bytes().iter().map(|byte| f64::from(*byte)).chain(std::iter::repeat(0.0)).take(*width)); value.len() <= *width } }
}
fn image_extension(path: &Path) -> Option<&str> {
	const IMAGE: &[&str] = &["gif", "webp", "avif", "jxl", "apng", "mp4", "mov", "avi", "mkv", "webm", "flv", "wmv", "jpg", "jpeg", "png", "bmp", "tiff", "tif", "ico", "heic", "heif", "pgm", "ppm", "pbm", "pnm", "tga", "qoi", "jp2", "raw", "cr2", "nef", "arw", "dng", "orf", "rw2", "psd", "exr", "hdr", "svg"]; path.extension().and_then(|extension| extension.to_str()).map(str::to_ascii_lowercase).and_then(|extension| IMAGE.contains(&extension.as_str()).then(|| path.extension().and_then(|value| value.to_str())).flatten())
}
fn configured_f64(name: &str, value: &str) -> Result<f64, RecipeError> {
	value.parse().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))
}
fn activation_config() -> Result<ActivationConfig, RecipeError> {
	Ok(ActivationConfig {
		leak: configured_f64("RECIPE_LEAK_SLOPE", env!("RECIPE_LEAK_SLOPE"))?,
		prelu: configured_f64("RECIPE_PRELU_INITIAL_SLOPE", env!("RECIPE_PRELU_INITIAL_SLOPE"))?,
		elu: configured_f64("RECIPE_ELU_ALPHA", env!("RECIPE_ELU_ALPHA"))?,
		selu_alpha: configured_f64("RECIPE_SELU_ALPHA", env!("RECIPE_SELU_ALPHA"))?,
		selu_scale: configured_f64("RECIPE_SELU_SCALE", env!("RECIPE_SELU_SCALE"))?,
		gelu_scale: configured_f64("RECIPE_GELU_SCALE", env!("RECIPE_GELU_SCALE"))?,
		gelu_cubic: configured_f64("RECIPE_GELU_CUBIC", env!("RECIPE_GELU_CUBIC"))?, })
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Calculation {
	Conv1d(u64, u64, u64, u64, u64, u64, u64),
	BiasAdd(u64, u64, u64, u64),
	Matmul(u64, u64, u64, u64),
	Add(u64),
	Relu(u64),
}
impl Calculation {
	const fn flops(&self) -> u64 {
		match self {
			Self::Conv1d(_, _, _, _, _, _, flops) | Self::BiasAdd(_, _, _, flops) | Self::Matmul(_, _, _, flops) => *flops,
			Self::Add(elements) | Self::Relu(elements) => *elements, } }
}
pub struct Lowering {
	tensors: Vec<Tensor>,
	operations: Vec<Operation>,
	reads: Vec<Value>,
	output: Value,
	calculations: Vec<Calculation>,
	transfer_bytes: u64,
	flops: u64,
	balance: f64,
}
impl Lowering {
	#[must_use]
	pub fn shape(&self, value: Value) -> Option<&[u64]> {
		self.tensors.get(value.0).map(|tensor| tensor.shape.as_slice()) }
	#[must_use]
	pub const fn transfer_bytes(&self) -> u64 {
		self.transfer_bytes }
	#[must_use]
	pub const fn flops(&self) -> u64 {
		self.flops }
	#[must_use]
	pub fn bytes_per_flop(&self) -> f64 {
		if self.flops == 0 { f64::INFINITY } else { self.transfer_bytes as f64 / self.flops as f64 } }
	#[must_use]
	pub fn bound(&self) -> Bound {
		if self.bytes_per_flop() >= self.balance { Bound::Transfer } else { Bound::Calculation } }
}
impl fmt::Display for Lowering {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let title = if self.operations.len() == 1 {
			match self.operations[0] {
				Operation::Conv1d(..) => "conv",
				Operation::Matmul(..) => "matmul",
				Operation::Add(..) => "add",
				Operation::Relu(..) => "relu", } } else {
			"recipe" }; writeln!(formatter, "{title}\n\tshape")?; for value in &self.reads {
			let tensor = &self.tensors[value.0]; writeln!(formatter, "\t\t{:<7} {}", tensor.name, Shape(&tensor.shape))?; }
		let output = &self.tensors[self.output.0]; writeln!(formatter, "\t\t{:<7} {}\n\ttransfer", output.name, Shape(&output.shape))?; for value in &self.reads {
			let tensor = &self.tensors[value.0]; let bytes = elements(&tensor.shape).map_err(|_| fmt::Error)? * tensor.dtype.bytes(); writeln!(formatter, "\t\tread {:<7} {bytes} bytes", tensor.name)?; }
		let output_bytes = elements(&output.shape).map_err(|_| fmt::Error)? * output.dtype.bytes(); writeln!(formatter, "\t\twrite {:<6} {output_bytes} bytes\n\t\ttotal        {} bytes\n\tcalc", output.name, self.transfer_bytes)?; for calculation in &self.calculations {
			match calculation {
				Calculation::Conv1d(n, cin, lin, cout, kernel, stride, flops) => writeln!(formatter, "\t\tconv1d\n\t\t\tn      {n}\n\t\t\tcin    {cin}\n\t\t\tlin    {lin}\n\t\t\tcout   {cout}\n\t\t\tkernel {kernel}\n\t\t\tstride {stride}\n\t\t\tflop   {flops}")?,
				Calculation::BiasAdd(n, cout, lout, flops) => writeln!(formatter, "\t\tbias_add\n\t\t\tn    {n}\n\t\t\tcout {cout}\n\t\t\tlout {lout}\n\t\t\tflop {flops}")?,
				Calculation::Matmul(m, k, n, flops) => writeln!(formatter, "\t\tmatmul\n\t\t\tm    {m}\n\t\t\tk    {k}\n\t\t\tn    {n}\n\t\t\tflop {flops}")?,
				Calculation::Add(elements) => writeln!(formatter, "\t\tadd\n\t\t\tflop {elements}")?,
				Calculation::Relu(elements) => writeln!(formatter, "\t\trelu\n\t\t\tflop {elements}")?, } }
		write!(formatter, "\tschedule\n\t\ttransfer {} bytes\n\t\tcalc     {} flop\n\t\tratio    {:.2} bytes/flop\n\t\tbound    {}", self.transfer_bytes, self.flops, self.bytes_per_flop(), self.bound()) }
}
struct Shape<'a>(&'a [u64]); impl fmt::Display for Shape<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("[")?; for (index, dimension) in self.0.iter().enumerate() {
			if index != 0 {
				formatter.write_str(", ")?; }
			write!(formatter, "{dimension}")?; }
		formatter.write_str("]") }
}
fn unique_name(tensors: &[Tensor], stem: &str) -> String {
	if !tensors.iter().any(|tensor| tensor.name == stem) {
		return stem.to_owned(); }
	let mut ordinal = 2_u64; loop {
		let candidate = format!("{stem}_{ordinal}"); if !tensors.iter().any(|tensor| tensor.name == candidate) {
			return candidate; }
		ordinal += 1; }
}
fn elements(shape: &[u64]) -> Result<u64, RecipeError> {
	if shape.is_empty() {
		return Err(RecipeError::new("tensor shape must have at least one dimension")); }
	if shape.contains(&0) {
		return Err(RecipeError::new("tensor dimensions must be positive")); }
	product(shape)
}
fn product(values: &[u64]) -> Result<u64, RecipeError> {
	values.iter().try_fold(1_u64, |product, value| product.checked_mul(*value).ok_or_else(|| RecipeError::new("dimension product overflow")))
}
fn rank<const N: usize>(shape: &[u64], role: &str) -> Result<[u64; N], RecipeError> {
	shape.try_into().map_err(|_| RecipeError::new(format!("{role} must have rank {N}")))
}
fn usize_extent(value: u64) -> Result<usize, RecipeError> {
	usize::try_from(value).map_err(|_| RecipeError::new("tensor extent does not fit host memory"))
}
fn cast(dtype: DType, value: f64) -> f64 {
	match dtype {
		DType::F32 => f64::from(value as f32),
		DType::F64 | DType::I32 | DType::I64 => value, }
}

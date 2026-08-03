use core::fmt; use std::{ cell::RefCell, collections::{BTreeMap, BTreeSet}, convert::Infallible, path::PathBuf, };

use recipe_core::{DeviceId, Label, MachineId, RunId, TargetIdentity, ToolchainIdentity};
use recipe_cuda::{DeploymentIdentity, REQUIRED_DRIVER_SYMBOLS, ToolchainIdentity as CudaToolchainIdentity};
use recipe_host::{DiskFileSpec, HostBackendConfig, HostDeviceBinding};
use recipe_kernel::{AmdTarget, ArtifactBuilder, KernelTarget, LoweringError, NvidiaTarget};
use recipe_native_probe::{NativeExecutionBindings, NativeGpuProbe, NativeProbeConfig, with_native_execution_bindings};
use recipe_prepare::{
	CudaArtifactPolicy, DeferredArtifactCompiler, NativePrepareError, RuntimeArtifactPolicy, TargetBuildSpec, };
use recipe_probe::{
	ExplicitPathProfileCache, GpuDescriptor, GpuDiscovery as _, HostInventory, MeasuredProfile, ProbeError,
	ProfileCache, ResolvedLocalInventory, };

const CUDA_BACKEND: &str = "nvidia-cuda-driver";
const CUDA_ABI: &str = "elf64-cubin";
const HSA_BACKEND: &str = "amd-rocr-hsa";
const HSA_TARGET_PREFIX: &str = "amdgcn-amd-amdhsa--";
thread_local! {
	static CURRENT_NATIVE_PROBE: RefCell<Option<(NativeProbeConfig, NativeGpuProbe)>> = const { RefCell::new(None) }; }

/// A failure before Recipe owns a finalized native execution session.
#[derive(Debug)]
#[non_exhaustive]
pub enum NativePreparationError { ProfileNotFound { path: PathBuf }, Probe(ProbeError), Lowering(LoweringError),
	TargetSpecification(NativePrepareError<Infallible>), LocalConfiguration(String), IdentityMismatch { detail: String }, }

impl fmt::Display for NativePreparationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self { Self::ProfileNotFound { path } => { write!( formatter,
					"measured profile does not exist at {}",
					path.display() ) }
			Self::Probe(error) => write!(formatter, "native probe/profile validation failed: {error}"),
			Self::Lowering(error) => write!(formatter, "native artifact builder is invalid: {error}"),
			Self::TargetSpecification(error) => {
				write!(formatter, "native target specification is invalid: {error}")
			}
			Self::LocalConfiguration(detail) => write!(formatter, "local native configuration failed: {detail}"),
			Self::IdentityMismatch { detail } => write!(formatter, "native identity mismatch: {detail}"),
		} } }

impl std::error::Error for NativePreparationError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self { Self::Probe(error) => Some(error), Self::Lowering(error) => Some(error),
			Self::TargetSpecification(error) => Some(error), Self::ProfileNotFound { .. }
			| Self::LocalConfiguration(_)
			| Self::IdentityMismatch { .. } => None, } } }

pub type NativePreparationResult<T> = Result<T, NativePreparationError>;

/// One measured GPU origin and the exact build identity selected for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeDeviceBuildTarget { device: DeviceId, origin: Label, target: TargetIdentity,
	toolchain: ToolchainIdentity, }

impl NativeDeviceBuildTarget {
	#[must_use]
	pub const fn device(&self) -> DeviceId { self.device }

	#[must_use]
	pub const fn origin(&self) -> &Label { &self.origin }

	#[must_use]
	pub const fn target(&self) -> &TargetIdentity { &self.target }

	#[must_use]
	pub const fn toolchain(&self) -> &ToolchainIdentity { &self.toolchain } }

/// Owned native compiler inputs covering every GPU on one exact local
/// measured machine.
#[derive(Clone, Debug)]
pub struct NativeTargetPlan { machine: MachineId, devices: Vec<NativeDeviceBuildTarget>, targets: Vec<TargetBuildSpec>,
}

/// Owned host-side device origins matching one measured local machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeHostPlan { machine: MachineId, ram: Vec<DeviceId>, storage: Vec<(DeviceId, PathBuf)>, }

impl NativeHostPlan {
	#[must_use]
	pub const fn machine(&self) -> MachineId { self.machine }



	/// Construct deterministic run-scoped host resources without performing
	/// I/O. Candidate realization later creates every path with `create_new`.
	pub fn backend_config( &self, run: RunId, worker_threads: usize, staging_bytes_per_worker: usize,
	) -> recipe_host::Result<HostBackendConfig> { let mut bindings = self .ram .iter() .copied()
			.map(|device| HostDeviceBinding::Ram { device }) .collect::<Vec<_>>(); for (device, root) in &self.storage {
			let stem = format!(".recipe-run-{run}-device-{device}");
			bindings.push(HostDeviceBinding::Disk { device: *device,
				arena: DiskFileSpec::new(root.join(format!("{stem}-arena")))?,
			}); }
		HostBackendConfig::new(worker_threads, staging_bytes_per_worker, bindings) } }

impl NativeTargetPlan {
	#[must_use]
	pub const fn machine(&self) -> MachineId { self.machine }

	#[must_use]
	pub fn devices(&self) -> &[NativeDeviceBuildTarget] { &self.devices }

	/// Unique specifications keyed by calculation target. Multiple identical
	/// GPUs share one compiler specification without losing their device
	/// entries in [`Self::devices`].
	#[must_use]
	pub fn target_build_specs(&self) -> &[TargetBuildSpec] { &self.targets }

	/// Construct the validated deferred compiler used by production
	/// preparation.
	///
	/// # Errors
	///
	/// Fails if these already checked specifications are modified into an
	/// invalid or duplicate target set before this call.
	pub fn deferred_compiler(&self) -> Result<DeferredArtifactCompiler, NativePrepareError<Infallible>> { DeferredArtifactCompiler::new(self.targets.clone()) }

}

/// Preparation-scoped native handles plus their exact owned compiler inputs.
#[derive(Debug)]
pub struct NativePreparationScope<'cuda, 'hsa> {
	bindings: NativeExecutionBindings<'cuda, 'hsa>,
	host: NativeHostPlan, targets: NativeTargetPlan, }

impl<'cuda, 'hsa> NativePreparationScope<'cuda, 'hsa> {
	#[must_use]
	pub const fn bindings(&self) -> &NativeExecutionBindings<'cuda, 'hsa> { &self.bindings }

	#[must_use]
	pub const fn targets(&self) -> &NativeTargetPlan { &self.targets }

	#[must_use]
	pub const fn host(&self) -> &NativeHostPlan { &self.host }

	#[must_use]
	pub fn into_parts( self, ) -> (
		NativeExecutionBindings<'cuda, 'hsa>,
		NativeHostPlan, NativeTargetPlan, ) { (self.bindings, self.host, self.targets) }

	#[must_use]
	pub fn into_targets(self) -> NativeTargetPlan { self.targets } }

fn with_native_preparation_from_probe<T>( probe: &NativeGpuProbe, config: &NativeProbeConfig, profile: &MeasuredProfile,
	host: &HostInventory,
	operation: impl for<'cuda, 'hsa> FnOnce(NativePreparationScope<'cuda, 'hsa>) -> NativePreparationResult<T>,
) -> NativePreparationResult<T> { let inventory = probe .discover_all() .map_err(NativePreparationError::Probe)?;
	let resolved = profile .resolve_local_inventory(host, &inventory) .map_err(NativePreparationError::Probe)?;
	let machine = resolved.machine(); let host_plan = owned_host_plan(&resolved);
	let devices = owned_gpu_inventory(&resolved); require_complete_local_gpu_scope(profile, machine, &devices)?;
	if devices.is_empty() { return Err(identity_error(
			"measured local machine contains no GPU calculation device",
		)); }

	let mut operation = Some(operation); let mut callback_result = None;
	with_native_execution_bindings(probe, profile, host, |bindings| {
		let scope = build_scope(config, machine, host_plan, &devices, bindings); callback_result = Some(match scope {
			Ok(scope) => { operation .take()
					.expect("native preparation callback is invoked at most once")(scope)
			}
			Err(error) => Err(error), }); Ok(()) }) .map_err(NativePreparationError::Probe)?;
	callback_result.ok_or_else(|| identity_error("native preparation callback was not invoked"))?
}

/// Resolve the exact current bare-metal cache identity produced by the
/// zero-argument `recipe probe`, load that profile, and lend matching native
/// bindings to one callback. This never selects a merely newest cache file.
/// The thread retains its exact probe root so repeated training calls reuse
/// one initialized GPU runtime while still rebuilding all per-run resources.
pub fn with_current_native_preparation<T>(
	operation: impl for<'cuda, 'hsa> FnOnce(
		&MeasuredProfile, &NativeProbeConfig,
		NativePreparationScope<'cuda, 'hsa>,
	) -> NativePreparationResult<T>, ) -> NativePreparationResult<T> {
	let current = crate::cli::current_native_inputs().map_err(NativePreparationError::LocalConfiguration)?;
	let cache = ExplicitPathProfileCache::new(current.profile_path.clone()).map_err(NativePreparationError::Probe)?;
	let profile = cache .load(current.profile_identity) .map_err(NativePreparationError::Probe)? .ok_or_else(|| {
			NativePreparationError::ProfileNotFound { path: current.profile_path.clone(), } })?;
	CURRENT_NATIVE_PROBE.with(|cached| { let mut cached = cached.borrow_mut();
		if let Some((config, _probe)) = cached.as_ref() && config != &current.config
		{ return Err(NativePreparationError::IdentityMismatch {
				detail: "current native configuration changed after this thread opened its GPU runtime"
					.to_owned(), }); }
		if cached.is_none() { *cached = Some((current.config.clone(), current.native)); }
		let probe = &cached .as_ref()
			.expect("the current native probe cache was populated above")
			.1; with_native_preparation_from_probe(probe, &current.config, &profile, &current.host, |scope| {
			operation(&profile, &current.config, scope) }) }) }

fn owned_gpu_inventory(resolved: &ResolvedLocalInventory<'_>) -> Vec<(DeviceId, GpuDescriptor)> {
	let mut devices = resolved .gpu() .iter() .map(|resolved| (resolved.device(), resolved.descriptor().clone()))
		.collect::<Vec<_>>(); devices.sort_by_key(|(device, _)| *device); devices }

fn require_complete_local_gpu_scope( profile: &MeasuredProfile, machine: MachineId,
	devices: &[(DeviceId, GpuDescriptor)], ) -> NativePreparationResult<()> { let local = devices .iter()
		.map(|(device, _)| *device) .collect::<BTreeSet<_>>(); let outside = profile .discovery .devices .iter()
		.filter(|discovered| discovered.calculation.is_some()) .filter_map(|discovered| { profile .topology
				.device(discovered.device) .filter(|device| device.machine != machine || !local.contains(&device.id))
				.map(|device| device.id) }) .collect::<Vec<_>>(); if outside.is_empty() { Ok(()) } else {
		Err(identity_error(format!(
			"local native scope cannot silently cover nonlocal GPU devices {outside:?}"
		))) } }

fn owned_host_plan(resolved: &ResolvedLocalInventory<'_>) -> NativeHostPlan {
	let mut ram = resolved .ram() .iter() .map(|device| device.device()) .collect::<Vec<_>>(); ram.sort();
	let mut storage = resolved .storage() .iter() .map(|device| (device.device(), device.domain().benchmark_root.clone()))
		.collect::<Vec<_>>(); storage.sort_by_key(|(device, _)| *device); NativeHostPlan { machine: resolved.machine(), ram,
		storage, } }

fn build_scope<'cuda, 'hsa>(
	config: &NativeProbeConfig, machine: MachineId, host: NativeHostPlan, devices: &[(DeviceId, GpuDescriptor)],
	bindings: NativeExecutionBindings<'cuda, 'hsa>,
) -> NativePreparationResult<NativePreparationScope<'cuda, 'hsa>> {
	if bindings.machine() != machine { return Err(identity_error(format!(
			"binding machine {} differs from resolved machine {machine}",
			bindings.machine() ))); }
	let cuda = unique_cuda_bindings(&bindings)?; let hsa = unique_hsa_bindings(&bindings)?; let expected = devices .iter()
		.map(|(device, _)| *device) .collect::<BTreeSet<_>>(); let actual = cuda .keys() .chain(hsa.keys()) .copied()
		.collect::<BTreeSet<_>>(); if actual != expected || cuda.keys().any(|device| hsa.contains_key(device)) {
		return Err(identity_error(format!(
			"native binding device set {actual:?} differs from measured GPU set {expected:?}"
		))); }

	let builder = ArtifactBuilder::new(config.kernels.toolchain.clone()).map_err(NativePreparationError::Lowering)?;
	let mut device_targets = Vec::with_capacity(devices.len());
	let mut unique_targets = BTreeMap::<TargetIdentity, TargetBuildSpec>::new(); for (device, descriptor) in devices {
		let spec = match descriptor.target.backend.as_str() { CUDA_BACKEND => { let binding = cuda .get(device)
					.ok_or_else(|| identity_error(format!("CUDA device {device} has no exact binding")))?;
				cuda_spec(config, descriptor, binding.deployment(), builder.clone())? }
			HSA_BACKEND => { let binding = hsa .get(device)
					.ok_or_else(|| identity_error(format!("HSA device {device} has no exact binding")))?;
				hsa_spec(config, descriptor, binding, builder.clone())? }
			backend => { return Err(identity_error(format!(
					"GPU device {device} uses unsupported native backend {backend}"
				))); } }; if let Some(existing) = unique_targets.get(&descriptor.target) {
			require_equivalent_specs(existing, &spec)?; } else { unique_targets.insert(descriptor.target.clone(), spec); }
		device_targets.push(NativeDeviceBuildTarget { device: *device, origin: descriptor.key.clone(),
			target: descriptor.target.clone(), toolchain: descriptor.toolchain.clone(), }); }
	let targets = unique_targets.into_values().collect::<Vec<_>>();
	DeferredArtifactCompiler::new(targets.clone()).map_err(NativePreparationError::TargetSpecification)?;
	Ok(NativePreparationScope { bindings, host, targets: NativeTargetPlan { machine, devices: device_targets, targets, },
	}) }

fn unique_cuda_bindings<'scope, 'cuda, 'hsa>(
	bindings: &'scope NativeExecutionBindings<'cuda, 'hsa>,
) -> NativePreparationResult<BTreeMap<DeviceId, &'scope recipe_native_executor::CudaBinding<'cuda>>> {
	let mut devices = BTreeMap::new(); for binding in bindings.cuda() {
		if devices.insert(binding.device(), binding).is_some() { return Err(identity_error(format!(
				"CUDA binding device {} appears more than once",
				binding.device() ))); } }
	Ok(devices) }

fn unique_hsa_bindings<'scope, 'cuda, 'hsa>(
	bindings: &'scope NativeExecutionBindings<'cuda, 'hsa>,
) -> NativePreparationResult<BTreeMap<DeviceId, &'scope recipe_native_executor::HsaBinding<'hsa>>> {
	let mut devices = BTreeMap::new(); for binding in bindings.hsa() {
		if devices.insert(binding.device(), binding).is_some() { return Err(identity_error(format!(
				"HSA binding device {} appears more than once",
				binding.device() ))); } }
	Ok(devices) }

fn cuda_spec( config: &NativeProbeConfig, descriptor: &GpuDescriptor, deployment: &DeploymentIdentity,
	builder: ArtifactBuilder, ) -> NativePreparationResult<TargetBuildSpec> {
	let sm_major = u8::try_from(deployment.target.major)
		.map_err(|error| identity_error(format!("CUDA SM major does not fit u8: {error}")))?;
	let sm_minor = u8::try_from(deployment.target.minor)
		.map_err(|error| identity_error(format!("CUDA SM minor does not fit u8: {error}")))?;
	let target = NvidiaTarget { sm_major, sm_minor, ptx_isa: config.cuda.ptx_isa, }; target.validate()
		.map_err(NativePreparationError::Lowering)?;
	if descriptor.target.abi.as_str() != CUDA_ABI || descriptor.target.architecture.as_str() != target.architecture()
	{ return Err(identity_error(format!(
			"CUDA deployment target {} does not match measured target {:?}",
			target.architecture(), descriptor.target ))); }
	let required_driver_symbols = REQUIRED_DRIVER_SYMBOLS .iter() .copied() .collect::<BTreeSet<_>>();
	if let Some(missing) = required_driver_symbols .iter()
		.find(|symbol| !deployment.driver_capabilities.supports(**symbol))
	{ return Err(identity_error(format!(
			"CUDA deployment omitted required Driver API symbol {}",
			missing.as_str() ))); }
	let driver = deployment.driver_version; Ok(TargetBuildSpec { target_identity: descriptor.target.clone(),
		target: KernelTarget::Nvidia(target), toolchain_identity: descriptor.toolchain.clone(), builder,
		scratch_parent: config.kernels.scratch_parent.clone(),
		runtime_policy: RuntimeArtifactPolicy::Cuda(CudaArtifactPolicy { toolchain: cuda_toolchain_identity(config)?,
			minimum_driver: driver, maximum_driver: Some(driver), required_driver_symbols, }), }) }

fn hsa_spec( config: &NativeProbeConfig, descriptor: &GpuDescriptor,
	binding: &recipe_native_executor::HsaBinding<'_>,
	builder: ArtifactBuilder, ) -> NativePreparationResult<TargetBuildSpec> {
	if descriptor.target.architecture.as_str() != binding.target_id()
		|| binding.code_object_version() != config.hsa.code_object_version
	{ return Err(identity_error(format!(
			"HSA binding target {} code-object v{} differs from measured target {:?} and configured v{}",
			binding.target_id(), binding.code_object_version(), descriptor.target, config.hsa.code_object_version ))); }
	let compiler_target = binding .target_id() .strip_prefix(HSA_TARGET_PREFIX) .ok_or_else(|| { identity_error(format!(
				"HSA binding target {} lacks the canonical AMDGPU prefix",
				binding.target_id() )) })?; let target = AmdTarget { target_id: compiler_target.to_owned(),
		code_object_version: binding.code_object_version(), }; target.validate() .map_err(NativePreparationError::Lowering)?;
	Ok(TargetBuildSpec { target_identity: descriptor.target.clone(), target: KernelTarget::Amd(target),
		toolchain_identity: descriptor.toolchain.clone(), builder, scratch_parent: config.kernels.scratch_parent.clone(),
		runtime_policy: RuntimeArtifactPolicy::Hsa, }) }

fn cuda_toolchain_identity(config: &NativeProbeConfig) -> NativePreparationResult<CudaToolchainIdentity> {
	let assembler = config .kernels .toolchain .ptx_assembler .as_ref()
		.ok_or_else(|| identity_error("measured NVIDIA target has no pinned PTX assembler"))?;
	Ok(CudaToolchainIdentity {
		// `recipe-cuda` predates the Rust-owned IR path and retains this legacy
		// field. State non-use explicitly rather than inventing a Zig version.
		zig_version: "not-used-recipe-rust-owned-ir".to_owned(),
		llvm_version: format!(
			"{};opt-sha256={};llc-sha256={}",
			config.kernels.release, config.kernels.toolchain.verifier.digest.to_hex(),
			config.kernels.toolchain.llvm_codegen.digest.to_hex() ), ptx_isa_version: config.cuda.ptx_isa.to_string(),
		ptxas_version: format!("sha256={}", assembler.digest.to_hex()),
		// Recipe invokes only the pinned assembler, not a CUDA toolkit API.
		cuda_toolkit_version: "not-claimed-pinned-ptxas-only".to_owned(),
		cubin_format: CUDA_ABI.to_owned(), }) }

fn require_equivalent_specs(existing: &TargetBuildSpec, candidate: &TargetBuildSpec) -> NativePreparationResult<()> {
	if existing.target_identity == candidate.target_identity && existing.target == candidate.target
		&& existing.toolchain_identity == candidate.toolchain_identity
		&& existing.scratch_parent == candidate.scratch_parent
		&& existing.runtime_policy == candidate.runtime_policy
	{ Ok(()) } else { Err(identity_error(format!(
			"devices sharing target {:?} produced different native build policies",
			existing.target_identity ))) } }

fn identity_error(detail: impl Into<String>) -> NativePreparationError { NativePreparationError::IdentityMismatch {
		detail: detail.into(), } }

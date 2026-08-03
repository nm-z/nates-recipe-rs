use std::{ cell::RefCell, collections::BTreeSet, fmt, fs, path::PathBuf, time::{Duration, Instant}, };

use recipe_core::{Label, TargetIdentity, TransferLaneCount, TransportKind}; use recipe_hsa::{
	AgentDescription, AgentUuidBody, DeviceType, DiscoveredAgent, DispatchGeometry, IsaTarget, MemoryLocation,
	MemoryPoolFlags, MemorySegment, Pending, PollStatus, QueueConfig, QueueKind, Runtime, Session, SystemDescription, };
use recipe_kernel::{
	AmdTarget, ArtifactBuilder, BuildPhase, BuiltArtifact, KernelTarget, LoweringOptions, lower_elementwise, };
use recipe_probe::{BoundedBenchmarkPlan, GpuDescriptor, GpuMeasurement, LinkDuplex, ProbeError, ProbeResult};

use crate::{ benchmark::{
		calculation_rate, capacity, compute_buffer_bytes, fill_input, fma_template, measured, plan_bytes,
		time_bounded, transfer_rate, verify_compute_output, }, config::{BackendLibrary, KernelBuildConfig, NativeProbeConfig},
	identity::{ PinnedLibrary, backend_toolchain_identity, label, library_identity, pci_accelerator_present, pci_surface,
		selected_library, }, native::Backend, };

const HSA_KERNEL_ENTRY: &str = "recipe_probe_fma_f32";
const COMPLETION_POLL_INITIAL_DELAY: Duration = Duration::from_micros(50);
const COMPLETION_POLL_MAXIMUM_DELAY: Duration = Duration::from_millis(2);
const TIMED_OUT_CLEANUP_INITIAL_DELAY: Duration = Duration::from_millis(10);
const TIMED_OUT_CLEANUP_MAXIMUM_DELAY: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug)]
struct CompletionPollBackoff { next_delay: Duration, maximum_delay: Duration, }

impl CompletionPollBackoff { const fn benchmark() -> Self { Self { next_delay: COMPLETION_POLL_INITIAL_DELAY,
			maximum_delay: COMPLETION_POLL_MAXIMUM_DELAY, } }

	const fn timed_out_cleanup() -> Self { Self { next_delay: TIMED_OUT_CLEANUP_INITIAL_DELAY,
			maximum_delay: TIMED_OUT_CLEANUP_MAXIMUM_DELAY, } }

	fn wait(&mut self, maximum: Duration) { std::thread::sleep(self.next_delay.min(maximum)); self.advance(); }

	fn advance(&mut self) { self.next_delay = self.next_delay.saturating_mul(2).min(self.maximum_delay); } }

pub(crate) struct HsaBackend { library: BackendLibrary, host_memory_key: Label, pci_sysfs_root: PathBuf,
	code_object_version: u8, kernels: KernelBuildConfig, runtime: RefCell<Option<HsaRuntimeState>>, }

struct HsaRuntimeState { library: PinnedLibrary, runtime: Runtime, }

impl fmt::Debug for HsaBackend {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("HsaBackend")
			.field("library", &self.library)
			.field("host_memory_key", &self.host_memory_key)
			.field("pci_sysfs_root", &self.pci_sysfs_root)
			.field("code_object_version", &self.code_object_version)
			.field("kernels", &self.kernels)
			.field("runtime_open", &self.runtime.borrow().is_some())
			.finish() } }

impl HsaBackend { pub(crate) fn new(config: &NativeProbeConfig) -> ProbeResult<Self> {
		if config.hsa.code_object_version == 0 { return Err(ProbeError::Discovery(
				"HSA code-object version must be nonzero".to_owned(),
			)); }
		Ok(Self { library: config.hsa.library.clone(), host_memory_key: config.host_memory_key.clone(),
			pci_sysfs_root: config.pci_sysfs_root.clone(), code_object_version: config.hsa.code_object_version,
			kernels: config.kernels.clone(), runtime: RefCell::new(None), }) }

	pub(crate) fn with_runtime<T>( &self, operation: impl FnOnce(&PinnedLibrary, &Runtime) -> ProbeResult<T>,
	) -> ProbeResult<Option<T>> { if !pci_accelerator_present(&self.pci_sysfs_root, 0x1002)? {
			if self.runtime.borrow().is_some() { return Err(ProbeError::Discovery(
					"every AMD PCI accelerator disappeared after HSA initialization".to_owned(),
				)); }
			return Ok(None); }
		let Some(library) = selected_library(&self.library, "ROCr/HSA")? else {
			if self.runtime.borrow().is_some() { return Err(ProbeError::Discovery(
					"ROCr/HSA library disappeared after initialization".to_owned(),
				)); }
			return Err(ProbeError::Discovery(
				"AMD PCI accelerator is present but no configured ROCr/HSA runtime library exists".to_owned(),
			)); }; let mut state = self.runtime.borrow_mut(); if let Some(current) = state.as_ref()
			&& current.library != library
		{ return Err(ProbeError::Discovery(
				"ROCr/HSA library identity changed after initialization".to_owned(),
			)); }
		if state.is_none() { let runtime = Runtime::open(library.path.as_os_str())
				.map_err(|error| ProbeError::Discovery(format!("load ROCr/HSA runtime: {error}")))?;
			*state = Some(HsaRuntimeState { library, runtime }); }
		let state = state.as_ref().expect("runtime state was initialized above");
		operation(&state.library, &state.runtime).map(Some) }

	pub(crate) const fn code_object_version(&self) -> u8 { self.code_object_version }

	pub(crate) fn descriptor( &self, library: &PinnedLibrary, system: &SystemDescription, description: &AgentDescription,
	) -> ProbeResult<Option<GpuDescriptor>> { if description.device_type != DeviceType::Gpu { return Ok(None); }
		if !description.supports_kernel_dispatch() { return Err(ProbeError::Discovery(format!(
				"HSA GPU {} lacks kernel-dispatch capability",
				description.identity.uuid ))); }
		if !matches!(description.identity.uuid.body(), AgentUuidBody::Value(_)) { return Err(ProbeError::Discovery(format!(
				"HSA GPU at agent name {:?} did not report a stable UUID",
				description.identity.name ))); }
		let pci = description.identity.pci_address.ok_or_else(|| { ProbeError::Discovery(format!(
				"HSA GPU {} did not report an exact PCI address",
				description.identity.uuid )) })?; let pci = pci.to_string(); let surface = pci_surface(&self.pci_sysfs_root, &pci)?;
		let target = exact_target(description)?; let target_tail = hsa_target_tail(target)?;
		let gpu = description.amd_gpu.as_ref().ok_or_else(|| { ProbeError::Discovery(format!(
				"HSA GPU {} omitted AMD capability properties",
				description.identity.uuid )) })?; let queue = description.queue.ok_or_else(|| { ProbeError::Discovery(format!(
				"HSA GPU {} omitted its physical queue limits",
				description.identity.uuid )) })?; let toolchain = backend_toolchain_identity( &self.kernels.toolchain,
			&self.kernels.release,
			"amd-hsa",
			&format!(
				"{target_tail}:code-object-v{}:dependent-f32-fma-chain-{}",
				self.code_object_version, self.kernels.fma_chain_length ), )?;
		let runtime = library_identity("rocr-library", library);
		let name = if gpu.product_name.trim().is_empty() { description.identity.name.clone() } else { gpu.product_name.clone()
		}; let subgroup_lanes = description.first_isa_wavefront_size.ok_or_else(|| { ProbeError::Discovery(format!(
				"HSA GPU {} omitted its native wavefront width",
				description.identity.uuid )) })?; let maximum_workgroup_lanes = description .isas .iter() .map(|isa| {
				isa.maximum_workgroup_size .min(u32::from(isa.maximum_workgroup_dimensions[0])) }) .min() .ok_or_else(|| {
				ProbeError::Discovery(format!(
					"HSA GPU {} omitted ISA workgroup limits",
					description.identity.uuid )) })?; let maximum_shared_memory_per_workgroup =
			driver_lds_capacity(description.identity.driver_node_id.ok_or_else(|| { ProbeError::Discovery(format!(
					"HSA GPU {} omitted its KFD node",
					description.identity.uuid )) })?)?; Ok(Some(GpuDescriptor { key: label(
				format!("hsa:{}@{pci}", description.identity.uuid),
				"HSA GPU key",
			)?,
			name: label(name, "HSA GPU name")?,
			host_memory_key: self.host_memory_key.clone(), target: TargetIdentity {
				backend: label("amd-rocr-hsa", "HSA target backend")?,
				architecture: label(target.as_str().to_owned(), "HSA target architecture")?,
				abi: label(
					format!("elf64-amdgpu-code-object-v{}", self.code_object_version),
					"HSA target ABI",
				)?, }, capacity_hint: capacity(hsa_capacity(description)?)?, driver: label( format!(
					"amdgpu-kfd-node-{}:{}",
					description.identity.driver_node_id.ok_or_else(|| { ProbeError::Discovery(format!(
							"HSA GPU {} omitted its ROCr-reported driver node",
							description.identity.uuid )) })?, surface.driver ),
				"HSA driver identity",
			)?, runtime_abi: label( format!(
					"hsa-{}.{}-amdext-{}.{}:{runtime}",
					system.hsa_version_major, system.hsa_version_minor, system.amd_extension_version_major,
					system.amd_extension_version_minor ),
				"HSA runtime identity",
			)?,
			firmware: label(surface.firmware, "HSA firmware surface")?,
			link_identity: label(surface.link, "HSA PCIe link identity")?,
			transport_kind: TransportKind::Pcie, toolchain, duplex: LinkDuplex::Full,
			host_to_device_maximum_inflight: TransferLaneCount::new(1)
				.map_err(|error| ProbeError::Discovery(format!("HSA host-to-device lane count: {error}")))?,
			device_to_host_maximum_inflight: TransferLaneCount::new(1)
				.map_err(|error| ProbeError::Discovery(format!("HSA device-to-host lane count: {error}")))?,
			asynchronous_submission: true, maximum_submission_queues: queue.maximum_queues, maximum_concurrent_tasks: 1,
			subgroup_lanes, maximum_workgroup_lanes, maximum_shared_memory_per_workgroup,
			transfer_overlaps_calculation: gpu.sdma_engine_count != 0, })) }

	fn benchmark_device(&self, expected: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> {
		let result = self.with_runtime(|library, runtime| self.benchmark_with_runtime(library, runtime, expected, plan))?;
		let Some(measurement) = result else { return Err(ProbeError::Benchmark(
				"ROCr/HSA backend disappeared before benchmark".to_owned(),
			)); }; Ok(measurement) }

	fn benchmark_with_runtime( &self, library: &PinnedLibrary, runtime: &Runtime, expected: &GpuDescriptor,
		plan: BoundedBenchmarkPlan, ) -> ProbeResult<GpuMeasurement> { let discovery = runtime .discover()
			.map_err(|error| ProbeError::Benchmark(format!("HSA rediscovery: {error}")))?;
		let system = discovery.system().clone(); let selected_uuid = discovery .agents() .iter() .filter_map(|agent| {
				match self.descriptor(library, &system, agent.description()) { Ok(Some(descriptor)) if descriptor == *expected => {
						Some(Ok(agent.description().identity.uuid.as_str().to_owned())) }
					Ok(Some(_)) | Ok(None) => None, Err(error) => Some(Err(error)), } }) .collect::<ProbeResult<Vec<_>>>()?;
		if selected_uuid.len() != 1 { return Err(ProbeError::Benchmark(format!(
				"HSA GPU {} disappeared, changed, or is ambiguous",
				expected.key ))); }
		let selected_uuid = &selected_uuid[0]; let agents = discovery.into_agents(); let gpu_numa = agents .iter()
			.find(|agent| agent.description().identity.uuid.as_str() == selected_uuid)
			.map(|agent| agent.description().identity.numa_node_id)
			.ok_or_else(|| ProbeError::Benchmark("selected HSA GPU vanished from owned inventory".to_owned()))?;
		let (cpu, gpu) = select_agents(agents, selected_uuid, gpu_numa)?;
		let capacity_bytes = hsa_capacity(gpu.description())?; let target = exact_target(gpu.description())?.clone();
		let session = gpu .into_session()
			.map_err(|error| ProbeError::Benchmark(format!("create HSA GPU session: {error}")))?;
		let bytes = plan_bytes(plan)?; if u64::try_from(bytes)
			.map_err(|_| ProbeError::Benchmark("HSA allocation size does not fit u64".to_owned()))?
			> capacity_bytes
		{ return Err(ProbeError::Benchmark(format!(
				"bounded HSA buffer {bytes} exceeds discovered available capacity {capacity_bytes}"
			))); }

		let host_source = cpu .allocate_fine(bytes)
			.map_err(|error| hsa_benchmark_error("allocate host source", error))?;
		let host_destination = cpu .allocate_fine(bytes)
			.map_err(|error| hsa_benchmark_error("allocate host destination", error))?;
		let device_source = session .allocate_coarse(bytes)
			.map_err(|error| hsa_benchmark_error("allocate GPU source", error))?;
		let device_destination = session .allocate_coarse(bytes)
			.map_err(|error| hsa_benchmark_error("allocate GPU destination", error))?;
		session .grant_access(&host_source)
			.map_err(|error| hsa_benchmark_error("grant GPU host-source access", error))?;
		session .grant_access(&host_destination)
			.map_err(|error| hsa_benchmark_error("grant GPU host-destination access", error))?;
		cpu.grant_access(&device_source)
			.map_err(|error| hsa_benchmark_error("grant CPU GPU-source access", error))?;
		cpu.grant_access(&device_destination)
			.map_err(|error| hsa_benchmark_error("grant CPU GPU-destination access", error))?;

		let mut source_bytes = vec![0_u8; bytes]; fill_input(&mut source_bytes);
		// SAFETY: `allocate_fine` selected a host-accessible coherent CPU pool,
		// and no device operation has been published for this allocation.
		unsafe { host_source .copy_from_host(0, &source_bytes)
				.map_err(|error| hsa_benchmark_error("initialize host source", error))?;
		}

		let h2d = time_bounded(plan, |remaining| { let mut pending = session
				.copy_async(&device_source, 0, &host_source, 0, bytes)
				.map_err(|error| hsa_benchmark_error("submit HSA host-to-device copy", error))?;
			complete_hsa(&mut pending, remaining, "HSA host-to-device copy")
		})?; let d2h = time_bounded(plan, |remaining| { let mut pending = session
				.copy_async(&host_destination, 0, &device_source, 0, bytes)
				.map_err(|error| hsa_benchmark_error("submit HSA device-to-host copy", error))?;
			complete_hsa(&mut pending, remaining, "HSA device-to-host copy")
		})?; let mut observed = vec![0_u8; bytes]; copy_hsa_to_host(&host_destination, &mut observed)?;
		if observed != source_bytes { return Err(ProbeError::Benchmark(
				"HSA host/device transfer verification failed".to_owned(),
			)); }
		let d2d = time_bounded(plan, |remaining| { let mut pending = session
				.copy_async(&device_destination, 0, &device_source, 0, bytes)
				.map_err(|error| hsa_benchmark_error("submit HSA device-memory copy", error))?;
			complete_hsa(&mut pending, remaining, "HSA device-to-device copy")
		})?; copy_hsa( &session, &host_destination, &device_destination, bytes, plan.maximum_duration,
			"HSA verification download",
		)?; copy_hsa_to_host(&host_destination, &mut observed)?; if observed != source_bytes {
			return Err(ProbeError::Benchmark(
				"HSA device-memory transfer verification failed".to_owned(),
			)); }

		let calculation = self.benchmark_calculation( &cpu, &session, &target, &host_destination, &device_source,
			&device_destination, &source_bytes, &mut observed, plan, )?; let bytes_u64 = u64::try_from(bytes)
			.map_err(|_| ProbeError::Benchmark("HSA transfer size does not fit u64".to_owned()))?;
		Ok(GpuMeasurement { capacity: measured(capacity(capacity_bytes)?), calculation_rate: measured(calculation),
			memory_rate: measured(transfer_rate(bytes_u64, d2d)?), host_to_device_rate: measured(transfer_rate(bytes_u64, h2d)?),
			device_to_host_rate: measured(transfer_rate(bytes_u64, d2h)?), }) }

	#[allow(clippy::too_many_arguments)]
	fn benchmark_calculation( &self,
		cpu: &DiscoveredAgent<'_>,
		session: &Session<'_>,
		isa_target: &IsaTarget,
		host_output: &recipe_hsa::Allocation<'_>,
		input: &recipe_hsa::Allocation<'_>,
		output: &recipe_hsa::Allocation<'_>,
		input_bytes: &[u8], output_bytes: &mut [u8], plan: BoundedBenchmarkPlan,
	) -> ProbeResult<recipe_core::FlopsPerSecond> { let bytes = compute_buffer_bytes(plan)?;
		let elements = u32::try_from(bytes / 4)
			.map_err(|_| ProbeError::Benchmark("HSA element count does not fit u32".to_owned()))?;
		let lanes = hsa_workgroup_lanes(session.description(), elements)?; let target = KernelTarget::Amd(AmdTarget {
			target_id: hsa_target_tail(isa_target)?.to_owned(), code_object_version: self.code_object_version, });
		let template = fma_template(bytes, self.kernels.fma_chain_length)?;
		let lowered = lower_elementwise(&template, &target, &LoweringOptions { entry_symbol: HSA_KERNEL_ENTRY.to_owned(),
			workgroup_lanes: u32::from(lanes), }) .map_err(kernel_benchmark_error)?;
		let builder = ArtifactBuilder::new(self.kernels.toolchain.clone()).map_err(kernel_benchmark_error)?;
		let artifact = builder .build( BuildPhase::Realize, &lowered, &target, &self.kernels.scratch_parent, )
			.map_err(kernel_benchmark_error)?; let BuiltArtifact::Hsaco { bytes: hsaco, inspection, .. } = artifact
		else { return Err(ProbeError::Benchmark(
				"HSA benchmark builder returned a non-HSACO artifact".to_owned(),
			)); }; let executable = session .load_hsaco(&hsaco)
			.map_err(|error| hsa_benchmark_error("load Recipe HSACO", error))?;
		let kernel = executable .kernel(&inspection.kernel.symbol)
			.map_err(|error| hsa_benchmark_error("find Recipe HSA kernel", error))?;
		if u64::from(kernel.metadata().kernarg_segment_size) != inspection.kernel.kernarg_segment_size {
			return Err(ProbeError::Benchmark(
				"ROCr kernarg size disagrees with inspected HSACO metadata".to_owned(),
			)); }
		let kernarg_size = usize::try_from(inspection.kernel.kernarg_segment_size)
			.map_err(|_| ProbeError::Benchmark("HSA kernarg size does not fit usize".to_owned()))?;
		let kernarg = cpu .allocate_kernarg(kernarg_size)
			.map_err(|error| hsa_benchmark_error("allocate HSA kernarg", error))?;
		session .grant_access(&kernarg)
			.map_err(|error| hsa_benchmark_error("grant GPU kernarg access", error))?;
		let mut arguments = vec![0_u8; kernarg_size]; write_u64(&mut arguments, 0, pointer_address(input)?)?;
		write_u64(&mut arguments, 8, pointer_address(output)?)?; write_u64(&mut arguments, 16, lowered.abi.elements.get())?;
		// SAFETY: `allocate_kernarg` selected a CPU host-accessible kernarg
		// pool and dispatch has not yet been published.
		unsafe { kernarg .copy_from_host(0, &arguments)
				.map_err(|error| hsa_benchmark_error("initialize HSA kernarg", error))?;
		}
		let queue_capability = session .description() .queue
			.ok_or_else(|| ProbeError::Benchmark("HSA GPU has no user-mode queue".to_owned()))?;
		let queue = session .create_queue(QueueConfig::new( queue_capability.minimum_packets, QueueKind::SingleProducer, ))
			.map_err(|error| hsa_benchmark_error("create HSA queue", error))?;
		let geometry = DispatchGeometry::one_dimensional(elements, lanes); let timed = time_bounded(plan, |remaining| {
			let mut pending = queue .dispatch(&kernel, Some(&kernarg), geometry)
				.map_err(|error| hsa_benchmark_error("submit Recipe HSA kernel", error))?;
			complete_hsa(&mut pending, remaining, "HSA Recipe-owned FLOP kernel")
		})?; copy_hsa( session, host_output, output, bytes, plan.maximum_duration,
			"HSA FLOP verification download",
		)?; copy_hsa_to_host(host_output, &mut output_bytes[..bytes])?;
		verify_compute_output(&input_bytes[..bytes], &output_bytes[..bytes])?; queue.close()
			.map_err(|error| hsa_benchmark_error("close HSA queue", error))?;
		drop(kernel); executable .close()
			.map_err(|error| hsa_benchmark_error("close HSA executable", error))?;
		kernarg .close()
			.map_err(|error| hsa_benchmark_error("close HSA kernarg", error))?;
		calculation_rate(lowered.work, timed) } }

fn driver_lds_capacity(node: u32) -> ProbeResult<recipe_core::ByteCount> {
	let path = PathBuf::from("/sys/class/kfd/kfd/topology/nodes")
		.join(node.to_string())
		.join("properties");
	let properties =
		fs::read_to_string(&path).map_err(|error| ProbeError::io("read KFD GPU properties", &path, error))?;
	let kilobytes = properties.lines().find_map(|line| { let mut fields = line.split_whitespace();
		match (fields.next(), fields.next(), fields.next()) {
			(Some("lds_size_in_kb"), Some(value), None) => Some(value),
			_ => None, } }); let kilobytes = kilobytes.ok_or_else(|| { ProbeError::Discovery(format!(
			"KFD properties at {} omit lds_size_in_kb",
			path.display() )) })?; let kilobytes = kilobytes.parse::<u64>().map_err(|error| { ProbeError::Discovery(format!(
			"KFD lds_size_in_kb at {} is not an integer: {error}",
			path.display() )) })?; let bytes = kilobytes .checked_mul(1024) .filter(|bytes| *bytes != 0) .ok_or_else(|| {
			ProbeError::Discovery(format!(
				"KFD LDS capacity at {} is zero or overflowed",
				path.display() )) })?; Ok(recipe_core::ByteCount::new(bytes)) }

impl Backend for HsaBackend { fn discover(&self) -> ProbeResult<Vec<GpuDescriptor>> { Ok(self
			.with_runtime(|library, runtime| { let discovery = runtime .discover()
					.map_err(|error| ProbeError::Discovery(format!("exhaustive HSA discovery: {error}")))?;
				let mut devices = Vec::new(); for agent in discovery.agents() { if let Some(descriptor) =
						self.descriptor(library, discovery.system(), agent.description())?
					{ devices.push(descriptor); } }
				Ok(devices) })? .unwrap_or_default()) }

	fn benchmark(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> { self.benchmark_device(device, plan) }
}

pub(crate) fn exact_target(description: &AgentDescription) -> ProbeResult<&IsaTarget> { let targets = description .isas
		.iter() .map(|isa| { isa.amd_target.as_ref().ok_or_else(|| { ProbeError::Discovery(format!(
					"HSA GPU {} exposed a non-AMDGPU ISA",
					description.identity.uuid )) }) }) .collect::<ProbeResult<Vec<_>>>()?; let identities = targets .iter()
		.map(|target| target.as_str()) .collect::<BTreeSet<_>>(); let specific = targets .iter() .copied()
		.filter(|target| !target.architecture().ends_with("-generic"))
		.collect::<Vec<_>>(); if specific.len() == 1 { return Ok(specific[0]); }
	if specific.is_empty() && identities.len() == 1 { return targets.into_iter().next().ok_or_else(|| {
			ProbeError::Discovery(format!(
				"HSA GPU {} exposed no artifact target",
				description.identity.uuid )) }); }
	if identities.is_empty() { return Err(ProbeError::Discovery(format!(
			"HSA GPU {} exposed no artifact target",
			description.identity.uuid ))); }
	if specific.len() > 1 { return Err(ProbeError::Discovery(format!(
			"HSA GPU {} exposed {} distinct non-generic artifact targets",
			description.identity.uuid, specific.len() ))); }
	Err(ProbeError::Discovery(format!(
		"HSA GPU {} exposed ambiguous generic artifact targets",
		description.identity.uuid ))) }

fn hsa_capacity(description: &AgentDescription) -> ProbeResult<u64> { let local_pool_bytes = description .memory_pools
		.iter() .filter(|pool| { pool.segment == MemorySegment::Global && pool.location == Some(MemoryLocation::Gpu)
				&& pool.runtime_allocation.is_some()
				&& pool .global_flags .is_some_and(|flags| flags.contains(MemoryPoolFlags::COARSE_GRAINED)) })
		.map(|pool| pool.size_bytes) .max(); match local_pool_bytes { Some(bytes) => { u64::try_from(bytes)
				.map_err(|_| ProbeError::Discovery("HSA local-memory pool size does not fit u64".to_owned()))
		}
		None => { let available = description .amd_gpu .as_ref() .ok_or_else(|| { ProbeError::Discovery(format!(
						"HSA GPU {} omitted AMD capacity properties",
						description.identity.uuid )) })? .available_memory_bytes; if available == 0 { Err(ProbeError::Discovery(format!(
					"HSA GPU {} exposed neither a local pool capacity nor available memory",
					description.identity.uuid ))) } else { Ok(available) } } } }

fn hsa_target_tail(target: &IsaTarget) -> ProbeResult<&str> { target.as_str()
		.strip_prefix("amdgcn-amd-amdhsa--")
		.ok_or_else(|| { ProbeError::Discovery(format!(
				"HSA target {} has no AMDGPU triple prefix",
				target.as_str() )) }) }

fn select_agents<'runtime>(
	agents: Vec<DiscoveredAgent<'runtime>>,
	gpu_uuid: &str, gpu_numa: u32,
) -> ProbeResult<(DiscoveredAgent<'runtime>, DiscoveredAgent<'runtime>)> {
	let mut gpu = None; let mut matching_cpu = None; let mut fallback_cpu = None; for agent in agents {
		let description = agent.description(); if description.identity.uuid.as_str() == gpu_uuid { if gpu.is_some() {
				return Err(ProbeError::Benchmark(
					"selected HSA GPU UUID is not unique".to_owned(),
				)); }
			gpu = Some(agent); } else if description.device_type == DeviceType::Cpu {
			if description.identity.numa_node_id == gpu_numa && matching_cpu.is_none() { matching_cpu = Some(agent);
			} else if fallback_cpu.is_none() { fallback_cpu = Some(agent); } } }
	let gpu = gpu.ok_or_else(|| ProbeError::Benchmark("selected HSA GPU is missing".to_owned()))?;
	let cpu = matching_cpu .or(fallback_cpu)
		.ok_or_else(|| ProbeError::Benchmark("ROCr exposed no CPU memory agent for GPU transfers".to_owned()))?;
	Ok((cpu, gpu)) }

fn hsa_workgroup_lanes(description: &AgentDescription, elements: u32) -> ProbeResult<u16> { let maximum = description
		.isas .iter() .map(|isa| { isa.maximum_workgroup_size .min(u32::from(isa.maximum_workgroup_dimensions[0])) }) .min()
		.ok_or_else(|| ProbeError::Benchmark("HSA GPU has no ISA limits".to_owned()))?
		.min(elements); if maximum == 0 { return Err(ProbeError::Benchmark(
			"HSA workgroup limit is zero".to_owned(),
		)); }
	let lanes = 1_u32 << (31 - maximum.leading_zeros());
	u16::try_from(lanes).map_err(|_| ProbeError::Benchmark("HSA workgroup lanes do not fit u16".to_owned()))
}

fn copy_hsa(
	session: &Session<'_>,
	destination: &recipe_hsa::Allocation<'_>,
	source: &recipe_hsa::Allocation<'_>,
	bytes: usize, timeout: Duration,
	operation: &'static str,
) -> ProbeResult<()> { let mut pending = session .copy_async(destination, 0, source, 0, bytes)
		.map_err(|error| hsa_benchmark_error("submit HSA verification copy", error))?;
	complete_hsa(&mut pending, timeout, operation) }

fn copy_hsa_to_host(allocation: &recipe_hsa::Allocation<'_>, destination: &mut [u8]) -> ProbeResult<()> {
	// SAFETY: callers pass an allocation produced by the CPU fine-grained pool
	// after the system-scope producer has reached terminal completion.
	unsafe { allocation .copy_to_host(0, destination)
			.map_err(|error| hsa_benchmark_error("read HSA host allocation", error))
	} }

fn complete_hsa(pending: &mut Pending<'_, '_>, timeout: Duration, operation: &'static str) -> ProbeResult<()> {
	let start = Instant::now(); let mut backoff = CompletionPollBackoff::benchmark(); loop { match pending .poll()
			.map_err(|error| hsa_benchmark_error(operation, error))?
		{ PollStatus::Complete => return Ok(()), PollStatus::Pending => {} }
		let elapsed = start.elapsed(); if elapsed >= timeout { break; }
		backoff.wait(timeout.saturating_sub(elapsed)); }

	// A live HSA token retains the signal, allocations, and executable needed
	// by its already-submitted operation. Await that operation at a low capped
	// query rate so error unwinding cannot release live resources and the
	// display driver is never hammered by a timeout cleanup loop.
	let mut cleanup_backoff = CompletionPollBackoff::timed_out_cleanup(); loop {
		cleanup_backoff.wait(TIMED_OUT_CLEANUP_MAXIMUM_DELAY); match pending .poll()
			.map_err(|error| hsa_benchmark_error(operation, error))?
		{ PollStatus::Complete => { return Err(ProbeError::Benchmark(format!(
					"{operation} exceeded its {timeout:?} deadline"
				))); }
			PollStatus::Pending => {} } } }

fn pointer_address(allocation: &recipe_hsa::Allocation<'_>) -> ProbeResult<u64> {
	u64::try_from(allocation.as_ptr() as usize)
		.map_err(|_| ProbeError::Benchmark("HSA pointer does not fit u64".to_owned()))
}

fn write_u64(destination: &mut [u8], offset: usize, value: u64) -> ProbeResult<()> { let end = offset .checked_add(8)
		.ok_or_else(|| ProbeError::Benchmark("HSA kernarg offset overflowed".to_owned()))?;
	let slot = destination .get_mut(offset..end)
		.ok_or_else(|| ProbeError::Benchmark("HSA kernarg is smaller than explicit Recipe ABI".to_owned()))?;
	slot.copy_from_slice(&value.to_ne_bytes()); Ok(()) }

fn hsa_benchmark_error(operation: &'static str, error: recipe_hsa::Error) -> ProbeError {
	ProbeError::Benchmark(format!("{operation}: {error}"))
}

fn kernel_benchmark_error(error: recipe_kernel::LoweringError) -> ProbeError {
	ProbeError::Benchmark(format!("Recipe-owned GPU artifact: {error}"))
}

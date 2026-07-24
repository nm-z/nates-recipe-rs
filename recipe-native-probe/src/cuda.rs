use core::ffi::c_void;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use recipe_core::{Label, TargetIdentity, TransferLaneCount, TransportKind};
use recipe_cuda::{
	CompletionStatus, Context, ContextFlags, DeviceBuffer, DeviceInfo, Dim3, Discovery, Driver, Event, LaunchConfig,
	Module, Pending, PinnedHostBuffer, SchedulingPolicy, Stream,
};
use recipe_kernel::{
	ArtifactBuilder, BuildPhase, BuiltArtifact, KernelTarget, LoweringOptions, NvidiaTarget, lower_elementwise,
};
use recipe_native_executor::CUDA_MAXIMUM_SUBMISSION_QUEUES;
use recipe_probe::{BoundedBenchmarkPlan, GpuDescriptor, GpuMeasurement, LinkDuplex, ProbeError, ProbeResult};

use crate::benchmark::{
	calculation_rate, capacity, compute_buffer_bytes, fill_input, fma_template, measured, plan_bytes, time_bounded,
	transfer_rate, verify_compute_output,
};
use crate::config::{BackendLibrary, KernelBuildConfig, NativeProbeConfig};
use crate::identity::{
	PinnedLibrary, backend_toolchain_identity, label, library_identity, pci_accelerator_present, pci_surface,
	selected_library,
};
use crate::native::Backend;

const CUDA_KERNEL_ENTRY: &str = "recipe_probe_fma_f32";
const CUDA_WORKGROUP_LANES: u32 = 256;
const COMPLETION_POLL_INITIAL_DELAY: Duration = Duration::from_micros(50);
const COMPLETION_POLL_MAXIMUM_DELAY: Duration = Duration::from_millis(2);
const TIMED_OUT_CLEANUP_INITIAL_DELAY: Duration = Duration::from_millis(10);
const TIMED_OUT_CLEANUP_MAXIMUM_DELAY: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug)]
struct CompletionPollBackoff {
	next_delay: Duration,
	maximum_delay: Duration,
}

impl CompletionPollBackoff {
	const fn benchmark() -> Self {
		Self {
			next_delay: COMPLETION_POLL_INITIAL_DELAY,
			maximum_delay: COMPLETION_POLL_MAXIMUM_DELAY,
		}
	}

	const fn timed_out_cleanup() -> Self {
		Self {
			next_delay: TIMED_OUT_CLEANUP_INITIAL_DELAY,
			maximum_delay: TIMED_OUT_CLEANUP_MAXIMUM_DELAY,
		}
	}

	fn wait(&mut self, maximum: Duration) {
		std::thread::sleep(self.next_delay.min(maximum));
		self.advance();
	}

	fn advance(&mut self) {
		self.next_delay = self.next_delay.saturating_mul(2).min(self.maximum_delay);
	}
}

#[derive(Clone, Debug)]
pub(crate) struct CudaBackend {
	library: BackendLibrary,
	host_memory_key: Label,
	pci_sysfs_root: PathBuf,
	ptx_isa: u16,
	kernels: KernelBuildConfig,
}

impl CudaBackend {
	pub(crate) fn new(config: &NativeProbeConfig) -> ProbeResult<Self> {
		Ok(Self {
			library: config.cuda.library.clone(),
			host_memory_key: config.host_memory_key.clone(),
			pci_sysfs_root: config.pci_sysfs_root.clone(),
			ptx_isa: config.cuda.ptx_isa,
			kernels: config.kernels.clone(),
		})
	}

	pub(crate) fn open(&self) -> ProbeResult<Option<(PinnedLibrary, Driver, Discovery)>> {
		if !pci_accelerator_present(&self.pci_sysfs_root, 0x10de)? {
			return Ok(None);
		}
		let Some(library) = selected_library(&self.library, "CUDA Driver")? else {
			return Err(ProbeError::Discovery(
				"NVIDIA PCI accelerator is present but no configured CUDA Driver library exists".to_owned(),
			));
		};
		let path = library.path.to_str().ok_or_else(|| {
			ProbeError::Discovery(format!(
				"CUDA Driver path {} is not valid UTF-8",
				library.path.display()
			))
		})?;
		let driver = Driver::load_from_path(path)
			.map_err(|error| ProbeError::Discovery(format!("load CUDA Driver: {error}")))?;
		let discovery = driver
			.discover()
			.map_err(|error| ProbeError::Discovery(format!("exhaustive CUDA discovery: {error}")))?;
		Ok(Some((library, driver, discovery)))
	}

	pub(crate) fn descriptor(
		&self,
		library: &PinnedLibrary,
		discovery: &Discovery,
		device: &DeviceInfo,
	) -> ProbeResult<GpuDescriptor> {
		let pci = parse_pci_bus_id(&device.pci_bus_id)?;
		if pci.domain != device.attributes.pci_domain_id
			|| pci.bus != device.attributes.pci_bus_id
			|| pci.device != device.attributes.pci_device_id
		{
			return Err(ProbeError::Discovery(format!(
				"CUDA PCI identity {} disagrees with Driver attributes {:04x}:{:02x}:{:02x}",
				device.pci_bus_id,
				device.attributes.pci_domain_id,
				device.attributes.pci_bus_id,
				device.attributes.pci_device_id
			)));
		}
		// The CUDA Driver may render hexadecimal bus components with uppercase
		// letters while Linux sysfs names PCI directories in lowercase. Keep
		// the numeric identity check above, then use one canonical spelling for
		// filesystem lookup and every retained identity digest.
		let sysfs_bdf = pci.sysfs_bdf();
		let surface = pci_surface(&self.pci_sysfs_root, &sysfs_bdf)?;
		let target = NvidiaTarget {
			sm_major: u8::try_from(device.compute_capability.major).map_err(|_| {
				ProbeError::Discovery(format!(
					"CUDA compute capability major {} does not fit u8",
					device.compute_capability.major
				))
			})?,
			sm_minor: u8::try_from(device.compute_capability.minor).map_err(|_| {
				ProbeError::Discovery(format!(
					"CUDA compute capability minor {} does not fit u8",
					device.compute_capability.minor
				))
			})?,
			ptx_isa: self.ptx_isa,
		};
		target.validate()
			.map_err(|error| ProbeError::Discovery(format!("CUDA artifact target: {error}")))?;
		let architecture = target.architecture();
		let toolchain = backend_toolchain_identity(
			&self.kernels.toolchain,
			&self.kernels.release,
			"nvidia-cuda",
			&format!(
				"{architecture}:ptx{}:dependent-f32-fma-chain-{}",
				self.ptx_isa, self.kernels.fma_chain_length
			),
		)?;
		let runtime = library_identity("cuda-driver-library", library);
		let key = format!("cuda:{}@{sysfs_bdf}", device.uuid);
		Ok(GpuDescriptor {
			key: label(key, "CUDA GPU key")?,
			name: label(device.name.clone(), "CUDA GPU name")?,
			host_memory_key: self.host_memory_key.clone(),
			target: TargetIdentity {
				backend: label("nvidia-cuda-driver", "CUDA target backend")?,
				architecture: label(architecture, "CUDA target architecture")?,
				abi: label("elf64-cubin", "CUDA target ABI")?,
			},
			capacity_hint: capacity(device.total_memory_bytes)?,
			driver: label(
				format!(
					"cuda-kernel-driver:{}:{}",
					discovery.driver_version.raw(),
					surface.driver
				),
				"CUDA driver identity",
			)?,
			runtime_abi: label(
				format!(
					"cuda-driver-api:{}:{runtime}",
					discovery.driver_version.raw()
				),
				"CUDA runtime identity",
			)?,
			firmware: label(surface.firmware, "CUDA firmware surface")?,
			link_identity: label(surface.link, "CUDA PCIe link identity")?,
			transport_kind: TransportKind::Pcie,
			toolchain,
			duplex: LinkDuplex::Full,
			host_to_device_maximum_inflight: TransferLaneCount::new(1)
				.map_err(|error| ProbeError::Discovery(format!("CUDA host-to-device lane count: {error}")))?,
			device_to_host_maximum_inflight: TransferLaneCount::new(1)
				.map_err(|error| ProbeError::Discovery(format!("CUDA device-to-host lane count: {error}")))?,
			asynchronous_submission: true,
			maximum_submission_queues: CUDA_MAXIMUM_SUBMISSION_QUEUES,
			maximum_concurrent_tasks: 1,
			transfer_overlaps_calculation: device.attributes.async_engine_count != 0
				&& device.attributes.concurrent_kernels,
		})
	}

	fn matching_device<'a>(
		&self,
		library: &PinnedLibrary,
		discovery: &'a Discovery,
		expected: &GpuDescriptor,
	) -> ProbeResult<&'a DeviceInfo> {
		let mut matched = None;
		for device in &discovery.devices {
			if self.descriptor(library, discovery, device)? == *expected {
				if matched.is_some() {
					return Err(ProbeError::Benchmark(format!(
						"CUDA descriptor {} is not unique",
						expected.key
					)));
				}
				matched = Some(device);
			}
		}
		matched.ok_or_else(|| {
			ProbeError::Benchmark(format!(
				"CUDA GPU {} disappeared or changed identity",
				expected.key
			))
		})
	}

	fn benchmark_open(&self, expected: &GpuDescriptor) -> ProbeResult<(Driver, Discovery, DeviceInfo)> {
		let Some((library, driver, discovery)) = self.open()? else {
			return Err(ProbeError::Benchmark(
				"CUDA backend disappeared before benchmark".to_owned(),
			));
		};
		let device = self
			.matching_device(&library, &discovery, expected)?
			.clone();
		Ok((driver, discovery, device))
	}

	fn benchmark_device(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> {
		let (driver, _discovery, info) = self.benchmark_open(device)?;
		let context = Context::create(&driver, &info, ContextFlags::new(SchedulingPolicy::Yield))
			.map_err(cuda_benchmark_error)?;
		let stream = Stream::create_nonblocking(&context).map_err(cuda_benchmark_error)?;
		let bytes = plan_bytes(plan)?;
		if u64::try_from(bytes)
			.map_err(|_| ProbeError::Benchmark("CUDA allocation size does not fit u64".to_owned()))?
			> info.total_memory_bytes
		{
			return Err(ProbeError::Benchmark(format!(
				"bounded CUDA buffer {bytes} exceeds discovered capacity {}",
				info.total_memory_bytes
			)));
		}
		let mut host_source = PinnedHostBuffer::allocate(&context, bytes).map_err(cuda_benchmark_error)?;
		let mut host_destination = PinnedHostBuffer::allocate(&context, bytes).map_err(cuda_benchmark_error)?;
		fill_input(host_source.as_mut_slice());
		let device_source = DeviceBuffer::allocate(&context, bytes).map_err(cuda_benchmark_error)?;
		let device_destination = DeviceBuffer::allocate(&context, bytes).map_err(cuda_benchmark_error)?;

		let h2d = time_bounded(plan, |remaining| {
			let event = Event::create_completion(&context).map_err(cuda_benchmark_error)?;
			// SAFETY: all ranges cover live same-context allocations; the
			// returned token is synchronously driven to terminal completion.
			let pending = unsafe { stream.copy_h2d(&device_source, 0, &host_source, 0, bytes, event) }
				.map_err(cuda_benchmark_error)?;
			complete_cuda(pending, remaining, "CUDA host-to-device copy")
		})?;

		let d2h = time_bounded(plan, |remaining| {
			let event = Event::create_completion(&context).map_err(cuda_benchmark_error)?;
			// SAFETY: all ranges cover live same-context allocations; the
			// returned token is synchronously driven to terminal completion.
			let pending = unsafe { stream.copy_d2h(&mut host_destination, 0, &device_source, 0, bytes, event) }
				.map_err(cuda_benchmark_error)?;
			complete_cuda(pending, remaining, "CUDA device-to-host copy")
		})?;
		if host_source.as_slice() != host_destination.as_slice() {
			return Err(ProbeError::Benchmark(
				"CUDA host/device transfer verification failed".to_owned(),
			));
		}

		let d2d = time_bounded(plan, |remaining| {
			let event = Event::create_completion(&context).map_err(cuda_benchmark_error)?;
			// SAFETY: all ranges cover live same-context allocations; the
			// returned token is synchronously driven to terminal completion.
			let pending = unsafe { stream.copy_d2d(&device_destination, 0, &device_source, 0, bytes, event) }
				.map_err(cuda_benchmark_error)?;
			complete_cuda(pending, remaining, "CUDA device-to-device copy")
		})?;
		copy_cuda_to_host(
			&stream,
			&context,
			&mut host_destination,
			&device_destination,
			bytes,
			plan.maximum_duration,
		)?;
		if host_source.as_slice() != host_destination.as_slice() {
			return Err(ProbeError::Benchmark(
				"CUDA device-memory transfer verification failed".to_owned(),
			));
		}

		let calculation = self.benchmark_calculation(
			&context,
			&stream,
			&device_source,
			&device_destination,
			&host_source,
			&mut host_destination,
			plan,
		)?;

		Ok(GpuMeasurement {
			capacity: measured(capacity(info.total_memory_bytes)?),
			calculation_rate: measured(calculation),
			memory_rate: measured(transfer_rate(
				u64::try_from(bytes)
					.map_err(|_| ProbeError::Benchmark("CUDA transfer size does not fit u64".to_owned()))?,
				d2d,
			)?),
			host_to_device_rate: measured(transfer_rate(
				u64::try_from(bytes)
					.map_err(|_| ProbeError::Benchmark("CUDA transfer size does not fit u64".to_owned()))?,
				h2d,
			)?),
			device_to_host_rate: measured(transfer_rate(
				u64::try_from(bytes)
					.map_err(|_| ProbeError::Benchmark("CUDA transfer size does not fit u64".to_owned()))?,
				d2h,
			)?),
		})
	}

	#[allow(clippy::too_many_arguments)]
	fn benchmark_calculation<'ctx>(
		&self,
		context: &'ctx Context,
		stream: &Stream<'ctx>,
		input: &DeviceBuffer<'ctx>,
		output: &DeviceBuffer<'ctx>,
		host_input: &PinnedHostBuffer<'ctx>,
		host_output: &mut PinnedHostBuffer<'ctx>,
		plan: BoundedBenchmarkPlan,
	) -> ProbeResult<recipe_core::FlopsPerSecond> {
		let bytes = compute_buffer_bytes(plan)?;
		let template = fma_template(bytes, self.kernels.fma_chain_length)?;
		let target = KernelTarget::Nvidia(NvidiaTarget {
			sm_major: u8::try_from(context.device().compute_capability.major)
				.map_err(|_| ProbeError::Benchmark("CUDA SM major does not fit u8".to_owned()))?,
			sm_minor: u8::try_from(context.device().compute_capability.minor)
				.map_err(|_| ProbeError::Benchmark("CUDA SM minor does not fit u8".to_owned()))?,
			ptx_isa: self.ptx_isa,
		});
		let lowered = lower_elementwise(
			&template,
			&target,
			&LoweringOptions {
				entry_symbol: CUDA_KERNEL_ENTRY.to_owned(),
				workgroup_lanes: CUDA_WORKGROUP_LANES,
			},
		)
		.map_err(kernel_benchmark_error)?;
		let builder = ArtifactBuilder::new(self.kernels.toolchain.clone()).map_err(kernel_benchmark_error)?;
		let artifact = builder
			.build(
				BuildPhase::Realize,
				&lowered,
				&target,
				&self.kernels.scratch_parent,
			)
			.map_err(kernel_benchmark_error)?;
		let BuiltArtifact::Cubin {
			bytes: cubin,
			inspection,
			..
		} = artifact
		else {
			return Err(ProbeError::Benchmark(
				"CUDA benchmark builder returned a non-cubin artifact".to_owned(),
			));
		};
		if inspection.entry_symbol != CUDA_KERNEL_ENTRY {
			return Err(ProbeError::Benchmark(
				"CUDA benchmark cubin entry identity changed".to_owned(),
			));
		}
		let module = Module::load_cubin(context, &cubin).map_err(cuda_benchmark_error)?;
		let function = module
			.function(CUDA_KERNEL_ENTRY)
			.map_err(cuda_benchmark_error)?;
		let elements = lowered.abi.elements.get();
		let blocks = elements
			.checked_add(u64::from(CUDA_WORKGROUP_LANES) - 1)
			.ok_or_else(|| ProbeError::Benchmark("CUDA grid calculation overflowed".to_owned()))?
			/ u64::from(CUDA_WORKGROUP_LANES);
		let grid_x =
			u32::try_from(blocks).map_err(|_| ProbeError::Benchmark("CUDA grid does not fit u32".to_owned()))?;
		let launch = LaunchConfig::new(
			Dim3::new(grid_x, 1, 1).map_err(cuda_benchmark_error)?,
			Dim3::new(CUDA_WORKGROUP_LANES, 1, 1).map_err(cuda_benchmark_error)?,
		);
		let timed = time_bounded(plan, |remaining| {
			let mut input_address = input.device_ptr().map_err(cuda_benchmark_error)?;
			let mut output_address = output.device_ptr().map_err(cuda_benchmark_error)?;
			let mut element_count = elements;
			let mut parameters = [
				(&raw mut input_address).cast::<c_void>(),
				(&raw mut output_address).cast::<c_void>(),
				(&raw mut element_count).cast::<c_void>(),
			];
			let keepalive = [input, output];
			let event = Event::create_completion(context).map_err(cuda_benchmark_error)?;
			// SAFETY: parameter storage exactly matches the inspected
			// one-input/one-output/i64 Recipe kernel ABI, and both referenced
			// allocations are retained through terminal completion.
			let pending = unsafe { stream.launch(&function, launch, &mut parameters, &keepalive, event) }
				.map_err(cuda_benchmark_error)?;
			complete_cuda(pending, remaining, "CUDA Recipe-owned FLOP kernel")
		})?;
		copy_cuda_to_host(
			stream,
			context,
			host_output,
			output,
			bytes,
			plan.maximum_duration,
		)?;
		verify_compute_output(
			&host_input.as_slice()[..bytes],
			&host_output.as_slice()[..bytes],
		)?;
		calculation_rate(lowered.work, timed)
	}
}

impl Backend for CudaBackend {
	fn discover(&self) -> ProbeResult<Vec<GpuDescriptor>> {
		let Some((library, _driver, discovery)) = self.open()? else {
			return Ok(Vec::new());
		};
		discovery
			.devices
			.iter()
			.map(|device| self.descriptor(&library, &discovery, device))
			.collect()
	}

	fn benchmark(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement> {
		self.benchmark_device(device, plan)
	}
}

fn copy_cuda_to_host<'op, 'ctx>(
	stream: &'op Stream<'ctx>,
	context: &'ctx Context,
	destination: &'op mut PinnedHostBuffer<'ctx>,
	source: &'op DeviceBuffer<'ctx>,
	bytes: usize,
	timeout: Duration,
) -> ProbeResult<()> {
	let event = Event::create_completion(context).map_err(cuda_benchmark_error)?;
	// SAFETY: all ranges cover live same-context allocations and the pending
	// token is driven to terminal completion before returning.
	let pending =
		unsafe { stream.copy_d2h(destination, 0, source, 0, bytes, event) }.map_err(cuda_benchmark_error)?;
	complete_cuda(pending, timeout, "CUDA verification download")
}

fn complete_cuda(mut pending: Pending<'_, '_>, timeout: Duration, operation: &'static str) -> ProbeResult<()> {
	let start = Instant::now();
	let mut backoff = CompletionPollBackoff::benchmark();
	loop {
		match pending.poll().map_err(cuda_benchmark_error)? {
			CompletionStatus::Complete => {
				drop(pending.recycle_event().map_err(cuda_benchmark_error)?);
				return Ok(());
			}
			CompletionStatus::Pending => {}
		}
		let elapsed = start.elapsed();
		if elapsed >= timeout {
			break;
		}
		backoff.wait(timeout.saturating_sub(elapsed));
	}

	// CUDA has no cancellation primitive, and this token only phantom-borrows
	// the live buffers used by the submission. Returning before completion
	// would permit those buffers to unwind while the GPU still uses them.
	// Continue awaiting this one operation with a large capped delay; never
	// submit replacement work or rapidly query the display driver's context.
	let mut cleanup_backoff = CompletionPollBackoff::timed_out_cleanup();
	loop {
		cleanup_backoff.wait(TIMED_OUT_CLEANUP_MAXIMUM_DELAY);
		match pending.poll().map_err(cuda_benchmark_error)? {
			CompletionStatus::Complete => {
				drop(pending.recycle_event().map_err(cuda_benchmark_error)?);
				return Err(ProbeError::Benchmark(format!(
					"{operation} exceeded its {timeout:?} deadline"
				)));
			}
			CompletionStatus::Pending => {}
		}
	}
}

fn cuda_benchmark_error(error: recipe_cuda::CudaError) -> ProbeError {
	ProbeError::Benchmark(format!("CUDA native operation: {error}"))
}

fn kernel_benchmark_error(error: recipe_kernel::LoweringError) -> ProbeError {
	ProbeError::Benchmark(format!("Recipe-owned GPU artifact: {error}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ParsedPci {
	domain: u32,
	bus: u32,
	device: u32,
	function: u32,
}

impl ParsedPci {
	fn sysfs_bdf(self) -> String {
		format!(
			"{:04x}:{:02x}:{:02x}.{}",
			self.domain, self.bus, self.device, self.function
		)
	}
}

fn parse_pci_bus_id(value: &str) -> ProbeResult<ParsedPci> {
	let (domain, tail) = value
		.split_once(':')
		.ok_or_else(|| ProbeError::Discovery(format!("CUDA PCI identity {value:?} has no domain")))?;
	let (bus, slot) = tail
		.split_once(':')
		.ok_or_else(|| ProbeError::Discovery(format!("CUDA PCI identity {value:?} has no bus")))?;
	let (device, function) = slot
		.split_once('.')
		.ok_or_else(|| ProbeError::Discovery(format!("CUDA PCI identity {value:?} has no function")))?;
	let parse_hex = |component: &str, field: &str| {
		u32::from_str_radix(component, 16).map_err(|error| {
			ProbeError::Discovery(format!(
				"CUDA PCI {field} {component:?} is invalid: {error}"
			))
		})
	};
	let function = function.parse::<u32>().map_err(|error| {
		ProbeError::Discovery(format!(
			"CUDA PCI function {function:?} is invalid: {error}"
		))
	})?;
	if function > 7 {
		return Err(ProbeError::Discovery(format!(
			"CUDA PCI function {function} is outside 0..=7"
		)));
	}
	Ok(ParsedPci {
		domain: parse_hex(domain, "domain")?,
		bus: parse_hex(bus, "bus")?,
		device: parse_hex(device, "device")?,
		function,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parses_exact_cuda_pci_function() {
		let parsed = parse_pci_bus_id("0000:0B:00.1").expect("valid PCI identity");
		assert_eq!(
			parsed,
			ParsedPci {
				domain: 0,
				bus: 11,
				device: 0,
				function: 1,
			}
		);
		assert_eq!(parsed.sysfs_bdf(), "0000:0b:00.1");
	}

	#[test]
	fn rejects_approximate_cuda_pci_function() {
		assert!(parse_pci_bus_id("04:00").is_err());
		assert!(parse_pci_bus_id("0000:04:00.8").is_err());
		assert!(parse_pci_bus_id("0000:gg:00.0").is_err());
	}

	#[test]
	fn timed_out_completion_polling_reaches_a_large_fixed_cap() {
		let mut backoff = CompletionPollBackoff::timed_out_cleanup();
		assert_eq!(backoff.next_delay, Duration::from_millis(10));
		for _ in 0..8 {
			backoff.advance();
		}
		assert_eq!(backoff.next_delay, Duration::from_millis(100));
		backoff.advance();
		assert_eq!(backoff.next_delay, Duration::from_millis(100));
	}
}

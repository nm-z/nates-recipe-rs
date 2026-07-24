use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::time::Instant;

use recipe_core::{
	ByteCount, BytesPerSecond, DeviceId, Digest, DiscoveryProfile, FlopsPerSecond, Label, MachineId, Property,
	TargetIdentity, ToolchainIdentity, Topology, TransferLaneCount, TransportKind,
};

use crate::error::ProbeResult;

pub const PROFILE_SCHEMA: u32 = 6;
pub const PROFILE_CODEC_SCHEMA: u32 = 6;
pub const PEER_BENCHMARK_PROTOCOL_SCHEMA: u16 = 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineFingerprint {
	pub hostname: Label,
	pub stable_id: Label,
	pub runtime_abi: Label,
	pub firmware: Label,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RamDomain {
	pub key: Label,
	pub capacity_hint: ByteCount,
	pub link_identity: Label,
	pub maximum_inflight_transfers: TransferLaneCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkDuplex {
	Full,
	Half,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageDomain {
	pub key: Label,
	pub name: Label,
	pub benchmark_root: PathBuf,
	pub capacity_hint: ByteCount,
	pub host_memory_key: Label,
	pub driver: Label,
	pub firmware: Label,
	pub link_identity: Label,
	pub transport_kind: TransportKind,
	pub duplex: LinkDuplex,
	pub maximum_concurrent_reads: TransferLaneCount,
	pub maximum_concurrent_writes: TransferLaneCount,
	pub asynchronous_submission: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkInterface {
	pub key: Label,
	pub name: Label,
	pub address: Label,
	pub driver: Label,
	pub firmware: Label,
	pub link_identity: Label,
	pub transport_kind: TransportKind,
	pub duplex: LinkDuplex,
	pub asynchronous_submission: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostInventory {
	pub machine: MachineFingerprint,
	pub ram: Vec<RamDomain>,
	pub storage: Vec<StorageDomain>,
	pub network: Vec<NetworkInterface>,
}

/// Implementations must enumerate every locally usable RAM, mounted storage,
/// and network domain or return an error rather than a partial inventory.
pub trait HostDiscovery: core::fmt::Debug {
	fn discover_host(&self) -> ProbeResult<HostInventory>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuDescriptor {
	pub key: Label,
	pub name: Label,
	pub host_memory_key: Label,
	pub target: TargetIdentity,
	pub capacity_hint: ByteCount,
	pub driver: Label,
	pub runtime_abi: Label,
	pub firmware: Label,
	pub link_identity: Label,
	pub transport_kind: TransportKind,
	pub toolchain: ToolchainIdentity,
	pub duplex: LinkDuplex,
	pub host_to_device_maximum_inflight: TransferLaneCount,
	pub device_to_host_maximum_inflight: TransferLaneCount,
	pub asynchronous_submission: bool,
	pub maximum_submission_queues: u32,
	pub maximum_concurrent_tasks: u32,
	pub transfer_overlaps_calculation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuInventory {
	pub exhaustive: bool,
	pub devices: Vec<GpuDescriptor>,
}

/// Injected by the native calculation backend. Enumeration must cover every
/// device visible to that backend.
pub trait GpuDiscovery: core::fmt::Debug {
	fn discover_all(&self) -> ProbeResult<GpuInventory>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundedBenchmarkPlan {
	pub buffer_bytes: ByteCount,
	pub iterations: u32,
	pub maximum_duration: Duration,
}

impl BoundedBenchmarkPlan {
	#[must_use]
	pub fn is_bounded(self) -> bool {
		self.buffer_bytes.get() != 0 && self.iterations != 0 && !self.maximum_duration.is_zero()
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RamMeasurement {
	pub capacity: Property<ByteCount>,
	pub transfer_rate: Property<BytesPerSecond>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageMeasurement {
	pub capacity: Property<ByteCount>,
	pub read_rate: Property<BytesPerSecond>,
	pub write_rate: Property<BytesPerSecond>,
}

pub trait HostBenchmarkIo: core::fmt::Debug {
	fn benchmark_ram(&self, domain: &RamDomain, plan: BoundedBenchmarkPlan) -> ProbeResult<RamMeasurement>;

	fn benchmark_storage(
		&self,
		domain: &StorageDomain,
		plan: BoundedBenchmarkPlan,
	) -> ProbeResult<StorageMeasurement>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuMeasurement {
	pub capacity: Property<ByteCount>,
	pub calculation_rate: Property<FlopsPerSecond>,
	pub memory_rate: Property<BytesPerSecond>,
	pub host_to_device_rate: Property<BytesPerSecond>,
	pub device_to_host_rate: Property<BytesPerSecond>,
}

/// Execution hooks remain abstract until owned native benchmark kernels land.
pub trait GpuBenchmarkIo: core::fmt::Debug {
	fn benchmark_gpu(&self, device: &GpuDescriptor, plan: BoundedBenchmarkPlan) -> ProbeResult<GpuMeasurement>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerDescriptor {
	pub session_id: Label,
	pub machine: MachineFingerprint,
	pub remote_memory_key: Label,
	pub local_memory_key: Label,
	pub local_interface_key: Label,
	pub remote_interface_identity: Label,
	pub remote_driver: Label,
	pub remote_firmware: Label,
	pub link_identity: Label,
	pub transport_kind: TransportKind,
	pub duplex: LinkDuplex,
	pub outbound_maximum_inflight: TransferLaneCount,
	pub inbound_maximum_inflight: TransferLaneCount,
	pub asynchronous_submission: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerMeasurement {
	pub remote_memory_capacity: Property<ByteCount>,
	pub remote_memory_rate: Property<BytesPerSecond>,
	pub outbound_rate: Option<Property<BytesPerSecond>>,
	pub inbound_rate: Option<Property<BytesPerSecond>>,
	pub evidence: PeerBenchmarkEvidence,
}

/// Cloneable caller-owned cancellation state for one bounded peer benchmark.
///
/// Cancellation is advisory between framed operations. The absolute deadline
/// remains the hard upper bound for any in-progress transport operation.
#[derive(Clone, Debug, Default)]
pub struct PeerBenchmarkCancellation {
	cancelled: Arc<AtomicBool>,
}

impl PeerBenchmarkCancellation {
	pub fn cancel(&self) {
		self.cancelled.store(true, Ordering::Release);
	}

	#[must_use]
	pub fn is_cancelled(&self) -> bool {
		self.cancelled.load(Ordering::Acquire)
	}
}

/// Caller-provided control plane for a peer benchmark attempt.
#[derive(Clone, Debug)]
pub struct PeerBenchmarkControl {
	absolute_deadline: Instant,
	cancellation: PeerBenchmarkCancellation,
}

impl PeerBenchmarkControl {
	#[must_use]
	pub const fn new(absolute_deadline: Instant, cancellation: PeerBenchmarkCancellation) -> Self {
		Self {
			absolute_deadline,
			cancellation,
		}
	}

	/// Create a fresh control whose absolute deadline is the plan's duration
	/// from now.
	///
	/// # Errors
	///
	/// Fails when the duration cannot be represented by [`Instant`].
	pub fn for_plan(plan: BoundedBenchmarkPlan) -> ProbeResult<Self> {
		let absolute_deadline = Instant::now()
			.checked_add(plan.maximum_duration)
			.ok_or_else(|| crate::ProbeError::Benchmark("peer benchmark deadline overflowed".to_owned()))?;
		Ok(Self::new(
			absolute_deadline,
			PeerBenchmarkCancellation::default(),
		))
	}

	#[must_use]
	pub const fn absolute_deadline(&self) -> Instant {
		self.absolute_deadline
	}

	#[must_use]
	pub const fn cancellation(&self) -> &PeerBenchmarkCancellation {
		&self.cancellation
	}

	#[must_use]
	pub fn failure(&self, phase: PeerBenchmarkPhase) -> Option<PeerBenchmarkFailure> {
		if self.cancellation.is_cancelled() {
			Some(PeerBenchmarkFailure::new(
				phase,
				PeerBenchmarkFailureKind::Cancelled,
				"peer benchmark was cancelled",
			))
		} else if Instant::now() >= self.absolute_deadline {
			Some(PeerBenchmarkFailure::new(
				phase,
				PeerBenchmarkFailureKind::Deadline,
				"peer benchmark absolute deadline elapsed",
			))
		} else {
			None
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerBenchmarkPhase {
	Validation,
	BeginExchange,
	ReadyExchange,
	DirectionalTransfer,
	CompletionExchange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerBenchmarkFailureKind {
	Cancelled,
	Deadline,
	Identity,
	Integrity,
	Protocol,
	Transport,
}

/// Structured terminal evidence for an unsuccessful benchmark attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerBenchmarkFailure {
	pub protocol_schema: u16,
	pub phase: PeerBenchmarkPhase,
	pub kind: PeerBenchmarkFailureKind,
	pub detail: String,
}

impl PeerBenchmarkFailure {
	#[must_use]
	pub fn new(phase: PeerBenchmarkPhase, kind: PeerBenchmarkFailureKind, detail: impl Into<String>) -> Self {
		Self {
			protocol_schema: PEER_BENCHMARK_PROTOCOL_SCHEMA,
			phase,
			kind,
			detail: detail.into(),
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerBenchmarkAttempt {
	Measured(PeerMeasurement),
	Failed(PeerBenchmarkFailure),
}

impl PeerBenchmarkAttempt {
	/// Require a successful measurement for profile construction.
	///
	/// # Errors
	///
	/// Converts structured failure evidence to a probe error without allowing a
	/// failed attempt to masquerade as a measured property.
	pub fn into_measurement(self) -> ProbeResult<PeerMeasurement> {
		match self {
			Self::Measured(measurement) => Ok(measurement),
			Self::Failed(failure) => Err(crate::ProbeError::Benchmark(format!(
				"peer benchmark failed during {:?} ({:?}): {}",
				failure.phase, failure.kind, failure.detail
			))),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerDuplexExecution {
	Simultaneous,
	Serialized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerEndpointEvidence {
	pub machine: Digest,
	pub profile: Digest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionalBenchmarkEvidence {
	pub total_bytes: ByteCount,
	pub elapsed_nanoseconds: u64,
	pub sample_count: u32,
	pub minimum_sample_nanoseconds: u64,
	pub maximum_sample_nanoseconds: u64,
	pub mean_sample_nanoseconds: u64,
	pub variance_nanoseconds_squared: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerBenchmarkEvidence {
	pub protocol_schema: u16,
	pub local_endpoint: PeerEndpointEvidence,
	pub remote_endpoint: PeerEndpointEvidence,
	pub execution: PeerDuplexExecution,
	pub outbound: DirectionalBenchmarkEvidence,
	pub inbound: DirectionalBenchmarkEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredPeerBenchmark {
	pub session_id: Label,
	pub outbound_rate: Property<BytesPerSecond>,
	pub inbound_rate: Property<BytesPerSecond>,
	pub evidence: PeerBenchmarkEvidence,
}

/// An explicit, established peer session is the only source of measured
/// network throughput.
pub trait PeerSession: core::fmt::Debug {
	fn descriptor(&self) -> ProbeResult<PeerDescriptor>;
	fn benchmark(&self, plan: BoundedBenchmarkPlan) -> ProbeResult<PeerMeasurement>;

	/// Run one explicitly controlled attempt and retain structured failure
	/// phase evidence.
	///
	/// Implementations with blocking I/O should override this method so the
	/// supplied absolute deadline is applied to every operation.
	fn benchmark_controlled(
		&self,
		plan: BoundedBenchmarkPlan,
		control: &PeerBenchmarkControl,
	) -> PeerBenchmarkAttempt {
		if let Some(failure) = control.failure(PeerBenchmarkPhase::Validation) {
			return PeerBenchmarkAttempt::Failed(failure);
		}
		let result = self.benchmark(plan);
		if let Some(failure) = control.failure(PeerBenchmarkPhase::CompletionExchange) {
			return PeerBenchmarkAttempt::Failed(failure);
		}
		match result {
			Ok(measurement) => PeerBenchmarkAttempt::Measured(measurement),
			Err(error) => PeerBenchmarkAttempt::Failed(PeerBenchmarkFailure::new(
				PeerBenchmarkPhase::DirectionalTransfer,
				PeerBenchmarkFailureKind::Transport,
				error.to_string(),
			)),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CacheIdentity {
	pub schema: u32,
	pub digest: Digest,
}

/// Stable discovered identity retained for one topology machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredMachineOrigin {
	pub machine: MachineId,
	pub fingerprint: MachineFingerprint,
}

/// Stable discovery key retained for one topology RAM device.
///
/// The key is scoped by the device's machine and comes from
/// [`RamDomain::key`] (or a measured peer's `remote_memory_key`), never from a
/// capacity/rate heuristic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredRamOrigin {
	pub device: DeviceId,
	pub key: Label,
}

/// Stable discovery key retained for one topology storage device.
///
/// The key is scoped by the device's machine and comes directly from
/// [`StorageDomain::key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredStorageOrigin {
	pub device: DeviceId,
	pub key: Label,
}

/// Stable discovery key retained for one topology GPU-memory device.
///
/// The key is scoped by the device's machine and comes directly from
/// [`GpuDescriptor::key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredGpuOrigin {
	pub device: DeviceId,
	pub key: Label,
}

/// Canonical identity provenance for topology objects whose core representation
/// intentionally contains no host-discovery strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredOrigins {
	pub machines: Vec<MeasuredMachineOrigin>,
	pub ram: Vec<MeasuredRamOrigin>,
	pub storage: Vec<MeasuredStorageOrigin>,
	pub gpu: Vec<MeasuredGpuOrigin>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasuredProfile {
	pub schema: u32,
	pub cache_identity: CacheIdentity,
	pub origins: MeasuredOrigins,
	pub benchmarks: BenchmarkMetadata,
	pub peer_benchmarks: Vec<MeasuredPeerBenchmark>,
	pub topology: Topology,
	pub discovery: DiscoveryProfile,
}

impl MeasuredProfile {
	#[must_use]
	pub fn is_cache_valid_for(&self, current: CacheIdentity) -> bool {
		self.schema == PROFILE_SCHEMA && self.cache_identity == current
	}

	#[must_use]
	pub fn machine_origin(&self, machine: MachineId) -> Option<&MeasuredMachineOrigin> {
		self.origins
			.machines
			.iter()
			.find(|origin| origin.machine == machine)
	}

	#[must_use]
	pub fn ram_origin(&self, device: DeviceId) -> Option<&MeasuredRamOrigin> {
		self.origins
			.ram
			.iter()
			.find(|origin| origin.device == device)
	}

	#[must_use]
	pub fn storage_origin(&self, device: DeviceId) -> Option<&MeasuredStorageOrigin> {
		self.origins
			.storage
			.iter()
			.find(|origin| origin.device == device)
	}

	#[must_use]
	pub fn gpu_origin(&self, device: DeviceId) -> Option<&MeasuredGpuOrigin> {
		self.origins
			.gpu
			.iter()
			.find(|origin| origin.device == device)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchmarkMetadata {
	pub seed_schema: u32,
	pub ram: BoundedBenchmarkPlan,
	pub storage: BoundedBenchmarkPlan,
	pub gpu: BoundedBenchmarkPlan,
	pub network: BoundedBenchmarkPlan,
}

pub trait ProfileCache: core::fmt::Debug {
	fn load(&self, identity: CacheIdentity) -> ProbeResult<Option<MeasuredProfile>>;
	fn store(&self, profile: &MeasuredProfile) -> ProbeResult<()>;
}

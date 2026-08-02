use std::collections::BTreeSet;

use recipe_core::{
	ArtifactId, ByteCount, DeviceId, Digest, DuplexMode, DuplexResourceId, FinalizedBundle, ResolvedTransferEndpoint,
	RunId, RunPhase, Task, TaskId, TaskKind, Topology, TransferLaneClaim,
};
use recipe_transport::EndpointIdentity;
use sha2::{Digest as _, Sha256};

use crate::{PROTOCOL_VERSION, RemoteError, RemoteResult};

pub(crate) const INIT_CHUNK_OVERHEAD: usize = 48;

const REQUIRED_CAPABILITIES: u64 = Capabilities::CHUNKED_INIT.0
	| Capabilities::BIDIRECTIONAL_DATA.0
	| Capabilities::PREPARED_EXECUTION.0
	| Capabilities::METRICS.0
	| Capabilities::CANCELLATION.0
	| Capabilities::EXACT_RELEASE_ACK.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Capabilities(u64);

impl Capabilities {
	pub const CHUNKED_INIT: Self = Self(1 << 0);
	pub const BIDIRECTIONAL_DATA: Self = Self(1 << 1);
	pub const PREPARED_EXECUTION: Self = Self(1 << 2);
	pub const METRICS: Self = Self(1 << 3);
	pub const CANCELLATION: Self = Self(1 << 4);
	pub const EXACT_RELEASE_ACK: Self = Self(1 << 5);
	pub const REQUIRED: Self = Self(REQUIRED_CAPABILITIES);

	#[must_use]
	pub const fn from_bits(bits: u64) -> Self { Self(bits) }

	#[must_use]
	pub const fn bits(self) -> u64 { self.0 }

	#[must_use]
	pub const fn contains(self, required: Self) -> bool { self.0 & required.0 == required.0 }
}

impl core::ops::BitOr for Capabilities {
	type Output = Self;

	fn bitor(self, right: Self) -> Self::Output { Self(self.0 | right.0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteIdentity {
	local: EndpointIdentity,
	remote: EndpointIdentity,
}

impl RemoteIdentity {
	pub fn new(local: EndpointIdentity, remote: EndpointIdentity) -> RemoteResult<Self> {
		if local.machine() == remote.machine() {
			return Err(RemoteError::InvalidConfiguration(
				"remote endpoints must identify distinct machines",
			));
		}
		Ok(Self { local, remote })
	}

	#[must_use]
	pub const fn local(self) -> EndpointIdentity { self.local }

	#[must_use]
	pub const fn remote(self) -> EndpointIdentity { self.remote }

	#[must_use]
	pub const fn reversed(self) -> Self {
		Self {
			local: self.remote,
			remote: self.local,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestLimits {
	max_artifacts: usize,
	max_tasks: usize,
	max_devices: usize,
	max_transfers: usize,
}

impl ManifestLimits {
	pub fn new(
		max_artifacts: usize,
		max_tasks: usize,
		max_devices: usize,
		max_transfers: usize,
	) -> RemoteResult<Self> {
		if max_artifacts == 0 || max_tasks == 0 || max_devices == 0 || max_transfers == 0 {
			return Err(RemoteError::InvalidConfiguration(
				"every remote manifest capacity must be nonzero",
			));
		}
		Ok(Self {
			max_artifacts,
			max_tasks,
			max_devices,
			max_transfers,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeSlots {
	task_slots: usize,
	data_slots: usize,
	metric_slots: usize,
}

impl RuntimeSlots {
	pub fn new(task_slots: usize, data_slots: usize, metric_slots: usize) -> RemoteResult<Self> {
		if task_slots == 0 || data_slots == 0 || metric_slots == 0 {
			return Err(RemoteError::InvalidConfiguration(
				"every remote runtime slot capacity must be nonzero",
			));
		}
		Ok(Self {
			task_slots,
			data_slots,
			metric_slots,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemoteLimits {
	max_message_bytes: usize,
	max_artifacts: usize,
	max_tasks: usize,
	max_devices: usize,
	max_transfers: usize,
	task_slots: usize,
	data_slots: usize,
	metric_slots: usize,
}

impl RemoteLimits {
	pub fn new(max_message_bytes: usize, manifest: ManifestLimits, runtime: RuntimeSlots) -> RemoteResult<Self> {
		if max_message_bytes < 256 {
			return Err(RemoteError::InvalidConfiguration(
				"remote messages require at least 256 bytes",
			));
		}
		if max_message_bytes > u32::MAX as usize {
			return Err(RemoteError::InvalidConfiguration(
				"remote message size does not fit the wire ABI",
			));
		}
		Ok(Self {
			max_message_bytes,
			max_artifacts: manifest.max_artifacts,
			max_tasks: manifest.max_tasks,
			max_devices: manifest.max_devices,
			max_transfers: manifest.max_transfers,
			task_slots: runtime.task_slots,
			data_slots: runtime.data_slots,
			metric_slots: runtime.metric_slots,
		})
	}

	#[must_use]
	pub const fn max_message_bytes(self) -> usize { self.max_message_bytes }

	#[must_use]
	pub const fn task_slots(self) -> usize { self.task_slots }

	#[must_use]
	pub const fn data_slots(self) -> usize { self.data_slots }

	#[must_use]
	pub const fn metric_slots(self) -> usize { self.metric_slots }

	pub(crate) const fn wire_tuple(self) -> [u64; 8] {
		[
			self.max_message_bytes as u64,
			self.max_artifacts as u64,
			self.max_tasks as u64,
			self.max_devices as u64,
			self.max_transfers as u64,
			self.task_slots as u64,
			self.data_slots as u64,
			self.metric_slots as u64,
		]
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataDirection {
	MasterToWorker,
	WorkerToMaster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DuplexClaim {
	pub resource: DuplexResourceId,
	pub mode: DuplexMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossTransfer {
	pub task: TaskId,
	pub direction: DataDirection,
	pub schedule: u64,
	pub bytes: ByteCount,
	pub claims: Box<[DuplexClaim]>,
}

impl CrossTransfer {
	fn new(
		task: TaskId,
		direction: DataDirection,
		schedule: u64,
		bytes: ByteCount,
		mut claims: Vec<DuplexClaim>,
	) -> RemoteResult<Self> {
		if task.get() == 0 {
			return Err(RemoteError::InvalidConfiguration(
				"cross-transfer task id must be nonzero",
			));
		}
		if schedule == u64::MAX {
			return Err(RemoteError::InvalidConfiguration(
				"cross-transfer schedule collides with the transport sentinel",
			));
		}
		if bytes == ByteCount::ZERO {
			return Err(RemoteError::InvalidConfiguration(
				"cross-transfer byte count must be nonzero",
			));
		}
		claims.sort_unstable();
		if claims
			.windows(2)
			.any(|pair| pair[0].resource == pair[1].resource)
		{
			return Err(RemoteError::InvalidConfiguration(
				"cross-transfer capacity resources must be unique",
			));
		}
		if claims.iter().any(|claim| claim.resource.get() == 0) {
			return Err(RemoteError::InvalidConfiguration(
				"cross-transfer capacity resource must be nonzero",
			));
		}
		Ok(Self {
			task,
			direction,
			schedule,
			bytes,
			claims: claims.into_boxed_slice(),
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestArtifact {
	pub id: ArtifactId,
	pub digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
	pub protocol: u16,
	pub bundle: Digest,
	pub draft: Digest,
	pub realization: Digest,
	pub artifacts: Box<[ManifestArtifact]>,
	pub digest: Digest,
}

impl Manifest {
	pub fn from_bundle(bundle: &FinalizedBundle, limits: RemoteLimits) -> RemoteResult<Self> {
		if bundle.artifacts().len() > limits.max_artifacts {
			return Err(RemoteError::CapacityExhausted("artifact manifest"));
		}
		let manifest_bytes = crate::codec::manifest_encoded_bytes(bundle.artifacts().len())
			.ok_or(RemoteError::CapacityExhausted("artifact manifest"))?;
		if manifest_bytes > limits.max_message_bytes() {
			return Err(RemoteError::CapacityExhausted("artifact manifest codec"));
		}
		let mut artifacts = bundle
			.artifacts()
			.iter()
			.map(|artifact| {
				ManifestArtifact {
					id: artifact.id,
					digest: artifact.digest,
				}
			})
			.collect::<Vec<_>>();
		artifacts.sort_by_key(|artifact| artifact.id);
		if artifacts.windows(2).any(|pair| pair[0].id == pair[1].id) {
			return Err(RemoteError::InvalidConfiguration(
				"artifact ids must be unique",
			));
		}
		let mut manifest = Self {
			protocol: PROTOCOL_VERSION,
			bundle: bundle.identity().digest(),
			draft: bundle.draft().digest(),
			realization: bundle.realization().digest(),
			artifacts: artifacts.into_boxed_slice(),
			digest: Digest::ZERO,
		};
		manifest.digest = manifest.compute_digest();
		Ok(manifest)
	}

	fn compute_digest(&self) -> Digest {
		let mut hash = Sha256::new();
		hash.update(b"recipe-remote-manifest-v1");
		hash.update(self.protocol.to_le_bytes());
		hash.update(self.bundle.bytes());
		hash.update(self.draft.bytes());
		hash.update(self.realization.bytes());
		hash.update((self.artifacts.len() as u64).to_le_bytes());
		for artifact in &self.artifacts {
			hash.update(artifact.id.get().to_le_bytes());
			hash.update(artifact.digest.bytes());
		}
		Digest::new(hash.finalize().into())
	}

	pub(crate) fn validate(&self) -> RemoteResult<()> {
		if self.protocol != PROTOCOL_VERSION {
			return Err(RemoteError::ManifestMismatch(
				"manifest protocol version differs",
			));
		}
		if self.bundle.is_zero() || self.draft.is_zero() || self.realization.is_zero() {
			return Err(RemoteError::ManifestMismatch(
				"manifest contains a zero plan identity",
			));
		}
		if self
			.artifacts
			.windows(2)
			.any(|pair| pair[0].id >= pair[1].id)
		{
			return Err(RemoteError::ManifestMismatch(
				"artifact manifest is not canonical",
			));
		}
		if self
			.artifacts
			.iter()
			.any(|artifact| artifact.digest.is_zero())
		{
			return Err(RemoteError::ManifestMismatch(
				"artifact manifest contains a zero digest",
			));
		}
		if self.compute_digest() != self.digest {
			return Err(RemoteError::ManifestMismatch(
				"manifest digest does not match its contents",
			));
		}
		Ok(())
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProgramTaskKind {
	Driver,
	CrossTransfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProgramTask {
	pub(crate) id: TaskId,
	pub(crate) phase: RunPhase,
	pub(crate) kind: ProgramTaskKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProgramDevice {
	pub(crate) id: DeviceId,
	pub(crate) image_bytes: ByteCount,
}

#[derive(Clone, Debug)]
pub struct ProvisionedProgram {
	manifest: Manifest,
	digest: Digest,
	tasks: Box<[ProgramTask]>,
	devices: Box<[ProgramDevice]>,
	transfers: Box<[CrossTransfer]>,
	half_duplex_resources: Box<[DuplexResourceId]>,
}

impl ProvisionedProgram {
	pub fn from_bundle(
		bundle: &FinalizedBundle,
		topology: &Topology,
		worker_devices: &[DeviceId],
		limits: RemoteLimits,
	) -> RemoteResult<Self> {
		let manifest = Manifest::from_bundle(bundle, limits)?;
		if bundle.topology() != topology.identity {
			return Err(RemoteError::ManifestMismatch(
				"provisioning topology differs from the finalized bundle",
			));
		}
		topology
			.validate()
			.and_then(|()| topology.validate_scheduling_properties())
			.map_err(|_| {
				RemoteError::InvalidConfiguration("remote provisioning requires a valid schedulable topology")
			})?;
		let worker_devices = canonical_devices(worker_devices, limits)?;
		let worker_set = worker_devices.iter().copied().collect::<BTreeSet<_>>();
		let mut devices = Vec::with_capacity(worker_devices.len());
		for device in &worker_devices {
			let image = bundle
				.init_image(*device)
				.ok_or(RemoteError::UnknownDevice(*device))?;
			devices.push(ProgramDevice {
				id: *device,
				image_bytes: image.bytes,
			});
		}

		let mut tasks = Vec::new();
		let mut transfer_specs = Vec::new();
		for task in bundle.tasks() {
			if task.id.get() == 0 {
				return Err(RemoteError::InvalidConfiguration(
					"task ids must be nonzero",
				));
			}
			let ownership = classify_task(bundle, task, &worker_set)?;
			match ownership {
				TaskOwnership::Master => {}
				TaskOwnership::WorkerDriver => {
					if task.phase == RunPhase::Init {
						return Err(RemoteError::InvalidConfiguration(
							"init-phase worker work must be represented by the one device image",
						));
					}
					tasks.push(ProgramTask {
						id: task.id,
						phase: task.phase,
						kind: ProgramTaskKind::Driver,
					});
				}
				TaskOwnership::Cross(direction, bytes) => {
					if task.phase == RunPhase::Init {
						return Err(RemoteError::InvalidConfiguration(
							"init-phase cross transfers must be represented by the one device image",
						));
					}
					transfer_specs.push(derive_cross_transfer(
						bundle, topology, task, direction, bytes,
					)?);
					tasks.push(ProgramTask {
						id: task.id,
						phase: task.phase,
						kind: ProgramTaskKind::CrossTransfer,
					});
				}
				TaskOwnership::InitImage => {}
			}
		}
		let transfers = schedule_cross_transfers(&devices, transfer_specs, limits)?;
		tasks.sort_by_key(|task| task.id);
		if tasks.len() > limits.max_tasks {
			return Err(RemoteError::CapacityExhausted("remote task manifest"));
		}
		if tasks.windows(2).any(|pair| pair[0].id == pair[1].id) {
			return Err(RemoteError::DuplicateTask(
				tasks.windows(2)
					.find(|pair| pair[0].id == pair[1].id)
					.map(|pair| pair[0].id)
					.ok_or(RemoteError::InvalidConfiguration(
						"duplicate task lookup failed",
					))?,
			));
		}
		let half_duplex_resources = transfers
			.iter()
			.flat_map(|transfer| transfer.claims.iter())
			.filter(|claim| claim.mode == DuplexMode::Half)
			.map(|claim| claim.resource)
			.collect::<BTreeSet<_>>()
			.into_iter()
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let mut program = Self {
			manifest,
			digest: Digest::ZERO,
			tasks: tasks.into_boxed_slice(),
			devices: devices.into_boxed_slice(),
			transfers: transfers.into_boxed_slice(),
			half_duplex_resources,
		};
		program.digest = program.compute_digest();
		Ok(program)
	}

	#[must_use]
	pub const fn manifest(&self) -> &Manifest { &self.manifest }

	#[must_use]
	pub const fn digest(&self) -> Digest { self.digest }

	#[must_use]
	pub fn worker_devices(&self) -> impl ExactSizeIterator<Item = DeviceId> + '_ {
		self.devices.iter().map(|device| device.id)
	}

	#[must_use]
	pub fn transfers(&self) -> &[CrossTransfer] { &self.transfers }

	pub(crate) fn tasks(&self) -> &[ProgramTask] { &self.tasks }

	pub(crate) fn devices(&self) -> &[ProgramDevice] { &self.devices }

	pub(crate) fn half_duplex_resources(&self) -> &[DuplexResourceId] { &self.half_duplex_resources }

	fn compute_digest(&self) -> Digest {
		let mut hash = Sha256::new();
		hash.update(b"recipe-remote-program-v1");
		hash.update(self.manifest.digest.bytes());
		hash.update((self.devices.len() as u64).to_le_bytes());
		for device in &self.devices {
			hash.update(device.id.get().to_le_bytes());
			hash.update(device.image_bytes.get().to_le_bytes());
		}
		hash.update((self.tasks.len() as u64).to_le_bytes());
		for task in &self.tasks {
			hash.update(task.id.get().to_le_bytes());
			hash.update([match task.phase {
				RunPhase::Init => 1,
				RunPhase::Loop => 2,
				RunPhase::Exit => 3,
			}]);
			hash.update([match task.kind {
				ProgramTaskKind::Driver => 1,
				ProgramTaskKind::CrossTransfer => 2,
			}]);
		}
		hash.update((self.transfers.len() as u64).to_le_bytes());
		for transfer in &self.transfers {
			hash.update(transfer.task.get().to_le_bytes());
			hash.update([match transfer.direction {
				DataDirection::MasterToWorker => 1,
				DataDirection::WorkerToMaster => 2,
			}]);
			hash.update(transfer.schedule.to_le_bytes());
			hash.update(transfer.bytes.get().to_le_bytes());
			hash.update((transfer.claims.len() as u64).to_le_bytes());
			for claim in &transfer.claims {
				hash.update(claim.resource.get().to_le_bytes());
				hash.update([match claim.mode {
					DuplexMode::Half => 1,
					DuplexMode::Full => 2,
				}]);
			}
		}
		Digest::new(hash.finalize().into())
	}
}

#[derive(Debug)]
struct CrossTransferSpec {
	task: TaskId,
	direction: DataDirection,
	start: u64,
	bytes: ByteCount,
	claims: Vec<DuplexClaim>,
}

fn derive_cross_transfer(
	bundle: &FinalizedBundle,
	topology: &Topology,
	task: &Task,
	direction: DataDirection,
	bytes: ByteCount,
) -> RemoteResult<CrossTransferSpec> {
	let TaskKind::Transfer(transfer) = &task.kind else {
		return Err(RemoteError::InvalidConfiguration(
			"cross-boundary work is not a finalized transfer",
		));
	};
	if transfer.bytes != bytes || transfer.route.len() != 1 {
		return Err(RemoteError::InvalidConfiguration(
			"remote transfers must be planner-expanded one-hop tasks",
		));
	}
	let link_id = transfer.route[0];
	let link = topology
		.link(link_id)
		.ok_or(RemoteError::InvalidConfiguration(
			"remote transfer route names an unknown measured link",
		))?;
	let endpoints = bundle
		.transfer_endpoints(task.id)
		.ok_or(RemoteError::UnknownTask(task.id))?;
	let (ResolvedTransferEndpoint::Device(source), ResolvedTransferEndpoint::Device(destination)) =
		(endpoints.source, endpoints.destination)
	else {
		return Err(RemoteError::InvalidConfiguration(
			"cross-boundary transfer endpoints must both be devices",
		));
	};
	if link.from != source.device || link.to != destination.device {
		return Err(RemoteError::InvalidConfiguration(
			"remote transfer direction differs from its measured topology link",
		));
	}
	let mut link_claims = transfer.lane_claims.iter().filter_map(|claim| {
		match claim {
			TransferLaneClaim::Link { link, .. } => Some(*link),
			TransferLaneClaim::External { .. } => None,
		}
	});
	if link_claims.next() != Some(link_id) || link_claims.next().is_some() {
		return Err(RemoteError::InvalidConfiguration(
			"remote transfer lane claim differs from its one-hop route",
		));
	}
	Ok(CrossTransferSpec {
		task: task.id,
		direction,
		start: task.window.start.get(),
		bytes,
		claims: vec![DuplexClaim {
			resource: link.capacity_resource,
			mode: link.duplex,
		}],
	})
}

fn schedule_cross_transfers(
	devices: &[ProgramDevice],
	mut specs: Vec<CrossTransferSpec>,
	limits: RemoteLimits,
) -> RemoteResult<Vec<CrossTransfer>> {
	specs.sort_by_key(|spec| (spec.direction, spec.start, spec.task));
	let init_chunks = init_chunk_count(devices, limits)?;
	let mut master_to_worker = init_chunks;
	let mut worker_to_master = 0_u64;
	let mut transfers = Vec::with_capacity(specs.len());
	for spec in specs {
		let schedule = match spec.direction {
			DataDirection::MasterToWorker => {
				let schedule = master_to_worker;
				master_to_worker = master_to_worker
					.checked_add(1)
					.ok_or(RemoteError::InvalidConfiguration(
						"master-to-worker schedule range overflowed",
					))?;
				schedule
			}
			DataDirection::WorkerToMaster => {
				let schedule = worker_to_master;
				worker_to_master = worker_to_master
					.checked_add(1)
					.ok_or(RemoteError::InvalidConfiguration(
						"worker-to-master schedule range overflowed",
					))?;
				schedule
			}
		};
		transfers.push(CrossTransfer::new(
			spec.task,
			spec.direction,
			schedule,
			spec.bytes,
			spec.claims,
		)?);
	}
	validate_transfer_order(&transfers, limits)?;
	let maximum_user_data = limits
		.max_message_bytes()
		.checked_sub(crate::codec::USER_DATA_OVERHEAD)
		.ok_or(RemoteError::InvalidConfiguration(
			"remote message capacity cannot hold user data",
		))?;
	if transfers
		.iter()
		.any(|transfer| usize::try_from(transfer.bytes.get()).map_or(true, |bytes| bytes > maximum_user_data))
	{
		return Err(RemoteError::CapacityExhausted(
			"cross-transfer codec payload",
		));
	}
	Ok(transfers)
}

pub(crate) fn init_chunk_count(devices: &[ProgramDevice], limits: RemoteLimits) -> RemoteResult<u64> {
	let chunk_bytes = limits
		.max_message_bytes()
		.checked_sub(INIT_CHUNK_OVERHEAD)
		.ok_or(RemoteError::InvalidConfiguration(
			"remote message capacity cannot hold an init chunk",
		))?;
	let mut chunks = 0_u64;
	for device in devices {
		let bytes = usize::try_from(device.image_bytes.get())
			.map_err(|_| RemoteError::InvalidConfiguration("init image byte count does not fit this host"))?;
		let count = bytes
			.checked_add(chunk_bytes - 1)
			.ok_or(RemoteError::InvalidConfiguration(
				"init chunk count overflowed",
			))? / chunk_bytes;
		chunks =
			chunks.checked_add(u64::try_from(count).map_err(|_| {
				RemoteError::InvalidConfiguration("init chunk count does not fit the wire ABI")
			})?)
			.ok_or(RemoteError::InvalidConfiguration(
				"init schedule range overflowed",
			))?;
	}
	Ok(chunks)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskOwnership {
	Master,
	WorkerDriver,
	Cross(DataDirection, ByteCount),
	InitImage,
}

fn classify_task(
	bundle: &FinalizedBundle,
	task: &recipe_core::Task,
	worker_devices: &BTreeSet<DeviceId>,
) -> RemoteResult<TaskOwnership> {
	match &task.kind {
		TaskKind::Calculation(calculation) => {
			match worker_devices.contains(&calculation.device) {
				true => Ok(TaskOwnership::WorkerDriver),
				false => Ok(TaskOwnership::Master),
			}
		}
		TaskKind::Metric(metric) => {
			let location = bundle
				.value_location(metric.value)
				.ok_or(RemoteError::UnknownTask(task.id))?;
			match worker_devices.contains(&location.device) {
				true => Ok(TaskOwnership::WorkerDriver),
				false => Ok(TaskOwnership::Master),
			}
		}
		TaskKind::Transfer(transfer) => {
			let endpoints = bundle
				.transfer_endpoints(task.id)
				.ok_or(RemoteError::UnknownTask(task.id))?;
			let source_worker = endpoint_is_worker(endpoints.source, worker_devices);
			let destination_worker = endpoint_is_worker(endpoints.destination, worker_devices);
			match (task.phase, source_worker, destination_worker) {
				(RunPhase::Init, false, true)
					if matches!(endpoints.source, ResolvedTransferEndpoint::External) =>
				{
					Ok(TaskOwnership::InitImage)
				}
				(_, true, true) => Ok(TaskOwnership::WorkerDriver),
				(_, false, false) => Ok(TaskOwnership::Master),
				(_, false, true) => {
					Ok(TaskOwnership::Cross(
						DataDirection::MasterToWorker,
						transfer.bytes,
					))
				}
				(_, true, false) => {
					Ok(TaskOwnership::Cross(
						DataDirection::WorkerToMaster,
						transfer.bytes,
					))
				}
			}
		}
	}
}

fn endpoint_is_worker(endpoint: ResolvedTransferEndpoint, worker_devices: &BTreeSet<DeviceId>) -> bool {
	match endpoint {
		ResolvedTransferEndpoint::Device(location) => worker_devices.contains(&location.device),
		ResolvedTransferEndpoint::External => false,
	}
}

fn canonical_devices(devices: &[DeviceId], limits: RemoteLimits) -> RemoteResult<Vec<DeviceId>> {
	if devices.is_empty() || devices.len() > limits.max_devices {
		return Err(RemoteError::InvalidConfiguration(
			"worker device count is outside the configured bounds",
		));
	}
	let mut result = devices.to_vec();
	result.sort_unstable();
	if result.iter().any(|device| device.get() == 0) {
		return Err(RemoteError::InvalidConfiguration(
			"worker device ids must be nonzero",
		));
	}
	if let Some(pair) = result.windows(2).find(|pair| pair[0] == pair[1]) {
		return Err(RemoteError::DuplicateDevice(pair[0]));
	}
	Ok(result)
}

fn validate_transfer_order(transfers: &[CrossTransfer], limits: RemoteLimits) -> RemoteResult<()> {
	if transfers.len() > limits.max_transfers {
		return Err(RemoteError::CapacityExhausted("cross-transfer manifest"));
	}
	for direction in [DataDirection::MasterToWorker, DataDirection::WorkerToMaster] {
		let mut prior = None;
		for transfer in transfers
			.iter()
			.filter(|transfer| transfer.direction == direction)
		{
			if prior.is_some_and(|stamp| transfer.schedule <= stamp) {
				return Err(RemoteError::InvalidConfiguration(
					"cross-transfer schedules must increase strictly in each direction",
				));
			}
			prior = Some(transfer.schedule);
		}
	}
	Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeCapacities {
	pub task_slots: usize,
	pub data_slots: usize,
	pub metric_slots: usize,
	pub scratch_bytes: usize,
	pub half_duplex_tokens: usize,
}

pub(crate) fn validate_run_id(run: RunId) -> RemoteResult<()> {
	if run.get() == 0 {
		Err(RemoteError::InvalidConfiguration(
			"remote run id must be nonzero",
		))
	} else {
		Ok(())
	}
}

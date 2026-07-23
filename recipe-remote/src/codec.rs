use recipe_core::{ByteCount, DeviceId, Digest, RunId, TaskId};
use recipe_transport::RuntimeLane;
use sha2::{Digest as _, Sha256};

use crate::driver::DriverFault;
use crate::model::{Capabilities, Manifest, RemoteIdentity, RemoteLimits};
use crate::session::{CancelReason, RemoteMetricValue};
use crate::{PROTOCOL_VERSION, RemoteError, RemoteResult};

const MAGIC: [u8; 8] = *b"RCPREM01";
const HEADER_BYTES: usize = 28;
const ARTIFACT_BYTES: usize = 40;
pub(crate) const USER_DATA_OVERHEAD: usize = HEADER_BYTES + 12;

pub(crate) fn manifest_encoded_bytes(artifacts: usize) -> Option<usize> {
	HEADER_BYTES
		.checked_add(164)?
		.checked_add(artifacts.checked_mul(ARTIFACT_BYTES)?)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PeerRole {
	Master = 1,
	Worker = 2,
}

impl PeerRole {
	fn decode(value: u8) -> RemoteResult<Self> {
		match value {
			1 => Ok(Self::Master),
			2 => Ok(Self::Worker),
			_ => Err(RemoteError::Codec("unknown peer role")),
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WireHello {
	pub(crate) role: PeerRole,
	pub(crate) capabilities: Capabilities,
	pub(crate) local_machine: Digest,
	pub(crate) local_profile: Digest,
	pub(crate) remote_machine: Digest,
	pub(crate) remote_profile: Digest,
	pub(crate) limits: [u64; 8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WireManifest<'a> {
	pub(crate) bundle: Digest,
	pub(crate) draft: Digest,
	pub(crate) realization: Digest,
	pub(crate) manifest_digest: Digest,
	pub(crate) program_digest: Digest,
	pub(crate) artifact_count: usize,
	artifact_bytes: &'a [u8],
}

impl WireManifest<'_> {
	pub(crate) fn matches_proof(
		self,
		bundle: Digest,
		draft: Digest,
		realization: Digest,
		manifest_digest: Digest,
		program_digest: Digest,
		artifact_count: usize,
	) -> bool {
		if self.bundle != bundle
			|| self.draft != draft
			|| self.realization != realization
			|| self.manifest_digest != manifest_digest
			|| self.program_digest != program_digest
			|| self.artifact_count != artifact_count
		{
			return false;
		}
		let mut hash = Sha256::new();
		hash.update(b"recipe-remote-manifest-v1");
		hash.update(PROTOCOL_VERSION.to_le_bytes());
		hash.update(self.bundle.bytes());
		hash.update(self.draft.bytes());
		hash.update(self.realization.bytes());
		hash.update((self.artifact_count as u64).to_le_bytes());
		let mut prior_id = None;
		for encoded in self.artifact_bytes.chunks_exact(ARTIFACT_BYTES) {
			let id_bytes: [u8; 8] = match encoded[..8].try_into() {
				Ok(bytes) => bytes,
				Err(_) => return false,
			};
			let digest_bytes: [u8; 32] = match encoded[8..].try_into() {
				Ok(bytes) => bytes,
				Err(_) => return false,
			};
			let id = u64::from_le_bytes(id_bytes);
			if id == 0 || prior_id.is_some_and(|prior| id <= prior) || digest_bytes == [0; 32] {
				return false;
			}
			prior_id = Some(id);
			hash.update(id_bytes);
			hash.update(digest_bytes);
		}
		let actual: [u8; 32] = hash.finalize().into();
		Digest::new(actual) == self.manifest_digest
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Message<'a> {
	Hello(WireHello),
	Manifest(WireManifest<'a>),
	ManifestAck {
		manifest: Digest,
		program: Digest,
	},
	Prepare,
	PrepareAck,
	InitBegin {
		device: DeviceId,
		bytes: ByteCount,
		digest: Digest,
	},
	InitChunk {
		device: DeviceId,
		offset: u64,
		bytes: &'a [u8],
	},
	InitEnd {
		device: DeviceId,
		bytes: ByteCount,
	},
	InitAck {
		device: DeviceId,
	},
	InitComplete,
	InitCompleteAck,
	Execute {
		task: TaskId,
	},
	TaskComplete {
		task: TaskId,
	},
	TaskFailed {
		task: TaskId,
		fault: DriverFault,
	},
	DriverFault {
		fault: DriverFault,
	},
	DataRequest {
		task: TaskId,
	},
	UserData {
		task: TaskId,
		bytes: &'a [u8],
	},
	DataAck {
		task: TaskId,
	},
	Metric {
		task: TaskId,
		value: RemoteMetricValue,
	},
	Cancel {
		reason: CancelReason,
	},
	CancelAck,
	BeginExit,
	ExitReady,
	Release {
		device: DeviceId,
	},
	ReleaseAck {
		device: DeviceId,
	},
	ExitComplete,
	ExitAck,
}

impl Message<'_> {
	pub(crate) const fn lane(self) -> RuntimeLane {
		match self {
			Self::InitChunk { .. } | Self::UserData { .. } => RuntimeLane::UserData,
			Self::Metric { .. } => RuntimeLane::Metrics,
			Self::Hello(_)
			| Self::Manifest(_)
			| Self::ManifestAck { .. }
			| Self::Prepare
			| Self::PrepareAck
			| Self::InitBegin { .. }
			| Self::InitEnd { .. }
			| Self::InitAck { .. }
			| Self::InitComplete
			| Self::InitCompleteAck
			| Self::Execute { .. }
			| Self::TaskComplete { .. }
			| Self::TaskFailed { .. }
			| Self::DriverFault { .. }
			| Self::DataRequest { .. }
			| Self::DataAck { .. }
			| Self::Cancel { .. }
			| Self::CancelAck
			| Self::BeginExit
			| Self::ExitReady
			| Self::Release { .. }
			| Self::ReleaseAck { .. }
			| Self::ExitComplete
			| Self::ExitAck => RuntimeLane::Control,
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Decoded<'a> {
	pub(crate) sequence: u64,
	pub(crate) run: RunId,
	pub(crate) message: Message<'a>,
}

pub(crate) fn encode(buffer: &mut [u8], sequence: u64, run: RunId, message: Message<'_>) -> RemoteResult<usize> {
	let mut writer = Writer::new(buffer);
	writer.bytes(&MAGIC)?;
	writer.u16(PROTOCOL_VERSION)?;
	writer.u8(tag(message))?;
	writer.u8(0)?;
	writer.u64(sequence)?;
	writer.u64(run.get())?;
	match message {
		Message::Hello(hello) => {
			writer.u8(hello.role as u8)?;
			writer.u64(hello.capabilities.bits())?;
			writer.digest(hello.local_machine)?;
			writer.digest(hello.local_profile)?;
			writer.digest(hello.remote_machine)?;
			writer.digest(hello.remote_profile)?;
			for value in hello.limits {
				writer.u64(value)?;
			}
		}
		Message::Manifest(manifest) => {
			writer.digest(manifest.bundle)?;
			writer.digest(manifest.draft)?;
			writer.digest(manifest.realization)?;
			writer.digest(manifest.manifest_digest)?;
			writer.digest(manifest.program_digest)?;
			writer.u32(u32::try_from(manifest.artifact_count)
				.map_err(|_| RemoteError::Codec("artifact count does not fit the wire ABI"))?)?;
			writer.bytes(manifest.artifact_bytes)?;
		}
		Message::ManifestAck { manifest, program } => {
			writer.digest(manifest)?;
			writer.digest(program)?;
		}
		Message::Prepare
		| Message::PrepareAck
		| Message::InitComplete
		| Message::InitCompleteAck
		| Message::CancelAck
		| Message::BeginExit
		| Message::ExitReady
		| Message::ExitComplete
		| Message::ExitAck => {}
		Message::InitBegin {
			device,
			bytes,
			digest,
		} => {
			writer.u64(device.get())?;
			writer.u64(bytes.get())?;
			writer.digest(digest)?;
		}
		Message::InitChunk {
			device,
			offset,
			bytes,
		} => {
			writer.u64(device.get())?;
			writer.u64(offset)?;
			writer.u32(u32::try_from(bytes.len())
				.map_err(|_| RemoteError::Codec("init chunk does not fit the wire ABI"))?)?;
			writer.bytes(bytes)?;
		}
		Message::InitEnd { device, bytes } => {
			writer.u64(device.get())?;
			writer.u64(bytes.get())?;
		}
		Message::InitAck { device } | Message::Release { device } | Message::ReleaseAck { device } => {
			writer.u64(device.get())?;
		}
		Message::Execute { task }
		| Message::TaskComplete { task }
		| Message::DataRequest { task }
		| Message::DataAck { task } => writer.u64(task.get())?,
		Message::TaskFailed { task, fault } => {
			writer.u64(task.get())?;
			writer.u32(fault.code)?;
			writer.u64(fault.detail)?;
		}
		Message::DriverFault { fault } => {
			writer.u32(fault.code)?;
			writer.u64(fault.detail)?;
		}
		Message::UserData { task, bytes } => {
			writer.u64(task.get())?;
			writer.u32(u32::try_from(bytes.len())
				.map_err(|_| RemoteError::Codec("user data does not fit the wire ABI"))?)?;
			writer.bytes(bytes)?;
		}
		Message::Metric { task, value } => {
			writer.u64(task.get())?;
			match value {
				RemoteMetricValue::F32(value) => {
					writer.u8(1)?;
					writer.u32(value.to_bits())?;
				}
				RemoteMetricValue::I32(value) => {
					writer.u8(2)?;
					writer.u32(u32::from_le_bytes(value.to_le_bytes()))?;
				}
			}
		}
		Message::Cancel { reason } => writer.u32(reason.code())?,
	}
	Ok(writer.finish())
}

pub(crate) fn encode_hello(
	buffer: &mut [u8],
	sequence: u64,
	run: RunId,
	role: PeerRole,
	identity: RemoteIdentity,
	limits: RemoteLimits,
) -> RemoteResult<usize> {
	encode(
		buffer,
		sequence,
		run,
		Message::Hello(WireHello {
			role,
			capabilities: Capabilities::REQUIRED,
			local_machine: identity.local().machine(),
			local_profile: identity.local().profile(),
			remote_machine: identity.remote().machine(),
			remote_profile: identity.remote().profile(),
			limits: limits.wire_tuple(),
		}),
	)
}

pub(crate) fn encode_manifest(
	buffer: &mut [u8],
	sequence: u64,
	run: RunId,
	manifest: &Manifest,
	program_digest: Digest,
) -> RemoteResult<usize> {
	let required = manifest_encoded_bytes(manifest.artifacts.len())
		.ok_or(RemoteError::Codec("manifest payload length overflowed"))?;
	if required > buffer.len() {
		return Err(RemoteError::CapacityExhausted("manifest codec buffer"));
	}
	let mut writer = Writer::new(buffer);
	writer.bytes(&MAGIC)?;
	writer.u16(PROTOCOL_VERSION)?;
	writer.u8(2)?;
	writer.u8(0)?;
	writer.u64(sequence)?;
	writer.u64(run.get())?;
	writer.digest(manifest.bundle)?;
	writer.digest(manifest.draft)?;
	writer.digest(manifest.realization)?;
	writer.digest(manifest.digest)?;
	writer.digest(program_digest)?;
	writer.u32(u32::try_from(manifest.artifacts.len())
		.map_err(|_| RemoteError::Codec("artifact count does not fit the wire ABI"))?)?;
	for artifact in &manifest.artifacts {
		writer.u64(artifact.id.get())?;
		writer.digest(artifact.digest)?;
	}
	Ok(writer.finish())
}

pub(crate) fn decode(payload: &[u8], limits: RemoteLimits) -> RemoteResult<Decoded<'_>> {
	if payload.len() > limits.max_message_bytes() {
		return Err(RemoteError::Codec("message exceeds the configured bound"));
	}
	let mut reader = Reader::new(payload);
	if reader.array::<8>()? != MAGIC {
		return Err(RemoteError::Codec("message magic differs"));
	}
	if reader.u16()? != PROTOCOL_VERSION {
		return Err(RemoteError::Codec("message protocol version differs"));
	}
	let tag = reader.u8()?;
	if reader.u8()? != 0 {
		return Err(RemoteError::Codec("message reserved byte is nonzero"));
	}
	let sequence = reader.u64()?;
	let run = RunId::new(reader.u64()?);
	let message = match tag {
		1 => Message::Hello(WireHello {
			role: PeerRole::decode(reader.u8()?)?,
			capabilities: Capabilities::from_bits(reader.u64()?),
			local_machine: reader.digest()?,
			local_profile: reader.digest()?,
			remote_machine: reader.digest()?,
			remote_profile: reader.digest()?,
			limits: [
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
				reader.u64()?,
			],
		}),
		2 => {
			let bundle = reader.digest()?;
			let draft = reader.digest()?;
			let realization = reader.digest()?;
			let manifest_digest = reader.digest()?;
			let program_digest = reader.digest()?;
			let artifact_count = usize::try_from(reader.u32()?)
				.map_err(|_| RemoteError::Codec("artifact count does not fit this host"))?;
			if artifact_count > limits.wire_tuple()[1] as usize {
				return Err(RemoteError::Codec(
					"artifact count exceeds configured bound",
				));
			}
			let bytes = artifact_count
				.checked_mul(ARTIFACT_BYTES)
				.ok_or(RemoteError::Codec("artifact byte count overflowed"))?;
			Message::Manifest(WireManifest {
				bundle,
				draft,
				realization,
				manifest_digest,
				program_digest,
				artifact_count,
				artifact_bytes: reader.take(bytes)?,
			})
		}
		3 => Message::ManifestAck {
			manifest: reader.digest()?,
			program: reader.digest()?,
		},
		4 => Message::Prepare,
		5 => Message::PrepareAck,
		6 => Message::InitBegin {
			device: DeviceId::new(reader.u64()?),
			bytes: ByteCount::new(reader.u64()?),
			digest: reader.digest()?,
		},
		7 => {
			let device = DeviceId::new(reader.u64()?);
			let offset = reader.u64()?;
			let bytes = usize::try_from(reader.u32()?)
				.map_err(|_| RemoteError::Codec("init chunk length does not fit this host"))?;
			Message::InitChunk {
				device,
				offset,
				bytes: reader.take(bytes)?,
			}
		}
		8 => Message::InitEnd {
			device: DeviceId::new(reader.u64()?),
			bytes: ByteCount::new(reader.u64()?),
		},
		9 => Message::InitAck {
			device: DeviceId::new(reader.u64()?),
		},
		10 => Message::InitComplete,
		11 => Message::InitCompleteAck,
		12 => Message::Execute {
			task: TaskId::new(reader.u64()?),
		},
		13 => Message::TaskComplete {
			task: TaskId::new(reader.u64()?),
		},
		14 => Message::TaskFailed {
			task: TaskId::new(reader.u64()?),
			fault: DriverFault {
				code: reader.u32()?,
				detail: reader.u64()?,
			},
		},
		15 => Message::DataRequest {
			task: TaskId::new(reader.u64()?),
		},
		16 => {
			let task = TaskId::new(reader.u64()?);
			let bytes = usize::try_from(reader.u32()?)
				.map_err(|_| RemoteError::Codec("user-data length does not fit this host"))?;
			Message::UserData {
				task,
				bytes: reader.take(bytes)?,
			}
		}
		17 => Message::DataAck {
			task: TaskId::new(reader.u64()?),
		},
		18 => {
			let task = TaskId::new(reader.u64()?);
			let kind = reader.u8()?;
			let bits = reader.u32()?;
			let value = match kind {
				1 => RemoteMetricValue::F32(f32::from_bits(bits)),
				2 => RemoteMetricValue::I32(i32::from_le_bytes(bits.to_le_bytes())),
				_ => return Err(RemoteError::Codec("unknown metric scalar type")),
			};
			Message::Metric { task, value }
		}
		19 => Message::Cancel {
			reason: CancelReason::new(reader.u32()?),
		},
		20 => Message::CancelAck,
		21 => Message::BeginExit,
		22 => Message::ExitReady,
		23 => Message::Release {
			device: DeviceId::new(reader.u64()?),
		},
		24 => Message::ReleaseAck {
			device: DeviceId::new(reader.u64()?),
		},
		25 => Message::ExitComplete,
		26 => Message::ExitAck,
		27 => Message::DriverFault {
			fault: DriverFault {
				code: reader.u32()?,
				detail: reader.u64()?,
			},
		},
		_ => return Err(RemoteError::Codec("unknown message tag")),
	};
	if !reader.is_empty() {
		return Err(RemoteError::Codec("message has trailing bytes"));
	}
	Ok(Decoded {
		sequence,
		run,
		message,
	})
}

const fn tag(message: Message<'_>) -> u8 {
	match message {
		Message::Hello(_) => 1,
		Message::Manifest(_) => 2,
		Message::ManifestAck { .. } => 3,
		Message::Prepare => 4,
		Message::PrepareAck => 5,
		Message::InitBegin { .. } => 6,
		Message::InitChunk { .. } => 7,
		Message::InitEnd { .. } => 8,
		Message::InitAck { .. } => 9,
		Message::InitComplete => 10,
		Message::InitCompleteAck => 11,
		Message::Execute { .. } => 12,
		Message::TaskComplete { .. } => 13,
		Message::TaskFailed { .. } => 14,
		Message::DataRequest { .. } => 15,
		Message::UserData { .. } => 16,
		Message::DataAck { .. } => 17,
		Message::Metric { .. } => 18,
		Message::Cancel { .. } => 19,
		Message::CancelAck => 20,
		Message::BeginExit => 21,
		Message::ExitReady => 22,
		Message::Release { .. } => 23,
		Message::ReleaseAck { .. } => 24,
		Message::ExitComplete => 25,
		Message::ExitAck => 26,
		Message::DriverFault { .. } => 27,
	}
}

struct Writer<'a> {
	buffer: &'a mut [u8],
	cursor: usize,
}

impl<'a> Writer<'a> {
	const fn new(buffer: &'a mut [u8]) -> Self {
		Self { buffer, cursor: 0 }
	}

	fn bytes(&mut self, bytes: &[u8]) -> RemoteResult<()> {
		let end = self
			.cursor
			.checked_add(bytes.len())
			.ok_or(RemoteError::Codec("message length overflowed"))?;
		let target = self
			.buffer
			.get_mut(self.cursor..end)
			.ok_or(RemoteError::CapacityExhausted("message codec buffer"))?;
		target.copy_from_slice(bytes);
		self.cursor = end;
		Ok(())
	}

	fn u8(&mut self, value: u8) -> RemoteResult<()> {
		self.bytes(&[value])
	}

	fn u16(&mut self, value: u16) -> RemoteResult<()> {
		self.bytes(&value.to_le_bytes())
	}

	fn u32(&mut self, value: u32) -> RemoteResult<()> {
		self.bytes(&value.to_le_bytes())
	}

	fn u64(&mut self, value: u64) -> RemoteResult<()> {
		self.bytes(&value.to_le_bytes())
	}

	fn digest(&mut self, value: Digest) -> RemoteResult<()> {
		self.bytes(&value.bytes())
	}

	const fn finish(self) -> usize {
		self.cursor
	}
}

struct Reader<'a> {
	bytes: &'a [u8],
	cursor: usize,
}

impl<'a> Reader<'a> {
	const fn new(bytes: &'a [u8]) -> Self {
		Self { bytes, cursor: 0 }
	}

	fn take(&mut self, length: usize) -> RemoteResult<&'a [u8]> {
		let end = self
			.cursor
			.checked_add(length)
			.ok_or(RemoteError::Codec("message range overflowed"))?;
		let result = self
			.bytes
			.get(self.cursor..end)
			.ok_or(RemoteError::Codec("message is truncated"))?;
		self.cursor = end;
		Ok(result)
	}

	fn array<const N: usize>(&mut self) -> RemoteResult<[u8; N]> {
		self.take(N)?
			.try_into()
			.map_err(|_| RemoteError::Codec("fixed-width field is truncated"))
	}

	fn u8(&mut self) -> RemoteResult<u8> {
		Ok(self.array::<1>()?[0])
	}

	fn u16(&mut self) -> RemoteResult<u16> {
		Ok(u16::from_le_bytes(self.array()?))
	}

	fn u32(&mut self) -> RemoteResult<u32> {
		Ok(u32::from_le_bytes(self.array()?))
	}

	fn u64(&mut self) -> RemoteResult<u64> {
		Ok(u64::from_le_bytes(self.array()?))
	}

	fn digest(&mut self) -> RemoteResult<Digest> {
		Ok(Digest::new(self.array()?))
	}

	fn is_empty(&self) -> bool {
		self.cursor == self.bytes.len()
	}
}

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::{Duration, Instant};

use recipe_core::Digest;
use sha2::{Digest as _, Sha256};

use crate::error::{TransportError, TransportResult};

pub(crate) const PROTOCOL_VERSION: u16 = 1;
pub(crate) const HEADER_BYTES: usize = 200;
pub(crate) const NO_SCHEDULE: u64 = u64::MAX;
const MAGIC: [u8; 8] = *b"RCPTRN01";
const MAXIMUM_PROTOCOL_PAYLOAD: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndpointIdentity {
	machine: Digest,
	profile: Digest,
}

impl EndpointIdentity {
	pub fn new(machine: Digest, profile: Digest) -> TransportResult<Self> {
		if machine.is_zero() {
			return Err(TransportError::InvalidIdentity("machine"));
		}
		if profile.is_zero() {
			return Err(TransportError::InvalidIdentity("profile"));
		}
		Ok(Self { machine, profile })
	}

	#[must_use]
	pub const fn machine(self) -> Digest {
		self.machine
	}

	#[must_use]
	pub const fn profile(self) -> Digest {
		self.profile
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionIdentity {
	local: EndpointIdentity,
	remote: EndpointIdentity,
}

impl SessionIdentity {
	pub fn new(local: EndpointIdentity, remote: EndpointIdentity) -> TransportResult<Self> {
		if local.machine == remote.machine {
			return Err(TransportError::InvalidConfiguration(
				"transport endpoints must identify distinct machines",
			));
		}
		Ok(Self { local, remote })
	}

	#[must_use]
	pub const fn local(self) -> EndpointIdentity {
		self.local
	}

	#[must_use]
	pub const fn remote(self) -> EndpointIdentity {
		self.remote
	}

	#[must_use]
	pub const fn reversed(self) -> Self {
		Self {
			local: self.remote,
			remote: self.local,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompletionToken(u64);

impl CompletionToken {
	pub fn new(value: u64) -> TransportResult<Self> {
		if value == 0 {
			return Err(TransportError::InvalidConfiguration(
				"completion token must be nonzero",
			));
		}
		Ok(Self(value))
	}

	#[must_use]
	pub const fn get(self) -> u64 {
		self.0
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameKind {
	ProbeBegin = 1,
	ProbeReady = 2,
	ProbeData = 3,
	ProbeComplete = 4,
	Control = 16,
	Metrics = 17,
	UserData = 18,
}

impl FrameKind {
	fn decode(value: u8) -> TransportResult<Self> {
		match value {
			1 => Ok(Self::ProbeBegin),
			2 => Ok(Self::ProbeReady),
			3 => Ok(Self::ProbeData),
			4 => Ok(Self::ProbeComplete),
			16 => Ok(Self::Control),
			17 => Ok(Self::Metrics),
			18 => Ok(Self::UserData),
			_ => Err(TransportError::InvalidFrame("unknown frame kind")),
		}
	}

	#[must_use]
	pub(crate) const fn is_runtime(self) -> bool {
		matches!(self, Self::Control | Self::Metrics | Self::UserData)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameMetadata {
	pub kind: FrameKind,
	pub token: CompletionToken,
	/// Static schedule position for user-data, or `None` for every other lane.
	pub schedule: Option<u64>,
}

impl FrameMetadata {
	pub fn new(kind: FrameKind, token: CompletionToken, schedule: Option<u64>) -> TransportResult<Self> {
		match (kind, schedule) {
			(FrameKind::UserData, Some(value)) if value != NO_SCHEDULE => {}
			(FrameKind::UserData, _) => {
				return Err(TransportError::InvalidFrame(
					"user-data frame requires a finite schedule position",
				));
			}
			(_, None) => {}
			(_, Some(_)) => {
				return Err(TransportError::InvalidFrame(
					"only user-data frames may carry a schedule position",
				));
			}
		}
		Ok(Self {
			kind,
			token,
			schedule,
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtocolLimits {
	max_payload: usize,
	operation_timeout: Duration,
}

impl ProtocolLimits {
	pub fn new(max_payload: usize, operation_timeout: Duration) -> TransportResult<Self> {
		if max_payload == 0 || max_payload > MAXIMUM_PROTOCOL_PAYLOAD || max_payload > u32::MAX as usize {
			return Err(TransportError::InvalidConfiguration(
				"maximum payload must be between 1 byte and 64 MiB",
			));
		}
		if operation_timeout.is_zero() {
			return Err(TransportError::InvalidConfiguration(
				"operation timeout must be nonzero",
			));
		}
		Ok(Self {
			max_payload,
			operation_timeout,
		})
	}

	#[must_use]
	pub const fn max_payload(self) -> usize {
		self.max_payload
	}

	#[must_use]
	pub const fn operation_timeout(self) -> Duration {
		self.operation_timeout
	}

	#[must_use]
	pub fn deadline(self) -> Instant {
		Instant::now() + self.operation_timeout
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceivedFrame<'a> {
	pub metadata: FrameMetadata,
	pub payload: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecodedHeader {
	pub(crate) metadata: FrameMetadata,
	pub(crate) payload_len: usize,
	pub(crate) sequence: u64,
	pub(crate) digest: [u8; 32],
}

#[derive(Debug)]
pub struct WireSender {
	stream: TcpStream,
	identity: SessionIdentity,
	limits: ProtocolLimits,
	next_sequence: u64,
	poisoned: bool,
}

impl WireSender {
	pub fn send(&mut self, metadata: FrameMetadata, payload: &[u8], deadline: Instant) -> TransportResult<()> {
		if self.poisoned {
			return Err(TransportError::Poisoned);
		}
		if payload.len() > self.limits.max_payload {
			return Err(TransportError::FrameTooLarge {
				declared: payload.len(),
				limit: self.limits.max_payload,
			});
		}
		let header = encode_header(
			self.identity,
			self.next_sequence,
			metadata,
			payload.len(),
			payload_digest(payload),
		)?;
		let result = write_all_deadline(&mut self.stream, &header, deadline)
			.and_then(|()| write_all_deadline(&mut self.stream, payload, deadline));
		if let Err(error) = result {
			self.poison();
			return Err(error);
		}
		self.next_sequence = self
			.next_sequence
			.checked_add(1)
			.ok_or(TransportError::ProtocolState("transmit sequence exhausted"))?;
		Ok(())
	}

	#[must_use]
	pub const fn next_sequence(&self) -> u64 {
		self.next_sequence
	}

	fn poison(&mut self) {
		self.poisoned = true;
		let _ = self.stream.shutdown(Shutdown::Both);
	}
}

#[derive(Debug)]
pub struct WireReceiver {
	stream: TcpStream,
	identity: SessionIdentity,
	limits: ProtocolLimits,
	next_sequence: u64,
	poisoned: bool,
}

impl WireReceiver {
	pub fn receive_into<'a>(
		&mut self,
		buffer: &'a mut [u8],
		deadline: Instant,
	) -> TransportResult<ReceivedFrame<'a>> {
		if self.poisoned {
			return Err(TransportError::Poisoned);
		}
		let result = self.receive_inner(buffer, deadline);
		if result.is_err() {
			self.poison();
		}
		result
	}

	fn receive_inner<'a>(&mut self, buffer: &'a mut [u8], deadline: Instant) -> TransportResult<ReceivedFrame<'a>> {
		let mut header = [0_u8; HEADER_BYTES];
		read_exact_deadline(&mut self.stream, &mut header, deadline)?;
		let decoded = decode_header(&header, self.identity, self.limits, self.next_sequence)?;
		if decoded.payload_len > buffer.len() {
			return Err(TransportError::BufferTooSmall {
				required: decoded.payload_len,
				available: buffer.len(),
			});
		}
		let payload = &mut buffer[..decoded.payload_len];
		read_exact_deadline(&mut self.stream, payload, deadline)?;
		if payload_digest(payload) != decoded.digest {
			return Err(TransportError::IntegrityFailure);
		}
		self.next_sequence = self
			.next_sequence
			.checked_add(1)
			.ok_or(TransportError::ProtocolState("receive sequence exhausted"))?;
		Ok(ReceivedFrame {
			metadata: decoded.metadata,
			payload,
		})
	}

	#[must_use]
	pub const fn next_sequence(&self) -> u64 {
		self.next_sequence
	}

	fn poison(&mut self) {
		self.poisoned = true;
		let _ = self.stream.shutdown(Shutdown::Both);
	}
}

pub fn split_stream(
	stream: TcpStream,
	identity: SessionIdentity,
	limits: ProtocolLimits,
) -> TransportResult<(WireSender, WireReceiver)> {
	stream.set_nonblocking(false)?;
	stream.set_nodelay(true)?;
	let write_stream = stream.try_clone()?;
	Ok((
		WireSender {
			stream: write_stream,
			identity,
			limits,
			next_sequence: 0,
			poisoned: false,
		},
		WireReceiver {
			stream,
			identity,
			limits,
			next_sequence: 0,
			poisoned: false,
		},
	))
}

pub(crate) fn encode_header(
	identity: SessionIdentity,
	sequence: u64,
	metadata: FrameMetadata,
	payload_len: usize,
	digest: [u8; 32],
) -> TransportResult<[u8; HEADER_BYTES]> {
	if payload_len > u32::MAX as usize {
		return Err(TransportError::FrameTooLarge {
			declared: payload_len,
			limit: u32::MAX as usize,
		});
	}
	let schedule = metadata.schedule.unwrap_or(NO_SCHEDULE);
	FrameMetadata::new(metadata.kind, metadata.token, metadata.schedule)?;
	let mut header = [0_u8; HEADER_BYTES];
	header[0..8].copy_from_slice(&MAGIC);
	header[8..10].copy_from_slice(&PROTOCOL_VERSION.to_be_bytes());
	header[10] = metadata.kind as u8;
	header[11] = 0;
	header[12..16].copy_from_slice(&(payload_len as u32).to_be_bytes());
	header[16..24].copy_from_slice(&sequence.to_be_bytes());
	header[24..32].copy_from_slice(&metadata.token.get().to_be_bytes());
	header[32..40].copy_from_slice(&schedule.to_be_bytes());
	header[40..72].copy_from_slice(&identity.local.machine.bytes());
	header[72..104].copy_from_slice(&identity.local.profile.bytes());
	header[104..136].copy_from_slice(&identity.remote.machine.bytes());
	header[136..168].copy_from_slice(&identity.remote.profile.bytes());
	header[168..200].copy_from_slice(&digest);
	Ok(header)
}

pub(crate) fn decode_header(
	header: &[u8; HEADER_BYTES],
	identity: SessionIdentity,
	limits: ProtocolLimits,
	expected_sequence: u64,
) -> TransportResult<DecodedHeader> {
	if header[0..8] != MAGIC {
		return Err(TransportError::InvalidFrame("bad magic"));
	}
	let version = u16::from_be_bytes([header[8], header[9]]);
	if version != PROTOCOL_VERSION {
		return Err(TransportError::UnsupportedVersion(version));
	}
	if header[11] != 0 {
		return Err(TransportError::InvalidFrame("reserved flags are nonzero"));
	}
	let kind = FrameKind::decode(header[10])?;
	let payload_len = u32::from_be_bytes(header[12..16].try_into().expect("fixed header slice")) as usize;
	if payload_len > limits.max_payload {
		return Err(TransportError::FrameTooLarge {
			declared: payload_len,
			limit: limits.max_payload,
		});
	}
	let sequence = u64::from_be_bytes(header[16..24].try_into().expect("fixed header slice"));
	if sequence != expected_sequence {
		return Err(TransportError::UnexpectedSequence {
			expected: expected_sequence,
			received: sequence,
		});
	}
	let raw_token = u64::from_be_bytes(header[24..32].try_into().expect("fixed header slice"));
	let token = CompletionToken::new(raw_token)?;
	let raw_schedule = u64::from_be_bytes(header[32..40].try_into().expect("fixed header slice"));
	let schedule = (raw_schedule != NO_SCHEDULE).then_some(raw_schedule);
	let metadata = FrameMetadata::new(kind, token, schedule)?;
	let sender_machine = &header[40..72];
	let sender_profile = &header[72..104];
	let receiver_machine = &header[104..136];
	let receiver_profile = &header[136..168];
	if sender_machine != identity.remote.machine.bytes()
		|| sender_profile != identity.remote.profile.bytes()
		|| receiver_machine != identity.local.machine.bytes()
		|| receiver_profile != identity.local.profile.bytes()
	{
		return Err(TransportError::UnexpectedIdentity);
	}
	let mut digest = [0_u8; 32];
	digest.copy_from_slice(&header[168..200]);
	Ok(DecodedHeader {
		metadata,
		payload_len,
		sequence,
		digest,
	})
}

pub(crate) fn payload_digest(payload: &[u8]) -> [u8; 32] {
	Sha256::digest(payload).into()
}

pub(crate) fn write_all_deadline(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> TransportResult<()> {
	while !bytes.is_empty() {
		stream.set_write_timeout(Some(remaining(deadline)?))?;
		match stream.write(bytes) {
			Ok(0) => return Err(TransportError::ConnectionClosed),
			Ok(written) => bytes = &bytes[written..],
			Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
			Err(error) => return Err(TransportError::io(&error)),
		}
	}
	Ok(())
}

pub(crate) fn read_exact_deadline(
	stream: &mut TcpStream,
	mut bytes: &mut [u8],
	deadline: Instant,
) -> TransportResult<()> {
	while !bytes.is_empty() {
		stream.set_read_timeout(Some(remaining(deadline)?))?;
		match stream.read(bytes) {
			Ok(0) => return Err(TransportError::ConnectionClosed),
			Ok(read) => {
				let (_, rest) = bytes.split_at_mut(read);
				bytes = rest;
			}
			Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
			Err(error) => return Err(TransportError::io(&error)),
		}
	}
	Ok(())
}

fn remaining(deadline: Instant) -> TransportResult<Duration> {
	deadline
		.checked_duration_since(Instant::now())
		.filter(|duration| !duration.is_zero())
		.ok_or(TransportError::DeadlineExceeded)
}

#[cfg(test)]
pub(crate) mod tests {
	use std::net::{TcpListener, TcpStream};
	use std::thread;

	use super::*;

	pub(crate) fn digest(byte: u8) -> Digest {
		Digest::new([byte; 32])
	}

	pub(crate) fn identities() -> (SessionIdentity, SessionIdentity) {
		let left = EndpointIdentity::new(digest(1), digest(2)).unwrap();
		let right = EndpointIdentity::new(digest(3), digest(4)).unwrap();
		let left_session = SessionIdentity::new(left, right).unwrap();
		(left_session, left_session.reversed())
	}

	pub(crate) fn tcp_pair() -> (TcpStream, TcpStream) {
		let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
		let address = listener.local_addr().unwrap();
		let connector = thread::spawn(move || TcpStream::connect(address).unwrap());
		let (accepted, _) = listener.accept().unwrap();
		(connector.join().unwrap(), accepted)
	}

	#[test]
	fn deterministic_round_trip_and_exact_identity() {
		let (left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(1024, Duration::from_secs(1)).unwrap();
		let (mut sender, _) = split_stream(left_stream, left_identity, limits).unwrap();
		let (_, mut receiver) = split_stream(right_stream, right_identity, limits).unwrap();
		let metadata = FrameMetadata::new(FrameKind::Control, CompletionToken::new(7).unwrap(), None).unwrap();
		sender.send(metadata, b"hello", limits.deadline()).unwrap();
		let mut buffer = [0_u8; 16];
		let received = receiver
			.receive_into(&mut buffer, limits.deadline())
			.unwrap();
		assert_eq!(received.metadata, metadata);
		assert_eq!(received.payload, b"hello");
	}

	#[test]
	fn rejects_identity_mismatch_and_poison_follows() {
		let (left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let wrong_local = EndpointIdentity::new(digest(9), digest(10)).unwrap();
		let wrong_identity = SessionIdentity::new(wrong_local, right_identity.remote()).unwrap();
		let limits = ProtocolLimits::new(1024, Duration::from_secs(1)).unwrap();
		let (mut sender, _) = split_stream(left_stream, left_identity, limits).unwrap();
		let (_, mut receiver) = split_stream(right_stream, wrong_identity, limits).unwrap();
		let metadata = FrameMetadata::new(FrameKind::Control, CompletionToken::new(7).unwrap(), None).unwrap();
		sender.send(metadata, b"x", limits.deadline()).unwrap();
		let mut buffer = [0_u8; 4];
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::UnexpectedIdentity)
		);
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::Poisoned)
		);
	}

	#[test]
	fn rejects_oversize_before_reading_payload() {
		let (mut left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(4, Duration::from_secs(1)).unwrap();
		let metadata = FrameMetadata::new(FrameKind::Control, CompletionToken::new(1).unwrap(), None).unwrap();
		let header = encode_header(left_identity, 0, metadata, 5, payload_digest(b"12345")).unwrap();
		left_stream.write_all(&header).unwrap();
		let (_, mut receiver) = split_stream(right_stream, right_identity, limits).unwrap();
		let mut buffer = [0_u8; 8];
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::FrameTooLarge {
				declared: 5,
				limit: 4
			})
		);
	}

	#[test]
	fn rejects_replay_and_corruption() {
		let (mut left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(32, Duration::from_secs(1)).unwrap();
		let metadata = FrameMetadata::new(FrameKind::Control, CompletionToken::new(1).unwrap(), None).unwrap();
		let header = encode_header(left_identity, 1, metadata, 1, payload_digest(b"a")).unwrap();
		left_stream.write_all(&header).unwrap();
		left_stream.write_all(b"a").unwrap();
		let (_, mut receiver) = split_stream(right_stream, right_identity, limits).unwrap();
		let mut buffer = [0_u8; 8];
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::UnexpectedSequence {
				expected: 0,
				received: 1
			})
		);

		let (mut left_stream, right_stream) = tcp_pair();
		let header = encode_header(left_identity, 0, metadata, 1, payload_digest(b"a")).unwrap();
		left_stream.write_all(&header).unwrap();
		left_stream.write_all(b"b").unwrap();
		let (_, mut receiver) = split_stream(right_stream, right_identity, limits).unwrap();
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::IntegrityFailure)
		);
	}

	#[test]
	fn rejects_nonzero_reserved_and_invalid_schedule_lane() {
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(32, Duration::from_secs(1)).unwrap();
		let metadata = FrameMetadata::new(FrameKind::Control, CompletionToken::new(1).unwrap(), None).unwrap();
		let mut header = encode_header(left_identity, 0, metadata, 0, payload_digest(&[])).unwrap();
		header[11] = 1;
		assert_eq!(
			decode_header(&header, right_identity, limits, 0),
			Err(TransportError::InvalidFrame("reserved flags are nonzero"))
		);
		assert_eq!(
			FrameMetadata::new(
				FrameKind::Control,
				CompletionToken::new(1).unwrap(),
				Some(4)
			),
			Err(TransportError::InvalidFrame(
				"only user-data frames may carry a schedule position"
			))
		);
	}

	#[test]
	fn absolute_deadline_rejects_a_silent_peer() {
		let (left_stream, right_stream) = tcp_pair();
		let (_left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(32, Duration::from_millis(20)).unwrap();
		let (_, mut receiver) = split_stream(right_stream, right_identity, limits).unwrap();
		let mut buffer = [0_u8; 8];
		assert_eq!(
			receiver.receive_into(&mut buffer, limits.deadline()),
			Err(TransportError::DeadlineExceeded)
		);
		drop(left_stream);
	}
}

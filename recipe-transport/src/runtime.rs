use std::io::{self, Read, Write};
use std::net::TcpStream;

use crate::error::{TransportError, TransportResult};
use crate::protocol::{
	CompletionToken, DecodedHeader, FrameKind, FrameMetadata, HEADER_BYTES, NO_SCHEDULE, ProtocolLimits,
	ReceivedFrame, SessionIdentity, decode_header, encode_header, payload_digest,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeLane {
	Control,
	Metrics,
	UserData,
}

impl RuntimeLane {
	const fn frame_kind(self) -> FrameKind {
		match self {
			Self::Control => FrameKind::Control,
			Self::Metrics => FrameKind::Metrics,
			Self::UserData => FrameKind::UserData,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScheduleStamp(u64);

impl ScheduleStamp {
	pub fn new(static_position: u64) -> TransportResult<Self> {
		if static_position == NO_SCHEDULE {
			return Err(TransportError::InvalidConfiguration(
				"schedule position collides with the unscheduled sentinel",
			));
		}
		Ok(Self(static_position))
	}

	#[must_use]
	pub const fn get(self) -> u64 {
		self.0
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelCapacities {
	control_slots: usize,
	metrics_slots: usize,
	user_data_slots: usize,
	max_payload: usize,
}

impl ChannelCapacities {
	pub fn new(
		control_slots: usize,
		metrics_slots: usize,
		user_data_slots: usize,
		max_payload: usize,
	) -> TransportResult<Self> {
		if control_slots == 0 || metrics_slots == 0 || user_data_slots == 0 {
			return Err(TransportError::InvalidConfiguration(
				"every runtime lane requires at least one fixed slot",
			));
		}
		if max_payload == 0 {
			return Err(TransportError::InvalidConfiguration(
				"runtime maximum payload must be nonzero",
			));
		}
		Ok(Self {
			control_slots,
			metrics_slots,
			user_data_slots,
			max_payload,
		})
	}

	#[must_use]
	pub const fn max_payload(self) -> usize {
		self.max_payload
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submission {
	pub token: CompletionToken,
	pub lane: RuntimeLane,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionState {
	Queued,
	Writing,
	Complete,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
	pub transmitted: Option<CompletionToken>,
	pub received: Option<CompletionToken>,
	pub advanced: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotState {
	Free,
	Queued,
	Writing,
	Complete,
}

#[derive(Debug)]
struct TxSlot {
	state: SlotState,
	metadata: Option<FrameMetadata>,
	header: [u8; HEADER_BYTES],
	header_written: usize,
	payload: Box<[u8]>,
	payload_len: usize,
	payload_written: usize,
	digest: [u8; 32],
	order: u64,
}

impl TxSlot {
	fn new(max_payload: usize) -> Self {
		Self {
			state: SlotState::Free,
			metadata: None,
			header: [0; HEADER_BYTES],
			header_written: 0,
			payload: vec![0; max_payload].into_boxed_slice(),
			payload_len: 0,
			payload_written: 0,
			digest: [0; 32],
			order: 0,
		}
	}

	fn reset(&mut self) {
		self.state = SlotState::Free;
		self.metadata = None;
		self.header.fill(0);
		self.header_written = 0;
		self.payload[..self.payload_len].fill(0);
		self.payload_len = 0;
		self.payload_written = 0;
		self.digest.fill(0);
		self.order = 0;
	}
}

#[derive(Debug)]
struct FixedLane {
	name: &'static str,
	slots: Box<[TxSlot]>,
}

impl FixedLane {
	fn new(name: &'static str, count: usize, max_payload: usize) -> Self {
		let slots = (0..count)
			.map(|_| TxSlot::new(max_payload))
			.collect::<Vec<_>>()
			.into_boxed_slice();
		Self { name, slots }
	}

	fn submit(&mut self, metadata: FrameMetadata, payload: &[u8], order: u64) -> TransportResult<usize> {
		let index = self
			.slots
			.iter()
			.position(|slot| slot.state == SlotState::Free)
			.ok_or(TransportError::CapacityExhausted(self.name))?;
		let slot = &mut self.slots[index];
		slot.payload[..payload.len()].copy_from_slice(payload);
		slot.payload_len = payload.len();
		slot.digest = payload_digest(payload);
		slot.metadata = Some(metadata);
		slot.order = order;
		slot.state = SlotState::Queued;
		Ok(index)
	}

	fn oldest_queued(&self) -> Option<usize> {
		self.slots
			.iter()
			.enumerate()
			.filter(|(_, slot)| slot.state == SlotState::Queued)
			.min_by_key(|(_, slot)| slot.order)
			.map(|(index, _)| index)
	}

	fn find_token(&self, token: CompletionToken) -> Option<usize> {
		self.slots.iter().position(|slot| {
			slot.metadata
				.is_some_and(|metadata| metadata.token == token)
		})
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SlotKey {
	lane: RuntimeLane,
	index: usize,
}

#[derive(Debug)]
struct ReceiveBuffer {
	header: [u8; HEADER_BYTES],
	header_read: usize,
	decoded: Option<DecodedHeader>,
	payload: Box<[u8]>,
	payload_read: usize,
	ready: bool,
}

impl ReceiveBuffer {
	fn new(max_payload: usize) -> Self {
		Self {
			header: [0; HEADER_BYTES],
			header_read: 0,
			decoded: None,
			payload: vec![0; max_payload].into_boxed_slice(),
			payload_read: 0,
			ready: false,
		}
	}

	fn reset(&mut self) {
		self.header.fill(0);
		self.header_read = 0;
		if let Some(decoded) = self.decoded {
			self.payload[..decoded.payload_len].fill(0);
		}
		self.decoded = None;
		self.payload_read = 0;
		self.ready = false;
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Activity {
	token: Option<CompletionToken>,
	advanced: bool,
}

/// Preallocated, nonblocking runtime transport.
///
/// Construction allocates every transmit slot and the receive buffer. The
/// submission, progress, completion, receive, and release paths perform no
/// heap allocation. Completed transmit slots remain unavailable until the
/// caller explicitly releases their completion token; received storage remains
/// unavailable until the caller explicitly releases that token.
#[derive(Debug)]
pub struct RuntimeChannel {
	stream: TcpStream,
	identity: SessionIdentity,
	limits: ProtocolLimits,
	control: FixedLane,
	metrics: FixedLane,
	user_data: FixedLane,
	receive: ReceiveBuffer,
	active: Option<SlotKey>,
	next_token: u64,
	next_order: u64,
	tx_sequence: u64,
	rx_sequence: u64,
	last_submitted_schedule: Option<ScheduleStamp>,
	last_received_schedule: Option<ScheduleStamp>,
	poisoned: bool,
}

impl RuntimeChannel {
	pub fn new(
		stream: TcpStream,
		identity: SessionIdentity,
		limits: ProtocolLimits,
		capacities: ChannelCapacities,
	) -> TransportResult<Self> {
		if capacities.max_payload > limits.max_payload() {
			return Err(TransportError::InvalidConfiguration(
				"runtime payload capacity exceeds protocol limit",
			));
		}
		stream.set_nonblocking(true)?;
		stream.set_nodelay(true)?;
		Ok(Self {
			stream,
			identity,
			limits,
			control: FixedLane::new(
				"control lane",
				capacities.control_slots,
				capacities.max_payload,
			),
			metrics: FixedLane::new(
				"metrics lane",
				capacities.metrics_slots,
				capacities.max_payload,
			),
			user_data: FixedLane::new(
				"user-data lane",
				capacities.user_data_slots,
				capacities.max_payload,
			),
			receive: ReceiveBuffer::new(capacities.max_payload),
			active: None,
			next_token: 1,
			next_order: 0,
			tx_sequence: 0,
			rx_sequence: 0,
			last_submitted_schedule: None,
			last_received_schedule: None,
			poisoned: false,
		})
	}

	/// Exact endpoint/profile identity bound into every frame header.
	#[must_use]
	pub const fn identity(&self) -> SessionIdentity {
		self.identity
	}

	/// First schedule position legal for the next outbound user-data frame.
	///
	/// This remains monotonic when a connected channel is reused for another
	/// prepared run.
	pub fn next_outbound_schedule_position(&self) -> TransportResult<u64> {
		next_schedule_position(self.last_submitted_schedule)
	}

	/// First schedule position legal for the next inbound user-data frame.
	///
	/// Application protocols use this to translate a new run's logical
	/// positions onto the connection-global schedule sequence.
	pub fn next_inbound_schedule_position(&self) -> TransportResult<u64> {
		next_schedule_position(self.last_received_schedule)
	}

	pub fn submit_control(&mut self, payload: &[u8]) -> TransportResult<Submission> {
		self.submit(RuntimeLane::Control, None, payload)
	}

	pub fn submit_metrics(&mut self, payload: &[u8]) -> TransportResult<Submission> {
		self.submit(RuntimeLane::Metrics, None, payload)
	}

	pub fn submit_user_data(&mut self, schedule: ScheduleStamp, payload: &[u8]) -> TransportResult<Submission> {
		if self
			.last_submitted_schedule
			.is_some_and(|previous| schedule <= previous)
		{
			return Err(TransportError::ProtocolState(
				"user-data schedule positions must be strictly increasing",
			));
		}
		let submission = self.submit(RuntimeLane::UserData, Some(schedule), payload)?;
		self.last_submitted_schedule = Some(schedule);
		Ok(submission)
	}

	fn submit(
		&mut self,
		lane: RuntimeLane,
		schedule: Option<ScheduleStamp>,
		payload: &[u8],
	) -> TransportResult<Submission> {
		self.ensure_healthy()?;
		if payload.len() > self.receive.payload.len() {
			return Err(TransportError::FrameTooLarge {
				declared: payload.len(),
				limit: self.receive.payload.len(),
			});
		}
		let token = CompletionToken::new(self.next_token)?;
		let metadata = FrameMetadata::new(lane.frame_kind(), token, schedule.map(ScheduleStamp::get))?;
		let order = self.next_order;
		match lane {
			RuntimeLane::Control => self.control.submit(metadata, payload, order)?,
			RuntimeLane::Metrics => self.metrics.submit(metadata, payload, order)?,
			RuntimeLane::UserData => self.user_data.submit(metadata, payload, order)?,
		};
		self.next_token = self
			.next_token
			.checked_add(1)
			.ok_or(TransportError::ProtocolState(
				"completion token space exhausted",
			))?;
		self.next_order = self
			.next_order
			.checked_add(1)
			.ok_or(TransportError::ProtocolState(
				"submission order space exhausted",
			))?;
		Ok(Submission { token, lane })
	}

	pub fn progress(&mut self) -> TransportResult<Progress> {
		self.ensure_healthy()?;
		let transmit = self.progress_write();
		let receive = self.progress_read();
		match (transmit, receive) {
			(Ok(transmit), Ok(receive)) => Ok(Progress {
				transmitted: transmit.token,
				received: receive.token,
				advanced: transmit.advanced || receive.advanced,
			}),
			(Err(error), _) | (_, Err(error)) => {
				self.poisoned = true;
				Err(error)
			}
		}
	}

	pub fn completion_state(&self, token: CompletionToken) -> TransportResult<CompletionState> {
		let slot = self.find_slot(token)?;
		let state = match self.slot(slot).state {
			SlotState::Queued => CompletionState::Queued,
			SlotState::Writing => CompletionState::Writing,
			SlotState::Complete => CompletionState::Complete,
			SlotState::Free => return Err(TransportError::UnknownCompletion(token.get())),
		};
		Ok(state)
	}

	pub fn release_completion(&mut self, token: CompletionToken) -> TransportResult<()> {
		let key = self.find_slot(token)?;
		if self.slot(key).state != SlotState::Complete {
			return Err(TransportError::ProtocolState(
				"transmit completion cannot be released before it is complete",
			));
		}
		self.slot_mut(key).reset();
		Ok(())
	}

	#[must_use]
	pub fn received(&self) -> Option<ReceivedFrame<'_>> {
		if !self.receive.ready {
			return None;
		}
		let decoded = self.receive.decoded?;
		Some(ReceivedFrame {
			metadata: decoded.metadata,
			payload: &self.receive.payload[..decoded.payload_len],
		})
	}

	pub fn release_received(&mut self, token: CompletionToken) -> TransportResult<()> {
		let decoded = self.receive.decoded.ok_or(TransportError::ProtocolState(
			"no received frame is pending",
		))?;
		if !self.receive.ready || decoded.metadata.token != token {
			return Err(TransportError::ProtocolState(
				"received completion token does not match the pending frame",
			));
		}
		self.receive.reset();
		Ok(())
	}

	fn progress_write(&mut self) -> TransportResult<Activity> {
		if self.active.is_none() {
			self.active = self.next_queued();
			if let Some(key) = self.active {
				let sequence = self.tx_sequence;
				let identity = self.identity;
				let slot = self.slot_mut(key);
				let metadata = slot
					.metadata
					.ok_or(TransportError::ProtocolState("queued slot has no metadata"))?;
				slot.header = encode_header(identity, sequence, metadata, slot.payload_len, slot.digest)?;
				slot.state = SlotState::Writing;
			}
		}
		let Some(key) = self.active else {
			return Ok(Activity::default());
		};
		let result = self.write_active(key);
		match result {
			Ok(activity) => Ok(activity),
			Err(error) => Err(error),
		}
	}

	fn write_active(&mut self, key: SlotKey) -> TransportResult<Activity> {
		let header_pending = self.slot(key).header_written < HEADER_BYTES;
		if header_pending {
			let written = {
				let (stream, slot) = self.stream_and_slot_mut(key);
				stream.write(&slot.header[slot.header_written..])
			};
			return self.record_header_write(key, written);
		}
		let payload_pending = self.slot(key).payload_written < self.slot(key).payload_len;
		if payload_pending {
			let written = {
				let (stream, slot) = self.stream_and_slot_mut(key);
				stream.write(&slot.payload[slot.payload_written..slot.payload_len])
			};
			return self.record_payload_write(key, written);
		}
		self.finish_active(key)
	}

	fn record_header_write(&mut self, key: SlotKey, result: io::Result<usize>) -> TransportResult<Activity> {
		match result {
			Ok(0) => Err(TransportError::ConnectionClosed),
			Ok(written) => {
				self.slot_mut(key).header_written += written;
				if self.slot(key).header_written == HEADER_BYTES && self.slot(key).payload_len == 0 {
					self.finish_active(key)
				} else {
					Ok(Activity {
						token: None,
						advanced: true,
					})
				}
			}
			Err(error)
				if matches!(
					error.kind(),
					io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
				) =>
			{
				Ok(Activity::default())
			}
			Err(error) => Err(TransportError::io(&error)),
		}
	}

	fn record_payload_write(&mut self, key: SlotKey, result: io::Result<usize>) -> TransportResult<Activity> {
		match result {
			Ok(0) => Err(TransportError::ConnectionClosed),
			Ok(written) => {
				self.slot_mut(key).payload_written += written;
				if self.slot(key).payload_written == self.slot(key).payload_len {
					self.finish_active(key)
				} else {
					Ok(Activity {
						token: None,
						advanced: true,
					})
				}
			}
			Err(error)
				if matches!(
					error.kind(),
					io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
				) =>
			{
				Ok(Activity::default())
			}
			Err(error) => Err(TransportError::io(&error)),
		}
	}

	fn finish_active(&mut self, key: SlotKey) -> TransportResult<Activity> {
		let token = self
			.slot(key)
			.metadata
			.ok_or(TransportError::ProtocolState("active slot has no metadata"))?
			.token;
		self.slot_mut(key).state = SlotState::Complete;
		self.active = None;
		self.tx_sequence = self
			.tx_sequence
			.checked_add(1)
			.ok_or(TransportError::ProtocolState("transmit sequence exhausted"))?;
		Ok(Activity {
			token: Some(token),
			advanced: true,
		})
	}

	fn progress_read(&mut self) -> TransportResult<Activity> {
		if self.receive.ready {
			return Ok(Activity {
				token: self.receive.decoded.map(|decoded| decoded.metadata.token),
				advanced: false,
			});
		}
		if self.receive.header_read < HEADER_BYTES {
			let result = self
				.stream
				.read(&mut self.receive.header[self.receive.header_read..]);
			match result {
				Ok(0) => return Err(TransportError::ConnectionClosed),
				Ok(read) => {
					self.receive.header_read += read;
					if self.receive.header_read == HEADER_BYTES {
						let decoded = decode_header(
							&self.receive.header,
							self.identity,
							self.limits,
							self.rx_sequence,
						)?;
						if !decoded.metadata.kind.is_runtime() {
							return Err(TransportError::InvalidFrame(
								"probe frame is not valid on a runtime channel",
							));
						}
						if let Some(schedule) = decoded.metadata.schedule {
							let stamp = ScheduleStamp::new(schedule)?;
							if self
								.last_received_schedule
								.is_some_and(|previous| stamp <= previous)
							{
								return Err(TransportError::ProtocolState(
									"received user-data schedule is not strictly increasing",
								));
							}
						}
						self.receive.decoded = Some(decoded);
						if decoded.payload_len == 0 {
							return self.finish_receive();
						}
					}
					return Ok(Activity {
						token: None,
						advanced: true,
					});
				}
				Err(error)
					if matches!(
						error.kind(),
						io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
					) =>
				{
					return Ok(Activity::default());
				}
				Err(error) => return Err(TransportError::io(&error)),
			}
		}
		let decoded = self.receive.decoded.ok_or(TransportError::ProtocolState(
			"complete header was not decoded",
		))?;
		let result = self
			.stream
			.read(&mut self.receive.payload[self.receive.payload_read..decoded.payload_len]);
		match result {
			Ok(0) => Err(TransportError::ConnectionClosed),
			Ok(read) => {
				self.receive.payload_read += read;
				if self.receive.payload_read == decoded.payload_len {
					self.finish_receive()
				} else {
					Ok(Activity {
						token: None,
						advanced: true,
					})
				}
			}
			Err(error)
				if matches!(
					error.kind(),
					io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
				) =>
			{
				Ok(Activity::default())
			}
			Err(error) => Err(TransportError::io(&error)),
		}
	}

	fn finish_receive(&mut self) -> TransportResult<Activity> {
		let decoded = self.receive.decoded.ok_or(TransportError::ProtocolState(
			"receive has no decoded header",
		))?;
		let payload = &self.receive.payload[..decoded.payload_len];
		if payload_digest(payload) != decoded.digest {
			return Err(TransportError::IntegrityFailure);
		}
		if let Some(schedule) = decoded.metadata.schedule {
			self.last_received_schedule = Some(ScheduleStamp::new(schedule)?);
		}
		self.receive.ready = true;
		self.rx_sequence = self
			.rx_sequence
			.checked_add(1)
			.ok_or(TransportError::ProtocolState("receive sequence exhausted"))?;
		Ok(Activity {
			token: Some(decoded.metadata.token),
			advanced: true,
		})
	}

	fn next_queued(&self) -> Option<SlotKey> {
		self.control
			.oldest_queued()
			.map(|index| SlotKey {
				lane: RuntimeLane::Control,
				index,
			})
			.or_else(|| {
				self.user_data.oldest_queued().map(|index| SlotKey {
					lane: RuntimeLane::UserData,
					index,
				})
			})
			.or_else(|| {
				self.metrics.oldest_queued().map(|index| SlotKey {
					lane: RuntimeLane::Metrics,
					index,
				})
			})
	}

	fn find_slot(&self, token: CompletionToken) -> TransportResult<SlotKey> {
		for lane in [
			RuntimeLane::Control,
			RuntimeLane::Metrics,
			RuntimeLane::UserData,
		] {
			let index = match lane {
				RuntimeLane::Control => self.control.find_token(token),
				RuntimeLane::Metrics => self.metrics.find_token(token),
				RuntimeLane::UserData => self.user_data.find_token(token),
			};
			if let Some(index) = index {
				return Ok(SlotKey { lane, index });
			}
		}
		Err(TransportError::UnknownCompletion(token.get()))
	}

	const fn slot(&self, key: SlotKey) -> &TxSlot {
		match key.lane {
			RuntimeLane::Control => &self.control.slots[key.index],
			RuntimeLane::Metrics => &self.metrics.slots[key.index],
			RuntimeLane::UserData => &self.user_data.slots[key.index],
		}
	}

	fn slot_mut(&mut self, key: SlotKey) -> &mut TxSlot {
		match key.lane {
			RuntimeLane::Control => &mut self.control.slots[key.index],
			RuntimeLane::Metrics => &mut self.metrics.slots[key.index],
			RuntimeLane::UserData => &mut self.user_data.slots[key.index],
		}
	}

	fn stream_and_slot_mut(&mut self, key: SlotKey) -> (&mut TcpStream, &mut TxSlot) {
		match key.lane {
			RuntimeLane::Control => (&mut self.stream, &mut self.control.slots[key.index]),
			RuntimeLane::Metrics => (&mut self.stream, &mut self.metrics.slots[key.index]),
			RuntimeLane::UserData => (&mut self.stream, &mut self.user_data.slots[key.index]),
		}
	}

	fn ensure_healthy(&self) -> TransportResult<()> {
		if self.poisoned {
			Err(TransportError::Poisoned)
		} else {
			Ok(())
		}
	}
}

fn next_schedule_position(last: Option<ScheduleStamp>) -> TransportResult<u64> {
	let next = match last {
		Some(last) => last
			.get()
			.checked_add(1)
			.ok_or(TransportError::ProtocolState(
				"user-data schedule positions exhausted",
			))?,
		None => 0,
	};
	ScheduleStamp::new(next)?;
	Ok(next)
}

#[cfg(test)]
mod tests {
	use std::time::{Duration, Instant};

	use super::*;
	use crate::protocol::tests::{identities, tcp_pair};

	fn channels() -> (RuntimeChannel, RuntimeChannel) {
		let (left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(64, Duration::from_secs(1)).unwrap();
		let capacities = ChannelCapacities::new(2, 2, 2, 64).unwrap();
		(
			RuntimeChannel::new(left_stream, left_identity, limits, capacities).unwrap(),
			RuntimeChannel::new(right_stream, right_identity, limits, capacities).unwrap(),
		)
	}

	fn drive_until_received<'a>(sender: &mut RuntimeChannel, receiver: &'a mut RuntimeChannel) -> ReceivedFrame<'a> {
		let deadline = Instant::now() + Duration::from_secs(2);
		loop {
			sender.progress().unwrap();
			receiver.progress().unwrap();
			if receiver.received().is_some() {
				return receiver.received().unwrap();
			}
			assert!(Instant::now() < deadline);
			std::thread::yield_now();
		}
	}

	#[test]
	fn lanes_are_separate_prioritized_and_explicitly_completed() {
		let (mut left, mut right) = channels();
		let data = left
			.submit_user_data(ScheduleStamp::new(7).unwrap(), b"data")
			.unwrap();
		let metrics = left.submit_metrics(b"metrics").unwrap();
		let control = left.submit_control(b"control").unwrap();

		let received = drive_until_received(&mut left, &mut right);
		assert_eq!(received.metadata.kind, FrameKind::Control);
		assert_eq!(received.payload, b"control");
		let received_token = received.metadata.token;
		right.release_received(received_token).unwrap();
		assert_eq!(
			left.completion_state(control.token).unwrap(),
			CompletionState::Complete
		);
		left.release_completion(control.token).unwrap();

		let received = drive_until_received(&mut left, &mut right);
		assert_eq!(received.metadata.kind, FrameKind::UserData);
		assert_eq!(received.metadata.schedule, Some(7));
		let received_token = received.metadata.token;
		right.release_received(received_token).unwrap();
		left.release_completion(data.token).unwrap();

		let received = drive_until_received(&mut left, &mut right);
		assert_eq!(received.metadata.kind, FrameKind::Metrics);
		let received_token = received.metadata.token;
		right.release_received(received_token).unwrap();
		left.release_completion(metrics.token).unwrap();
	}

	#[test]
	fn fixed_capacity_requires_explicit_release() {
		let (mut left, mut right) = channels();
		let first = left.submit_control(b"one").unwrap();
		let second = left.submit_control(b"two").unwrap();
		assert_eq!(
			left.submit_control(b"three"),
			Err(TransportError::CapacityExhausted("control lane"))
		);
		let received_token = {
			let received = drive_until_received(&mut left, &mut right);
			received.metadata.token
		};
		right.release_received(received_token).unwrap();
		assert_eq!(
			left.submit_control(b"still-full"),
			Err(TransportError::CapacityExhausted("control lane"))
		);
		left.release_completion(first.token).unwrap();
		assert!(left.submit_control(b"reused").is_ok());
		assert_ne!(first.token, second.token);
	}

	#[test]
	fn scheduled_user_data_must_be_monotonic() {
		let (mut left, _right) = channels();
		left.submit_user_data(ScheduleStamp::new(5).unwrap(), b"a")
			.unwrap();
		assert_eq!(
			left.submit_user_data(ScheduleStamp::new(5).unwrap(), b"b"),
			Err(TransportError::ProtocolState(
				"user-data schedule positions must be strictly increasing"
			))
		);
		assert_eq!(
			left.submit_user_data(ScheduleStamp::new(4).unwrap(), b"c"),
			Err(TransportError::ProtocolState(
				"user-data schedule positions must be strictly increasing"
			))
		);
	}

	#[test]
	fn release_requires_matching_completed_token() {
		let (mut left, _right) = channels();
		let queued = left.submit_metrics(b"x").unwrap();
		assert_eq!(
			left.release_completion(queued.token),
			Err(TransportError::ProtocolState(
				"transmit completion cannot be released before it is complete"
			))
		);
		assert_eq!(
			left.release_received(queued.token),
			Err(TransportError::ProtocolState(
				"no received frame is pending"
			))
		);
	}
}

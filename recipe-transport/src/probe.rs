use std::net::TcpStream;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use recipe_core::{ByteCount, BytesPerSecond, Property, PropertyProvenance};
use recipe_probe::{
	BoundedBenchmarkPlan, DirectionalBenchmarkEvidence, LinkDuplex, PEER_BENCHMARK_PROTOCOL_SCHEMA,
	PeerBenchmarkAttempt, PeerBenchmarkControl, PeerBenchmarkEvidence, PeerBenchmarkFailure,
	PeerBenchmarkFailureKind, PeerBenchmarkPhase, PeerDescriptor, PeerDuplexExecution, PeerEndpointEvidence,
	PeerMeasurement, PeerSession, ProbeResult,
};

use crate::error::{TransportError, TransportResult};
use crate::protocol::{
	CompletionToken, FrameKind, FrameMetadata, ProtocolLimits, SessionIdentity, WireReceiver, WireSender,
	split_stream,
};

const PROBE_PAYLOAD_SCHEMA: u16 = PEER_BENCHMARK_PROTOCOL_SCHEMA;
const BEGIN_BYTES: usize = 40;
const COMPLETE_BYTES: usize = 16;
const MAXIMUM_ITERATIONS: u32 = 1_000_000;
const MAXIMUM_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasuredLocalMemory {
	pub capacity: ByteCount,
	pub transfer_rate: BytesPerSecond,
}

impl MeasuredLocalMemory {
	pub fn new(capacity: ByteCount, transfer_rate: BytesPerSecond) -> TransportResult<Self> {
		if capacity == ByteCount::ZERO {
			return Err(TransportError::InvalidConfiguration(
				"measured local memory capacity must be nonzero",
			));
		}
		Ok(Self {
			capacity,
			transfer_rate,
		})
	}
}

#[derive(Debug)]
struct ProbeConnection {
	sender: WireSender,
	receiver: WireReceiver,
	identity: SessionIdentity,
	limits: ProtocolLimits,
	next_round: u32,
}

#[derive(Debug)]
pub struct TcpPeerSession {
	descriptor: PeerDescriptor,
	local_memory: MeasuredLocalMemory,
	connection: Mutex<ProbeConnection>,
}

impl TcpPeerSession {
	pub fn new(
		stream: TcpStream,
		identity: SessionIdentity,
		limits: ProtocolLimits,
		descriptor: PeerDescriptor,
		local_memory: MeasuredLocalMemory,
	) -> TransportResult<Self> {
		if !descriptor.asynchronous_submission {
			return Err(TransportError::InvalidConfiguration(
				"peer descriptor must declare asynchronous submission",
			));
		}
		let (sender, receiver) = split_stream(stream, identity, limits)?;
		Ok(Self {
			descriptor,
			local_memory,
			connection: Mutex::new(ProbeConnection {
				sender,
				receiver,
				identity,
				limits,
				next_round: 1,
			}),
		})
	}

	fn run_benchmark(
		&self,
		plan: BoundedBenchmarkPlan,
		control: &PeerBenchmarkControl,
	) -> Result<PeerMeasurement, PeerBenchmarkFailure> {
		check_control(control, PeerBenchmarkPhase::Validation)?;
		validate_plan(plan).map_err(|error| benchmark_failure(PeerBenchmarkPhase::Validation, error))?;
		let attempt_deadline = Instant::now()
			.checked_add(plan.maximum_duration)
			.unwrap_or(control.absolute_deadline())
			.min(control.absolute_deadline());
		let bounded_control = PeerBenchmarkControl::new(attempt_deadline, control.cancellation().clone());
		let control = &bounded_control;
		let mut connection = lock_connection(&self.connection, control)?;
		if plan.buffer_bytes.get() > connection.limits.max_payload() as u64 {
			return Err(benchmark_failure(
				PeerBenchmarkPhase::Validation,
				TransportError::FrameTooLarge {
					declared: usize::try_from(plan.buffer_bytes.get()).unwrap_or(usize::MAX),
					limit: connection.limits.max_payload(),
				},
			));
		}
		let round = connection.next_round;
		connection.next_round = round
			.checked_add(1)
			.ok_or(TransportError::ProtocolState("probe round exhausted"))
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::Validation, error))?;

		check_control(control, PeerBenchmarkPhase::BeginExchange)?;
		let local_begin = encode_begin(plan, self.descriptor.duplex, self.local_memory)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::BeginExchange, error))?;
		let begin_token =
			token(round, 1, 0).map_err(|error| benchmark_failure(PeerBenchmarkPhase::BeginExchange, error))?;
		let begin_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::BeginExchange,
		)?;
		send_exact(
			&mut connection.sender,
			FrameKind::ProbeBegin,
			begin_token,
			&local_begin,
			begin_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::BeginExchange, error))?;
		check_control(control, PeerBenchmarkPhase::BeginExchange)?;
		let mut begin_buffer = [0_u8; BEGIN_BYTES];
		let begin_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::BeginExchange,
		)?;
		let remote_begin = receive_exact(
			&mut connection.receiver,
			FrameKind::ProbeBegin,
			begin_token,
			&mut begin_buffer,
			begin_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::BeginExchange, error))?;
		let remote_memory = decode_and_validate_begin(remote_begin, plan, self.descriptor.duplex)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::BeginExchange, error))?;

		check_control(control, PeerBenchmarkPhase::ReadyExchange)?;
		let ready_token =
			token(round, 2, 0).map_err(|error| benchmark_failure(PeerBenchmarkPhase::ReadyExchange, error))?;
		let ready_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::ReadyExchange,
		)?;
		send_exact(
			&mut connection.sender,
			FrameKind::ProbeReady,
			ready_token,
			&[],
			ready_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::ReadyExchange, error))?;
		check_control(control, PeerBenchmarkPhase::ReadyExchange)?;
		let mut empty = [];
		let ready_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::ReadyExchange,
		)?;
		receive_exact(
			&mut connection.receiver,
			FrameKind::ProbeReady,
			ready_token,
			&mut empty,
			ready_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::ReadyExchange, error))?;

		check_control(control, PeerBenchmarkPhase::DirectionalTransfer)?;
		let payload_len = usize::try_from(plan.buffer_bytes.get())
			.map_err(|_| TransportError::InvalidConfiguration("probe buffer does not fit address space"))
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		let payload = vec![probe_pattern(round); payload_len];
		let measurement_deadline = Instant::now()
			.checked_add(plan.maximum_duration)
			.unwrap_or(control.absolute_deadline())
			.min(control.absolute_deadline());
		let operation_timeout = connection.limits.operation_timeout();
		let identity = connection.identity;
		let ProbeConnection {
			sender, receiver, ..
		} = &mut *connection;
		let (outbound_evidence, inbound_evidence) = match self.descriptor.duplex {
			LinkDuplex::Full => measure_full_duplex(
				sender,
				receiver,
				round,
				plan.iterations,
				&payload,
				measurement_deadline,
				operation_timeout,
				control,
			),
			LinkDuplex::Half => measure_half_duplex(
				sender,
				receiver,
				identity,
				round,
				plan.iterations,
				&payload,
				measurement_deadline,
				operation_timeout,
				control,
			),
		}
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		check_control(control, PeerBenchmarkPhase::DirectionalTransfer)?;
		let total_bytes = total_bytes(plan)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		let outbound_evidence = outbound_evidence
			.finish(total_bytes)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		let inbound_evidence = inbound_evidence
			.finish(total_bytes)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		let outbound_rate = measured_rate(total_bytes, outbound_evidence.elapsed_nanoseconds)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;
		let inbound_rate = measured_rate(total_bytes, inbound_evidence.elapsed_nanoseconds)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::DirectionalTransfer, error))?;

		check_control(control, PeerBenchmarkPhase::CompletionExchange)?;
		let complete = encode_complete(outbound_rate, inbound_rate);
		let complete_token = token(round, 4, 0)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::CompletionExchange, error))?;
		let completion_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::CompletionExchange,
		)?;
		send_exact(
			&mut connection.sender,
			FrameKind::ProbeComplete,
			complete_token,
			&complete,
			completion_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::CompletionExchange, error))?;
		check_control(control, PeerBenchmarkPhase::CompletionExchange)?;
		let mut complete_buffer = [0_u8; COMPLETE_BYTES];
		let completion_deadline = operation_deadline(
			connection.limits,
			control,
			PeerBenchmarkPhase::CompletionExchange,
		)?;
		let remote_complete = receive_exact(
			&mut connection.receiver,
			FrameKind::ProbeComplete,
			complete_token,
			&mut complete_buffer,
			completion_deadline,
		)
		.map_err(|error| benchmark_failure(PeerBenchmarkPhase::CompletionExchange, error))?;
		validate_complete(remote_complete)
			.map_err(|error| benchmark_failure(PeerBenchmarkPhase::CompletionExchange, error))?;
		check_control(control, PeerBenchmarkPhase::CompletionExchange)?;

		Ok(PeerMeasurement {
			remote_memory_capacity: measured(remote_memory.capacity),
			remote_memory_rate: measured(remote_memory.transfer_rate),
			outbound_rate: Some(measured(outbound_rate)),
			inbound_rate: Some(measured(inbound_rate)),
			evidence: PeerBenchmarkEvidence {
				protocol_schema: PROBE_PAYLOAD_SCHEMA,
				local_endpoint: endpoint_evidence(identity.local()),
				remote_endpoint: endpoint_evidence(identity.remote()),
				execution: match self.descriptor.duplex {
					LinkDuplex::Full => PeerDuplexExecution::Simultaneous,
					LinkDuplex::Half => PeerDuplexExecution::Serialized,
				},
				outbound: outbound_evidence,
				inbound: inbound_evidence,
			},
		})
	}
}

const fn endpoint_evidence(endpoint: crate::protocol::EndpointIdentity) -> PeerEndpointEvidence {
	PeerEndpointEvidence {
		machine: endpoint.machine(),
		profile: endpoint.profile(),
	}
}

impl PeerSession for TcpPeerSession {
	fn descriptor(&self) -> ProbeResult<PeerDescriptor> {
		Ok(self.descriptor.clone())
	}

	fn benchmark(&self, plan: BoundedBenchmarkPlan) -> ProbeResult<PeerMeasurement> {
		let control = match PeerBenchmarkControl::for_plan(plan) {
			Ok(control) => control,
			Err(error) => return Err(error),
		};
		self.benchmark_controlled(plan, &control).into_measurement()
	}

	fn benchmark_controlled(
		&self,
		plan: BoundedBenchmarkPlan,
		control: &PeerBenchmarkControl,
	) -> PeerBenchmarkAttempt {
		match self.run_benchmark(plan, control) {
			Ok(measurement) => PeerBenchmarkAttempt::Measured(measurement),
			Err(failure) => PeerBenchmarkAttempt::Failed(failure),
		}
	}
}

fn lock_connection<'a>(
	connection: &'a Mutex<ProbeConnection>,
	control: &PeerBenchmarkControl,
) -> Result<std::sync::MutexGuard<'a, ProbeConnection>, PeerBenchmarkFailure> {
	loop {
		check_control(control, PeerBenchmarkPhase::Validation)?;
		match connection.try_lock() {
			Ok(connection) => return Ok(connection),
			Err(std::sync::TryLockError::WouldBlock) => thread::yield_now(),
			Err(std::sync::TryLockError::Poisoned(_)) => {
				return Err(benchmark_failure(
					PeerBenchmarkPhase::Validation,
					TransportError::Poisoned,
				));
			}
		}
	}
}

fn check_control(control: &PeerBenchmarkControl, phase: PeerBenchmarkPhase) -> Result<(), PeerBenchmarkFailure> {
	match control.failure(phase) {
		Some(failure) => Err(failure),
		None => Ok(()),
	}
}

fn operation_deadline(
	limits: ProtocolLimits,
	control: &PeerBenchmarkControl,
	phase: PeerBenchmarkPhase,
) -> Result<Instant, PeerBenchmarkFailure> {
	check_control(control, phase)?;
	Ok(Instant::now()
		.checked_add(limits.operation_timeout())
		.unwrap_or(control.absolute_deadline())
		.min(control.absolute_deadline()))
}

fn benchmark_failure(phase: PeerBenchmarkPhase, error: TransportError) -> PeerBenchmarkFailure {
	let kind = match error {
		TransportError::Cancelled => PeerBenchmarkFailureKind::Cancelled,
		TransportError::DeadlineExceeded => PeerBenchmarkFailureKind::Deadline,
		TransportError::InvalidIdentity(_) | TransportError::UnexpectedIdentity => {
			PeerBenchmarkFailureKind::Identity
		}
		TransportError::IntegrityFailure => PeerBenchmarkFailureKind::Integrity,
		TransportError::UnsupportedVersion(_)
		| TransportError::InvalidFrame(_)
		| TransportError::UnexpectedSequence { .. }
		| TransportError::ProtocolState(_)
		| TransportError::UnknownCompletion(_) => PeerBenchmarkFailureKind::Protocol,
		TransportError::InvalidConfiguration(_)
		| TransportError::FrameTooLarge { .. }
		| TransportError::BufferTooSmall { .. }
		| TransportError::ConnectionClosed
		| TransportError::Io(_)
		| TransportError::CapacityExhausted(_)
		| TransportError::Probe(_)
		| TransportError::Poisoned => PeerBenchmarkFailureKind::Transport,
	};
	PeerBenchmarkFailure::new(phase, kind, error.to_string())
}

fn validate_plan(plan: BoundedBenchmarkPlan) -> TransportResult<()> {
	if !plan.is_bounded() {
		return Err(TransportError::InvalidConfiguration(
			"peer benchmark plan must be bounded and nonzero",
		));
	}
	if plan.iterations > MAXIMUM_ITERATIONS {
		return Err(TransportError::InvalidConfiguration(
			"peer benchmark iteration limit exceeded",
		));
	}
	total_bytes(plan)?;
	Ok(())
}

fn total_bytes(plan: BoundedBenchmarkPlan) -> TransportResult<u64> {
	plan.buffer_bytes
		.get()
		.checked_mul(u64::from(plan.iterations))
		.filter(|bytes| *bytes <= MAXIMUM_TOTAL_BYTES)
		.ok_or(TransportError::InvalidConfiguration(
			"peer benchmark total byte limit exceeded",
		))
}

fn encode_begin(
	plan: BoundedBenchmarkPlan,
	duplex: LinkDuplex,
	memory: MeasuredLocalMemory,
) -> TransportResult<[u8; BEGIN_BYTES]> {
	let duration_nanos = u64::try_from(plan.maximum_duration.as_nanos())
		.map_err(|_| TransportError::InvalidConfiguration("benchmark duration is too large"))?;
	let mut payload = [0_u8; BEGIN_BYTES];
	payload[0..2].copy_from_slice(&PROBE_PAYLOAD_SCHEMA.to_be_bytes());
	payload[2] = match duplex {
		LinkDuplex::Full => 1,
		LinkDuplex::Half => 2,
	};
	payload[3] = 0;
	payload[4..12].copy_from_slice(&plan.buffer_bytes.get().to_be_bytes());
	payload[12..16].copy_from_slice(&plan.iterations.to_be_bytes());
	payload[16..24].copy_from_slice(&duration_nanos.to_be_bytes());
	payload[24..32].copy_from_slice(&memory.capacity.get().to_be_bytes());
	payload[32..40].copy_from_slice(&memory.transfer_rate.get().to_be_bytes());
	Ok(payload)
}

fn decode_and_validate_begin(
	payload: &[u8],
	expected_plan: BoundedBenchmarkPlan,
	expected_duplex: LinkDuplex,
) -> TransportResult<MeasuredLocalMemory> {
	if payload.len() != BEGIN_BYTES {
		return Err(TransportError::InvalidFrame(
			"wrong probe-begin payload length",
		));
	}
	let schema = read_u16(payload, 0)?;
	if schema != PROBE_PAYLOAD_SCHEMA {
		return Err(TransportError::UnsupportedVersion(schema));
	}
	if payload[3] != 0 {
		return Err(TransportError::InvalidFrame(
			"probe-begin reserved byte is nonzero",
		));
	}
	let duplex = match payload[2] {
		1 => LinkDuplex::Full,
		2 => LinkDuplex::Half,
		_ => return Err(TransportError::InvalidFrame("invalid probe duplex value")),
	};
	if duplex != expected_duplex {
		return Err(TransportError::InvalidFrame(
			"peer duplex declaration disagrees",
		));
	}
	let remote_plan = BoundedBenchmarkPlan {
		buffer_bytes: ByteCount::new(read_u64(payload, 4)?),
		iterations: read_u32(payload, 12)?,
		maximum_duration: Duration::from_nanos(read_u64(payload, 16)?),
	};
	if remote_plan != expected_plan {
		return Err(TransportError::InvalidFrame(
			"peer benchmark plan disagrees",
		));
	}
	let capacity = ByteCount::new(read_u64(payload, 24)?);
	let rate =
		BytesPerSecond::new(read_u64(payload, 32)?).map_err(|error| TransportError::Probe(error.to_string()))?;
	MeasuredLocalMemory::new(capacity, rate)
}

#[derive(Clone, Copy, Debug)]
struct DirectionAccumulator {
	started: Instant,
	sample_count: u32,
	minimum_nanoseconds: u64,
	maximum_nanoseconds: u64,
	sum_nanoseconds: u128,
	sum_squared_nanoseconds: u128,
}

impl DirectionAccumulator {
	fn new() -> Self {
		Self {
			started: Instant::now(),
			sample_count: 0,
			minimum_nanoseconds: u64::MAX,
			maximum_nanoseconds: 0,
			sum_nanoseconds: 0,
			sum_squared_nanoseconds: 0,
		}
	}

	fn record(&mut self, duration: Duration) -> TransportResult<()> {
		let nanoseconds = canonical_nanoseconds(duration)?;
		self.sample_count = self
			.sample_count
			.checked_add(1)
			.ok_or(TransportError::ProtocolState(
				"probe sample count exhausted",
			))?;
		self.minimum_nanoseconds = self.minimum_nanoseconds.min(nanoseconds);
		self.maximum_nanoseconds = self.maximum_nanoseconds.max(nanoseconds);
		self.sum_nanoseconds = self
			.sum_nanoseconds
			.checked_add(u128::from(nanoseconds))
			.ok_or_else(|| TransportError::Probe("probe duration sum overflowed".to_owned()))?;
		self.sum_squared_nanoseconds = self
			.sum_squared_nanoseconds
			.checked_add(u128::from(nanoseconds) * u128::from(nanoseconds))
			.ok_or_else(|| TransportError::Probe("probe duration variance overflowed".to_owned()))?;
		Ok(())
	}

	fn finish(self, total_bytes: u64) -> TransportResult<DirectionalBenchmarkEvidence> {
		if self.sample_count == 0 {
			return Err(TransportError::Probe(
				"peer benchmark produced no directional samples".to_owned(),
			));
		}
		let elapsed_nanoseconds = canonical_nanoseconds(self.started.elapsed())?;
		let count = u128::from(self.sample_count);
		let mean = self.sum_nanoseconds / count;
		let variance = (self.sum_squared_nanoseconds / count).saturating_sub(mean * mean);
		Ok(DirectionalBenchmarkEvidence {
			total_bytes: ByteCount::new(total_bytes),
			elapsed_nanoseconds,
			sample_count: self.sample_count,
			minimum_sample_nanoseconds: self.minimum_nanoseconds,
			maximum_sample_nanoseconds: self.maximum_nanoseconds,
			mean_sample_nanoseconds: u64::try_from(mean)
				.map_err(|_| TransportError::Probe("probe mean duration does not fit u64".to_owned()))?,
			variance_nanoseconds_squared: variance,
		})
	}
}

fn canonical_nanoseconds(duration: Duration) -> TransportResult<u64> {
	u64::try_from(duration.as_nanos().max(1))
		.map_err(|_| TransportError::Probe("probe duration does not fit u64 nanoseconds".to_owned()))
}

fn measure_full_duplex(
	sender: &mut WireSender,
	receiver: &mut WireReceiver,
	round: u32,
	iterations: u32,
	payload: &[u8],
	phase_deadline: Instant,
	operation_timeout: Duration,
	control: &PeerBenchmarkControl,
) -> TransportResult<(DirectionAccumulator, DirectionAccumulator)> {
	let barrier = Arc::new(Barrier::new(2));
	thread::scope(|scope| {
		let receive_barrier = Arc::clone(&barrier);
		let receiver_task = scope.spawn(move || {
			receive_barrier.wait();
			measure_receive(
				receiver,
				round,
				iterations,
				payload.len(),
				phase_deadline,
				operation_timeout,
				control,
			)
		});
		barrier.wait();
		let outbound = measure_send(
			sender,
			round,
			iterations,
			payload,
			phase_deadline,
			operation_timeout,
			control,
		);
		let inbound = receiver_task
			.join()
			.map_err(|_| TransportError::Probe("receive benchmark thread panicked".to_owned()))?;
		match (outbound, inbound) {
			(Ok(outbound), Ok(inbound)) => Ok((outbound, inbound)),
			(Err(error), _) | (_, Err(error)) => Err(error),
		}
	})
}

fn measure_half_duplex(
	sender: &mut WireSender,
	receiver: &mut WireReceiver,
	identity: SessionIdentity,
	round: u32,
	iterations: u32,
	payload: &[u8],
	phase_deadline: Instant,
	operation_timeout: Duration,
	control: &PeerBenchmarkControl,
) -> TransportResult<(DirectionAccumulator, DirectionAccumulator)> {
	let local_is_first = identity.local().machine().bytes() < identity.remote().machine().bytes();
	if local_is_first {
		let outbound = measure_send(
			sender,
			round,
			iterations,
			payload,
			phase_deadline,
			operation_timeout,
			control,
		)?;
		let inbound = measure_receive(
			receiver,
			round,
			iterations,
			payload.len(),
			phase_deadline,
			operation_timeout,
			control,
		)?;
		Ok((outbound, inbound))
	} else {
		let inbound = measure_receive(
			receiver,
			round,
			iterations,
			payload.len(),
			phase_deadline,
			operation_timeout,
			control,
		)?;
		let outbound = measure_send(
			sender,
			round,
			iterations,
			payload,
			phase_deadline,
			operation_timeout,
			control,
		)?;
		Ok((outbound, inbound))
	}
}

fn measure_send(
	sender: &mut WireSender,
	round: u32,
	iterations: u32,
	payload: &[u8],
	phase_deadline: Instant,
	operation_timeout: Duration,
	control: &PeerBenchmarkControl,
) -> TransportResult<DirectionAccumulator> {
	let mut measurements = DirectionAccumulator::new();
	for iteration in 0..iterations {
		let deadline = transfer_operation_deadline(control, phase_deadline, operation_timeout)?;
		let sample_started = Instant::now();
		send_exact(
			sender,
			FrameKind::ProbeData,
			token(round, 3, iteration)?,
			payload,
			deadline,
		)?;
		measurements.record(sample_started.elapsed())?;
	}
	transfer_control_check(control, phase_deadline)?;
	Ok(measurements)
}

fn measure_receive(
	receiver: &mut WireReceiver,
	round: u32,
	iterations: u32,
	payload_len: usize,
	phase_deadline: Instant,
	operation_timeout: Duration,
	control: &PeerBenchmarkControl,
) -> TransportResult<DirectionAccumulator> {
	let mut payload = vec![0_u8; payload_len];
	let mut measurements = DirectionAccumulator::new();
	for iteration in 0..iterations {
		let deadline = transfer_operation_deadline(control, phase_deadline, operation_timeout)?;
		let sample_started = Instant::now();
		let received = receive_exact(
			receiver,
			FrameKind::ProbeData,
			token(round, 3, iteration)?,
			&mut payload,
			deadline,
		)?;
		if received.iter().any(|byte| *byte != probe_pattern(round)) {
			return Err(TransportError::IntegrityFailure);
		}
		measurements.record(sample_started.elapsed())?;
	}
	transfer_control_check(control, phase_deadline)?;
	Ok(measurements)
}

fn transfer_operation_deadline(
	control: &PeerBenchmarkControl,
	phase_deadline: Instant,
	operation_timeout: Duration,
) -> TransportResult<Instant> {
	transfer_control_check(control, phase_deadline)?;
	let hard_deadline = phase_deadline.min(control.absolute_deadline());
	Ok(Instant::now()
		.checked_add(operation_timeout)
		.unwrap_or(hard_deadline)
		.min(hard_deadline))
}

fn transfer_control_check(control: &PeerBenchmarkControl, phase_deadline: Instant) -> TransportResult<()> {
	if control.cancellation().is_cancelled() {
		Err(TransportError::Cancelled)
	} else if Instant::now() >= phase_deadline.min(control.absolute_deadline()) {
		Err(TransportError::DeadlineExceeded)
	} else {
		Ok(())
	}
}

fn send_exact(
	sender: &mut WireSender,
	kind: FrameKind,
	token: CompletionToken,
	payload: &[u8],
	deadline: Instant,
) -> TransportResult<()> {
	sender.send(FrameMetadata::new(kind, token, None)?, payload, deadline)
}

fn receive_exact<'a>(
	receiver: &mut WireReceiver,
	expected_kind: FrameKind,
	expected_token: CompletionToken,
	buffer: &'a mut [u8],
	deadline: Instant,
) -> TransportResult<&'a [u8]> {
	let received = receiver.receive_into(buffer, deadline)?;
	if received.metadata.kind != expected_kind || received.metadata.token != expected_token {
		return Err(TransportError::InvalidFrame(
			"probe frame kind or completion token is out of phase",
		));
	}
	Ok(received.payload)
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn measured_rate(bytes: u64, nanoseconds: u64) -> TransportResult<BytesPerSecond> {
	let rate = u128::from(bytes)
		.checked_mul(1_000_000_000)
		.and_then(|scaled| scaled.checked_div(u128::from(nanoseconds)))
		.ok_or(TransportError::Probe(
			"directional rate calculation overflowed".to_owned(),
		))?
		.clamp(1, u128::from(u64::MAX));
	BytesPerSecond::new(
		u64::try_from(rate)
			.map_err(|_| TransportError::Probe("directional rate does not fit the profile schema".to_owned()))?,
	)
	.map_err(|error| TransportError::Probe(error.to_string()))
}

fn token(round: u32, phase: u8, iteration: u32) -> TransportResult<CompletionToken> {
	let value = (u64::from(round) << 32) | (u64::from(phase) << 24) | u64::from(iteration + 1);
	CompletionToken::new(value)
}

const fn probe_pattern(round: u32) -> u8 {
	round.to_le_bytes()[0] ^ 0xa5
}

fn encode_complete(outbound: BytesPerSecond, inbound: BytesPerSecond) -> [u8; COMPLETE_BYTES] {
	let mut payload = [0_u8; COMPLETE_BYTES];
	payload[0..8].copy_from_slice(&outbound.get().to_be_bytes());
	payload[8..16].copy_from_slice(&inbound.get().to_be_bytes());
	payload
}

fn validate_complete(payload: &[u8]) -> TransportResult<()> {
	if payload.len() != COMPLETE_BYTES {
		return Err(TransportError::InvalidFrame(
			"wrong probe-complete payload length",
		));
	}
	BytesPerSecond::new(read_u64(payload, 0)?).map_err(|error| {
		TransportError::Probe(format!(
			"peer reported invalid outbound completion: {error}"
		))
	})?;
	BytesPerSecond::new(read_u64(payload, 8)?)
		.map_err(|error| TransportError::Probe(format!("peer reported invalid inbound completion: {error}")))?;
	Ok(())
}

fn read_u16(payload: &[u8], offset: usize) -> TransportResult<u16> {
	let bytes = payload
		.get(offset..offset + 2)
		.ok_or(TransportError::InvalidFrame("truncated integer"))?;
	Ok(u16::from_be_bytes(bytes.try_into().map_err(|_| {
		TransportError::InvalidFrame("truncated integer")
	})?))
}

fn read_u32(payload: &[u8], offset: usize) -> TransportResult<u32> {
	let bytes = payload
		.get(offset..offset + 4)
		.ok_or(TransportError::InvalidFrame("truncated integer"))?;
	Ok(u32::from_be_bytes(bytes.try_into().map_err(|_| {
		TransportError::InvalidFrame("truncated integer")
	})?))
}

fn read_u64(payload: &[u8], offset: usize) -> TransportResult<u64> {
	let bytes = payload
		.get(offset..offset + 8)
		.ok_or(TransportError::InvalidFrame("truncated integer"))?;
	Ok(u64::from_be_bytes(bytes.try_into().map_err(|_| {
		TransportError::InvalidFrame("truncated integer")
	})?))
}

#[cfg(test)]
mod tests {
	use std::time::{Duration, Instant};

	use recipe_core::{ByteCount, BytesPerSecond, Label, PropertyProvenance, TransferLaneCount, TransportKind};
	use recipe_probe::{
		BoundedBenchmarkPlan, MachineFingerprint, PeerBenchmarkCancellation, PeerDescriptor, PeerSession,
	};

	use super::*;
	use crate::protocol::tests::{identities, tcp_pair};

	fn label(value: &str) -> Label {
		Label::new(value).unwrap()
	}

	fn fingerprint(name: &str) -> MachineFingerprint {
		MachineFingerprint {
			hostname: label(name),
			stable_id: label(&format!("{name}-stable")),
			runtime_abi: label("linux-x86_64"),
			firmware: label("fixture-firmware"),
		}
	}

	fn descriptor(remote: &str, duplex: LinkDuplex) -> PeerDescriptor {
		PeerDescriptor {
			session_id: label(&format!("session-to-{remote}")),
			machine: fingerprint(remote),
			remote_memory_key: label("remote-ram"),
			local_memory_key: label("local-ram"),
			local_interface_key: label("local-eth"),
			remote_interface_identity: label("remote-eth"),
			remote_driver: label("fixture-driver"),
			remote_firmware: label("fixture-nic-firmware"),
			link_identity: label("established-test-link"),
			transport_kind: TransportKind::Ethernet,
			duplex,
			outbound_maximum_inflight: TransferLaneCount::new(1).unwrap(),
			inbound_maximum_inflight: TransferLaneCount::new(1).unwrap(),
			asynchronous_submission: true,
		}
	}

	fn memory(capacity: u64, rate: u64) -> MeasuredLocalMemory {
		MeasuredLocalMemory::new(ByteCount::new(capacity), BytesPerSecond::new(rate).unwrap()).unwrap()
	}

	fn plan(bytes: u64) -> BoundedBenchmarkPlan {
		BoundedBenchmarkPlan {
			buffer_bytes: ByteCount::new(bytes),
			iterations: 4,
			maximum_duration: Duration::from_secs(2),
		}
	}

	fn sessions(duplex: LinkDuplex) -> (TcpPeerSession, TcpPeerSession) {
		let (left_stream, right_stream) = tcp_pair();
		let (left_identity, right_identity) = identities();
		let limits = ProtocolLimits::new(8192, Duration::from_secs(3)).unwrap();
		(
			TcpPeerSession::new(
				left_stream,
				left_identity,
				limits,
				descriptor("right", duplex),
				memory(16_000_000_000, 70_000_000_000),
			)
			.unwrap(),
			TcpPeerSession::new(
				right_stream,
				right_identity,
				limits,
				descriptor("left", duplex),
				memory(32_000_000_000, 80_000_000_000),
			)
			.unwrap(),
		)
	}

	fn run_pair(
		left: &TcpPeerSession,
		right: &TcpPeerSession,
		left_plan: BoundedBenchmarkPlan,
		right_plan: BoundedBenchmarkPlan,
	) -> (ProbeResult<PeerMeasurement>, ProbeResult<PeerMeasurement>) {
		thread::scope(|scope| {
			let left_task = scope.spawn(|| left.benchmark(left_plan));
			let right_task = scope.spawn(|| right.benchmark(right_plan));
			(left_task.join().unwrap(), right_task.join().unwrap())
		})
	}

	fn run_controlled_pair(
		left: &TcpPeerSession,
		right: &TcpPeerSession,
		left_plan: BoundedBenchmarkPlan,
		right_plan: BoundedBenchmarkPlan,
		left_control: &PeerBenchmarkControl,
		right_control: &PeerBenchmarkControl,
	) -> (PeerBenchmarkAttempt, PeerBenchmarkAttempt) {
		thread::scope(|scope| {
			let left_task = scope.spawn(|| left.benchmark_controlled(left_plan, left_control));
			let right_task = scope.spawn(|| right.benchmark_controlled(right_plan, right_control));
			(left_task.join().unwrap(), right_task.join().unwrap())
		})
	}

	fn failed(attempt: PeerBenchmarkAttempt) -> PeerBenchmarkFailure {
		match attempt {
			PeerBenchmarkAttempt::Measured(_) => panic!("benchmark unexpectedly measured"),
			PeerBenchmarkAttempt::Failed(failure) => failure,
		}
	}

	fn assert_measured(
		measurement: &PeerMeasurement,
		remote_capacity: u64,
		remote_rate: u64,
		execution: PeerDuplexExecution,
		plan: BoundedBenchmarkPlan,
	) {
		assert_eq!(
			measurement.remote_memory_capacity.value.get(),
			remote_capacity
		);
		assert_eq!(measurement.remote_memory_rate.value.get(), remote_rate);
		assert_eq!(
			measurement.remote_memory_capacity.provenance,
			PropertyProvenance::Measured
		);
		assert_eq!(
			measurement.remote_memory_rate.provenance,
			PropertyProvenance::Measured
		);
		let outbound = measurement.outbound_rate.unwrap();
		let inbound = measurement.inbound_rate.unwrap();
		assert_eq!(outbound.provenance, PropertyProvenance::Measured);
		assert_eq!(inbound.provenance, PropertyProvenance::Measured);
		assert!(outbound.value.get() > 0);
		assert!(inbound.value.get() > 0);
		assert_eq!(measurement.evidence.protocol_schema, PROBE_PAYLOAD_SCHEMA);
		assert_eq!(measurement.evidence.execution, execution);
		assert_ne!(
			measurement.evidence.local_endpoint.machine,
			measurement.evidence.remote_endpoint.machine
		);
		let expected_bytes = plan.buffer_bytes.get() * u64::from(plan.iterations);
		for evidence in [measurement.evidence.outbound, measurement.evidence.inbound] {
			assert_eq!(evidence.total_bytes.get(), expected_bytes);
			assert_eq!(evidence.sample_count, plan.iterations);
			assert_ne!(evidence.elapsed_nanoseconds, 0);
			assert!(evidence.minimum_sample_nanoseconds <= evidence.mean_sample_nanoseconds);
			assert!(evidence.mean_sample_nanoseconds <= evidence.maximum_sample_nanoseconds);
			assert!(evidence.maximum_sample_nanoseconds <= evidence.elapsed_nanoseconds);
		}
	}

	#[test]
	fn full_duplex_measures_both_directions_simultaneously() {
		let (left, right) = sessions(LinkDuplex::Full);
		let benchmark = plan(4096);
		let (left_result, right_result) = run_pair(&left, &right, benchmark, benchmark);
		assert_measured(
			&left_result.unwrap(),
			32_000_000_000,
			80_000_000_000,
			PeerDuplexExecution::Simultaneous,
			benchmark,
		);
		assert_measured(
			&right_result.unwrap(),
			16_000_000_000,
			70_000_000_000,
			PeerDuplexExecution::Simultaneous,
			benchmark,
		);
	}

	#[test]
	fn half_duplex_measures_both_directions_serially() {
		let (left, right) = sessions(LinkDuplex::Half);
		let benchmark = plan(4096);
		let (left_result, right_result) = run_pair(&left, &right, benchmark, benchmark);
		assert_measured(
			&left_result.unwrap(),
			32_000_000_000,
			80_000_000_000,
			PeerDuplexExecution::Serialized,
			benchmark,
		);
		assert_measured(
			&right_result.unwrap(),
			16_000_000_000,
			70_000_000_000,
			PeerDuplexExecution::Serialized,
			benchmark,
		);
	}

	#[test]
	fn disagreement_and_incomplete_sessions_fail_closed() {
		let (left, right) = sessions(LinkDuplex::Full);
		let (left_result, right_result) = run_pair(&left, &right, plan(4096), plan(2048));
		assert!(left_result.is_err());
		assert!(right_result.is_err());

		let (left_stream, right_stream) = tcp_pair();
		drop(right_stream);
		let (left_identity, _) = identities();
		let limits = ProtocolLimits::new(8192, Duration::from_millis(100)).unwrap();
		let left = TcpPeerSession::new(
			left_stream,
			left_identity,
			limits,
			descriptor("gone", LinkDuplex::Full),
			memory(16_000_000_000, 70_000_000_000),
		)
		.unwrap();
		assert!(left.benchmark(plan(4096)).is_err());
	}

	#[test]
	fn controlled_benchmark_reports_cancellation_before_io() {
		let (left, _right) = sessions(LinkDuplex::Full);
		let cancellation = PeerBenchmarkCancellation::default();
		cancellation.cancel();
		let control = PeerBenchmarkControl::new(Instant::now() + Duration::from_secs(1), cancellation);
		let failure = failed(left.benchmark_controlled(plan(4096), &control));
		assert_eq!(failure.phase, PeerBenchmarkPhase::Validation);
		assert_eq!(failure.kind, PeerBenchmarkFailureKind::Cancelled);
		assert_eq!(failure.protocol_schema, PROBE_PAYLOAD_SCHEMA);
	}

	#[test]
	fn controlled_benchmark_reports_expired_absolute_deadline_before_io() {
		let (left, _right) = sessions(LinkDuplex::Full);
		let control = PeerBenchmarkControl::new(Instant::now(), PeerBenchmarkCancellation::default());
		let failure = failed(left.benchmark_controlled(plan(4096), &control));
		assert_eq!(failure.phase, PeerBenchmarkPhase::Validation);
		assert_eq!(failure.kind, PeerBenchmarkFailureKind::Deadline);
	}

	#[test]
	fn controlled_benchmark_reports_protocol_disagreement_in_begin_exchange() {
		let (left, right) = sessions(LinkDuplex::Full);
		let left_control = PeerBenchmarkControl::for_plan(plan(4096)).unwrap();
		let right_control = PeerBenchmarkControl::for_plan(plan(2048)).unwrap();
		let (left_attempt, right_attempt) = run_controlled_pair(
			&left,
			&right,
			plan(4096),
			plan(2048),
			&left_control,
			&right_control,
		);
		for failure in [failed(left_attempt), failed(right_attempt)] {
			assert_eq!(failure.phase, PeerBenchmarkPhase::BeginExchange);
			assert_eq!(failure.kind, PeerBenchmarkFailureKind::Protocol);
		}
	}

	#[test]
	fn bounded_plan_deadline_bounds_a_stalled_begin_exchange() {
		let (left_stream, _idle_peer) = tcp_pair();
		let (left_identity, _) = identities();
		let limits = ProtocolLimits::new(8192, Duration::from_secs(3)).unwrap();
		let left = TcpPeerSession::new(
			left_stream,
			left_identity,
			limits,
			descriptor("idle", LinkDuplex::Full),
			memory(16_000_000_000, 70_000_000_000),
		)
		.unwrap();
		let bounded_plan = BoundedBenchmarkPlan {
			maximum_duration: Duration::from_millis(40),
			..plan(4096)
		};
		let control = PeerBenchmarkControl::new(
			Instant::now() + Duration::from_secs(3),
			PeerBenchmarkCancellation::default(),
		);
		let started = Instant::now();
		let failure = failed(left.benchmark_controlled(bounded_plan, &control));
		assert_eq!(failure.phase, PeerBenchmarkPhase::BeginExchange);
		assert_eq!(failure.kind, PeerBenchmarkFailureKind::Deadline);
		assert!(started.elapsed() < Duration::from_millis(500));
	}

	#[test]
	fn caller_absolute_deadline_bounds_a_stalled_begin_exchange() {
		let (left_stream, _idle_peer) = tcp_pair();
		let (left_identity, _) = identities();
		let limits = ProtocolLimits::new(8192, Duration::from_secs(3)).unwrap();
		let left = TcpPeerSession::new(
			left_stream,
			left_identity,
			limits,
			descriptor("idle", LinkDuplex::Full),
			memory(16_000_000_000, 70_000_000_000),
		)
		.unwrap();
		let control = PeerBenchmarkControl::new(
			Instant::now() + Duration::from_millis(40),
			PeerBenchmarkCancellation::default(),
		);
		let started = Instant::now();
		let failure = failed(left.benchmark_controlled(plan(4096), &control));
		assert_eq!(failure.phase, PeerBenchmarkPhase::BeginExchange);
		assert_eq!(failure.kind, PeerBenchmarkFailureKind::Deadline);
		assert!(started.elapsed() < Duration::from_millis(500));
	}
}

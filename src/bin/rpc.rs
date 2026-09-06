//! Ethernet worker transport for distributed Recipe runs.
//!
//! The handshake fixes one training or inference session. Scheduled payloads
//! and four-byte metrics then cross the same authenticated stream. Metrics are
//! forwarded to the worker's stdout, while cancellation is acknowledged before
//! the session socket closes.

use std::{
	env,
	error::Error,
	fmt, fs,
	io::{self, Read as _, Write as _},
	net::{Shutdown, TcpListener, TcpStream},
	path::PathBuf,
	process::ExitCode,
};

use recipe_core::{ByteCount, Digest, RunId, TaskId};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use write::{always, err, line, net};

const MAGIC: [u8; 8] = *b"RCPERPC1";
const PROTOCOL_VERSION: u16 = 1;
const HEADER_BYTES: usize = 116;
const NO_SCHEDULE: u64 = u64::MAX;
const MASTER_TRAINING: u32 = 0x0001_0001;
const WORKER_TRAINING: u32 = 0x0001_0002;
const MASTER_INFERENCE: u32 = 0x0002_0001;
const WORKER_INFERENCE: u32 = 0x0002_0002;
const FAULT_PROTOCOL: u32 = 1;
const FAULT_IO: u32 = 2;

fn main() -> ExitCode {
	match run(env::args_os().skip(1)) {
		Ok(()) => return ExitCode::SUCCESS,
		Err(error) => {
			drop(err(format!("rpc: {error}\n{}", usage())));
			return ExitCode::FAILURE;
		}
	}
}

fn run(mut arguments: impl Iterator<Item = std::ffi::OsString>) -> RpcResult<()> {
	let config_path = arguments
		.next()
		.ok_or_else(|| return RpcError::configuration("missing CONFIG_TOML"))?;
	if config_path == "-h" || config_path == "--help" {
		always(usage());
		return Ok(());
	}
	if arguments.next().is_some() {
		return Err(RpcError::configuration(
			"unexpected argument after CONFIG_TOML",
		));
	}
	let config = Config::load(PathBuf::from(config_path))?;
	let listener = TcpListener::bind(&config.rpc.listen_address).map_err(|source| {
		return RpcError::Io {
			operation: "bind worker listener",
			source,
		};
	})?;
	let bound = listener.local_addr().map_err(|source| {
		return RpcError::Io {
			operation: "read worker listener address",
			source,
		};
	})?;
	line(net, format!("rpc\tworker\tlisten\t{bound}"));
	for accepted in listener.incoming() {
		let stream = accepted.map_err(|source| {
			return RpcError::Io {
				operation: "accept worker session",
				source,
			};
		})?;
		let peer = stream.peer_addr().map_err(|source| {
			return RpcError::Io {
				operation: "read worker peer address",
				source,
			};
		})?;
		line(net, format!("rpc\tworker\tpeer\t{peer}"));
		if let Err(error) = WorkerSession::accept(stream, config.rpc.max_payload_bytes).and_then(WorkerSession::serve) {
			drop(err(format!("rpc\tworker\tpeer\t{peer}\terror\t{error}")));
		}
	}
	return Ok(());
}

const fn usage() -> &'static str {
	return "usage: rpc CONFIG_TOML\n\
	 CONFIG_TOML requires [rpc], listen_address, and max_payload_bytes";
}

type RpcResult<T> = Result<T, RpcError>;

#[derive(Debug)]
enum RpcError {
	Configuration(String),
	Protocol(&'static str),
	Io {
		operation: &'static str,
		source: io::Error,
	},
}

impl RpcError {
	fn configuration(detail: impl Into<String>) -> Self { return Self::Configuration(detail.into()); }

	const fn protocol(detail: &'static str) -> Self { return Self::Protocol(detail); }

	const fn fault_code(&self) -> u32 {
		match self {
			Self::Configuration(_) | Self::Protocol(_) => return FAULT_PROTOCOL,
			Self::Io { .. } => return FAULT_IO,
		}
	}
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
	rpc: RpcConfig,
}

impl Config {
	fn load(path: PathBuf) -> RpcResult<Self> {
		let source = fs::read_to_string(&path).map_err(|error| {
			return RpcError::configuration(format!(
				"read RPC configuration {}: {error}",
				path.display()
			));
		})?;
		let config: Self = toml::from_str(&source).map_err(|error| {
			return RpcError::configuration(format!(
				"parse RPC configuration {}: {error}",
				path.display()
			));
		})?;
		if config.rpc.listen_address.trim().is_empty() {
			return Err(RpcError::configuration(
				"rpc.listen_address must not be empty",
			));
		}
		if config.rpc.max_payload_bytes == 0 || config.rpc.max_payload_bytes > u32::MAX as usize {
			return Err(RpcError::configuration(
				"rpc.max_payload_bytes must fit a nonzero wire u32",
			));
		}
		return Ok(config);
	}
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RpcConfig {
	listen_address: String,
	max_payload_bytes: usize,
}

impl fmt::Display for RpcError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Configuration(detail) => return write!(f, "invalid configuration: {detail}"),
			Self::Protocol(detail) => return write!(f, "invalid wire state: {detail}"),
			Self::Io { operation, source } => return write!(f, "{operation}: {source}"),
		}
	}
}

impl Error for RpcError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Io { source, .. } => return Some(source),
			Self::Configuration(_) | Self::Protocol(_) => return None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum FrameKind {
	Hello = 1,
	HelloAck = 2,
	Transfer = 3,
	TransferAck = 4,
	MetricF32 = 5,
	MetricI32 = 6,
	MetricAck = 7,
	Cancel = 8,
	CancelAck = 9,
	Complete = 10,
	CompleteAck = 11,
	Fault = 12,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkloadKind {
	Training,
	Inference,
}

impl WorkloadKind {
	const fn from_master_code(code: u32) -> RpcResult<Self> {
		match code {
			MASTER_TRAINING => return Ok(Self::Training),
			MASTER_INFERENCE => return Ok(Self::Inference),
			_ => return Err(RpcError::protocol("hello role or workload is invalid")),
		}
	}

	const fn worker_code(self) -> u32 {
		match self {
			Self::Training => return WORKER_TRAINING,
			Self::Inference => return WORKER_INFERENCE,
		}
	}
}

impl fmt::Display for WorkloadKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Training => return f.write_str("training"),
			Self::Inference => return f.write_str("inference"),
		}
	}
}

impl FrameKind {
	const fn decode(tag: u8) -> RpcResult<Self> {
		match tag {
			1 => return Ok(Self::Hello),
			2 => return Ok(Self::HelloAck),
			3 => return Ok(Self::Transfer),
			4 => return Ok(Self::TransferAck),
			5 => return Ok(Self::MetricF32),
			6 => return Ok(Self::MetricI32),
			7 => return Ok(Self::MetricAck),
			8 => return Ok(Self::Cancel),
			9 => return Ok(Self::CancelAck),
			10 => return Ok(Self::Complete),
			11 => return Ok(Self::CompleteAck),
			12 => return Ok(Self::Fault),
			_ => return Err(RpcError::protocol("unknown frame kind")),
		}
	}

	const fn tag(self) -> u8 { return self as u8; }
}

#[derive(Clone, Debug)]
struct Frame {
	kind: FrameKind,
	sequence: u64,
	run: RunId,
	task: TaskId,
	schedule: Option<u64>,
	code: u32,
	session: Digest,
	payload: Vec<u8>,
}

impl Frame {
	const fn hello_ack(hello: &Self, workload: WorkloadKind) -> Self {
		return Self {
			kind: FrameKind::HelloAck,
			sequence: hello.sequence,
			run: hello.run,
			task: TaskId::new(0),
			schedule: None,
			code: workload.worker_code(),
			session: hello.session,
			payload: Vec::new(),
		};
	}

	const fn acknowledgment(&self, kind: FrameKind, sequence: u64) -> Self {
		return Self {
			kind,
			sequence,
			run: self.run,
			task: self.task,
			schedule: self.schedule,
			code: self.code,
			session: self.session,
			payload: Vec::new(),
		};
	}

	fn fault(&self, sequence: u64, error: &RpcError) -> Self {
		return Self {
			kind: FrameKind::Fault,
			sequence,
			run: self.run,
			task: self.task,
			schedule: None,
			code: error.fault_code(),
			session: self.session,
			payload: error.to_string().into_bytes(),
		};
	}
}

#[derive(Debug)]
struct WorkerSession {
	stream: TcpStream,
	run: RunId,
	identity: Digest,
	next_receive_sequence: u64,
	next_send_sequence: u64,
	last_schedule: Option<u64>,
	max_payload_bytes: usize,
}

impl WorkerSession {
	fn accept(mut stream: TcpStream, max_payload_bytes: usize) -> RpcResult<Self> {
		stream.set_nodelay(true).map_err(|source| {
			return RpcError::Io {
				operation: "enable TCP_NODELAY",
				source,
			};
		})?;
		let hello = read_frame(&mut stream, max_payload_bytes)?;
		let workload = validate_hello(&hello)?;
		write_frame(
			&mut stream,
			&Frame::hello_ack(&hello, workload),
			max_payload_bytes,
		)?;
		line(
			net,
			format!(
				"rpc\tsession\trun\t{}\tworkload\t{workload}\tidentity\t{}",
				hello.run,
				digest_text(hello.session)
			),
		);
		return Ok(Self {
			stream,
			run: hello.run,
			identity: hello.session,
			next_receive_sequence: 1,
			next_send_sequence: 1,
			last_schedule: None,
			max_payload_bytes,
		});
	}

	fn serve(mut self) -> RpcResult<()> {
		loop {
			let frame = read_frame(&mut self.stream, self.max_payload_bytes)?;
			if let Err(error) = self
				.validate_common(&frame)
				.and_then(|()| return self.dispatch(&frame))
			{
				let fault = frame.fault(self.take_send_sequence()?, &error);
				let _ignored = write_frame(&mut self.stream, &fault, self.max_payload_bytes);
				let _ignored = self.stream.shutdown(Shutdown::Both);
				return Err(error);
			}
			self.next_receive_sequence = self
				.next_receive_sequence
				.checked_add(1)
				.ok_or_else(|| return RpcError::protocol("receive sequence exhausted"))?;
			match frame.kind {
				FrameKind::Transfer => self.acknowledge(&frame, FrameKind::TransferAck)?,
				FrameKind::MetricF32 | FrameKind::MetricI32 => {
					self.acknowledge(&frame, FrameKind::MetricAck)?;
				}
				FrameKind::Cancel => {
					self.acknowledge(&frame, FrameKind::CancelAck)?;
					self.stream.shutdown(Shutdown::Both).map_err(|source| {
						return RpcError::Io {
							operation: "close cancelled session",
							source,
						};
					})?;
					return Ok(());
				}
				FrameKind::Complete => {
					self.acknowledge(&frame, FrameKind::CompleteAck)?;
					self.stream.shutdown(Shutdown::Both).map_err(|source| {
						return RpcError::Io {
							operation: "close completed session",
							source,
						};
					})?;
					return Ok(());
				}
				FrameKind::Hello | FrameKind::HelloAck | FrameKind::TransferAck | FrameKind::MetricAck | FrameKind::CancelAck | FrameKind::CompleteAck | FrameKind::Fault => {
					return Err(RpcError::protocol("worker received a response-only frame"));
				}
			}
		}
	}

	fn validate_common(&mut self, frame: &Frame) -> RpcResult<()> {
		if frame.sequence != self.next_receive_sequence {
			return Err(RpcError::protocol(
				"frame sequence is replayed or out of order",
			));
		}
		if frame.run != self.run {
			return Err(RpcError::protocol(
				"frame run ID differs from the handshake",
			));
		}
		if frame.session != self.identity {
			return Err(RpcError::protocol(
				"frame session identity differs from the handshake",
			));
		}
		return Ok(());
	}

	fn dispatch(&mut self, frame: &Frame) -> RpcResult<()> {
		match frame.kind {
			FrameKind::Transfer => return self.receive_transfer(frame),
			FrameKind::MetricF32 => return self.forward_f32_metric(frame),
			FrameKind::MetricI32 => return self.forward_i32_metric(frame),
			FrameKind::Cancel => return self.receive_cancel(frame),
			FrameKind::Complete => return self.receive_complete(frame),
			FrameKind::Hello | FrameKind::HelloAck | FrameKind::TransferAck | FrameKind::MetricAck | FrameKind::CancelAck | FrameKind::CompleteAck | FrameKind::Fault => {
				return Err(RpcError::protocol(
					"frame is invalid in an active worker session",
				));
			}
		}
	}

	fn receive_transfer(&mut self, frame: &Frame) -> RpcResult<()> {
		if frame.task.get() == 0 {
			return Err(RpcError::protocol("transfer requires a nonzero task ID"));
		}
		let schedule = frame
			.schedule
			.ok_or_else(|| return RpcError::protocol("transfer requires a schedule stamp"))?;
		if self
			.last_schedule
			.is_some_and(|last| return schedule <= last)
		{
			return Err(RpcError::protocol("transfer schedule stamps must increase"));
		}
		if frame.payload.is_empty() {
			return Err(RpcError::protocol("transfer payload must not be empty"));
		}
		let bytes = ByteCount::new(u64::try_from(frame.payload.len()).map_err(|_error| return RpcError::protocol("transfer length does not fit ByteCount"))?);
		self.last_schedule = Some(schedule);
		line(
			net,
			format!(
				"rpc\ttransfer\trun\t{}\ttask\t{}\tschedule\t{schedule}\tbytes\t{}",
				frame.run,
				frame.task,
				bytes.get()
			),
		);
		return Ok(());
	}

	fn forward_f32_metric(&self, frame: &Frame) -> RpcResult<()> {
		let bytes: [u8; 4] = frame
			.payload
			.as_slice()
			.try_into()
			.map_err(|_error| return RpcError::protocol("f32 metric payload must be four bytes"))?;
		self.validate_metric(frame)?;
		line(
			net,
			format!(
				"rpc\tmetric\trun\t{}\ttask\t{}\tf32\t{}",
				frame.run,
				frame.task,
				f32::from_bits(u32::from_le_bytes(bytes))
			),
		);
		return Ok(());
	}

	fn forward_i32_metric(&self, frame: &Frame) -> RpcResult<()> {
		let bytes: [u8; 4] = frame
			.payload
			.as_slice()
			.try_into()
			.map_err(|_error| return RpcError::protocol("i32 metric payload must be four bytes"))?;
		self.validate_metric(frame)?;
		line(
			net,
			format!(
				"rpc\tmetric\trun\t{}\ttask\t{}\ti32\t{}",
				frame.run,
				frame.task,
				i32::from_le_bytes(bytes)
			),
		);
		return Ok(());
	}

	const fn validate_metric(&self, frame: &Frame) -> RpcResult<()> {
		if frame.task.get() == 0 {
			return Err(RpcError::protocol("metric requires a nonzero task ID"));
		}
		if frame.schedule.is_some() {
			return Err(RpcError::protocol(
				"metric cannot carry a transfer schedule stamp",
			));
		}
		return Ok(());
	}

	fn receive_cancel(&self, frame: &Frame) -> RpcResult<()> {
		if frame.task.get() != 0 || frame.schedule.is_some() || !frame.payload.is_empty() {
			return Err(RpcError::protocol(
				"cancel must contain only its reason code",
			));
		}
		line(
			net,
			format!("rpc\tcancel\trun\t{}\treason\t{}", frame.run, frame.code),
		);
		return Ok(());
	}

	fn receive_complete(&self, frame: &Frame) -> RpcResult<()> {
		if frame.task.get() != 0 || frame.schedule.is_some() || frame.code != 0 || !frame.payload.is_empty() {
			return Err(RpcError::protocol(
				"complete frame must not contain task state",
			));
		}
		line(net, format!("rpc\tcomplete\trun\t{}", frame.run));
		return Ok(());
	}

	fn acknowledge(&mut self, frame: &Frame, kind: FrameKind) -> RpcResult<()> {
		let sequence = self.take_send_sequence()?;
		return write_frame(
			&mut self.stream,
			&frame.acknowledgment(kind, sequence),
			self.max_payload_bytes,
		);
	}

	fn take_send_sequence(&mut self) -> RpcResult<u64> {
		let sequence = self.next_send_sequence;
		self.next_send_sequence = sequence
			.checked_add(1)
			.ok_or_else(|| return RpcError::protocol("send sequence exhausted"))?;
		return Ok(sequence);
	}
}

fn validate_hello(frame: &Frame) -> RpcResult<WorkloadKind> {
	if frame.kind != FrameKind::Hello {
		return Err(RpcError::protocol("first frame must be hello"));
	}
	if frame.sequence != 0 {
		return Err(RpcError::protocol("hello sequence must be zero"));
	}
	if frame.run.get() == 0 {
		return Err(RpcError::protocol("hello run ID must be nonzero"));
	}
	if frame.task.get() != 0 || frame.schedule.is_some() || !frame.payload.is_empty() {
		return Err(RpcError::protocol("hello contains runtime state"));
	}
	let workload = WorkloadKind::from_master_code(frame.code)?;
	if frame.session.is_zero() {
		return Err(RpcError::protocol("hello session identity must be nonzero"));
	}
	return Ok(workload);
}

fn read_frame(stream: &mut TcpStream, max_payload_bytes: usize) -> RpcResult<Frame> {
	let mut header = [0u8; HEADER_BYTES];
	stream.read_exact(&mut header).map_err(|source| {
		return RpcError::Io {
			operation: "read frame header",
			source,
		};
	})?;
	let mut reader = HeaderReader::new(&header);
	if reader.array::<8>()? != MAGIC {
		return Err(RpcError::protocol("frame magic differs"));
	}
	if reader.u16()? != PROTOCOL_VERSION {
		return Err(RpcError::protocol("frame protocol version differs"));
	}
	let kind = FrameKind::decode(reader.u8()?)?;
	if reader.u8()? != 0 {
		return Err(RpcError::protocol("frame flags must be zero"));
	}
	let sequence = reader.u64()?;
	let run = RunId::new(reader.u64()?);
	let task = TaskId::new(reader.u64()?);
	let schedule = match reader.u64()? {
		NO_SCHEDULE => None,
		value => Some(value),
	};
	let code = reader.u32()?;
	let payload_len = usize::try_from(reader.u32()?).map_err(|_error| return RpcError::protocol("payload length does not fit usize"))?;
	if payload_len > max_payload_bytes {
		return Err(RpcError::protocol("payload exceeds the wire maximum"));
	}
	let session = Digest::new(reader.array::<32>()?);
	let expected_payload_digest = reader.array::<32>()?;
	reader.finish()?;
	let mut payload = vec![0u8; payload_len];
	stream.read_exact(&mut payload).map_err(|source| {
		return RpcError::Io {
			operation: "read frame payload",
			source,
		};
	})?;
	if payload_digest(&payload) != expected_payload_digest {
		return Err(RpcError::protocol("payload digest differs"));
	}
	return Ok(Frame {
		kind,
		sequence,
		run,
		task,
		schedule,
		code,
		session,
		payload,
	});
}

fn write_frame(stream: &mut TcpStream, frame: &Frame, max_payload_bytes: usize) -> RpcResult<()> {
	if frame.payload.len() > max_payload_bytes {
		return Err(RpcError::protocol("payload exceeds the wire maximum"));
	}
	let payload_len = u32::try_from(frame.payload.len()).map_err(|_error| return RpcError::protocol("payload length does not fit the wire"))?;
	let mut header = Vec::with_capacity(HEADER_BYTES);
	header.extend_from_slice(&MAGIC);
	header.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
	header.push(frame.kind.tag());
	header.push(0);
	header.extend_from_slice(&frame.sequence.to_le_bytes());
	header.extend_from_slice(&frame.run.get().to_le_bytes());
	header.extend_from_slice(&frame.task.get().to_le_bytes());
	header.extend_from_slice(&frame.schedule.unwrap_or(NO_SCHEDULE).to_le_bytes());
	header.extend_from_slice(&frame.code.to_le_bytes());
	header.extend_from_slice(&payload_len.to_le_bytes());
	header.extend_from_slice(&frame.session.bytes());
	header.extend_from_slice(&payload_digest(&frame.payload));
	if header.len() != HEADER_BYTES {
		return Err(RpcError::protocol("encoded header length differs"));
	}
	stream.write_all(&header).map_err(|source| {
		return RpcError::Io {
			operation: "write frame header",
			source,
		};
	})?;
	stream.write_all(&frame.payload).map_err(|source| {
		return RpcError::Io {
			operation: "write frame payload",
			source,
		};
	})?;
	return stream.flush().map_err(|source| {
		return RpcError::Io {
			operation: "flush frame",
			source,
		};
	});
}

fn payload_digest(payload: &[u8]) -> [u8; 32] {
	let mut digest = Sha256::new();
	digest.update(b"recipe-rpc-payload-v1");
	digest.update(payload);
	return digest.finalize().into();
}

fn digest_text(digest: Digest) -> String {
	let mut text = String::with_capacity(64);
	for byte in digest.bytes() {
		use fmt::Write as _;
		write!(&mut text, "{byte:02x}").expect("String writes cannot fail");
	}
	return text;
}

#[derive(Debug)]
struct HeaderReader<'a> {
	bytes: &'a [u8],
	offset: usize,
}

impl<'a> HeaderReader<'a> {
	const fn new(bytes: &'a [u8]) -> Self { return Self { bytes, offset: 0 }; }

	fn array<const N: usize>(&mut self) -> RpcResult<[u8; N]> {
		let end = self
			.offset
			.checked_add(N)
			.ok_or_else(|| return RpcError::protocol("header offset overflow"))?;
		let bytes = self
			.bytes
			.get(self.offset..end)
			.ok_or_else(|| return RpcError::protocol("header field is truncated"))?;
		self.offset = end;
		return bytes
			.try_into()
			.map_err(|_error| return RpcError::protocol("header field length differs"));
	}

	fn u8(&mut self) -> RpcResult<u8> { return Ok(self.array::<1>()?[0]); }

	fn u16(&mut self) -> RpcResult<u16> { return Ok(u16::from_le_bytes(self.array()?)); }

	fn u32(&mut self) -> RpcResult<u32> { return Ok(u32::from_le_bytes(self.array()?)); }

	fn u64(&mut self) -> RpcResult<u64> { return Ok(u64::from_le_bytes(self.array()?)); }

	const fn finish(self) -> RpcResult<()> {
		if self.offset == self.bytes.len() {
			return Ok(());
		}
		return Err(RpcError::protocol("header contains trailing bytes"));
	}
}

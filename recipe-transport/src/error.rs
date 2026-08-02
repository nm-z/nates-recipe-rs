use core::fmt;
use std::io;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportError {
	InvalidConfiguration(&'static str),
	InvalidIdentity(&'static str),
	UnsupportedVersion(u16),
	InvalidFrame(&'static str),
	FrameTooLarge { declared: usize, limit: usize },
	BufferTooSmall { required: usize, available: usize },
	UnexpectedSequence { expected: u64, received: u64 },
	UnexpectedIdentity,
	IntegrityFailure,
	Cancelled,
	DeadlineExceeded,
	ConnectionClosed,
	Io(io::ErrorKind),
	ProtocolState(&'static str),
	CapacityExhausted(&'static str),
	UnknownCompletion(u64),
	Probe(String),
	Poisoned,
}

impl TransportError {
	#[must_use]
	pub(crate) fn io(error: &io::Error) -> Self {
		match error.kind() {
			io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Self::DeadlineExceeded,
			kind => Self::Io(kind),
		}
	}
}

impl fmt::Display for TransportError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::InvalidConfiguration(message) => {
				write!(formatter, "invalid transport configuration: {message}")
			}
			Self::InvalidIdentity(name) => write!(formatter, "{name} identity must not be zero"),
			Self::UnsupportedVersion(version) => write!(formatter, "unsupported transport version {version}"),
			Self::InvalidFrame(message) => write!(formatter, "invalid transport frame: {message}"),
			Self::FrameTooLarge { declared, limit } => {
				write!(formatter, "frame payload {declared} exceeds limit {limit}")
			}
			Self::BufferTooSmall {
				required,
				available,
			} => {
				write!(
					formatter,
					"receive buffer has {available} bytes but frame requires {required}"
				)
			}
			Self::UnexpectedSequence { expected, received } => {
				write!(
					formatter,
					"expected frame sequence {expected}, received {received}"
				)
			}
			Self::UnexpectedIdentity => {
				formatter.write_str("frame endpoint identity does not match this session")
			}
			Self::IntegrityFailure => formatter.write_str("frame payload digest does not match"),
			Self::Cancelled => formatter.write_str("transport operation was cancelled"),
			Self::DeadlineExceeded => formatter.write_str("transport deadline exceeded"),
			Self::ConnectionClosed => formatter.write_str("peer closed the connection"),
			Self::Io(kind) => write!(formatter, "transport I/O {kind:?}"),
			Self::ProtocolState(message) => write!(formatter, "invalid protocol state: {message}"),
			Self::CapacityExhausted(name) => write!(formatter, "{name} capacity is exhausted"),
			Self::UnknownCompletion(token) => write!(formatter, "unknown completion token {token}"),
			Self::Probe(message) => write!(formatter, "peer benchmark failed: {message}"),
			Self::Poisoned => formatter.write_str("transport channel is poisoned"),
		}
	}
}

impl std::error::Error for TransportError {}

impl From<io::Error> for TransportError {
	fn from(error: io::Error) -> Self { Self::io(&error) }
}

pub type TransportResult<T> = Result<T, TransportError>;

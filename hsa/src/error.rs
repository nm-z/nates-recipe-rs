use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Error {
	PathContainsNul {
		path: String,
	},
	NameContainsNul {
		field: &'static str,
	},
	LibraryOpen {
		path: String,
		detail: String,
	},
	MissingSymbol {
		library: String,
		symbol: &'static str,
		detail: String,
	},
	Hsa {
		operation: &'static str,
		status: i32,
		message: Option<String>,
	},
	InvalidUtf8 {
		field: &'static str,
	},
	InvalidIdentity {
		kind: &'static str,
		value: String,
		reason: &'static str,
	},
	InvalidAttribute {
		field: &'static str,
		value: u64,
	},
	CallbackPanicked {
		operation: &'static str,
	},
	AllocationFailed {
		field: &'static str,
		requested: usize,
	},
	RuntimeClosed,
	UnsupportedAgent {
		reason: &'static str,
	},
	InvalidQueueSize {
		requested: u32,
		minimum: u32,
		maximum: u32,
		reason: &'static str,
	},
	UnsupportedQueueKind {
		requested: crate::QueueKind,
		advertised: crate::QueueKind,
	},
	NullQueue,
	InvalidQueueReturned {
		kind: u32,
		features: u32,
		size: u32,
		base_is_null: bool,
	},
	EmptyCodeObject,
	InvalidMemoryPoolIndex {
		index: usize,
		pool_count: usize,
	},
	MemoryPoolNotAllocatable {
		index: usize,
	},
	NoMatchingMemoryPool {
		kind: &'static str,
	},
	InvalidAllocationSize {
		requested: usize,
		maximum: Option<usize>,
	},
	NullAllocation,
	ResourceBusy {
		resource: &'static str,
	},
	SymbolNotKernel {
		name: String,
		kind: i32,
	},
	InvalidKernel {
		reason: &'static str,
	},
	InvalidDispatch {
		reason: &'static str,
	},
	QueueFull {
		write_index: u64,
		read_index: u64,
		size: u32,
	},
	InvalidProgressRequest {
		required_packets: u32,
		queue_size: u32,
	},
	CopyOutOfBounds {
		buffer: &'static str,
		offset: usize,
		size: usize,
		capacity: usize,
	},
	NullSignal,
	AsyncSignal {
		value: i64,
	},
	SessionPoisoned {
		status: i32,
		source_queue_id: Option<u64>,
		epoch: u64,
	},
	DeferredRetirement {
		pending: usize,
		poisoned: bool,
	},
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::PathContainsNul { path } => {
				write!(formatter, "shared-library path contains NUL: {path:?}")
			}
			Self::NameContainsNul { field } => write!(formatter, "{field} contains an interior NUL"),
			Self::LibraryOpen { path, detail } => {
				write!(
					formatter,
					"could not open shared library {path:?}: {detail}"
				)
			}
			Self::MissingSymbol {
				library,
				symbol,
				detail,
			} => {
				write!(
					formatter,
					"shared library {library:?} lacks required symbol {symbol}: {detail}"
				)
			}
			Self::Hsa {
				operation,
				status,
				message,
			} => {
				write!(formatter, "{operation} failed with HSA status {status:#x}")?;
				if let Some(message) = message {
					write!(formatter, " ({message})")?;
				}
				Ok(())
			}
			Self::InvalidUtf8 { field } => {
				write!(formatter, "{field} returned by ROCr is not valid UTF-8")
			}
			Self::InvalidIdentity {
				kind,
				value,
				reason,
			} => write!(formatter, "invalid {kind} identity {value:?}: {reason}"),
			Self::InvalidAttribute { field, value } => {
				write!(formatter, "invalid ROCr value {value:#x} for {field}")
			}
			Self::CallbackPanicked { operation } => {
				write!(formatter, "{operation} callback panicked")
			}
			Self::AllocationFailed { field, requested } => {
				write!(
					formatter,
					"could not reserve {requested} bytes while reading {field}"
				)
			}
			Self::RuntimeClosed => write!(formatter, "the HSA runtime is already closed"),
			Self::UnsupportedAgent { reason } => {
				write!(
					formatter,
					"agent cannot back a Recipe GPU session: {reason}"
				)
			}
			Self::InvalidQueueSize {
				requested,
				minimum,
				maximum,
				reason,
			} => {
				write!(
					formatter,
					"queue size {requested} is invalid for [{minimum}, {maximum}]: {reason}"
				)
			}
			Self::UnsupportedQueueKind {
				requested,
				advertised,
			} => {
				write!(
					formatter,
					"queue kind {requested:?} is incompatible with advertised kind {advertised:?}"
				)
			}
			Self::NullQueue => write!(formatter, "ROCr reported success but returned a null queue"),
			Self::InvalidQueueReturned {
				kind,
				features,
				size,
				base_is_null,
			} => {
				write!(
					formatter,
					"ROCr returned invalid queue fields: kind={kind}, features={features:#x}, size={size}, base_is_null={base_is_null}"
				)
			}
			Self::EmptyCodeObject => write!(formatter, "HSACO code object is empty"),
			Self::InvalidMemoryPoolIndex { index, pool_count } => {
				write!(
					formatter,
					"memory-pool index {index} is outside the discovered pool count {pool_count}"
				)
			}
			Self::MemoryPoolNotAllocatable { index } => {
				write!(formatter, "memory pool {index} is not runtime allocatable")
			}
			Self::NoMatchingMemoryPool { kind } => {
				write!(
					formatter,
					"no allocatable {kind} memory pool was discovered"
				)
			}
			Self::InvalidAllocationSize { requested, maximum } => {
				write!(
					formatter,
					"allocation size {requested} is invalid for maximum {maximum:?}"
				)
			}
			Self::NullAllocation => {
				write!(
					formatter,
					"ROCr reported success but returned a null allocation"
				)
			}
			Self::ResourceBusy { resource } => {
				write!(formatter, "{resource} still has live dependent resources")
			}
			Self::SymbolNotKernel { name, kind } => {
				write!(
					formatter,
					"executable symbol {name:?} has non-kernel kind {kind}"
				)
			}
			Self::InvalidKernel { reason } => write!(formatter, "invalid HSA kernel: {reason}"),
			Self::InvalidDispatch { reason } => write!(formatter, "invalid HSA dispatch: {reason}"),
			Self::QueueFull {
				write_index,
				read_index,
				size,
			} => {
				write!(
					formatter,
					"HSA queue is full at write index {write_index}, read index {read_index}, size {size}"
				)
			}
			Self::InvalidProgressRequest {
				required_packets,
				queue_size,
			} => {
				write!(
					formatter,
					"capacity request for {required_packets} packets is invalid for queue size {queue_size}"
				)
			}
			Self::CopyOutOfBounds {
				buffer,
				offset,
				size,
				capacity,
			} => {
				write!(
					formatter,
					"{buffer} copy range offset {offset} plus size {size} exceeds capacity {capacity}"
				)
			}
			Self::NullSignal => write!(formatter, "ROCr returned an invalid zero signal handle"),
			Self::AsyncSignal { value } => {
				write!(
					formatter,
					"HSA completion signal reported asynchronous failure {value}"
				)
			}
			Self::SessionPoisoned {
				status,
				source_queue_id,
				epoch,
			} => {
				write!(
					formatter,
					"HSA session is poisoned by async status {status:#x} from queue {source_queue_id:?} at epoch {epoch}"
				)
			}
			Self::DeferredRetirement { pending, poisoned } => {
				write!(
					formatter,
					"{pending} deferred HSA operation(s) remain unresolved (session_poisoned={poisoned}); their signals and referenced resources were retained"
				)
			}
		}
	}
}

impl std::error::Error for Error {}

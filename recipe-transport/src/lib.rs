#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Explicit, bounded TCP transport for Recipe.
//!
//! This crate never opens a listener, resolves a name, discovers an interface,
//! selects a peer, or establishes security. Callers supply an already connected
//! [`std::net::TcpStream`] and exact endpoint identities. Every frame carries
//! and validates both machine and profile identities.

mod error;
mod probe;
mod protocol;
mod runtime;

pub use error::{TransportError, TransportResult};
pub use probe::{MeasuredLocalMemory, TcpPeerSession};
pub use protocol::{
	CompletionToken, EndpointIdentity, FrameKind, FrameMetadata, ProtocolLimits, ReceivedFrame, SessionIdentity,
	WireReceiver, WireSender, split_stream,
};
pub use runtime::{
	ChannelCapacities, CompletionState, Progress, RuntimeChannel, RuntimeLane, ScheduleStamp, Submission,
};

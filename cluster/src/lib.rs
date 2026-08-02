#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Deterministic assembly of complete per-machine bare-metal measurements.
//!
//! Cluster membership and transport endpoints are configuration. Devices,
//! rates, capacities, topology links, duplex behavior, and lane counts are
//! accepted only from canonical member profiles and explicit peer-probe
//! evidence. Endpoint machine identities come from retained stable machine
//! fingerprints, and network gateways resolve through retained RAM-domain
//! keys.

mod assemble;
mod error;
mod hash;
mod model;

pub use assemble::{ClusterProfileCodec, assemble_cluster};
pub use error::{ClusterError, ClusterResult};
pub use model::{
	CLUSTER_SCHEMA, ClusterConfiguration, MeasuredNetworkPair, MemberProfileIdentity, MemberSpec, NetworkPairSpec,
	SubmittedProfile,
};

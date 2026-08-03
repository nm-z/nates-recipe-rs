#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Bare-metal discovery and bounded measurement pipeline for Recipe.
//!
//! Seed configuration controls probe bounds only. The emitted topology and
//! discovery profile contain measured properties and stable discovered
//! identities, including machine fingerprints and exact RAM, storage, and GPU
//! origins; users never enumerate devices or enter production rates.

pub mod cache; pub mod codec; pub mod engine; pub mod error; pub mod hash; pub mod local; pub mod model;
pub mod resolve; pub mod seed;

pub use cache::*; pub use codec::*; pub use engine::*; pub use error::*; pub use model::*; pub use resolve::*;
pub use seed::*;

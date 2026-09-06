#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Recipe's OGDL-derived ordered graph syntax.
//!
//! The syntax deliberately gives spaces no structural meaning. Leading tab
//! characters select a line's depth; every subsequent tab creates another
//! child on the same line. Newlines start another chain. Node text must be
//! nonempty and cannot contain tabs, carriage returns, or line feeds.
//!
//! This crate currently represents an ordered rooted forest. It does not
//! implement OGDL shared references, anchors, links, or cycles, so callers must
//! not treat it as a general-graph serialization.

mod graph;
mod parser;
mod serializer;

pub use graph::{Graph, Node, NodeId};

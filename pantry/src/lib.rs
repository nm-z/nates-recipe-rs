#![deny(clippy::unwrap_used)]
#![deny(clippy::match_wild_err_arm)]
//! Owns all parsing: loaders (csv/arff/zip/dir) + column-type detection. Parse
//! any format, detect column types via the char-level detector, export OGDL.
//! Depends on `recipe-infer` (the forward engine) plus leaf utility crates only —
//! it knows nothing of the trainer, builders, or encoded datasets above it.

pub type Mat = ndarray::Array2<f64>;
pub type Vec1 = ndarray::Array1<f64>;

pub mod data;
pub mod detect;

pub use data::*;
pub use detect::*;

/// A detected column type. The encoder above turns these into numeric columns
/// (one-hot / index / token-id / day-count); here it is just the taxonomy the
/// loaders and the char-level detector speak in.
#[derive(Clone)]
#[allow(dead_code)]
pub enum Kind {
	Numeric,
	Temporal,
	Categorical(Vec<String>),
	Ordinal(Vec<String>),
	Text(Vec<String>),
	Image,
}

/// One named column with its detected (or declared, for ARFF) `Kind`.
#[derive(Clone)]
pub struct Attr {
	pub name: String,
	pub kind: Kind,
}

/// MemAvailable from /proc/meminfo, in bytes. `usize::MAX` if it can't be read
/// (no guard rather than a false positive). Used by the CSV/parse RAM guards.
pub fn available_ram_bytes() -> usize {
	std::fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(usize::MAX, |kb| kb.saturating_mul(1024))
}

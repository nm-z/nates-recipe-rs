#![allow(unsafe_code)]

use std::fs;

pub type Mat = ndarray::Array2<f64>;
pub type Vec1 = ndarray::Array1<f64>;

pub const TEXT_CONTEXT: usize = 256;

pub mod bpe;
pub mod data;
pub mod detect;
pub mod encode;

pub use data::*;
pub use detect::*;

#[derive(Clone)]
pub enum Kind {
	Numeric,
	Temporal,
	Categorical(Vec<String>),
	Ordinal(Vec<String>),
	Text(Vec<String>),
	Image,
}

#[derive(Clone)]
pub struct Attr {
	pub name: String,
	pub kind: Kind,
}

pub fn available_ram_bytes() -> usize {
	fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(usize::MAX, |kb| kb.saturating_mul(1024))
}

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Dependency-clean external representation handling for Recipe.
//!
//! This crate owns lexical ingestion and framing, not payload calculations.
//! Numerical preprocessing remains a GPU calculation. The decimal validators
//! here establish the C2 representation round-trip boundary before f32 or
//! int32 payload bytes are admitted to a run. External files are copied into
//! bounded, content-addressed preparation snapshots and closed before runtime;
//! model-format parsers expose validated encoded spans without decoding their
//! calculation payloads on the CPU.

mod image;
mod numeric;
mod safetensors;
mod source;
mod table;

pub use image::{
	ExternalValue, ImagePackError, ImagePackErrorKind, ImagePackResult, PackedInitImage, pack_init_images,
};
pub use numeric::{
	DecimalError, DecimalErrorKind, F32_GUARANTEED_SIGNIFICANT_DIGITS, F32Decimal, I32_GUARANTEED_SIGNIFICANT_DIGITS,
	I32Decimal, parse_contract_f32, parse_contract_i32,
};
pub use safetensors::{
	SafeTensorArchive, SafeTensorDType, SafeTensorEntry, SafeTensorError, SafeTensorErrorKind, SafeTensorLimits,
	SafeTensorResult, parse_safetensors,
};
pub use source::{SourceError, SourceErrorKind, SourceLimit, SourceResult, SourceSnapshot, read_source_snapshot};
pub use table::{
	Delimiter, HeaderMode, IngestError, IngestErrorKind, IngestLimits, IngestResult, RawTable, TableRequest,
	parse_table, read_table,
};

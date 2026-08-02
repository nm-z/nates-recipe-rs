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

mod dataset;
mod gguf;
mod gguf_ogdl;
mod image;
mod image_header;
mod inference;
mod numeric;
mod prepare;
mod safetensors;
mod semantic;
mod source;
mod table;

pub use dataset::{
	DatasetSourceError, DatasetSourceErrorKind, DatasetSourceResult, DistilledDataset, SourceFormat, distill_dataset,
	distill_datasets,
};
pub use gguf::{
	GgufArchive, GgufEndian, GgufError, GgufErrorKind, GgufLimits, GgufMetadataArray, GgufMetadataEntry,
	GgufMetadataType, GgufMetadataValue, GgufResult, GgufTensor, GgufTensorType, parse_gguf,
};
pub use gguf_ogdl::{
	GgufModelDescriptor, GgufOgdlError, GgufOgdlErrorKind, GgufOgdlResult, gguf_to_structural_ogdl,
	gguf_to_structural_ogdl_stream, inspect_gguf_model_stream, structural_ogdl_declared_gguf_bytes,
	structural_ogdl_to_gguf, structural_ogdl_to_gguf_stream,
};
pub use image::{
	ExternalValue, ImagePackError, ImagePackErrorKind, ImagePackResult, PackedInitImage, pack_init_images,
};
pub use image_header::{EncodedImageFormat, EncodedImageMetadata, ImageColorModel, ImageValueLayout, ImageValueRange};
pub use inference::{
	InferenceDataPath, InferenceFeatureEncoding, InferenceFeatureSchema, InferencePrepareError,
	InferencePrepareErrorKind, InferencePrepareResult, PreparedInferenceDataset, PreparedInferenceFeature,
	PreparedInferenceValues, prepare_inference_table,
};
pub use numeric::{
	DecimalError, DecimalErrorKind, F32_GUARANTEED_SIGNIFICANT_DIGITS, F32Decimal, I32_GUARANTEED_SIGNIFICANT_DIGITS,
	I32Decimal, parse_contract_f32, parse_contract_i32,
};
pub use prepare::{
	CategoricalObservation, ColumnPattern, ComparisonOperator, DenseMatrix, PartitionKind, PredicateLiteral,
	PreparationRequest, PrepareError, PrepareErrorKind, PrepareResult, PreparedDataset, PreparedPartition,
	PreparedValues, PreparedVector, RowPredicate, TemporalOrigin, TrainFraction, VariableWidthVector, VectorMetadata,
	VectorRole, VectorSchema, prepare_inferred_table, prepare_table, select_table,
};
pub use safetensors::{
	SafeTensorArchive, SafeTensorDType, SafeTensorEntry, SafeTensorError, SafeTensorErrorKind, SafeTensorLimits,
	SafeTensorResult, parse_safetensors,
};
pub use semantic::{
	AmbiguousVectorModel, CategoricalEncodingModel, InferredVector, InferredVectorList, SemanticError,
	SemanticErrorKind, SemanticResult, SemanticType, VectorEncoding, VectorEvidence, infer_table_vectors,
};
pub use source::{SourceError, SourceErrorKind, SourceLimit, SourceResult, SourceSnapshot, read_source_snapshot};
pub use table::{
	Delimiter, HeaderMode, IngestError, IngestErrorKind, IngestLimits, IngestResult, RawTable, TableRequest,
	parse_table, read_table,
};

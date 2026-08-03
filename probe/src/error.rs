use core::fmt; use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProbeError { Contract { line: Option<usize>, message: String, }, Discovery(String), Benchmark(String),
	IncompleteGpuEnumeration, MissingMeasurement(String), IncompletePeerMeasurement { peer: String,
		direction: &'static str,
	}, InvalidProfile(String), Cache(String), Io {
		operation: &'static str,
		path: PathBuf, message: String, }, }

impl ProbeError {
	#[must_use]
	pub fn contract(line: Option<usize>, message: impl Into<String>) -> Self { Self::Contract { line,
			message: message.into(), } }

	#[must_use]
	pub fn io(operation: &'static str, path: impl Into<PathBuf>, error: impl fmt::Display) -> Self {
		Self::Io { operation, path: path.into(), message: error.to_string(), } } }

impl fmt::Display for ProbeError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self { Self::Contract { line: Some(line), message,
			} => write!(formatter, "contract line {line}: {message}"),
			Self::Contract { line: None, message,
			} => write!(formatter, "contract: {message}"),
			Self::Discovery(message) => write!(formatter, "discovery: {message}"),
			Self::Benchmark(message) => write!(formatter, "benchmark: {message}"),
			Self::IncompleteGpuEnumeration => {
				formatter.write_str("GPU discovery did not prove exhaustive enumeration")
			}
			Self::MissingMeasurement(message) => {
				write!(formatter, "missing measured property: {message}")
			}
			Self::IncompletePeerMeasurement { peer, direction } => { write!( formatter,
					"peer {peer} has no measured {direction} throughput"
				) }
			Self::InvalidProfile(message) => write!(formatter, "invalid measured profile: {message}"),
			Self::Cache(message) => write!(formatter, "profile cache: {message}"),
			Self::Io { operation, path, message,
			} => write!(formatter, "{operation} {}: {message}", path.display()),
		} } }

impl std::error::Error for ProbeError {}

pub type ProbeResult<T> = Result<T, ProbeError>;

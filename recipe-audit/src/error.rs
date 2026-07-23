use std::fmt;
use std::path::PathBuf;

/// An input or collection failure. Audit errors always fail the gate.
#[derive(Debug)]
pub enum AuditError {
	Configuration(String),
	InvalidDependencyGraph(String),
	InvalidCargoMetadata(String),
	Lexical {
		path: String,
		line: u64,
		reason: &'static str,
	},
	InvalidElf {
		path: PathBuf,
		reason: String,
	},
	Io {
		path: PathBuf,
		source: std::io::Error,
	},
}

impl AuditError {
	pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
		Self::Io {
			path: path.into(),
			source,
		}
	}
}

impl fmt::Display for AuditError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Configuration(message) => write!(formatter, "invalid audit configuration: {message}"),
			Self::InvalidDependencyGraph(message) => {
				write!(formatter, "invalid dependency graph: {message}")
			}
			Self::InvalidCargoMetadata(message) => {
				write!(formatter, "invalid Cargo metadata: {message}")
			}
			Self::Lexical { path, line, reason } => {
				write!(formatter, "{path}:{line}: lexical audit failed: {reason}")
			}
			Self::InvalidElf { path, reason } => {
				write!(formatter, "invalid ELF {}: {reason}", path.display())
			}
			Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
		}
	}
}

impl std::error::Error for AuditError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Io { source, .. } => Some(source),
			_ => None,
		}
	}
}

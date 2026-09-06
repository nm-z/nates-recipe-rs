use core::{fmt, num::NonZeroU64};
use std::{
	fs::File,
	io::Read as _,
	path::{Path, PathBuf},
};

use recipe_core::Digest;
use sha2::{Digest as _, Sha256};

/// Maximum admitted bytes for one external file snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLimit(NonZeroU64);

impl SourceLimit {
	/// Construct a nonzero source bound.
	///
	/// # Errors
	///
	/// Returns [`SourceErrorKind::InvalidLimit`] when `bytes` is zero.
	pub fn new(bytes: u64) -> SourceResult<Self> {
		NonZeroU64::new(bytes).map(Self).ok_or_else(|| {
			SourceError::new(
				SourceErrorKind::InvalidLimit,
				"source byte limit must be nonzero",
			)
		})
	}

	#[must_use]
	pub const fn bytes(self) -> NonZeroU64 { self.0 }
}

/// Immutable, content-addressed pre-run copy of one external file.
///
/// The open file is closed before this value is returned. Runtime code can
/// only consume the retained bytes and cannot perform another file read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSnapshot {
	path: PathBuf,
	bytes: Vec<u8>,
	digest: Digest,
}

impl SourceSnapshot {
	#[must_use]
	pub fn path(&self) -> &Path { &self.path }

	#[must_use]
	pub fn bytes(&self) -> &[u8] { &self.bytes }

	#[must_use]
	pub const fn digest(&self) -> Digest { self.digest }

	#[must_use]
	pub fn into_bytes(self) -> Vec<u8> { self.bytes }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceErrorKind {
	InvalidLimit,
	Io,
	NotRegularFile,
	LimitExceeded,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceError {
	pub kind: SourceErrorKind,
	pub path: Option<PathBuf>,
	pub detail: String,
}

impl SourceError {
	fn new(kind: SourceErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			path: None,
			detail: detail.into(),
		}
	}

	fn for_path(mut self, path: &Path) -> Self {
		self.path = Some(path.to_path_buf());
		self
	}
}

impl fmt::Display for SourceError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)?;
		if let Some(path) = &self.path {
			write!(formatter, " [{}]", path.display())?;
		}
		Ok(())
	}
}

impl std::error::Error for SourceError {}

pub type SourceResult<T> = Result<T, SourceError>;

/// Read one regular file exactly once into a bounded, content-addressed
/// preparation snapshot.
///
/// Metadata is used only for an early refusal and allocation hint. The actual
/// read is capped at `limit + 1`, so a concurrently growing file cannot bypass
/// the configured bound. No file handle or callback is retained.
///
/// # Errors
///
/// Returns an I/O, regular-file, arithmetic, or configured-bound error without
/// exposing a partial snapshot.
pub fn read_source_snapshot(path: &Path, limit: SourceLimit) -> SourceResult<SourceSnapshot> {
	let file = File::open(path).map_err(|error| SourceError::new(SourceErrorKind::Io, format!("open source: {error}")).for_path(path))?;
	let metadata = file
		.metadata()
		.map_err(|error| SourceError::new(SourceErrorKind::Io, format!("inspect source: {error}")).for_path(path))?;
	if !metadata.is_file() {
		return Err(SourceError::new(
			SourceErrorKind::NotRegularFile,
			"external source is not a regular file",
		)
		.for_path(path));
	}
	if metadata.len() > limit.bytes().get() {
		return Err(SourceError::new(
			SourceErrorKind::LimitExceeded,
			format!(
				"source metadata reports {} bytes, limit is {}",
				metadata.len(),
				limit.bytes()
			),
		)
		.for_path(path));
	}
	let read_limit = limit.bytes().get().checked_add(1).ok_or_else(|| {
		SourceError::new(
			SourceErrorKind::ArithmeticOverflow,
			"source read bound exceeds u64",
		)
		.for_path(path)
	})?;
	let initial_capacity = usize::try_from(metadata.len())
		.unwrap_or(usize::MAX)
		.min(1_048_576);
	let mut reader = file.take(read_limit);
	let mut bytes = Vec::with_capacity(initial_capacity);
	reader.read_to_end(&mut bytes)
		.map_err(|error| SourceError::new(SourceErrorKind::Io, format!("read source: {error}")).for_path(path))?;
	let count = u64::try_from(bytes.len()).map_err(|error| {
		SourceError::new(
			SourceErrorKind::ArithmeticOverflow,
			format!("read source length cannot be represented as u64: {error}"),
		)
		.for_path(path)
	})?;
	if count > limit.bytes().get() {
		return Err(SourceError::new(
			SourceErrorKind::LimitExceeded,
			format!(
				"source grew to {count} bytes while reading, limit is {}",
				limit.bytes()
			),
		)
		.for_path(path));
	}
	let digest = Digest::new(Sha256::digest(&bytes).into());
	Ok(SourceSnapshot {
		path: path.to_path_buf(),
		bytes,
		digest,
	})
}

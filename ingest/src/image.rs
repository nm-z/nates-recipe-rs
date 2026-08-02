use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{ByteCount, DType, DeviceId, InitDataImage, ValueId};

/// One graph-level input payload admitted before a run is prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalValue<'a> {
	pub logical: ValueId,
	pub bytes: &'a [u8],
}

impl<'a> ExternalValue<'a> {
	#[must_use]
	pub const fn new(logical: ValueId, bytes: &'a [u8]) -> Self { Self { logical, bytes } }
}

/// A complete, immutable byte image for one device's sole init upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedInitImage {
	device: DeviceId,
	image: ValueId,
	bytes: Vec<u8>,
}

impl PackedInitImage {
	#[must_use]
	pub const fn device(&self) -> DeviceId { self.device }

	#[must_use]
	pub const fn image(&self) -> ValueId { self.image }

	#[must_use]
	pub fn bytes(&self) -> &[u8] { &self.bytes }

	#[must_use]
	pub fn into_bytes(self) -> Vec<u8> { self.bytes }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ImagePackErrorKind {
	DuplicateSource,
	MissingSource,
	UnexpectedSource,
	SourceSizeMismatch,
	DuplicateDevice,
	DuplicateMember,
	ConflictingContract,
	InvalidManifest,
	ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImagePackError {
	pub kind: ImagePackErrorKind,
	pub detail: String,
}

impl ImagePackError {
	fn new(kind: ImagePackErrorKind, detail: impl Into<String>) -> Self {
		Self {
			kind,
			detail: detail.into(),
		}
	}
}

impl fmt::Display for ImagePackError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	}
}

impl std::error::Error for ImagePackError {}

pub type ImagePackResult<T> = Result<T, ImagePackError>;

/// Materialize every device's finalized init image from graph-level inputs.
///
/// A logical source may appear in several device manifests; its bytes are
/// replicated into each selected resident copy. Gaps in an image, including
/// preallocated device fault flags, remain zero initialized. The function
/// rejects missing, duplicate, unexpected, conflicting, or incorrectly sized
/// sources and never returns a partial image set.
///
/// # Errors
///
/// Returns a fail-closed contract or source error before any image is exposed.
pub fn pack_init_images(
	manifests: &[InitDataImage],
	sources: &[ExternalValue<'_>],
) -> ImagePackResult<Vec<PackedInitImage>> {
	let source_map = index_sources(sources)?;
	let required = validate_manifests(manifests)?;
	for logical in source_map.keys() {
		if !required.contains_key(logical) {
			return Err(ImagePackError::new(
				ImagePackErrorKind::UnexpectedSource,
				format!("logical input {logical} is absent from every init-image manifest"),
			));
		}
	}
	for (logical, (_, expected_bytes)) in &required {
		let Some(source) = source_map.get(logical).copied() else {
			return Err(ImagePackError::new(
				ImagePackErrorKind::MissingSource,
				format!("logical input {logical} has no admitted payload"),
			));
		};
		let actual_bytes = u64::try_from(source.len()).map_err(|error| {
			ImagePackError::new(
				ImagePackErrorKind::ArithmeticOverflow,
				format!("logical input {logical} byte length cannot be represented: {error}"),
			)
		})?;
		if actual_bytes != expected_bytes.get() {
			return Err(ImagePackError::new(
				ImagePackErrorKind::SourceSizeMismatch,
				format!(
					"logical input {logical} has {actual_bytes} bytes, expected {}",
					expected_bytes.get()
				),
			));
		}
	}

	let mut packed = Vec::with_capacity(manifests.len());
	for manifest in manifests {
		let image_len = usize::try_from(manifest.bytes.get()).map_err(|error| {
			ImagePackError::new(
				ImagePackErrorKind::ArithmeticOverflow,
				format!(
					"device {} init-image size cannot address host memory: {error}",
					manifest.device
				),
			)
		})?;
		let mut bytes = vec![0_u8; image_len];
		for member in &manifest.members {
			let source = source_map[&member.logical];
			let start = usize::try_from(member.image_offset.get()).map_err(|error| {
				ImagePackError::new(
					ImagePackErrorKind::ArithmeticOverflow,
					format!(
						"device {} member {} offset cannot address host memory: {error}",
						manifest.device, member.logical
					),
				)
			})?;
			let end = start.checked_add(source.len()).ok_or_else(|| {
				ImagePackError::new(
					ImagePackErrorKind::ArithmeticOverflow,
					format!(
						"device {} member {} host range overflowed",
						manifest.device, member.logical
					),
				)
			})?;
			let destination = bytes.get_mut(start..end).ok_or_else(|| {
				ImagePackError::new(
					ImagePackErrorKind::InvalidManifest,
					format!(
						"device {} member {} lies outside its init image",
						manifest.device, member.logical
					),
				)
			})?;
			destination.copy_from_slice(source);
		}
		packed.push(PackedInitImage {
			device: manifest.device,
			image: manifest.image,
			bytes,
		});
	}
	packed.sort_by_key(PackedInitImage::device);
	Ok(packed)
}

fn index_sources<'a>(sources: &'a [ExternalValue<'a>]) -> ImagePackResult<BTreeMap<ValueId, &'a [u8]>> {
	let mut result = BTreeMap::new();
	for source in sources {
		if result.insert(source.logical, source.bytes).is_some() {
			return Err(ImagePackError::new(
				ImagePackErrorKind::DuplicateSource,
				format!(
					"logical input {} was supplied more than once",
					source.logical
				),
			));
		}
	}
	Ok(result)
}

fn validate_manifests(manifests: &[InitDataImage]) -> ImagePackResult<BTreeMap<ValueId, (DType, ByteCount)>> {
	let mut devices = BTreeSet::new();
	let mut required = BTreeMap::new();
	for manifest in manifests {
		if !devices.insert(manifest.device) {
			return Err(ImagePackError::new(
				ImagePackErrorKind::DuplicateDevice,
				format!("device {} has more than one init image", manifest.device),
			));
		}
		if manifest.bytes == ByteCount::ZERO {
			return Err(ImagePackError::new(
				ImagePackErrorKind::InvalidManifest,
				format!("device {} init image is empty", manifest.device),
			));
		}
		let mut logical_members = BTreeSet::new();
		let mut physical_members = BTreeSet::new();
		let mut occupied = Vec::with_capacity(manifest.members.len());
		for member in &manifest.members {
			if !logical_members.insert(member.logical) || !physical_members.insert(member.physical) {
				return Err(ImagePackError::new(
					ImagePackErrorKind::DuplicateMember,
					format!(
						"device {} repeats logical or physical member {}",
						manifest.device, member.logical
					),
				));
			}
			if member.image_offset.get() % u64::from(member.dtype.byte_width()) != 0 {
				return Err(ImagePackError::new(
					ImagePackErrorKind::InvalidManifest,
					format!(
						"device {} member {} is not scalar aligned",
						manifest.device, member.logical
					),
				));
			}
			let Some(end) = member.image_offset.checked_end(member.bytes) else {
				return Err(ImagePackError::new(
					ImagePackErrorKind::ArithmeticOverflow,
					format!(
						"device {} member {} range overflowed",
						manifest.device, member.logical
					),
				));
			};
			if end > manifest.bytes {
				return Err(ImagePackError::new(
					ImagePackErrorKind::InvalidManifest,
					format!(
						"device {} member {} exceeds its init image",
						manifest.device, member.logical
					),
				));
			}
			occupied.push((member.image_offset.get(), end.get()));
			match required.get(&member.logical).copied() {
				Some(contract) if contract != (member.dtype, member.bytes) => {
					return Err(ImagePackError::new(
						ImagePackErrorKind::ConflictingContract,
						format!(
							"logical input {} has conflicting replicated type or size",
							member.logical
						),
					));
				}
				Some(_) => {}
				None => {
					required.insert(member.logical, (member.dtype, member.bytes));
				}
			}
		}
		occupied.sort_unstable();
		if occupied.windows(2).any(|pair| pair[0].1 > pair[1].0) {
			return Err(ImagePackError::new(
				ImagePackErrorKind::InvalidManifest,
				format!("device {} init-image members overlap", manifest.device),
			));
		}
	}
	Ok(required)
}

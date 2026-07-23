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
	pub const fn new(logical: ValueId, bytes: &'a [u8]) -> Self {
		Self { logical, bytes }
	}
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
	pub const fn device(&self) -> DeviceId {
		self.device
	}

	#[must_use]
	pub const fn image(&self) -> ValueId {
		self.image
	}

	#[must_use]
	pub fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	#[must_use]
	pub fn into_bytes(self) -> Vec<u8> {
		self.bytes
	}
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

#[cfg(test)]
mod tests {
	use recipe_core::{ByteOffset, InitDataImageMember};

	use super::*;

	const LOGICAL_A: ValueId = ValueId::new(101);
	const LOGICAL_B: ValueId = ValueId::new(102);
	const GPU: DeviceId = DeviceId::new(1);
	const RAM: DeviceId = DeviceId::new(2);

	fn member(logical: ValueId, physical: u64, dtype: DType, bytes: u64, offset: u64) -> InitDataImageMember {
		InitDataImageMember {
			logical,
			physical: ValueId::new(physical),
			dtype,
			bytes: ByteCount::new(bytes),
			image_offset: ByteOffset::new(offset),
		}
	}

	fn manifests() -> Vec<InitDataImage> {
		vec![
			InitDataImage {
				device: RAM,
				image: ValueId::new(2),
				bytes: ByteCount::new(12),
				members: vec![
					member(LOGICAL_A, 20, DType::F32, 4, 0),
					member(LOGICAL_B, 21, DType::I32, 4, 8),
				],
			},
			InitDataImage {
				device: GPU,
				image: ValueId::new(1),
				bytes: ByteCount::new(8),
				members: vec![member(LOGICAL_A, 10, DType::F32, 4, 4)],
			},
		]
	}

	#[test]
	fn packs_each_device_once_and_zero_fills_non_input_regions() {
		let a = [1, 2, 3, 4];
		let b = [5, 6, 7, 8];
		let packed = pack_init_images(
			&manifests(),
			&[
				ExternalValue::new(LOGICAL_B, &b),
				ExternalValue::new(LOGICAL_A, &a),
			],
		)
		.unwrap();
		assert_eq!(packed.len(), 2);
		assert_eq!(packed[0].device(), GPU);
		assert_eq!(packed[0].bytes(), &[0, 0, 0, 0, 1, 2, 3, 4]);
		assert_eq!(packed[1].device(), RAM);
		assert_eq!(packed[1].bytes(), &[1, 2, 3, 4, 0, 0, 0, 0, 5, 6, 7, 8]);
	}

	#[test]
	fn rejects_incomplete_ambiguous_and_unexpected_sources() {
		let a = [1, 2, 3, 4];
		let b = [5, 6, 7, 8];
		let cases = [
			(
				vec![ExternalValue::new(LOGICAL_A, &a)],
				ImagePackErrorKind::MissingSource,
			),
			(
				vec![
					ExternalValue::new(LOGICAL_A, &a),
					ExternalValue::new(LOGICAL_A, &a),
					ExternalValue::new(LOGICAL_B, &b),
				],
				ImagePackErrorKind::DuplicateSource,
			),
			(
				vec![
					ExternalValue::new(LOGICAL_A, &a),
					ExternalValue::new(LOGICAL_B, &b),
					ExternalValue::new(ValueId::new(999), &a),
				],
				ImagePackErrorKind::UnexpectedSource,
			),
		];
		for (sources, expected) in cases {
			assert_eq!(
				pack_init_images(&manifests(), &sources).unwrap_err().kind,
				expected
			);
		}
	}

	#[test]
	fn rejects_wrong_source_size_and_invalid_or_conflicting_manifests() {
		let a = [1, 2, 3, 4];
		let b = [5, 6, 7, 8];
		assert_eq!(
			pack_init_images(
				&manifests(),
				&[
					ExternalValue::new(LOGICAL_A, &a[..3]),
					ExternalValue::new(LOGICAL_B, &b),
				],
			)
			.unwrap_err()
			.kind,
			ImagePackErrorKind::SourceSizeMismatch
		);

		let mut overlapping = manifests();
		overlapping[0].members[1].image_offset = overlapping[0].members[0].image_offset;
		assert_eq!(
			pack_init_images(&overlapping, &[]).unwrap_err().kind,
			ImagePackErrorKind::InvalidManifest
		);

		let mut conflicting = manifests();
		conflicting[1].bytes = ByteCount::new(12);
		conflicting[1].members[0].bytes = ByteCount::new(8);
		assert_eq!(
			pack_init_images(&conflicting, &[]).unwrap_err().kind,
			ImagePackErrorKind::ConflictingContract
		);
	}

	#[test]
	fn permits_zero_byte_logical_values_and_empty_device_manifests() {
		let manifests = vec![
			InitDataImage {
				device: GPU,
				image: ValueId::new(1),
				bytes: ByteCount::new(4),
				members: vec![member(LOGICAL_A, 10, DType::F32, 0, 0)],
			},
			InitDataImage {
				device: RAM,
				image: ValueId::new(2),
				bytes: ByteCount::new(4),
				members: vec![],
			},
		];
		let packed = pack_init_images(&manifests, &[ExternalValue::new(LOGICAL_A, &[])]).unwrap();
		assert_eq!(packed[0].bytes(), &[0; 4]);
		assert_eq!(packed[1].bytes(), &[0; 4]);
	}
}

use std::collections::BTreeSet;

use recipe_core::{ByteCount, DType};

use crate::{LanguageError, LanguageErrorKind, LanguageResult};

/// A fixed-rank, fixed-extent tensor shape.
///
/// Empty extents are valid metadata and produce zero elements. They are
/// resolved as a no-dispatch calculation during planning rather than by
/// launching a fake one-lane kernel.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Shape { extents: Vec<u64>, elements: u64, }

impl Shape { pub fn new(extents: Vec<u64>) -> LanguageResult<Self> { if extents.is_empty() {
			return Err(LanguageError::new( LanguageErrorKind::EmptyShape,
				"scalar payloads use shape [1]; rank-zero payload shapes are not implicit",
			)); }
		let mut elements = 1_u64; for extent in &extents { elements = elements.checked_mul(*extent).ok_or_else(|| {
				LanguageError::new( LanguageErrorKind::ShapeOverflow,
					"shape element count overflowed u64",
				) })?; }
		Ok(Self { extents, elements }) }

	#[must_use]
	pub fn extents(&self) -> &[u64] { &self.extents }

	#[must_use]
	pub fn rank(&self) -> usize { self.extents.len() }

	#[must_use]
	pub const fn elements(&self) -> u64 { self.elements }

	#[must_use]
	pub const fn is_empty(&self) -> bool { self.elements == 0 }

	pub fn bytes(&self, dtype: DType) -> LanguageResult<ByteCount> { self.elements
			.checked_mul(u64::from(dtype.byte_width())) .map(ByteCount::new) .ok_or_else(|| { LanguageError::new(
					LanguageErrorKind::ByteSizeOverflow,
					"typed tensor byte size overflowed u64",
				) }) }

	pub fn broadcast_result(inputs: &[&Self]) -> LanguageResult<Self> {
		let rank = inputs.iter().map(|shape| shape.rank()).max().unwrap_or(0); if rank == 0 { return Err(LanguageError::new(
				LanguageErrorKind::EmptyShape,
				"broadcast requires at least one shaped input",
			)); }
		let mut extents = vec![1_u64; rank]; for shape in inputs { let leading = rank - shape.rank();
			for (index, extent) in shape.extents.iter().copied().enumerate() { let result = &mut extents[leading + index];
				if *result == 1 { *result = extent; } else if extent != 1 && extent != *result { return Err(LanguageError::new(
						LanguageErrorKind::ShapeMismatch, format!(
							"broadcast extent {extent} conflicts with {} at result axis {}",
							*result, leading + index ), )); } } }
		Self::new(extents) }

	pub fn reduced(&self, axes: &AxisSet, keep_dimensions: bool) -> LanguageResult<Self> {
		axes.validate_rank(self.rank())?; let mut result = Vec::with_capacity(self.rank());
		for (axis, extent) in self.extents.iter().copied().enumerate() { if axes.contains(axis) { if keep_dimensions {
					result.push(1); } } else { result.push(extent); } }
		if result.is_empty() { result.push(1); }
		Self::new(result) }

	pub fn gather_result(&self, axis: usize, indices: &Self) -> LanguageResult<Self> { if axis >= self.rank() {
			return Err(LanguageError::new( LanguageErrorKind::InvalidAxis,
				format!("gather axis {axis} is outside rank {}", self.rank()),
			)); }
		let mut extents = Vec::with_capacity(self.rank() - 1 + indices.rank());
		extents.extend_from_slice(&self.extents[..axis]); extents.extend_from_slice(indices.extents());
		extents.extend_from_slice(&self.extents[axis + 1..]); Self::new(extents) } }

/// Sorted, nonempty set of tensor axes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxisSet { axes: Vec<usize>, }

impl AxisSet { pub fn new(mut axes: Vec<usize>) -> LanguageResult<Self> { if axes.is_empty() {
			return Err(LanguageError::new( LanguageErrorKind::InvalidAxis,
				"axis set must not be empty",
			)); }
		axes.sort_unstable(); let unique = axes.iter().copied().collect::<BTreeSet<_>>(); if unique.len() != axes.len() {
			return Err(LanguageError::new( LanguageErrorKind::DuplicateAxis,
				"axis set contains a duplicate axis",
			)); }
		Ok(Self { axes }) }

	#[must_use]
	pub fn as_slice(&self) -> &[usize] { &self.axes }

	#[must_use]
	pub fn contains(&self, axis: usize) -> bool { self.axes.binary_search(&axis).is_ok() }

	pub fn validate_rank(&self, rank: usize) -> LanguageResult<()> {
		if let Some(axis) = self.axes.iter().copied().find(|axis| *axis >= rank) { return Err(LanguageError::new(
				LanguageErrorKind::InvalidAxis,
				format!("axis {axis} is outside rank {rank}"),
			)); }
		Ok(()) } }

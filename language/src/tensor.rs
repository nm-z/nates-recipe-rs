use recipe_core::{ByteCount, DType, ValueId};

use crate::{LanguageError, LanguageErrorKind, LanguageResult, Shape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContiguousOrder {
	RowMajor,
	ColumnMajor,
}

/// Static element layout. Strides and offset are metadata, not payload
/// calculation values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TensorLayout {
	pub offset_elements: u64,
	pub strides: Vec<u64>,
}

impl TensorLayout {
	pub fn contiguous(shape: &Shape, order: ContiguousOrder) -> LanguageResult<Self> {
		let mut strides = vec![0_u64; shape.rank()];
		let mut stride = 1_u64;
		match order {
			ContiguousOrder::RowMajor => {
				for (axis, extent) in shape.extents().iter().copied().enumerate().rev() {
					strides[axis] = stride;
					stride = stride.checked_mul(extent).ok_or_else(|| {
						LanguageError::new(
							LanguageErrorKind::InvalidLayout,
							"row-major stride overflowed",
						)
					})?;
				}
			}
			ContiguousOrder::ColumnMajor => {
				for (axis, extent) in shape.extents().iter().copied().enumerate() {
					strides[axis] = stride;
					stride = stride.checked_mul(extent).ok_or_else(|| {
						LanguageError::new(
							LanguageErrorKind::InvalidLayout,
							"column-major stride overflowed",
						)
					})?;
				}
			}
		}
		Ok(Self {
			offset_elements: 0,
			strides,
		})
	}

	pub fn validate(&self, shape: &Shape) -> LanguageResult<()> {
		if self.strides.len() != shape.rank() {
			return Err(LanguageError::new(
				LanguageErrorKind::InvalidLayout,
				format!(
					"layout has {} strides for rank {}",
					self.strides.len(),
					shape.rank()
				),
			));
		}
		if shape.elements() != 0
			&& self
				.strides
				.iter()
				.zip(shape.extents())
				.any(|(stride, extent)| *extent > 1 && *stride == 0)
		{
			return Err(LanguageError::new(
				LanguageErrorKind::InvalidLayout,
				"a non-singleton payload axis cannot have zero stride",
			));
		}
		if shape.elements() != 0 {
			let mut axes = shape
				.extents()
				.iter()
				.copied()
				.zip(self.strides.iter().copied())
				.filter(|(extent, _)| *extent > 1)
				.collect::<Vec<_>>();
			axes.sort_by_key(|(_, stride)| *stride);
			let mut occupied_span = 1_u64;
			for (extent, stride) in axes {
				if stride < occupied_span {
					return Err(LanguageError::new(
						LanguageErrorKind::InvalidLayout,
						"tensor layout maps multiple logical elements to the same storage element",
					));
				}
				occupied_span = (extent - 1)
					.checked_mul(stride)
					.and_then(|contribution| contribution.checked_add(occupied_span))
					.ok_or_else(|| {
						LanguageError::new(
							LanguageErrorKind::InvalidLayout,
							"layout non-overlap validation overflowed",
						)
					})?;
			}
		}
		self.span_elements(shape)?;
		Ok(())
	}

	pub fn span_elements(&self, shape: &Shape) -> LanguageResult<u64> {
		if shape.elements() == 0 {
			return Ok(0);
		}
		let mut last = self.offset_elements;
		for (extent, stride) in shape.extents().iter().zip(&self.strides) {
			let contribution = extent
				.saturating_sub(1)
				.checked_mul(*stride)
				.ok_or_else(|| {
					LanguageError::new(
						LanguageErrorKind::InvalidLayout,
						"layout span multiplication overflowed",
					)
				})?;
			last = last.checked_add(contribution).ok_or_else(|| {
				LanguageError::new(
					LanguageErrorKind::InvalidLayout,
					"layout span addition overflowed",
				)
			})?;
		}
		last.checked_add(1)
			.ok_or_else(|| LanguageError::new(LanguageErrorKind::InvalidLayout, "layout span overflowed"))
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tensor {
	pub id: ValueId,
	pub dtype: DType,
	pub shape: Shape,
	pub layout: TensorLayout,
	/// Size of the logical backing object before placement.
	pub storage_bytes: ByteCount,
	pub external_input: bool,
	pub external_output: bool,
}

impl Tensor {
	pub fn contiguous(id: ValueId, dtype: DType, shape: Shape, external_input: bool, external_output: bool) -> LanguageResult<Self> {
		let layout = TensorLayout::contiguous(&shape, ContiguousOrder::RowMajor)?;
		let storage_bytes = shape.bytes(dtype)?;
		Ok(Self {
			id,
			dtype,
			shape,
			layout,
			storage_bytes,
			external_input,
			external_output,
		})
	}

	pub fn validate(&self) -> LanguageResult<()> {
		self.layout
			.validate(&self.shape)
			.map_err(|error| error.for_value(self.id))?;
		let span = self
			.layout
			.span_elements(&self.shape)?
			.checked_mul(u64::from(self.dtype.byte_width()))
			.ok_or_else(|| {
				LanguageError::new(
					LanguageErrorKind::ByteSizeOverflow,
					"tensor layout span overflowed bytes",
				)
				.for_value(self.id)
			})?;
		if span > self.storage_bytes.get() {
			return Err(LanguageError::new(
				LanguageErrorKind::InvalidLayout,
				format!(
					"layout requires {span} bytes but storage declares {}",
					self.storage_bytes.get()
				),
			)
			.for_value(self.id));
		}
		Ok(())
	}
}

use recipe_core::ByteCount;

use crate::{OperationError, OperationErrorKind, OperationResult};

const MAX_I32_INDEX: u64 = i32::MAX as u64;

/// Immutable index metadata for a valid, stride-one, channelwise 1-D
/// convolution.
///
/// Logical input is `[batch, input_length, input_channels]`, filters are
/// `[kernel_size, input_channels, filters]`, and output is
/// `[batch, output_length, filters]` where
/// `output_length = input_length - kernel_size + 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelwiseConvolution1dPreparation {
	batch: u64,
	input_length: u64,
	input_channels: u64,
	filters: u64,
	kernel_size: u64,
	output_length: u64,
	input_elements: u64,
	output_elements: u64,
	window_indices: Vec<i32>,
	input_gradient_indices: Vec<i32>,
	input_gradient_validity: Vec<u32>,
	forward_workspace: ByteCount,
	backward_workspace: ByteCount,
}

impl ChannelwiseConvolution1dPreparation {
	#[must_use]
	pub const fn batch(&self) -> u64 { self.batch }

	#[must_use]
	pub const fn input_length(&self) -> u64 { self.input_length }

	#[must_use]
	pub const fn input_channels(&self) -> u64 { self.input_channels }

	#[must_use]
	pub const fn filters(&self) -> u64 { self.filters }

	#[must_use]
	pub const fn kernel_size(&self) -> u64 { self.kernel_size }

	#[must_use]
	pub const fn output_length(&self) -> u64 { self.output_length }

	#[must_use]
	pub const fn input_elements(&self) -> u64 { self.input_elements }

	#[must_use]
	pub const fn output_elements(&self) -> u64 { self.output_elements }

	/// Absolute flat-input indices in row-major
	/// `[batch, output_length, kernel_size, input_channels]` order.
	#[must_use]
	pub fn window_indices(&self) -> &[i32] { &self.window_indices }

	/// Absolute flat-output-gradient indices in row-major
	/// `[batch, input_length, kernel_size, filters]` order. Invalid boundary
	/// contributions use index zero and are removed by the matching validity
	/// image.
	#[must_use]
	pub fn input_gradient_indices(&self) -> &[i32] { &self.input_gradient_indices }

	/// Exact f32 zero/one bits matching [`Self::input_gradient_indices`].
	#[must_use]
	pub fn input_gradient_validity(&self) -> &[u32] { &self.input_gradient_validity }

	#[must_use]
	pub const fn input_shape(&self) -> [u64; 1] { [self.input_elements] }

	#[must_use]
	pub const fn window_indices_shape(&self) -> [u64; 4] {
		[
			self.batch,
			self.output_length,
			self.kernel_size,
			self.input_channels,
		]
	}

	#[must_use]
	pub const fn output_shape(&self) -> [u64; 3] { [self.batch, self.output_length, self.filters] }

	#[must_use]
	pub const fn input_gradient_indices_shape(&self) -> [u64; 4] {
		[
			self.batch,
			self.input_length,
			self.kernel_size,
			self.filters,
		]
	}

	#[must_use]
	pub const fn forward_workspace(&self) -> ByteCount { self.forward_workspace }

	#[must_use]
	pub const fn backward_workspace(&self) -> ByteCount { self.backward_workspace }
}

/// Prepare all checked receptive-field and deterministic input-gradient
/// coordinates for one valid, stride-one 1-D convolution.
pub fn prepare_channelwise_convolution_1d(
	batch: u64,
	input_length: u64,
	input_channels: u64,
	filters: u64,
	kernel_size: u64,
) -> OperationResult<ChannelwiseConvolution1dPreparation> {
	for (name, value) in [
		("batch", batch),
		("input_length", input_length),
		("input_channels", input_channels),
		("filters", filters),
		("kernel_size", kernel_size),
	] {
		if value == 0 {
			return Err(OperationError::new(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{name} must be positive"),
			));
		}
	}
	if kernel_size > input_length {
		return Err(OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("kernel_size {kernel_size} exceeds input_length {input_length}"),
		));
	}
	let output_length = input_length - kernel_size + 1;
	let input_elements = checked_product(
		&[batch, input_length, input_channels],
		"convolution input elements",
	)?;
	let output_elements = checked_product(
		&[batch, output_length, filters],
		"convolution output elements",
	)?;
	if input_elements > MAX_I32_INDEX || output_elements > MAX_I32_INDEX {
		return Err(OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			"convolution input and output must fit the checked int32 index domain",
		));
	}
	let window_elements = checked_product(
		&[batch, output_length, kernel_size, input_channels],
		"convolution receptive-field elements",
	)?;
	let input_gradient_elements = checked_product(
		&[batch, input_length, kernel_size, filters],
		"convolution input-gradient contributions",
	)?;
	let window_capacity = usize::try_from(window_elements).map_err(|error| {
		OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("convolution receptive-field table does not fit host usize: {error}"),
		)
	})?;
	let input_gradient_capacity = usize::try_from(input_gradient_elements).map_err(|error| {
		OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("convolution input-gradient table does not fit host usize: {error}"),
		)
	})?;

	let mut window_indices = Vec::with_capacity(window_capacity);
	for batch_index in 0..batch {
		for output_index in 0..output_length {
			for kernel_index in 0..kernel_size {
				for channel in 0..input_channels {
					let input_index = (((batch_index * input_length + output_index + kernel_index)
						* input_channels) + channel) as i32;
					window_indices.push(input_index);
				}
			}
		}
	}

	let mut input_gradient_indices = Vec::with_capacity(input_gradient_capacity);
	let mut input_gradient_validity = Vec::with_capacity(input_gradient_capacity);
	for batch_index in 0..batch {
		for input_index in 0..input_length {
			for kernel_index in 0..kernel_size {
				let output_index = input_index.checked_sub(kernel_index);
				let valid = output_index.is_some_and(|output_index| output_index < output_length);
				for filter in 0..filters {
					let gradient_index = output_index.filter(|_| valid).map_or(0, |output_index| {
						((batch_index * output_length + output_index) * filters + filter) as i32
					});
					input_gradient_indices.push(gradient_index);
					input_gradient_validity.push(if valid {
						1.0_f32.to_bits()
					} else {
						0.0_f32.to_bits()
					});
				}
			}
		}
	}

	let forward_workspace = workspace_bytes(window_elements, "convolution forward workspace")?;
	let backward_workspace = workspace_bytes(
		window_elements
			.checked_add(input_gradient_elements)
			.ok_or_else(|| overflow("convolution backward workspace elements"))?,
		"convolution backward workspace",
	)?;
	Ok(ChannelwiseConvolution1dPreparation {
		batch,
		input_length,
		input_channels,
		filters,
		kernel_size,
		output_length,
		input_elements,
		output_elements,
		window_indices,
		input_gradient_indices,
		input_gradient_validity,
		forward_workspace,
		backward_workspace,
	})
}

fn checked_product(values: &[u64], role: &str) -> OperationResult<u64> {
	values.iter().try_fold(1_u64, |product, value| {
		product.checked_mul(*value).ok_or_else(|| overflow(role))
	})
}

fn workspace_bytes(elements: u64, role: &str) -> OperationResult<ByteCount> {
	elements
		.checked_mul(4)
		.map(ByteCount::new)
		.ok_or_else(|| overflow(role))
}

fn overflow(role: &str) -> OperationError {
	OperationError::new(
		OperationErrorKind::UnsupportedConcreteShape,
		format!("{role} overflowed u64"),
	)
}

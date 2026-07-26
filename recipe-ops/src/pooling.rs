use recipe_core::ByteCount;

use crate::{OperationError, OperationErrorKind, OperationResult, PreparedParameter, PreparedParameters};

const MAX_I32_INDEX: u64 = i32::MAX as u64;

/// Static metadata for Recipe's channelwise, non-overlapping one-dimensional
/// maximum-pool composition.
///
/// Logical values are `[batch, input_length, channels]`. The materializer uses
/// a flat view of that storage and emits `[batch, groups, channels]`, where
/// `groups = ceil(input_length / pool_size)`. A final short window is retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChannelwiseMaxPool1dPreparation {
	batch: u64,
	input_length: u64,
	channels: u64,
	pool_size: u64,
	groups: u64,
	window_width: u64,
	input_elements: u64,
	output_elements: u64,
	window_indices: Vec<i32>,
	winner_bases: Vec<i32>,
	gradient_batch_indices: Vec<i32>,
	forward_workspace: ByteCount,
	backward_workspace: ByteCount,
}

impl ChannelwiseMaxPool1dPreparation {
	#[must_use]
	pub const fn batch(&self) -> u64 {
		self.batch
	}

	#[must_use]
	pub const fn input_length(&self) -> u64 {
		self.input_length
	}

	#[must_use]
	pub const fn channels(&self) -> u64 {
		self.channels
	}

	#[must_use]
	pub const fn pool_size(&self) -> u64 {
		self.pool_size
	}

	#[must_use]
	pub const fn groups(&self) -> u64 {
		self.groups
	}

	/// Width of the prepared rectangular window table. This is `pool_size`
	/// unless the pool is wider than the complete input.
	#[must_use]
	pub const fn window_width(&self) -> u64 {
		self.window_width
	}

	#[must_use]
	pub const fn input_elements(&self) -> u64 {
		self.input_elements
	}

	#[must_use]
	pub const fn output_elements(&self) -> u64 {
		self.output_elements
	}

	/// Row-major `[batch, groups, channels, window_width]` absolute indices
	/// into the flat input. Slots after the final short window repeat its final
	/// real coordinate.
	#[must_use]
	pub fn window_indices(&self) -> &[i32] {
		&self.window_indices
	}

	/// Row-major `[batch, groups, channels]` absolute first coordinates. The
	/// forward composition adds `local_winner * channels` to each base.
	#[must_use]
	pub fn winner_bases(&self) -> &[i32] {
		&self.winner_bases
	}

	/// Identity batch coordinates used by the backward composition's checked
	/// gather stage.
	#[must_use]
	pub fn gradient_batch_indices(&self) -> &[i32] {
		&self.gradient_batch_indices
	}

	#[must_use]
	pub const fn input_shape(&self) -> [u64; 1] {
		[self.input_elements]
	}

	#[must_use]
	pub const fn window_indices_shape(&self) -> [u64; 4] {
		[self.batch, self.groups, self.channels, self.window_width]
	}

	#[must_use]
	pub const fn output_shape(&self) -> [u64; 3] {
		[self.batch, self.groups, self.channels]
	}

	#[must_use]
	pub const fn forward_workspace(&self) -> ByteCount {
		self.forward_workspace
	}

	#[must_use]
	pub const fn backward_workspace(&self) -> ByteCount {
		self.backward_workspace
	}

	#[must_use]
	pub fn forward_parameters(&self, tree_lanes: u64) -> PreparedParameters {
		[
			("batch".to_owned(), PreparedParameter::U64(self.batch)),
			(
				"channelwise_nonoverlap".to_owned(),
				PreparedParameter::Bool(true),
			),
			("channels".to_owned(), PreparedParameter::U64(self.channels)),
			(
				"input_length".to_owned(),
				PreparedParameter::U64(self.input_length),
			),
			(
				"pool_size".to_owned(),
				PreparedParameter::U64(self.pool_size),
			),
			("tree_lanes".to_owned(), PreparedParameter::U64(tree_lanes)),
			(
				"tail_window_repeats_last_coordinate".to_owned(),
				PreparedParameter::Bool(true),
			),
			(
				"window_indices_encode_channelwise_nonoverlap".to_owned(),
				PreparedParameter::Bool(true),
			),
			(
				"winner_bases_encode_channelwise_nonoverlap".to_owned(),
				PreparedParameter::Bool(true),
			),
		]
		.into_iter()
		.collect()
	}

	#[must_use]
	pub fn backward_parameters(&self) -> PreparedParameters {
		[
			("batch".to_owned(), PreparedParameter::U64(self.batch)),
			(
				"channelwise_nonoverlap".to_owned(),
				PreparedParameter::Bool(true),
			),
			("channels".to_owned(), PreparedParameter::U64(self.channels)),
			(
				"gradient_batch_indices_identity".to_owned(),
				PreparedParameter::Bool(true),
			),
			(
				"input_gradient_base_zero".to_owned(),
				PreparedParameter::Bool(true),
			),
			(
				"input_length".to_owned(),
				PreparedParameter::U64(self.input_length),
			),
			(
				"pool_size".to_owned(),
				PreparedParameter::U64(self.pool_size),
			),
			(
				"winning_indices_from_matching_forward".to_owned(),
				PreparedParameter::Bool(true),
			),
			(
				"winning_indices_unique".to_owned(),
				PreparedParameter::Bool(true),
			),
		]
		.into_iter()
		.collect()
	}
}

/// Prepare the immutable index metadata for a channelwise, non-overlapping
/// one-dimensional maximum pool.
pub fn prepare_channelwise_max_pool_1d(
	batch: u64,
	input_length: u64,
	channels: u64,
	pool_size: u64,
) -> OperationResult<ChannelwiseMaxPool1dPreparation> {
	for (name, value) in [
		("batch", batch),
		("input_length", input_length),
		("channels", channels),
		("pool_size", pool_size),
	] {
		if value == 0 {
			return Err(OperationError::new(
				OperationErrorKind::UnsupportedConcreteShape,
				format!("{name} must be positive"),
			));
		}
	}

	let input_elements = checked_product(
		&[batch, input_length, channels],
		"maximum-pool input elements",
	)?;
	if input_elements > MAX_I32_INDEX {
		return Err(OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			"maximum-pool input elements must fit the int32 index domain",
		));
	}
	let groups = input_length.div_ceil(pool_size);
	let window_width = input_length.min(pool_size);
	let output_elements = checked_product(&[batch, groups, channels], "maximum-pool output elements")?;
	let window_entries = checked_product(
		&[output_elements, window_width],
		"maximum-pool window-index entries",
	)?;
	if window_entries > MAX_I32_INDEX {
		return Err(OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			"maximum-pool window-index entries must fit the int32 index domain",
		));
	}

	let mut window_indices = Vec::with_capacity(usize_extent(window_entries, "window-index entries")?);
	let mut winner_bases = Vec::with_capacity(usize_extent(output_elements, "winner bases")?);
	let gradient_batch_indices = (0..batch)
		.map(|index| i32_coordinate(index, "maximum-pool gradient batch coordinate"))
		.collect::<OperationResult<Vec<_>>>()?;
	for batch_index in 0..batch {
		for group in 0..groups {
			let group_start = group
				.checked_mul(pool_size)
				.ok_or_else(|| arithmetic_overflow("maximum-pool group start"))?;
			for channel in 0..channels {
				let base = flat_coordinate(batch_index, group_start, channel, input_length, channels)?;
				winner_bases.push(i32_coordinate(base, "maximum-pool winner base")?);
				for offset in 0..window_width {
					let position = group_start
						.checked_add(offset)
						.ok_or_else(|| arithmetic_overflow("maximum-pool window position"))?
						.min(input_length - 1);
					let coordinate = flat_coordinate(batch_index, position, channel, input_length, channels)?;
					window_indices.push(i32_coordinate(
						coordinate,
						"maximum-pool window coordinate",
					)?);
				}
			}
		}
	}

	let workspace_elements = window_entries
		.checked_add(
			output_elements
				.checked_mul(2)
				.ok_or_else(|| arithmetic_overflow("maximum-pool result workspace"))?,
		)
		.ok_or_else(|| arithmetic_overflow("maximum-pool forward workspace"))?;
	let forward_workspace = ByteCount::new(
		workspace_elements
			.checked_mul(4)
			.ok_or_else(|| arithmetic_overflow("maximum-pool forward workspace bytes"))?,
	);
	let backward_workspace = ByteCount::new(
		output_elements
			.checked_mul(12)
			.ok_or_else(|| arithmetic_overflow("maximum-pool backward workspace bytes"))?,
	);

	Ok(ChannelwiseMaxPool1dPreparation {
		batch,
		input_length,
		channels,
		pool_size,
		groups,
		window_width,
		input_elements,
		output_elements,
		window_indices,
		winner_bases,
		gradient_batch_indices,
		forward_workspace,
		backward_workspace,
	})
}

fn flat_coordinate(batch: u64, position: u64, channel: u64, input_length: u64, channels: u64) -> OperationResult<u64> {
	batch.checked_mul(input_length)
		.and_then(|value| value.checked_add(position))
		.and_then(|value| value.checked_mul(channels))
		.and_then(|value| value.checked_add(channel))
		.ok_or_else(|| arithmetic_overflow("maximum-pool flat coordinate"))
}

fn checked_product(factors: &[u64], role: &str) -> OperationResult<u64> {
	factors.iter().try_fold(1_u64, |product, factor| {
		product
			.checked_mul(*factor)
			.ok_or_else(|| arithmetic_overflow(role))
	})
}

fn usize_extent(value: u64, role: &str) -> OperationResult<usize> {
	usize::try_from(value).map_err(|error| {
		OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{role} does not fit the host address domain: {error}"),
		)
	})
}

fn i32_coordinate(value: u64, role: &str) -> OperationResult<i32> {
	i32::try_from(value).map_err(|error| {
		OperationError::new(
			OperationErrorKind::UnsupportedConcreteShape,
			format!("{role} does not fit int32: {error}"),
		)
	})
}

fn arithmetic_overflow(role: &str) -> OperationError {
	OperationError::new(
		OperationErrorKind::WorkspaceArithmeticOverflow,
		format!("{role} overflowed u64"),
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn reference_forward(prepared: &ChannelwiseMaxPool1dPreparation, values: &[f32]) -> (Vec<f32>, Vec<i32>) {
		assert_eq!(
			values.len(),
			usize::try_from(prepared.input_elements()).unwrap()
		);
		let rows = usize::try_from(prepared.output_elements()).unwrap();
		let window = usize::try_from(prepared.window_width()).unwrap();
		let channels = i32::try_from(prepared.channels()).unwrap();
		let mut pooled = Vec::with_capacity(rows);
		let mut winners = Vec::with_capacity(rows);
		for row in 0..rows {
			let indices = &prepared.window_indices()[row * window..(row + 1) * window];
			let mut local_winner = 0_usize;
			let mut maximum = values[usize::try_from(indices[0]).unwrap()];
			for (local, coordinate) in indices.iter().copied().enumerate().skip(1) {
				let candidate = values[usize::try_from(coordinate).unwrap()];
				if candidate > maximum {
					maximum = candidate;
					local_winner = local;
				}
			}
			let winner = prepared.winner_bases()[row] + i32::try_from(local_winner).unwrap() * channels;
			assert_eq!(winner, indices[local_winner]);
			pooled.push(maximum);
			winners.push(winner);
		}
		(pooled, winners)
	}

	fn reference_backward(
		prepared: &ChannelwiseMaxPool1dPreparation,
		winners: &[i32],
		output_gradient: &[f32],
	) -> Vec<f32> {
		assert_eq!(
			winners.len(),
			usize::try_from(prepared.output_elements()).unwrap()
		);
		assert_eq!(output_gradient.len(), winners.len());
		let mut input_gradient = vec![0.0_f32; usize::try_from(prepared.input_elements()).unwrap()];
		let mut written = vec![false; input_gradient.len()];
		for (winner, gradient) in winners.iter().copied().zip(output_gradient.iter().copied()) {
			let winner = usize::try_from(winner).unwrap();
			assert!(
				!written[winner],
				"channelwise non-overlap produced a duplicate winner"
			);
			written[winner] = true;
			input_gradient[winner] = gradient;
		}
		input_gradient
	}

	#[test]
	fn preparation_retains_tail_and_never_crosses_channels() {
		let prepared = prepare_channelwise_max_pool_1d(1, 5, 2, 2).unwrap();
		assert_eq!(prepared.input_shape(), [10]);
		assert_eq!(prepared.window_indices_shape(), [1, 3, 2, 2]);
		assert_eq!(prepared.output_shape(), [1, 3, 2]);
		assert_eq!(
			prepared.window_indices(),
			[0, 2, 1, 3, 4, 6, 5, 7, 8, 8, 9, 9]
		);
		assert_eq!(prepared.winner_bases(), [0, 1, 4, 5, 8, 9]);
		assert_eq!(prepared.gradient_batch_indices(), [0]);
		assert_eq!(prepared.forward_workspace(), ByteCount::new(96));
		assert_eq!(prepared.backward_workspace(), ByteCount::new(72));
	}

	#[test]
	fn pool_wider_than_input_uses_one_real_width_window() {
		let prepared = prepare_channelwise_max_pool_1d(2, 3, 2, u64::MAX).unwrap();
		assert_eq!(prepared.groups(), 1);
		assert_eq!(prepared.window_width(), 3);
		assert_eq!(prepared.window_indices_shape(), [2, 1, 2, 3]);
		assert_eq!(
			prepared.window_indices(),
			[0, 2, 4, 1, 3, 5, 6, 8, 10, 7, 9, 11]
		);
	}

	#[test]
	fn size_one_is_an_exact_identity_across_batches_and_channels() {
		let prepared = prepare_channelwise_max_pool_1d(2, 3, 2, 1).unwrap();
		assert_eq!(prepared.groups(), 3);
		assert_eq!(prepared.window_width(), 1);
		assert_eq!(prepared.output_shape(), [2, 3, 2]);
		assert_eq!(prepared.window_indices(), (0_i32..12).collect::<Vec<_>>());
		assert_eq!(prepared.winner_bases(), (0_i32..12).collect::<Vec<_>>());
		let values = (0_u16..12).map(f32::from).collect::<Vec<_>>();
		let (pooled, winners) = reference_forward(&prepared, &values);
		assert_eq!(pooled, values);
		assert_eq!(winners, (0_i32..12).collect::<Vec<_>>());
		let output_gradient = (1_u16..=12).map(f32::from).collect::<Vec<_>>();
		assert_eq!(
			reference_backward(&prepared, &winners, &output_gradient),
			output_gradient
		);
	}

	#[test]
	fn ties_tail_batches_and_channels_use_lowest_global_winners_and_exact_backward_routes() {
		let prepared = prepare_channelwise_max_pool_1d(2, 5, 2, 3).unwrap();
		assert_eq!(prepared.output_shape(), [2, 2, 2]);
		assert_eq!(prepared.window_indices_shape(), [2, 2, 2, 3]);
		assert_eq!(
			prepared.window_indices(),
			[
				0, 2, 4, 1, 3, 5, 6, 8, 8, 7, 9, 9, 10, 12, 14, 11, 13, 15, 16, 18, 18, 17, 19, 19,
			]
		);
		let values = [
			5.0, 1.0, 5.0, 9.0, 2.0, 9.0, 7.0, 4.0, 7.0, 4.0, 1.0, 8.0, 4.0, 8.0, 4.0, 3.0, 6.0, 10.0, 9.0, 10.0,
		];
		let (pooled, winners) = reference_forward(&prepared, &values);
		assert_eq!(pooled, [5.0, 9.0, 7.0, 4.0, 4.0, 8.0, 9.0, 10.0]);
		assert_eq!(winners, [0, 3, 6, 7, 12, 11, 18, 17]);
		let output_gradient = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
		assert_eq!(
			reference_backward(&prepared, &winners, &output_gradient),
			[
				1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 6.0, 5.0, 0.0, 0.0, 0.0, 0.0, 8.0, 7.0,
				0.0,
			]
		);
	}

	#[test]
	fn zero_extent_and_unindexable_payload_fail_before_allocation() {
		assert_eq!(
			prepare_channelwise_max_pool_1d(1, 0, 1, 2)
				.unwrap_err()
				.kind,
			OperationErrorKind::UnsupportedConcreteShape
		);
		assert_eq!(
			prepare_channelwise_max_pool_1d(i32::MAX as u64, 2, 1, 1)
				.unwrap_err()
				.kind,
			OperationErrorKind::UnsupportedConcreteShape
		);
	}
}

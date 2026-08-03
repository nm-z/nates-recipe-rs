use super::super::*;
use crate::{
	CheckpointPoolImage,
	inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler,
	},
};

impl InferenceBlock for CheckpointPoolImage {
	fn compile_inference(
		&self,
		compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> {
		Ok(BlockInference {
			output: compiler.compile_pool(
				context.block_index,
				self,
				context.input,
				[
					context.rows,
					context.width,
					context.logical_length,
					context.logical_channels,
				],
				context.tree_lanes,
			)?,
			width: self.output_width().get(),
			logical_length: self.output_length().get(),
			logical_channels: self.channels().get(),
		})
	}
}

impl DeclaredBlock for DensePool {
	fn clone_box(&self) -> Box<dyn DeclaredBlock> {
		Box::new(*self)
	}

	fn compile_forward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> {
		let (logical, state) = context.logical.pooled(*self)?;
		let (output, preparation, winners, input_matrix_indices, output_group_indices) = compiler
			.compile_pool_forward(
				context.input,
				[context.partition_rows, self.size().get()],
				state,
				[
					ExternalInputRole::TrainingPoolWindowIndices {
						block: context.block_index,
					},
					ExternalInputRole::TrainingPoolWinnerBases {
						block: context.block_index,
					},
				],
				context.config.reduction_tree_lanes,
				compiler.training_domain,
			)?;
		let gradient_batch_indices = compiler.external_i32_tensor(
			ExternalInputRole::TrainingPoolGradientBatchIndices {
				block: context.block_index,
			},
			shape(&[context.partition_rows])?,
			preparation.gradient_batch_indices(),
		)?;
		Ok(BlockForward {
			output,
			logical,
			routing: self
				.routing(logical.length)
				.map(|routing| (routing, logical.channels)),
			tape: Box::new(PoolValues {
				declaration: *self,
				state,
				preparation,
				winners,
				gradient_batch_indices,
				input_matrix_indices,
				output_group_indices,
			}),
		})
	}
}

impl RealizedBlock for RealizedPool {
	fn clone_box(&self) -> Box<dyn RealizedBlock> {
		Box::new(*self)
	}

	fn visit_parameter_states(&self, _visit: &mut dyn FnMut(ParameterState)) {}

	fn compile_validation(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> {
		let logical = context.logical.pooled(self.declaration)?.0;
		let output = compiler
			.compile_pool_forward(
				context.input,
				[context.rows, self.declaration.size().get()],
				self.state,
				[
					ExternalInputRole::ValidationPoolWindowIndices {
						block: context.block_index,
					},
					ExternalInputRole::ValidationPoolWinnerBases {
						block: context.block_index,
					},
				],
				context.config.reduction_tree_lanes,
				context.domain,
			)?
			.0;
		Ok(BlockValidation {
			output,
			logical,
			routing: self
				.declaration
				.routing(logical.length)
				.map(|routing| (routing, logical.channels)),
		})
	}

	fn checkpoint(
		&self,
		_training: &CompiledTraining,
		index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_pool(self.declaration, self.state, index)
	}
}

impl CompiledBlock for PoolValues {
	fn backward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> {
		let input_width = self
			.state
			.input_width()
			.expect("validated pool input width");
		let output_width = self
			.state
			.output_width()
			.expect("validated pool output width");
		compiler.require_tensor(
			context.gradient,
			DType::F32,
			&[context.partition_rows, output_width.get()],
			"maximum-pool output gradient",
		)?;
		let gradient = compiler.mask_f32_with_zero(context.gradient, context.validity, compiler.training_domain)?;
		let grouped_gradient = compiler.gather(
			gradient,
			self.output_group_indices,
			shape(&[
				context.partition_rows,
				self.state.output_length().get(),
				self.state.channels().get(),
			])?,
			1,
			compiler.training_domain,
		)?;
		let flat_shape = shape(&[self.preparation.input_elements()])?;
		let input_gradient_base = compiler.zero_f32_tensor(flat_shape.clone())?;
		let flat_gradient = compiler.tensor(DType::F32, flat_shape)?;
		compiler.materialize(
			"recipe_max_pool_1d_backward",
			&[
				("output_gradient", grouped_gradient),
				("winning_indices", self.winners),
				("gradient_batch_indices", self.gradient_batch_indices),
				("input_gradient_base", input_gradient_base),
			],
			&[("input_gradient", flat_gradient)],
			"output_gradient",
			&self.preparation.backward_parameters(),
			compiler.training_domain,
		)?;
		let input_gradient = compiler.gather(
			flat_gradient,
			self.input_matrix_indices,
			shape(&[context.partition_rows, input_width.get()])?,
			0,
			compiler.training_domain,
		)?;
		Ok(BlockBackward {
			input_gradient: Some(input_gradient),
			parameters: Vec::new(),
		})
	}

	fn optimize(
		&self,
		_compiler: &mut GraphCompiler,
		_updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> {
		Ok(DenseBlockState::from_realized(Box::new(RealizedPool {
			declaration: self.declaration,
			state: self.state,
		})))
	}
}

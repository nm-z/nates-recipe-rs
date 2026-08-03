use super::super::*;
use crate::{
	CheckpointAttentionImage,
	inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileError, InferenceCompileErrorKind,
		InferenceCompileResult, InferenceGraphCompiler,
	},
};

impl InferenceBlock for CheckpointAttentionImage {
	fn compile_inference(
		&self,
		compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> {
		let width = self
			.sequence_length()
			.get()
			.checked_mul(self.dimensions().get())
			.ok_or_else(|| {
				InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					"attention inference output width overflowed u64",
				)
			})?;
		Ok(BlockInference {
			output: compiler.compile_attention(
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
			width,
			logical_length: self.sequence_length().get(),
			logical_channels: self.dimensions().get(),
		})
	}
}

impl DeclaredBlock for DenseAttention {
	fn clone_box(&self) -> Box<dyn DeclaredBlock> {
		Box::new(*self)
	}

	fn compile_forward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> {
		let (logical, head_dimension) = context.logical.attended(*self)?;
		let sequence_length = context.logical.length;
		let dimensions = context.logical.channels;
		let width = checked_product(
			&[sequence_length.get(), dimensions.get()],
			"attention input width",
		)?;
		compiler.require_tensor(
			context.input,
			DType::F32,
			&[context.partition_rows, width],
			"causal-attention input",
		)?;
		checked_i32(
			checked_product(
				&[
					context.partition_rows,
					sequence_length.get(),
					dimensions.get(),
				],
				"attention element count",
			)?,
			"attention element count",
		)?;
		let matrix_shape = shape(&[dimensions.get(), dimensions.get()])?;
		let query = compiler.initialize_weight(
			matrix_shape.clone(),
			dimensions.get(),
			context.config.random_seed,
		)?;
		let key = compiler.initialize_weight(
			matrix_shape.clone(),
			dimensions.get(),
			context.config.random_seed,
		)?;
		let value = compiler.initialize_weight(
			matrix_shape.clone(),
			dimensions.get(),
			context.config.random_seed,
		)?;
		let output_parameter =
			compiler.initialize_weight(matrix_shape, dimensions.get(), context.config.random_seed)?;
		let (output, forward) = compiler.compile_attention_forward(
			context.input,
			[
				context.partition_rows,
				sequence_length.get(),
				dimensions.get(),
				self.heads().get(),
				head_dimension.get(),
			],
			AttentionForwardParameters {
				query: query.value,
				key: key.value,
				value: value.value,
				output: output_parameter.value,
			},
			context.config.reduction_tree_lanes,
			compiler.training_domain,
		)?;
		Ok(BlockForward {
			output,
			logical,
			routing: None,
			tape: Box::new(AttentionValues {
				declaration: *self,
				input: forward.input,
				queries: forward.queries,
				keys: forward.keys,
				values: forward.values,
				probabilities: forward.probabilities,
				context: forward.context,
				query,
				key,
				value,
				output: output_parameter,
				sequence_length,
				dimensions,
				heads: self.heads(),
				head_dimension,
			}),
		})
	}
}

impl RealizedBlock for RealizedAttention {
	fn clone_box(&self) -> Box<dyn RealizedBlock> {
		Box::new(self.clone())
	}

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) {
		for parameter in [
			self.state.query,
			self.state.key,
			self.state.value,
			self.state.output,
		] {
			visit(parameter);
		}
	}

	fn compile_validation(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> {
		let sequence = self.state.sequence_length.get();
		let dimensions = self.state.dimensions.get();
		let width = checked_product(&[sequence, dimensions], "validation attention width")?;
		compiler.require_tensor(
			context.input,
			DType::F32,
			&[context.rows, width],
			"validation causal-attention input",
		)?;
		Ok(BlockValidation {
			output: compiler
				.compile_attention_forward(
					context.input,
					[
						context.rows,
						sequence,
						dimensions,
						self.state.heads.get(),
						self.state.head_dimension.get(),
					],
					AttentionForwardParameters {
						query: self.state.query.updated_parameter,
						key: self.state.key.updated_parameter,
						value: self.state.value.updated_parameter,
						output: self.state.output.updated_parameter,
					},
					context.config.reduction_tree_lanes,
					context.domain,
				)?
				.0,
			logical: context.logical.attended(self.declaration)?.0,
			routing: None,
		})
	}

	fn checkpoint(
		&self,
		training: &CompiledTraining,
		_index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_attention(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Attention)
	}
}

impl CompiledBlock for AttentionValues {
	fn backward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> {
		let gradient = context.gradient;
		let rows = context.partition_rows;
		let validity = context.validity;
		let tree_lanes = context.tree_lanes;
		let result: TrainingCompileResult<(Vec<ParameterGradient>, ValueId)> = {
			let sequence = self.sequence_length.get();
			let dimensions = self.dimensions.get();
			let heads = self.heads.get();
			let head_dimension = self.head_dimension.get();
			let width = checked_product(&[sequence, dimensions], "attention backward width")?;
			compiler.require_tensor(
				gradient,
				DType::F32,
				&[rows, width],
				"causal-attention output gradient",
			)?;
			let gradient = compiler.mask_f32_with_zero(gradient, validity, compiler.training_domain)?;
			let gradient_sequence = compiler.reinterpret_tensor(
				gradient,
				shape(&[rows, sequence, dimensions])?,
				compiler.training_domain,
			)?;

			let output_gradient = compiler.attention_weight_gradient(
				self.context,
				gradient_sequence,
				dimensions,
				compiler.training_domain,
			)?;
			let context_sequence_gradient = compiler.attention_input_gradient(
				[gradient_sequence, self.output.value],
				[rows, sequence, dimensions],
				compiler.training_domain,
			)?;
			let context_heads = compiler.reinterpret_tensor(
				context_sequence_gradient,
				shape(&[rows, sequence, heads, head_dimension])?,
				compiler.training_domain,
			)?;
			let context_head_gradient = compiler.sequence_to_head_major(
				context_heads,
				rows,
				sequence,
				heads,
				head_dimension,
				compiler.training_domain,
			)?;

			let score_shape = shape(&[rows, heads, sequence, sequence])?;
			let probability_gradient = compiler.tensor(DType::F32, score_shape.clone())?;
			compiler.emit(
				vec![context_head_gradient, self.values],
				vec![probability_gradient],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0), (1, 2)],
					contract_axes: vec![(3, 3)],
				}),
				forbidden_aliases(2, 1),
				compiler.training_domain,
			)?;
			let value_gradient_heads =
				compiler.tensor(DType::F32, shape(&[rows, heads, sequence, head_dimension])?)?;
			compiler.emit(
				vec![self.probabilities, context_head_gradient],
				vec![value_gradient_heads],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0), (1, 1)],
					contract_axes: vec![(2, 2)],
				}),
				forbidden_aliases(2, 1),
				compiler.training_domain,
			)?;

			let probability_product = compiler.elementwise_f32(
				score_shape.clone(),
				vec![probability_gradient, self.probabilities],
				multiply_program()?,
				compiler.training_domain,
			)?;
			let row_product_sum = compiler.tensor(DType::F32, shape(&[rows, heads, sequence, 1])?)?;
			compiler.reduce_value(
				[probability_product, row_product_sum],
				ReduceOperator::Sum,
				&[3],
				true,
				tree_lanes,
				compiler.training_domain,
			)?;
			let centered_probability_gradient = compiler.elementwise_f32(
				score_shape.clone(),
				vec![probability_gradient, row_product_sum],
				subtract_program()?,
				compiler.training_domain,
			)?;
			let scaled_score_gradient = compiler.elementwise_f32(
				score_shape.clone(),
				vec![self.probabilities, centered_probability_gradient],
				multiply_program()?,
				compiler.training_domain,
			)?;
			let score_gradient = compiler.elementwise_f32(
				score_shape,
				vec![scaled_score_gradient],
				multiply_constant_program(1.0 / (head_dimension as f32).sqrt())?,
				compiler.training_domain,
			)?;

			let head_gradient_shape = shape(&[rows, heads, sequence, head_dimension])?;
			let query_gradient_heads = compiler.tensor(DType::F32, head_gradient_shape.clone())?;
			compiler.emit(
				vec![score_gradient, self.keys],
				vec![query_gradient_heads],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0), (1, 2)],
					contract_axes: vec![(3, 1)],
				}),
				forbidden_aliases(2, 1),
				compiler.training_domain,
			)?;
			let key_gradient_heads = compiler.tensor(DType::F32, head_gradient_shape)?;
			compiler.emit(
				vec![score_gradient, self.queries],
				vec![key_gradient_heads],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0), (1, 2)],
					contract_axes: vec![(2, 1)],
				}),
				forbidden_aliases(2, 1),
				compiler.training_domain,
			)?;

			let query_gradient = compiler.attention_head_gradient_to_sequence(
				query_gradient_heads,
				rows,
				sequence,
				heads,
				head_dimension,
				dimensions,
			)?;
			let key_gradient = compiler.attention_head_gradient_to_sequence(
				key_gradient_heads,
				rows,
				sequence,
				heads,
				head_dimension,
				dimensions,
			)?;
			let value_gradient = compiler.attention_head_gradient_to_sequence(
				value_gradient_heads,
				rows,
				sequence,
				heads,
				head_dimension,
				dimensions,
			)?;

			let query_weight_gradient = compiler.attention_weight_gradient(
				self.input,
				query_gradient,
				dimensions,
				compiler.training_domain,
			)?;
			let key_weight_gradient = compiler.attention_weight_gradient(
				self.input,
				key_gradient,
				dimensions,
				compiler.training_domain,
			)?;
			let value_weight_gradient = compiler.attention_weight_gradient(
				self.input,
				value_gradient,
				dimensions,
				compiler.training_domain,
			)?;
			let query_input_gradient = compiler.attention_input_gradient(
				[query_gradient, self.query.value],
				[rows, sequence, dimensions],
				compiler.training_domain,
			)?;
			let key_input_gradient = compiler.attention_input_gradient(
				[key_gradient, self.key.value],
				[rows, sequence, dimensions],
				compiler.training_domain,
			)?;
			let value_input_gradient = compiler.attention_input_gradient(
				[value_gradient, self.value.value],
				[rows, sequence, dimensions],
				compiler.training_domain,
			)?;
			let input_sequence_gradient = compiler.elementwise_f32(
				shape(&[rows, sequence, dimensions])?,
				vec![
					query_input_gradient,
					key_input_gradient,
					value_input_gradient,
				],
				sum_program(3)?,
				compiler.training_domain,
			)?;
			let input_gradient = compiler.reinterpret_tensor(
				input_sequence_gradient,
				shape(&[rows, width])?,
				compiler.training_domain,
			)?;
			Ok((
				vec![
					ParameterGradient::new(ParameterRole::AttentionQuery, query_weight_gradient),
					ParameterGradient::new(ParameterRole::AttentionKey, key_weight_gradient),
					ParameterGradient::new(ParameterRole::AttentionValue, value_weight_gradient),
					ParameterGradient::new(ParameterRole::AttentionOutput, output_gradient),
				],
				input_gradient,
			))
		};
		let (parameters, input_gradient) = result?;
		Ok(BlockBackward {
			input_gradient: Some(input_gradient),
			parameters,
		})
	}

	fn optimize(
		&self,
		compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> {
		Ok(DenseBlockState::from_realized(Box::new(RealizedAttention {
			declaration: self.declaration,
			state: DenseAttentionState {
				sequence_length: self.sequence_length,
				dimensions: self.dimensions,
				heads: self.heads,
				head_dimension: self.head_dimension,
				query: updates.apply(compiler, ParameterRole::AttentionQuery, self.query)?,
				key: updates.apply(compiler, ParameterRole::AttentionKey, self.key)?,
				value: updates.apply(compiler, ParameterRole::AttentionValue, self.value)?,
				output: updates.apply(compiler, ParameterRole::AttentionOutput, self.output)?,
			},
		})))
	}
}

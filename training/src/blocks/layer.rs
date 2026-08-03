use super::super::*;
use crate::{
	CheckpointLayerImage,
	inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult,
		InferenceGraphCompiler, identity_exhausted,
	},
};

impl InferenceBlock for CheckpointLayerImage {
	fn compile_inference(
		&self,
		compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> {
		let width = self.declaration().width().get();
		let output = compiler.compile_layer(
			*context.layer_index,
			self,
			context.input,
			[context.rows, context.width],
			context.normalization_epsilon,
			context.tree_lanes,
		)?;
		*context.layer_index = context
			.layer_index
			.checked_add(1)
			.ok_or_else(identity_exhausted)?;
		Ok(BlockInference {
			output,
			width,
			logical_length: width,
			logical_channels: 1,
		})
	}
}

pub(super) struct LayerForwardContext<'a> {
	pub input: ValueId,
	pub input_width: u64,
	pub partition_rows: u64,
	pub validity: ValueId,
	pub valid_count: ValueId,
	pub routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>,
	pub config: &'a DenseTrainingConfig,
}

pub(super) struct LayerValidationContext<'a> {
	pub input: ValueId,
	pub rows: u64,
	pub routing: Option<(DenseGroupToNeuronRouting, NonZeroU64)>,
	pub config: &'a DenseTrainingConfig,
	pub domain: IterationDomain,
}

pub(super) struct LayerBackward {
	pub gradient: GradientPair,
	pub input_gradient: Option<ValueId>,
}

pub(super) trait LayerDeclarationLifecycle {
	fn compile_layer_forward(
		&self,
		compiler: &mut GraphCompiler,
		context: LayerForwardContext<'_>,
	) -> TrainingCompileResult<(ValueId, LayerValues)>;

	fn compile_layer_validation(
		&self,
		compiler: &mut GraphCompiler,
		state: &DenseLayerState,
		context: LayerValidationContext<'_>,
	) -> TrainingCompileResult<ValueId>;
}

pub(super) trait LayerTapeLifecycle {
	fn compile_layer_backward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockBackwardContext,
	) -> TrainingCompileResult<LayerBackward>;

	fn compile_layer_optimizer(
		&self,
		compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseLayerState>;
}

impl LayerDeclarationLifecycle for DenseLayer {
	fn compile_layer_forward(
		&self,
		compiler: &mut GraphCompiler,
		context: LayerForwardContext<'_>,
	) -> TrainingCompileResult<(ValueId, LayerValues)> {
		let output_width = self.width().get();
		let output_shape = shape(&[context.partition_rows, output_width])?;
		let routing_mask = context
			.routing
			.map(|(routing, channels)| {
				compiler.group_routing_mask(context.input_width, output_width, channels, routing)
			})
			.transpose()?;
		let mut weight = compiler.initialize_weight(
			shape(&[context.input_width, output_width])?,
			context.input_width,
			context.config.random_seed,
		)?;
		if let Some(mask) = routing_mask {
			let masked = compiler.elementwise_f32(
				shape(&[context.input_width, output_width])?,
				vec![weight.value, mask],
				masked_zero_f32_program()?,
				IterationDomain::first(),
			)?;
			weight.value = masked;
		}
		let forward_weight = weight.value;
		let bias = compiler.initialize_zero_parameter(shape(&[output_width])?)?;
		let preactivation = compiler.tensor(DType::F32, output_shape)?;
		compiler.materialize(
			"gpu_linear_into",
			&[
				("input", context.input),
				("weight", forward_weight),
				("bias", bias.value),
			],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(),
			compiler.training_domain,
		)?;
		let mut current = preactivation;
		let mut operations = Vec::with_capacity(self.operations().len());
		for operation in self.operations().iter().copied() {
			let (output, values) = compiler.compile_training_operation(
				current,
				operation,
				output_width,
				(context.partition_rows, context.validity, context.valid_count, context.config),
			)?;
			current = output;
			operations.push(values);
		}
		Ok((
			current,
			LayerValues {
				declaration: self.clone(),
				input: context.input,
				weight,
				forward_weight,
				routing_mask,
				bias,
				operations,
			},
		))
	}

	fn compile_layer_validation(
		&self,
		compiler: &mut GraphCompiler,
		state: &DenseLayerState,
		context: LayerValidationContext<'_>,
	) -> TrainingCompileResult<ValueId> {
		let output_width = self.width().get();
		let input_width = compiler.matrix_width(context.input, "validation dense input")?;
		let routing_mask = context
			.routing
			.map(|(routing, channels)| compiler.group_routing_mask(input_width, output_width, channels, routing))
			.transpose()?;
		let forward_weight = if let Some(mask) = routing_mask {
			compiler.elementwise_f32(
				shape(&[input_width, output_width])?,
				vec![state.weight.updated_parameter, mask],
				masked_zero_f32_program()?,
				context.domain,
			)?
		} else {
			state.weight.updated_parameter
		};
		let preactivation = compiler.tensor(DType::F32, shape(&[context.rows, output_width])?)?;
		compiler.materialize(
			"gpu_linear_into",
			&[
				("input", context.input),
				("weight", forward_weight),
				("bias", state.bias.updated_parameter),
			],
			&[("output", preactivation)],
			"input",
			&PreparedParameters::new(),
			context.domain,
		)?;
		let mut current = preactivation;
		let mut prelu_states = state.prelu.iter();
		for operation in self.operations().iter().copied() {
			let prelu = validation_prelu_parameter(operation, &mut prelu_states);
			current = compiler.apply_validation_operation(
				current,
				operation,
				prelu,
				[context.rows, output_width],
				context.config,
				context.domain,
			)?;
		}
		Ok(current)
	}
}

impl LayerTapeLifecycle for LayerValues {
	fn compile_layer_backward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockBackwardContext,
	) -> TrainingCompileResult<LayerBackward> {
		let mut gradient = context.gradient;
		let mut operation_gradients = vec![None; self.operations.len()];
		for operation_index in (0..self.operations.len()).rev() {
			let (input_gradient, parameter_gradient) = compiler.backward_operation(
				gradient,
				self.operations[operation_index],
				context.validity,
				context.valid_count,
				context.epsilon,
				context.tree_lanes,
			)?;
			gradient = input_gradient;
			operation_gradients[operation_index] = parameter_gradient;
		}
		let gradient = compiler.mask_f32_with_zero(gradient, context.validity, compiler.training_domain)?;
		let safe_input = compiler.mask_f32_with_zero(self.input, context.validity, compiler.training_domain)?;
		let weight_shape = compiler.tensor_ref(self.weight.value)?.shape.clone();
		let bias_shape = compiler.tensor_ref(self.bias.value)?.shape.clone();
		let raw_weight_gradient = compiler.tensor(DType::F32, weight_shape.clone())?;
		let bias_gradient = compiler.tensor(DType::F32, bias_shape)?;
		let prepared = [(
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(context.tree_lanes)),
		)]
		.into_iter()
		.collect::<PreparedParameters>();
		let input_gradient = match context.input_gradient {
			InputGradientRequirement::Required => {
				let input_width = compiler.tensor_ref(self.input)?.shape.extents()[1];
				let input_gradient =
					compiler.tensor(DType::F32, shape(&[context.partition_rows, input_width])?)?;
				compiler.materialize(
					"gpu_linear_backward_full_into",
					&[
						("output_gradient", gradient),
						("input", safe_input),
						("weight", self.forward_weight),
					],
					&[
						("input_gradient", input_gradient),
						("weight_gradient", raw_weight_gradient),
						("bias_gradient", bias_gradient),
					],
					"output_gradient",
					&prepared,
					compiler.training_domain,
				)?;
				Some(input_gradient)
			}
			InputGradientRequirement::Omit => {
				compiler.materialize(
					"gpu_linear_backward_weights_only_into",
					&[("output_gradient", gradient), ("input", safe_input)],
					&[
						("weight_gradient", raw_weight_gradient),
						("bias_gradient", bias_gradient),
					],
					"output_gradient",
					&prepared,
					compiler.training_domain,
				)?;
				None
			}
		};
		let weight_gradient = if let Some(mask) = self.routing_mask {
			compiler.elementwise_f32(
				weight_shape,
				vec![raw_weight_gradient, mask],
				masked_zero_f32_program()?,
				compiler.training_domain,
			)?
		} else {
			raw_weight_gradient
		};
		Ok(LayerBackward {
			gradient: GradientPair {
				weight: weight_gradient,
				bias: bias_gradient,
				prelu: operation_gradients.into_iter().flatten().collect(),
			},
			input_gradient,
		})
	}

	fn compile_layer_optimizer(
		&self,
		compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseLayerState> {
		let weight = updates.apply(compiler, ParameterRole::LayerWeight, self.weight)?;
		let bias = updates.apply(compiler, ParameterRole::LayerBias, self.bias)?;
		let prelu = compiler.update_operation_parameters(&self.operations, updates)?;
		Ok(DenseLayerState {
			weight,
			bias,
			prelu,
		})
	}
}

impl DeclaredBlock for DenseLayer {
	fn clone_box(&self) -> Box<dyn DeclaredBlock> {
		Box::new(self.clone())
	}

	fn output_width(&self) -> Option<NonZeroU64> {
		Some(self.width())
	}

	fn output_operations(&self) -> &[DenseOperation] {
		self.operations()
	}

	fn legacy_layer(&self) -> Option<&DenseLayer> {
		Some(self)
	}

	fn compile_forward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> {
		let (output, tape) = self.compile_layer_forward(
			compiler,
			LayerForwardContext {
				input: context.input,
				input_width: context.logical.width()?,
				partition_rows: context.partition_rows,
				validity: context.validity,
				valid_count: context.valid_count,
				routing: context.routing,
				config: context.config,
			},
		)?;
		Ok(BlockForward {
			output,
			logical: LogicalFeatureShape::from_width(self.width().get())?,
			routing: None,
			tape: Box::new(tape),
		})
	}
}

impl RealizedBlock for RealizedLayer {
	fn clone_box(&self) -> Box<dyn RealizedBlock> {
		Box::new(self.clone())
	}

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) {
		visit(self.state.weight);
		visit(self.state.bias);
		self.state.prelu.iter().copied().for_each(visit);
	}

	fn legacy_layer_state(&self) -> Option<&DenseLayerState> {
		Some(&self.state)
	}

	fn compile_validation(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> {
		Ok(BlockValidation {
			output: self.declaration.compile_layer_validation(
				compiler,
				&self.state,
				LayerValidationContext {
					input: context.input,
					rows: context.rows,
					routing: context.routing,
					config: context.config,
					domain: context.domain,
				},
			)?,
			logical: LogicalFeatureShape::from_width(self.declaration.width().get())?,
			routing: None,
		})
	}

	fn checkpoint(
		&self,
		training: &CompiledTraining,
		_index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_layer(training, self.declaration.clone(), self.state.clone())
			.map(crate::checkpoint::CheckpointBlock::Layer)
	}
}

impl CompiledBlock for LayerValues {
	fn backward(
		&self,
		compiler: &mut GraphCompiler,
		context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> {
		let result = self.compile_layer_backward(compiler, context)?;
		let mut parameters = Vec::new();
		push_layer_gradients(&mut parameters, &result.gradient);
		Ok(BlockBackward {
			input_gradient: result.input_gradient,
			parameters,
		})
	}

	fn optimize(
		&self,
		compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> {
		Ok(DenseBlockState::from_realized(Box::new(RealizedLayer {
			declaration: self.declaration.clone(),
			state: self.compile_layer_optimizer(compiler, updates)?,
		})))
	}
}

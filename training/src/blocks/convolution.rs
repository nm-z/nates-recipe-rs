use super::super::*; use crate::{ CheckpointConvolutionImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointConvolutionImage {
	fn output_width(&self) -> NonZeroU64 { self.geometry().output_width().expect("validated convolution width") }

	fn output_operations(&self) -> &[DenseOperation] { self.declaration().operations() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let geometry = self.geometry(); Ok(BlockInference {
			output: compiler.compile_convolution( context.block_index, self, context.input, [ context.rows, context.width,
					context.logical_length, context.logical_channels, ], context.normalization_epsilon, context.tree_lanes, )?,
			width: geometry .output_width()
				.expect("validated convolution output width is nonzero")
				.get(), logical_length: geometry.output_length().get(), logical_channels: geometry.filters().get(), }) } }

impl DeclaredBlock for DenseConvolution { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(self.clone()) }

	fn output_operations(&self) -> &[DenseOperation] { self.operations() }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let (logical, geometry) = context.logical.convolved(self)?;
		let output_width = geometry .output_width()
			.expect("validated convolution output width");
		let safe_input = compiler.mask_f32_with_zero(context.input, context.validity, compiler.training_domain)?;
		let fan_in = geometry.kernel().get() * geometry.input_channels().get(); let weight = compiler.initialize_weight(
			shape(&[ geometry.kernel().get(), geometry.input_channels().get(), geometry.filters().get(), ])?, fan_in,
			context.config.random_seed, )?; let bias = compiler.initialize_zero_parameter(shape(&[geometry.filters().get()])?)?;
		let (mut output, preparation, columns, output_group_indices) = compiler.compile_convolution_forward(
			[safe_input, weight.value, bias.value], context.partition_rows, geometry,
			ExternalInputRole::TrainingConvolutionWindowIndices { block: context.block_index, }, compiler.training_domain, )?;
		let mut operations = Vec::with_capacity(self.operations().len()); for operation in self.operations().iter().copied() {
			let (next, tape) = compiler.compile_training_operation( output, operation, output_width.get(),
				(context.partition_rows, context.validity, context.valid_count, context.config), )?; output = next;
			operations.push(tape); }
		Ok(BlockForward { output, logical, routing: None, tape: Box::new(ConvolutionValues { declaration: self.clone(),
				geometry, preparation, columns, output_group_indices, weight, bias, operations, }), }) } }

impl RealizedBlock for RealizedConvolution { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.weight);
		visit(self.state.bias); self.state.prelu.iter().copied().for_each(visit); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let output_width = self .state .geometry .output_width()
			.expect("validated convolution output width");
		let (mut output, ..) = compiler.compile_convolution_forward( [ context.input, self.state.weight.updated_parameter,
				self.state.bias.updated_parameter, ], context.rows, self.state.geometry,
			ExternalInputRole::ValidationConvolutionWindowIndices { block: context.block_index, }, context.domain, )?;
		let mut prelu = self.state.prelu.iter(); for operation in self.declaration.operations().iter().copied() {
			let parameter = validation_prelu_parameter(operation, &mut prelu); output = compiler.apply_validation_operation(
				output, operation, parameter, [context.rows, output_width.get()], context.config, context.domain, )?; }
		Ok(BlockValidation { output, logical: context.logical.convolved(&self.declaration)?.0, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_convolution(training, self.declaration.clone(), self.state.clone())
			.map(crate::checkpoint::CheckpointBlock::Convolution) } }

impl CompiledBlock for ConvolutionValues { fn backward( &self, compiler: &mut GraphCompiler,
		context: BlockBackwardContext, ) -> TrainingCompileResult<BlockBackward> { let mut gradient = context.gradient;
		let mut operation_gradients = vec![None; self.operations.len()];
		for operation_index in (0..self.operations.len()).rev() {
			let (input_gradient, parameter_gradient) = compiler.backward_operation( gradient, self.operations[operation_index],
				context.validity, context.valid_count, context.epsilon, context.tree_lanes, )?; gradient = input_gradient;
			operation_gradients[operation_index] = parameter_gradient; }
		let output_width = self .geometry .output_width()
			.expect("validated convolution output width");
		let input_width = self .geometry .input_width()
			.expect("validated convolution input width");
		compiler.require_tensor( gradient, DType::F32, &[context.partition_rows, output_width.get()],
			"convolution output gradient",
		)?; let gradient = compiler.mask_f32_with_zero(gradient, context.validity, compiler.training_domain)?;
		let grouped_gradient = compiler.gather( gradient, self.output_group_indices, shape(&self.preparation.output_shape())?,
			1, compiler.training_domain, )?; let weight_gradient = compiler.tensor( DType::F32, shape(&[
				self.geometry.kernel().get(), self.geometry.input_channels().get(), self.geometry.filters().get(), ])?, )?;
		compiler.emit( vec![self.columns, grouped_gradient], vec![weight_gradient], PrimitiveKind::Contraction(Contraction {
				batch_axes: Vec::new(), contract_axes: vec![(0, 0), (1, 1)], }), forbidden_aliases(2, 1), compiler.training_domain,
		)?; let bias_gradient = compiler.sum_tensor( grouped_gradient, shape(&[self.geometry.filters().get()])?, &[0, 1],
			context.tree_lanes, compiler.training_domain, )?; let input_gradient =
			match context.input_gradient { InputGradientRequirement::Required => {
					let (flat_gradient, _) = compiler.pack_matrix_to_flat( gradient, context.partition_rows, output_width.get(),
						compiler.training_domain, )?; let contribution_shape = shape(&self.preparation.input_gradient_indices_shape())?;
					let contribution_indices = compiler.external_i32_tensor(
						ExternalInputRole::TrainingConvolutionInputGradientIndices { block: context.block_index, },
						contribution_shape.clone(), self.preparation.input_gradient_indices(), )?;
					let contribution_validity = compiler.external_f32_tensor(
						ExternalInputRole::TrainingConvolutionInputGradientValidity { block: context.block_index, },
						contribution_shape.clone(), self.preparation.input_gradient_validity(), )?; let contributions = compiler.gather(
						flat_gradient, contribution_indices, contribution_shape.clone(), 0, compiler.training_domain, )?;
					let valid_contributions = compiler.elementwise_f32( contribution_shape, vec![contributions, contribution_validity],
						masked_zero_f32_program()?, compiler.training_domain, )?; let grouped_input_gradient = compiler.tensor(
						DType::F32, shape(&[ context.partition_rows, self.geometry.input_length().get(),
							self.geometry.input_channels().get(), ])?, )?; compiler.emit( vec![valid_contributions, self.weight.value],
						vec![grouped_input_gradient], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
							contract_axes: vec![(2, 0), (3, 2)], }), forbidden_aliases(2, 1), compiler.training_domain, )?;
					let (input_gradient, _) = compiler.unpack_pool_to_matrix( grouped_input_gradient, context.partition_rows,
						self.geometry.input_length().get(), self.geometry.input_channels().get(), input_width.get(),
						compiler.training_domain, )?; Some(compiler.mask_f32_with_zero( input_gradient, context.validity,
						compiler.training_domain, )?) }
				InputGradientRequirement::Omit => None, }; let gradient = GradientPair { weight: weight_gradient,
			bias: bias_gradient, prelu: operation_gradients.into_iter().flatten().collect(), }; let mut parameters = vec![
			gradient.weight, gradient.bias, ]; parameters.extend(gradient.prelu); Ok(BlockBackward { input_gradient, parameters,
		}) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { let weight = updates.apply(compiler, self.weight)?;
		let bias = updates.apply(compiler, self.bias)?;
		let prelu = compiler.update_operation_parameters(&self.operations, updates)?;
		Ok(DenseBlockState::from_realized(Box::new(RealizedConvolution { declaration: self.declaration.clone(),
			state: DenseConvolutionState { geometry: self.geometry, weight, bias, prelu, }, }))) } }

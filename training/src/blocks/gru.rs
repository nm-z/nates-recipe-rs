use super::super::*; use crate::{ CheckpointGruImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointGruImage { fn output_width(&self) -> NonZeroU64 { self.width() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.width().get(); Ok(BlockInference {
			output: compiler.compile_gru( context.block_index, self, context.input, context.rows, context.width, )?, width,
			logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseGru { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn output_width(&self) -> Option<NonZeroU64> { Some(self.width()) }

	fn leading_recurrent_name(&self) -> Option<&'static str> { Some("GRU") }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let input = context.input; let rows = context.partition_rows;
		let sequence_length = context.logical.length; let config = context.config;
		let result: TrainingCompileResult<(ValueId, GruValues)> = { compiler.require_tensor( input, DType::F32,
				&[rows, sequence_length.get()],
				"GRU input sequence",
			)?; checked_i32(
				checked_product(&[rows, sequence_length.get()], "GRU input element count")?,
				"GRU input element count",
			)?; let width = self.width(); let reset_input_weight =
				compiler.initialize_weight(shape(&[1, width.get()])?, 1, config.random_seed)?;
			let reset_recurrent_weight = compiler.initialize_weight( shape(&[width.get(), width.get()])?, width.get(),
				config.random_seed, )?; let reset_bias = compiler.initialize_zero_parameter(shape(&[width.get()])?)?;
			let update_input_weight = compiler.initialize_weight(shape(&[1, width.get()])?, 1, config.random_seed)?;
			let update_recurrent_weight = compiler.initialize_weight( shape(&[width.get(), width.get()])?, width.get(),
				config.random_seed, )?; let update_bias = compiler.initialize_zero_parameter(shape(&[width.get()])?)?;
			let candidate_input_weight = compiler.initialize_weight(shape(&[1, width.get()])?, 1, config.random_seed)?;
			let candidate_recurrent_weight = compiler.initialize_weight( shape(&[width.get(), width.get()])?, width.get(),
				config.random_seed, )?; let candidate_bias = compiler.initialize_zero_parameter(shape(&[width.get()])?)?;
			let forward_parameters = GruForwardParameters { reset: RecurrentGateParameters {
					input_weight: reset_input_weight.value, recurrent_weight: reset_recurrent_weight.value, bias: reset_bias.value, },
				update: RecurrentGateParameters { input_weight: update_input_weight.value,
					recurrent_weight: update_recurrent_weight.value, bias: update_bias.value, }, candidate: RecurrentGateParameters {
					input_weight: candidate_input_weight.value, recurrent_weight: candidate_recurrent_weight.value,
					bias: candidate_bias.value, }, }; let domain = compiler.training_domain; let (hidden, steps) = lower_gru_sequence(
				&mut TrainingForwardGraph { compiler, domain }, input, rows, sequence_length.get(), width.get(), forward_parameters,
			)?; Ok(( hidden, GruValues { declaration: *self, steps, reset_input_weight, reset_recurrent_weight, reset_bias,
					update_input_weight, update_recurrent_weight, update_bias, candidate_input_weight, candidate_recurrent_weight,
					candidate_bias, sequence_length, width, }, )) }; let (output, tape) = result?; Ok(BlockForward { output,
			logical: context.logical.recurrent(self.width())?, routing: None, tape: Box::new(tape), }) } }

impl RealizedBlock for RealizedGru { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.reset_input_weight);
		visit(self.state.reset_recurrent_weight); visit(self.state.reset_bias); visit(self.state.update_input_weight);
		visit(self.state.update_recurrent_weight); visit(self.state.update_bias); visit(self.state.candidate_input_weight);
		visit(self.state.candidate_recurrent_weight); visit(self.state.candidate_bias); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let input = context.input; let rows = context.rows;
		let state = &self.state; let domain = context.domain; let result: TrainingCompileResult<ValueId> = {
			compiler.require_tensor( input, DType::F32, &[rows, state.sequence_length.get()],
				"validation GRU input sequence",
			)?; let width = state.width.get(); let forward_parameters = GruForwardParameters { reset: RecurrentGateParameters {
					input_weight: state.reset_input_weight.updated_parameter,
					recurrent_weight: state.reset_recurrent_weight.updated_parameter, bias: state.reset_bias.updated_parameter, },
				update: RecurrentGateParameters { input_weight: state.update_input_weight.updated_parameter,
					recurrent_weight: state.update_recurrent_weight.updated_parameter, bias: state.update_bias.updated_parameter, },
				candidate: RecurrentGateParameters { input_weight: state.candidate_input_weight.updated_parameter,
					recurrent_weight: state.candidate_recurrent_weight.updated_parameter, bias: state.candidate_bias.updated_parameter,
				}, }; Ok(lower_gru_sequence( &mut TrainingForwardGraph { compiler, domain }, input, rows,
				state.sequence_length.get(), width, forward_parameters, )? .0) }; Ok(BlockValidation { output: result?,
			logical: context.logical.recurrent(self.declaration.width())?, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_gru(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Gru) } }

impl CompiledBlock for GruValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let gradient = context.gradient; let rows = context.partition_rows;
		let validity = context.validity; let tree_lanes = context.tree_lanes;
		let result: TrainingCompileResult<Vec<ParameterGradient>> = { compiler.require_tensor( gradient, DType::F32,
				&[rows, self.width.get()],
				"GRU output gradient",
			)?; let hidden_shape = shape(&[rows, self.width.get()])?; let mut hidden_gradient =
				compiler.mask_f32_with_zero(gradient, validity, compiler.training_domain)?;
			let mut reset_input_weight_gradient = None; let mut reset_recurrent_weight_gradient = None;
			let mut reset_bias_gradient = None; let mut update_input_weight_gradient = None;
			let mut update_recurrent_weight_gradient = None; let mut update_bias_gradient = None;
			let mut candidate_input_weight_gradient = None; let mut candidate_recurrent_weight_gradient = None;
			let mut candidate_bias_gradient = None;

			for step in self.steps.iter().rev() { let candidate_gradient = compiler.tensor(DType::F32, hidden_shape.clone())?;
				let update_gradient = compiler.tensor(DType::F32, hidden_shape.clone())?;
				let direct_previous_hidden_gradient = compiler.tensor(DType::F32, hidden_shape.clone())?; compiler.emit_elementwise(
					vec![ hidden_gradient, step.candidate, step.update, step.previous_hidden, ], vec![ candidate_gradient,
						update_gradient, direct_previous_hidden_gradient, ], gru_hidden_backward_program()?, compiler.training_domain, )?;

				let candidate_preactivation_gradient = compiler.backward_activation( candidate_gradient,
					step.candidate_preactivation, step.candidate, DenseActivation::Tanh, )?;
				let candidate_preactivation_gradient = compiler.mask_f32_with_zero( candidate_preactivation_gradient, validity,
					compiler.training_domain, )?; let step_candidate_input_weight_gradient = compiler.gru_matrix_gradient( step.input,
					candidate_preactivation_gradient, 1, self.width.get(), )?; compiler.accumulate_recurrent_gradient(
					&mut candidate_input_weight_gradient, step_candidate_input_weight_gradient, )?;
				let step_candidate_recurrent_weight_gradient = compiler.gru_matrix_gradient( step.reset_hidden,
					candidate_preactivation_gradient, self.width.get(), self.width.get(), )?; compiler.accumulate_recurrent_gradient(
					&mut candidate_recurrent_weight_gradient, step_candidate_recurrent_weight_gradient, )?;
				let step_candidate_bias_gradient = compiler.gru_bias_gradient( candidate_preactivation_gradient, self.width.get(),
					tree_lanes, )?; compiler.accumulate_recurrent_gradient( &mut candidate_bias_gradient, step_candidate_bias_gradient,
				)?;

				let reset_hidden_gradient = compiler.gru_projection_input_gradient( candidate_preactivation_gradient,
					self.candidate_recurrent_weight.value, rows, self.width.get(), )?;
				let reset_gradient = compiler.tensor(DType::F32, hidden_shape.clone())?;
				let candidate_previous_hidden_gradient = compiler.tensor(DType::F32, hidden_shape.clone())?;
				compiler.emit_elementwise( vec![reset_hidden_gradient, step.reset, step.previous_hidden],
					vec![reset_gradient, candidate_previous_hidden_gradient], gru_reset_product_backward_program()?,
					compiler.training_domain, )?;

				let reset_preactivation_gradient = compiler.backward_activation( reset_gradient, step.reset_preactivation,
					step.reset, DenseActivation::Sigmoid, )?; let reset_preactivation_gradient = compiler.mask_f32_with_zero(
					reset_preactivation_gradient, validity, compiler.training_domain, )?;
				let step_reset_input_weight_gradient = compiler.gru_matrix_gradient( step.input, reset_preactivation_gradient, 1,
					self.width.get(), )?; compiler.accumulate_recurrent_gradient( &mut reset_input_weight_gradient,
					step_reset_input_weight_gradient, )?; let step_reset_recurrent_weight_gradient = compiler.gru_matrix_gradient(
					step.previous_hidden, reset_preactivation_gradient, self.width.get(), self.width.get(), )?;
				compiler.accumulate_recurrent_gradient( &mut reset_recurrent_weight_gradient, step_reset_recurrent_weight_gradient,
				)?; let step_reset_bias_gradient =
					compiler.gru_bias_gradient(reset_preactivation_gradient, self.width.get(), tree_lanes)?;
				compiler.accumulate_recurrent_gradient(&mut reset_bias_gradient, step_reset_bias_gradient)?;

				let update_preactivation_gradient = compiler.backward_activation( update_gradient, step.update_preactivation,
					step.update, DenseActivation::Sigmoid, )?; let update_preactivation_gradient = compiler.mask_f32_with_zero(
					update_preactivation_gradient, validity, compiler.training_domain, )?;
				let step_update_input_weight_gradient = compiler.gru_matrix_gradient( step.input, update_preactivation_gradient, 1,
					self.width.get(), )?; compiler.accumulate_recurrent_gradient( &mut update_input_weight_gradient,
					step_update_input_weight_gradient, )?; let step_update_recurrent_weight_gradient = compiler.gru_matrix_gradient(
					step.previous_hidden, update_preactivation_gradient, self.width.get(), self.width.get(), )?;
				compiler.accumulate_recurrent_gradient( &mut update_recurrent_weight_gradient,
					step_update_recurrent_weight_gradient, )?; let step_update_bias_gradient = compiler.gru_bias_gradient(
					update_preactivation_gradient, self.width.get(), tree_lanes, )?;
				compiler.accumulate_recurrent_gradient(&mut update_bias_gradient, step_update_bias_gradient)?;

				let reset_previous_hidden_gradient = compiler.gru_projection_input_gradient( reset_preactivation_gradient,
					self.reset_recurrent_weight.value, rows, self.width.get(), )?;
				let update_previous_hidden_gradient = compiler.gru_projection_input_gradient( update_preactivation_gradient,
					self.update_recurrent_weight.value, rows, self.width.get(), )?;
				let previous_hidden_gradient = compiler.elementwise_f32( hidden_shape.clone(), vec![
						direct_previous_hidden_gradient, candidate_previous_hidden_gradient, reset_previous_hidden_gradient,
						update_previous_hidden_gradient, ], sum_program(4)?, compiler.training_domain, )?;
				hidden_gradient = previous_hidden_gradient; }

			Ok([ reset_input_weight_gradient, reset_recurrent_weight_gradient, reset_bias_gradient, update_input_weight_gradient,
				update_recurrent_weight_gradient, update_bias_gradient, candidate_input_weight_gradient,
				candidate_recurrent_weight_gradient, candidate_bias_gradient, ] .into_iter()
			.map(|value| value.expect("nonzero GRU sequence has every gradient"))
			.collect()) }; Ok(BlockBackward { input_gradient: None, parameters: result?, }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedGru {
			declaration: self.declaration, state: DenseGruState { sequence_length: self.sequence_length, width: self.width,
				reset_input_weight: updates.apply(compiler, self.reset_input_weight)?,
				reset_recurrent_weight: updates.apply(compiler, self.reset_recurrent_weight)?,
				reset_bias: updates.apply(compiler, self.reset_bias)?,
				update_input_weight: updates.apply(compiler, self.update_input_weight)?,
				update_recurrent_weight: updates.apply(compiler, self.update_recurrent_weight)?,
				update_bias: updates.apply(compiler, self.update_bias)?,
				candidate_input_weight: updates.apply(compiler, self.candidate_input_weight)?,
				candidate_recurrent_weight: updates.apply(compiler, self.candidate_recurrent_weight)?,
				candidate_bias: updates.apply(compiler, self.candidate_bias)?, }, }))) } }

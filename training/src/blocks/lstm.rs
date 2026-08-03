use super::super::*; use crate::{ CheckpointLstmImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointLstmImage { fn output_width(&self) -> NonZeroU64 { self.width() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.width().get(); Ok(BlockInference {
			output: compiler.compile_lstm( context.block_index, self, context.input, context.rows, context.width, )?, width,
			logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseLstm { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn output_width(&self) -> Option<NonZeroU64> { Some(self.width()) }

	fn leading_recurrent_name(&self) -> Option<&'static str> { Some("LSTM") }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let input = context.input; let rows = context.partition_rows;
		let sequence_length = context.logical.length; let config = context.config;
		let result: TrainingCompileResult<(ValueId, LstmValues)> = { compiler.require_tensor( input, DType::F32,
				&[rows, sequence_length.get()],
				"LSTM input sequence",
			)?; checked_i32(
				checked_product(&[rows, sequence_length.get()], "LSTM input element count")?,
				"LSTM input element count",
			)?; let width = self.width(); let initialize_gate = |compiler: &mut GraphCompiler| { Ok::<_, TrainingCompileError>((
					compiler.initialize_weight(shape(&[1, width.get()])?, 1, config.random_seed)?, compiler.initialize_weight(
						shape(&[width.get(), width.get()])?, width.get(), config.random_seed, )?,
					compiler.initialize_zero_parameter(shape(&[width.get()])?)?, )) };
			let (input_gate_input_weight, input_gate_recurrent_weight, input_gate_bias) = initialize_gate(compiler)?;
			let (forget_gate_input_weight, forget_gate_recurrent_weight, forget_gate_bias) = initialize_gate(compiler)?;
			let (output_gate_input_weight, output_gate_recurrent_weight, output_gate_bias) = initialize_gate(compiler)?;
			let (candidate_input_weight, candidate_recurrent_weight, candidate_bias) = initialize_gate(compiler)?;
			let forward_parameters = LstmForwardParameters { input: RecurrentGateParameters {
					input_weight: input_gate_input_weight.value, recurrent_weight: input_gate_recurrent_weight.value,
					bias: input_gate_bias.value, }, forget: RecurrentGateParameters { input_weight: forget_gate_input_weight.value,
					recurrent_weight: forget_gate_recurrent_weight.value, bias: forget_gate_bias.value, },
				output: RecurrentGateParameters { input_weight: output_gate_input_weight.value,
					recurrent_weight: output_gate_recurrent_weight.value, bias: output_gate_bias.value, },
				candidate: RecurrentGateParameters { input_weight: candidate_input_weight.value,
					recurrent_weight: candidate_recurrent_weight.value, bias: candidate_bias.value, }, };
			let domain = compiler.training_domain; let (hidden, steps) = lower_lstm_sequence(
				&mut TrainingForwardGraph { compiler, domain }, input, rows, sequence_length.get(), width.get(), forward_parameters,
			)?; Ok(( hidden, LstmValues { declaration: *self, steps, input_gate_input_weight, input_gate_recurrent_weight,
					input_gate_bias, forget_gate_input_weight, forget_gate_recurrent_weight, forget_gate_bias,
					output_gate_input_weight, output_gate_recurrent_weight, output_gate_bias, candidate_input_weight,
					candidate_recurrent_weight, candidate_bias, sequence_length, width, }, )) }; let (output, tape) = result?;
		Ok(BlockForward { output, logical: context.logical.recurrent(self.width())?, routing: None, tape: Box::new(tape), }) }
}

impl RealizedBlock for RealizedLstm { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.input_gate_input_weight);
		visit(self.state.input_gate_recurrent_weight); visit(self.state.input_gate_bias);
		visit(self.state.forget_gate_input_weight); visit(self.state.forget_gate_recurrent_weight);
		visit(self.state.forget_gate_bias); visit(self.state.output_gate_input_weight);
		visit(self.state.output_gate_recurrent_weight); visit(self.state.output_gate_bias);
		visit(self.state.candidate_input_weight); visit(self.state.candidate_recurrent_weight);
		visit(self.state.candidate_bias); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let input = context.input; let rows = context.rows;
		let state = &self.state; let domain = context.domain; let result: TrainingCompileResult<ValueId> = {
			compiler.require_tensor( input, DType::F32, &[rows, state.sequence_length.get()],
				"validation LSTM input sequence",
			)?; let width = state.width.get(); let forward_parameters = LstmForwardParameters { input: RecurrentGateParameters {
					input_weight: state.input_gate_input_weight.updated_parameter,
					recurrent_weight: state.input_gate_recurrent_weight.updated_parameter,
					bias: state.input_gate_bias.updated_parameter, }, forget: RecurrentGateParameters {
					input_weight: state.forget_gate_input_weight.updated_parameter,
					recurrent_weight: state.forget_gate_recurrent_weight.updated_parameter,
					bias: state.forget_gate_bias.updated_parameter, }, output: RecurrentGateParameters {
					input_weight: state.output_gate_input_weight.updated_parameter,
					recurrent_weight: state.output_gate_recurrent_weight.updated_parameter,
					bias: state.output_gate_bias.updated_parameter, }, candidate: RecurrentGateParameters {
					input_weight: state.candidate_input_weight.updated_parameter,
					recurrent_weight: state.candidate_recurrent_weight.updated_parameter, bias: state.candidate_bias.updated_parameter,
				}, }; Ok(lower_lstm_sequence( &mut TrainingForwardGraph { compiler, domain }, input, rows,
				state.sequence_length.get(), width, forward_parameters, )? .0) }; Ok(BlockValidation { output: result?,
			logical: context.logical.recurrent(self.declaration.width())?, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_lstm(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Lstm) } }

impl CompiledBlock for LstmValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let gradient = context.gradient; let rows = context.partition_rows;
		let validity = context.validity; let tree_lanes = context.tree_lanes;
		let result: TrainingCompileResult<Vec<ParameterGradient>> = { compiler.require_tensor( gradient, DType::F32,
				&[rows, self.width.get()],
				"LSTM output gradient",
			)?; let tensor_shape = shape(&[rows, self.width.get()])?; let mut hidden_gradient =
				compiler.mask_f32_with_zero(gradient, validity, compiler.training_domain)?;
			let mut cell_gradient = compiler.zero_f32_tensor(tensor_shape.clone())?;
			let mut input_gate_input_weight_gradient = None; let mut input_gate_recurrent_weight_gradient = None;
			let mut input_gate_bias_gradient = None; let mut forget_gate_input_weight_gradient = None;
			let mut forget_gate_recurrent_weight_gradient = None; let mut forget_gate_bias_gradient = None;
			let mut output_gate_input_weight_gradient = None; let mut output_gate_recurrent_weight_gradient = None;
			let mut output_gate_bias_gradient = None; let mut candidate_input_weight_gradient = None;
			let mut candidate_recurrent_weight_gradient = None; let mut candidate_bias_gradient = None;

			for step in self.steps.iter().rev() { let output_gate_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?;
				let cell_activation_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?; compiler.emit_elementwise(
					vec![hidden_gradient, step.output_gate, step.cell_activation],
					vec![output_gate_gradient, cell_activation_gradient], lstm_hidden_backward_program()?, compiler.training_domain,
				)?; let cell_from_hidden = compiler.backward_activation( cell_activation_gradient, step.cell, step.cell_activation,
					DenseActivation::Tanh, )?; let cell_from_hidden =
					compiler.mask_f32_with_zero(cell_from_hidden, validity, compiler.training_domain)?; let total_cell_gradient =
					compiler.exact_add(cell_gradient, cell_from_hidden, compiler.training_domain)?;

				let forget_gate_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?;
				let previous_cell_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?;
				let input_gate_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?;
				let candidate_gradient = compiler.tensor(DType::F32, tensor_shape.clone())?; compiler.emit_elementwise( vec![
						total_cell_gradient, step.forget_gate, step.previous_cell, step.input_gate, step.candidate, ], vec![
						forget_gate_gradient, previous_cell_gradient, input_gate_gradient, candidate_gradient, ],
					lstm_cell_backward_program()?, compiler.training_domain, )?;

				let input_gate_preactivation_gradient = compiler.backward_activation( input_gate_gradient,
					step.input_gate_preactivation, step.input_gate, DenseActivation::Sigmoid, )?;
				let input_gate_preactivation_gradient = compiler.mask_f32_with_zero( input_gate_preactivation_gradient, validity,
					compiler.training_domain, )?; let forget_gate_preactivation_gradient = compiler.backward_activation(
					forget_gate_gradient, step.forget_gate_preactivation, step.forget_gate, DenseActivation::Sigmoid, )?;
				let forget_gate_preactivation_gradient = compiler.mask_f32_with_zero( forget_gate_preactivation_gradient, validity,
					compiler.training_domain, )?; let output_gate_preactivation_gradient = compiler.backward_activation(
					output_gate_gradient, step.output_gate_preactivation, step.output_gate, DenseActivation::Sigmoid, )?;
				let output_gate_preactivation_gradient = compiler.mask_f32_with_zero( output_gate_preactivation_gradient, validity,
					compiler.training_domain, )?; let candidate_preactivation_gradient = compiler.backward_activation(
					candidate_gradient, step.candidate_preactivation, step.candidate, DenseActivation::Tanh, )?;
				let candidate_preactivation_gradient = compiler.mask_f32_with_zero( candidate_preactivation_gradient, validity,
					compiler.training_domain, )?;

				let input_gate = compiler.recurrent_gate_backward( [ step.input, step.previous_hidden,
						input_gate_preactivation_gradient, self.input_gate_recurrent_weight.value, ], [rows, self.width.get()],
					tree_lanes, )?; let forget_gate = compiler.recurrent_gate_backward( [ step.input, step.previous_hidden,
						forget_gate_preactivation_gradient, self.forget_gate_recurrent_weight.value, ], [rows, self.width.get()],
					tree_lanes, )?; let output_gate = compiler.recurrent_gate_backward( [ step.input, step.previous_hidden,
						output_gate_preactivation_gradient, self.output_gate_recurrent_weight.value, ], [rows, self.width.get()],
					tree_lanes, )?; let candidate = compiler.recurrent_gate_backward( [ step.input, step.previous_hidden,
						candidate_preactivation_gradient, self.candidate_recurrent_weight.value, ], [rows, self.width.get()], tree_lanes,
				)?;

				compiler.accumulate_recurrent_gradient( &mut input_gate_input_weight_gradient, input_gate.input_weight, )?;
				compiler.accumulate_recurrent_gradient( &mut input_gate_recurrent_weight_gradient, input_gate.recurrent_weight, )?;
				compiler.accumulate_recurrent_gradient(&mut input_gate_bias_gradient, input_gate.bias)?;
				compiler.accumulate_recurrent_gradient( &mut forget_gate_input_weight_gradient, forget_gate.input_weight, )?;
				compiler.accumulate_recurrent_gradient( &mut forget_gate_recurrent_weight_gradient, forget_gate.recurrent_weight,
				)?; compiler.accumulate_recurrent_gradient(&mut forget_gate_bias_gradient, forget_gate.bias)?;
				compiler.accumulate_recurrent_gradient( &mut output_gate_input_weight_gradient, output_gate.input_weight, )?;
				compiler.accumulate_recurrent_gradient( &mut output_gate_recurrent_weight_gradient, output_gate.recurrent_weight,
				)?; compiler.accumulate_recurrent_gradient(&mut output_gate_bias_gradient, output_gate.bias)?;
				compiler.accumulate_recurrent_gradient( &mut candidate_input_weight_gradient, candidate.input_weight, )?;
				compiler.accumulate_recurrent_gradient( &mut candidate_recurrent_weight_gradient, candidate.recurrent_weight, )?;
				compiler.accumulate_recurrent_gradient(&mut candidate_bias_gradient, candidate.bias)?;

				let previous_hidden_gradient = compiler.elementwise_f32( tensor_shape.clone(), vec![ input_gate.previous_hidden,
						forget_gate.previous_hidden, output_gate.previous_hidden, candidate.previous_hidden, ], sum_program(4)?,
					compiler.training_domain, )?; hidden_gradient = previous_hidden_gradient; cell_gradient = previous_cell_gradient; }

			Ok([ input_gate_input_weight_gradient, input_gate_recurrent_weight_gradient, input_gate_bias_gradient,
				forget_gate_input_weight_gradient, forget_gate_recurrent_weight_gradient, forget_gate_bias_gradient,
				output_gate_input_weight_gradient, output_gate_recurrent_weight_gradient, output_gate_bias_gradient,
				candidate_input_weight_gradient, candidate_recurrent_weight_gradient, candidate_bias_gradient, ] .into_iter()
			.map(|value| value.expect("nonzero LSTM sequence has every gradient"))
			.collect()) }; Ok(BlockBackward { input_gradient: None, parameters: result?, }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedLstm {
			declaration: self.declaration, state: DenseLstmState { sequence_length: self.sequence_length, width: self.width,
				input_gate_input_weight: updates.apply(compiler, self.input_gate_input_weight)?,
				input_gate_recurrent_weight: updates.apply(compiler, self.input_gate_recurrent_weight)?,
				input_gate_bias: updates.apply(compiler, self.input_gate_bias)?,
				forget_gate_input_weight: updates.apply(compiler, self.forget_gate_input_weight)?,
				forget_gate_recurrent_weight: updates.apply(compiler, self.forget_gate_recurrent_weight)?,
				forget_gate_bias: updates.apply(compiler, self.forget_gate_bias)?,
				output_gate_input_weight: updates.apply(compiler, self.output_gate_input_weight)?,
				output_gate_recurrent_weight: updates.apply(compiler, self.output_gate_recurrent_weight)?,
				output_gate_bias: updates.apply(compiler, self.output_gate_bias)?,
				candidate_input_weight: updates.apply(compiler, self.candidate_input_weight)?,
				candidate_recurrent_weight: updates.apply(compiler, self.candidate_recurrent_weight)?,
				candidate_bias: updates.apply(compiler, self.candidate_bias)?, }, }))) } }

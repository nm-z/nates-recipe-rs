use super::super::*; use crate::{ CheckpointRnnImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointRnnImage { fn output_width(&self) -> NonZeroU64 { self.width() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.width().get(); Ok(BlockInference {
			output: compiler.compile_rnn( context.block_index, self, context.input, context.rows, context.width, )?, width,
			logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseRnn { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn output_width(&self) -> Option<NonZeroU64> { Some(self.width()) }

	fn leading_recurrent_name(&self) -> Option<&'static str> { Some("vanilla RNN") }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let input = context.input; let rows = context.partition_rows;
		let sequence_length = context.logical.length; let config = context.config;
		let result: TrainingCompileResult<(ValueId, RnnValues)> = { compiler.require_tensor( input, DType::F32,
				&[rows, sequence_length.get()],
				"vanilla-RNN input sequence",
			)?; checked_i32( checked_product( &[rows, sequence_length.get()],
					"vanilla-RNN input element count",
				)?,
				"vanilla-RNN input element count",
			)?; let width = self.width();
			let input_weight = compiler.initialize_weight(shape(&[1, width.get()])?, 1, config.random_seed)?;
			let recurrent_weight = compiler.initialize_weight( shape(&[width.get(), width.get()])?, width.get(),
				config.random_seed, )?; let bias = compiler.initialize_zero_parameter(shape(&[width.get()])?)?;
			let forward_parameters = RecurrentGateParameters { input_weight: input_weight.value,
				recurrent_weight: recurrent_weight.value, bias: bias.value, }; let domain = compiler.training_domain;
			let (hidden, steps) = lower_rnn_sequence( &mut TrainingForwardGraph { compiler, domain }, input, rows,
				sequence_length.get(), width.get(), forward_parameters, )?; Ok(( hidden, RnnValues { declaration: *self, steps,
					input_weight, recurrent_weight, bias, sequence_length, width, }, )) }; let (output, tape) = result?;
		Ok(BlockForward { output, logical: context.logical.recurrent(self.width())?, routing: None, tape: Box::new(tape), }) }
}

impl RealizedBlock for RealizedRnn { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.input_weight);
		visit(self.state.recurrent_weight); visit(self.state.bias); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let input = context.input; let rows = context.rows;
		let state = &self.state; let domain = context.domain; let result: TrainingCompileResult<ValueId> = {
			compiler.require_tensor( input, DType::F32, &[rows, state.sequence_length.get()],
				"validation vanilla-RNN input sequence",
			)?; let width = state.width.get(); let forward_parameters = RecurrentGateParameters {
				input_weight: state.input_weight.updated_parameter, recurrent_weight: state.recurrent_weight.updated_parameter,
				bias: state.bias.updated_parameter, }; Ok(lower_rnn_sequence( &mut TrainingForwardGraph { compiler, domain }, input,
				rows, state.sequence_length.get(), width, forward_parameters, )? .0) }; Ok(BlockValidation { output: result?,
			logical: context.logical.recurrent(self.declaration.width())?, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_rnn(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Rnn) } }

impl CompiledBlock for RnnValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let gradient = context.gradient; let rows = context.partition_rows;
		let validity = context.validity; let tree_lanes = context.tree_lanes;
		let result: TrainingCompileResult<Vec<ParameterGradient>> = { compiler.require_tensor( gradient, DType::F32,
				&[rows, self.width.get()],
				"vanilla-RNN output gradient",
			)?; let mut hidden_gradient = compiler.mask_f32_with_zero(gradient, validity, compiler.training_domain)?;
			let mut input_weight_gradient = None; let mut recurrent_weight_gradient = None; let mut bias_gradient = None;
			for step in self.steps.iter().rev() { let preactivation_gradient = compiler.backward_activation( hidden_gradient,
					step.preactivation, step.hidden, DenseActivation::Tanh, )?;
				let preactivation_gradient = compiler.mask_f32_with_zero( preactivation_gradient, validity,
					compiler.training_domain, )?;

				let step_input_weight_gradient = compiler.tensor(DType::F32, shape(&[1, self.width.get()])?)?; compiler.emit(
					vec![step.input, preactivation_gradient], vec![step_input_weight_gradient],
					PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(), contract_axes: vec![(0, 0)], }),
					forbidden_aliases(2, 1), compiler.training_domain, )?; compiler
					.accumulate_recurrent_gradient(&mut input_weight_gradient, step_input_weight_gradient)?;

				let step_recurrent_weight_gradient = compiler.tensor(DType::F32, shape(&[self.width.get(), self.width.get()])?)?;
				compiler.emit( vec![step.previous_hidden, preactivation_gradient], vec![step_recurrent_weight_gradient],
					PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(), contract_axes: vec![(0, 0)], }),
					forbidden_aliases(2, 1), compiler.training_domain, )?; compiler.accumulate_recurrent_gradient(
					&mut recurrent_weight_gradient, step_recurrent_weight_gradient, )?;

				let step_bias_gradient = compiler.sum_tensor( preactivation_gradient, shape(&[self.width.get()])?, &[0], tree_lanes,
					compiler.training_domain, )?; compiler.accumulate_recurrent_gradient(&mut bias_gradient, step_bias_gradient)?;

				let previous_hidden_gradient = compiler.tensor(DType::F32, shape(&[rows, self.width.get()])?)?; compiler.emit(
					vec![preactivation_gradient, self.recurrent_weight.value], vec![previous_hidden_gradient],
					PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(), contract_axes: vec![(1, 1)], }),
					forbidden_aliases(2, 1), compiler.training_domain, )?; hidden_gradient = previous_hidden_gradient; }
			Ok(vec![
				input_weight_gradient.expect("nonzero RNN sequence has an input-weight gradient"),
				recurrent_weight_gradient.expect("nonzero RNN sequence has a recurrent-weight gradient"),
				bias_gradient.expect("nonzero RNN sequence has a bias gradient"),
			]) }; Ok(BlockBackward { input_gradient: None, parameters: result?, }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedRnn {
			declaration: self.declaration, state: DenseRnnState { sequence_length: self.sequence_length, width: self.width,
				input_weight: updates.apply(compiler, self.input_weight)?,
				recurrent_weight: updates.apply(compiler, self.recurrent_weight)?, bias: updates.apply(compiler, self.bias)?, },
		}))) } }

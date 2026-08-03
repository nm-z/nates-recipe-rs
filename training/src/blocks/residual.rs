use super::super::*;
use super::layer::{LayerDeclarationLifecycle as _, LayerForwardContext, LayerTapeLifecycle as _, LayerValidationContext};
use crate::{ CheckpointResidualImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointResidualImage {
	fn output_width(&self) -> NonZeroU64 { CheckpointResidualImage::output_width(self) }

	fn output_operations(&self) -> &[DenseOperation] { self.operations() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.output_width().get(); Ok(BlockInference {
			output: compiler.compile_residual(self, context)?, width, logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseResidual { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(self.clone()) }

	fn output_width(&self) -> Option<NonZeroU64> { DenseResidual::output_width(self) }

	fn output_operations(&self) -> &[DenseOperation] { self.operations() }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let residual_input = context.input;
		let residual_input_width = context.logical.width()?; let mut branch_current = residual_input;
		let mut branch_width = residual_input_width; let mut branch = Vec::with_capacity(self.branch().len());
		for operation in self.branch() { match operation { DenseResidualOperation::Layer(layer) => {
					let (output, tape) = layer.compile_layer_forward( compiler, LayerForwardContext { input: branch_current,
							input_width: branch_width, partition_rows: context.partition_rows, validity: context.validity,
							valid_count: context.valid_count, routing: None, config: context.config, }, )?; branch_current = output;
					branch_width = layer.width().get(); branch.push(ResidualBranchValues::Layer(tape)); }
				DenseResidualOperation::Operation(operation) => { let (output, tape) = compiler.compile_training_operation(
						branch_current, *operation, branch_width,
						(context.partition_rows, context.validity, context.valid_count, context.config), )?; branch_current = output;
					branch.push(ResidualBranchValues::Operation(tape)); } } }
		let output_width = self .output_width()
			.expect("validated residual output width");
		let (skip, projection) = match residual_input_width == output_width.get() { true => (residual_input, None), false => {
				let projection = compiler.initialize_weight( shape(&[residual_input_width, output_width.get()])?,
					residual_input_width, context.config.random_seed, )?; let skip = compiler.bias_free_linear( residual_input,
					projection.value, context.partition_rows, output_width.get(), compiler.training_domain, )?;
				(skip, Some(projection)) } }; let mut output = compiler.exact_add(branch_current, skip, compiler.training_domain)?;
		let mut operations = Vec::with_capacity(self.operations().len()); for operation in self.operations().iter().copied() {
			let (next, tape) = compiler.compile_training_operation( output, operation, output_width.get(),
				(context.partition_rows, context.validity, context.valid_count, context.config), )?; output = next;
			operations.push(tape); }
		Ok(BlockForward { output, logical: LogicalFeatureShape::from_width(output_width.get())?, routing: None,
			tape: Box::new(ResidualValues { declaration: self.clone(), input: residual_input, branch, projection, operations, }),
		}) } }

impl RealizedBlock for RealizedResidual { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { for layer in &self.state.branch {
			visit(layer.weight); visit(layer.bias); layer.prelu.iter().copied().for_each(&mut *visit); }
		self.state .branch_prelu .iter() .copied() .for_each(&mut *visit);
		self.state.projection.iter().copied().for_each(&mut *visit); self.state.prelu.iter().copied().for_each(visit); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let residual_input = context.input; let mut branch = residual_input;
		let mut layer_states = self.state.branch.iter(); let mut branch_prelu = self.state.branch_prelu.iter();
		for operation in self.declaration.branch() { match operation { DenseResidualOperation::Layer(layer) => {
					branch = layer.compile_layer_validation( compiler,
						layer_states.next().expect("realized residual layer state"),
						LayerValidationContext { input: branch, rows: context.rows, routing: None, config: context.config,
							domain: context.domain, }, )?; }
				DenseResidualOperation::Operation(operation) => {
					let prelu = validation_prelu_parameter(*operation, &mut branch_prelu);
					branch = compiler.apply_validation_operation( branch, *operation, prelu, [ context.rows,
							compiler.matrix_width(branch, "residual validation branch")?,
						], context.config, context.domain, )?; } } }
		let output_width = self .declaration .output_width()
			.expect("validated residual output width");
		let skip = match self.state.projection { None => residual_input, Some(projection) => compiler.bias_free_linear(
				residual_input, projection.updated_parameter, context.rows, output_width.get(), context.domain, )?, };
		let mut output = compiler.exact_add(branch, skip, context.domain)?; let mut prelu = self.state.prelu.iter();
		for operation in self.declaration.operations().iter().copied() {
			let parameter = validation_prelu_parameter(operation, &mut prelu); output = compiler.apply_validation_operation(
				output, operation, parameter, [context.rows, output_width.get()], context.config, context.domain, )?; }
		Ok(BlockValidation { output, logical: LogicalFeatureShape::from_width(output_width.get())?, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_residual(training, &self.declaration, &self.state)
			.map(crate::checkpoint::CheckpointBlock::Residual) } }

impl CompiledBlock for ResidualValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let mut gradient = context.gradient;
		let mut post_gradients = vec![None; self.operations.len()]; for operation_index in (0..self.operations.len()).rev() {
			let (input_gradient, parameter_gradient) = compiler.backward_operation( gradient, self.operations[operation_index],
				context.validity, context.valid_count, context.epsilon, context.tree_lanes, )?; gradient = input_gradient;
			post_gradients[operation_index] = parameter_gradient; }
		let merge_gradient = gradient; let mut branch_parameters = vec![None; self.branch.len()];
		let mut branch_gradient = merge_gradient; for branch_index in (0..self.branch.len()).rev() {
			match &self.branch[branch_index] { ResidualBranchValues::Operation(operation) => {
					let (input_gradient, parameter_gradient) = compiler.backward_operation( branch_gradient, *operation,
						context.validity, context.valid_count, context.epsilon, context.tree_lanes, )?; branch_gradient = input_gradient;
					branch_parameters[branch_index] = Some(parameter_gradient .into_iter() .collect()); }
				ResidualBranchValues::Layer(layer) => { let result = layer.compile_layer_backward( compiler, BlockBackwardContext {
							gradient: branch_gradient, input_gradient: InputGradientRequirement::Required, ..context }, )?;
					branch_gradient = result .input_gradient
						.expect("residual layer emits its input gradient");
					let mut parameters = Vec::new(); push_layer_gradients(&mut parameters, &result.gradient);
					branch_parameters[branch_index] = Some(parameters); } } }
		let (projection_gradient, skip_gradient) = match self.projection { Some(projection) => { let safe_input =
					compiler.mask_f32_with_zero(self.input, context.validity, compiler.training_domain)?;
				let safe_gradient = compiler.mask_f32_with_zero( merge_gradient, context.validity, compiler.training_domain, )?;
				let weight_shape = compiler.tensor_ref(projection.value)?.shape.clone();
				let weight_gradient = compiler.tensor(DType::F32, weight_shape)?; compiler.emit( vec![safe_input, safe_gradient],
					vec![weight_gradient], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
						contract_axes: vec![(0, 0)], }), forbidden_aliases(2, 1), compiler.training_domain, )?;
				let input_shape = compiler.tensor_ref(self.input)?.shape.clone();
				let input_gradient = compiler.tensor(DType::F32, input_shape)?; compiler.emit(
					vec![safe_gradient, projection.value], vec![input_gradient], PrimitiveKind::Contraction(Contraction {
						batch_axes: Vec::new(), contract_axes: vec![(1, 1)], }), forbidden_aliases(2, 1), compiler.training_domain, )?;
				(Some(weight_gradient), input_gradient) }
			None => (None, merge_gradient), };
		let input_gradient = compiler.exact_add(branch_gradient, skip_gradient, compiler.training_domain)?;
		let mut parameters = branch_parameters .into_iter()
			.flat_map(|parameters| parameters.expect("residual branch emits its parameter tape"))
			.collect::<Vec<_>>(); parameters.extend(projection_gradient);
		parameters.extend(post_gradients.into_iter().flatten()); Ok(BlockBackward { input_gradient: Some(input_gradient),
			parameters, }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { let mut branch = Vec::new(); let mut branch_prelu = Vec::new();
		for operation in &self.branch { match operation { ResidualBranchValues::Layer(layer) => {
					branch.push(layer.compile_layer_optimizer(compiler, updates)?); }
				ResidualBranchValues::Operation(operation) => { branch_prelu.extend(
						compiler.update_operation_parameters(core::slice::from_ref(operation), updates)?, ); } } }
		let projection = self .projection .map(|projection| updates.apply(compiler, projection)) .transpose()?;
		let prelu = compiler.update_operation_parameters(&self.operations, updates)?;
		Ok(DenseBlockState::from_realized(Box::new(RealizedResidual { declaration: self.declaration.clone(),
			state: DenseResidualState { branch, branch_prelu, projection, prelu, }, }))) } }

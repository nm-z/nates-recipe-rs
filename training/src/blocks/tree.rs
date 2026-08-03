use super::super::*; use crate::{ CheckpointTreeImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointTreeImage {
	fn output_width(&self) -> NonZeroU64 { CheckpointTreeImage::output_width(self) }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.output_width().get(); Ok(BlockInference {
			output: compiler.compile_tree( context.block_index, self, context.input, context.rows, context.width,
				context.tree_lanes, )?, width, logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseTree { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let input = context.input; let targets = context.targets;
		let supervision = context.validity; let rows = context.partition_rows; let input_width = context.logical.width()?;
		let output_width = context.target_width; let task = context.task; let block_index = context.block_index;
		let config = context.config; let result: TrainingCompileResult<(ValueId, TreeValues)> = {
			let input_width = NonZeroU64::new(input_width).expect("validated tree input width");
			let output_width = NonZeroU64::new(output_width).expect("validated tree output width");
			let requirements = tree_ensemble_inference_requirements( rows, input_width.get(), self.trees().get(),
				self.depth().get(), output_width.get(), )?;
			let internal_nodes_per_tree = NonZeroU64::new(requirements.internal_nodes_per_tree)
				.expect("tree requirements contain internal nodes");
			let leaves_per_tree =
				NonZeroU64::new(requirements.leaves_per_tree).expect("tree requirements contain leaves");
			let split_elements = checked_product( &[self.trees().get(), internal_nodes_per_tree.get()],
				"tree split element count",
			)?; let leaf_elements = checked_product( &[ self.trees().get(), leaves_per_tree.get(), output_width.get(), ],
				"tree leaf-value element count",
			)?; let split_shape = shape(&[split_elements])?;
			let target_matrix = compiler.tree_target_matrix(targets, supervision, rows, output_width, task)?;
			let (fresh_split_features, fresh_split_thresholds) = compiler.compile_supervised_tree_structure(
				[input, target_matrix, supervision], rows, [input_width, output_width, internal_nodes_per_tree], *self,
				config.reduction_tree_lanes, config.random_seed, )?; let resumed_split_features = compiler.external_i32_zeros(
				ExternalInputRole::ResumeTreeSplitFeatures { block: block_index }, split_shape.clone(), )?;
			let split_features = compiler.select_resume_i32_tensor( fresh_split_features, resumed_split_features,
				split_shape.clone(), )?; compiler.external_outputs.insert(split_features);
			let resumed_split_thresholds = compiler.external_f32_zeros(
				ExternalInputRole::ResumeTreeSplitThresholds { block: block_index }, split_shape.clone(), )?;
			let split_thresholds = compiler.select_resume_tensor( fresh_split_thresholds, resumed_split_thresholds, split_shape,
			)?; compiler.external_outputs.insert(split_thresholds);

			let leaf_values = compiler.initialize_zero_parameter(shape(&[leaf_elements])?)?; let (flat_input, _) =
				compiler.pack_matrix_to_flat(input, rows, input_width.get(), compiler.training_domain)?;
			let output = compiler.materialize_tree_predictions(
				[flat_input, split_features, split_thresholds, leaf_values.value], rows, [input_width, output_width], *self,
				config.reduction_tree_lanes, compiler.training_domain, )?; let leaf_indices = compiler.route_tree_leaf_indices(
				[flat_input, split_features, split_thresholds], rows, [ input_width, output_width, internal_nodes_per_tree,
					leaves_per_tree, ], *self, compiler.training_domain, )?; Ok(( output, TreeValues { declaration: *self, input_width,
					output_width, internal_nodes_per_tree, leaves_per_tree, split_features, split_thresholds, leaf_values,
					leaf_indices, }, )) }; let (output, tape) = result?; Ok(BlockForward { output,
			logical: LogicalFeatureShape::from_width(context.target_width)?, routing: None, tape: Box::new(tape), }) } }

impl RealizedBlock for RealizedTree { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(*self) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.leaf_values); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let (flat, _) = compiler.pack_matrix_to_flat( context.input,
			context.rows, context.logical.width()?, context.domain, )?; Ok(BlockValidation {
			output: compiler.materialize_tree_predictions( [ flat, self.state.split_features, self.state.split_thresholds,
					self.state.leaf_values.updated_parameter, ], context.rows, [self.state.input_width, self.state.output_width],
				self.declaration, context.config.reduction_tree_lanes, context.domain, )?,
			logical: LogicalFeatureShape::from_width(self.state.output_width.get())?, routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_tree(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Tree) } }

impl CompiledBlock for TreeValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let gradient = context.gradient; let rows = context.partition_rows;
		let tree_lanes = context.tree_lanes; let result: TrainingCompileResult<ValueId> = {
			let outputs = self.output_width.get(); let trees = self.declaration.trees().get();
			let leaf_elements = checked_product( &[trees, self.leaves_per_tree.get(), outputs],
				"self leaf-gradient element count",
			)?; let (flat_gradient, _) = compiler.pack_matrix_to_flat(gradient, rows, outputs, compiler.training_domain)?;
			let gradient_indices = compiler.identity_indices(shape(&[rows, 1, outputs])?)?; let row_gradients = compiler.gather(
				flat_gradient, gradient_indices, shape(&[rows, 1, outputs])?, 0, compiler.training_domain, )?;
			let contributions = compiler.elementwise_f32( shape(&[rows, trees, outputs])?,
				vec![row_gradients, self.leaf_indices], repeat_f32_with_i32_program()?, compiler.training_domain, )?;
			let leaf_gradient = compiler.deterministic_segment_sum( self.leaf_indices, contributions, leaf_elements, rows,
				tree_lanes, compiler.training_domain, )?; Ok(leaf_gradient) }; Ok(BlockBackward { input_gradient: None,
			parameters: vec![result?], }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedTree {
			declaration: self.declaration, state: DenseTreeState { declaration: self.declaration, input_width: self.input_width,
				output_width: self.output_width, internal_nodes_per_tree: self.internal_nodes_per_tree,
				leaves_per_tree: self.leaves_per_tree, split_features: self.split_features, split_thresholds: self.split_thresholds,
				leaf_values: updates.apply(compiler, self.leaf_values)?, }, }))) } }

use super::super::*; use crate::{ CheckpointKMeansImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointKMeansImage { fn output_width(&self) -> NonZeroU64 { self.clusters() }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self.clusters().get(); Ok(BlockInference {
			output: compiler.compile_kmeans( context.block_index, self, context.input, context.rows, context.width,
				context.tree_lanes, )?, width, logical_length: width, logical_channels: 1, }) } }

impl DeclaredBlock for DenseKMeans { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn output_width(&self) -> Option<NonZeroU64> { Some(self.clusters()) }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let input_width = context.logical.width()?; compiler.require_tensor(
			context.input, DType::F32, &[context.partition_rows, input_width],
			"K-means training input",
		)?; let clusters = self.clusters();
		let input_width = NonZeroU64::new(input_width).expect("validated K-means input width");
		let centroid_shape = shape(&[clusters.get(), input_width.get()])?;
		let fresh_centroids = compiler.tensor(DType::F32, centroid_shape.clone())?;
		let initialization_requirements = kmeans_initialization_requirements(clusters.get())?;
		let initialization_first_value = compiler.next_value; let initialization_first_kernel = compiler.next_kernel;
		compiler.next_value = compiler .next_value .checked_add(initialization_requirements.intermediate_values)
			.ok_or_else(identity_exhausted)?; compiler.next_kernel = compiler .next_kernel
			.checked_add(initialization_requirements.kernels) .ok_or_else(identity_exhausted)?;
		let initialization = materialize_kmeans_initialization(&KMeansInitializationRequest {
			points: compiler.tensor_ref(context.input)?.clone(), centroids: compiler.tensor_ref(fresh_centroids)?.clone(),
			identity_namespace: IdentityNamespace::new( ValueId::new(initialization_first_value),
				initialization_requirements.intermediate_values, KernelTemplateId::new(initialization_first_kernel),
				initialization_requirements.kernels, ), workspace_limit: initialization_requirements.workspace_bytes, })?;
		compiler.insert_materialized_graph(&initialization.graph, IterationDomain::first())?;
		let resumed_centroids = compiler.external_f32_zeros( ExternalInputRole::ResumeKMeansCentroids {
				block: context.block_index, }, centroid_shape.clone(), )?; let initial_centroids =
			compiler.select_resume_tensor(fresh_centroids, resumed_centroids, centroid_shape.clone())?;
		let updated_centroids = compiler.tensor(DType::F32, centroid_shape)?; let output = compiler.tensor( DType::F32,
			shape(&[context.partition_rows, clusters.get()])?, )?; let lloyd_requirements =
			kmeans_lloyd_requirements(context.partition_rows, input_width.get(), clusters.get())?;
		let lloyd_first_value = compiler.next_value; let lloyd_first_kernel = compiler.next_kernel;
		compiler.next_value = compiler .next_value .checked_add(lloyd_requirements.intermediate_values)
			.ok_or_else(identity_exhausted)?; compiler.next_kernel = compiler .next_kernel
			.checked_add(lloyd_requirements.kernels) .ok_or_else(identity_exhausted)?;
		let lloyd = materialize_kmeans_lloyd_step(&KMeansLloydRequest { points: compiler.tensor_ref(context.input)?.clone(),
			prior_centroids: compiler.tensor_ref(initial_centroids)?.clone(),
			updated_centroids: compiler.tensor_ref(updated_centroids)?.clone(), distances: compiler.tensor_ref(output)?.clone(),
			tree_lanes: context.config.reduction_tree_lanes, identity_namespace: IdentityNamespace::new(
				ValueId::new(lloyd_first_value), lloyd_requirements.intermediate_values, KernelTemplateId::new(lloyd_first_kernel),
				lloyd_requirements.kernels, ), workspace_limit: lloyd_requirements.workspace_bytes, })?;
		compiler.insert_materialized_graph(&lloyd.graph, compiler.training_domain)?;
		compiler.external_outputs.insert(updated_centroids); let state = DenseKMeansState { input_width, clusters,
			initial_centroids, updated_centroids, update_kernel: lloyd.centroid_update_kernel, }; Ok(BlockForward { output,
			logical: LogicalFeatureShape::from_width(self.clusters().get())?,
			routing: self.routing().map(|routing| (routing, NonZeroU64::MIN)), tape: Box::new(KMeansValues { declaration: *self,
				input: context.input, distances: output, state, }), }) } }

impl RealizedBlock for RealizedKMeans { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(*self) }

	fn visit_parameter_states(&self, _visit: &mut dyn FnMut(ParameterState)) {}

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let input_width = context.logical.width()?; compiler.require_tensor(
			context.input, DType::F32, &[context.rows, input_width],
			"validation K-means input",
		)?; compiler.require_tensor( self.state.updated_centroids, DType::F32, &[self.state.clusters.get(), input_width],
			"validation K-means centroids",
		)?; let output = compiler.tensor( DType::F32, shape(&[context.rows, self.state.clusters.get()])?, )?;
		let parameters = [
			("queries".to_owned(), PreparedParameter::U64(context.rows)),
			(
				"training_rows".to_owned(),
				PreparedParameter::U64(self.state.clusters.get()), ),
			("dimensions".to_owned(), PreparedParameter::U64(input_width)),
			(
				"tree_lanes".to_owned(),
				PreparedParameter::U64(u64::from(context.config.reduction_tree_lanes)), ), ] .into_iter()
		.collect::<PreparedParameters>(); compiler.materialize(
			"gpu_pairwise_l2",
			&[
				("query", context.input),
				("training", self.state.updated_centroids),
			],
			&[("distances", output)],
			"query",
			&parameters, context.domain, )?; Ok(BlockValidation { output,
			logical: LogicalFeatureShape::from_width(self.declaration.clusters().get())?, routing: self .declaration .routing()
				.map(|routing| (routing, NonZeroU64::MIN)), }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_kmeans(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::KMeans) } }

impl CompiledBlock for KMeansValues { fn backward( &self, compiler: &mut GraphCompiler, context: BlockBackwardContext,
	) -> TrainingCompileResult<BlockBackward> { let input_gradient = match context.input_gradient {
			InputGradientRequirement::Omit => None, InputGradientRequirement::Required => {
				let input_width = self.state.input_width.get(); let clusters = self.state.clusters.get(); compiler.require_tensor(
					context.gradient, DType::F32, &[context.partition_rows, clusters],
					"K-means output gradient",
				)?; let matrix_shape = shape(&[context.partition_rows, clusters])?; let scaled_gradient = compiler.elementwise_f32(
					matrix_shape, vec![context.gradient, self.distances], kmeans_distance_gradient_scale_program(context.epsilon)?,
					compiler.training_domain, )?; let row_scales = compiler.sum_tensor( scaled_gradient,
					shape(&[context.partition_rows, 1])?, &[1], context.tree_lanes, compiler.training_domain, )?;
				let input_term = compiler.elementwise_f32( shape(&[context.partition_rows, input_width])?,
					vec![self.input, row_scales], multiply_program()?, compiler.training_domain, )?; let centroid_term =
					compiler.tensor(DType::F32, shape(&[context.partition_rows, input_width])?)?; compiler.emit(
					vec![scaled_gradient, self.state.updated_centroids], vec![centroid_term], PrimitiveKind::Contraction(Contraction {
						batch_axes: Vec::new(), contract_axes: vec![(1, 0)], }), forbidden_aliases(2, 1), compiler.training_domain, )?;
				let gradient = compiler.elementwise_f32( shape(&[context.partition_rows, input_width])?,
					vec![input_term, centroid_term], subtract_program()?, compiler.training_domain, )?;
				Some(compiler.mask_f32_with_zero(gradient, context.validity, compiler.training_domain)?) } }; Ok(BlockBackward {
			input_gradient, parameters: Vec::new(), }) }

	fn optimize( &self, _compiler: &mut GraphCompiler,
		_updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedKMeans {
			declaration: self.declaration, state: self.state, }))) } }

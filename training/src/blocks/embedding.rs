use super::super::*; use crate::{ CheckpointEmbeddingImage, inference::{
		BlockInference, BlockInferenceContext, InferenceBlock, InferenceCompileError, InferenceCompileErrorKind,
		InferenceCompileResult, InferenceGraphCompiler, }, };

impl InferenceBlock for CheckpointEmbeddingImage {
	fn token_vocabulary(&self) -> Option<u64> { Some(self.vocabulary().get()) }

	fn output_width(&self) -> NonZeroU64 { NonZeroU64::new(self.sequence_length().get() * self.dimensions().get()).expect("validated embedding width") }

	fn compile_inference( &self, compiler: &mut InferenceGraphCompiler,
		context: BlockInferenceContext<'_>,
	) -> InferenceCompileResult<BlockInference> { let width = self .sequence_length() .get()
			.checked_mul(self.dimensions().get()) .ok_or_else(|| { InferenceCompileError::new(
					InferenceCompileErrorKind::ArithmeticOverflow,
					"embedding inference output width overflowed u64",
				) })?; Ok(BlockInference { output: compiler.compile_embedding( context.block_index, self, context.input,
				context.rows, context.width, )?, width, logical_length: self.sequence_length().get(),
			logical_channels: self.dimensions().get(), }) } }

impl DeclaredBlock for DenseEmbedding { fn clone_box(&self) -> Box<dyn DeclaredBlock> { Box::new(*self) }

	fn leading_embedding(&self) -> Option<DenseEmbedding> { Some(*self) }

	fn compile_forward( &self, compiler: &mut GraphCompiler,
		context: BlockForwardContext<'_>,
	) -> TrainingCompileResult<BlockForward> { let logical = context.logical.embedded(*self)?;
		let table = compiler.initialize_weight( shape(&[self.vocabulary().get(), self.dimensions().get()])?,
			self.dimensions().get(), context.config.random_seed, )?; let (output, indices) = compiler.compile_embedding_forward(
			context.input, context.partition_rows, context.logical.length.get(), self.dimensions().get(), table.value,
			compiler.training_domain, )?; Ok(BlockForward { output, logical, routing: None, tape: Box::new(EmbeddingValues {
				declaration: *self, indices, table, sequence_length: context.logical.length, dimensions: self.dimensions(),
				vocabulary: self.vocabulary(), }), }) } }

impl RealizedBlock for RealizedEmbedding { fn clone_box(&self) -> Box<dyn RealizedBlock> { Box::new(self.clone()) }

	fn visit_parameter_states(&self, visit: &mut dyn FnMut(ParameterState)) { visit(self.state.table); }

	fn compile_validation( &self, compiler: &mut GraphCompiler,
		context: BlockValidationContext<'_>,
	) -> TrainingCompileResult<BlockValidation> { let output = compiler .compile_embedding_forward( context.input,
				context.rows, self.state.sequence_length.get(), self.state.dimensions.get(), self.state.table.updated_parameter,
				context.domain, )? .0; Ok(BlockValidation { output, logical: context.logical.embedded(self.declaration)?,
			routing: None, }) }

	fn checkpoint( &self, training: &CompiledTraining, _index: usize,
	) -> crate::checkpoint::CheckpointResult<crate::checkpoint::CheckpointBlock> {
		crate::checkpoint::checkpoint_embedding(training, self.declaration, self.state)
			.map(crate::checkpoint::CheckpointBlock::Embedding) } }

impl CompiledBlock for EmbeddingValues { fn backward( &self, compiler: &mut GraphCompiler,
		context: BlockBackwardContext, ) -> TrainingCompileResult<BlockBackward> {
		let output_width = self.sequence_length.get() * self.dimensions.get(); compiler.require_tensor( context.gradient,
			DType::F32, &[context.partition_rows, output_width],
			"embedding output gradient",
		)?; let gradient = compiler.mask_f32_with_zero(context.gradient, context.validity, compiler.training_domain)?;
		let token_rows = context.partition_rows * self.sequence_length.get();
		let flat = compiler.pack_contiguous_tensor_to_flat(gradient, compiler.training_domain)?;
		let row_gradients = compiler.gather_flat_as( flat, shape(&[token_rows, self.dimensions.get()])?,
			compiler.training_domain, )?;
		let base = compiler.zero_f32_tensor(shape(&[self.vocabulary.get(), self.dimensions.get()])?)?;
		let table = compiler.tensor( DType::F32, shape(&[self.vocabulary.get(), self.dimensions.get()])?, )?;
		let parameters = [
			("rows".to_owned(), PreparedParameter::U64(token_rows)),
			(
				"columns".to_owned(),
				PreparedParameter::U64(self.dimensions.get()), ), (
				"vocabulary".to_owned(),
				PreparedParameter::U64(self.vocabulary.get()), ), ] .into_iter() .collect::<PreparedParameters>();
		compiler.materialize(
			"gpu_embedding_backward",
			&[
				("gradient", row_gradients),
				("indices", self.indices),
				("gradient_table_base", base),
			],
			&[("gradient_table", table)],
			"gradient",
			&parameters, compiler.training_domain, )?; Ok(BlockBackward { input_gradient: None, parameters: vec![table], }) }

	fn optimize( &self, compiler: &mut GraphCompiler,
		updates: &mut ParameterUpdates<'_>,
	) -> TrainingCompileResult<DenseBlockState> { Ok(DenseBlockState::from_realized(Box::new(RealizedEmbedding {
			declaration: self.declaration, state: DenseEmbeddingState { sequence_length: self.sequence_length,
				dimensions: self.dimensions, vocabulary: self.vocabulary, table: updates.apply(compiler, self.table)?, }, }))) } }

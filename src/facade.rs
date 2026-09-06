use std::cell::RefCell;

pub use api::*;

pub struct Recipe;

#[expect(
	non_upper_case_globals,
	reason = "preserves the public Recipe builder value"
)]
pub static recipe: Recipe = Recipe;

#[derive(Default)]
struct RecipeSequence { data: Option<Data>, model: Option<Model>, }

thread_local! { static RECIPE_SEQUENCE: RefCell<RecipeSequence> = RefCell::new(RecipeSequence::default()); }

fn begin_recipe_data(data: Data) { RECIPE_SEQUENCE.with(|sequence| { let mut sequence = sequence.borrow_mut();
		sequence.data = Some(data); sequence.model = None; }); }

pub(crate) fn remember_recipe_data(data: Data) { begin_recipe_data(data); }

fn begin_recipe_model(model: Model) { RECIPE_SEQUENCE.with(|sequence| sequence.borrow_mut().model = Some(model)); }

pub(crate) fn remember_recipe_model(model: Model) {
	RECIPE_SEQUENCE.with(|sequence| sequence.borrow_mut().model = Some(model)); }

pub(crate) fn take_recipe_inference_sequence() -> Result<(Data, Model), &'static str> {
	return take_recipe_sequence_with_diagnostics(
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration",
		"recipe.infer().evaluate() requires a preceding recipe.model() declaration",
	) }

pub(crate) fn take_recipe_training_sequence() -> Result<(Data, Model), &'static str> {
	return take_recipe_sequence_with_diagnostics(
		"recipe.train().run() requires a preceding recipe.data(...) declaration",
		"recipe.train().run() requires a preceding recipe.model() declaration",
	) }

fn take_recipe_sequence_with_diagnostics(
	missing_data: &'static str,
	missing_model: &'static str,
) -> Result<(Data, Model), &'static str> {
	return RECIPE_SEQUENCE.with(|sequence| { let mut sequence = sequence.borrow_mut(); let data = sequence.data.take();
		let model = sequence.model.take(); let data = data.ok_or(missing_data)?; let model = model.ok_or(missing_model)?;
		return Ok((data, model)) }) }

trait IntoDataSources { fn into_data_sources(self) -> Vec<String>; }

impl IntoDataSources for () { fn into_data_sources(self) -> Vec<String> { return Vec::new() } }

impl IntoDataSources for &str { fn into_data_sources(self) -> Vec<String> { return vec![self.to_owned()] } }

impl IntoDataSources for String { fn into_data_sources(self) -> Vec<String> { return vec![self] } }

impl IntoDataSources for &String { fn into_data_sources(self) -> Vec<String> { return vec![self.clone()] } }

impl<S, const N: usize> IntoDataSources for [S; N]
where S: AsRef<str>, { fn into_data_sources(self) -> Vec<String> { return self.into_iter()
			.map(|source| return source.as_ref().to_owned()) .collect() } }

impl<S> IntoDataSources for Vec<S>
where S: AsRef<str>, { fn into_data_sources(self) -> Vec<String> { return self.into_iter()
			.map(|source| return source.as_ref().to_owned()) .collect() } }

impl<S> IntoDataSources for &[S]
where S: AsRef<str>, { fn into_data_sources(self) -> Vec<String> { return self.iter() .map(|source| return source.as_ref().to_owned())
			.collect() } }

impl Recipe {
	#[expect(
		private_bounds,
		reason = "the public builder accepts only Recipe-supported data source forms"
	)]
	pub fn data(&self, sources: impl IntoDataSources) -> Data { let mut data = Data::empty();
		for source in sources.into_data_sources() { data = data.set(&source); }
		begin_recipe_data(data.clone()); return data }

	pub fn model(&self) -> Model { let model = Model::new(); begin_recipe_model(model.clone()); return model }

	#[must_use]
	pub const fn train(&self) -> Train { return Train::new() }

	#[must_use]
	pub const fn infer(&self) -> Infer { return Infer::new() } }

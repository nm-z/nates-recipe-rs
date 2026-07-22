pub mod data;
pub mod model;
pub mod train;
pub mod infer;

use crate::api::data::{Data, parked_data};
use crate::api::model::{Model, parked_model};
use pantry::encode::Dataset;
use recipe_infer::LayerSpec;

pub trait IntoLayer {
	fn into_layer(self) -> LayerSpec;
}

pub trait SavePath {
	fn or_default(self) -> String;
}
impl SavePath for () {
	fn or_default(self) -> String {
		"model.ogdl".to_string()
	}
}
impl SavePath for &str {
	fn or_default(self) -> String {
		self.to_string()
	}
}
impl SavePath for String {
	fn or_default(self) -> String {
		self
	}
}

pub enum Prepared<'a> {
	Owned(Dataset),
	Borrowed(&'a Dataset),
}

impl<'a> Prepared<'a> {
	pub fn get(&self) -> &Dataset {
		match self {
			Prepared::Owned(d) => d,
			Prepared::Borrowed(d) => d,
		}
	}
}

pub enum InferOnly {
	Fit,
	Forward,
}

pub trait RunData {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>>;
	fn target_names(&self) -> Vec<String>;
	fn raw_rows(&self) -> Option<Vec<Vec<String>>>;
	fn raw_headers(&self) -> Option<Vec<String>>;
	fn infer_only(&self) -> InferOnly;
}

impl RunData for Dataset {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>> {
		Ok(Prepared::Borrowed(self))
	}
	fn target_names(&self) -> Vec<String> {
		Vec::new()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		None
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		None
	}
	fn infer_only(&self) -> InferOnly {
		InferOnly::Fit
	}
}

impl RunData for Option<Dataset> {
	fn prepared<'a>(&'a self) -> anyhow::Result<Prepared<'a>> {
		let ds = self
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("no test dataset — use .test() or .split()"))?;
		Ok(Prepared::Borrowed(ds))
	}
	fn target_names(&self) -> Vec<String> {
		Vec::new()
	}
	fn raw_rows(&self) -> Option<Vec<Vec<String>>> {
		None
	}
	fn raw_headers(&self) -> Option<Vec<String>> {
		None
	}
	fn infer_only(&self) -> InferOnly {
		InferOnly::Forward
	}
}

pub enum DataHandle<'a> {
	Ref(&'a dyn RunData),
	Parked(Data),
}

impl DataHandle<'_> {
	pub fn get(&self) -> &dyn RunData {
		match self {
			DataHandle::Ref(d) => *d,
			DataHandle::Parked(d) => d,
		}
	}
}

pub trait RunArg {
	fn resolve(&self) -> DataHandle<'_>;
}

impl RunArg for &Data {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &Dataset {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &Option<Dataset> {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl RunArg for &dyn RunData {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Ref(*self)
	}
}

impl<F: for<'a> Fn(&'a str) -> Data> RunArg for F {
	fn resolve(&self) -> DataHandle<'_> {
		DataHandle::Parked(parked_data())
	}
}

pub enum ModelHandle<'a> {
	Ref(&'a Model),
	Parked(Model),
}

impl ModelHandle<'_> {
	pub fn get(&self) -> &Model {
		match self {
			ModelHandle::Ref(m) => m,
			ModelHandle::Parked(m) => m,
		}
	}
}

pub trait ModelArg {
	fn resolve(&self) -> ModelHandle<'_>;
}

impl ModelArg for &Model {
	fn resolve(&self) -> ModelHandle<'_> {
		ModelHandle::Ref(self)
	}
}

impl<F: Fn() -> Model> ModelArg for F {
	fn resolve(&self) -> ModelHandle<'_> {
		ModelHandle::Parked(parked_model())
	}
}

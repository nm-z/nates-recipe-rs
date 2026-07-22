pub mod compose;
pub mod graph;
pub mod intent;
pub mod metric;
pub mod model;
pub mod work;

pub use metric::{LogItem, Metric};
pub use model::{
	Activation, ConcatDims, LayerDims, LayerKind, LayerSpec, Loss, Param, concat_layer_dims,
	pinned_vocab,
};
pub use work::Work;

pub use intent::{ObjectId, ObjectRef, ObjectiveIntent, RecipeIntent, Slot};
pub use compose::{
	Architecture, Block, IntoLayerCandidate, IntoObjectiveCandidate, IntoTargetCandidate,
};
pub use graph::SemanticGraph;

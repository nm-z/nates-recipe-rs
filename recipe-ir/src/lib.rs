pub mod arch;
pub mod compose;
pub mod graph;
pub mod intent;
pub mod metric;
pub mod model;
pub mod work;

pub use arch::{
	COMPOSABLE, Comp, Ffn, Hy, HyMode, NormK, Recur, Spec, VERIFIED, arch_has_recurrence, comp_of, is_delta_arch,
	norm_is_layer, supported, supported_names, verified,
};

pub use metric::{LogItem, Metric};
pub use model::{
	Activation, ConcatDims, LayerDims, LayerKind, LayerSpec, Loss, Param, concat_layer_dims, pinned_vocab,
};
pub use work::Work;

pub use compose::{Architecture, Block, IntoLayerCandidate, IntoObjectiveCandidate, IntoTargetCandidate};
pub use graph::SemanticGraph;
pub use intent::{ObjectId, ObjectRef, ObjectiveIntent, RecipeIntent, Slot};

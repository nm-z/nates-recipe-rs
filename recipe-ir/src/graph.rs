use crate::model::{Activation, Loss, Param};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OpId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ParamId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dim(pub usize);

#[derive(Clone, PartialEq, Debug)]
pub struct Shape {
	pub dims: Vec<Dim>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum OpKind {
	Dense(Activation),
	Embed { dim: usize, vocab: usize },
	Attn { dim: usize, heads: usize },
	Conv { cin: usize, k: usize, stride: usize, act: Activation },
	Activation(Activation),
	LossReduce(Loss),
}

#[derive(Clone, PartialEq, Debug)]
pub struct ValueNode {
	pub id: ValueId,
	pub shape: Shape,
	pub produced_by: Option<OpId>,
}

#[derive(Clone, PartialEq)]
pub struct OpNode {
	pub id: OpId,
	pub kind: OpKind,
	pub inputs: Vec<ValueId>,
	pub outputs: Vec<ValueId>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ObjectApplication {
	pub object: super::intent::ObjectId,
	pub op: OpId,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dependency {
	pub from: ValueId,
	pub to: OpId,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TargetProducer {
	pub target: super::intent::ObjectId,
	pub value: ValueId,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ObjectiveNode {
	pub scalar: ValueId,
	pub wrt: Vec<ParamId>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ParamNode {
	pub id: ParamId,
	pub owner: OpId,
	pub role: Param,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DiffEdge {
	pub from: ValueId,
	pub to: ValueId,
}

#[derive(Clone, PartialEq, Default)]
pub struct SemanticGraph {
	pub values: Vec<ValueNode>,
	pub ops: Vec<OpNode>,
	pub applications: Vec<ObjectApplication>,
	pub deps: Vec<Dependency>,
	pub target_producers: Vec<TargetProducer>,
	pub objectives: Vec<ObjectiveNode>,
	pub params: Vec<ParamNode>,
	pub fwd_edges: Vec<DiffEdge>,
	pub rev_edges: Vec<DiffEdge>,
}

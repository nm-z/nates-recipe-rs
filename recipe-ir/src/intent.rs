use crate::model::{LayerSpec, Loss};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ObjectId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ValueId(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ObjectKind {
	Model,
	Data,
	Target,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ObjectDecl {
	pub id: ObjectId,
	pub kind: ObjectKind,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ObjectRef {
	Object(ObjectId),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ValueRef {
	Value(ValueId),
	Produced(ObjectId),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Slot<T> {
	Given(T),
	Unspecified,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ObjectiveIntent {
	Builtin(Loss),
	Reference(ObjectRef),
	Expression(ValueRef),
	Unspecified,
}

#[derive(Clone)]
pub struct ModelDecl {
	pub id: ObjectId,
	pub layers: Vec<LayerSpec>,
	pub objective: ObjectiveIntent,
	pub lr: Slot<f64>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DataDecl {
	pub id: ObjectId,
	pub source: Slot<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TargetDecl {
	pub id: ObjectId,
	pub owner: ObjectId,
	pub columns: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ObjectivePosition {
	pub model: ObjectId,
	pub intent: ObjectiveIntent,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TrainRequest {
	pub model: ObjectId,
	pub data: ObjectId,
	pub epochs: Slot<usize>,
	pub log_every: Slot<usize>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct InferRequest {
	pub model: ObjectId,
	pub data: Option<ObjectId>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum UserConstraint {
	OutputUnits(ObjectId, usize),
	ExplicitLoss(ObjectId, Loss),
	ExplicitTarget(ObjectId),
}

#[derive(Clone, Default)]
pub struct RecipeIntent {
	pub objects: Vec<ObjectDecl>,
	pub models: Vec<ModelDecl>,
	pub datasets: Vec<DataDecl>,
	pub targets: Vec<TargetDecl>,
	pub objectives: Vec<ObjectivePosition>,
	pub trainings: Vec<TrainRequest>,
	pub inferences: Vec<InferRequest>,
	pub constraints: Vec<UserConstraint>,
	pub next_id: u32,
}

impl RecipeIntent {
	pub fn fresh_id(&mut self) -> ObjectId {
		let id = ObjectId(self.next_id);
		self.next_id += 1;
		return id;
	}
}

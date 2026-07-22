use crate::intent::{ObjectRef, ValueRef};
use crate::model::{LayerSpec, Loss};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Position {
	Layer,
	Objective,
	Target,
}

#[derive(Clone, Copy)]
pub enum LayerCandidate {
	Spec(LayerSpec),
}

#[derive(Clone, Copy, PartialEq)]
pub enum ObjectiveCandidate {
	Loss(Loss),
	Surrogate(ObjectRef),
	Expr(ValueRef),
}

#[derive(Clone, PartialEq, Debug)]
pub enum TargetCandidate {
	Columns(Vec<String>),
}

pub trait IntoLayerCandidate {
	fn layer_candidate(self) -> LayerCandidate;
}

pub trait IntoObjectiveCandidate {
	fn objective_candidate(self) -> ObjectiveCandidate;
}

pub trait IntoTargetCandidate {
	fn target_candidate(self) -> TargetCandidate;
}

pub trait Block {
	fn emit(&self, out: &mut Vec<LayerSpec>);
}

pub trait Architecture {
	fn blocks(&self) -> &[LayerSpec];
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ComposeError {
	WrongPosition { pos: Position, got: &'static str },
}

impl IntoObjectiveCandidate for Loss {
	fn objective_candidate(self) -> ObjectiveCandidate {
		return ObjectiveCandidate::Loss(self);
	}
}

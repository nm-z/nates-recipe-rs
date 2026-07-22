use recipe::{Model, mse};
use recipe_runtime::graph::rev_reached_ops;
use recipe_runtime::resolve::{DataFacts, TargetKind, resolve_model};
use std::collections::HashSet;

fn facts() -> DataFacts {
	return DataFacts {
		n: 100,
		d: 8,
		k: 3,
		target_kind: TargetKind::None,
	};
}

#[test]
fn surrogate_reference_resolves_through_referenced_loss() {
	let bench = Model::new().layer(24).leak().layer(1).loss(mse).lr(0.001);
	let tile = Model::new().layer(32).leak().layer(3).loss(&bench).lr(0.001);

	let Ok(res) = resolve_model(&tile.objective(), tile.lr_intent(), tile.specs(), &facts())
	else {
		panic!("surrogate reference must resolve");
	};

	assert!(res.loss == recipe_ir::Loss::Mse, "resolved loss is bench's mse");
	assert!(!res.notes.is_empty(), "resolution records at least one note");

	let g = res.graph.expect("surrogate resolution carries a composed graph");
	assert_eq!(g.objectives.len(), 1, "exactly one merged ObjectiveNode");

	let referrer_params: u32 = 4;
	let total_params: usize = 8;
	let obj = &g.objectives[0];
	assert_eq!(
		obj.wrt.len(),
		total_params,
		"wrt covers every param of both models"
	);
	assert!(
		obj.wrt.iter().any(|p| p.0 < referrer_params),
		"wrt includes referrer params"
	);
	assert!(
		obj.wrt.iter().any(|p| p.0 >= referrer_params),
		"wrt includes referenced params"
	);

	let reached = rev_reached_ops(&g);
	let param_owner_ops: HashSet<u32> = g.params.iter().map(|p| p.owner.0).collect();
	let reached_owners = param_owner_ops
		.iter()
		.filter(|owner| reached.contains(owner))
		.count();
	assert_eq!(
		reached_owners, 4,
		"reverse path reaches both chains' param-owner ops (tile 2 + bench 2)"
	);
}

#[test]
fn surrogate_reference_to_unregistered_model_errors() {
	let objective = recipe_ir::ObjectiveIntent::Reference(recipe_ir::ObjectRef::Object(
		recipe_ir::ObjectId(4_000_000),
	));
	let specs = [recipe_ir::LayerSpec::Dense(3, recipe_ir::Activation::Linear)];

	let err = resolve_model(&objective, Some(0.001), &specs, &facts())
		.err()
		.expect("reference to an unregistered id must error");
	let msg = err.message();
	assert!(
		msg.contains("never registered"),
		"message names the missing registration: {msg}"
	);
}

#[test]
fn surrogate_reference_chain_errors_as_cycle() {
	let base = Model::new().layer(24).leak().layer(1).loss(mse).lr(0.001);
	let mid = Model::new().layer(16).leak().layer(5).loss(&base).lr(0.001);
	let top = Model::new().layer(32).leak().layer(5).loss(&mid).lr(0.001);

	let err = resolve_model(&top.objective(), top.lr_intent(), top.specs(), &facts())
		.err()
		.expect("a surrogate whose target is itself a reference must error");
	let msg = err.message();
	assert!(
		msg.contains("chain of surrogates"),
		"message flags the surrogate-of-surrogate cycle: {msg}"
	);
}

#[test]
fn surrogate_reference_to_layerless_model_errors_on_dims() {
	let empty = Model::new().loss(mse);
	let tile = Model::new().layer(32).leak().layer(3).loss(&empty).lr(0.001);

	let err = resolve_model(&tile.objective(), tile.lr_intent(), tile.specs(), &facts())
		.err()
		.expect("a layerless referenced model must error");
	let msg = err.message();
	assert!(
		msg.contains("has no layers"),
		"dimensional guard flags the empty referenced model: {msg}"
	);
}

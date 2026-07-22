use recipe::{Model, bce, mse};
use recipe_runtime::graph::rev_reached_ops;
use recipe_runtime::resolve::{DataFacts, TargetKind, resolve_model, upsert_model};
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
fn surrogate_reference_chain_to_ground_resolves() {
	let base = Model::new().layer(24).leak().layer(1).loss(mse).lr(0.001);
	let mid = Model::new().layer(16).leak().layer(5).loss(&base).lr(0.001);
	let top = Model::new().layer(32).leak().layer(5).loss(&mid).lr(0.001);

	let Ok(res) = resolve_model(&top.objective(), top.lr_intent(), top.specs(), &facts())
	else {
		panic!("a reference chain that grounds in a concrete loss must resolve");
	};
	assert!(
		res.loss == recipe_ir::Loss::Mse,
		"the chain grounds in base's concrete mse"
	);
	assert!(
		res.graph.is_some(),
		"a resolved chain carries a composed graph for the outer hop"
	);
}

#[test]
fn surrogate_reference_genuine_cycle_errors() {
	let a = recipe_ir::ObjectId(9_100_001);
	let b = recipe_ir::ObjectId(9_100_002);
	let specs = vec![
		recipe_ir::LayerSpec::Dense(4, recipe_ir::Activation::LeakyRelu),
		recipe_ir::LayerSpec::Dense(4, recipe_ir::Activation::Linear),
	];
	upsert_model(
		a,
		specs.clone(),
		recipe_ir::ObjectiveIntent::Reference(recipe_ir::ObjectRef::Object(b)),
		Some(0.001),
	);
	upsert_model(
		b,
		specs.clone(),
		recipe_ir::ObjectiveIntent::Reference(recipe_ir::ObjectRef::Object(a)),
		Some(0.001),
	);

	let objective = recipe_ir::ObjectiveIntent::Reference(recipe_ir::ObjectRef::Object(a));
	let referrer_specs = [recipe_ir::LayerSpec::Dense(4, recipe_ir::Activation::Linear)];
	let err = resolve_model(&objective, Some(0.001), &referrer_specs, &facts())
		.err()
		.expect("a reference that loops back on itself must error");
	let msg = err.message();
	assert!(
		msg.contains("cycle"),
		"message flags the genuine reference cycle: {msg}"
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

#[test]
fn mutation_after_reference_is_visible() {
	let bench = Model::new().layer(24).leak().layer(1).loss(mse).lr(0.001);
	let tile = Model::new().layer(32).leak().layer(1).loss(&bench).lr(0.001);

	let bench = bench.loss(bce);
	let _held = &bench;

	let Ok(res) = resolve_model(&tile.objective(), tile.lr_intent(), tile.specs(), &facts())
	else {
		panic!("a reference to a mutated model must still resolve");
	};
	assert!(
		res.loss == recipe_ir::Loss::Bce,
		"resolution sees bench's live mutated loss (bce), not a referenced-time snapshot (mse)"
	);
}

#[test]
fn ambiguity_when_real_target_and_reference_coexist() {
	let bench = Model::new().layer(24).leak().layer(1).loss(mse).lr(0.001);
	let tile = Model::new().layer(32).leak().layer(1).loss(&bench).lr(0.001);

	let with_target = DataFacts {
		n: 100,
		d: 8,
		k: 1,
		target_kind: TargetKind::Continuous,
	};
	let err = resolve_model(&tile.objective(), tile.lr_intent(), tile.specs(), &with_target)
		.err()
		.expect("a real dataset target plus a reference objective is ambiguous");
	let msg = err.message();
	assert!(msg.contains("ambiguous"), "names the ambiguity: {msg}");
	assert!(
		msg.contains("dataset target"),
		"names the train-on-target interpretation: {msg}"
	);
	assert!(
		msg.contains("referenced model"),
		"names the train-through-surrogate interpretation: {msg}"
	);
}

#[test]
fn referenced_omitted_loss_never_defaults_to_mse() {
	let critic = Model::new().layer(24).leak().layer(1);
	let student = Model::new().layer(32).leak().layer(1).loss(&critic).lr(0.001);

	let err = resolve_model(&student.objective(), student.lr_intent(), student.specs(), &facts())
		.err()
		.expect("a referenced model with an omitted loss and no context cannot silently become mse");
	let msg = err.message();
	assert!(
		msg.contains("omitted") && msg.contains("no context loss"),
		"the error explains the missing ground, never a silent mse default: {msg}"
	);
}

#[test]
fn eval_omitted_loss_resolves_through_context() {
	let m = Model::new().layer(16).leak().layer(1);

	let binary = DataFacts {
		n: 50,
		d: 4,
		k: 1,
		target_kind: TargetKind::Binary,
	};
	let Ok(res) = resolve_model(&m.objective(), m.lr_intent(), m.specs(), &binary) else {
		panic!("eval of an omitted-loss model resolves its loss from data context");
	};
	assert!(
		res.loss == recipe_ir::Loss::Bce,
		"eval's resolve entry picks bce from the binary target context, not a mse default"
	);
}

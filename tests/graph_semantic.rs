use recipe_ir::graph::{DiffEdge, OpKind, ParamId, ValueId};
use recipe_ir::{Activation, LayerDims, LayerKind, Loss, ObjectId};
use recipe_runtime::graph::build_forward_graph;

fn mk(kind: LayerKind, in_dim: usize, out_dim: usize, act: Activation) -> LayerDims {
	return LayerDims {
		kind,
		in_dim,
		out_dim,
		act,
		dim: 0,
		vocab: 0,
		heads: 0,
		conv_cin: 0,
		conv_k: 0,
		conv_stride: 0,
	};
}

fn scalar_is_rank0(g: &recipe_ir::SemanticGraph) {
	let scalar = g.objectives[0].scalar;
	let sv = g
		.values
		.iter()
		.find(|v| v.id == scalar)
		.expect("scalar value node present");
	assert!(sv.shape.dims.is_empty(), "loss scalar must be rank 0");
}

fn wrt_covers_all_params(g: &recipe_ir::SemanticGraph) {
	let params: Vec<ParamId> = g.params.iter().map(|p| p.id).collect();
	assert_eq!(g.objectives.len(), 1, "exactly one objective");
	assert_eq!(g.objectives[0].wrt, params, "wrt must cover every param");
}

fn activation_op_count(g: &recipe_ir::SemanticGraph) -> usize {
	return g
		.ops
		.iter()
		.filter(|o| matches!(o.kind, OpKind::Activation(_)))
		.count();
}

#[test]
fn dense_leak_dense_chain() {
	let dims = [
		mk(LayerKind::Dense, 4, 8, Activation::LeakyRelu),
		mk(LayerKind::Dense, 8, 1, Activation::Linear),
	];
	let g = build_forward_graph(ObjectId(7), &dims, Loss::Mse);

	let kinds: Vec<OpKind> = g.ops.iter().map(|o| o.kind).collect();
	assert_eq!(kinds.len(), 3, "two structural ops + one loss op");
	assert!(kinds[0] == OpKind::Dense(Activation::LeakyRelu));
	assert!(kinds[1] == OpKind::Dense(Activation::Linear));
	assert!(kinds[2] == OpKind::LossReduce(Loss::Mse));

	assert_eq!(
		activation_op_count(&g),
		0,
		"LeakyRelu is not in saves_preact — no preact Activation op"
	);

	assert_eq!(
		g.rev_edges,
		vec![
			DiffEdge {
				from: ValueId(3),
				to: ValueId(2),
			},
			DiffEdge {
				from: ValueId(2),
				to: ValueId(1),
			},
		],
		"rev_edges: seed scalar->v_last then v1->v0, layer 0 emits none"
	);
	assert_eq!(g.rev_edges.len(), dims.len(), "rev_edges count == layers");

	scalar_is_rank0(&g);
	wrt_covers_all_params(&g);
}

#[test]
fn embed_attn_dense_chain() {
	let dims = [
		mk(LayerKind::Embed, 3, 48, Activation::Linear),
		mk(LayerKind::Attn, 48, 48, Activation::Linear),
		mk(LayerKind::Dense, 48, 2, Activation::Linear),
	];
	let g = build_forward_graph(ObjectId(3), &dims, Loss::Ce);

	let kinds: Vec<OpKind> = g.ops.iter().map(|o| o.kind).collect();
	assert_eq!(kinds.len(), 4, "three structural ops + one loss op");
	assert!(kinds[0] == OpKind::Embed { dim: 0, vocab: 0 });
	assert!(kinds[1] == OpKind::Attn { dim: 0, heads: 0 });
	assert!(kinds[2] == OpKind::Dense(Activation::Linear));
	assert!(kinds[3] == OpKind::LossReduce(Loss::Ce));

	assert_eq!(
		activation_op_count(&g),
		0,
		"no saves_preact activations in this chain"
	);

	assert_eq!(
		g.rev_edges,
		vec![
			DiffEdge {
				from: ValueId(4),
				to: ValueId(3),
			},
			DiffEdge {
				from: ValueId(3),
				to: ValueId(2),
			},
			DiffEdge {
				from: ValueId(2),
				to: ValueId(1),
			},
		],
		"rev_edges: seed then v2->v1, v1->v0; embed at layer 0 emits none"
	);
	assert_eq!(g.rev_edges.len(), dims.len(), "rev_edges count == layers");

	scalar_is_rank0(&g);
	wrt_covers_all_params(&g);
}

#[test]
fn gelu_dense_preact_node() {
	let dims = [
		mk(LayerKind::Dense, 4, 8, Activation::Gelu),
		mk(LayerKind::Dense, 8, 1, Activation::Linear),
	];
	let g = build_forward_graph(ObjectId(1), &dims, Loss::Mse);

	let kinds: Vec<OpKind> = g.ops.iter().map(|o| o.kind).collect();
	assert_eq!(kinds.len(), 4, "Dense + Activation(Gelu) + Dense + loss");
	assert!(kinds[0] == OpKind::Dense(Activation::Linear));
	assert!(kinds[1] == OpKind::Activation(Activation::Gelu));
	assert!(kinds[2] == OpKind::Dense(Activation::Linear));
	assert!(kinds[3] == OpKind::LossReduce(Loss::Mse));

	assert_eq!(
		activation_op_count(&g),
		1,
		"Gelu is in saves_preact — exactly one Activation op"
	);

	let act =
		g.ops.iter()
			.find(|o| matches!(o.kind, OpKind::Activation(_)))
			.expect("activation op present");
	assert_eq!(act.inputs.len(), 1, "activation consumes one value");
	assert_eq!(act.outputs.len(), 1, "activation produces one value");
	let preact_id = act.inputs[0];
	let preact = g
		.values
		.iter()
		.find(|v| v.id == preact_id)
		.expect("preact value present");
	let structural = preact
		.produced_by
		.expect("preact produced by the structural op");
	let structural_kind =
		g.ops.iter()
			.find(|o| o.id == structural)
			.map(|o| o.kind)
			.expect("structural op present");
	assert!(
		matches!(structural_kind, OpKind::Dense(_)),
		"preact must be produced by the structural Dense op"
	);

	assert_eq!(
		g.rev_edges,
		vec![
			DiffEdge {
				from: ValueId(4),
				to: ValueId(3),
			},
			DiffEdge {
				from: ValueId(3),
				to: ValueId(2),
			},
		],
		"rev value edges track layer outputs, not the preact node"
	);
	assert_eq!(g.rev_edges.len(), dims.len(), "rev_edges count == layers");

	assert!(
		g.fwd_edges.windows(2).all(|w| w[0].to == w[1].from),
		"fwd_edges form a single chain"
	);
	assert_eq!(
		g.fwd_edges.last().map(|e| e.to),
		Some(g.objectives[0].scalar),
		"fwd chain ends at the loss scalar"
	);

	scalar_is_rank0(&g);
	wrt_covers_all_params(&g);
}

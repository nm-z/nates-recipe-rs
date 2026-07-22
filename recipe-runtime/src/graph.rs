use recipe_ir::graph::{
	DiffEdge, Dim, ObjectApplication, ObjectiveNode, OpId, OpKind, OpNode, ParamId, ParamNode,
	SemanticGraph, Shape, ValueId, ValueNode,
};
use recipe_ir::{Activation, LayerDims, LayerKind, Loss, ObjectId, Param};

pub fn build_forward_graph(model_id: ObjectId, dims: &[LayerDims], loss: Loss) -> SemanticGraph {
	let mut g = SemanticGraph::default();
	let mut nv: u32 = 0;
	let mut no: u32 = 0;
	let mut np: u32 = 0;

	let in_dim = dims.first().map_or(0, |d| d.in_dim);
	let v_in = ValueId(nv);
	nv += 1;
	g.values.push(ValueNode {
		id: v_in,
		shape: Shape {
			dims: vec![Dim(in_dim)],
		},
		produced_by: None,
	});

	let l_count = dims.len();
	let mut layer_out: Vec<ValueId> = Vec::with_capacity(l_count);
	let mut prev = v_in;

	for d in dims.iter() {
		let kind = match d.kind {
			LayerKind::Dense => OpKind::Dense,
			LayerKind::Embed => OpKind::Embed,
			LayerKind::Attn => OpKind::Attn,
			LayerKind::Conv => OpKind::Conv,
		};
		let op_id = OpId(no);
		no += 1;

		let w_id = ParamId(np);
		np += 1;
		g.params.push(ParamNode {
			id: w_id,
			owner: op_id,
			role: Param::W,
		});
		let b_id = ParamId(np);
		np += 1;
		g.params.push(ParamNode {
			id: b_id,
			owner: op_id,
			role: Param::B,
		});

		let saves =
			d.act != Activation::Linear && crate::plan::saves_preact(d.act).is_some();

		let out_val = match saves {
			true => {
				let preact = ValueId(nv);
				nv += 1;
				g.values.push(ValueNode {
					id: preact,
					shape: Shape {
						dims: vec![Dim(d.out_dim)],
					},
					produced_by: Some(op_id),
				});
				g.ops.push(OpNode {
					id: op_id,
					kind,
					inputs: vec![prev],
					outputs: vec![preact],
				});
				let act_op = OpId(no);
				no += 1;
				let out = ValueId(nv);
				nv += 1;
				g.values.push(ValueNode {
					id: out,
					shape: Shape {
						dims: vec![Dim(d.out_dim)],
					},
					produced_by: Some(act_op),
				});
				g.ops.push(OpNode {
					id: act_op,
					kind: OpKind::Activation(d.act),
					inputs: vec![preact],
					outputs: vec![out],
				});
				out
			}
			false => {
				let out = ValueId(nv);
				nv += 1;
				g.values.push(ValueNode {
					id: out,
					shape: Shape {
						dims: vec![Dim(d.out_dim)],
					},
					produced_by: Some(op_id),
				});
				g.ops.push(OpNode {
					id: op_id,
					kind,
					inputs: vec![prev],
					outputs: vec![out],
				});
				out
			}
		};

		g.applications.push(ObjectApplication {
			object: model_id,
			op: op_id,
		});
		g.fwd_edges.push(DiffEdge {
			from: prev,
			to: out_val,
		});
		layer_out.push(out_val);
		prev = out_val;
	}

	let v_last = prev;
	let loss_op = OpId(no);
	let scalar = ValueId(nv);
	g.values.push(ValueNode {
		id: scalar,
		shape: Shape { dims: vec![] },
		produced_by: Some(loss_op),
	});
	g.ops.push(OpNode {
		id: loss_op,
		kind: OpKind::LossReduce(loss),
		inputs: vec![v_last],
		outputs: vec![scalar],
	});
	g.fwd_edges.push(DiffEdge {
		from: v_last,
		to: scalar,
	});

	let wrt: Vec<ParamId> = g.params.iter().map(|p| p.id).collect();
	g.objectives.push(ObjectiveNode { scalar, wrt });

	g.rev_edges.push(DiffEdge {
		from: scalar,
		to: v_last,
	});
	for l in (1..l_count).rev() {
		g.rev_edges.push(DiffEdge {
			from: layer_out[l],
			to: layer_out[l - 1],
		});
	}

	return g;
}

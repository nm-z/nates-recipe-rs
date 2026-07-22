use recipe_ir::graph::{
	Dependency, DiffEdge, Dim, ObjectApplication, ObjectiveNode, OpId, OpKind, OpNode, ParamId,
	ParamNode, SemanticGraph, Shape, TargetProducer, ValueId, ValueNode,
};
use recipe_ir::{Activation, LayerDims, LayerKind, LayerSpec, Loss, ObjectId, Param};
use std::collections::HashSet;

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
	let mut prev_width = in_dim;

	for d in dims.iter() {
		let saves =
			d.act != Activation::Linear && crate::plan::saves_preact(d.act).is_some();
		let fused = match saves {
			true => Activation::Linear,
			false => d.act,
		};
		let kind = match d.kind {
			LayerKind::Dense => OpKind::Dense(fused),
			LayerKind::Embed => OpKind::Embed {
				dim: d.dim,
				vocab: d.vocab,
			},
			LayerKind::Attn => OpKind::Attn {
				dim: d.dim,
				heads: d.heads,
			},
			LayerKind::Conv => OpKind::Conv {
				cin: d.conv_cin,
				k: d.conv_k,
				stride: d.conv_stride,
				act: fused,
			},
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

		let side = d.in_dim.saturating_sub(prev_width);
		let inputs = match side {
			0 => vec![prev],
			c => {
				let v_side = ValueId(nv);
				nv += 1;
				g.values.push(ValueNode {
					id: v_side,
					shape: Shape {
						dims: vec![Dim(c)],
					},
					produced_by: None,
				});
				vec![prev, v_side]
			}
		};

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
					inputs,
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
					inputs,
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
		prev_width = d.out_dim;
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

pub fn logical_dims(specs: &[LayerSpec], in_dim: usize) -> Vec<LayerDims> {
	let vocab = recipe_ir::pinned_vocab(specs).unwrap_or(0);
	let mut dims: Vec<LayerDims> = Vec::with_capacity(specs.len());
	let mut cur = in_dim;
	for spec in specs.iter() {
		let ld = match *spec {
			LayerSpec::Dense(units, act) => {
				let out = units;
				let d = LayerDims {
					kind: LayerKind::Dense,
					in_dim: cur,
					out_dim: out,
					act,
					dim: 0,
					vocab: 0,
					heads: 0,
					conv_cin: 0,
					conv_k: 0,
					conv_stride: 0,
				};
				cur = out;
				d
			}
			LayerSpec::Embed(dim, _vocab_spec) => {
				let out = cur * dim;
				let d = LayerDims {
					kind: LayerKind::Embed,
					in_dim: cur,
					out_dim: out,
					act: Activation::Linear,
					dim,
					vocab,
					heads: 0,
					conv_cin: 0,
					conv_k: 0,
					conv_stride: 0,
				};
				cur = out;
				d
			}
			LayerSpec::Attn(heads) => {
				let d_tok = dims.last().map_or(cur, |e: &LayerDims| {
					if e.kind == LayerKind::Embed {
						e.dim
					} else {
						e.out_dim
					}
				});
				LayerDims {
					kind: LayerKind::Attn,
					in_dim: cur,
					out_dim: cur,
					act: Activation::Linear,
					dim: d_tok,
					vocab: 0,
					heads,
					conv_cin: 0,
					conv_k: 0,
					conv_stride: 0,
				}
			}
			LayerSpec::Conv(filters, kernel, stride, act) => {
				let cin = dims.last().map_or(1, |prev: &LayerDims| {
					if prev.kind == LayerKind::Conv {
						let prev_lout = (prev.in_dim / prev.conv_cin - prev.conv_k)
							/ prev.conv_stride
							+ 1;
						prev.out_dim / prev_lout
					} else {
						1
					}
				});
				let lin = cur / cin;
				let lout = (lin - kernel) / stride + 1;
				let out = filters * lout;
				let d = LayerDims {
					kind: LayerKind::Conv,
					in_dim: cur,
					out_dim: out,
					act,
					dim: 0,
					vocab: 0,
					heads: 0,
					conv_cin: cin,
					conv_k: kernel,
					conv_stride: stride,
				};
				cur = out;
				d
			}
		};
		dims.push(ld);
	}
	return dims;
}

fn loss_input_value(g: &SemanticGraph) -> Option<ValueId> {
	return g
		.ops
		.iter()
		.find(|o| matches!(o.kind, OpKind::LossReduce(_)))
		.and_then(|o| o.inputs.first().copied());
}

pub fn compose_surrogate(referrer: &SemanticGraph, referenced: &SemanticGraph) -> SemanticGraph {
	let nv = referrer.values.len() as u32;
	let no = referrer.ops.len() as u32;
	let np = referrer.params.len() as u32;
	let rv = |v: ValueId| ValueId(v.0 + nv);
	let ro = |o: OpId| OpId(o.0 + no);
	let rp = |p: ParamId| ParamId(p.0 + np);

	let mut g = referrer.clone();

	for v in referenced.values.iter() {
		g.values.push(ValueNode {
			id: rv(v.id),
			shape: v.shape.clone(),
			produced_by: v.produced_by.map(ro),
		});
	}
	for o in referenced.ops.iter() {
		g.ops.push(OpNode {
			id: ro(o.id),
			kind: o.kind,
			inputs: o.inputs.iter().copied().map(rv).collect(),
			outputs: o.outputs.iter().copied().map(rv).collect(),
		});
	}
	for p in referenced.params.iter() {
		g.params.push(ParamNode {
			id: rp(p.id),
			owner: ro(p.owner),
			role: p.role,
		});
	}
	for a in referenced.applications.iter() {
		g.applications.push(ObjectApplication {
			object: a.object,
			op: ro(a.op),
		});
	}
	for d in referenced.deps.iter() {
		g.deps.push(Dependency {
			from: rv(d.from),
			to: ro(d.to),
		});
	}
	for t in referenced.target_producers.iter() {
		g.target_producers.push(TargetProducer {
			target: t.target,
			value: rv(t.value),
		});
	}
	for e in referenced.fwd_edges.iter() {
		g.fwd_edges.push(DiffEdge {
			from: rv(e.from),
			to: rv(e.to),
		});
	}
	for e in referenced.rev_edges.iter() {
		g.rev_edges.push(DiffEdge {
			from: rv(e.from),
			to: rv(e.to),
		});
	}

	let referrer_last = loss_input_value(referrer).unwrap_or(ValueId(0));
	let referenced_in = rv(ValueId(0));
	let referenced_first_op = referenced.ops.first().map_or(OpId(no), |o| ro(o.id));
	let referenced_first_out = referenced
		.rev_edges
		.last()
		.map_or(referenced_in, |e| rv(e.to));

	g.deps.push(Dependency {
		from: referrer_last,
		to: referenced_first_op,
	});
	g.fwd_edges.push(DiffEdge {
		from: referrer_last,
		to: referenced_in,
	});
	g.rev_edges.push(DiffEdge {
		from: referenced_first_out,
		to: referenced_in,
	});
	g.rev_edges.push(DiffEdge {
		from: referenced_in,
		to: referrer_last,
	});

	let scalar = rv(referenced
		.objectives
		.first()
		.map_or(ValueId(0), |ob| ob.scalar));
	let mut wrt: Vec<ParamId> = referrer.params.iter().map(|p| p.id).collect();
	wrt.extend(referenced.params.iter().map(|p| rp(p.id)));
	g.objectives = vec![ObjectiveNode { scalar, wrt }];

	return g;
}

pub fn rev_reached_ops(g: &SemanticGraph) -> HashSet<u32> {
	let mut ops: HashSet<u32> = HashSet::new();
	let Some(start) = g.objectives.first().map(|o| o.scalar) else {
		return ops;
	};
	let mut seen: HashSet<u32> = HashSet::new();
	seen.insert(start.0);
	let mut stack = vec![start];
	while let Some(v) = stack.pop() {
		for e in g.rev_edges.iter() {
			if e.from == v && seen.insert(e.to.0) {
				stack.push(e.to);
			}
		}
	}
	for val in g.values.iter() {
		if seen.contains(&val.id.0) {
			if let Some(op) = val.produced_by {
				ops.insert(op.0);
			}
		}
	}
	return ops;
}

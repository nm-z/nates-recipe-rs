use crate::{Graph, NodeId};

#[derive(Clone, Copy)]
enum Placement { First, Inline, Line(usize), }

pub(crate) fn serialize(graph: &Graph) -> String { let mut output = String::new();
	let mut pending = Vec::with_capacity(graph.len());
	for (index, root) in graph.roots().iter().copied().enumerate().rev() { let placement = if index == 0 { Placement::First
		} else { Placement::Line(0) }; pending.push((root, 0, placement)); }

	while let Some((id, depth, placement)) = pending.pop() { match placement { Placement::First => {}
			Placement::Inline => output.push('\t'),
			Placement::Line(indentation) => {
				output.push('\n');
				for _ in 0..indentation {
					output.push('\t');
				} } }
		let node = graph .node(id)
			.expect("graph-owned node identity must resolve");
		output.push_str(node.text()); push_children(&mut pending, node.children(), depth + 1); }
	output }

fn push_children(pending: &mut Vec<(NodeId, usize, Placement)>, children: &[NodeId], depth: usize) {
	for (index, child) in children.iter().copied().enumerate().rev() { let placement = if index == 0 { Placement::Inline
		} else { Placement::Line(depth) }; pending.push((child, depth, placement)); } }

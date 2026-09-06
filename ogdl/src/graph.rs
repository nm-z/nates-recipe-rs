use core::{fmt, str::FromStr};

use write::{Errored, err};

/// Stable identity of a node inside one [`Graph`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(usize);

impl NodeId {
	#[must_use]
	pub const fn index(self) -> usize { self.0 }
}

/// One immutable-text node and its ordered tree relationships.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
	text: String,
	parent: Option<NodeId>,
	children: Vec<NodeId>,
}

impl Node {
	#[must_use]
	pub fn text(&self) -> &str { &self.text }

	#[must_use]
	pub const fn parent(&self) -> Option<NodeId> { self.parent }

	#[must_use]
	pub fn children(&self) -> &[NodeId] { &self.children }
}

/// An ordered forest stored in an append-only arena.
///
/// Fields are private so every `NodeId`, parent edge, and child edge remains
/// internally consistent. Node identities are local to one graph.
#[derive(Clone, Debug, Default)]
pub struct Graph {
	nodes: Vec<Node>,
	roots: Vec<NodeId>,
}

impl Graph {
	#[must_use]
	pub const fn new() -> Self {
		Self {
			nodes: Vec::new(),
			roots: Vec::new(),
		}
	}

	pub fn parse(input: &str) -> Result<Self, Errored> { crate::parser::parse(input) }

	#[must_use]
	pub fn len(&self) -> usize { self.nodes.len() }

	#[must_use]
	pub fn is_empty(&self) -> bool { self.nodes.is_empty() }

	#[must_use]
	pub fn roots(&self) -> &[NodeId] { &self.roots }

	#[must_use]
	pub fn node(&self, id: NodeId) -> Option<&Node> { self.nodes.get(id.0) }

	pub fn nodes(&self) -> impl ExactSizeIterator<Item = (NodeId, &Node)> {
		self.nodes
			.iter()
			.enumerate()
			.map(|(index, node)| (NodeId(index), node))
	}

	pub fn append_root(&mut self, text: impl Into<String>) -> Result<NodeId, Errored> {
		let text = text.into();
		validate_text(&text)?;
		Ok(self.push_node(None, text))
	}

	pub fn append_child(&mut self, parent: NodeId, text: impl Into<String>) -> Result<NodeId, Errored> {
		if self.node(parent).is_none() {
			err(format!("unknown parent node {}", parent.index()))?;
		}
		let text = text.into();
		validate_text(&text)?;
		Ok(self.push_node(Some(parent), text))
	}

	#[must_use]
	pub fn to_canonical_string(&self) -> String { crate::serializer::serialize(self) }

	pub(crate) fn push_node(&mut self, parent: Option<NodeId>, text: String) -> NodeId {
		let id = NodeId(self.nodes.len());
		self.nodes.push(Node {
			text,
			parent,
			children: Vec::new(),
		});
		if let Some(parent) = parent {
			self.nodes[parent.0].children.push(id);
		} else {
			self.roots.push(id);
		}
		id
	}
}

impl PartialEq for Graph {
	fn eq(&self, other: &Self) -> bool {
		if self.roots.len() != other.roots.len() || self.nodes.len() != other.nodes.len() {
			return false;
		}
		let mut pending = self
			.roots
			.iter()
			.copied()
			.zip(other.roots.iter().copied())
			.collect::<Vec<_>>();
		while let Some((left_id, right_id)) = pending.pop() {
			let left = &self.nodes[left_id.0];
			let right = &other.nodes[right_id.0];
			if left.text != right.text || left.children.len() != right.children.len() {
				return false;
			}
			pending.extend(
				left.children
					.iter()
					.copied()
					.zip(right.children.iter().copied()),
			);
		}
		true
	}
}

impl Eq for Graph {}

impl FromStr for Graph {
	type Err = Errored;

	fn from_str(input: &str) -> Result<Self, Self::Err> { Self::parse(input) }
}

impl fmt::Display for Graph {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.to_canonical_string()) }
}

fn validate_text(text: &str) -> Result<(), Errored> {
	if text.is_empty() {
		return err("invalid node text at character 1: empty");
	}
	for (index, character) in text.chars().enumerate() {
		let what = match character {
			'\t' => Some("tab"),
			'\r' => Some("carriage return"),
			'\n' => Some("line feed"),
			_ => None,
		};
		if let Some(what) = what {
			return err(format!(
				"invalid node text at character {}: {what}",
				index + 1
			));
		}
	}
	Ok(())
}

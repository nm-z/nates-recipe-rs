//! Minimal OGDL: an indentation-defined tree of name/value nodes. Parse text to
//! a tree, look up by dotted path, build one programmatically, serialize back.
//! Nothing else — no templates, expressions, schemas, binary, or flow format.

use std::path::Path;
use std::{fmt, fs, io};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
	pub name: String,
	pub value: Option<String>,
	pub children: Vec<Node>,
}

impl Node {
	/// A bare node: a name, no value, no children.
	pub fn new(name: &str) -> Node {
		Node { name: name.to_string(), value: None, children: Vec::new() }
	}

	/// Parse OGDL text into a root container whose children are the top-level
	/// entries. Leading whitespace sets nesting by raw width; `name=value` splits
	/// on the first `=`; a bare word is name-only; blank lines are skipped.
	pub fn parse(text: &str) -> Node {
		let mut root = Node::new("");
		// `path` = deepest open node by child index; `stack` = its indent width.
		let mut path: Vec<usize> = Vec::new();
		let mut stack: Vec<usize> = Vec::new();
		for line in text.lines() {
			let content = line.trim();
			if content.is_empty() {
				continue;
			}
			let indent = line.len() - line.trim_start().len();
			while stack.last().is_some_and(|&w| w >= indent) {
				stack.pop();
				path.pop();
			}
			let node = match content.split_once('=') {
				Some((k, v)) => Node { name: k.trim().to_string(), value: Some(v.trim().to_string()), children: Vec::new() },
				None => Node::new(content),
			};
			let parent = root.at_mut(&path);
			parent.children.push(node);
			path.push(parent.children.len() - 1);
			stack.push(indent);
		}
		root
	}

	fn at_mut(&mut self, path: &[usize]) -> &mut Node {
		let mut n = self;
		for &i in path {
			n = &mut n.children[i];
		}
		n
	}

	/// Look up a node by a dot-separated path of child names.
	pub fn get(&self, path: &str) -> Option<&Node> {
		let mut n = self;
		for seg in path.split('.') {
			n = n.children.iter().find(|c| c.name == seg)?;
		}
		Some(n)
	}

	/// The value at `path`, if present.
	pub fn get_value(&self, path: &str) -> Option<&str> {
		self.get(path)?.value.as_deref()
	}
	/// The value at `path` parsed as one `f64`.
	pub fn get_f64(&self, path: &str) -> Option<f64> {
		self.get_value(path)?.parse().ok()
	}
	/// The value at `path` split on whitespace, each token parsed as `f64`.
	pub fn get_values(&self, path: &str) -> Option<Vec<f64>> {
		self.get_value(path)?.split_whitespace().map(|t| t.parse().ok()).collect()
	}

	/// Append a `key=value` child; returns `&mut self` so calls chain.
	pub fn set(&mut self, key: &str, value: &str) -> &mut Node {
		self.children.push(Node { name: key.to_string(), value: Some(value.to_string()), children: Vec::new() });
		self
	}
	/// Append a bare child and return `&mut` to it, so further `.set()`s nest in.
	pub fn child(&mut self, name: &str) -> &mut Node {
		self.children.push(Node::new(name));
		self.children.last_mut().expect("just pushed")
	}

	/// Read a file and parse it.
	pub fn from_file(path: &str) -> io::Result<Node> {
		Ok(Node::parse(&fs::read_to_string(path)?))
	}
	/// Serialize and write to a file, creating parent dirs as needed.
	pub fn to_file(&self, path: &str) -> io::Result<()> {
		if let Some(dir) = Path::new(path).parent()
			&& !dir.as_os_str().is_empty()
		{
			fs::create_dir_all(dir)?;
		}
		fs::write(path, self.to_string())
	}

	fn write_at(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
		for _ in 0..depth {
			f.write_str("    ")?;
		}
		match &self.value {
			Some(v) => writeln!(f, "{}={}", self.name, v)?,
			None => writeln!(f, "{}", self.name)?,
		}
		for c in &self.children {
			c.write_at(f, depth + 1)?;
		}
		Ok(())
	}
}

/// Inverse of `parse`: four spaces per depth; the root's own name is not printed.
impl fmt::Display for Node {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for c in &self.children {
			c.write_at(f, 0)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::Node;
	const SAMPLE: &str = "acc=0.987
embed
    0=-0.03 0.18
    1=0.08 -0.21
z1
    w=0.01 -0.02
    b=0.001
";
	#[test]
	fn round_trip() {
		assert_eq!(Node::parse(SAMPLE).to_string(), SAMPLE);
	}
	#[test]
	fn structure_and_gets() {
		let t = Node::parse(SAMPLE);
		let names: Vec<&str> = t.children.iter().map(|c| c.name.as_str()).collect();
		assert_eq!(names, ["acc", "embed", "z1"]);
		assert_eq!(t.get("embed").expect("embed").children.len(), 2);
		assert_eq!(t.get_value("z1.b"), Some("0.001"));
		assert_eq!(t.get_f64("acc"), Some(0.987));
		assert_eq!(t.get_values("z1.w"), Some(vec![0.01, -0.02]));
		assert_eq!(t.get_value("embed.0"), Some("-0.03 0.18"));
		assert!(t.get("missing.x").is_none());
		assert!(t.get_f64("z1.w").is_none());
	}
	#[test]
	fn build_round_trips() {
		let mut root = Node::new("model");
		root.set("acc", "0.987");
		root.child("embed").set("0", "-0.03 0.18").set("1", "0.08 -0.21");
		root.child("z1").set("w", "0.01 -0.02").set("b", "0.001");
		let text = root.to_string();
		assert_eq!(text, SAMPLE);
		let back = Node::parse(&text);
		assert_eq!(back.get_f64("acc"), Some(0.987));
		assert_eq!(back.get_values("embed.1"), Some(vec![0.08, -0.21]));
		assert_eq!(back.get_value("z1.b"), Some("0.001"));
	}
	#[test]
	fn empty() {
		let t = Node::parse("");
		assert!(t.children.is_empty());
		assert_eq!(t.to_string(), "");
	}
	#[test]
	fn nested_three_levels() {
		let text = "a\n    b\n        c=1\n";
		let t = Node::parse(text);
		assert_eq!(t.get_value("a.b.c"), Some("1"));
		assert_eq!(t.to_string(), text);
		let mut r = Node::new("r");
		r.child("a").child("b").child("c").set("d", "9");
		assert_eq!(r.get_value("a.b.c.d"), Some("9"));
	}
	#[test]
	fn blank_lines_and_tabs() {
		let t = Node::parse("\n\nx=1\n   \ny=2\n");
		assert_eq!(t.children.len(), 2);
		assert_eq!(t.get_value("x"), Some("1"));
		assert_eq!(Node::parse("p\n\tq=2\n").get_value("p.q"), Some("2"));
	}
}

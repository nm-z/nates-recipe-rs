use std::fmt;
use std::fs;
use std::ops;
use std::sync::{Arc, LazyLock, Mutex};

const NL: char = '\u{a}';

const FIRST_LINE_SIBLINGS: bool = true;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
	pub name: String,
	pub children: Vec<Self>,
}

impl Node {
	pub(crate) fn leaf(name: &str) -> Self {
		return Self {
			name: name.to_string(),
			children: Vec::new(),
		};
	}

	#[inline]
	pub fn parse(text: &str) -> Self {
		let mut root = Self::leaf("");
		let mut stack: Vec<Vec<usize>> = Vec::new();
		for raw in text.lines() {
			if raw.trim().is_empty() {
				continue;
			}
			let tabs = raw.chars().take_while(|&c| return c == '\t').count();
			let depth = tabs.min(stack.len());
			let mut toks = raw[tabs..].split('\t');
			let name = toks.next().unwrap_or("");
			let mut node = Self::leaf(name);
			for v in toks {
				node.children.push(Self::leaf(v));
			}
			let parent = match depth {
				0 => Vec::new(),
				_ => stack[depth.saturating_sub(1)].clone(),
			};
			let n = Self::at_mut(&mut root, &parent);
			n.children.push(node);
			let mut path = parent;
			path.push(n.children.len().saturating_sub(1));
			stack.truncate(depth);
			stack.push(path);
		}
		return root;
	}

	fn at_mut<'a>(mut n: &'a mut Self, path: &[usize]) -> &'a mut Self {
		for &i in path {
			n = &mut n.children[i];
		}
		return n;
	}

	#[inline]
	pub fn select(&self, path: &str) -> Option<&Self> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| return !s.is_empty()) {
			cur = cur.step(seg)?;
		}
		return Some(cur);
	}

	fn step(&self, seg: &str) -> Option<&Self> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				return self.children.get(i.wrapping_sub(1));
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = &seg[..brace];
					let sel = seg[brace.saturating_add(1)..].trim_end_matches('}');
					let nth: usize = match sel.chars().next() {
						Some(_c) => sel.parse().ok()?,
						None => 1,
					};
					return self
						.children
						.iter()
						.filter(|c| return c.name == name)
						.nth(nth.wrapping_sub(1));
				}
				None => return self.children.iter().find(|c| return c.name == seg),
			},
		}
	}

	pub(crate) fn select_mut(&mut self, path: &str) -> Option<&mut Self> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| return !s.is_empty()) {
			let idx = cur.step_index(seg)?;
			cur = &mut cur.children[idx];
		}
		return Some(cur);
	}

	fn step_index(&self, seg: &str) -> Option<usize> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				return i.checked_sub(1).filter(|&i| return i < self.children.len());
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = &seg[..brace];
					let sel = seg[brace.saturating_add(1)..].trim_end_matches('}');
					let nth: usize = match sel.chars().next() {
						Some(_c) => sel.parse().ok()?,
						None => 1,
					};
					return (0..self.children.len())
						.filter(|&i| return self.children[i].name == name)
						.nth(nth.wrapping_sub(1));
				}
				None => return self.children.iter().position(|c| return c.name == seg),
			},
		}
	}

	fn write_to(&self, out: &mut String, depth: usize) {
		let mut lines = Vec::new();
		walk_lines(self, &mut Vec::new(), depth, &mut lines);
		for (l, _p, _r) in lines {
			out.push_str(&l);
			out.push(NL);
		}
	}

	#[inline]
	pub fn serialize(&self) -> String {
		let mut s = String::new();
		self.write_to(&mut s, 0);
		return s;
	}
}

impl ops::Index<usize> for Node {
	type Output = Self;
	#[inline]
	fn index(&self, i: usize) -> &Self {
		return &self.children[i];
	}
}

impl fmt::Display for Node {
	#[inline]
	fn fmt<'z>(&self, f: &mut fmt::Formatter<'z>) -> fmt::Result {
		let vals: Vec<&str> = self
			.children
			.iter()
			.filter(|c| return c.children.is_empty())
			.map(|c| return c.name.as_str())
			.collect();
		return f.write_str(&vals.join(" "));
	}
}

#[derive(Clone)]
pub struct Graph {
	pub root: Arc<Mutex<Node>>,
	pub shown: Arc<Mutex<usize>>,
}

impl Graph {
	#[inline]
	pub fn empty() -> Self {
		return Self {
			root: Arc::new(Mutex::new(Node::leaf(""))),
			shown: Arc::new(Mutex::new(0)),
		};
	}

	#[inline]
	pub fn with<R>(&self, f: impl FnOnce(&mut Node) -> R) -> R {
		let mut n = self.root.lock().unwrap_or_else(|p| return p.into_inner());
		return f(&mut n);
	}
}

mod alias {
	use super::{Graph, LazyLock};
	pub static OGDL: LazyLock<Graph> = LazyLock::new(Graph::empty);
}
pub use alias::OGDL as ogdl;

pub struct Ogdl;

impl Ogdl {
	#[inline]
	pub fn file(path: &str) -> Graph {
		return Graph::read(path);
	}
}

#[inline]
pub fn file(path: &str) -> Graph {
	return Graph::read(path);
}

#[inline]
pub fn text(src: &str) -> Graph {
	let g = Graph::empty();
	g.with(|root| *root = Node::parse(src));
	return g;
}

pub trait ItnlArg {
	type Out;
	fn apply(self, g: Graph) -> Self::Out;
}

impl ItnlArg for () {
	type Out = Graph;
	#[inline]
	fn apply(self, g: Graph) -> Graph {
		return g;
	}
}

impl ItnlArg for &str {
	type Out = Node;
	#[inline]
	fn apply(self, g: Graph) -> Node {
		return g.with(|root| {
			let mut n = root.select(self).cloned().unwrap_or_default();
			n.name = self.to_string();
			return n;
		});
	}
}

impl ItnlArg for Graph {
	type Out = Self;
	#[inline]
	fn apply(self, g: Graph) -> Self {
		let src = self.snapshot();
		g.with(|root| *root = src);
		return g;
	}
}

impl ItnlArg for Node {
	type Out = Graph;
	#[inline]
	fn apply(self, g: Graph) -> Graph {
		g.with(|root| *root = self);
		return g;
	}
}

pub trait DelArg {
	fn apply(self, g: Graph);
}

pub struct NamedChild<'a> {
	pub name: &'a str,
	pub parent: &'a Node,
}

impl<'a> DelArg for NamedChild<'a> {
	#[inline]
	fn apply(self, g: Graph) {
		let name = self.name;
		let parent = self.parent;
		g.with(|root| match root.select_mut(&parent.name) {
			Some(p) => {
				p.children.retain(|c| return c.name != name);
			}
			None => {
				root.children.retain(|c| return c.name != name);
			}
		});
	}
}

impl DelArg for &Node {
	#[inline]
	fn apply(self, g: Graph) {
		fn strip(n: &mut Node, target: &Node) {
			n.children.retain(|c| return c != target);
			for c in &mut n.children {
				strip(c, target);
			}
		}
		g.with(|root| strip(root, self));
	}
}

pub trait Value {
	fn into_nodes(self) -> Vec<Node>;
}

macro_rules! value {
      (scalar: $($t:ty),*) => {$( impl Value for $t {
            #[inline]
            fn into_nodes(self) -> Vec<Node> { return vec![Node::leaf(&self.to_string())] }
      } )*};
      (str: $($t:ty),*) => {$( impl Value for $t {
            #[inline]
            fn into_nodes(self) -> Vec<Node> { return vec![Node::leaf(self.as_ref())] }
      } )*};
      (floats: $($t:ty),*) => {$( impl Value for $t {
            #[inline]
            fn into_nodes(self) -> Vec<Node> { return self.iter().map(|x| return Node::leaf(&x.to_string())).collect() }
      } )*};
      (strs: $($t:ty),*) => {$( impl Value for $t {
            #[inline]
            fn into_nodes(self) -> Vec<Node> { return self.iter().map(|s| return Node::leaf(s)).collect() }
      } )*};
}
value!(scalar: f64, i64, i32, usize, bool);
value!(str: &str, String, &String);
value!(floats: Vec<f64>, &[f64]);
value!(strs: Vec<String>, &[String]);

impl Graph {
	fn read(path: &str) -> Self {
		let g = Self::empty();
		let text = fs::read_to_string(path).unwrap_or_default();
		g.with(|root| *root = Node::parse(&text));
		return g;
	}

	#[inline]
	pub fn snapshot(&self) -> Node {
		return self.with(|root| return root.clone());
	}

	#[inline]
	pub fn section(&self, path: &str) -> String {
		return self.with(|root| {
			let depth = path
				.split('.')
				.filter(|s| return !s.is_empty())
				.count()
				.saturating_sub(1);
			match root.select(path) {
				None => return String::new(),
				Some(n) => {
					let wrap = Node {
						name: String::new(),
						children: vec![n.clone()],
					};
					let mut s = String::new();
					wrap.write_to(&mut s, depth);
					return s;
				}
			}
		});
	}

	#[inline]
	pub fn itnl<A: ItnlArg>(&self, a: A) -> A::Out {
		return a.apply(self.clone());
	}

	#[inline]
	pub fn file(&self, path: &str) -> Self {
		let head = self.with(|root| return root.children.first().map(|_c| ()));
		match head {
			None => {
				let text = fs::read_to_string(path).unwrap_or_default();
				self.with(|root| *root = Node::parse(&text));
			}
			Some(_head) => {
				let text = self.with(|root| return root.serialize());
				fs::write(path, text).ok();
			}
		}
		return self.clone();
	}

	#[inline]
	pub fn add<V: Value>(&self, value: V, path: &str) -> Self {
		let kids = value.into_nodes();
		self.with(|root| {
			let mut cur = root;
			for seg in path.split('.').filter(|s| return !s.is_empty()) {
				let idx = match cur.children.iter().position(|c| return c.name == seg) {
					Some(i) => i,
					None => {
						cur.children.push(Node::leaf(seg));
						cur.children.len().saturating_sub(1)
					}
				};
				cur = &mut cur.children[idx];
			}
			cur.children.extend(kids);
		});
		return self.clone();
	}

	#[inline]
	pub fn del<D: DelArg>(&self, d: D) -> Self {
		d.apply(self.clone());
		return self.clone();
	}
}

pub struct Block {
	pub nodes: Vec<Node>,
	pub sel: Option<Vec<usize>>,
}

/// Flattens a node subtree into tab-indented text lines paired with their child-index path and a rider flag.
/// Rider children collapse onto one tab-separated line and are not descended into; all others recurse one depth deeper.
fn walk_lines(
	n: &Node,
	path: &mut Vec<usize>,
	depth: usize,
	out: &mut Vec<(String, Vec<usize>, bool)>,
) {
	for (i, c) in n.children.iter().enumerate() {
		path.push(i);
		let mut line = String::new();
		for _t in 0..depth {
			line.push('\t');
		}
		line.push_str(&c.name);
		match !c.children.is_empty()
			&& c.children.iter().all(|leaf| {
				return leaf.children.is_empty()
					&& !leaf.name.contains('\t')
					&& (leaf.name.parse::<f64>().is_ok()
						|| leaf.name.chars().any(|ch| return !ch.is_ascii()));
			}) {
			true => {
				for v in &c.children {
					line.push('\t');
					line.push_str(&v.name);
				}
				out.push((line, path.clone(), true));
			}
			false => {
				out.push((line, path.clone(), false));
				walk_lines(c, path, depth.saturating_add(1), out);
			}
		}
		path.pop();
	}
}

/// Flattens the tree under `root` into rendered rows in document order, each carrying its tab-indented text, child-index path, and whether it is a value-rider line.
/// Rider children collapse onto the parent's line and are not recursed into; the path/rider fields drive Display, `show` selection, and the shown-cursor delta.
fn lines_of(root: &Node) -> Vec<(String, Vec<usize>, bool)> {
	let mut out = Vec::new();
	walk_lines(root, &mut Vec::new(), 0, &mut out);
	return out;
}

impl fmt::Display for Block {
	#[inline]
	fn fmt<'z>(&self, f: &mut fmt::Formatter<'z>) -> fmt::Result {
		let wrap = Node {
			name: String::new(),
			children: self.nodes.clone(),
		};
		let text: Vec<String> = lines_of(&wrap)
			.into_iter()
			.map(|(l, _p, _r)| return l)
			.collect();
		return f.write_str(&text.join("\n"));
	}
}

impl Block {
	#[inline]
	pub fn show(self) -> String {
		let Some(sel) = self.sel else {
			return String::new();
		};
		return ogdl.with(|root| {
			let ls = lines_of(root);
			let hit =
				ls.iter()
					.position(|(_l, p, _r)| return *p == sel)
					.or_else(|| match sel.split_last() {
						Some((_i, par)) => {
							return ls.iter().position(|(_l, p, _r)| return p == par);
						}
						None => return None,
					});
			let Some(mut end) = hit else {
				return String::new();
			};
			while end.saturating_add(1) < ls.len()
				&& ls[end.saturating_add(1)].1.starts_with(&sel)
				&& ls[end.saturating_add(1)].2
			{
				end = end.saturating_add(1);
			}
			let mut sh = ogdl.shown.lock().unwrap_or_else(|p| return p.into_inner());
			if end < *sh {
				return String::new();
			}
			let out: Vec<&str> = ls[*sh..=end]
				.iter()
				.map(|(l, _p, _r)| return l.as_str())
				.collect();
			*sh = end.saturating_add(1);
			return out.join("\n");
		});
	}
}

/// Parses tab-indented OGDL text into a root forest plus the index path to the last inserted node.
/// Leading/trailing blank lines are stripped; the first content line sets the depth anchor and its tab-separated fields become sibling roots; deeper lines nest by tab count relative to the anchor.
/// All-blank input yields two empty vecs.
fn parse_doc(text: &str) -> (Vec<Node>, Vec<usize>) {
	let all: Vec<&str> = text.split(NL).collect();
	let Some(first) = all.iter().position(|l| return !l.trim().is_empty()) else {
		return (Vec::new(), Vec::new());
	};
	let last = all
		.iter()
		.rposition(|l| return !l.trim().is_empty())
		.unwrap_or(first);
	let lines = &all[first..=last];
	let anchor = lines[0].chars().take_while(|&c| return c == '\t').count();
	let mut root = Node::leaf("");
	let mut stack: Vec<Vec<usize>> = Vec::new();
	let mut lastpath: Vec<usize> = Vec::new();
	let mut past_first_newline = false;
	for raw in lines {
		if raw.trim().is_empty() {
			continue;
		}
		let tabs = raw.chars().take_while(|&c| return c == '\t').count();
		let rest = &raw[tabs..];
		match past_first_newline {
			false => {
				match FIRST_LINE_SIBLINGS {
					true => {
						for name in rest.split('\t') {
							root.children.push(Node::leaf(name));
							let path = vec![root.children.len().saturating_sub(1)];
							stack.clear();
							stack.push(path.clone());
							lastpath = path;
						}
					}
					false => {
						for (d, name) in rest.split('\t').enumerate() {
							let parent = match d {
								0 => Vec::new(),
								_ => stack[d.saturating_sub(1)].clone(),
							};
							let n = Node::at_mut(&mut root, &parent);
							n.children.push(Node::leaf(name));
							let mut path = parent;
							path.push(n.children.len().saturating_sub(1));
							stack.truncate(d);
							stack.push(path.clone());
							lastpath = path;
						}
					}
				}
				past_first_newline = true;
			}
			true => {
				let depth = tabs.saturating_sub(anchor).min(stack.len());
				let parent = match depth {
					0 => Vec::new(),
					_ => stack[depth.saturating_sub(1)].clone(),
				};
				let n = Node::at_mut(&mut root, &parent);
				n.children.push(Node::leaf(rest));
				let mut path = parent;
				path.push(n.children.len().saturating_sub(1));
				stack.truncate(depth);
				stack.push(path.clone());
				lastpath = path;
			}
		}
	}
	return (root.children, lastpath);
}

#[macro_export]
macro_rules! ogdl {
	(@go $p:ident; $i:ident) => { $crate::__macro_support::seg(&mut $p, stringify!($i)) };
	(@go $p:ident; $s:literal) => {{
		const _: [(); 0] = [(); $crate::__macro_support::doc_ok($s)];
		$crate::__macro_support::doc(&mut $p, $s);
	}};
	(@go $p:ident; $i:ident [$n:expr]) => {{
		$crate::__macro_support::seg(&mut $p, stringify!($i));
		$crate::__macro_support::idx(&mut $p, $n);
	}};
	(@go $p:ident; $i:ident {$n:expr}) => { $crate::__macro_support::sel(&mut $p, stringify!($i), $n) };
	(@go $p:ident; $i:ident {$n:expr} [$m:expr]) => {{
		$crate::__macro_support::sel(&mut $p, stringify!($i), $n);
		$crate::__macro_support::idx(&mut $p, $m);
	}};
	(@go $p:ident; $i:ident *) => { $crate::__macro_support::star(&mut $p, stringify!($i)) };
	(@go $p:ident; $i:ident {}) => { $crate::__macro_support::del(&mut $p, stringify!($i)) };
	(@go $p:ident; & $i:ident) => { $crate::__macro_support::val(&mut $p, &$i) };
	(@go $p:ident; & $i:ident [$n:expr]) => {{
		$crate::__macro_support::val(&mut $p, &$i);
		$crate::__macro_support::idx(&mut $p, $n);
	}};
	(@go $p:ident; & $i:ident . $($rest:tt)+) => {{
		$crate::__macro_support::val(&mut $p, &$i);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; & $i:ident [$n:expr] . $($rest:tt)+) => {{
		$crate::__macro_support::val(&mut $p, &$i);
		$crate::__macro_support::idx(&mut $p, $n);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $s:literal [$n:expr]) => {{
		$crate::__macro_support::seg(&mut $p, $s);
		$crate::__macro_support::idx(&mut $p, $n);
	}};
	(@go $p:ident; $s:literal {$n:expr}) => { $crate::__macro_support::sel(&mut $p, $s, $n) };
	(@go $p:ident; $s:literal {$n:expr} [$m:expr]) => {{
		$crate::__macro_support::sel(&mut $p, $s, $n);
		$crate::__macro_support::idx(&mut $p, $m);
	}};
	(@go $p:ident; $s:literal *) => { $crate::__macro_support::star(&mut $p, $s) };
	(@go $p:ident; $s:literal {}) => { $crate::__macro_support::del(&mut $p, $s) };
	(@go $p:ident; $s:literal [$n:expr] . $($rest:tt)+) => {{
		$crate::__macro_support::seg(&mut $p, $s);
		$crate::__macro_support::idx(&mut $p, $n);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $s:literal {$n:expr} . $($rest:tt)+) => {{
		$crate::__macro_support::sel(&mut $p, $s, $n);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $s:literal {$n:expr} [$m:expr] . $($rest:tt)+) => {{
		$crate::__macro_support::sel(&mut $p, $s, $n);
		$crate::__macro_support::idx(&mut $p, $m);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $s:literal * . $($rest:tt)+) => {{
		$crate::__macro_support::star(&mut $p, $s);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $i:ident . $($rest:tt)+) => {{
		$crate::__macro_support::seg(&mut $p, stringify!($i));
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $s:literal . $($rest:tt)+) => {{
		const _: [(); 0] = [(); $crate::__macro_support::doc_ok($s)];
		$crate::__macro_support::doc(&mut $p, $s);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $i:ident {$n:expr} . $($rest:tt)+) => {{
		$crate::__macro_support::sel(&mut $p, stringify!($i), $n);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $i:ident {$n:expr} [$m:expr] . $($rest:tt)+) => {{
		$crate::__macro_support::sel(&mut $p, stringify!($i), $n);
		$crate::__macro_support::idx(&mut $p, $m);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $i:ident [$n:expr] . $($rest:tt)+) => {{
		$crate::__macro_support::seg(&mut $p, stringify!($i));
		$crate::__macro_support::idx(&mut $p, $n);
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $i:ident * . $($rest:tt)+) => {{
		$crate::__macro_support::star(&mut $p, stringify!($i));
		$crate::ogdl!(@go $p; $($rest)+);
	}};
	(@go $p:ident; $($bad:tt)*) => {
		const _: [(); 0] = [(); $crate::__macro_support::invalid_path()];
	};
	($($path:tt)+) => {{
		let mut p = $crate::__macro_support::start();
		$crate::ogdl!(@go p; $($path)+);
		$crate::__macro_support::fin(p)
	}};
}

#[macro_export]
macro_rules! del {
	($g:expr, $a:ident . $b:ident {}) => {{ $crate::__macro_support::del_all(&$g, stringify!($a), stringify!($b)) }};
}

#[doc(hidden)]
pub mod __macro_support {
	use super::{Graph, Node, fmt, ogdl, parse_doc};

	#[inline]
	pub fn del_all(g: &Graph, parent: &str, name: &str) -> Graph {
		g.with(|root: &mut Node| {
			let target = match parent.chars().next() {
				None => Some(&mut *root),
				Some(_c) => root.select_mut(parent),
			};
			let Some(p) = target else { return };
			p.children.retain(|c| return c.name != name);
		});
		return g.clone();
	}

	pub struct P {
		pub cur: Vec<Vec<usize>>,
		pub anchor: Option<Vec<Vec<usize>>>,
	}

	#[inline]
	pub fn start() -> P {
		return P {
			cur: vec![Vec::new()],
			anchor: None,
		};
	}

	fn hold(p: &mut P) {
		if p.anchor.is_none() {
			p.anchor = Some(p.cur.clone());
		}
	}

	fn at<'a>(mut n: &'a Node, path: &[usize]) -> &'a Node {
		for &i in path {
			n = &n.children[i];
		}
		return n;
	}

	#[inline]
	pub fn seg(p: &mut P, name: &str) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				let i = match n.children.iter().position(|c| return c.name == name) {
					Some(i) => i,
					None => {
						n.children.push(Node::leaf(name));
						n.children.len().saturating_sub(1)
					}
				};
				let mut q = path.clone();
				q.push(i);
				next.push(q);
			}
			p.cur = next;
		});
		hold(p);
	}

	#[inline]
	pub const fn invalid_path() -> usize {
		return 1;
	}

	#[inline]
	pub const fn doc_ok(s: &str) -> usize {
		let b = s.as_bytes();
		let mut i = 0;
		let mut anchor = 0;
		let mut prev: usize = 0;
		let mut seen = false;
		while i < b.len() {
			let start = i;
			while i < b.len() && b[i] != 10 {
				i = i.saturating_add(1);
			}
			let end = i;
			i = i.saturating_add(1);
			let mut j = start;
			while j < end && (b[j] == 9 || b[j] == 32) {
				j = j.saturating_add(1);
			}
			if j == end {
				continue;
			}
			let mut tabs = start;
			while tabs < end && b[tabs] == 9 {
				tabs = tabs.saturating_add(1);
			}
			if tabs < end && b[tabs] == 32 {
				return 2;
			}
			let ntabs = tabs.saturating_sub(start);
			match seen {
				false => {
					anchor = ntabs;
					seen = true;
					let mut k = tabs;
					let mut d: usize = 0;
					let mut cell = tabs;
					while k <= end {
						if k == end || b[k] == 9 {
							if k == cell {
								return 5;
							}
							d = d.saturating_add(1);
							cell = k.saturating_add(1);
						}
						k = k.saturating_add(1);
					}
					prev = match super::FIRST_LINE_SIBLINGS {
						true => 0,
						false => d.saturating_sub(1),
					};
				}
				true => {
					if ntabs < anchor {
						return 3;
					}
					let depth = ntabs.saturating_sub(anchor);
					if depth > prev.saturating_add(1) {
						return 4;
					}
					prev = depth;
				}
			}
		}
		match seen {
			false => return 1,
			true => return 0,
		}
	}

	#[inline]
	pub fn doc(p: &mut P, text: &str) {
		let (roots, last) = parse_doc(text);
		if roots.is_empty() {
			hold(p);
			return;
		}
		if roots.len() == 1 && roots[0].children.is_empty() {
			seg(p, &roots[0].name);
			return;
		}
		ogdl.with(|root| {
			let mut next = Vec::new();
			let mut tops = Vec::new();
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				let base = n.children.len();
				n.children.extend(roots.iter().cloned());
				for r in 0..roots.len() {
					let mut t = path.clone();
					t.push(base.saturating_add(r));
					tops.push(t);
				}
				let mut q = path.clone();
				q.push(base.saturating_add(last[0]));
				q.extend(&last[1..]);
				next.push(q);
			}
			p.cur = next;
			if p.anchor.is_none() {
				p.anchor = Some(tops);
			}
		});
	}

	#[inline]
	pub fn idx(p: &mut P, k: usize) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = at(&*root, path);
				if k < n.children.len() {
					let mut q = path.clone();
					q.push(k);
					next.push(q);
				}
			}
			p.cur = next;
		});
		hold(p);
	}

	#[inline]
	pub fn sel(p: &mut P, name: &str, k: usize) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = at(&*root, path);
				let hit = (0..n.children.len())
					.filter(|&i| return n.children[i].name == name)
					.nth(k);
				if let Some(i) = hit {
					let mut q = path.clone();
					q.push(i);
					next.push(q);
				}
			}
			p.cur = next;
		});
		hold(p);
	}

	#[inline]
	pub fn star(p: &mut P, name: &str) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = at(&*root, path);
				for i in
					(0..n.children.len()).filter(|&i| return n.children[i].name == name)
				{
					for j in 0..n.children[i].children.len() {
						let mut q = path.clone();
						q.push(i);
						q.push(j);
						next.push(q);
					}
				}
			}
			p.cur = next;
		});
		hold(p);
	}

	#[inline]
	pub fn val(p: &mut P, v: impl fmt::Display) {
		seg(p, &v.to_string());
	}

	#[inline]
	pub fn del(p: &mut P, name: &str) {
		ogdl.with(|root| {
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				n.children.retain(|c| return c.name != name);
			}
		});
		hold(p);
	}

	#[inline]
	pub fn fin(p: P) -> super::Block {
		let paths = p.anchor.unwrap_or_default();
		let sel = p.cur.first().cloned();
		return ogdl.with(|root| {
			return super::Block {
				nodes: paths.iter().map(|q| return at(&*root, q).clone()).collect(),
				sel,
			};
		});
	}
}

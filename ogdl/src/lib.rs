use std::fmt;
use std::fs;
use std::sync::{Arc, LazyLock, Mutex};

const NL: char = '\u{a}';

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
	pub name: String,
	pub children: Vec<Node>,
}

impl Node {
	pub(crate) fn leaf(name: &str) -> Node {
		Node {
			name: name.to_string(),
			children: Vec::new(),
		}
	}

	pub fn parse(text: &str) -> Node {
		let mut root = Node::leaf("");
		let mut path: Vec<usize> = Vec::new();
		let mut widths: Vec<usize> = Vec::new();
		for raw in text.lines() {
			let content = raw.trim();
			let Some(_c) = content.chars().next() else {
				continue;
			};
			let width = raw.len() - raw.trim_start().len();
			while widths.last().is_some_and(|&w| w >= width) {
				widths.pop();
				path.pop();
			}
			let line = content.replacen('=', " ", 1);
			let mut toks = line.split_whitespace();
			let name = toks.next().unwrap_or("");
			let mut node = Node::leaf(name);
			for v in toks {
				node.children.push(Node::leaf(v));
			}
			let parent = Node::at_mut(&mut root, &path);
			parent.children.push(node);
			path.push(parent.children.len() - 1);
			widths.push(width);
		}
		root
	}

	fn at_mut<'a>(mut n: &'a mut Node, path: &[usize]) -> &'a mut Node {
		for &i in path {
			n = &mut n.children[i];
		}
		n
	}

	pub fn select(&self, path: &str) -> Option<&Node> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| !s.is_empty()) {
			cur = cur.step(seg)?;
		}
		Some(cur)
	}

	fn step(&self, seg: &str) -> Option<&Node> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				self.children.get(i.wrapping_sub(1))
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = &seg[..brace];
					let sel = seg[brace + 1..].trim_end_matches('}');
					let nth: usize = match sel.chars().next() {
						Some(_c) => sel.parse().ok()?,
						None => 1,
					};
					self.children
						.iter()
						.filter(|c| c.name == name)
						.nth(nth.wrapping_sub(1))
				}
				None => self.children.iter().find(|c| c.name == seg),
			},
		}
	}

	pub(crate) fn select_mut(&mut self, path: &str) -> Option<&mut Node> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| !s.is_empty()) {
			let idx = cur.step_index(seg)?;
			cur = &mut cur.children[idx];
		}
		Some(cur)
	}

	fn step_index(&self, seg: &str) -> Option<usize> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				i.checked_sub(1).filter(|&i| i < self.children.len())
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = &seg[..brace];
					let sel = seg[brace + 1..].trim_end_matches('}');
					let nth: usize = match sel.chars().next() {
						Some(_c) => sel.parse().ok()?,
						None => 1,
					};
					(0..self.children.len())
						.filter(|&i| self.children[i].name == name)
						.nth(nth.wrapping_sub(1))
				}
				None => self.children.iter().position(|c| c.name == seg),
			},
		}
	}

	fn write_to(&self, out: &mut String, depth: usize) {
		for c in &self.children {
			for _d in 0..depth {
				out.push_str("    ");
			}
			out.push_str(&c.name);
			let eligible = match c.children.iter().find(|g| !g.children.is_empty()) {
				Some(_nonleaf) => None,
				None => c.children.first(),
			};
			match eligible {
				Some(_first) => {
					for g in &c.children {
						out.push(' ');
						out.push_str(&g.name);
					}
					out.push(NL);
				}
				None => {
					out.push(NL);
					c.write_to(out, depth + 1);
				}
			}
		}
	}

	pub fn serialize(&self) -> String {
		let mut s = String::new();
		self.write_to(&mut s, 0);
		s
	}
}

impl std::ops::Index<usize> for Node {
	type Output = Node;
	fn index(&self, i: usize) -> &Node {
		&self.children[i]
	}
}

impl fmt::Display for Node {
	fn fmt<'z>(&self, f: &mut fmt::Formatter<'z>) -> fmt::Result {
		let vals: Vec<&str> = self
			.children
			.iter()
			.filter(|c| c.children.is_empty())
			.map(|c| c.name.as_str())
			.collect();
		f.write_str(&vals.join(" "))
	}
}

#[derive(Clone)]
pub struct Graph {
	pub root: Arc<Mutex<Node>>,
	pub shown: Arc<Mutex<usize>>,
}

impl Graph {
	pub fn empty() -> Graph {
		Graph {
			root: Arc::new(Mutex::new(Node::leaf(""))),
			shown: Arc::new(Mutex::new(0)),
		}
	}

	pub fn with<R>(&self, f: impl FnOnce(&mut Node) -> R) -> R {
		let mut n = self.root.lock().unwrap_or_else(|p| p.into_inner());
		f(&mut n)
	}
}

mod alias {
	use super::{Graph, LazyLock};
	pub static OGDL: LazyLock<Graph> = LazyLock::new(Graph::empty);
}
pub use alias::OGDL as ogdl;

pub struct Ogdl;

impl Ogdl {
	pub fn file(path: &str) -> Graph {
		Graph::read(path)
	}
}

pub fn file(path: &str) -> Graph {
	Graph::read(path)
}

pub fn text(src: &str) -> Graph {
	let g = Graph::empty();
	g.with(|root| *root = Node::parse(src));
	g
}

pub trait ItnlArg {
	type Out;
	fn apply(self, g: Graph) -> Self::Out;
}

impl ItnlArg for () {
	type Out = Graph;
	fn apply(self, g: Graph) -> Graph {
		g
	}
}

impl ItnlArg for &str {
	type Out = Node;
	fn apply(self, g: Graph) -> Node {
		g.with(|root| {
			let mut n = root.select(self).cloned().unwrap_or_default();
			n.name = self.to_string();
			n
		})
	}
}

impl ItnlArg for Graph {
	type Out = Graph;
	fn apply(self, g: Graph) -> Graph {
		let src = self.snapshot();
		g.with(|root| *root = src);
		g
	}
}

impl ItnlArg for Node {
	type Out = Graph;
	fn apply(self, g: Graph) -> Graph {
		g.with(|root| *root = self);
		g
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
	fn apply(self, g: Graph) {
		let name = self.name;
		let parent = self.parent;
		g.with(|root| match root.select_mut(&parent.name) {
			Some(p) => {
				p.children.retain(|c| c.name != name);
			}
			None => {
				root.children.retain(|c| c.name != name);
			}
		});
	}
}

impl DelArg for &Node {
	fn apply(self, g: Graph) {
		fn strip(n: &mut Node, target: &Node) {
			n.children.retain(|c| c != target);
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
            fn into_nodes(self) -> Vec<Node> { vec![Node::leaf(&self.to_string())] }
      } )*};
      (str: $($t:ty),*) => {$( impl Value for $t {
            fn into_nodes(self) -> Vec<Node> { vec![Node::leaf(self.as_ref())] }
      } )*};
      (floats: $($t:ty),*) => {$( impl Value for $t {
            fn into_nodes(self) -> Vec<Node> { self.iter().map(|x| Node::leaf(&x.to_string())).collect() }
      } )*};
      (strs: $($t:ty),*) => {$( impl Value for $t {
            fn into_nodes(self) -> Vec<Node> { self.iter().map(|s| Node::leaf(s)).collect() }
      } )*};
}
value!(scalar: f64, i64, i32, usize, bool);
value!(str: &str, String, &String);
value!(floats: Vec<f64>, &[f64]);
value!(strs: Vec<String>, &[String]);

fn ensure_path<'a>(root: &'a mut Node, path: &str) -> &'a mut Node {
	let mut cur = root;
	for seg in path.split('.').filter(|s| !s.is_empty()) {
		let idx = match cur.children.iter().position(|c| c.name == seg) {
			Some(i) => i,
			None => {
				cur.children.push(Node::leaf(seg));
				cur.children.len() - 1
			}
		};
		cur = &mut cur.children[idx];
	}
	cur
}

impl Graph {
	fn read(path: &str) -> Graph {
		let g = Graph::empty();
		let text = fs::read_to_string(path).unwrap_or_default();
		g.with(|root| *root = Node::parse(&text));
		g
	}

	pub fn snapshot(&self) -> Node {
		self.with(|root| root.clone())
	}

	pub fn section(&self, path: &str) -> String {
		self.with(|root| {
			let depth = path
				.split('.')
				.filter(|s| !s.is_empty())
				.count()
				.saturating_sub(1);
			match root.select(path) {
				None => String::new(),
				Some(n) => {
					let wrap = Node {
						name: String::new(),
						children: vec![n.clone()],
					};
					let mut s = String::new();
					wrap.write_to(&mut s, depth);
					s
				}
			}
		})
	}

	pub fn itnl<A: ItnlArg>(&self, a: A) -> A::Out {
		a.apply(self.clone())
	}

	pub fn file(&self, path: &str) -> Graph {
		let head = self.with(|root| root.children.first().map(|_c| ()));
		match head {
			None => {
				let text = fs::read_to_string(path).unwrap_or_default();
				self.with(|root| *root = Node::parse(&text));
			}
			Some(_head) => {
				let text = self.with(|root| root.serialize());
				fs::write(path, text).ok();
			}
		}
		self.clone()
	}

	pub fn add<V: Value>(&self, value: V, path: &str) -> Graph {
		let kids = value.into_nodes();
		self.with(|root| ensure_path(root, path).children.extend(kids));
		self.clone()
	}

	pub fn del<D: DelArg>(&self, d: D) -> Graph {
		d.apply(self.clone());
		self.clone()
	}
}

pub struct Block(pub Node);

impl fmt::Display for Block {
	fn fmt<'z>(&self, f: &mut fmt::Formatter<'z>) -> fmt::Result {
		let mut s = String::new();
		match self.0.name.is_empty() {
			true => self.0.write_to(&mut s, 0),
			false => {
				let wrap = Node {
					name: String::new(),
					children: vec![self.0.clone()],
				};
				wrap.write_to(&mut s, 0);
			}
		}
		f.write_str(s.trim_end())
	}
}

fn parse_doc(text: &str) -> (Vec<Node>, Vec<usize>) {
	let all: Vec<&str> = text.split(NL).collect();
	let Some(first) = all.iter().position(|l| !l.trim().is_empty()) else {
		return (Vec::new(), Vec::new());
	};
	let last = all
		.iter()
		.rposition(|l| !l.trim().is_empty())
		.unwrap_or(first);
	let lines = &all[first..=last];
	let anchor = lines[0].chars().take_while(|&c| c == '\t').count();
	let mut root = Node::leaf("");
	let mut stack: Vec<Vec<usize>> = Vec::new();
	let mut lastpath: Vec<usize> = Vec::new();
	let mut past_first_newline = false;
	for raw in lines {
		if raw.trim().is_empty() {
			continue;
		}
		let tabs = raw.chars().take_while(|&c| c == '\t').count();
		let rest = &raw[tabs..];
		match past_first_newline {
			false => {
				for (d, name) in rest.split('\t').enumerate() {
					let parent = match d {
						0 => Vec::new(),
						_ => stack[d - 1].clone(),
					};
					let n = Node::at_mut(&mut root, &parent);
					n.children.push(Node::leaf(name));
					let mut path = parent;
					path.push(n.children.len() - 1);
					stack.truncate(d);
					stack.push(path.clone());
					lastpath = path;
				}
				past_first_newline = true;
			}
			true => {
				let depth = tabs.saturating_sub(anchor).min(stack.len());
				let parent = match depth {
					0 => Vec::new(),
					_ => stack[depth - 1].clone(),
				};
				let n = Node::at_mut(&mut root, &parent);
				n.children.push(Node::leaf(rest));
				let mut path = parent;
				path.push(n.children.len() - 1);
				stack.truncate(depth);
				stack.push(path.clone());
				lastpath = path;
			}
		}
	}
	(root.children, lastpath)
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
	use super::{Graph, Node, ogdl, parse_doc};

	pub fn del_all(g: &Graph, parent: &str, name: &str) -> Graph {
		g.with(|root: &mut Node| {
			let target = match parent.chars().next() {
				None => Some(&mut *root),
				Some(_c) => root.select_mut(parent),
			};
			let Some(p) = target else { return };
			p.children.retain(|c| c.name != name);
		});
		g.clone()
	}

	pub struct P {
		pub cur: Vec<Vec<usize>>,
		pub anchor: std::option::Option<Vec<usize>>,
	}

	pub fn start() -> P {
		P {
			cur: vec![Vec::new()],
			anchor: None,
		}
	}

	fn hold(p: &mut P) {
		if p.anchor.is_none() {
			p.anchor = p.cur.first().cloned();
		}
	}

	fn at<'a>(mut n: &'a Node, path: &[usize]) -> &'a Node {
		for &i in path {
			n = &n.children[i];
		}
		n
	}

	pub fn seg(p: &mut P, name: &str) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				let i = match n.children.iter().position(|c| c.name == name) {
					Some(i) => i,
					None => {
						n.children.push(Node::leaf(name));
						n.children.len() - 1
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

	pub const fn doc_ok(s: &str) -> usize {
		let b = s.as_bytes();
		let mut i = 0;
		let mut anchor = 0;
		let mut prev = 0;
		let mut seen = false;
		while i < b.len() {
			let start = i;
			while i < b.len() && b[i] != 10 {
				i += 1;
			}
			let end = i;
			i += 1;
			let mut j = start;
			while j < end && (b[j] == 9 || b[j] == 32) {
				j += 1;
			}
			if j == end {
				continue;
			}
			let mut tabs = start;
			while tabs < end && b[tabs] == 9 {
				tabs += 1;
			}
			if tabs < end && b[tabs] == 32 {
				return 2;
			}
			let ntabs = tabs - start;
			match seen {
				false => {
					anchor = ntabs;
					seen = true;
					let mut k = tabs;
					let mut d = 0;
					let mut cell = tabs;
					while k <= end {
						if k == end || b[k] == 9 {
							if k == cell {
								return 5;
							}
							d += 1;
							cell = k + 1;
						}
						k += 1;
					}
					prev = d - 1;
				}
				true => {
					if ntabs < anchor {
						return 3;
					}
					let depth = ntabs - anchor;
					if depth > prev + 1 {
						return 4;
					}
					prev = depth;
				}
			}
		}
		match seen {
			false => 1,
			true => 0,
		}
	}

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
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				let base = n.children.len();
				n.children.extend(roots.iter().cloned());
				let mut q = path.clone();
				q.push(base + last[0]);
				q.extend(&last[1..]);
				next.push(q);
			}
			p.cur = next;
		});
		hold(p);
	}

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

	pub fn sel(p: &mut P, name: &str, k: usize) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = at(&*root, path);
				let hit = (0..n.children.len())
					.filter(|&i| n.children[i].name == name)
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

	pub fn star(p: &mut P, name: &str) {
		ogdl.with(|root| {
			let mut next = Vec::new();
			for path in &p.cur {
				let n = at(&*root, path);
				for i in (0..n.children.len()).filter(|&i| n.children[i].name == name) {
					let mut q = path.clone();
					q.push(i);
					next.push(q);
				}
			}
			p.cur = next;
		});
		hold(p);
	}

	pub fn del(p: &mut P, name: &str) {
		ogdl.with(|root| {
			for path in &p.cur {
				let n = Node::at_mut(root, path);
				n.children.retain(|c| c.name != name);
			}
		});
		hold(p);
	}

	pub fn fin(p: P) -> super::Block {
		let path = p.anchor.unwrap_or_default();
		ogdl.with(|root| super::Block(at(&*root, &path).clone()))
	}
}

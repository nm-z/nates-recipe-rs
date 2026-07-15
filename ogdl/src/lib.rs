extern crate alloc;

#[doc(hidden)]
pub mod __macro_support;
/// Houses the process-wide default graph behind a stable path.
mod alias;

use alloc::fmt;
use alloc::sync::Arc;
use core::ops;
use std::fs;
use std::io;
use std::sync::{LazyLock, Mutex};

pub use alias::OGDL as ogdl;

/// Generates [`Value`] implementations that flatten a type into leaf nodes.
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

/// When true, the first content line's tab-separated fields become sibling roots rather than a nested chain.
const FIRST_LINE_SIBLINGS: bool = true;

/// Newline character used as the OGDL line separator.
const NL: char = '\u{a}';

#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Node {
	pub children: Vec<Self>,
	pub name: String,
}

impl Node {
	/// Descends `path` of child indices from `n`, returning the mutable node reached.
	fn at_mut<'a>(mut n: &'a mut Self, path: &[usize]) -> &'a mut Self {
		for &i in path {
			n = &mut n.children[i];
		}
		return n;
	}

	/// Builds a childless node holding `name`.
	#[inline]
	#[must_use]
	pub fn leaf(name: &str) -> Self {
		return Self {
			name: name.to_owned(),
			children: Vec::new(),
		};
	}

	#[inline]
	#[must_use]
	pub const fn new(name: String, children: Vec<Self>) -> Self {
		return Self { children, name };
	}

	#[inline]
	#[must_use]
	pub fn parse(text: &str) -> Self {
		let mut root = Self::leaf("");
		let mut stack: Vec<Vec<usize>> = Vec::new();
		for raw in text.lines() {
			if raw.trim().is_empty() {
				continue;
			}
			let tabs = raw.chars().take_while(|&c| return c == '\t').count();
			let depth = tabs.min(stack.len());
			let mut toks = raw.get(tabs..).unwrap_or("").split('\t');
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

	#[inline]
	#[must_use]
	pub fn select(&self, path: &str) -> Option<&Self> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| return !s.is_empty()) {
			cur = cur.step(seg)?;
		}
		return Some(cur);
	}

	/// Like [`select`](Self::select) but walks and returns a mutable reference along the path.
	pub(crate) fn select_mut(&mut self, path: &str) -> Option<&mut Self> {
		let mut cur = self;
		for seg in path.split('.').filter(|s| return !s.is_empty()) {
			let idx = cur.step_index(seg)?;
			cur = &mut cur.children[idx];
		}
		return Some(cur);
	}

	#[inline]
	#[must_use]
	pub fn serialize(&self) -> String {
		let mut s = String::new();
		self.write_to(&mut s, 0);
		return s;
	}

	/// Resolves one dotted-path segment against direct children, honoring `[n]` index and `{name}` selector syntax.
	fn step(&self, seg: &str) -> Option<&Self> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				return self.children.get(i.wrapping_sub(1));
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = seg.get(..brace).unwrap_or("");
					let sel = seg
						.get(brace.saturating_add(1)..)
						.unwrap_or("")
						.trim_end_matches('}');
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

	/// Returns the child index a single segment resolves to, or `None`.
	fn step_index(&self, seg: &str) -> Option<usize> {
		match seg.strip_prefix('[') {
			Some(rest) => {
				let i: usize = rest.trim_end_matches(']').parse().ok()?;
				return i.checked_sub(1).filter(|&j| return j < self.children.len());
			}
			None => match seg.find('{') {
				Some(brace) => {
					let name = seg.get(..brace).unwrap_or("");
					let sel = seg
						.get(brace.saturating_add(1)..)
						.unwrap_or("")
						.trim_end_matches('}');
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

	/// Appends this node's subtree as tab-indented lines to `out`, starting at `depth`.
	fn write_to(&self, out: &mut String, depth: usize) {
		let mut lines = Vec::new();
		walk_lines(self, &mut Vec::new(), depth, &mut lines);
		for (l, _p, _r) in lines {
			out.push_str(&l);
			out.push(NL);
		}
	}
}

impl ops::Index<usize> for Node {
	type Output = Self;
	#[inline]
	fn index(&self, index: usize) -> &Self {
		return &self.children[index];
	}
}

impl fmt::Display for Node {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[non_exhaustive]
pub struct Graph {
	pub root: Arc<Mutex<Node>>,
	pub shown: Arc<Mutex<usize>>,
}

impl Graph {
	#[inline]
	#[must_use]
	pub fn add<V: Value>(&self, value: V, path: &str) -> Self {
		let kids = value.into_nodes();
		self.with(|root| {
			let mut cur = root;
			for seg in path.split('.').filter(|s| return !s.is_empty()) {
				let idx = if let Some(hit) =
					cur.children.iter().position(|c| return c.name == seg)
				{
					hit
				} else {
					cur.children.push(Node::leaf(seg));
					cur.children.len().saturating_sub(1)
				};
				cur = &mut cur.children[idx];
			}
			cur.children.extend(kids);
		});
		return self.clone();
	}

	#[inline]
	#[must_use]
	pub fn del<D: DelArg>(&self, d: D) -> Self {
		d.apply(self.clone());
		return self.clone();
	}

	#[inline]
	#[must_use]
	pub fn empty() -> Self {
		return Self {
			root: Arc::new(Mutex::new(Node::leaf(""))),
			shown: Arc::new(Mutex::new(0)),
		};
	}

	#[inline]
	#[must_use]
	pub fn file(&self, path: &str) -> Self {
		let head = self.with(|root| return root.children.first().map(|_c| ()));
		match head {
			None => {
				let text = fs::read_to_string(path).unwrap_or_default();
				self.with(|root| *root = Node::parse(&text));
			}
			Some(_head) => {
				let text = self.with(|root| return root.serialize());
				if let Err(e) = fs::write(path, text) {
					use std::io::Write as _;
					drop(io::stderr().write_all(format!("{e}\n").as_bytes()));
				}
			}
		}
		return self.clone();
	}

	#[inline]
	pub fn itnl<A: ItnlArg>(&self, a: A) -> A::Out {
		return a.apply(self.clone());
	}

	/// Mutates the tree while erasing any already-printed terminal lines the mutation invalidates.
	/// Captures the serialization before and after `f`, then reconciles the shown cursor so deletions
	/// clear stale terminal output on a TTY and clamp the append-only cursor on a pipe.
	#[inline]
	pub(crate) fn mutate_shown<R>(&self, f: impl FnOnce(&mut Node) -> R) -> R {
		let mut n = self.root.lock().unwrap_or_else(|p| return p.into_inner());
		let before: Vec<String> = lines_of(&n)
			.into_iter()
			.map(|(l, _p, _r)| return l)
			.collect();
		let r = f(&mut n);
		let after: Vec<String> = lines_of(&n)
			.into_iter()
			.map(|(l, _p, _r)| return l)
			.collect();
		drop(n);
		let mut sh = self.shown.lock().unwrap_or_else(|p| return p.into_inner());
		reconcile(&before, &after, &mut sh);
		drop(sh);
		return r;
	}

	/// Loads a graph from the OGDL file at `path`, empty when the read fails.
	fn read(path: &str) -> Self {
		let g = Self::empty();
		let text = fs::read_to_string(path).unwrap_or_default();
		g.with(|root| *root = Node::parse(&text));
		return g;
	}

	#[inline]
	#[must_use]
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
	#[must_use]
	pub fn snapshot(&self) -> Node {
		return self.with(|root| return root.clone());
	}

	#[inline]
	pub fn with<R>(&self, f: impl FnOnce(&mut Node) -> R) -> R {
		let mut n = self.root.lock().unwrap_or_else(|p| return p.into_inner());
		return f(&mut n);
	}
}

#[non_exhaustive]
pub struct Ogdl;

impl Ogdl {
	#[inline]
	#[must_use]
	pub fn file(path: &str) -> Graph {
		return Graph::read(path);
	}
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
			self.clone_into(&mut n.name);
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

#[non_exhaustive]
pub struct NamedChild<'a> {
	pub name: &'a str,
	pub parent: &'a Node,
}

impl<'a> NamedChild<'a> {
	#[inline]
	#[must_use]
	pub const fn new(name: &'a str, parent: &'a Node) -> Self {
		return Self { name, parent };
	}
}

impl DelArg for NamedChild<'_> {
	#[inline]
	fn apply(self, g: Graph) {
		let name = self.name;
		let parent = self.parent;
		g.mutate_shown(|root| match root.select_mut(&parent.name) {
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
		g.mutate_shown(|root| return strip(root, self));
	}
}

pub trait Value {
	fn into_nodes(self) -> Vec<Node>;
}

value!(scalar: f64, i64, i32, usize, bool);
value!(str: &str, String, &String);
value!(floats: Vec<f64>, &[f64]);
value!(strs: Vec<String>, &[String]);

#[non_exhaustive]
pub struct Block {
	pub nodes: Vec<Node>,
	pub sel: Option<Vec<usize>>,
}

impl fmt::Display for Block {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
			let hit = ls.iter().position(|row| return row.1 == sel).or_else(|| {
				match sel.split_last() {
					Some((_i, par)) => {
						return ls
							.iter()
							.position(|row| return row.1.as_slice() == par);
					}
					None => return None,
				}
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
				.map(|row| return row.0.as_str())
				.collect();
			*sh = end.saturating_add(1);
			drop(sh);
			return out.join("\n");
		});
	}
}

#[inline]
#[must_use]
pub fn file(path: &str) -> Graph {
	return Graph::read(path);
}

#[inline]
#[must_use]
pub fn text(src: &str) -> Graph {
	let g = Graph::empty();
	g.with(|root| *root = Node::parse(src));
	return g;
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
		if !c.children.is_empty()
			&& c.children.iter().all(|leaf| {
				return leaf.children.is_empty()
					&& !leaf.name.contains('\t')
					&& (leaf.name.parse::<f64>().is_ok()
						|| leaf.name.chars().any(|ch| return !ch.is_ascii()));
			}) {
			for v in &c.children {
				line.push('\t');
				line.push_str(&v.name);
			}
			out.push((line, path.clone(), true));
		} else {
			out.push((line, path.clone(), false));
			walk_lines(c, path, depth.saturating_add(1), out);
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

/// Returns the first line index at which the pre-mutation lines `before` and post-mutation lines `after` differ.
/// Equal when one is a prefix of the other, in which case the shorter length is returned.
#[doc(hidden)]
#[inline]
#[must_use]
pub fn divergence(before: &[String], after: &[String]) -> usize {
	let mut i: usize = 0;
	while before.get(i).is_some() && before.get(i) == after.get(i) {
		i = i.saturating_add(1);
	}
	return i;
}

/// Reconciles already-printed terminal lines against a mutation that changed the tree.
/// On a TTY, moves the cursor up over the invalidated shown lines, clears to end of screen, reprints the
/// survivors from the divergence point, and sets `shown` to the reprinted count. On a pipe, printed history
/// is append-only, so it only clamps `shown` to the new tree length to keep later `Write::block` deltas valid.
#[doc(hidden)]
#[inline]
pub fn reconcile(before: &[String], after: &[String], shown: &mut usize) {
	use std::io::IsTerminal as _;
	use std::io::Write as _;
	let sh = *shown;
	if sh == 0 {
		return;
	}
	let div = divergence(before, after);
	if div >= sh {
		*shown = sh.min(after.len());
		return;
	}
	if !io::stderr().is_terminal() {
		*shown = sh.min(after.len());
		return;
	}
	let removed = before.len().saturating_sub(after.len());
	let survive = sh.saturating_sub(removed).max(div).min(after.len());
	let up = sh.saturating_sub(div);
	let mut out = format!("\u{1b}[{up}A\u{1b}[0J");
	for row in after.get(div..survive).unwrap_or(&[]) {
		out.push_str(row);
		out.push('\n');
	}
	drop(write!(io::stderr(), "{out}"));
	*shown = survive;
}

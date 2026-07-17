//! Glob selection over a [`Node`] tree: two spatial axes plus one-step
//! lineage, composed left to right.
//!
//! The grid is 0-indexed everywhere: rows run top to bottom in document
//! order (`[n]`, one node per row), levels run parents to children
//! (`(m)`, a set per level). Atoms:
//!
//! - `name`            every node with that name
//! - `[n]`             the node at row n
//! - `(m)`             every node at level m
//! - `*[n]` / `[n]*`   rows below / rows above n, exclusive
//! - `*(m)` / `(m)*`   levels left / levels right of m, exclusive
//! - `*x` / `x*`       parents / children of every x match, exclusive
//!
//! Juxtaposition inside a segment intersects (`[0](0)`, `*[0][5]*` is the
//! strictly-between interval); `.` between segments collects the union.
//! Display is the matched lines at their native indent in document order.
//! Empty positions select nothing: a leaf's `x*`, `*[n]` past the last
//! row, an out-of-range coordinate are all the empty set, never an error.
//! A trailing `{}` deletes every matched branch ([`Node::glob_del`]).

use crate::Node;
use std::collections::BTreeSet;

enum Atom {
	All,
	Name(String),
	NameUp(String),
	NameDown(String),
	Row(usize),
	RowBelow(usize),
	RowAbove(usize),
	Lvl(usize),
	LvlLeft(usize),
	LvlRight(usize),
}

enum Core {
	Name(String),
	Row(usize),
	Lvl(usize),
}

struct Grid<'n> {
	name: Vec<&'n str>,
	depth: Vec<usize>,
	parent: Vec<Option<usize>>,
	path: Vec<Vec<usize>>,
}

fn flatten<'n>(root: &'n Node) -> Grid<'n> {
	let mut g = Grid {
		name: Vec::new(),
		depth: Vec::new(),
		parent: Vec::new(),
		path: Vec::new(),
	};
	fn rec<'n>(n: &'n Node, depth: usize, parent: Option<usize>, path: &mut Vec<usize>, g: &mut Grid<'n>) {
		for (i, c) in n.children.iter().enumerate() {
			path.push(i);
			let row = g.name.len();
			g.name.push(c.name.as_str());
			g.depth.push(depth);
			g.parent.push(parent);
			g.path.push(path.clone());
			rec(c, depth.saturating_add(1), Some(row), path, g);
			path.pop();
		}
	}
	rec(root, 0, None, &mut Vec::new(), &mut g);
	return g;
}

fn coord(body: &str, seg: &str) -> Result<usize, String> {
	return body
		.parse()
		.map_err(|_e| format!("glob: bad coordinate {body:?} in segment {seg:?}"));
}

fn parse_segment(seg: &str) -> Result<Vec<Atom>, String> {
	if seg == "*" {
		return Ok(vec![Atom::All]);
	}
	let mut atoms: Vec<Atom> = Vec::new();
	let mut prefix = false;
	let mut suffixable = false;
	let bytes = seg.as_bytes();
	let mut i = 0usize;
	while i < bytes.len() {
		let core: Core = match bytes[i] {
			b'*' => {
				if suffixable {
					let last = atoms.pop().ok_or_else(|| format!("glob: dangling * in {seg:?}"))?;
					atoms.push(match last {
						Atom::Name(n) => Atom::NameDown(n),
						Atom::Row(n) => Atom::RowAbove(n),
						Atom::Lvl(n) => Atom::LvlRight(n),
						_other => return Err(format!("glob: star on starred atom in {seg:?}")),
					});
					suffixable = false;
				} else if prefix {
					return Err(format!("glob: ** without a coordinate between in {seg:?}"));
				} else {
					prefix = true;
				}
				i = i.saturating_add(1);
				continue;
			}
			b'[' | b'(' => {
				let (close, mk): (u8, fn(usize) -> Core) = if bytes[i] == b'[' {
					(b']', Core::Row)
				} else {
					(b')', Core::Lvl)
				};
				let end = seg[i..]
					.find(close as char)
					.map(|p| return i.saturating_add(p))
					.ok_or_else(|| format!("glob: unclosed bracket in {seg:?}"))?;
				let n = coord(&seg[i.saturating_add(1)..end], seg)?;
				i = end.saturating_add(1);
				mk(n)
			}
			_other => {
				let start = i;
				while i < bytes.len() && !b"*[](){}.".contains(&bytes[i]) {
					i = i.saturating_add(1);
				}
				if start == i {
					return Err(format!("glob: unexpected {:?} in {seg:?}", bytes[i] as char));
				}
				Core::Name(seg[start..i].to_owned())
			}
		};
		let atom = match (prefix, core) {
			(false, Core::Name(n)) => Atom::Name(n),
			(true, Core::Name(n)) => Atom::NameUp(n),
			(false, Core::Row(n)) => Atom::Row(n),
			(true, Core::Row(n)) => Atom::RowBelow(n),
			(false, Core::Lvl(n)) => Atom::Lvl(n),
			(true, Core::Lvl(n)) => Atom::LvlLeft(n),
		};
		suffixable = !prefix;
		prefix = false;
		atoms.push(atom);
	}
	if prefix {
		return Err(format!("glob: trailing * with no coordinate in {seg:?}"));
	}
	if atoms.is_empty() {
		return Err(format!("glob: empty segment in {seg:?}"));
	}
	return Ok(atoms);
}

fn eval_atom(a: &Atom, g: &Grid) -> BTreeSet<usize> {
	let rows = 0..g.name.len();
	return match a {
		Atom::All => rows.collect(),
		Atom::Name(n) => rows.filter(|&r| return g.name[r] == n).collect(),
		Atom::NameUp(n) => rows
			.filter(|&r| return g.name[r] == n)
			.filter_map(|r| return g.parent[r])
			.collect(),
		Atom::NameDown(n) => rows
			.filter(|&r| return g.parent[r].is_some_and(|p| return g.name[p] == n))
			.collect(),
		Atom::Row(n) => rows.filter(|&r| return r == *n).collect(),
		Atom::RowBelow(n) => rows.filter(|&r| return r > *n).collect(),
		Atom::RowAbove(n) => rows.filter(|&r| return r < *n).collect(),
		Atom::Lvl(m) => rows.filter(|&r| return g.depth[r] == *m).collect(),
		Atom::LvlLeft(m) => rows.filter(|&r| return g.depth[r] < *m).collect(),
		Atom::LvlRight(m) => rows.filter(|&r| return g.depth[r] > *m).collect(),
	};
}

fn eval_rows(g: &Grid, expr: &str) -> Result<BTreeSet<usize>, String> {
	let mut out = BTreeSet::new();
	for seg in expr.split('.') {
		let atoms = parse_segment(seg)?;
		let mut it = atoms.iter().map(|a| return eval_atom(a, g));
		let first = it.next().unwrap_or_default();
		let seg_rows = it.fold(first, |acc, s| return acc.intersection(&s).copied().collect());
		out.extend(seg_rows);
	}
	return Ok(out);
}

impl Node {
	/// Every node matched by `expr`, in document order.
	pub fn glob(&self, expr: &str) -> Result<Vec<&Self>, String> {
		let g = flatten(self);
		let rows = eval_rows(&g, expr)?;
		return Ok(rows
			.into_iter()
			.map(|r| {
				let mut n = self;
				for &i in &g.path[r] {
					n = &n.children[i];
				}
				return n;
			})
			.collect());
	}

	/// The matched lines at their native indent, document order, one per row.
	pub fn glob_show(&self, expr: &str) -> Result<String, String> {
		let g = flatten(self);
		let rows = eval_rows(&g, expr)?;
		return Ok(rows
			.into_iter()
			.map(|r| return format!("{}{}", "\t".repeat(g.depth[r]), g.name[r]))
			.collect::<Vec<_>>()
			.join("\n"));
	}

	/// Deletes every branch matched by `expr` (a trailing `{}` is accepted
	/// and stripped; bare `{}` or `*{}` wipes the whole tree).
	pub fn glob_del(&mut self, expr: &str) -> Result<usize, String> {
		let body = expr.strip_suffix("{}").unwrap_or(expr);
		if body.is_empty() || body == "*" {
			let n = self.children.len();
			self.children.clear();
			return Ok(n);
		}
		let paths: Vec<Vec<usize>> = {
			let g = flatten(self);
			let rows = eval_rows(&g, body)?;
			rows.into_iter().rev().map(|r| return g.path[r].clone()).collect()
		};
		let n = paths.len();
		for p in paths {
			let Some((&last, up)) = p.split_last() else {
				continue;
			};
			let mut cur = &mut *self;
			for &i in up {
				cur = &mut cur.children[i];
			}
			if last < cur.children.len() {
				cur.children.remove(last);
			}
		}
		return Ok(n);
	}
}

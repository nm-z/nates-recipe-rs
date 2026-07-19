//! Glob selection over a [`Node`] tree: two spatial axes plus one-step
//! lineage, composed left to right.
//!
//! The grid is 0-indexed everywhere: rows run top to bottom in document
//! order (`[n]`, one node per row), levels run parents to children
//! (`(m)`, a set per level). Atoms:
//!
//! Star-before looks toward an axis's minus side, star-after toward plus,
//! uniformly: left|right for `()`, up|down for `[]`, upstm|dnstm for names.
//!
//! - `name`            every node with that name
//! - `[n]`             the node at row n
//! - `(m)`             every node at level m
//! - `*[n]` / `[n]*`   rows above / rows below n, exclusive
//! - `*(m)` / `(m)*`   levels left / levels right of m, exclusive
//! - `*x` / `x*`       x with its parents / x with its children; inside a
//!   route the anchor is the viewpoint and drops out, leaving one step
//!
//! Juxtaposition inside a segment intersects; a single star between two
//! coordinates serves both sides (`[0]*(1)` is `[0]*` and `*(1)`), and an
//! empty operand drops out as identity instead of annihilating. Across
//! dots, a plus-view segment followed by a minus-view segment on the same
//! axis is a bounds pair selecting the strictly-between interval
//! (`(0)*.*(3)`, `[0]*.*[5]`); every other segment collects (union).
//! Matches come back in walk order away from the anchor ([`Node::glob`]);
//! [`Node::glob_show`] prints matched lines at native indent in document
//! order. Empty positions select nothing: a leaf's `x*`, `*[0]` above the
//! first row, an out-of-range coordinate are the empty set, never an error.

use crate::Node;
use std::collections::BTreeSet;

enum Atom {
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
						Atom::Row(n) => Atom::RowBelow(n),
						Atom::Lvl(n) => Atom::LvlRight(n),
						_other => return Err(format!("glob: star on starred atom in {seg:?}")),
					});
					suffixable = false;
					let shared = bytes
						.get(i.saturating_add(1))
						.is_some_and(|&b| return b != b'*');
					if shared {
						prefix = true;
					}
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
			(true, Core::Row(n)) => Atom::RowAbove(n),
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

fn by_level(g: &Grid, want: impl Fn(usize) -> bool, outward: bool) -> Vec<usize> {
	let max = g.depth.iter().copied().max().unwrap_or(0);
	let mut levels: Vec<usize> = (0..=max).filter(|&m| return want(m)).collect();
	if outward {
		levels.reverse();
	}
	return levels
		.into_iter()
		.flat_map(|m| return (0..g.name.len()).filter(move |&r| return g.depth[r] == m))
		.collect();
}

fn eval_atom(a: &Atom, g: &Grid, route: bool) -> Vec<usize> {
	let all = || return 0..g.name.len();
	return match a {
		Atom::Name(n) => all().filter(|&r| return g.name[r] == n).collect(),
		Atom::NameUp(n) => all()
			.filter(|&r| return g.name[r] == n)
			.flat_map(|r| {
				let mut v = Vec::new();
				if let Some(p) = g.parent[r] {
					if !route {
						v.push(r);
					}
					v.push(p);
				}
				return v;
			})
			.collect(),
		Atom::NameDown(n) => all()
			.filter(|&r| return g.name[r] == n)
			.flat_map(|r| {
				let kids: Vec<usize> =
					all().filter(|&c| return g.parent[c] == Some(r)).collect();
				let mut v = Vec::new();
				if !kids.is_empty() {
					if !route {
						v.push(r);
					}
					v.extend(kids);
				}
				return v;
			})
			.collect(),
		Atom::Row(n) => all().filter(|&r| return r == *n).collect(),
		Atom::RowBelow(n) => all().filter(|&r| return r > *n).collect(),
		Atom::RowAbove(n) => (0..*n.min(&g.name.len())).rev().collect(),
		Atom::Lvl(m) => all().filter(|&r| return g.depth[r] == *m).collect(),
		Atom::LvlLeft(m) => by_level(g, |lv| return lv < *m, true),
		Atom::LvlRight(m) => by_level(g, |lv| return lv > *m, false),
	};
}

#[derive(Clone, Copy, PartialEq)]
enum Axis {
	Row,
	Lvl,
}

fn seg_view(atoms: &[Atom]) -> Option<(Axis, bool)> {
	let [a] = atoms else { return None };
	return match a {
		Atom::RowBelow(_n) => Some((Axis::Row, true)),
		Atom::RowAbove(_n) => Some((Axis::Row, false)),
		Atom::LvlRight(_m) => Some((Axis::Lvl, true)),
		Atom::LvlLeft(_m) => Some((Axis::Lvl, false)),
		_other => None,
	};
}

fn eval_segment(g: &Grid, seg: &str, route: bool) -> Result<(Vec<usize>, Option<(Axis, bool)>), String> {
	let atoms = parse_segment(seg)?;
	let view = seg_view(&atoms);
	let mut sets: Vec<Vec<usize>> = atoms
		.iter()
		.map(|a| return eval_atom(a, g, route))
		.filter(|v| return !v.is_empty())
		.collect();
	if sets.is_empty() {
		return Ok((Vec::new(), view));
	}
	let mut first = sets.remove(0);
	let rest: Vec<BTreeSet<usize>> = sets
		.into_iter()
		.map(|v| return v.into_iter().collect())
		.collect();
	if !rest.is_empty() {
		first.sort_unstable();
	}
	let rows = first
		.into_iter()
		.filter(|r| return rest.iter().all(|s| return s.contains(r)))
		.collect();
	return Ok((rows, view));
}

fn eval_rows(g: &Grid, expr: &str) -> Result<Vec<usize>, String> {
	let route = expr.contains('.');
	let mut out: Vec<usize> = Vec::new();
	let mut seen = BTreeSet::new();
	let flush = |rows: Vec<usize>, out: &mut Vec<usize>, seen: &mut BTreeSet<usize>| {
		for r in rows {
			if seen.insert(r) {
				out.push(r);
			}
		}
	};
	let mut pending: Option<(Vec<usize>, Option<(Axis, bool)>)> = None;
	for seg in expr.split('.') {
		let (rows, view) = eval_segment(g, seg, route)?;
		let bounds = match (&pending, view) {
			(Some((prows, Some((pa, true)))), Some((a, false))) => {
				*pa == a && !prows.is_empty() && !rows.is_empty()
			}
			_other => false,
		};
		if bounds {
			let (prows, _pv) = pending.take().unwrap_or_default();
			let inside: BTreeSet<usize> = rows.into_iter().collect();
			let mut mid: Vec<usize> =
				prows.into_iter().filter(|r| return inside.contains(r)).collect();
			mid.sort_unstable();
			flush(mid, &mut out, &mut seen);
		} else {
			if let Some((prows, _pv)) = pending.take() {
				flush(prows, &mut out, &mut seen);
			}
			pending = Some((rows, view));
		}
	}
	if let Some((prows, _pv)) = pending.take() {
		flush(prows, &mut out, &mut seen);
	}
	return Ok(out);
}

impl Node {
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

	pub fn glob_show(&self, expr: &str) -> Result<String, String> {
		let g = flatten(self);
		let mut rows = eval_rows(&g, expr)?;
		rows.sort_unstable();
		return Ok(rows
			.into_iter()
			.map(|r| return format!("{}{}", "\t".repeat(g.depth[r]), g.name[r]))
			.collect::<Vec<_>>()
			.join("\n"));
	}
}

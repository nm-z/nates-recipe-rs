use super::{Graph, Node, fmt, ogdl};

#[non_exhaustive]
pub struct P {
	pub anchor: Option<Vec<Vec<usize>>>,
	pub cur: Vec<Vec<usize>>,
}

fn at<'a>(mut n: &'a Node, path: &[usize]) -> &'a Node {
	for &i in path {
		n = &n.children[i];
	}
	return n;
}

fn hold(p: &mut P) {
	if p.anchor.is_none() {
		p.anchor = Some(p.cur.clone());
	}
}

#[inline]
#[must_use]
pub fn del_all(g: &Graph, parent: &str, name: &str) -> Graph {
	g.mutate_shown(|root: &mut Node| {
		let target = match parent.chars().next() {
			None => Some(&mut *root),
			Some(_c) => root.select_mut(parent),
		};
		let Some(p) = target else { return };
		p.children.retain(|c| return c.name != name);
	});
	return g.clone();
}

#[inline]
#[must_use]
pub fn start() -> P {
	return P {
		cur: vec![Vec::new()],
		anchor: None,
	};
}

#[inline]
pub fn seg(p: &mut P, name: &str) {
	ogdl.with(|root| {
		let mut next = Vec::new();
		for path in &p.cur {
			let n = Node::at_mut(root, path);
			let i = if let Some(hit) = n.children.iter().position(|c| return c.name == name) {
				hit
			} else {
				n.children.push(Node::leaf(name));
				n.children.len().saturating_sub(1)
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
#[must_use]
pub const fn invalid_path() -> usize {
	return 1;
}

#[inline]
#[must_use]
pub const fn doc_ok(src: &str) -> usize {
	let bytes = src.as_bytes();
	let mut pos = 0;
	let mut anchor = 0;
	let mut prev: usize = 0;
	let mut seen = false;
	while pos < bytes.len() {
		let start = pos;
		while pos < bytes.len() && bytes[pos] != 10 {
			pos = pos.saturating_add(1);
		}
		let end = pos;
		pos = pos.saturating_add(1);
		let mut ws = start;
		while ws < end && (bytes[ws] == 9 || bytes[ws] == 32) {
			ws = ws.saturating_add(1);
		}
		if ws == end {
			continue;
		}
		let mut tabs = start;
		while tabs < end && bytes[tabs] == 9 {
			tabs = tabs.saturating_add(1);
		}
		if tabs < end && bytes[tabs] == 32 {
			return 2;
		}
		let ntabs = tabs.saturating_sub(start);
		if seen {
			if ntabs < anchor {
				return 3;
			}
			let depth = ntabs.saturating_sub(anchor);
			if depth > prev.saturating_add(1) {
				return 4;
			}
			prev = depth;
		} else {
			anchor = ntabs;
			seen = true;
			let mut scan = tabs;
			let mut cells: usize = 0;
			let mut cell = tabs;
			while scan <= end {
				if scan == end || bytes[scan] == 9 {
					if scan == cell {
						return 5;
					}
					cells = cells.saturating_add(1);
					cell = scan.saturating_add(1);
				}
				scan = scan.saturating_add(1);
			}
			prev = if super::FIRST_LINE_SIBLINGS {
				0
			} else {
				cells.saturating_sub(1)
			};
		}
	}
	if seen {
		return 0;
	}
	return 1;
}

#[inline]
pub fn doc(p: &mut P, text: &str) {
	let all: Vec<&str> = text.split(super::NL).collect();
	let Some(first) = all.iter().position(|l| return !l.trim().is_empty()) else {
		hold(p);
		return;
	};
	let last_line = all
		.iter()
		.rposition(|l| return !l.trim().is_empty())
		.unwrap_or(first);
	let lines = &all[first..=last_line];
	let anchor = lines[0].chars().take_while(|&c| return c == '\t').count();
	let mut tree = Node::leaf("");
	let mut stack: Vec<Vec<usize>> = Vec::new();
	let mut lastpath: Vec<usize> = Vec::new();
	let mut past_first_newline = false;
	for raw in lines {
		if raw.trim().is_empty() {
			continue;
		}
		let tabs = raw.chars().take_while(|&c| return c == '\t').count();
		let rest = raw.get(tabs..).unwrap_or("");
		if past_first_newline {
			let depth = tabs.saturating_sub(anchor).min(stack.len());
			let parent = match depth {
				0 => Vec::new(),
				_ => stack[depth.saturating_sub(1)].clone(),
			};
			let n = Node::at_mut(&mut tree, &parent);
			n.children.push(Node::leaf(rest));
			let mut path = parent;
			path.push(n.children.len().saturating_sub(1));
			stack.truncate(depth);
			stack.push(path.clone());
			lastpath = path;
		} else {
			if super::FIRST_LINE_SIBLINGS {
				for name in rest.split('\t') {
					tree.children.push(Node::leaf(name));
					let path = vec![tree.children.len().saturating_sub(1)];
					stack.clear();
					stack.push(path.clone());
					lastpath = path;
				}
			} else {
				for (d, name) in rest.split('\t').enumerate() {
					let parent = match d {
						0 => Vec::new(),
						_ => stack[d.saturating_sub(1)].clone(),
					};
					let n = Node::at_mut(&mut tree, &parent);
					n.children.push(Node::leaf(name));
					let mut path = parent;
					path.push(n.children.len().saturating_sub(1));
					stack.truncate(d);
					stack.push(path.clone());
					lastpath = path;
				}
			}
			past_first_newline = true;
		}
	}
	let roots = tree.children;
	let last = lastpath;
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
			for i in (0..n.children.len()).filter(|&i| return n.children[i].name == name) {
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
	ogdl.mutate_shown(|root| {
		for path in &p.cur {
			let n = Node::at_mut(root, path);
			n.children.retain(|c| return c.name != name);
		}
	});
	hold(p);
}

#[inline]
#[must_use]
pub fn glob_block(expr: &str) -> super::Block {
	return ogdl.with(|root| {
		let nodes = root
			.glob(expr)
			.map(|v| return v.iter().map(|n| return Node::leaf(&n.name)).collect())
			.unwrap_or_default();
		return super::Block { nodes, sel: None };
	});
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

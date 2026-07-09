//! OGDL — an indentation-defined tree. Public API is FOUR methods on the graph
//! handle: `itnl`, `file`, `add`, `del`, reachable three ways (a static `ogdl`,
//! the `Ogdl::file` constructor, or the free `ogdl::file`). Values are child
//! nodes; whitespace is the separator; the writer uses a 4-space indent. Tabs and
//! spaces (and legacy `=`) are accepted on read.
#![allow(non_upper_case_globals)]

use std::fmt;
use std::fs;
use std::sync::{Mutex, OnceLock};

// ── Node ─────────────────────────────────────────────────────────────────────
// The tree. A node's "value" is its child node(s): `VRAM 12` is a node named
// VRAM with one child named 12. Fields are public and `Index<usize>` walks the
// children — that is the whole read/traverse surface (no getter methods).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
      pub name: String,
      pub children: Vec<Node>,
}

impl Node {
      pub(crate) fn leaf(name: &str) -> Node {
            Node { name: name.to_string(), children: Vec::new() }
      }

      fn is_leaf(&self) -> bool {
            self.children.is_empty()
      }

      // ── parse: indentation tree; tokens after the first are leaf children ──
      // Depth by leading-whitespace width on a stack (tabs or spaces, internally
      // consistent per file). A legacy `=` is treated as the first separator.
      pub(crate) fn parse(text: &str) -> Node {
            let mut root = Node::leaf("");
            let mut path: Vec<usize> = Vec::new();
            let mut widths: Vec<usize> = Vec::new();
            for raw in text.lines() {
                  let content = raw.trim();
                  if content.is_empty() {
                        continue;
                  }
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

      // ── path selectors (OGDL spec): a.b  a.1  a.b{n}  a.b{}  a[n] ──
      // `{n}`/`[n]` are 1-indexed per the OGDL path spec (n-1'th subnode). `{}`
      // is handled by `select_all`. Returns the FIRST match here.
      pub(crate) fn select(&self, path: &str) -> Option<&Node> {
            let mut cur = self;
            for seg in path.split('.').filter(|s| !s.is_empty()) {
                  cur = cur.step(seg)?;
            }
            Some(cur)
      }

      fn step(&self, seg: &str) -> Option<&Node> {
            if let Some(rest) = seg.strip_prefix('[') {
                  let i: usize = rest.trim_end_matches(']').parse().ok()?;
                  return self.children.get(i.wrapping_sub(1));
            }
            if let Some((name, sel)) = seg.split_once('{') {
                  let sel = sel.trim_end_matches('}');
                  let nth: usize = if sel.is_empty() { 1 } else { sel.parse().ok()? };
                  return self.children.iter().filter(|c| c.name == name).nth(nth.wrapping_sub(1));
            }
            self.children.iter().find(|c| c.name == seg)
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
            if let Some(rest) = seg.strip_prefix('[') {
                  let i: usize = rest.trim_end_matches(']').parse().ok()?;
                  return i.checked_sub(1).filter(|&i| i < self.children.len());
            }
            if let Some((name, sel)) = seg.split_once('{') {
                  let sel = sel.trim_end_matches('}');
                  let nth: usize = if sel.is_empty() { 1 } else { sel.parse().ok()? };
                  return self
                        .children
                        .iter()
                        .enumerate()
                        .filter(|(_, c)| c.name == name)
                        .map(|(i, _)| i)
                        .nth(nth.wrapping_sub(1));
            }
            self.children.iter().position(|c| c.name == seg)
      }

      // ── writer: 4-space indent. All-leaf children go inline (`VRAM 12`);
      // otherwise each child nests on its own line (order preserved → lossless).
      fn write_to(&self, out: &mut String, depth: usize) {
            for c in &self.children {
                  for _ in 0..depth {
                        out.push_str("    ");
                  }
                  out.push_str(&c.name);
                  if !c.children.is_empty() && c.children.iter().all(Node::is_leaf) {
                        for g in &c.children {
                              out.push(' ');
                              out.push_str(&g.name);
                        }
                        out.push('\n');
                  } else {
                        out.push('\n');
                        c.write_to(out, depth + 1);
                  }
            }
      }

      pub(crate) fn serialize(&self) -> String {
            let mut s = String::new();
            self.write_to(&mut s, 0);
            s
      }
}

// `a[2]` — positional child access, verbatim as the spec's Index impl.
impl std::ops::Index<usize> for Node {
      type Output = Node;
      fn index(&self, i: usize) -> &Node {
            &self.children[i]
      }
}

// A node's Display is its VALUE: the leaf children joined by spaces (what
// `println!("{}", ogdl.itnl("engi.GPU0.VRAM"))` prints). File serialization uses
// `serialize`, not Display, so the two never collide.
impl fmt::Display for Node {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let vals: Vec<&str> = self.children.iter().filter(|c| c.is_leaf()).map(|c| c.name.as_str()).collect();
            f.write_str(&vals.join(" "))
      }
}

// ── graph registry ───────────────────────────────────────────────────────────
// Every graph lives in a global slab; an `Ogdl` is a Copy handle (an index into
// it). This is why every method can take `&self` and hand back a fresh handle for
// chaining, and why the same `Ogdl` value stays usable after `.add().del()`.
struct Reg {
      graphs: Vec<Node>,
}

fn reg() -> &'static Mutex<Reg> {
      static REG: OnceLock<Mutex<Reg>> = OnceLock::new();
      REG.get_or_init(|| Mutex::new(Reg { graphs: vec![Node::leaf("")] }))
}

fn with<R>(id: usize, f: impl FnOnce(&mut Node) -> R) -> R {
      let mut g = reg().lock().unwrap_or_else(|p| p.into_inner());
      while g.graphs.len() <= id {
            g.graphs.push(Node::leaf(""));
      }
      f(&mut g.graphs[id])
}

fn fresh() -> usize {
      let mut g = reg().lock().unwrap_or_else(|p| p.into_inner());
      g.graphs.push(Node::leaf(""));
      g.graphs.len() - 1
}

/// A handle to one graph. Copy; the tree lives in the registry.
#[derive(Clone, Copy, Debug)]
pub struct Ogdl(usize);

/// The process-wide default graph — import style 1 (`use ogdl::*; ogdl.file(..)`).
pub static ogdl: Ogdl = Ogdl(0);

impl Ogdl {
      /// Constructor form of `file` (import styles 2 & 3): read a file into a NEW
      /// graph and return its handle. `Ogdl::file("g.ogdl")` / `ogdl::file(..)`.
      pub fn file(path: &str) -> Ogdl {
            let id = fresh();
            let text = fs::read_to_string(path).unwrap_or_default();
            with(id, |root| *root = Node::parse(&text));
            Ogdl(id)
      }

      pub(crate) fn snapshot(self) -> Node {
            with(self.0, |root| root.clone())
      }
}

/// The free-function entry point — import style 3 (`ogdl::file("g.ogdl")`).
pub fn file(path: &str) -> Ogdl {
      Ogdl::file(path)
}

// ── itnl argument dispatch ───────────────────────────────────────────────────
// `itnl(())` returns the handle (chain on to `.file`), `itnl("a.b")` selects and
// returns the Node there (Display = its value; `.children`/Index to traverse),
// `itnl(node/handle)` binds that graph as the handle's tree.
pub trait Itnl {
      type Out;
      fn apply(self, g: Ogdl) -> Self::Out;
}

impl Itnl for () {
      type Out = Ogdl;
      fn apply(self, g: Ogdl) -> Ogdl {
            g
      }
}

impl Itnl for &str {
      type Out = Node;
      fn apply(self, g: Ogdl) -> Node {
            with(g.0, |root| {
                  // Return the selected subtree tagged with the path it was found
                  // at, so it can be handed straight to add/del as a locatable
                  // target. Display uses the children (the value), not this name.
                  let mut n = root.select(self).cloned().unwrap_or_default();
                  n.name = self.to_string();
                  n
            })
      }
}

impl Itnl for Ogdl {
      type Out = Ogdl;
      fn apply(self, g: Ogdl) -> Ogdl {
            let src = self.snapshot();
            with(g.0, |root| *root = src);
            g
      }
}

impl Itnl for Node {
      type Out = Ogdl;
      fn apply(self, g: Ogdl) -> Ogdl {
            with(g.0, |root| *root = self);
            g
      }
}

// ── del argument dispatch ────────────────────────────────────────────────────
// `del("1", a)` (name + node), `del(a[2])` (positional node), `del!(a.b{})`
// (all matching, via the macro). Each yields a path the graph deletes.
pub trait DelArg {
      fn apply(self, g: Ogdl);
}

impl DelArg for (&str, &Node) {
      fn apply(self, g: Ogdl) {
            let (name, parent) = self;
            with(g.0, |root| {
                  if let Some(p) = root.select_mut(&parent.name) {
                        p.children.retain(|c| c.name != name);
                  } else {
                        root.children.retain(|c| c.name != name);
                  }
            });
      }
}

impl DelArg for &Node {
      fn apply(self, g: Ogdl) {
            with(g.0, |root| root.children.retain(|c| c != self));
      }
}

// ── the four methods ─────────────────────────────────────────────────────────
/// The chaining surface: `itnl` / `file` / `add` / `del`. Import styles 2 and 3
/// bring this into scope (`use ogdl::Chain;`); style 1's glob does it for free.
pub trait Chain {
      fn itnl<A: Itnl>(&self, a: A) -> A::Out;
      fn file(&self, path: &str) -> Ogdl;
      fn add(&self, name: &str, target: &Node) -> Ogdl;
      fn del<D: DelArg>(&self, d: D) -> Ogdl;
}

impl Chain for Ogdl {
      fn itnl<A: Itnl>(&self, a: A) -> A::Out {
            a.apply(*self)
      }

      // Direction from state: an empty graph reads the file in, a populated one
      // writes it out. (Same name both ways — the crate figures it out.)
      fn file(&self, path: &str) -> Ogdl {
            let empty = with(self.0, |root| root.children.is_empty());
            if empty {
                  let text = fs::read_to_string(path).unwrap_or_default();
                  with(self.0, |root| *root = Node::parse(&text));
            } else {
                  let text = with(self.0, |root| root.serialize());
                  let _ = fs::write(path, text);
            }
            *self
      }

      fn add(&self, name: &str, target: &Node) -> Ogdl {
            with(self.0, |root| {
                  let dst = if target.name.is_empty() {
                        Some(&mut *root)
                  } else {
                        root.select_mut(&target.name)
                  };
                  if let Some(dst) = dst {
                        dst.children.push(Node::leaf(name));
                  }
            });
            *self
      }

      fn del<D: DelArg>(&self, d: D) -> Ogdl {
            d.apply(*self);
            *self
      }
}

/// `del!(g, a.b{})` — delete every child named `b` under `a`. The `a.b{}` form is
/// not a legal expression in argument position, so it rides a macro.
#[macro_export]
macro_rules! del {
      ($g:expr, $a:ident . $b:ident {}) => {{
            $crate::__del_all($g, stringify!($a), stringify!($b))
      }};
}

#[doc(hidden)]
pub fn __del_all(g: Ogdl, parent: &str, name: &str) -> Ogdl {
      with(g.0, |root| {
            let target = if parent.is_empty() { Some(&mut *root) } else { root.select_mut(parent) };
            if let Some(p) = target {
                  p.children.retain(|c| c.name != name);
            }
      });
      g
}

#[cfg(test)]
mod tests {
      use super::*;

      const SAMPLE: &str = "engi\n    GPU0\n        VRAM 12\n        FLOPs 380\n    CPU\n        RAM 31\n";

      #[test]
      fn round_trip_itnl_file_itnl() {
            let dir = std::env::temp_dir();
            let p = dir.join("nrs_ogdl_spec_rt.ogdl");
            fs::write(&p, SAMPLE).expect("write");
            let ps = p.to_str().expect("utf8");
            let g = Ogdl::file(ps);
            let back = g.snapshot().serialize();
            assert_eq!(back, SAMPLE, "file -> itnl -> serialize lossless");
      }

      #[test]
      fn select_value() {
            let g = Ogdl(fresh());
            with(g.0, |r| *r = Node::parse(SAMPLE));
            assert_eq!(format!("{}", g.itnl("engi.GPU0.VRAM")), "12");
            assert_eq!(format!("{}", g.itnl("engi.CPU.RAM")), "31");
      }

      #[test]
      fn index_and_selectors() {
            let root = Node::parse("a\n    b\n        x\n    b\n        y\n    1\n        z\n");
            let a = &root.children[0];
            assert_eq!(a.name, "a");
            assert_eq!(a[0].name, "b"); // Index<usize> = children[i]
            assert_eq!(a.select("b").expect("b").children[0].name, "x"); // first b
            assert_eq!(a.select("b{2}").expect("b{2}").children[0].name, "y"); // 2nd b
            assert_eq!(a.select("1").expect("1").children[0].name, "z"); // name "1"
            assert_eq!(a.select("[2]").expect("[2]").name, "b"); // 2nd subnode (n-1)
      }

      #[test]
      fn add_del() {
            let g = Ogdl(fresh());
            with(g.0, |r| *r = Node::parse("a\n    b\n"));
            let a = Node::leaf("a");
            g.add("c", &a);
            assert!(g.snapshot().select("a").expect("a").children.iter().any(|c| c.name == "c"));
            del!(g, a.b {});
            assert!(!g.snapshot().select("a").expect("a").children.iter().any(|c| c.name == "b"));
      }
}

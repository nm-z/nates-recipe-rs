#![allow(non_upper_case_globals)]

use std::fmt;
use std::fs;
use std::sync::{Arc, LazyLock, Mutex};

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

impl std::ops::Index<usize> for Node {
      type Output = Node;
      fn index(&self, i: usize) -> &Node {
            &self.children[i]
      }
}

impl fmt::Display for Node {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let vals: Vec<&str> = self.children.iter().filter(|c| c.is_leaf()).map(|c| c.name.as_str()).collect();
            f.write_str(&vals.join(" "))
      }
}

#[derive(Clone)]
pub struct Graph {
      root: Arc<Mutex<Node>>,
}

impl Graph {
      fn empty() -> Graph {
            Graph { root: Arc::new(Mutex::new(Node::leaf(""))) }
      }

      fn with<R>(&self, f: impl FnOnce(&mut Node) -> R) -> R {
            let mut n = self.root.lock().unwrap_or_else(|p| p.into_inner());
            f(&mut n)
      }
}

pub static ogdl: LazyLock<Graph> = LazyLock::new(Graph::empty);

pub struct Ogdl;

impl Ogdl {
      pub fn file(path: &str) -> Graph {
            Graph::read(path)
      }
}

pub fn file(path: &str) -> Graph {
      Graph::read(path)
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

impl DelArg for (&str, &Node) {
      fn apply(self, g: Graph) {
            let (name, parent) = self;
            g.with(|root| {
                  if let Some(p) = root.select_mut(&parent.name) {
                        p.children.retain(|c| c.name != name);
                  } else {
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

      pub(crate) fn snapshot(&self) -> Node {
            self.with(|root| root.clone())
      }

      pub fn itnl<A: ItnlArg>(&self, a: A) -> A::Out {
            a.apply(self.clone())
      }

      pub fn file(&self, path: &str) -> Graph {
            let empty = self.with(|root| root.children.is_empty());
            if empty {
                  let text = fs::read_to_string(path).unwrap_or_default();
                  self.with(|root| *root = Node::parse(&text));
            } else {
                  let text = self.with(|root| root.serialize());
                  let _ = fs::write(path, text);
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

#[macro_export]
macro_rules! del {
      ($g:expr, $a:ident . $b:ident {}) => {{
            $crate::__macro_support::del_all(&$g, stringify!($a), stringify!($b))
      }};
}

#[doc(hidden)]
pub mod __macro_support {
      use super::{Graph, Node};

      pub fn del_all(g: &Graph, parent: &str, name: &str) -> Graph {
            g.with(|root: &mut Node| {
                  let target = if parent.is_empty() { Some(&mut *root) } else { root.select_mut(parent) };
                  if let Some(p) = target {
                        p.children.retain(|c| c.name != name);
                  }
            });
            g.clone()
      }
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
            let g = Graph::empty();
            g.with(|r| *r = Node::parse(SAMPLE));
            assert_eq!(format!("{}", g.itnl("engi.GPU0.VRAM")), "12");
            assert_eq!(format!("{}", g.itnl("engi.CPU.RAM")), "31");
      }

      #[test]
      fn index_and_selectors() {
            let root = Node::parse("a\n    b\n        x\n    b\n        y\n    1\n        z\n");
            let a = &root.children[0];
            assert_eq!(a.name, "a");
            assert_eq!(a[0].name, "b");
            assert_eq!(a.select("b").expect("b").children[0].name, "x");
            assert_eq!(a.select("b{2}").expect("b{2}").children[0].name, "y");
            assert_eq!(a.select("1").expect("1").children[0].name, "z");
            assert_eq!(a.select("[2]").expect("[2]").name, "b");
      }

      #[test]
      fn handles_free_on_drop() {
            let g = Graph::empty();
            assert_eq!(Arc::strong_count(&g.root), 1);
            let g2 = g.clone();
            assert_eq!(Arc::strong_count(&g.root), 2);
            drop(g2);
            assert_eq!(Arc::strong_count(&g.root), 1);
            g.add("x", "a");
            assert_eq!(Arc::strong_count(&g.root), 1, "chain temporaries leak no refs");
      }

      #[test]
      fn arity_forms_dispatch() {
            let g = Graph::empty();
            g.with(|r| *r = Node::parse("a\n    x\n    y\n"));
            let _ = g.itnl(()).itnl("a");
            let a = g.itnl("a");
            g.del(&a[0]);
            assert!(g.itnl("a").children.iter().all(|c| c.name != "x"));
            g.del(("y", &a));
            assert!(g.itnl("a").children.is_empty());
      }

      #[test]
      fn add_del() {
            let g = Graph::empty();
            g.with(|r| *r = Node::parse("a\n    b\n"));
            g.add("c", "a");
            assert!(g.snapshot().select("a").expect("a").children.iter().any(|c| c.name == "c"));
            del!(g, a.b {});
            assert!(!g.snapshot().select("a").expect("a").children.iter().any(|c| c.name == "b"));
      }

      #[test]
      fn typed_serialize_roundtrip() {
            let g = Graph::empty();
            let weights = vec![0.01_f64, -0.02, 0.03];
            let labels = vec!["cat".to_string(), "dog".to_string()];
            g.add(weights.clone(), "z1.w")
                  .add(0.5_f64, "z1.b")
                  .add(42_i64, "z1.neurons")
                  .add(true, "z1.bias")
                  .add("relu", "z1.act")
                  .add(labels.clone(), "z1.classes");
            let w: Vec<f64> = g.itnl("z1.w").children.iter().filter_map(|c| c.name.parse().ok()).collect();
            assert_eq!(w, weights);
            assert_eq!(format!("{}", g.itnl("z1.b")), "0.5");
            assert_eq!(format!("{}", g.itnl("z1.neurons")), "42");
            assert_eq!(format!("{}", g.itnl("z1.bias")), "true");
            assert_eq!(format!("{}", g.itnl("z1.act")), "relu");
            let classes: Vec<String> = g.itnl("z1.classes").children.iter().map(|c| c.name.clone()).collect();
            assert_eq!(classes, labels);
      }
}

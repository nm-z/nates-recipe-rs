use crate::{Mat, Vec1};
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

#[derive(Clone)]
enum Kind {
      Numeric,
      Nominal(Vec<String>),
}

#[derive(Clone)]
struct Attr {
      name: String,
      kind: Kind,
}

pub struct Data {
      attrs: Vec<Attr>,
      rows: Vec<Vec<String>>,
      target: usize,
}

pub struct Dataset {
      pub x: Mat,
      pub y: Vec1,
}

pub struct DataSplit {
      pub train: Dataset,
      pub test: Dataset,
}

/// Split one ARFF/CSV line into trimmed, unquoted fields, respecting single-quote quoting.
fn split_fields(line: &str) -> Vec<String> {
      let mut out = Vec::new();
      let mut cur = String::new();
      let mut quoted = false;
      for c in line.chars() {
            match c {
                  '\'' => quoted = !quoted,
                  ',' if !quoted => {
                        out.push(cur.trim().to_string());
                        cur.clear();
                  }
                  _ => cur.push(c),
            }
      }
      out.push(cur.trim().to_string());
      out
}

impl Data {
      pub fn load(path: &str) -> Data {
            let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
                  if e.kind() == std::io::ErrorKind::NotFound {
                        let cwd = std::env::current_dir()
                              .map(|p| p.display().to_string())
                              .unwrap_or_else(|_| ".".to_string());
                        let name = std::path::Path::new(path)
                              .file_name()
                              .and_then(|s| s.to_str())
                              .unwrap_or(path);
                        eprintln!("couldn't find '{name}' in {cwd}");
                        eprintln!("run: find ~ -name '{name}'");
                  } else {
                        eprintln!("Data::load: cannot read {path}: {e}");
                  }
                  std::process::exit(1);
            });
            let mut attrs = Vec::new();
            let mut rows = Vec::new();
            let mut in_data = false;
            for raw in text.lines() {
                  let line = raw.trim();
                  if line.is_empty() || line.starts_with('%') {
                        continue;
                  }
                  if in_data {
                        rows.push(split_fields(line));
                        continue;
                  }
                  let lower = line.to_ascii_lowercase();
                  if lower.starts_with("@attribute") {
                        attrs.push(parse_attribute(line));
                  } else if lower.starts_with("@data") {
                        in_data = true;
                  }
            }
            assert!(!attrs.is_empty(), "Data::load: no @attribute lines in {path}");
            assert!(!rows.is_empty(), "Data::load: no @data rows in {path}");
            Data { attrs, rows, target: 0 }
      }

      pub fn target(mut self, name: &str) -> Data {
            self.target = self
                  .attrs
                  .iter()
                  .position(|a| a.name == name)
                  .unwrap_or_else(|| panic!("Data::target: no attribute named '{name}'"));
            self
      }

      pub fn split(self, train_frac: f64) -> DataSplit {
            let (x, y) = self.materialize();
            let n = x.nrows();
            let mut idx: Vec<usize> = (0..n).collect();
            idx.shuffle(&mut ChaCha8Rng::seed_from_u64(42));
            let n_train = (n as f64 * train_frac).round() as usize;
            let take = |sel: &[usize]| -> Dataset {
                  let cols = x.ncols();
                  let mut xd = Vec::with_capacity(sel.len() * cols);
                  let mut yd = Vec::with_capacity(sel.len());
                  for &i in sel {
                        xd.extend(x.row(i).iter().copied());
                        yd.push(y[i]);
                  }
                  Dataset {
                        x: Mat::from_shape_vec((sel.len(), cols), xd)
                              .expect("split: x reshape"),
                        y: Vec1::from(yd),
                  }
            };
            DataSplit {
                  train: take(&idx[..n_train]),
                  test: take(&idx[n_train..]),
            }
      }

      /// Build the encoded design matrix X and target vector y.
      fn materialize(&self) -> (Mat, Vec1) {
            let n = self.rows.len();
            let mut cols: Vec<Vec<f64>> = Vec::new();
            let mut y = vec![0.0f64; n];
            for (ai, attr) in self.attrs.iter().enumerate() {
                  if ai == self.target {
                        let cats = match &attr.kind {
                              Kind::Nominal(c) => c,
                              Kind::Numeric => panic!("target '{}' must be nominal", attr.name),
                        };
                        for (r, row) in self.rows.iter().enumerate() {
                              y[r] = cats
                                    .iter()
                                    .position(|c| c == &row[ai])
                                    .unwrap_or_else(|| panic!(
                                          "target value '{}' not in declared classes {:?}",
                                          row[ai], cats
                                    )) as f64;
                        }
                        continue;
                  }
                  match &attr.kind {
                        Kind::Numeric => {
                              let mut col = vec![0.0f64; n];
                              for (r, row) in self.rows.iter().enumerate() {
                                    col[r] = row[ai].parse::<f64>().unwrap_or_else(|e| {
                                          panic!("attr '{}' value '{}': {e}", attr.name, row[ai])
                                    });
                              }
                              cols.push(col);
                        }
                        Kind::Nominal(categories) => {
                              for cat in categories {
                                    let mut col = vec![0.0f64; n];
                                    for (r, row) in self.rows.iter().enumerate() {
                                          if &row[ai] == cat {
                                                col[r] = 1.0;
                                          }
                                    }
                                    cols.push(col);
                              }
                        }
                  }
            }
            let ncols = cols.len();
            let mut data = Vec::with_capacity(n * ncols);
            for r in 0..n {
                  for c in &cols {
                        data.push(c[r]);
                  }
            }
            (
                  Mat::from_shape_vec((n, ncols), data).expect("materialize: reshape"),
                  Vec1::from(y),
            )
      }
}

/// Parse one `@attribute 'name' { a, b }` or `@attribute name real` line.
fn parse_attribute(line: &str) -> Attr {
      let rest = line[ "@attribute".len()..].trim();
      let (name, spec) = if let Some(r) = rest.strip_prefix('\'') {
            let end = r.find('\'').expect("attribute: unterminated quoted name");
            (r[..end].to_string(), r[end + 1..].trim())
      } else {
            let end = rest.find(char::is_whitespace).expect("attribute: missing type");
            (rest[..end].to_string(), rest[end..].trim())
      };
      let kind = if spec.starts_with('{') {
            let inner = spec
                  .trim_start_matches('{')
                  .trim_end_matches('}');
            Kind::Nominal(split_fields(inner))
      } else {
            Kind::Numeric
      };
      Attr { name, kind }
}

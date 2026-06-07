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
      // The requested target name (from `.target`). For ARFF it's resolved to the
      // `target` index; for tabular set/test it's matched against column names.
      target_name: Option<String>,
      source: String,
      // A separate test file (.test) and/or an internal split fraction (.split).
      // Neither set → no test set → no eval.
      test_path: Option<String>,
      split_frac: Option<f64>,
      // Feature patterns to drop: an exact column name, a `group:*` glob, a bare
      // header, or a group name. The framework keeps everything it finds; the user
      // decides what's useless.
      exclude: Vec<String>,
}

pub struct Dataset {
      pub x: Mat,
      pub y: Vec1,
      pub source: String,
      // True when the target column was present in this set (train always; a
      // Kaggle test.csv has no target → false → eval skips scoring, still predicts).
      pub has_target: bool,
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

/// Read + parse an ARFF file into (attributes, data rows). Exits on read error.
fn parse_arff(path: &str) -> (Vec<Attr>, Vec<Vec<String>>) {
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
                  eprintln!("Data: cannot read {path}: {e}");
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
      assert!(!attrs.is_empty(), "Data: no @attribute lines in {path}");
      assert!(!rows.is_empty(), "Data: no @data rows in {path}");
      (attrs, rows)
}

/// Cell `j` of a raw row as `&str` (missing/short → "").
fn cell(row: &[String], j: usize) -> &str {
      row.get(j).map_or("", |s| s.as_str())
}

/// Infer an ARFF-style schema from raw rows: a column is `Numeric` if every
/// non-empty cell parses as f64, else `Nominal` with its sorted distinct values.
/// Categories are taken from whatever rows are passed (always the SET, so a test
/// file is later encoded against the same category list).
fn infer_attrs(headers: &[String], rows: &[Vec<String>]) -> Vec<Attr> {
      headers
            .iter()
            .enumerate()
            .map(|(j, name)| {
                  let numeric = rows
                        .iter()
                        .all(|row| cell(row, j).is_empty() || cell(row, j).parse::<f64>().is_ok());
                  let kind = if numeric {
                        Kind::Numeric
                  } else {
                        let cats: std::collections::BTreeSet<String> = rows
                              .iter()
                              .map(|row| cell(row, j))
                              .filter(|c| !c.is_empty())
                              .map(|c| c.to_string())
                              .collect();
                        Kind::Nominal(cats.into_iter().collect())
                  };
                  Attr { name: name.clone(), kind }
            })
            .collect()
}

/// Encode raw `rows` against `attrs` into `(feature_names, X, y)`. Feature
/// numerics pass through (blank/unparseable → NaN); feature nominals one-hot as
/// `name=cat`. The `target` column (if any) is label-encoded for a Nominal kind
/// and parsed for a Numeric kind; a blank or unseen target value → NaN. With
/// `target = None`, every column is a feature and `y` is all zeros.
fn encode(attrs: &[Attr], rows: &[Vec<String>], target: Option<usize>) -> (Vec<String>, Mat, Vec1) {
      let n = rows.len();
      let mut names: Vec<String> = Vec::new();
      let mut cols: Vec<Vec<f64>> = Vec::new();
      let mut y = vec![0.0f64; n];
      for (ai, attr) in attrs.iter().enumerate() {
            if Some(ai) == target {
                  match &attr.kind {
                        Kind::Nominal(cats) => {
                              for (r, row) in rows.iter().enumerate() {
                                    let v = cell(row, ai);
                                    y[r] = cats.iter().position(|c| c == v).map_or(f64::NAN, |p| p as f64);
                              }
                        }
                        Kind::Numeric => {
                              for (r, row) in rows.iter().enumerate() {
                                    y[r] = cell(row, ai).parse::<f64>().unwrap_or(f64::NAN);
                              }
                        }
                  }
                  continue;
            }
            match &attr.kind {
                  Kind::Numeric => {
                        names.push(attr.name.clone());
                        let mut col = vec![0.0f64; n];
                        for (r, row) in rows.iter().enumerate() {
                              col[r] = cell(row, ai).parse::<f64>().unwrap_or(f64::NAN);
                        }
                        cols.push(col);
                  }
                  Kind::Nominal(cats) => {
                        for cat in cats {
                              names.push(format!("{}={cat}", attr.name));
                              let mut col = vec![0.0f64; n];
                              for (r, row) in rows.iter().enumerate() {
                                    if cell(row, ai) == cat {
                                          col[r] = 1.0;
                                    }
                              }
                              cols.push(col);
                        }
                  }
            }
      }
      let w = cols.len();
      let mut data = vec![0.0f64; n * w];
      for (j, col) in cols.iter().enumerate() {
            for (i, v) in col.iter().enumerate() {
                  data[i * w + j] = *v;
            }
      }
      (names, Mat::from_shape_vec((n, w), data).expect("encode: reshape"), Vec1::from(y))
}

use crate::data::DirGroup;

/// Per-group inferred schema, so a `.test` source encodes against the SET's
/// categories (keyed by group name; `""` for a single un-grouped file).
type Schema = std::collections::BTreeMap<String, Vec<Attr>>;

/// A source assembled into one training table: namespaced feature `names`, design
/// matrix `x`, target `y`, the `samples` count, and any groups `skipped` because
/// their rows couldn't be hash-aligned to the sample group.
struct Assembled {
      names: Vec<String>,
      // Parallel to `names`: (matrix index, column) locating each feature's values.
      sources: Vec<(usize, usize)>,
      // Per-source-matrix encoded values at that group's own row count (mats[0] is
      // the sample group, n rows; others are un-broadcast group matrices).
      mats: Vec<Mat>,
      // Per matrix, per-sample source row (None → NaN). gather[0] is identity.
      gather: Vec<Vec<Option<usize>>>,
      y: Vec1,
      samples: usize,
      skipped: Vec<String>,
      sample_group: String,
}

impl Assembled {
      /// Materialize the `keep` columns (in order) into a dense `n × keep.len()`
      /// matrix, gathering each from its group matrix via that group's per-sample
      /// index. Only kept columns are built — a dropped/excluded group costs nothing.
      fn select(&self, keep: &[String]) -> Mat {
            let n = self.samples;
            let w = keep.len();
            let idx: std::collections::HashMap<&str, usize> =
                  self.names.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
            let mut data = vec![0.0f64; n * w];
            for (jc, name) in keep.iter().enumerate() {
                  let (mi, col) = self.sources[idx[name.as_str()]];
                  let m = &self.mats[mi];
                  let g = &self.gather[mi];
                  for i in 0..n {
                        data[i * w + jc] = g[i].map_or(f64::NAN, |r| m[[r, col]]);
                  }
            }
            Mat::from_shape_vec((n, w), data).expect("select reshape")
      }
}

/// Namespaced feature name: bare for an un-grouped file, `group:col` for a dir.
fn namespaced(group: &str, col: &str) -> String {
      if group.is_empty() { col.to_string() } else { format!("{group}:{col}") }
}

/// Namespaced names of every TABLE column across the groups — the only target
/// candidates (a pixel is never a target), used for target resolution.
fn table_names(groups: &[DirGroup]) -> Vec<String> {
      let mut out = Vec::new();
      for g in groups {
            if let DirGroup::Table { name, headers, .. } = g {
                  for h in headers {
                        out.push(namespaced(name, h));
                  }
            }
      }
      out
}

/// Load a path into raw groups: a directory → one group per file type; a single
/// file → one un-grouped Table (no hash). Encoding happens later, in `assemble`.
fn load_groups(path: &str) -> Vec<DirGroup> {
      if std::path::Path::new(path).is_dir() {
            crate::data::load_dir_groups(path).expect("load dir")
      } else {
            let (headers, cells) =
                  crate::data::read_raw_csv(std::path::Path::new(path)).expect("read csv");
            let hashes = vec![String::new(); cells.len()];
            vec![DirGroup::Table { name: String::new(), headers, hashes, cells }]
      }
}

/// Encode one group into `(namespaced_names, X, y)`. A Table uses its (set-inferred)
/// schema; the target column is encoded into `y` only for the sample group
/// (`target_col = Some`). An Image is already numeric. Hashes are read separately
/// via `group_hashes` so a skipped group never needs encoding.
fn encode_group(
      g: &DirGroup,
      schema: &mut Schema,
      schema_in: Option<&Schema>,
      target_col: Option<usize>,
) -> (Vec<String>, Mat, Vec1) {
      match g {
            DirGroup::Table { name, headers, cells, .. } => {
                  // Infer from THIS source's own headers (so a test with fewer/extra
                  // columns is encoded by name, not position), but reuse the SET's
                  // category lists for shared columns so one-hot columns match up.
                  let mut attrs = infer_attrs(headers, cells);
                  if let Some(set_attrs) = schema_in.and_then(|s| s.get(name)) {
                        for a in attrs.iter_mut() {
                              if let Some(sa) = set_attrs.iter().find(|s| s.name == a.name) {
                                    a.kind = sa.kind.clone();
                              }
                        }
                  }
                  schema.insert(name.clone(), attrs.clone());
                  let (fnames, x, y) = encode(&attrs, cells, target_col);
                  let names = fnames.iter().map(|f| namespaced(name, f)).collect();
                  (names, x, y)
            }
            DirGroup::Image { name, dim, pixels, .. } => {
                  let n = pixels.len();
                  let mut data = vec![0.0f64; n * dim];
                  for (i, px) in pixels.iter().enumerate() {
                        for (j, v) in px.iter().take(*dim).enumerate() {
                              data[i * dim + j] = *v;
                        }
                  }
                  let names = (0..*dim).map(|i| namespaced(name, &format!("px{i}"))).collect();
                  let x = Mat::from_shape_vec((n, *dim), data).expect("image reshape");
                  (names, x, Vec1::zeros(n))
            }
      }
}

/// Per-row hashes of a group (the sample-correlation ids), borrowed cheaply.
fn group_hashes(g: &DirGroup) -> &[String] {
      match g {
            DirGroup::Table { hashes, .. } | DirGroup::Image { hashes, .. } => hashes,
      }
}

/// Assemble raw `groups` into one table. The group owning `target` defines the
/// samples (its rows, at full resolution); every other group is joined by hash —
/// a 1-row-per-hash group broadcasts onto the sample rows, an equal-rows-per-hash
/// group aligns by within-hash position, and a group whose row counts don't match
/// is reported in `skipped` and left out (never aggregated). `schema_in` (the set
/// schema) is reused so a test source encodes against the same categories.
fn assemble(
      groups: &[DirGroup],
      target: Option<&str>,
      schema_in: Option<&Schema>,
      sample_hint: Option<&str>,
) -> (Assembled, Schema) {
      let mut schema: Schema = Schema::new();

      // Which group/column is the target? (A table column whose namespaced name matches.)
      let mut sample_idx = 0usize;
      let mut target_col: Option<usize> = None;
      if let Some(t) = target {
            for (gi, g) in groups.iter().enumerate() {
                  if let DirGroup::Table { name, headers, .. } = g {
                        if let Some(ci) = headers.iter().position(|h| namespaced(name, h) == t) {
                              sample_idx = gi;
                              target_col = Some(ci);
                              break;
                        }
                  }
            }
      }
      // Target column absent here (e.g. an unlabeled test source): keep the SET's
      // sample group via `sample_hint`; else the sole group / sole table group.
      if target_col.is_none() {
            let by_hint = sample_hint.and_then(|h| groups.iter().position(|g| group_name(g) == h));
            sample_idx = match by_hint {
                  Some(i) => i,
                  None => {
                        let tables: Vec<usize> = groups
                              .iter()
                              .enumerate()
                              .filter(|(_, g)| matches!(g, DirGroup::Table { .. }))
                              .map(|(i, _)| i)
                              .collect();
                        match (groups.len(), tables.as_slice()) {
                              (1, _) => 0,
                              (_, [only]) => *only,
                              _ => panic!(
                                    "multiple groups and no resolvable target — name it with .target() so the sample file is known"
                              ),
                        }
                  }
            };
      }

      // Encode the sample group (carries the target). Its matrix backs mats[0] with
      // an identity gather. Other groups push their OWN (un-broadcast) matrix plus a
      // per-sample gather index — values are materialized later, only for kept
      // columns, so a dropped/excluded image group never broadcasts (no blow-up).
      let (s_names, s_x, y) = encode_group(&groups[sample_idx], &mut schema, schema_in, target_col);
      let s_hashes = group_hashes(&groups[sample_idx]);
      let n = s_x.nrows();

      let mut names: Vec<String> = Vec::new();
      let mut sources: Vec<(usize, usize)> = Vec::new();
      let mut mats: Vec<Mat> = Vec::new();
      let mut gather: Vec<Vec<Option<usize>>> = Vec::new();
      let mut skipped: Vec<String> = Vec::new();

      for (j, nm) in s_names.iter().enumerate() {
            names.push(nm.clone());
            sources.push((0, j));
      }
      gather.push((0..n).map(Some).collect());
      mats.push(s_x);

      // Sample-group hash bookkeeping: count per hash + each row's within-hash position.
      let mut s_count: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
      for h in s_hashes {
            *s_count.entry(h.as_str()).or_insert(0) += 1;
      }
      let mut s_pos = vec![0usize; n];
      {
            let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
            for (i, h) in s_hashes.iter().enumerate() {
                  let c = seen.entry(h.as_str()).or_insert(0);
                  s_pos[i] = *c;
                  *c += 1;
            }
      }

      for (gi, g) in groups.iter().enumerate() {
            if gi == sample_idx {
                  continue;
            }
            // Decide the join from hashes ALONE — never encode a group we'll skip
            // (a mismatched 5M-row group must not be materialized just to be dropped).
            let g_hashes = group_hashes(g);
            let mut by_hash: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
            for (gi2, h) in g_hashes.iter().enumerate() {
                  by_hash.entry(h.as_str()).or_default().push(gi2);
            }
            // Unrelated table: shares no hash with the sample group (a separate table
            // in a relational dump — e.g. Cities.csv vs SampleSubmission.csv). Report
            // it, don't fabricate NaN columns by "joining" on nothing.
            let shares = by_hash.keys().any(|h| s_count.contains_key(*h));
            if !shares {
                  skipped.push(format!(
                        "{}: shares no join key with the sample group — separate table, use .exclude() or join manually",
                        group_name(g)
                  ));
                  continue;
            }
            let all_one = by_hash.values().all(|v| v.len() == 1);
            // Aligns if, for every sample hash present in this group, counts match.
            let aligns = s_count
                  .iter()
                  .all(|(h, &sc)| by_hash.get(*h).map_or(true, |v| v.len() == sc));
            if !all_one && !aligns {
                  skipped.push(format!(
                        "{}: {} rows across {} hashes don't hash-align to the sample group ({} samples) — excluded, use .exclude() or join manually",
                        group_name(g), g_hashes.len(), by_hash.len(), n
                  ));
                  continue;
            }
            // An image group is one image per well: broadcasting its pixels into every
            // sample row would duplicate it thousands of times (the 125 GB blow-up).
            // Keep it OUT of the dense matrix — it stays one copy per well, linked by
            // hash, for an index-based GPU path, not copied into rows here.
            if matches!(g, DirGroup::Image { .. }) {
                  skipped.push(format!(
                        "{}: image group ({} wells) kept out of the feature matrix — one copy per well, not duplicated into rows",
                        group_name(g), by_hash.len()
                  ));
                  continue;
            }
            let (g_names, g_x, _gy) = encode_group(g, &mut schema, schema_in, None);
            // Per-sample source row in this group (or None → NaN), broadcast (1/hash)
            // or position-aligned (equal counts). Values gathered lazily in `select`.
            let src: Vec<Option<usize>> = (0..n)
                  .map(|i| {
                        let h = s_hashes[i].as_str();
                        by_hash.get(h).and_then(|v| {
                              if all_one { v.first().copied() } else { v.get(s_pos[i]).copied() }
                        })
                  })
                  .collect();
            let mi = mats.len();
            for (j, nm) in g_names.iter().enumerate() {
                  names.push(nm.clone());
                  sources.push((mi, j));
            }
            gather.push(src);
            mats.push(g_x);
      }

      (
            Assembled {
                  names,
                  sources,
                  mats,
                  gather,
                  y,
                  samples: n,
                  skipped,
                  sample_group: group_name(&groups[sample_idx]).to_string(),
            },
            schema,
      )
}

/// A group's display name (`""` un-grouped file shows as its path elsewhere).
fn group_name(g: &DirGroup) -> &str {
      match g {
            DirGroup::Table { name, .. } | DirGroup::Image { name, .. } => name,
      }
}

/// Does exclude `pattern` match feature `name`? Exact, `group:*` glob, group name,
/// or bare header (the part after `:`).
fn exclude_match(pattern: &str, name: &str) -> bool {
      if pattern == name {
            return true;
      }
      if let Some(g) = pattern.strip_suffix(":*") {
            return name.split_once(':').map(|(ng, _)| ng) == Some(g);
      }
      if name.split_once(':').map(|(ng, _)| ng) == Some(pattern) {
            return true;
      }
      col_after(name) == pattern
}

impl Data {
      /// Start a data builder. Call `.set(path)` to load the predictors file.
      pub fn load() -> Data {
            Data {
                  attrs: Vec::new(),
                  rows: Vec::new(),
                  target: 0,
                  target_name: None,
                  source: String::new(),
                  test_path: None,
                  split_frac: None,
                  exclude: Vec::new(),
            }
      }

      /// Load the predictors. Accepts anything: an ARFF file (parsed now, schema
      /// known up front), or a CSV file / directory of hash-correlated samples
      /// (loaded as a named table in `prepare`). The `.target` is found in the set.
      pub fn set(mut self, path: &str) -> Data {
            self.source = path.to_string();
            if is_arff(path) {
                  let (attrs, rows) = parse_arff(path);
                  self.attrs = attrs;
                  self.rows = rows;
            }
            self
      }

      /// Name the target column. For an ARFF set it's resolved to the attribute
      /// index now; for a tabular set/dir it's matched against column names in
      /// `prepare` (exact, `group:name`, or trailing `:name`).
      pub fn target(mut self, name: &str) -> Data {
            self.target_name = Some(name.to_string());
            if !self.attrs.is_empty() {
                  self.target = self
                        .attrs
                        .iter()
                        .position(|a| a.name == name)
                        .unwrap_or_else(|| panic!("Data::target: no attribute named '{name}'"));
            }
            self
      }

      /// Use a separate pre-split test file (encoded with the train schema).
      pub fn test(mut self, path: &str) -> Data {
            self.test_path = Some(path.to_string());
            self
      }

      /// Drop features matching `pattern`: an exact column name, a `group:*` glob,
      /// a group name, or a bare header (matches that column in any group).
      pub fn exclude(mut self, pattern: &str) -> Data {
            self.exclude.push(pattern.to_string());
            self
      }

      /// Hold out `1 - train_frac` of the `.set` file as the test set.
      pub fn split(mut self, train_frac: f64) -> Data {
            self.split_frac = Some(train_frac);
            self
      }

      /// Build a train `Dataset` and an optional test `Dataset`. An ARFF set keeps
      /// its self-describing schema path; anything else (CSV file or directory, for
      /// both `.set` and `.test`) goes through the unified named-table path that
      /// auto-detects features + target, aligns train↔test on shared columns, and
      /// prints its interpretation.
      pub fn prepare(&self) -> (Dataset, Option<Dataset>) {
            let (mut train, test) = if self.attrs.is_empty() {
                  self.prepare_table()
            } else {
                  self.prepare_arff()
            };
            // Report where NaNs are, then drop any train sample carrying a NaN (in
            // the target or any feature). Reasonable default until imputation lands.
            report_nans(&train, test.as_ref());
            drop_nan_samples(&mut train);
            (train, test)
      }

      /// ARFF set: encode against the declared schema; a `.test` ARFF is encoded
      /// with that same schema (so it aligns by construction).
      fn prepare_arff(&self) -> (Dataset, Option<Dataset>) {
            let (_, x, y) = encode(&self.attrs, &self.rows, Some(self.target));
            if let Some(frac) = self.split_frac {
                  let (tr, te) = shuffle_split(&x, &y, frac, &self.source);
                  report_split(tr.x.nrows(), te.x.nrows());
                  (tr, Some(te))
            } else if let Some(tp) = &self.test_path {
                  let (_, trows) = parse_arff(tp);
                  let (_, tx, ty) = encode(&self.attrs, &trows, Some(self.target));
                  report_split(x.nrows(), tx.nrows());
                  (
                        Dataset { x, y, source: self.source.clone(), has_target: true },
                        Some(Dataset { x: tx, y: ty, source: tp.clone(), has_target: true }),
                  )
            } else {
                  (Dataset { x, y, source: self.source.clone(), has_target: true }, None)
            }
      }

      /// Unified path for CSV files and directories. Each source is parsed into raw
      /// groups, then `assemble`d into one table where the group owning `.target`
      /// defines the samples and the rest are hash-joined. Train/test align on the
      /// shared (namespaced) feature columns; `.exclude` patterns are dropped. The
      /// target is `.target` (matched by name) or, when a test exists, the lone
      /// train-only column.
      fn prepare_table(&self) -> (Dataset, Option<Dataset>) {
            let set_groups = load_groups(&self.source);
            let set_tnames = table_names(&set_groups);

            let test_groups = self.test_path.as_ref().map(|tp| (load_groups(tp), tp.clone()));
            let test_tnames: Option<Vec<String>> = match (&test_groups, self.split_frac) {
                  (Some((g, _)), _) => Some(table_names(g)),
                  (None, Some(_)) => Some(set_tnames.clone()),
                  (None, None) => None,
            };
            let t = self.resolve_target(&set_tnames, test_tnames.as_deref());

            let (set, schema) = assemble(&set_groups, t.as_deref(), None, None);
            let keep = |name: &str| !self.exclude.iter().any(|p| exclude_match(p, name));

            if let Some((tg, tp)) = &test_groups {
                  let (test, _) = assemble(tg, t.as_deref(), Some(&schema), Some(&set.sample_group));
                  // A Kaggle test.csv legitimately omits the target — that's the thing
                  // to predict. Record its absence; don't crash.
                  let test_has_target = t.as_ref().is_some_and(|tgt| test.names.iter().any(|n| n == tgt));
                  let tset: std::collections::HashSet<&str> =
                        test.names.iter().map(|s| s.as_str()).collect();
                  let feats: Vec<String> = set
                        .names
                        .iter()
                        .filter(|n| tset.contains(n.as_str()) && keep(n))
                        .cloned()
                        .collect();
                  assert!(
                        !feats.is_empty(),
                        "set ({}) and test ({tp}) share no feature columns — data is non-correlated",
                        self.source
                  );
                  report_parsed(&set, &feats, t.as_deref(), Some(&test), None);
                  let train = Dataset { x: set.select(&feats), y: set.y, source: self.source.clone(), has_target: true };
                  let testds = Dataset { x: test.select(&feats), y: test.y, source: (*tp).clone(), has_target: test_has_target };
                  return (train, Some(testds));
            }

            let feats: Vec<String> = set.names.iter().filter(|n| keep(n)).cloned().collect();
            let x = set.select(&feats);
            if let Some(frac) = self.split_frac {
                  let (tr, te) = shuffle_split(&x, &set.y, frac, &self.source);
                  report_parsed(&set, &feats, t.as_deref(), None, Some((tr.x.nrows(), te.x.nrows())));
                  return (tr, Some(te));
            }
            report_parsed(&set, &feats, t.as_deref(), None, None);
            (Dataset { x, y: set.y, source: self.source.clone(), has_target: true }, None)
      }

      /// Resolve the target column name. `.target` wins (matched exactly, as a
      /// `group:name`, or by the trailing `:name`); otherwise, when a test set is
      /// present, a single column that exists in the set but not the test is taken
      /// as the target (the thing to predict). Ambiguous (many train-only columns)
      /// or absent → `None`.
      fn resolve_target(&self, set_names: &[String], test_names: Option<&[String]>) -> Option<String> {
            if let Some(want) = &self.target_name {
                  let hit = set_names.iter().find(|n| {
                        n.as_str() == want
                              || n.ends_with(&format!(":{want}"))
                              || n.rsplit(':').next() == Some(want.as_str())
                  });
                  match hit {
                        Some(h) => return Some(h.clone()),
                        None => {
                              // Clear OGDL error + clean exit, not a Rust backtrace, so
                              // the user can read the columns and fix `.target(...)`.
                              eprintln!("\x1b[1;31mtarget '{want}' not found\x1b[0m");
                              eprintln!("    available columns");
                              for n in set_names {
                                    eprintln!("        {n}");
                              }
                              std::process::exit(1);
                        }
                  }
            }
            if let Some(tn) = test_names {
                  let tset: std::collections::HashSet<&str> = tn.iter().map(|s| s.as_str()).collect();
                  let only: Vec<&String> =
                        set_names.iter().filter(|n| !tset.contains(n.as_str())).collect();
                  if only.len() == 1 {
                        return Some(only[0].clone());
                  }
            }
            None
      }

}

/// Print the parsed shape as OGDL: green `parsed` root → the feature-column schema
/// ONCE, the target, then the sample counts per partition (train/eval for a split,
/// or set/test) and any `unjoined` groups. The schema is identical across
/// partitions, so it's never repeated, and no per-source path is shown.
fn report_parsed(
      set: &Assembled,
      feats: &[String],
      target: Option<&str>,
      test: Option<&Assembled>,
      split: Option<(usize, usize)>,
) {
      eprintln!("\x1b[32mparsed\x1b[0m");
      let refs: Vec<&str> = feats.iter().map(|s| s.as_str()).collect();
      emit_section("feature column", &refs, 4);
      if let Some(t) = target {
            eprintln!("    target  {t}");
      }
      match split {
            Some((train_rows, eval_rows)) => {
                  eprintln!("    train  {}", plural(train_rows, "sample"));
                  eprintln!("    eval  {}", plural(eval_rows, "sample"));
            }
            None => {
                  eprintln!("    set  {}", plural(set.samples, "sample"));
                  if let Some(te) = test {
                        eprintln!("    test  {}", plural(te.samples, "sample"));
                  }
            }
      }
      for s in &set.skipped {
            eprintln!("    \x1b[33munjoined\x1b[0m  {s}");
      }
}

/// Strip a `group:` prefix from a feature name for display.
fn col_after(c: &str) -> &str {
      c.split_once(':').map_or(c, |(_, s)| s)
}

/// True if every column is a flattened-image pixel (`group:px<n>`), so the group
/// is an image rather than a table of named columns.
fn is_image_group(cols: &[&str]) -> bool {
      !cols.is_empty()
            && cols.iter().all(|c| {
                  col_after(c)
                        .strip_prefix("px")
                        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            })
}

/// Image group dimensions `(width, height, channels)` from the value count, when
/// it factors as a square RGB image (matches `image_to_row`); else `None`.
fn img_dims(n: usize) -> Option<(usize, usize, usize)> {
      let px = n / 3;
      let side = (px as f64).sqrt() as usize;
      (n % 3 == 0 && side * side == px).then_some((side, side, 3))
}

/// `n unit` with a plural `s` unless `n == 1`.
fn plural(n: usize, unit: &str) -> String {
      format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}

/// A `N feature columns` / `N target columns` node with its columns as direct
/// sibling children (the `group:` prefix stripped). An image group shows its
/// encoding instead of pixel names. Skipped when empty.
fn emit_section(unit: &str, list: &[&str], indent: usize) {
      if list.is_empty() {
            return;
      }
      eprintln!("{}{}", " ".repeat(indent), plural(list.len(), unit));
      let cpad = " ".repeat(indent + 4);
      let mut groups: std::collections::BTreeMap<&str, Vec<&str>> = std::collections::BTreeMap::new();
      for c in list {
            groups.entry(feat_group(c)).or_default().push(c);
      }
      for (_g, members) in groups {
            if is_image_group(&members) {
                  match img_dims(members.len()) {
                        Some((w, h, c)) => {
                              eprintln!("{cpad}{w} width");
                              eprintln!("{cpad}{h} height");
                              eprintln!("{cpad}{c} channels (RGB)");
                        }
                        None => eprintln!("{cpad}flattened → {} values", members.len()),
                  }
            } else {
                  for m in members {
                        eprintln!("{cpad}{}", col_after(m));
                  }
            }
      }
}

/// Whether `path` is an ARFF file (by extension) — the only self-describing
/// format that bypasses the named-table path.
fn is_arff(path: &str) -> bool {
      std::path::Path::new(path).extension().and_then(|e| e.to_str()) == Some("arff")
}

/// Drop train samples carrying any NaN — in the target or any feature. Test is
/// left intact (its samples must still be predicted). A reasonable default until
/// feature imputation is added.
pub(crate) fn drop_nan_samples(train: &mut Dataset) {
      let keep: Vec<usize> = (0..train.y.len())
            .filter(|&i| !train.y[i].is_nan() && train.x.row(i).iter().all(|v| !v.is_nan()))
            .collect();
      let dropped = train.y.len() - keep.len();
      if dropped == 0 {
            return;
      }
      train.x = train.x.select(ndarray::Axis(0), &keep);
      train.y = train.y.select(ndarray::Axis(0), &keep);
      eprintln!(
            "\x1b[32mhandled\x1b[0m\n    train\n        dropped {dropped} {} (NaN)",
            if dropped == 1 { "sample" } else { "samples" }
      );
}

/// `(NaN cells in features, feature rows touched, NaN cells in target)`.
pub(crate) fn nan_stats(d: &Dataset) -> (usize, usize, usize) {
      let cells = d.x.iter().filter(|v| v.is_nan()).count();
      let rows = d.x.outer_iter().filter(|r| r.iter().any(|v| v.is_nan())).count();
      let target = d.y.iter().filter(|v| v.is_nan()).count();
      (cells, rows, target)
}

/// OGDL warning of where NaNs live in the final train/test data — printed only
/// when some exist. Detection only; handling is decided by the caller.
fn report_nans(train: &Dataset, test: Option<&Dataset>) {
      let (tf, tr, tt) = nan_stats(train);
      let (ef, er, et) = test.map(nan_stats).unwrap_or((0, 0, 0));
      if tf + tt + ef + et == 0 {
            return;
      }
      let rows = |n: usize| if n == 1 { "row" } else { "rows" };
      eprintln!("\x1b[1;31mnans\x1b[0m");
      if tf > 0 || tt > 0 {
            eprintln!("    train");
            if tf > 0 {
                  eprintln!("        {tf} in features ({tr} {})", rows(tr));
            }
            if tt > 0 {
                  eprintln!("        {tt} in target");
            }
      }
      if ef > 0 || et > 0 {
            eprintln!("    test");
            if ef > 0 {
                  eprintln!("        {ef} in features ({er} {})", rows(er));
            }
            if et > 0 {
                  eprintln!("        {et} in target");
            }
      }
}

/// Shuffled `train_frac`/`1-train_frac` row split (seed 42), reporting the split.
fn shuffle_split(x: &Mat, y: &Vec1, train_frac: f64, source: &str) -> (Dataset, Dataset) {
      let n = x.nrows();
      let mut idx: Vec<usize> = (0..n).collect();
      idx.shuffle(&mut ChaCha8Rng::seed_from_u64(42));
      let n_train = (n as f64 * train_frac).round() as usize;
      let cols = x.ncols();
      let take = |sel: &[usize]| -> Dataset {
            let mut xd = Vec::with_capacity(sel.len() * cols);
            let mut yd = Vec::with_capacity(sel.len());
            for &i in sel {
                  xd.extend(x.row(i).iter().copied());
                  yd.push(y[i]);
            }
            Dataset {
                  x: Mat::from_shape_vec((sel.len(), cols), xd).expect("split: x reshape"),
                  y: Vec1::from(yd),
                  source: source.to_string(),
                  has_target: true,
            }
      };
      (take(&idx[..n_train]), take(&idx[n_train..]))
}

fn report_split(ntr: usize, nte: usize) {
      let pct = ntr as f64 / (ntr + nte).max(1) as f64 * 100.0;
      eprintln!("split: {pct:.1}% train ({ntr}) / {:.1}% test ({nte})", 100.0 - pct);
}

/// Group prefix of a `group:column` feature name (everything before the first `:`).
fn feat_group(name: &str) -> &str {
      name.split_once(':').map_or(name, |(g, _)| g)
}

/// Parse one `@attribute 'name' { a, b }` or `@attribute name real` line.
fn parse_attribute(line: &str) -> Attr {
      let rest = line["@attribute".len()..].trim();
      let (name, spec) = if let Some(r) = rest.strip_prefix('\'') {
            let end = r.find('\'').expect("attribute: unterminated quoted name");
            (r[..end].to_string(), r[end + 1..].trim())
      } else {
            let end = rest.find(char::is_whitespace).expect("attribute: missing type");
            (rest[..end].to_string(), rest[end..].trim())
      };
      let kind = if spec.starts_with('{') {
            let inner = spec.trim_start_matches('{').trim_end_matches('}');
            Kind::Nominal(split_fields(inner))
      } else {
            Kind::Numeric
      };
      Attr { name, kind }
}

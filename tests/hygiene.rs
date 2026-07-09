use std::fs;
use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

const SRC_ROOTS: &[&str] = &["src", "recipe-infer/src", "ogdl/src"];
const LEAK_ROOTS: &[&str] = &["src", "recipe-infer/src", "ogdl/src", "examples"];
const LIB_ROOTS: &[&str] = &["src/lib.rs", "src/utils", "recipe-infer/src", "ogdl/src"];

const LEAK_PATTERNS: &[&str] = &["Box::leak", "mem::forget", "ManuallyDrop::new"];
const STATIC_MUT_RETURN: &str = "-> &'static mut";
const UNFINISHED_MARKERS: &[&str] =
      &["todo!", "unimplemented!", "unreachable!", "FIXME", "HACK", "XXX"];
const HOME_PATHS: &[&str] = &["/home/nate", "/home/claude", "/Users/"];
const GLOBAL_MUT_PATTERNS: &[&str] = &["static mut ", "lazy_static!"];
const ASSERTION_KEYWORDS: &[&str] = &["assert", "panic", "expect", "should_panic"];

const SPY: &str = "gpu-core/src/callspy.rs";
const SPY_DECL: &str = "pub const N: usize = ";
const SPY_COUNTERS: usize = 47;
const MAX_COMMENT_RUN: usize = 2;
const MAX_DOC_LINES_ON_PUB_FN: usize = 1;
const MAX_FILE_LINES: usize = 2000;

fn rs_files(roots: &[&str]) -> Vec<PathBuf> {
      let mut out = Vec::new();
      for r in roots {
            collect_rs(&Path::new(ROOT).join(r), &mut out);
      }
      out.sort();
      out
}

fn collect_rs(p: &Path, out: &mut Vec<PathBuf>) {
      if p.is_file() {
            if p.extension().is_some_and(|e| e == "rs") {
                  out.push(p.to_path_buf());
            }
            return;
      }
      let Ok(rd) = fs::read_dir(p) else { return };
      for e in rd.flatten() {
            collect_rs(&e.path(), out);
      }
}

fn rel(p: &Path) -> String {
      p.strip_prefix(ROOT).unwrap_or(p).display().to_string()
}

fn path_contains(p: &Path, word: &str) -> bool {
      rel(p).contains(word)
}

fn read(p: &Path) -> String {
      fs::read_to_string(p).unwrap_or_else(|e| panic!("read {}: {e}", rel(p)))
}

fn assert_absent(check: &str, hits: &[String]) {
      assert!(hits.is_empty(), "FAIL: {check}\n  {}", hits.join("\n  "));
}

struct Marks {
      chars: Vec<Vec<char>>,
      in_comment: Vec<Vec<bool>>,
      is_code: Vec<Vec<bool>>,
}

enum Lex {
      Normal,
      LineComment,
      BlockComment(usize),
      Str,
      RawStr(usize),
      CharLit,
}

fn mark(src: &str) -> Marks {
      let nlines = src.lines().count().max(1);
      let mut m = Marks {
            chars: vec![Vec::new(); nlines],
            in_comment: vec![Vec::new(); nlines],
            is_code: vec![Vec::new(); nlines],
      };
      let ch: Vec<char> = src.chars().collect();
      let at = |i: usize| ch.get(i).copied().unwrap_or('\0');
      let mut lex = Lex::Normal;
      let mut line = 0usize;
      let mut i = 0usize;
      while i < ch.len() {
            let c = ch[i];
            if c == '\n' {
                  if let Lex::LineComment = lex {
                        lex = Lex::Normal;
                  }
                  line += 1;
                  i += 1;
                  continue;
            }
            let take = |m: &mut Marks, n: usize, comment: bool, code: bool| {
                  for k in 0..n {
                        m.chars[line].push(at(i + k));
                        m.in_comment[line].push(comment);
                        m.is_code[line].push(code);
                  }
            };
            match lex {
                  Lex::Normal => {
                        if c == '/' && at(i + 1) == '/' {
                              lex = Lex::LineComment;
                              take(&mut m, 2, true, false);
                              i += 2;
                        } else if c == '/' && at(i + 1) == '*' {
                              lex = Lex::BlockComment(1);
                              take(&mut m, 2, true, false);
                              i += 2;
                        } else if c == '"' {
                              lex = Lex::Str;
                              take(&mut m, 1, false, false);
                              i += 1;
                        } else if c == 'r' && raw_string_hashes(&ch, i).is_some() {
                              let h = raw_string_hashes(&ch, i).unwrap_or(0);
                              lex = Lex::RawStr(h);
                              take(&mut m, h + 2, false, false);
                              i += h + 2;
                        } else if c == '\'' && is_char_literal(&ch, i) {
                              lex = Lex::CharLit;
                              take(&mut m, 1, false, false);
                              i += 1;
                        } else {
                              take(&mut m, 1, false, true);
                              i += 1;
                        }
                  }
                  Lex::LineComment => {
                        take(&mut m, 1, true, false);
                        i += 1;
                  }
                  Lex::BlockComment(d) => {
                        if c == '/' && at(i + 1) == '*' {
                              lex = Lex::BlockComment(d + 1);
                              take(&mut m, 2, true, false);
                              i += 2;
                        } else if c == '*' && at(i + 1) == '/' {
                              lex = if d == 1 { Lex::Normal } else { Lex::BlockComment(d - 1) };
                              take(&mut m, 2, true, false);
                              i += 2;
                        } else {
                              take(&mut m, 1, true, false);
                              i += 1;
                        }
                  }
                  Lex::Str => {
                        if c == '\\' {
                              take(&mut m, 2, false, false);
                              i += 2;
                        } else {
                              if c == '"' {
                                    lex = Lex::Normal;
                              }
                              take(&mut m, 1, false, false);
                              i += 1;
                        }
                  }
                  Lex::RawStr(h) => {
                        if c == '"' && (1..=h).all(|k| at(i + k) == '#') {
                              lex = Lex::Normal;
                              take(&mut m, h + 1, false, false);
                              i += h + 1;
                        } else {
                              take(&mut m, 1, false, false);
                              i += 1;
                        }
                  }
                  Lex::CharLit => {
                        if c == '\\' {
                              take(&mut m, 2, false, false);
                              i += 2;
                        } else {
                              if c == '\'' {
                                    lex = Lex::Normal;
                              }
                              take(&mut m, 1, false, false);
                              i += 1;
                        }
                  }
            }
      }
      m
}

fn raw_string_hashes(ch: &[char], i: usize) -> Option<usize> {
      if i > 0 && (ch[i - 1].is_alphanumeric() || ch[i - 1] == '_') {
            return None;
      }
      let mut h = 0;
      while ch.get(i + 1 + h) == Some(&'#') {
            h += 1;
      }
      (ch.get(i + 1 + h) == Some(&'"')).then_some(h)
}

fn is_char_literal(ch: &[char], i: usize) -> bool {
      ch.get(i + 1) == Some(&'\\') || ch.get(i + 2) == Some(&'\'')
}

impl Marks {
      fn has_comment(&self, l: usize) -> bool {
            self.in_comment[l].iter().any(|&x| x)
      }

      fn without_comments(&self, l: usize) -> String {
            self.chars[l]
                  .iter()
                  .zip(&self.in_comment[l])
                  .map(|(c, &x)| if x { ' ' } else { *c })
                  .collect()
      }

      fn code_only(&self, l: usize) -> String {
            self.chars[l]
                  .iter()
                  .zip(&self.is_code[l])
                  .map(|(c, &x)| if x { *c } else { ' ' })
                  .collect()
      }

      fn nlines(&self) -> usize {
            self.chars.len()
      }
}

fn cfg_test_lines(code: &[String]) -> Vec<bool> {
      let n = code.len();
      let mut mask = vec![false; n];
      let mut i = 0;
      while i < n {
            if !code[i].trim_start().starts_with("#[cfg(test)]") {
                  i += 1;
                  continue;
            }
            let (mut depth, mut opened, mut j) = (0i32, false, i);
            while j < n {
                  for c in code[j].chars() {
                        match c {
                              '{' => {
                                    depth += 1;
                                    opened = true;
                              }
                              '}' => depth -= 1,
                              _ => {}
                        }
                  }
                  mask[j] = true;
                  if opened && depth <= 0 {
                        break;
                  }
                  if !opened && j > i && code[j].contains(';') {
                        break;
                  }
                  j += 1;
            }
            i = j + 1;
      }
      mask
}

fn non_test_lines(src: &str) -> (Marks, Vec<bool>) {
      let m = mark(src);
      let code: Vec<String> = (0..m.nlines()).map(|l| m.code_only(l)).collect();
      let is_test = cfg_test_lines(&code);
      (m, is_test)
}

fn is_decorative(c: char) -> bool {
      let u = c as u32;
      (0x2190..=0x21FF).contains(&u)
            || (0x2500..=0x257F).contains(&u)
            || (0x2580..=0x259F).contains(&u)
            || u == 0x00B5
}

fn has_bare_println(line: &str) -> bool {
      let b = line.as_bytes();
      line.match_indices("println!")
            .any(|(i, _)| i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
}

fn returns_mut_type(line: &str) -> bool {
      line.match_indices("-> &mut ")
            .any(|(i, s)| line[i + s.len()..].starts_with(|c: char| c.is_ascii_uppercase()))
}

fn doc_lines_above(lines: &[&str], i: usize) -> usize {
      let mut d = 0usize;
      while i > d && lines[i - 1 - d].trim_start().starts_with("///") {
            d += 1;
      }
      d
}

fn test_fn_body(lines: &[&str], code: &[String], at: usize) -> Option<(String, String)> {
      let n = code.len();
      let mut j = at;
      while j < n && !code[j].contains("fn ") {
            j += 1;
      }
      if j >= n {
            return None;
      }
      let name = lines[j].trim().to_owned();
      let (mut depth, mut opened) = (0i32, false);
      let mut body = String::new();
      for k in at..n {
            body.push_str(lines[k]);
            body.push('\n');
            for c in code[k].chars() {
                  match c {
                        '{' => {
                              depth += 1;
                              opened = true;
                        }
                        '}' => depth -= 1,
                        _ => {}
                  }
            }
            if opened && depth <= 0 {
                  break;
            }
      }
      Some((name, body))
}

#[test]
fn h00_scanner_sees_sources() {
      let f = rs_files(LEAK_ROOTS);
      assert!(f.len() >= 12, "scanner found only {} .rs files", f.len());
      for want in ["src/lib.rs", "src/utils/train.rs", "recipe-infer/src/params.rs"] {
            assert!(f.iter().any(|p| rel(p) == want), "scanner missed {want}");
      }
}

#[test]
fn h01_no_memory_leak_pattern() {
      let mut hits = Vec::new();
      for p in rs_files(LEAK_ROOTS) {
            for (i, l) in read(&p).lines().enumerate() {
                  for pat in LEAK_PATTERNS {
                        if l.contains(pat) {
                              hits.push(format!("{}:{}: {pat}", rel(&p), i + 1));
                        }
                  }
            }
      }
      assert_absent("memory leak pattern", &hits);
}

#[test]
fn h02_no_static_mut_return() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            for (i, l) in read(&p).lines().enumerate() {
                  if l.contains(STATIC_MUT_RETURN) {
                        hits.push(format!("{}:{}", rel(&p), i + 1));
                  }
            }
      }
      assert_absent("static mut return", &hits);
}

#[test]
fn h03_no_comments() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            if path_contains(&p, "test") {
                  continue;
            }
            let m = mark(&read(&p));
            for l in 0..m.nlines() {
                  if m.has_comment(l) {
                        hits.push(format!("{}:{}", rel(&p), l + 1));
                  }
            }
      }
      assert_absent("comment", &hits);
}

#[test]
fn h04_no_bare_unwrap() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            if path_contains(&p, "test") {
                  continue;
            }
            let (m, is_test) = non_test_lines(&read(&p));
            for l in 0..m.nlines() {
                  if !is_test[l] && m.without_comments(l).contains(".unwrap()") {
                        hits.push(format!("{}:{}", rel(&p), l + 1));
                  }
            }
      }
      assert_absent("bare unwrap", &hits);
}

#[test]
fn h05_no_unfinished_code_marker() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            if path_contains(&p, "test") {
                  continue;
            }
            let src = read(&p);
            let (_, is_test) = non_test_lines(&src);
            for (i, l) in src.lines().enumerate() {
                  if is_test.get(i).copied().unwrap_or(false) {
                        continue;
                  }
                  for pat in UNFINISHED_MARKERS {
                        if l.contains(pat) {
                              hits.push(format!("{}:{}: {pat}", rel(&p), i + 1));
                        }
                  }
            }
      }
      assert_absent("unfinished code marker", &hits);
}

#[test]
fn h06_no_println_in_library() {
      let mut hits = Vec::new();
      for p in rs_files(LIB_ROOTS) {
            if path_contains(&p, "test") || path_contains(&p, "example") {
                  continue;
            }
            let (m, is_test) = non_test_lines(&read(&p));
            for l in 0..m.nlines() {
                  if !is_test[l] && has_bare_println(&m.without_comments(l)) {
                        hits.push(format!("{}:{}", rel(&p), l + 1));
                  }
            }
      }
      assert_absent("println in library", &hits);
}

#[test]
fn h07_ascii_only() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            for (i, l) in read(&p).lines().enumerate() {
                  if let Some(c) = l.chars().find(|c| is_decorative(*c)) {
                        hits.push(format!("{}:{}: U+{:04X} {c}", rel(&p), i + 1, c as u32));
                  }
            }
      }
      assert_absent("unicode decoration", &hits);
}

#[test]
fn h08_no_hardcoded_home_path() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            if path_contains(&p, "test") {
                  continue;
            }
            let m = mark(&read(&p));
            for l in 0..m.nlines() {
                  let line = m.without_comments(l);
                  for pat in HOME_PATHS {
                        if line.contains(pat) {
                              hits.push(format!("{}:{}: {pat}", rel(&p), l + 1));
                        }
                  }
            }
      }
      assert_absent("hardcoded home path", &hits);
}

#[test]
fn h09_dead_import_report() {
      let mut uses = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            for (i, l) in read(&p).lines().enumerate() {
                  if l.starts_with("use ") {
                        uses.push(format!("{}:{}: {}", rel(&p), i + 1, l.trim()));
                  }
            }
      }
      for u in uses.iter().take(50) {
            eprintln!("{u}");
      }
      eprintln!("WARN: check above for dead imports (manual review)");
      assert!(!uses.is_empty(), "no `use` lines found: scanner is broken");
}

#[test]
fn h11_no_global_mutable_state() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            for (i, l) in read(&p).lines().enumerate() {
                  for pat in GLOBAL_MUT_PATTERNS {
                        if l.contains(pat) {
                              hits.push(format!("{}:{}: {pat}", rel(&p), i + 1));
                        }
                  }
            }
      }
      assert_absent("global mutable state", &hits);
}

#[test]
fn h12_no_assertion_free_test() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            let src = read(&p);
            let lines: Vec<&str> = src.lines().collect();
            let m = mark(&src);
            let code: Vec<String> = (0..m.nlines()).map(|l| m.code_only(l)).collect();
            for (i, l) in code.iter().enumerate() {
                  if !l.trim_start().starts_with("#[test]") {
                        continue;
                  }
                  let Some((name, body)) = test_fn_body(&lines, &code, i) else { continue };
                  if !ASSERTION_KEYWORDS.iter().any(|k| body.contains(k)) {
                        hits.push(format!(
                              "{}:{}: test with no assertions: {name}",
                              rel(&p),
                              i + 1
                        ));
                  }
            }
      }
      assert_absent("assertion-free test", &hits);
}

#[test]
fn h13_builder_takes_owned_self() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            if path_contains(&p, "test") {
                  continue;
            }
            for (i, l) in read(&p).lines().enumerate() {
                  if !l.contains("&mut self") {
                        continue;
                  }
                  if l.contains("-> &mut Self") || returns_mut_type(l) {
                        hits.push(format!("{}:{}: {}", rel(&p), i + 1, l.trim()));
                  }
            }
      }
      assert_absent("&mut self builder (use owned self -> Self)", &hits);
}

#[test]
fn h14_no_file_over_2000_lines() {
      let mut hits = Vec::new();
      for p in rs_files(SRC_ROOTS) {
            let n = read(&p).lines().count();
            if n > MAX_FILE_LINES {
                  hits.push(format!("{}: {n} lines", rel(&p)));
            }
      }
      assert_absent("file too long", &hits);
}

#[test]
fn h15_spy_counter_count() {
      let src = read(&Path::new(ROOT).join(SPY));
      let decl = src.lines().find_map(|l| l.trim().strip_prefix(SPY_DECL));
      let decl = decl.unwrap_or_else(|| panic!("{SPY}: `{SPY_DECL}` not found"));
      let n: usize = decl
            .trim_end_matches(';')
            .trim()
            .parse()
            .unwrap_or_else(|e| panic!("{SPY}: N is not a number: {e}"));
      assert_eq!(n, SPY_COUNTERS, "FAIL: callspy N={n}, expected {SPY_COUNTERS}");
}

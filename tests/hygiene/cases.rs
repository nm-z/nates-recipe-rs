use std::fs;
use std::path::Path;
use std::path::PathBuf;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

const SRC_ROOTS: &[&str] = &[
	"src",
	"recipe-infer/src",
	"pantry/src",
	"vramspy/src",
	"gpu-core/src",
	"log/src",
	"ogdl/src",
];

const BUILD_SCRIPTS: &[&str] = &["build.rs", "gpu-core/build.rs"];

enum Ev {
	Open,
	Close,
	Fun,
	Unsafe,
	Code,
	Doc(usize),
	Sep,
	Block(usize),
	Line(usize),
	Safety(usize),
}

struct Scope {
	is_unsafe: bool,
	safety: usize,
}

fn raw_hashes(ch: &[char], i: usize) -> Option<usize> {
	if i > 0 && (ch[i - 1].is_alphanumeric() || ch[i - 1] == '_') {
		return None;
	}
	let mut h = 0usize;
	while ch.get(i + 1 + h) == Some(&'#') {
		h += 1;
	}
	if ch.get(i + 1 + h) == Some(&'"') {
		return Some(h);
	}
	return None;
}

fn word_ev(w: &str) -> Ev {
	if w == "fn" {
		return Ev::Fun;
	}
	if w == "unsafe" {
		return Ev::Unsafe;
	}
	return Ev::Code;
}

fn classify(c: &[char], line: usize) -> Ev {
	let tail: String = c.iter().copied().skip(2).collect();
	if tail.starts_with('!') {
		return Ev::Doc(line);
	}
	if tail.starts_with('/') && !tail.starts_with("//") {
		return Ev::Doc(line);
	}
	if tail.trim_start().starts_with("SAFETY:") {
		return Ev::Safety(line);
	}
	let t = tail.trim();
	if t.starts_with('─') && t.ends_with('─') {
		return Ev::Sep;
	}
	return Ev::Line(line);
}

fn mark(src: &str) -> Vec<Ev> {
	let ch: Vec<char> = src.chars().collect();
	let n = ch.len();
	let mut evs = Vec::new();
	let mut word = String::new();
	let mut i = 0usize;
	let mut line = 1usize;
	while i < n {
		let c = ch[i];
		let raw = c == 'r' && raw_hashes(&ch, i).is_some();
		let is_id = (c.is_ascii_alphanumeric() || c == '_') && !raw;
		if !is_id && !word.is_empty() {
			evs.push(word_ev(&word));
			word.clear();
		}
		if is_id {
			word.push(c);
			i += 1;
			continue;
		}
		if raw {
			let h = raw_hashes(&ch, i).unwrap_or(0);
			i += h + 2;
			while i < n {
				if ch[i] == '\n' {
					line += 1;
				}
				if ch[i] == '"' && (1..=h).all(|k| ch.get(i + k) == Some(&'#')) {
					i += h + 1;
					break;
				}
				i += 1;
			}
			continue;
		}
		if c == '/' && ch.get(i + 1) == Some(&'/') {
			let mut j = i + 2;
			while j < n && ch[j] != '\n' {
				j += 1;
			}
			evs.push(classify(&ch[i..j], line));
			i = j;
			continue;
		}
		if c == '/' && ch.get(i + 1) == Some(&'*') {
			evs.push(Ev::Block(line));
			i += 2;
			let mut depth = 1usize;
			while i < n && depth > 0 {
				if ch[i] == '\n' {
					line += 1;
					i += 1;
				} else if ch[i] == '/' && ch.get(i + 1) == Some(&'*') {
					depth += 1;
					i += 2;
				} else if ch[i] == '*' && ch.get(i + 1) == Some(&'/') {
					depth -= 1;
					i += 2;
				} else {
					i += 1;
				}
			}
			continue;
		}
		if c == '"' {
			i += 1;
			while i < n {
				if ch[i] == '\\' {
					i += 2;
					continue;
				}
				if ch[i] == '\n' {
					line += 1;
				}
				if ch[i] == '"' {
					i += 1;
					break;
				}
				i += 1;
			}
			continue;
		}
		if c == '\'' && (ch.get(i + 1) == Some(&'\\') || ch.get(i + 2) == Some(&'\'')) {
			i += 1;
			while i < n {
				if ch[i] == '\\' {
					i += 2;
					continue;
				}
				if ch[i] == '\'' {
					i += 1;
					break;
				}
				i += 1;
			}
			continue;
		}
		match c {
			'\n' => {
				line += 1;
			}
			'{' => {
				evs.push(Ev::Open);
			}
			'}' => {
				evs.push(Ev::Close);
			}
			_ => {
				if !c.is_whitespace() {
					evs.push(Ev::Code);
				}
			}
		}
		i += 1;
	}
	if !word.is_empty() {
		evs.push(word_ev(&word));
	}
	return evs;
}

fn scan(name: &str, src: &str) -> Vec<String> {
	let mut out = Vec::new();
	let mut stack: Vec<Scope> = Vec::new();
	let mut pending: Vec<(usize, usize)> = Vec::new();
	let mut cand_unsafe = false;
	for ev in mark(src) {
		match ev {
			Ev::Fun => {
				cand_unsafe = false;
			}
			Ev::Unsafe => {
				cand_unsafe = true;
				let d = stack.len();
				let mut lines = Vec::new();
				let mut kept = Vec::new();
				for pr in &pending {
					if pr.1 == d {
						lines.push(pr.0);
					} else {
						kept.push(*pr);
					}
				}
				pending = kept;
				for ln in lines.iter().skip(1) {
					out.push(format!(
						"{name}:{ln}: extra // SAFETY: comment (budget 1 per unsafe construct)"
					));
				}
			}
			Ev::Open => {
				stack.push(Scope {
					is_unsafe: cand_unsafe,
					safety: 0,
				});
				cand_unsafe = false;
			}
			Ev::Close => {
				if !stack.is_empty() {
					stack.truncate(stack.len() - 1);
				}
				let d = stack.len();
				let mut kept = Vec::new();
				for pr in &pending {
					if pr.1 > d {
						out.push(format!(
							"{name}:{}: // SAFETY: comment not attached to an unsafe construct",
							pr.0
						));
					} else {
						kept.push(*pr);
					}
				}
				pending = kept;
				cand_unsafe = false;
			}
			Ev::Code => {
				cand_unsafe = false;
			}
			Ev::Doc(ln) => {
				out.push(format!("{name}:{ln}: doc comment (banned)"));
			}
			Ev::Sep => {}
			Ev::Block(ln) => {
				out.push(format!("{name}:{ln}: block comment (banned)"));
			}
			Ev::Safety(ln) => {
				let d = stack.len();
				if d > 0 && stack[d - 1].is_unsafe {
					stack[d - 1].safety += 1;
					if stack[d - 1].safety > 1 {
						out.push(format!(
							"{name}:{ln}: extra // SAFETY: comment (budget 1 per unsafe block)"
						));
					}
				} else {
					pending.push((ln, d));
				}
			}
			Ev::Line(ln) => {
				out.push(format!(
					"{name}:{ln}: // comment (banned; only // SAFETY: and // ─ separators)"
				));
			}
		}
	}
	for pr in &pending {
		out.push(format!(
			"{name}:{}: // SAFETY: comment not attached to an unsafe construct",
			pr.0
		));
	}
	return out;
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
	if dir.is_symlink() {
		return;
	}
	if dir.is_file() {
		if dir.extension().is_some_and(|e| e == "rs") {
			out.push(dir.to_path_buf());
		}
		return;
	}
	let Ok(rd) = fs::read_dir(dir) else {
		return;
	};
	for e in rd.flatten() {
		walk(&e.path(), out);
	}
	return;
}

#[test]
fn h03_comment_ban() {
	let clean = "fn f() {\n\tlet s = \"// not a comment /* nope */\";\n\treturn;\n}\n";
	assert!(
		scan("self/clean.rs", clean).is_empty(),
		"clean sample was flagged"
	);
	let dirty = "fn f() {\n\t// one\n\t// two\n\t/* block */\n}\n";
	assert!(
		!scan("self/dirty.rs", dirty).is_empty(),
		"dirty sample was not flagged"
	);
	let doc = "/// yap\nfn f() {\n\treturn;\n}\n";
	assert!(
		!scan("self/doc.rs", doc).is_empty(),
		"doc comment was not flagged"
	);
	let sep = "fn f() {\n\t// ── stage ──\n\t// ─\n\treturn;\n}\n";
	assert!(
		scan("self/sep.rs", sep).is_empty(),
		"separator lines were flagged"
	);
	assert!(
		matches!(classify(&['/', '/', '/', ' ', 'x'], 1), Ev::Doc(1)),
		"/// must classify as a doc comment"
	);
	let mut leaked = 0usize;
	for e in mark("let s = \"// x /* y */\";") {
		match e {
			Ev::Doc(_) | Ev::Line(_) | Ev::Block(_) | Ev::Safety(_) => {
				leaked += 1;
			}
			_ => {}
		}
	}
	assert_eq!(
		leaked, 0,
		"lexer leaked comment tokens out of a string literal"
	);

	let mut files = Vec::new();
	for r in SRC_ROOTS {
		walk(&Path::new(ROOT).join(r), &mut files);
	}
	for b in BUILD_SCRIPTS {
		let p = Path::new(ROOT).join(b);
		if p.is_file() {
			files.push(p);
		}
	}
	files.sort();

	let mut hits = Vec::new();
	for p in &files {
		let name = p
			.strip_prefix(ROOT)
			.unwrap_or_else(|_| p.as_path())
			.display()
			.to_string();
		let src = fs::read_to_string(p).expect("read source file");
		hits.extend(scan(&name, &src));
	}
	assert!(
		hits.is_empty(),
		"comment-budget violations:\n  {}",
		hits.join("\n  ")
	);
	return;
}

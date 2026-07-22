
use std::fs;
use std::path::{Path, PathBuf};

fn source_files(root: &Path, out: &mut Vec<PathBuf>) {
	let entries = fs::read_dir(root).unwrap_or_else(|e| panic!("read_dir {}: {e}", root.display()));
	for entry in entries {
		let path = entry.expect("dir entry").path();
		if path.is_dir() {
			source_files(&path, out);
		} else {
			out.push(path);
		}
	}
}

fn line_of(text: &str, byte: usize) -> usize {
	return text[..byte].bytes().filter(|&b| b == b'\n').count() + 1;
}

fn has_none_token(span: &str) -> bool {
	let bytes = span.as_bytes();
	let ident = |b: u8| return b == b'_' || b.is_ascii_alphanumeric();
	let mut from = 0;
	while let Some(pos) = span[from..].find("None") {
		let start = from + pos;
		let before_ok = start == 0 || !ident(bytes[start - 1]);
		let after = start + 4;
		let after_ok = after >= bytes.len() || !ident(bytes[after]);
		if before_ok && after_ok {
			return true;
		}
		from = after;
	}
	return false;
}

fn paren_span(text: &str, open: usize) -> Option<(usize, usize)> {
	let mut depth = 0usize;
	for (off, ch) in text[open..].char_indices() {
		if ch == '(' {
			depth += 1;
		} else if ch == ')' {
			depth -= 1;
			if depth == 0 {
				return Some((open, open + off + 1));
			}
		}
	}
	return None;
}

#[test]
fn no_legacy_decode_symbols_in_sources() {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
	let roots = [manifest.join("recipe-infer/src"), manifest.join("gpu-core/src")];
	let banned = ["decode_scan", "has_scan_op", "gpu_gqa_attn", "gpu_mla_attn"];
	let mut files = Vec::new();
	for root in &roots {
		source_files(root, &mut files);
	}
	files.sort();
	assert!(!files.is_empty(), "no source files found under {roots:?}");
	let mut violations: Vec<String> = Vec::new();
	for path in &files {
		let Ok(text) = fs::read_to_string(path) else {
			continue;
		};
		for sym in banned {
			let mut from = 0;
			while let Some(pos) = text[from..].find(sym) {
				let at = from + pos;
				violations.push(format!(
					"{}:{}: legacy symbol `{sym}` still present",
					path.display(),
					line_of(&text, at)
				));
				from = at + sym.len();
			}
		}
		let call = "attn_block(";
		let mut from = 0;
		while let Some(pos) = text[from..].find(call) {
			let at = from + pos;
			let open = at + call.len() - 1;
			match paren_span(&text, open) {
				Some((s, e)) if has_none_token(&text[s..e]) => {
					violations.push(format!(
						"{}:{}: attn_block call still passes a literal None decode context",
						path.display(),
						line_of(&text, at)
					));
				}
				Some(_) => {}
				None => {
					violations.push(format!(
						"{}:{}: attn_block call with unbalanced parens (scan could not verify it)",
						path.display(),
						line_of(&text, at)
					));
				}
			}
			from = open + 1;
		}
	}
	eprintln!(
		"scanned {} source files under recipe-infer/src + gpu-core/src",
		files.len()
	);
	assert!(
		violations.is_empty(),
		"legacy decode surface not fully deleted ({} violations):\n{}",
		violations.len(),
		violations.join("\n")
	);
}

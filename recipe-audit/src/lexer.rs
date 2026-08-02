use crate::SourceKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LexemeKind {
	Identifier,
	String,
	At,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Lexeme {
	pub kind: LexemeKind,
	pub text: String,
	pub line: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LexError {
	pub line: u64,
	pub reason: &'static str,
}

pub(crate) fn lex(contents: &str, kind: SourceKind) -> Result<Vec<Lexeme>, LexError> {
	let bytes = contents.as_bytes();
	let mut tokens = Vec::new();
	let mut index = 0;
	let mut line = 1_u64;

	while index < bytes.len() {
		match bytes[index] {
			b'\n' => {
				line += 1;
				index += 1;
			}
			b'/' if bytes.get(index + 1) == Some(&b'/') => {
				index = skip_line_comment(bytes, index + 2);
			}
			b'/' if bytes.get(index + 1) == Some(&b'*') => {
				let start_line = line;
				(index, line) = skip_block_comment(bytes, index + 2, line).ok_or(LexError {
					line: start_line,
					reason: "unterminated block comment",
				})?;
			}
			b';' if kind == SourceKind::LlvmIr => {
				index = skip_line_comment(bytes, index + 1);
			}
			b'#' if kind == SourceKind::BuildMetadata => {
				index = skip_line_comment(bytes, index + 1);
			}
			b'@' => {
				tokens.push(Lexeme {
					kind: LexemeKind::At,
					text: "@".into(),
					line,
				});
				index += 1;
			}
			b'"' => {
				let start_line = line;
				let (end, end_line) = normal_string_end(bytes, index + 1, line, b'"').ok_or(LexError {
					line: start_line,
					reason: "unterminated string literal",
				})?;
				tokens.push(Lexeme {
					kind: LexemeKind::String,
					text: contents[index + 1..end - 1].to_owned(),
					line: start_line,
				});
				index = end;
				line = end_line;
			}
			b'\'' if kind != SourceKind::BuildMetadata => {
				if kind == SourceKind::Rust
					&& let Some(end) = rust_lifetime_end(bytes, index)
				{
					index = end;
					continue;
				}
				let start_line = line;
				(index, line) = normal_string_end(bytes, index + 1, line, b'\'').ok_or(LexError {
					line: start_line,
					reason: "unterminated character literal",
				})?;
			}
			b'r' if matches!(kind, SourceKind::Rust) => {
				if let Some((content_start, hashes)) = rust_raw_string_start(bytes, index) {
					let start_line = line;
					let (end, end_line, content_end) =
						rust_raw_string_end(bytes, content_start, hashes, line).ok_or(LexError {
							line: start_line,
							reason: "unterminated raw string literal",
						})?;
					tokens.push(Lexeme {
						kind: LexemeKind::String,
						text: contents[content_start..content_end].to_owned(),
						line: start_line,
					});
					index = end;
					line = end_line;
				} else {
					index = push_identifier(contents, bytes, index, line, &mut tokens);
				}
			}
			byte if is_identifier_start(byte) => {
				index = push_identifier(contents, bytes, index, line, &mut tokens);
			}
			_ => index += 1,
		}
	}

	Ok(tokens)
}

fn push_identifier(contents: &str, bytes: &[u8], start: usize, line: u64, tokens: &mut Vec<Lexeme>) -> usize {
	let mut end = start + 1;
	while end < bytes.len() && is_identifier_continue(bytes[end]) {
		end += 1;
	}
	tokens.push(Lexeme {
		kind: LexemeKind::Identifier,
		text: contents[start..end].to_owned(),
		line,
	});
	end
}

fn is_identifier_start(byte: u8) -> bool { byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'$' | b'.') }

fn is_identifier_continue(byte: u8) -> bool {
	byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$' | b'.' | b'-')
}

fn skip_line_comment(bytes: &[u8], mut index: usize) -> usize {
	while index < bytes.len() && bytes[index] != b'\n' {
		index += 1;
	}
	index
}

fn skip_block_comment(bytes: &[u8], mut index: usize, mut line: u64) -> Option<(usize, u64)> {
	let mut depth = 1_u32;
	while index < bytes.len() {
		if bytes[index] == b'\n' {
			line += 1;
			index += 1;
		} else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
			depth = depth.checked_add(1)?;
			index += 2;
		} else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
			depth -= 1;
			index += 2;
			if depth == 0 {
				return Some((index, line));
			}
		} else {
			index += 1;
		}
	}
	None
}

fn normal_string_end(bytes: &[u8], mut index: usize, mut line: u64, delimiter: u8) -> Option<(usize, u64)> {
	while index < bytes.len() {
		match bytes[index] {
			b'\\' => index = index.checked_add(2)?,
			byte if byte == delimiter => return Some((index + 1, line)),
			b'\n' => {
				line += 1;
				index += 1;
			}
			_ => index += 1,
		}
	}
	None
}

fn rust_raw_string_start(bytes: &[u8], index: usize) -> Option<(usize, usize)> {
	if bytes.get(index) != Some(&b'r') {
		return None;
	}
	let mut cursor = index + 1;
	let mut hashes = 0;
	while bytes.get(cursor) == Some(&b'#') {
		hashes += 1;
		cursor += 1;
	}
	(bytes.get(cursor) == Some(&b'"')).then_some((cursor + 1, hashes))
}

fn rust_lifetime_end(bytes: &[u8], apostrophe: usize) -> Option<usize> {
	let mut cursor = apostrophe.checked_add(1)?;
	let first = *bytes.get(cursor)?;
	if !first.is_ascii_alphabetic() && first != b'_' {
		return None;
	}
	cursor += 1;
	while bytes
		.get(cursor)
		.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
	{
		cursor += 1;
	}
	// A one-codepoint character such as `'x'` is not a lifetime. Longer
	// quoted identifiers are invalid Rust and should still fail closed through
	// the normal character-literal path.
	(bytes.get(cursor) != Some(&b'\'')).then_some(cursor)
}

fn rust_raw_string_end(bytes: &[u8], mut index: usize, hashes: usize, mut line: u64) -> Option<(usize, u64, usize)> {
	while index < bytes.len() {
		if bytes[index] == b'\n' {
			line += 1;
			index += 1;
			continue;
		}
		if bytes[index] == b'"'
			&& bytes
				.get(index + 1..index + 1 + hashes)
				.is_some_and(|candidate| candidate.iter().all(|byte| *byte == b'#'))
		{
			return Some((index + 1 + hashes, line, index));
		}
		index += 1;
	}
	None
}

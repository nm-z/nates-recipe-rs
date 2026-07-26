use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use serde_json::{Map, Value};
use syn::visit::{self, Visit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArityRule {
	EmptyData,
	KnnReduction,
	ResidualBranch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MethodCallLocation {
	method_start: usize,
	method_end: usize,
	open_end: usize,
	close_start: usize,
	argument_count: usize,
}

#[derive(Default)]
struct MethodCallVisitor {
	calls: Vec<MethodCallLocation>,
}

impl<'ast> Visit<'ast> for MethodCallVisitor {
	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		let method = call.method.span().byte_range();
		let open = call.paren_token.span.open().byte_range();
		let close = call.paren_token.span.close().byte_range();
		self.calls.push(MethodCallLocation {
			method_start: method.start,
			method_end: method.end,
			open_end: open.end,
			close_start: close.start,
			argument_count: call.args.len(),
		});
		visit::visit_expr_method_call(self, call);
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Insertion {
	original_offset: usize,
	generated_start: usize,
	length: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceRewrite {
	original: String,
	generated: String,
	insertions: Vec<Insertion>,
	line_starts: Vec<usize>,
}

impl SourceRewrite {
	pub(crate) fn generated(&self) -> &str {
		&self.generated
	}

	fn generated_to_original(&self, offset: usize) -> usize {
		let mut inserted = 0usize;
		for insertion in &self.insertions {
			let start = insertion.generated_start;
			let end = start.saturating_add(insertion.length);
			if offset < start {
				break;
			}
			if offset <= end {
				return insertion.original_offset;
			}
			inserted = inserted.saturating_add(insertion.length);
		}
		offset.saturating_sub(inserted).min(self.original.len())
	}

	fn line_column(&self, offset: usize) -> (usize, usize) {
		let offset = offset.min(self.original.len());
		let line_index = self
			.line_starts
			.partition_point(|start| *start <= offset)
			.saturating_sub(1);
		let line_start = self.line_starts[line_index];
		let column = self.original[line_start..offset].chars().count() + 1;
		(line_index + 1, column)
	}

	fn line(&self, one_based: usize) -> &str {
		let index = one_based.saturating_sub(1);
		let Some(start) = self.line_starts.get(index).copied() else {
			return "";
		};
		let end = self
			.line_starts
			.get(index + 1)
			.copied()
			.unwrap_or(self.original.len());
		let line = self.original[start..end]
			.strip_suffix('\n')
			.unwrap_or(&self.original[start..end]);
		line.strip_suffix('\r').unwrap_or(line)
	}
}

#[derive(Clone, Debug)]
enum DiagnosticEntry {
	Json(Value),
	Raw(String),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct DiagnosticStream {
	entries: Vec<DiagnosticEntry>,
}

impl DiagnosticStream {
	pub(crate) fn parse(bytes: &[u8]) -> Self {
		let mut entries = Vec::new();
		for line in bytes.split_inclusive(|byte| *byte == b'\n') {
			let without_newline = line.strip_suffix(b"\n").unwrap_or(line);
			match serde_json::from_slice::<Value>(without_newline) {
				Ok(value) if value.get("$message_type").and_then(Value::as_str) == Some("diagnostic") => {
					entries.push(DiagnosticEntry::Json(value));
				}
				_ => entries.push(DiagnosticEntry::Raw(
					String::from_utf8_lossy(line).into_owned(),
				)),
			}
		}
		Self { entries }
	}

	pub(crate) fn original_rendering(&self) -> String {
		let mut output = String::new();
		for entry in &self.entries {
			match entry {
				DiagnosticEntry::Json(value) => {
					if let Some(rendered) = value.get("rendered").and_then(Value::as_str) {
						output.push_str(rendered);
					} else {
						render_diagnostic(value, &mut output);
					}
				}
				DiagnosticEntry::Raw(raw) => output.push_str(raw),
			}
		}
		output
	}

	pub(crate) fn mapped_rendering(
		&self,
		rewrite: &SourceRewrite,
		compiler_source: &Path,
		original_source: &Path,
	) -> String {
		let mut output = String::new();
		let compiler = compiler_source.to_string_lossy();
		let original = original_source.to_string_lossy();
		for entry in &self.entries {
			match entry {
				DiagnosticEntry::Json(value) => {
					let mut value = value.clone();
					remap_diagnostic(&mut value, rewrite, &compiler, &original);
					render_diagnostic(&value, &mut output);
				}
				DiagnosticEntry::Raw(raw) => {
					output.push_str(&raw.replace(compiler.as_ref(), original.as_ref()))
				}
			}
		}
		output
	}

	fn diagnostics(&self) -> impl Iterator<Item = &Value> {
		self.entries.iter().filter_map(|entry| match entry {
			DiagnosticEntry::Json(value) => Some(value),
			DiagnosticEntry::Raw(_) => None,
		})
	}
}

pub(crate) fn arity_rewrite(source_path: &Path, source: &str, diagnostics: &DiagnosticStream) -> Option<SourceRewrite> {
	let syntax = syn::parse_file(source).ok()?;
	let mut visitor = MethodCallVisitor::default();
	visitor.visit_file(&syntax);

	let source_name = source_path.to_string_lossy();
	let mut edits = BTreeMap::<usize, &'static str>::new();
	for diagnostic in diagnostics.diagnostics() {
		let Some(rule) = recipe_arity_rule(diagnostic) else {
			continue;
		};
		let Some(primary) = primary_source_span(diagnostic, &source_name) else {
			continue;
		};
		let start = usize::try_from(primary.get("byte_start")?.as_u64()?).ok()?;
		let end = usize::try_from(primary.get("byte_end")?.as_u64()?).ok()?;
		let call = visitor
			.calls
			.iter()
			.find(|call| call.method_start == start && call.method_end == end)?;
		match rule {
			ArityRule::EmptyData if call.argument_count == 0 => {
				insert_edit(&mut edits, call.open_end, "()")?;
			}
			ArityRule::KnnReduction if call.argument_count == 2 => {
				insert_edit(&mut edits, call.open_end, "[")?;
				insert_edit(&mut edits, call.close_start, "]")?;
			}
			ArityRule::ResidualBranch if call.argument_count >= 2 => {
				insert_edit(&mut edits, call.open_end, "[")?;
				insert_edit(&mut edits, call.close_start, "]")?;
			}
			_ => return None,
		}
	}
	if edits.is_empty() {
		return None;
	}

	let mut generated = String::with_capacity(source.len() + edits.values().map(|edit| edit.len()).sum::<usize>());
	let mut original_cursor = 0usize;
	let mut generated_inserted = 0usize;
	let mut insertions = Vec::with_capacity(edits.len());
	for (offset, edit) in edits {
		if offset < original_cursor || offset > source.len() || !source.is_char_boundary(offset) {
			return None;
		}
		generated.push_str(&source[original_cursor..offset]);
		let generated_start = offset + generated_inserted;
		generated.push_str(edit);
		insertions.push(Insertion {
			original_offset: offset,
			generated_start,
			length: edit.len(),
		});
		generated_inserted += edit.len();
		original_cursor = offset;
	}
	generated.push_str(&source[original_cursor..]);

	let mut line_starts = vec![0];
	line_starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
	Some(SourceRewrite {
		original: source.to_owned(),
		generated,
		insertions,
		line_starts,
	})
}

fn insert_edit(edits: &mut BTreeMap<usize, &'static str>, offset: usize, edit: &'static str) -> Option<()> {
	match edits.get(&offset) {
		Some(existing) if *existing != edit => None,
		Some(_) => Some(()),
		None => {
			edits.insert(offset, edit);
			Some(())
		}
	}
}

fn recipe_arity_rule(diagnostic: &Value) -> Option<ArityRule> {
	if diagnostic.pointer("/code/code").and_then(Value::as_str) != Some("E0061") {
		return None;
	}
	if method_defined_here(diagnostic, "src/facade.rs", "fn data", "IntoDataSources") {
		Some(ArityRule::EmptyData)
	} else if method_defined_here(diagnostic, "src/api.rs", "fn knn", "IntoKnnSpec") {
		Some(ArityRule::KnnReduction)
	} else if method_defined_here(
		diagnostic,
		"src/api.rs",
		"fn residual",
		"IntoResidualBranch",
	) {
		Some(ArityRule::ResidualBranch)
	} else {
		None
	}
}

fn method_defined_here(diagnostic: &Value, suffix: &str, method: &str, bound: &str) -> bool {
	diagnostic
		.get("children")
		.and_then(Value::as_array)
		.into_iter()
		.flatten()
		.filter(|child| child.get("message").and_then(Value::as_str) == Some("method defined here"))
		.flat_map(|child| {
			child.get("spans")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
		})
		.any(|span| {
			let file_matches = span
				.get("file_name")
				.and_then(Value::as_str)
				.is_some_and(|file| Path::new(file).ends_with(suffix));
			let signature_matches = span
				.get("text")
				.and_then(Value::as_array)
				.into_iter()
				.flatten()
				.filter_map(|line| line.get("text").and_then(Value::as_str))
				.any(|line| line.contains(method) && line.contains(bound));
			file_matches && signature_matches
		})
}

fn primary_source_span<'a>(diagnostic: &'a Value, source: &str) -> Option<&'a Value> {
	diagnostic.get("spans")?.as_array()?.iter().find(|span| {
		span.get("is_primary").and_then(Value::as_bool) == Some(true)
			&& span.get("file_name").and_then(Value::as_str) == Some(source)
	})
}

fn remap_diagnostic(diagnostic: &mut Value, rewrite: &SourceRewrite, compiler_source: &str, original_source: &str) {
	if let Some(message) = diagnostic
		.get("message")
		.and_then(Value::as_str)
		.map(str::to_owned)
	{
		diagnostic["message"] = Value::String(message.replace(compiler_source, original_source));
	}
	if let Some(spans) = diagnostic.get_mut("spans").and_then(Value::as_array_mut) {
		for span in spans {
			remap_span(span, rewrite, compiler_source, original_source);
		}
	}
	if let Some(children) = diagnostic.get_mut("children").and_then(Value::as_array_mut) {
		for child in children {
			remap_diagnostic(child, rewrite, compiler_source, original_source);
		}
	}
}

fn remap_span(span: &mut Value, rewrite: &SourceRewrite, compiler_source: &str, original_source: &str) {
	let Some(file) = span.get("file_name").and_then(Value::as_str) else {
		return;
	};
	if file != compiler_source && file != original_source {
		return;
	}
	let Some(generated_start) = span.get("byte_start").and_then(Value::as_u64) else {
		return;
	};
	let Some(generated_end) = span.get("byte_end").and_then(Value::as_u64) else {
		return;
	};
	let Ok(generated_start) = usize::try_from(generated_start) else {
		return;
	};
	let Ok(generated_end) = usize::try_from(generated_end) else {
		return;
	};
	let start = rewrite.generated_to_original(generated_start);
	let end = rewrite.generated_to_original(generated_end).max(start);
	let (line_start, column_start) = rewrite.line_column(start);
	let (line_end, column_end) = rewrite.line_column(end);

	let mut replacement = Map::new();
	if let Some(existing) = span.as_object() {
		replacement.extend(existing.clone());
	}
	replacement.insert(
		"file_name".to_owned(),
		Value::String(original_source.to_owned()),
	);
	replacement.insert("byte_start".to_owned(), Value::from(start));
	replacement.insert("byte_end".to_owned(), Value::from(end));
	replacement.insert("line_start".to_owned(), Value::from(line_start));
	replacement.insert("line_end".to_owned(), Value::from(line_end));
	replacement.insert("column_start".to_owned(), Value::from(column_start));
	replacement.insert("column_end".to_owned(), Value::from(column_end));
	replacement.insert(
		"text".to_owned(),
		Value::Array(
			(line_start..=line_end)
				.map(|line| {
					let text = rewrite.line(line);
					let width = text.chars().count() + 1;
					let highlight_start = if line == line_start { column_start } else { 1 };
					let highlight_end = if line == line_end { column_end } else { width };
					serde_json::json!({
						"text": text,
						"highlight_start": highlight_start,
						"highlight_end": highlight_end.max(highlight_start + 1).min(width.max(2)),
					})
				})
				.collect(),
		),
	);
	*span = Value::Object(replacement);
}

fn render_diagnostic(diagnostic: &Value, output: &mut String) {
	let level = diagnostic
		.get("level")
		.and_then(Value::as_str)
		.unwrap_or("error");
	let message = diagnostic
		.get("message")
		.and_then(Value::as_str)
		.unwrap_or("compiler diagnostic");
	let code = diagnostic.pointer("/code/code").and_then(Value::as_str);
	match code {
		Some(code) => {
			let _ = writeln!(output, "{level}[{code}]: {message}");
		}
		None if level == "failure-note" => {
			let _ = writeln!(output, "{message}");
		}
		None => {
			let _ = writeln!(output, "{level}: {message}");
		}
	}
	if let Some(span) = preferred_span(diagnostic) {
		render_span(span, output);
	}
	if let Some(children) = diagnostic.get("children").and_then(Value::as_array) {
		for child in children {
			render_diagnostic(child, output);
		}
	}
	output.push('\n');
}

fn preferred_span(diagnostic: &Value) -> Option<&Value> {
	let spans = diagnostic.get("spans")?.as_array()?;
	spans.iter()
		.find(|span| span.get("is_primary").and_then(Value::as_bool) == Some(true))
		.or_else(|| spans.first())
}

fn render_span(span: &Value, output: &mut String) {
	let file = span
		.get("file_name")
		.and_then(Value::as_str)
		.unwrap_or("<source>");
	let line = span.get("line_start").and_then(Value::as_u64).unwrap_or(1);
	let column = span
		.get("column_start")
		.and_then(Value::as_u64)
		.unwrap_or(1);
	let _ = writeln!(output, " --> {file}:{line}:{column}");
	let Some(lines) = span.get("text").and_then(Value::as_array) else {
		return;
	};
	for (offset, source) in lines.iter().enumerate() {
		let source_line = source.get("text").and_then(Value::as_str).unwrap_or("");
		let number = usize::try_from(line).unwrap_or(1) + offset;
		let start = source
			.get("highlight_start")
			.and_then(Value::as_u64)
			.and_then(|value| usize::try_from(value).ok())
			.unwrap_or(1);
		let end = source
			.get("highlight_end")
			.and_then(Value::as_u64)
			.and_then(|value| usize::try_from(value).ok())
			.unwrap_or(start + 1);
		let _ = writeln!(output, "{number:>4} | {source_line}");
		let marker = "^".repeat(end.saturating_sub(start).max(1));
		let label = span.get("label").and_then(Value::as_str).unwrap_or("");
		let _ = writeln!(
			output,
			"     | {}{marker} {label}",
			" ".repeat(start.saturating_sub(1))
		);
	}
}

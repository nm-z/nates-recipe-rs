use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use proc_macro2::{Spacing, TokenStream, TokenTree};
use serde_json::{Map, Value};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};
use syn::{Expr, Ident, Token};

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
struct RewriteRange {
	original_start: usize,
	original_end: usize,
	generated_start: usize,
	generated_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TextEdit {
	start: usize,
	end: usize,
	replacement: String,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceRewrite {
	original: String,
	generated: String,
	ranges: Vec<RewriteRange>,
	line_starts: Vec<usize>,
}

impl SourceRewrite {
	pub(crate) fn generated(&self) -> &str {
		&self.generated
	}

	fn generated_to_original(&self, offset: usize) -> usize {
		let mut original_cursor = 0usize;
		let mut generated_cursor = 0usize;
		for range in &self.ranges {
			if offset < range.generated_start {
				return original_cursor
					.saturating_add(offset.saturating_sub(generated_cursor))
					.min(range.original_start);
			}
			if offset <= range.generated_end {
				let original_length = range.original_end.saturating_sub(range.original_start);
				let generated_length = range.generated_end.saturating_sub(range.generated_start);
				if original_length == generated_length {
					return range
						.original_start
						.saturating_add(offset.saturating_sub(range.generated_start))
						.min(range.original_end);
				}
				return if offset == range.generated_end {
					range.original_end
				} else {
					range.original_start
				};
			}
			original_cursor = range.original_end;
			generated_cursor = range.generated_end;
		}
		original_cursor
			.saturating_add(offset.saturating_sub(generated_cursor))
			.min(self.original.len())
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
enum NamedGradShape {
	Exact {
		field_start: usize,
		field_end: usize,
		colon_start: usize,
		colon_end: usize,
	},
	Malformed(String),
}

#[derive(Clone, Debug)]
struct NamedGradCandidate {
	method_start: usize,
	method_end: usize,
	arguments_start: usize,
	arguments_end: usize,
	shape: NamedGradShape,
}

#[derive(Clone, Debug)]
pub(crate) struct NamedGradProbe {
	candidates: Vec<NamedGradCandidate>,
	rewrite: SourceRewrite,
}

impl NamedGradProbe {
	pub(crate) fn generated(&self) -> &str {
		self.rewrite.generated()
	}
}

struct NamedGradField {
	name: Ident,
	colon: Token![:],
	_value: Expr,
}

impl Parse for NamedGradField {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		Ok(Self {
			name: input.parse()?,
			colon: input.parse()?,
			_value: input.parse()?,
		})
	}
}

struct NamedGradArguments {
	fields: Punctuated<NamedGradField, Token![,]>,
}

impl Parse for NamedGradArguments {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		Ok(Self {
			fields: Punctuated::parse_terminated(input)?,
		})
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
		self.mapped_rendering_chain(&[(rewrite, compiler_source, original_source)])
	}

	pub(crate) fn mapped_rendering_chain(&self, rewrites: &[(&SourceRewrite, &Path, &Path)]) -> String {
		let mut output = String::new();
		for entry in &self.entries {
			match entry {
				DiagnosticEntry::Json(value) => {
					let mut value = value.clone();
					for (rewrite, compiler_source, original_source) in rewrites {
						let compiler = compiler_source.to_string_lossy();
						let original = original_source.to_string_lossy();
						remap_diagnostic(&mut value, rewrite, &compiler, &original);
					}
					render_diagnostic(&value, &mut output);
				}
				DiagnosticEntry::Raw(raw) => {
					let mut raw = raw.clone();
					for (_, compiler_source, original_source) in rewrites {
						raw = raw.replace(
							compiler_source.to_string_lossy().as_ref(),
							original_source.to_string_lossy().as_ref(),
						);
					}
					output.push_str(&raw);
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

/// Replaces only lexically named `.grad(...)` argument lists with a deliberate
/// two-argument call. Rustc can then prove which receiver resolves to
/// Recipe's `Model::grad` without accepting the non-Rust user syntax first.
pub(crate) fn named_grad_probe(source: &str) -> Option<NamedGradProbe> {
	let tokens = source.parse::<TokenStream>().ok()?;
	let mut candidates = Vec::new();
	collect_named_grad_candidates(tokens, &mut candidates);
	if candidates.is_empty() {
		return None;
	}

	let edits = candidates
		.iter()
		.map(|candidate| TextEdit {
			start: candidate.arguments_start,
			end: candidate.arguments_end,
			replacement: "(), ()".to_owned(),
		})
		.collect();
	let rewrite = build_rewrite(source, edits)?;
	Some(NamedGradProbe {
		candidates,
		rewrite,
	})
}

pub(crate) fn named_grad_rewrite(
	source_path: &Path,
	probe_path: &Path,
	probe: &NamedGradProbe,
	diagnostics: &DiagnosticStream,
) -> Result<Option<SourceRewrite>, String> {
	let probe_name = probe_path.to_string_lossy();
	let mut recipe_model_calls = Vec::new();
	for diagnostic in diagnostics.diagnostics() {
		if diagnostic.pointer("/code/code").and_then(Value::as_str) != Some("E0061")
			|| !method_defined_here(diagnostic, "src/api.rs", "fn grad", "gradient: Grad")
		{
			continue;
		}
		let Some(primary) = primary_source_span(diagnostic, &probe_name) else {
			continue;
		};
		let Some(start) = primary
			.get("byte_start")
			.and_then(Value::as_u64)
			.and_then(|value| usize::try_from(value).ok())
		else {
			continue;
		};
		let Some(end) = primary
			.get("byte_end")
			.and_then(Value::as_u64)
			.and_then(|value| usize::try_from(value).ok())
		else {
			continue;
		};
		let original_start = probe.rewrite.generated_to_original(start);
		let original_end = probe.rewrite.generated_to_original(end);
		if let Some(index) = probe.candidates.iter().position(|candidate| {
			(candidate.method_start == original_start && candidate.method_end == original_end)
				|| (original_start <= candidate.method_start && original_end >= candidate.method_end)
		}) {
			recipe_model_calls.push(index);
		}
	}
	recipe_model_calls.sort_unstable();
	recipe_model_calls.dedup();
	if recipe_model_calls.is_empty() {
		return Ok(None);
	}

	let mut edits = Vec::new();
	for index in recipe_model_calls {
		let candidate = &probe.candidates[index];
		match &candidate.shape {
			NamedGradShape::Exact {
				field_start,
				field_end,
				colon_start,
				colon_end,
			} => {
				edits.push(TextEdit {
					start: *field_start,
					end: *field_end,
					replacement: "::recipe::clip".to_owned(),
				});
				edits.push(TextEdit {
					start: *colon_start,
					end: *colon_end,
					replacement: "(".to_owned(),
				});
				edits.push(TextEdit {
					start: candidate.arguments_end,
					end: candidate.arguments_end,
					replacement: ")".to_owned(),
				});
			}
			NamedGradShape::Malformed(message) => {
				return Err(render_named_grad_error(
					source_path,
					&probe.rewrite,
					candidate,
					message,
				));
			}
		}
	}

	build_rewrite(&probe.rewrite.original, edits)
		.ok_or_else(|| "could not safely rewrite Recipe Model `.grad(clip: EXPR)` source".to_owned())
		.map(Some)
}

fn collect_named_grad_candidates(stream: TokenStream, candidates: &mut Vec<NamedGradCandidate>) {
	let tokens: Vec<_> = stream.into_iter().collect();
	for window in tokens.windows(3) {
		let [
			TokenTree::Punct(dot),
			TokenTree::Ident(method),
			TokenTree::Group(arguments),
		] = window
		else {
			continue;
		};
		if dot.as_char() != '.' || method != "grad" || arguments.delimiter() != proc_macro2::Delimiter::Parenthesis
		{
			continue;
		}
		let argument_tokens: Vec<_> = arguments.stream().into_iter().collect();
		let looks_named = matches!(
			argument_tokens.as_slice(),
			[TokenTree::Ident(_), TokenTree::Punct(colon), ..]
				if colon.as_char() == ':' && colon.spacing() == Spacing::Alone
		);
		if !looks_named {
			continue;
		}
		let method_span = method.span().byte_range();
		let open = arguments.span_open().byte_range();
		let close = arguments.span_close().byte_range();
		candidates.push(NamedGradCandidate {
			method_start: method_span.start,
			method_end: method_span.end,
			arguments_start: open.end,
			arguments_end: close.start,
			shape: classify_named_grad_arguments(arguments.stream()),
		});
	}
	for token in tokens {
		if let TokenTree::Group(group) = token {
			collect_named_grad_candidates(group.stream(), candidates);
		}
	}
}

fn classify_named_grad_arguments(tokens: TokenStream) -> NamedGradShape {
	let arguments = match syn::parse2::<NamedGradArguments>(tokens) {
		Ok(arguments) => arguments,
		Err(error) => {
			return NamedGradShape::Malformed(format!(
				"malformed Recipe Model `.grad(...)` fields: {error}; expected exactly `clip: EXPR`"
			));
		}
	};
	let fields: Vec<_> = arguments.fields.into_iter().collect();
	let clip_fields = fields.iter().filter(|field| field.name == "clip").count();
	if clip_fields > 1 {
		return NamedGradShape::Malformed(
			"duplicate named field `clip` in Recipe Model `.grad(...)`; declare `clip: EXPR` once".to_owned(),
		);
	}
	if let Some(field) = fields.iter().find(|field| field.name != "clip") {
		return NamedGradShape::Malformed(format!(
			"unknown named field `{}` in Recipe Model `.grad(...)`; expected `clip: EXPR`",
			field.name
		));
	}
	let [field] = fields.as_slice() else {
		return NamedGradShape::Malformed(
			"Recipe Model `.grad(...)` accepts exactly one named field: `clip: EXPR`".to_owned(),
		);
	};
	let name = field.name.span().byte_range();
	let colon = field.colon.span().byte_range();
	NamedGradShape::Exact {
		field_start: name.start,
		field_end: name.end,
		colon_start: colon.start,
		colon_end: colon.end,
	}
}

fn render_named_grad_error(
	source_path: &Path,
	rewrite: &SourceRewrite,
	candidate: &NamedGradCandidate,
	message: &str,
) -> String {
	let (line, column) = rewrite.line_column(candidate.arguments_start);
	let source_line = rewrite.line(line);
	let line_end = rewrite
		.line_starts
		.get(line)
		.copied()
		.unwrap_or(rewrite.original.len());
	let highlight_end = candidate.arguments_end.min(line_end);
	let highlight_width = rewrite.original[candidate.arguments_start..highlight_end]
		.chars()
		.count()
		.max(1);
	format!(
		"{message}\n --> {}:{line}:{column}\n{line:>4} | {source_line}\n     | {}{}",
		source_path.display(),
		" ".repeat(column.saturating_sub(1)),
		"^".repeat(highlight_width),
	)
}

fn build_rewrite(source: &str, mut edits: Vec<TextEdit>) -> Option<SourceRewrite> {
	edits.sort_by_key(|edit| (edit.start, edit.end));
	let additional = edits.iter().fold(0usize, |total, edit| {
		total.saturating_add(
			edit.replacement
				.len()
				.saturating_sub(edit.end.saturating_sub(edit.start)),
		)
	});
	let mut generated = String::with_capacity(source.len().saturating_add(additional));
	let mut ranges = Vec::with_capacity(edits.len());
	let mut original_cursor = 0usize;
	for edit in edits {
		if edit.start < original_cursor
			|| edit.end < edit.start
			|| edit.end > source.len()
			|| !source.is_char_boundary(edit.start)
			|| !source.is_char_boundary(edit.end)
		{
			return None;
		}
		generated.push_str(&source[original_cursor..edit.start]);
		let generated_start = generated.len();
		generated.push_str(&edit.replacement);
		let generated_end = generated.len();
		ranges.push(RewriteRange {
			original_start: edit.start,
			original_end: edit.end,
			generated_start,
			generated_end,
		});
		original_cursor = edit.end;
	}
	generated.push_str(&source[original_cursor..]);

	let mut line_starts = vec![0];
	line_starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
	Some(SourceRewrite {
		original: source.to_owned(),
		generated,
		ranges,
		line_starts,
	})
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
	build_rewrite(
		source,
		edits.into_iter()
			.map(|(offset, replacement)| TextEdit {
				start: offset,
				end: offset,
				replacement: replacement.to_owned(),
			})
			.collect(),
	)
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

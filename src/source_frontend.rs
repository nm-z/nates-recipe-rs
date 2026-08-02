use std::{collections::BTreeMap, fmt::Write as _, path::Path};

use proc_macro2::{Spacing, TokenStream, TokenTree};
use serde_json::{Map, Value};
use syn::{
	Expr, Ident, Token,
	parse::{Parse, ParseStream},
	punctuated::Punctuated,
	spanned::Spanned as _,
	visit::{self, Visit},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecipeReceiver {
	Facade,
	Data,
	Model,
	Train,
	Infer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MethodCallLocation {
	method: String,
	method_start: usize,
	method_end: usize,
	open_end: usize,
	close_start: usize,
	argument_count: usize,
	receiver: Option<RecipeReceiver>,
}

struct MethodCallVisitor<'a> {
	calls: Vec<MethodCallLocation>,
	bindings: &'a BTreeMap<String, RecipeReceiver>,
}

impl<'ast> Visit<'ast> for MethodCallVisitor<'_> {
	fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
		let method = call.method.span().byte_range();
		let open = call.paren_token.span.open().byte_range();
		let close = call.paren_token.span.close().byte_range();
		self.calls.push(MethodCallLocation {
			method: call.method.to_string(),
			method_start: method.start,
			method_end: method.end,
			open_end: open.end,
			close_start: close.start,
			argument_count: call.args.len(),
			receiver: classify_recipe_expression(&call.receiver, self.bindings),
		});
		visit::visit_expr_method_call(self, call);
	}
}

struct LocalBinding<'ast> {
	name: String,
	explicit: Option<RecipeReceiver>,
	initializer: Option<&'ast Expr>,
}

#[derive(Default)]
struct LocalBindingVisitor<'ast> {
	bindings: Vec<LocalBinding<'ast>>,
}

impl<'ast> Visit<'ast> for LocalBindingVisitor<'ast> {
	fn visit_local(&mut self, local: &'ast syn::Local) {
		let (pattern, explicit) = match &local.pat {
			syn::Pat::Type(typed) => (typed.pat.as_ref(), classify_recipe_type(&typed.ty)),
			pattern => (pattern, None),
		};
		if let syn::Pat::Ident(identifier) = pattern {
			self.bindings.push(LocalBinding {
				name: identifier.ident.to_string(),
				explicit,
				initializer: local.init.as_ref().map(|init| init.expr.as_ref()),
			});
		}
		visit::visit_local(self, local);
	}
}

fn classify_recipe_type(ty: &syn::Type) -> Option<RecipeReceiver> {
	let syn::Type::Path(path) = ty else {
		return None;
	};
	match path.path.segments.last()?.ident.to_string().as_str() {
		"Data" => Some(RecipeReceiver::Data),
		"Model" => Some(RecipeReceiver::Model),
		"Train" => Some(RecipeReceiver::Train),
		"Infer" => Some(RecipeReceiver::Infer),
		_ => None,
	}
}

fn collect_recipe_bindings(syntax: &syn::File) -> BTreeMap<String, RecipeReceiver> {
	let mut visitor = LocalBindingVisitor::default();
	visitor.visit_file(syntax);
	let mut resolved = BTreeMap::new();
	loop {
		let mut changed = false;
		for binding in &visitor.bindings {
			let kind = binding.explicit.or_else(|| {
				binding
					.initializer
					.and_then(|expression| classify_recipe_expression(expression, &resolved))
			});
			if let Some(kind) = kind
				&& resolved.get(&binding.name) != Some(&kind)
			{
				resolved.insert(binding.name.clone(), kind);
				changed = true;
			}
		}
		if !changed {
			return resolved;
		}
	}
}

fn classify_recipe_expression(
	expression: &Expr,
	bindings: &BTreeMap<String, RecipeReceiver>,
) -> Option<RecipeReceiver> {
	match expression {
		Expr::Path(path) => {
			let last = path.path.segments.last()?.ident.to_string();
			bindings
				.get(&last)
				.copied()
				.or_else(|| (last == "recipe").then_some(RecipeReceiver::Facade))
		}
		Expr::MethodCall(call) => {
			let receiver = classify_recipe_expression(&call.receiver, bindings)?;
			match receiver {
				RecipeReceiver::Facade => {
					match call.method.to_string().as_str() {
						"data" => Some(RecipeReceiver::Data),
						"model" => Some(RecipeReceiver::Model),
						"train" => Some(RecipeReceiver::Train),
						"infer" => Some(RecipeReceiver::Infer),
						_ => None,
					}
				}
				kind => Some(kind),
			}
		}
		Expr::Group(group) => classify_recipe_expression(&group.expr, bindings),
		Expr::Paren(parenthesized) => classify_recipe_expression(&parenthesized.expr, bindings),
		Expr::Reference(reference) => classify_recipe_expression(&reference.expr, bindings),
		_ => None,
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
	pub(crate) fn generated(&self) -> &str { &self.generated }

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
				_ => {
					entries.push(DiagnosticEntry::Raw(
						String::from_utf8_lossy(line).into_owned(),
					))
				}
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
}

/// Perform the complete Recipe-only syntax lowering before invoking rustc.
/// Receiver selection is derived from explicit Recipe facade chains and local
/// `Data`/`Model`/`Train`/`Infer` bindings; compiler diagnostics never control
/// parsing or trigger a retry.
pub(crate) fn lower_recipe_source(source_path: &Path, source: &str) -> Result<Option<SourceRewrite>, String> {
	let tokens = match source.parse::<TokenStream>() {
		Ok(tokens) => tokens,
		Err(_) => return Ok(None),
	};
	let mut named_grad_candidates = Vec::new();
	collect_named_grad_candidates(tokens, &mut named_grad_candidates);
	let probe_rewrite = if named_grad_candidates.is_empty() {
		None
	} else {
		Some(build_rewrite(
			source,
			named_grad_candidates
				.iter()
				.map(|candidate| {
					TextEdit {
						start: candidate.arguments_start,
						end: candidate.arguments_end,
						replacement: "::recipe::clip(1.0)".to_owned(),
					}
				})
				.collect(),
		)
		.ok_or_else(|| "could not construct the Recipe syntax-classification source".to_owned())?)
	};
	let classification_source = probe_rewrite
		.as_ref()
		.map_or(source, SourceRewrite::generated);
	let syntax = match syn::parse_file(classification_source) {
		Ok(syntax) => syntax,
		Err(_) => return Ok(None),
	};
	let bindings = collect_recipe_bindings(&syntax);
	let mut visitor = MethodCallVisitor {
		calls: Vec::new(),
		bindings: &bindings,
	};
	visitor.visit_file(&syntax);

	let to_original = |offset| {
		probe_rewrite
			.as_ref()
			.map_or(offset, |rewrite| rewrite.generated_to_original(offset))
	};
	let recipe_grad_calls = visitor
		.calls
		.iter()
		.filter(|call| call.method == "grad" && call.receiver == Some(RecipeReceiver::Model))
		.map(|call| (to_original(call.method_start), to_original(call.method_end)))
		.collect::<Vec<_>>();
	let mut edits = Vec::new();
	for candidate in &named_grad_candidates {
		if !recipe_grad_calls
			.iter()
			.any(|span| *span == (candidate.method_start, candidate.method_end))
		{
			continue;
		}
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
				let diagnostic_source = probe_rewrite
					.as_ref()
					.expect("named-gradient diagnostics require a classification rewrite");
				return Err(render_named_grad_error(
					source_path,
					diagnostic_source,
					candidate,
					message,
				));
			}
		}
	}

	for call in visitor.calls {
		let method_start = to_original(call.method_start);
		let method_end = to_original(call.method_end);
		let open_end = to_original(call.open_end);
		let close_start = to_original(call.close_start);
		match (call.receiver, call.method.as_str(), call.argument_count) {
			(Some(RecipeReceiver::Facade), "data", 0) => {
				edits.push(TextEdit {
					start: open_end,
					end: open_end,
					replacement: "()".to_owned(),
				})
			}
			(Some(RecipeReceiver::Model), "residual", 2..) => {
				edits.push(TextEdit {
					start: open_end,
					end: open_end,
					replacement: "[".to_owned(),
				});
				edits.push(TextEdit {
					start: close_start,
					end: close_start,
					replacement: "]".to_owned(),
				});
			}
			(Some(RecipeReceiver::Train), "save", 2) => {
				edits.push(TextEdit {
					start: method_start,
					end: method_end,
					replacement: "__recipe_save_pair".to_owned(),
				})
			}
			(Some(RecipeReceiver::Train), "resume", 2) => {
				edits.push(TextEdit {
					start: method_start,
					end: method_end,
					replacement: "__recipe_resume_pair".to_owned(),
				})
			}
			_ => {}
		}
	}
	if edits.is_empty() {
		return Ok(None);
	}
	build_rewrite(source, edits)
		.ok_or_else(|| "Recipe syntax edits overlap or address an invalid source boundary".to_owned())
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

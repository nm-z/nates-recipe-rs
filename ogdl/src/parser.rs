use crate::{Graph, ParseError, ParseErrorKind, SourceLocation};

pub(crate) fn parse(input: &str) -> Result<Graph, ParseError> { if input.is_empty() { return Ok(Graph::new()); }

	let bytes = input.as_bytes(); let mut graph = Graph::new(); let mut ancestors = Vec::new(); let mut line = 1;
	let mut line_start = 0; let mut cursor = 0;

	while cursor < bytes.len() { match bytes[cursor] {
			b'\r' if bytes.get(cursor + 1) != Some(&b'\n') => {
				let column = input[line_start..cursor].chars().count() + 1; return Err(ParseError::new(
					ParseErrorKind::BareCarriageReturn, SourceLocation::new(line, column, cursor), )); }
			b'\n' => {
				let line_end = if cursor > line_start && bytes[cursor - 1] == b'\r' {
					cursor - 1 } else { cursor }; parse_line( &input[line_start..line_end], line, line_start, &mut graph,
					&mut ancestors, )?; line += 1; line_start = cursor + 1; }
			_ => {} }
		cursor += 1; }

	if line_start < input.len() { parse_line( &input[line_start..], line, line_start, &mut graph, &mut ancestors, )?; }
	Ok(graph) }

fn parse_line( source: &str, line: usize, line_offset: usize, graph: &mut Graph, ancestors: &mut Vec<crate::NodeId>,
) -> Result<(), ParseError> {
	let indentation = source.bytes().take_while(|byte| *byte == b'\t').count();
	if indentation == source.len() { return Err(ParseError::new( ParseErrorKind::EmptyNode,
			SourceLocation::new(line, indentation + 1, line_offset + indentation), )); }
	if indentation > ancestors.len() { return Err(ParseError::new( ParseErrorKind::IndentationJump { found: indentation,
				maximum: ancestors.len(), }, SourceLocation::new(line, 1, line_offset), )); }

	let mut segments = Vec::new(); let mut segment_start = indentation;
	for (relative, character) in source[indentation..].char_indices() {
		if character != '\t' {
			continue; }
		let delimiter = indentation + relative; if delimiter == segment_start {
			return Err(empty_node_error(source, line, line_offset, delimiter)); }
		segments.push(&source[segment_start..delimiter]); segment_start = delimiter + 1; }
	if segment_start == source.len() { return Err(empty_node_error(source, line, line_offset, segment_start)); }
	segments.push(&source[segment_start..]);

	let mut parent = if indentation == 0 { None } else { Some(ancestors[indentation - 1]) };
	ancestors.truncate(indentation); for text in segments { let node = graph.push_node(parent, text.to_owned());
		ancestors.push(node); parent = Some(node); }
	Ok(()) }

fn empty_node_error(source: &str, line: usize, line_offset: usize, byte: usize) -> ParseError {
	let column = source[..byte].chars().count() + 1; ParseError::new( ParseErrorKind::EmptyNode,
		SourceLocation::new(line, column, line_offset + byte), ) }

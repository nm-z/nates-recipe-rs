use write::{Errored, err};

use crate::Graph;

pub(crate) fn parse(input: &str) -> Result<Graph, Errored> {
	if input.is_empty() {
		return Ok(Graph::new());
	}

	let bytes = input.as_bytes();
	let mut graph = Graph::new();
	let mut ancestors = Vec::new();
	let mut line = 1;
	let mut line_start = 0;
	let mut cursor = 0;

	while cursor < bytes.len() {
		match bytes[cursor] {
			b'\r' if bytes.get(cursor + 1) != Some(&b'\n') => {
				let column = input[line_start..cursor].chars().count() + 1;
				err(format!(
					"line {line}, column {column}: bare carriage return; use LF or CRLF"
				))?;
			}
			b'\n' => {
				let line_end = if cursor > line_start && bytes[cursor - 1] == b'\r' {
					cursor - 1
				} else {
					cursor
				};
				parse_line(
					&input[line_start..line_end],
					line,
					&mut graph,
					&mut ancestors,
				)?;
				line += 1;
				line_start = cursor + 1;
			}
			_ => {}
		}
		cursor += 1;
	}

	if line_start < input.len() {
		parse_line(&input[line_start..], line, &mut graph, &mut ancestors)?;
	}
	Ok(graph)
}

fn parse_line(source: &str, line: usize, graph: &mut Graph, ancestors: &mut Vec<crate::NodeId>) -> Result<(), Errored> {
	let indentation = source.bytes().take_while(|byte| *byte == b'\t').count();
	if indentation == source.len() {
		return err(format!(
			"line {line}, column {}: empty node",
			indentation + 1
		));
	}
	if indentation > ancestors.len() {
		return err(format!(
			"line {line}, column 1: indentation depth {indentation} skips an ancestor; maximum depth is {}",
			ancestors.len()
		));
	}

	let mut segments = Vec::new();
	let mut segment_start = indentation;
	for (relative, character) in source[indentation..].char_indices() {
		if character != '\t' {
			continue;
		}
		let delimiter = indentation + relative;
		if delimiter == segment_start {
			return empty_node_error(source, line, delimiter);
		}
		segments.push(&source[segment_start..delimiter]);
		segment_start = delimiter + 1;
	}
	if segment_start == source.len() {
		return empty_node_error(source, line, segment_start);
	}
	segments.push(&source[segment_start..]);

	let mut parent = if indentation == 0 {
		None
	} else {
		Some(ancestors[indentation - 1])
	};
	ancestors.truncate(indentation);
	for text in segments {
		let node = graph.push_node(parent, text.to_owned());
		ancestors.push(node);
		parent = Some(node);
	}
	Ok(())
}

fn empty_node_error(source: &str, line: usize, byte: usize) -> Result<(), Errored> {
	let column = source[..byte].chars().count() + 1;
	err(format!("line {line}, column {column}: empty node"))
}

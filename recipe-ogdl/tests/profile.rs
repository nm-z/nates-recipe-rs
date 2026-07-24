use recipe_ogdl::{Graph, GraphError, NodeId, NodeTextErrorKind, ParseErrorKind};

fn text(graph: &Graph, id: NodeId) -> &str {
	graph.node(id).expect("test node must exist").text()
}

#[test]
fn spaces_are_literal_text_without_quotes() {
	let graph = Graph::parse(" system name  \t child with spaces \n\tsecond child").expect("valid profile");
	let root = graph.roots()[0];
	assert_eq!(text(&graph, root), " system name  ");
	assert_eq!(
		graph.node(root)
			.expect("root")
			.children()
			.iter()
			.map(|id| text(&graph, *id))
			.collect::<Vec<_>>(),
		[" child with spaces ", "second child"]
	);
}

#[test]
fn inline_tabs_form_parent_child_chains() {
	let graph = Graph::parse("runtime\tops\tcalcs\tin-place").expect("valid inline chain");
	let runtime = graph.roots()[0];
	let ops = graph.node(runtime).expect("runtime").children()[0];
	let calcs = graph.node(ops).expect("ops").children()[0];
	let in_place = graph.node(calcs).expect("calcs").children()[0];
	assert_eq!(text(&graph, runtime), "runtime");
	assert_eq!(text(&graph, ops), "ops");
	assert_eq!(text(&graph, calcs), "calcs");
	assert_eq!(text(&graph, in_place), "in-place");
}

#[test]
fn leading_tabs_attach_to_prior_ancestors() {
	let graph = Graph::parse("root\n\tchild a\n\t\tgrandchild\n\tchild b").expect("valid indentation");
	let root = graph.roots()[0];
	let children = graph.node(root).expect("root").children();
	assert_eq!(children.len(), 2);
	assert_eq!(text(&graph, children[0]), "child a");
	assert_eq!(text(&graph, children[1]), "child b");
	assert_eq!(
		text(
			&graph,
			graph.node(children[0]).expect("child a").children()[0]
		),
		"grandchild"
	);
}

#[test]
fn roots_and_siblings_preserve_source_order() {
	let graph = Graph::parse("first\tone\n\ttwo\nsecond\nthird").expect("valid forest");
	assert_eq!(
		graph.roots()
			.iter()
			.map(|id| text(&graph, *id))
			.collect::<Vec<_>>(),
		["first", "second", "third"]
	);
	let first = graph.roots()[0];
	assert_eq!(
		graph.node(first)
			.expect("first")
			.children()
			.iter()
			.map(|id| text(&graph, *id))
			.collect::<Vec<_>>(),
		["one", "two"]
	);
}

#[test]
fn crlf_is_accepted_and_canonicalized_to_lf() {
	let graph = Graph::parse("root\r\n\tfirst\r\n\tsecond\r\n").expect("valid CRLF");
	assert_eq!(graph.to_canonical_string(), "root\tfirst\n\tsecond");
}

#[test]
fn indentation_cannot_skip_an_ancestor() {
	let error = Graph::parse("root\n\t\tgrandchild").expect_err("depth two has no depth-one ancestor");
	assert_eq!(
		error.kind(),
		&ParseErrorKind::IndentationJump {
			found: 2,
			maximum: 1,
		}
	);
	assert_eq!(error.location().line(), 2);
	assert_eq!(error.location().column(), 1);
	assert_eq!(error.location().offset(), 5);
}

#[test]
fn first_line_cannot_be_indented() {
	let error = Graph::parse("\troot").expect_err("first node cannot have a parent");
	assert_eq!(
		error.kind(),
		&ParseErrorKind::IndentationJump {
			found: 1,
			maximum: 0,
		}
	);
	assert_eq!(error.location().line(), 1);
	assert_eq!(error.location().column(), 1);
}

#[test]
fn empty_and_consecutive_tab_nodes_are_rejected() {
	let empty_line = Graph::parse("root\n\nchild").expect_err("blank line is an empty node");
	assert_eq!(empty_line.kind(), &ParseErrorKind::EmptyNode);
	assert_eq!(
		(empty_line.location().line(), empty_line.location().column()),
		(2, 1)
	);

	let consecutive = Graph::parse("root\t\tchild").expect_err("inline node is empty");
	assert_eq!(consecutive.kind(), &ParseErrorKind::EmptyNode);
	assert_eq!(
		(
			consecutive.location().line(),
			consecutive.location().column()
		),
		(1, 6)
	);

	let trailing = Graph::parse("root\t").expect_err("trailing node is empty");
	assert_eq!(trailing.kind(), &ParseErrorKind::EmptyNode);
	assert_eq!(
		(trailing.location().line(), trailing.location().column()),
		(1, 6)
	);

	let tabs_only = Graph::parse("\t").expect_err("indented empty node");
	assert_eq!(tabs_only.kind(), &ParseErrorKind::EmptyNode);
	assert_eq!(
		(tabs_only.location().line(), tabs_only.location().column()),
		(1, 2)
	);
}

#[test]
fn bare_carriage_return_is_rejected_at_its_character_column() {
	let error = Graph::parse("røot\rchild").expect_err("bare carriage return");
	assert_eq!(error.kind(), &ParseErrorKind::BareCarriageReturn);
	assert_eq!((error.location().line(), error.location().column()), (1, 5));
	assert_eq!(error.location().offset(), 5);
}

#[test]
fn empty_document_is_an_empty_forest_but_a_blank_line_is_not() {
	assert!(Graph::parse("").expect("empty document").is_empty());
	assert!(Graph::parse("\n").is_err());
	assert!(Graph::parse("\r\n").is_err());
}

#[test]
fn canonical_serialization_roundtrips_the_ordered_forest() {
	let source = "system\n\tmaster\n\t\tdevice\n\tworkers\n\t\tworker 0\n\t\t\tdevice 0\n\t\tworker 1\nmachine two";
	let graph = Graph::parse(source).expect("valid forest");
	let canonical = graph.to_canonical_string();
	assert_eq!(
		canonical,
		"system\tmaster\tdevice\n\tworkers\tworker 0\tdevice 0\n\t\tworker 1\nmachine two"
	);
	assert_eq!(
		Graph::parse(&canonical).expect("canonical form parses"),
		graph
	);
	assert_eq!(graph.to_string(), canonical);
}

#[test]
fn builder_rejects_unrepresentable_text_and_unknown_parents() {
	let mut graph = Graph::new();
	assert_eq!(
		graph.append_root(""),
		Err(GraphError::InvalidNodeText {
			kind: NodeTextErrorKind::Empty,
			character: 1,
		})
	);
	assert_eq!(
		graph.append_root("bad\ttext"),
		Err(GraphError::InvalidNodeText {
			kind: NodeTextErrorKind::Tab,
			character: 4,
		})
	);
	let mut other = Graph::new();
	let foreign = other.append_root("foreign").expect("foreign root");
	assert_eq!(
		graph.append_child(foreign, "child"),
		Err(GraphError::UnknownParent(foreign))
	);
}

#[test]
fn structural_equality_does_not_depend_on_arena_insertion_order() {
	let parsed = Graph::parse("a\tx\nb\ty").expect("valid forest");
	let mut built = Graph::new();
	let a = built.append_root("a").expect("root a");
	let b = built.append_root("b").expect("root b");
	built.append_child(b, "y").expect("child y");
	built.append_child(a, "x").expect("child x");
	assert_eq!(built, parsed);
	assert_eq!(
		Graph::parse(&built.to_canonical_string()).expect("roundtrip"),
		built
	);
}

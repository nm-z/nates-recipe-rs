use ogdl::ogdl;

#[test]
fn first_line_tab_is_edge() {
	assert_eq!(format!("{}", ogdl!(d1.r"child	child")), "d1\n    child child");
}

#[test]
fn dot_is_depth() {
	assert_eq!(format!("{}", ogdl!(d2.r"parent".r"child")), "d2\n    parent child");
}

#[test]
fn newline_makes_siblings() {
	ogdl!(d3.r"
child
child
");
	assert_eq!(format!("{}", ogdl!(d3)), "d3 child child");
	ogdl!(d3.child{1}.q);
	assert_eq!(format!("{}", ogdl!(d3)), "d3\n    child\n    child q");
}

#[test]
fn leading_tab_is_depth() {
	ogdl!(d4.r"
parent
	child
");
	assert_eq!(format!("{}", ogdl!(d4)), "d4\n    parent child");
}

#[test]
fn inline_tab_after_newline_is_content() {
	ogdl!(d5.r"
parent
	child	dog
");
	assert_eq!(format!("{}", ogdl!(d5)), "d5\n    parent child\tdog");
}

#[test]
fn content_tab_only_in_own_name() {
	ogdl!(d6.r"
parent
	child
	dog	parrot
");
	assert_eq!(format!("{}", ogdl!(d6)), "d6\n    parent child dog\tparrot");
}

#[test]
fn first_line_edge_then_sibling() {
	ogdl!(d7.r"
inline	tab
newline
");
	assert_eq!(format!("{}", ogdl!(d7)), "d7\n    inline tab\n    newline");
}

#[test]
fn skeleton_doc_form() {
	ogdl!(d14.r"
measuring:
	engi
		ETH
			1GbE
		CPU
			RAM
");
	assert_eq!(
		format!("{}", ogdl!(d14)),
		"d14\n    measuring:\n        engi\n            ETH 1GbE\n            CPU RAM"
	);
}

#[test]
fn invalid_docs_are_compile_errors() {
	use ogdl::__macro_support::doc_ok;
	assert_eq!(doc_ok("\n x\n"), 2);
	assert_eq!(doc_ok("\n\ta\nb\n"), 3);
	assert_eq!(doc_ok("\na\n\t\t\tb\n"), 4);
	assert_eq!(doc_ok("\n\n"), 1);
	assert_eq!(doc_ok("a\t"), 5);
	assert_eq!(doc_ok("a\t\tb"), 5);
	assert_eq!(doc_ok("\nmeasuring:\n\tengi\n\t\tETH\n"), 0);
	assert_eq!(doc_ok("child\tchild"), 0);
	assert_eq!(doc_ok("a\n\tb\tc\n"), 0);
}

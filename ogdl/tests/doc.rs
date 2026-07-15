use ogdl::ogdl;

#[test]
fn first_line_tab_is_sibling() {
	assert_eq!(
		format!("{}", ogdl!(d1."child	child")),
		"d1\n\tchild\n\tchild"
	);
}

#[test]
fn dot_is_depth() {
	assert_eq!(
		format!("{}", ogdl!(d2."parent"."child")),
		"d2\n\tparent\n\t\tchild"
	);
}

#[test]
fn newline_makes_siblings() {
	ogdl!(d3."
child
child
");
	assert_eq!(format!("{}", ogdl!(d3)), "d3\n\tchild\n\tchild");
	ogdl!(d3.child{1}.q);
	assert_eq!(format!("{}", ogdl!(d3)), "d3\n\tchild\n\tchild\n\t\tq");
}

#[test]
fn leading_tab_is_depth() {
	ogdl!(d4."
parent
	child
");
	assert_eq!(format!("{}", ogdl!(d4)), "d4\n\tparent\n\t\tchild");
}

#[test]
fn inline_tab_after_newline_is_content() {
	ogdl!(d5."
parent
	child	dog
");
	assert_eq!(format!("{}", ogdl!(d5)), "d5\n\tparent\n\t\tchild\tdog");
}

#[test]
fn content_tab_only_in_own_name() {
	ogdl!(d6."
parent
	child
	dog	parrot
");
	assert_eq!(
		format!("{}", ogdl!(d6)),
		"d6\n\tparent\n\t\tchild\n\t\tdog\tparrot"
	);
}

#[test]
fn first_line_tabs_then_newline_siblings() {
	ogdl!(d7."
inline	tab
newline
");
	assert_eq!(format!("{}", ogdl!(d7)), "d7\n\tinline\n\ttab\n\tnewline");
}

#[test]
fn skeleton_doc_form() {
	ogdl!(d14."
measuring:
	engi
		ETH
			1GbE
		CPU
			RAM
");
	assert_eq!(
		format!("{}", ogdl!(d14)),
		"d14\n\tmeasuring:\n\t\tengi\n\t\t\tETH\n\t\t\t\t1GbE\n\t\t\tCPU\n\t\t\t\tRAM"
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

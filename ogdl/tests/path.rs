use ogdl::ogdl;

#[test]
fn create_and_display() {
	assert_eq!(format!("{}", ogdl!(p1.b.c)), "p1\n    b c");
}

#[test]
fn navigate_not_duplicate() {
	ogdl!(p2.b.c);
	ogdl!(p2.b.c);
	assert_eq!(format!("{}", ogdl!(p2)), "p2\n    b c");
}

#[test]
fn values_accumulate() {
	ogdl!(p3.w.r"0.1");
	ogdl!(p3.w.r"0.2");
	assert_eq!(format!("{}", ogdl!(p3)), "p3\n    w 0.1 0.2");
}

#[test]
fn selector_zero_based() {
	ogdl!(p4.r"
b
	x
b
	y
");
	ogdl!(p4.b{1}.z);
	assert_eq!(format!("{}", ogdl!(p4)), "p4\n    b x\n    b y z");
}

#[test]
fn index_zero_based() {
	ogdl!(p5.r"
b
	x
	y
");
	ogdl!(p5.b[1].q);
	assert_eq!(format!("{}", ogdl!(p5)), "p5\n    b\n        x\n        y q");
}

#[test]
fn selector_then_index() {
	ogdl!(p6.r"
b
	x
	y
b
	z
");
	ogdl!(p6.b{0}[1].m);
	assert_eq!(
		format!("{}", ogdl!(p6)),
		"p6\n    b\n        x\n        y m\n    b z"
	);
}

#[test]
fn star_selects_every_match() {
	ogdl!(p7.r"
b
	x
b
	y
");
	ogdl!(p7.b*.k);
	assert_eq!(format!("{}", ogdl!(p7)), "p7\n    b x k\n    b y k");
}

#[test]
fn braces_delete_branch() {
	ogdl!(p8.b.c);
	ogdl!(p8.d);
	ogdl!(p8.b{});
	assert_eq!(format!("{}", ogdl!(p8)), "p8 d");
}

#[test]
fn missing_selector_selects_nothing() {
	ogdl!(p9.x.y);
	ogdl!(p9.b{3}.z);
	assert_eq!(format!("{}", ogdl!(p9)), "p9\n    x y");
}

#[test]
fn missing_index_selects_nothing() {
	ogdl!(p10.b.c);
	ogdl!(p10.b[5].q);
	assert_eq!(format!("{}", ogdl!(p10)), "p10\n    b c");
}

#[test]
fn unicode_names() {
	ogdl!(p11.r"日本語".x);
	assert_eq!(format!("{}", ogdl!(p11)), "p11\n    日本語 x");
}

#[test]
fn literal_head_path() {
	assert_eq!(format!("{}", ogdl!(r"p12 space".k)), "p12 space k");
}

#[test]
fn chain_of_literals() {
	assert_eq!(
		format!("{}", ogdl!(r"e13".r"2".r"3".r"eyes on me")),
		"e13\n    2\n        3 eyes on me"
	);
}

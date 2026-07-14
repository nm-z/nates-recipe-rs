use ogdl::ogdl;

#[test]
fn create_and_display() {
	assert_eq!(format!("{}", ogdl!(p1.b.c)), "p1\n\tb\n\t\tc");
}

#[test]
fn navigate_not_duplicate() {
	ogdl!(p2.b.c);
	ogdl!(p2.b.c);
	assert_eq!(format!("{}", ogdl!(p2)), "p2\n\tb\n\t\tc");
}

#[test]
fn values_ride_the_parent_line() {
	ogdl!(p3.w.r"0.1");
	ogdl!(p3.w.r"0.2");
	assert_eq!(format!("{}", ogdl!(p3)), "p3\n\tw\t0.1\t0.2");
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
	assert_eq!(
		format!("{}", ogdl!(p4)),
		"p4\n\tb\n\t\tx\n\tb\n\t\ty\n\t\tz"
	);
}

#[test]
fn index_zero_based() {
	ogdl!(p5.r"
b
	x
	y
");
	ogdl!(p5.b[1].q);
	assert_eq!(
		format!("{}", ogdl!(p5)),
		"p5\n\tb\n\t\tx\n\t\ty\n\t\t\tq"
	);
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
		"p6\n\tb\n\t\tx\n\t\ty\n\t\t\tm\n\tb\n\t\tz"
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
	assert_eq!(
		format!("{}", ogdl!(p7)),
		"p7\n\tb\n\t\tx\n\t\tk\n\tb\n\t\ty\n\t\tk"
	);
}

#[test]
fn braces_delete_branch() {
	ogdl!(p8.b.c);
	ogdl!(p8.d);
	ogdl!(p8.b{});
	assert_eq!(format!("{}", ogdl!(p8)), "p8\n\td");
}

#[test]
fn missing_selector_selects_nothing() {
	ogdl!(p9.x.y);
	ogdl!(p9.b{3}.z);
	assert_eq!(format!("{}", ogdl!(p9)), "p9\n\tx\n\t\ty");
}

#[test]
fn missing_index_selects_nothing() {
	ogdl!(p10.b.c);
	ogdl!(p10.b[5].q);
	assert_eq!(format!("{}", ogdl!(p10)), "p10\n\tb\n\t\tc");
}

#[test]
fn unicode_names() {
	ogdl!(p11.r"日本語".x);
	assert_eq!(format!("{}", ogdl!(p11)), "p11\n\t日本語\n\t\tx");
}

#[test]
fn literal_head_path() {
	assert_eq!(format!("{}", ogdl!(r"p12 space".k)), "p12 space\n\tk");
}

#[test]
fn variable_value_segment() {
	let eth = 0.125;
	ogdl!(p14.r"1GbE".&eth);
	assert_eq!(format!("{}", ogdl!(p14)), "p14\n\t1GbE\t0.125");
}

#[test]
fn variable_name_segment() {
	let host = "engi";
	let vram = 12868124672u64;
	ogdl!(p15.&host.VRAM.&vram);
	ogdl!(p15.&host.VRAM.&vram);
	assert_eq!(
		format!("{}", ogdl!(p15)),
		"p15\n\tengi\n\t\tVRAM\t12868124672"
	);
}

#[test]
fn literal_suffix_selects() {
	ogdl!(p16.r"
b
	x
b
	y
");
	ogdl!(p16.r"b"{1}.z);
	ogdl!(p16.r"b"*.k);
	assert_eq!(
		format!("{}", ogdl!(p16)),
		"p16\n\tb\n\t\tx\n\t\tk\n\tb\n\t\ty\n\t\tz\n\t\tk"
	);
	ogdl!(p16.r"b"{});
	assert_eq!(format!("{}", ogdl!(p16)), "p16");
}

#[test]
fn variable_index_segment() {
	let name = "n17";
	ogdl!(p17.&name.x);
	ogdl!(p17.&name.y);
	ogdl!(p17.&name[1].q);
	assert_eq!(
		format!("{}", ogdl!(p17)),
		"p17\n\tn17\n\t\tx\n\t\ty\n\t\t\tq"
	);
}

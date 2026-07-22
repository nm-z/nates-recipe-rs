use ogdl::ogdl;

#[test]
fn corpus_shapes() {
	assert_eq!(format!("{}", ogdl!("hello")), "hello");
	assert_eq!(format!("{}", ogdl!("a"."b"."c")), "a\n\tb\n\t\tc");
	assert_eq!(format!("{}", ogdl!("a	b	c")), "a\nb\nc");
	assert_eq!(
		format!(
			"{}",
			ogdl!("
a
b
c
")
		),
		"a\nb\nc"
	);
	assert_eq!(
		format!(
			"{}",
			ogdl!("
parent
	child1
	child2
	child3
")
		),
		"parent\n\tchild1\n\tchild2\n\tchild3"
	);
	assert_eq!(
		format!("{}", ogdl!("ETH"."1GbE"."0.125")),
		"ETH\n\t1GbE\t0.125"
	);
	let speed = 0.125f64;
	assert_eq!(
		format!("{}", ogdl!("ETH"."1GbE".&speed)),
		"ETH\n\t1GbE\t0.125"
	);
	let host = "engi";
	assert_eq!(
		format!("{}", ogdl!("measuring:".&host."ETH")),
		"measuring:\n\tengi\n\t\tETH"
	);
	assert_eq!(
		format!(
			"{}",
			ogdl!("
parent
	dog	parrot
")
		),
		"parent\n\tdog\tparrot"
	);
	assert_eq!(
		format!(
			"{}",
			ogdl!("\u{30c7}\u{30fc}\u{30bf}"."\u{540d}\u{524d}"."\u{592a}\u{90ce}")
		),
		"\u{30c7}\u{30fc}\u{30bf}\n\t\u{540d}\u{524d}\t\u{592a}\u{90ce}"
	);
	assert_eq!(
		format!("{}", ogdl!("hello world"."foo bar")),
		"hello world\n\tfoo bar"
	);
	assert_eq!(format!("{}", ogdl!("1GbE"."0.125")), "1GbE\t0.125");
}

use ogdl::ogdl;

#[test]
fn corpus_shapes() {
	assert_eq!(format!("{}", ogdl!(r"hello")), "hello");
	assert_eq!(format!("{}", ogdl!(r"a".r"b".r"c")), "a\n\tb\n\t\tc");
	assert_eq!(format!("{}", ogdl!(r"a	b	c")), "a\nb\nc");
	assert_eq!(
		format!(
			"{}",
			ogdl!(r"
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
			ogdl!(r"
parent
	child1
	child2
	child3
")
		),
		"parent\n\tchild1\n\tchild2\n\tchild3"
	);
	assert_eq!(
		format!("{}", ogdl!(r"ETH".r"1GbE".r"0.125")),
		"ETH\n\t1GbE\t0.125"
	);
	let speed = 0.125;
	assert_eq!(
		format!("{}", ogdl!(r"ETH".r"1GbE".&speed)),
		"ETH\n\t1GbE\t0.125"
	);
	let host = "engi";
	assert_eq!(
		format!("{}", ogdl!(r"measuring:".&host.r"ETH")),
		"measuring:\n\tengi\n\t\tETH"
	);
	assert_eq!(
		format!(
			"{}",
			ogdl!(r"
parent
	dog	parrot
")
		),
		"parent\n\tdog\tparrot"
	);
	assert_eq!(
		format!("{}", ogdl!(r"データ".r"名前".r"太郎")),
		"データ\n\t名前\t太郎"
	);
	assert_eq!(
		format!("{}", ogdl!(r"hello world".r"foo bar")),
		"hello world\n\tfoo bar"
	);
	assert_eq!(format!("{}", ogdl!(r"1GbE".r"0.125")), "1GbE\t0.125");
}

use ogdl::ogdl;

#[test]
fn nontty_delete_clamps_shown_cursor() {
	ogdl!("root"."a");
	ogdl!("root"."b");
	ogdl!("root"."c");
	let first = ogdl!("root"."c").show();
	assert_eq!(first, "root\n\ta\n\tb\n\tc");
	ogdl!("root"."b"{});
	ogdl!("root"."d");
	let second = ogdl!("root"."d").show();
	assert_eq!(second, "\td");
}

#[test]
fn divergence_finds_first_changed_line() {
	let before: Vec<String> = ["r", "a", "b", "c"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	let after: Vec<String> = ["r", "a", "c"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	assert_eq!(ogdl::divergence(&before, &after), 2);
	assert_eq!(ogdl::divergence(&before, &before.clone()), 4);
	let empty: Vec<String> = Vec::new();
	assert_eq!(ogdl::divergence(&before, &empty), 0);
	assert_eq!(ogdl::divergence(&empty, &before), 0);
}

#[test]
fn reconcile_clamps_shown_to_new_length() {
	let before: Vec<String> = ["r", "a", "b", "c"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	let after: Vec<String> = ["r", "a", "c"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	let mut shown: usize = 4;
	ogdl::reconcile(&before, &after, &mut shown);
	assert_eq!(shown, 3);
}

#[test]
fn reconcile_below_cursor_leaves_shown_untouched() {
	let before: Vec<String> = ["r", "a", "b", "c"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	let after: Vec<String> = ["r", "a", "b"]
		.iter()
		.map(|s| return (*s).to_owned())
		.collect();
	let mut shown: usize = 2;
	ogdl::reconcile(&before, &after, &mut shown);
	assert_eq!(shown, 2);
}

#[test]
fn reconcile_zero_shown_is_noop() {
	let before: Vec<String> = vec!["r".to_owned(), "b".to_owned()];
	let after: Vec<String> = vec!["r".to_owned()];
	let mut shown: usize = 0;
	ogdl::reconcile(&before, &after, &mut shown);
	assert_eq!(shown, 0);
}

//! The glob grammar's adjudicated corpus: every worksheet cell and elicited
//! example that survived the final law, as executable truth.

use ogdl::Node;

fn sel(tree: &str, expr: &str) -> String {
	let names: Vec<String> = Node::parse(tree)
		.glob(expr)
		.expect("glob parse")
		.iter()
		.map(|n| return n.name.clone())
		.collect();
	return names.join(" ");
}

const CHAIN: &str = "a\n\tb\n\t\tc";
const FIVE: &str = "a\n\tb\n\tc\n\t\td\ne";
const BOUNDS: &str = "a\n\tb\n\t\tc\n\td\n\t\te\n\t\t\tf";
const FORK: &str = "a\n\tb\n\tc\n\t\td\ne\n\tf";
const SPLIT: &str = "a\n\tb\n\t\tc\nd\n\te";
const PROBE1: &str = "a\n\tb\n\t\tc\nd";
const PROBE2: &str = "a\n\tb\n\tc\nd";

#[test]
fn chain_levels() {
	assert_eq!(sel(CHAIN, "(0)"), "a");
	assert_eq!(sel(CHAIN, "(1)"), "b");
	assert_eq!(sel(CHAIN, "(2)"), "c");
	assert_eq!(sel(CHAIN, "*(0)"), "");
	assert_eq!(sel(CHAIN, "*(1)"), "a");
	assert_eq!(sel(CHAIN, "*(2)"), "a b");
	assert_eq!(sel(CHAIN, "(0)*"), "b c");
	assert_eq!(sel(CHAIN, "(1)*"), "c");
	assert_eq!(sel(CHAIN, "(2)*"), "");
}

#[test]
fn chain_rows() {
	assert_eq!(sel(CHAIN, "[0]"), "a");
	assert_eq!(sel(CHAIN, "[1]"), "b");
	assert_eq!(sel(CHAIN, "[2]"), "c");
	assert_eq!(sel(CHAIN, "*[0]"), "b c");
	assert_eq!(sel(CHAIN, "*[1]"), "c");
	assert_eq!(sel(CHAIN, "*[2]"), "");
	assert_eq!(sel(CHAIN, "[0]*"), "");
	assert_eq!(sel(CHAIN, "[1]*"), "a");
	assert_eq!(sel(CHAIN, "[2]*"), "a b");
}

#[test]
fn chain_lineage_one_step() {
	assert_eq!(sel(CHAIN, "*b"), "a");
	assert_eq!(sel(CHAIN, "b*"), "c");
	assert_eq!(sel(CHAIN, "*a"), "");
	assert_eq!(sel(CHAIN, "c*"), "");
}

#[test]
fn five_node_levels_and_rows() {
	assert_eq!(sel(FIVE, "(0)"), "a e");
	assert_eq!(sel(FIVE, "(1)"), "b c");
	assert_eq!(sel(FIVE, "(2)"), "d");
	assert_eq!(sel(FIVE, "*(2)"), "a b c e");
	assert_eq!(sel(FIVE, "(0)*"), "b c d");
	assert_eq!(sel(FIVE, "[3]"), "d");
	assert_eq!(sel(FIVE, "*[0]"), "b c d e");
	assert_eq!(sel(FIVE, "*[2]"), "d e");
	assert_eq!(sel(FIVE, "[4]*"), "a b c d");
	assert_eq!(sel(FIVE, "*c"), "a");
	assert_eq!(sel(FIVE, "c*"), "d");
}

#[test]
fn probe1_four_spellings_of_a() {
	for expr in ["a", "[0](0)", "*b", "[1]*"] {
		assert_eq!(sel(PROBE1, expr), "a", "{expr}");
	}
}

#[test]
fn probe2_leaf_star_is_nothing() {
	for expr in ["b*.d", "d", "[3]", "*[2]"] {
		assert_eq!(sel(PROBE2, expr), "d", "{expr}");
	}
}

#[test]
fn fork_three_spellings_of_ade() {
	for expr in ["*(1).(1)*", "(0).(2)", "*b.c*.*f"] {
		assert_eq!(sel(FORK, expr), "a d e", "{expr}");
	}
}

#[test]
fn split_two_spellings_of_bde() {
	for expr in ["[3].[4].[1]", "d.d*.a*"] {
		assert_eq!(sel(SPLIT, expr), "b d e", "{expr}");
	}
}

#[test]
fn bounds_walk_collects() {
	assert_eq!(sel(BOUNDS, "a.d.*e"), "a d");
}

#[test]
fn juxtaposition_is_between() {
	assert_eq!(sel(BOUNDS, "*[0][5]*"), "b c d e");
	assert_eq!(sel(BOUNDS, "(0)**(3)"), "b c d e");
	assert_eq!(sel(BOUNDS, "[0](0)"), "a");
	assert_eq!(sel(BOUNDS, "(1)[3]"), "d");
	assert_eq!(sel(BOUNDS, "(1)[1]"), "b");
}

#[test]
fn out_of_range_is_nothing() {
	assert_eq!(sel(PROBE1, "[9]"), "");
	assert_eq!(sel(PROBE1, "(9)"), "");
	assert_eq!(sel(PROBE1, "z"), "");
	assert_eq!(sel(PROBE1, "*[3]"), "");
}

#[test]
fn display_native_indent_doc_order() {
	let fork = Node::parse(FORK);
	assert_eq!(fork.glob_show("*(1).(1)*").expect("show"), "a\n\t\td\ne");
	let split = Node::parse(SPLIT);
	assert_eq!(split.glob_show("[3].[4].[1]").expect("show"), "\tb\nd\n\te");
	assert_eq!(Node::parse(PROBE1).glob_show("a").expect("show"), "a");
}

#[test]
fn every_subset_of_five_is_selectable() {
	let node_names = ["a", "b", "c", "d", "e"];
	for mask in 1u32..32 {
		let expr: Vec<String> = (0..5)
			.filter(|i| return mask & (1 << i) != 0)
			.map(|i| return format!("[{i}]"))
			.collect();
		let want: Vec<&str> = (0..5)
			.filter(|i| return mask & (1 << i) != 0)
			.map(|i| return node_names[i as usize])
			.collect();
		assert_eq!(sel(FIVE, &expr.join(".")), want.join(" "), "mask {mask:#07b}");
	}
}

#[test]
fn delete_branch() {
	let mut n = Node::parse(FIVE);
	assert_eq!(n.glob_del("c{}").expect("del"), 1);
	assert_eq!(n.serialize(), "a\n\tb\ne\n");
}

#[test]
fn delete_level_takes_branches() {
	let mut n = Node::parse(FIVE);
	assert_eq!(n.glob_del("(1){}").expect("del"), 2);
	assert_eq!(n.serialize(), "a\ne\n");
}

#[test]
fn delete_wipe_is_legal() {
	let mut n = Node::parse(FIVE);
	assert_eq!(n.glob_del("{}").expect("del"), 2);
	assert_eq!(n.serialize(), "");
	let mut n2 = Node::parse(FIVE);
	assert_eq!(n2.glob_del("*{}").expect("del"), 2);
	assert_eq!(n2.serialize(), "");
}

#[test]
fn delete_missing_is_noop() {
	let mut n = Node::parse(FIVE);
	assert_eq!(n.glob_del("z{}").expect("del"), 0);
	assert_eq!(n.serialize(), format!("{FIVE}\n"));
}

#[test]
fn malformed_expressions_error() {
	let n = Node::parse(FIVE);
	assert!(n.glob("[x]").is_err());
	assert!(n.glob("[1").is_err());
	assert!(n.glob("a..b").is_err());
	assert!(n.glob("**a").is_err());
}

#[test]
fn bare_star_is_everything() {
	assert_eq!(sel(PROBE1, "*"), "a b c d");
}

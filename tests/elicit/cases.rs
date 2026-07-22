
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

fn set(tree: &str, expr: &str) -> String {
	let mut names: Vec<String> = Node::parse(tree)
		.glob(expr)
		.expect("glob parse")
		.iter()
		.map(|n| return n.name.clone())
		.collect();
	names.sort();
	return names.join(" ");
}

const T1: &str = "a\n\tb\n\tc\n\t\td\n\te\nf\n\tg";
const T2: &str = "a\n\tb\n\t\tc\n\td\n\t\te\n\t\t\tf";
const T3: &str = "a\n\tb\n\tc\n\t\td\ne\n\tf";
const T4: &str = "a\n\tb\n\t\tc\nd\n\te";
const T5: &str = "a\n\tb\n\t\tc\nd";
const T6: &str = "a\n\tb\n\tc\nd";
const T5N: &str = "a\n\tb\n\tc\n\t\td\ne";

#[test]
fn case1_image_pair() {
	assert_eq!(sel(T1, "[0]*(1)"), "f");
}

#[test]
fn case2_1_walk_collects() {
	assert_eq!(set(T2, "a.d.*e"), "a d");
}

#[test]
fn case2_2_level_bounds() {
	assert_eq!(set(T2, "(0)*.*(3)"), "b c d e");
}

#[test]
fn case2_3_row_bounds() {
	assert_eq!(set(T2, "[0]*.*[5]"), "b c d e");
}

#[test]
fn case2_4_bracket_swap_rows() {
	assert_eq!(set(T2, "[1].[2]"), "b c");
}

#[test]
fn case2_5_bracket_swap_levels() {
	assert_eq!(set(T2, "(1).(2).(3).(4)"), "b c d e f");
}

#[test]
fn case2_6_bracket_swap_pair() {
	assert_eq!(set(T2, "(3).(4)"), "f");
}

#[test]
fn case2_7_subtree_star() {
	assert_eq!(set(T2, "a.d*"), "a e");
}

#[test]
fn case3_equality_chain_ade() {
	for expr in ["*(1).(1)*", "(0).(2)", "*b.c*.*f"] {
		assert_eq!(set(T3, expr), "a d e", "{expr}");
	}
	assert_eq!(set(T3, "b*.*f"), "e");
}

#[test]
fn case4_equality_chain_bde() {
	for expr in ["[3].[4].[1]", "d.d*.a*"] {
		assert_eq!(set(T4, expr), "b d e", "{expr}");
	}
	assert_eq!(set(T4, "d.[1]"), "b d");
}

#[test]
fn case5_probe_branch_depth() {
	for expr in ["a", "[0](0)", "*[1]"] {
		assert_eq!(set(T5, expr), "a", "{expr}");
	}
	assert_eq!(set(T5, "*b"), "a b");
}

#[test]
fn case6_probe_childless_star() {
	for expr in ["b*.d", "d", "[3]", "[2]*"] {
		assert_eq!(set(T6, expr), "d", "{expr}");
	}
}

#[test]
fn case7_aborted_step_lookback() {
	assert_eq!(set(T3, "(2).*e"), "d");
}

#[test]
fn case8_five_node_worksheet() {
	assert_eq!(sel(T5N, "*c"), "c a");
	assert_eq!(sel(T5N, "c*"), "c d");
	assert_eq!(sel(T5N, "(0)"), "a e");
	assert_eq!(sel(T5N, "(1)"), "b c");
	assert_eq!(sel(T5N, "(2)"), "d");
	assert_eq!(sel(T5N, "*(0)"), "");
	assert_eq!(sel(T5N, "*(1)"), "a e");
	assert_eq!(sel(T5N, "*(2)"), "b c a e");
	assert_eq!(sel(T5N, "(0)*"), "b c d");
	assert_eq!(sel(T5N, "(1)*"), "d");
	assert_eq!(sel(T5N, "(2)*"), "");
	assert_eq!(sel(T5N, "[0]"), "a");
	assert_eq!(sel(T5N, "[1]"), "b");
	assert_eq!(sel(T5N, "[2]"), "c");
	assert_eq!(sel(T5N, "[3]"), "d");
	assert_eq!(sel(T5N, "[4]"), "e");
	assert_eq!(sel(T5N, "[0]*"), "b c d e");
	assert_eq!(sel(T5N, "[1]*"), "c d e");
	assert_eq!(sel(T5N, "[2]*"), "d e");
	assert_eq!(sel(T5N, "[3]*"), "e");
	assert_eq!(sel(T5N, "[4]*"), "");
	assert_eq!(sel(T5N, "*[0]"), "");
	assert_eq!(sel(T5N, "*[1]"), "a");
	assert_eq!(sel(T5N, "*[2]"), "b a");
	assert_eq!(sel(T5N, "*[3]"), "c b a");
	assert_eq!(sel(T5N, "*[4]"), "d c b a");
}

const T3N: &str = "a\n\tb\n\t\tc";
const TQ: &str = "Q\n\tu\n\ta\n\t\tr\nk";

fn cell(tree: &str, expr: &str, want: &str) {
	assert_eq!(sel(tree, expr), want, "{expr}");
}

fn doc_names(tree: &str) -> Vec<String> {
	return tree
		.lines()
		.map(|l| return l.trim_start_matches('\t').to_owned())
		.collect();
}

fn coverage(tree: &str, inv: &[&str]) -> (String, String) {
	use std::collections::BTreeSet;
	let names = doc_names(tree);
	let reach: BTreeSet<BTreeSet<String>> = inv
		.iter()
		.map(|e| {
			return sel(tree, e)
				.split_whitespace()
				.map(str::to_owned)
				.collect();
		})
		.collect();
	let n = names.len();
	let mut matched: Vec<Vec<String>> = Vec::new();
	let mut unmatched: Vec<Vec<String>> = Vec::new();
	for mask in 0u32..(1 << n) {
		let subset: Vec<String> = (0..n)
			.filter(|i| return mask & (1 << i) != 0)
			.map(|i| return names[i].clone())
			.collect();
		let key: BTreeSet<String> = subset.iter().cloned().collect();
		if reach.contains(&key) {
			matched.push(subset);
		} else {
			unmatched.push(subset);
		}
	}
	let fmt = |list: &[Vec<String>], label: &str, total: usize| -> String {
		let mut out = String::new();
		for size in 0..=n {
			let mut g: Vec<String> = list
				.iter()
				.filter(|s| return s.len() == size)
				.map(|s| return s.join(" "))
				.collect();
			g.sort();
			if g.is_empty() {
				continue;
			}
			let word = if size == 1 { "object" } else { "objects" };
			out.push_str(&format!("{size} {word}:\n"));
			if size == 0 {
				out.push_str("\t\u{2205}\n");
			} else {
				for line in g {
					out.push_str(&format!("\t{line}\n"));
				}
			}
		}
		out.push_str(&format!("{label}: {}/{total}", list.len()));
		return out;
	};
	return (
		fmt(&matched, "matched", 1 << n),
		fmt(&unmatched, "unmatched", 1 << n),
	);
}

const INV3: &[&str] = &[
	"*b", "b*", "(0)", "(1)", "(2)", "*(0)", "*(1)", "*(2)", "(0)*", "(1)*", "(2)*",
	"[0]", "[1]", "[2]", "[0]*", "[1]*", "[2]*", "*[0]", "*[1]", "*[2]",
];
const INV5: &[&str] = &[
	"*c", "c*", "(0)", "(1)", "(2)", "*(0)", "*(1)", "*(2)", "(0)*", "(1)*", "(2)*",
	"[0]", "[1]", "[2]", "[3]", "[4]", "[0]*", "[1]*", "[2]*", "[3]*", "[4]*",
	"*[0]", "*[1]", "*[2]", "*[3]", "*[4]",
];

#[test] fn case8_1_1() { cell(T3N, "*b", "b a"); }
#[test] fn case8_1_2() { cell(T3N, "b*", "b c"); }
#[test] fn case8_1_3() { cell(T3N, "(0)", "a"); }
#[test] fn case8_1_4() { cell(T3N, "(1)", "b"); }
#[test] fn case8_1_5() { cell(T3N, "(2)", "c"); }
#[test] fn case8_1_6() { cell(T3N, "*(0)", ""); }
#[test] fn case8_1_7() { cell(T3N, "*(1)", "a"); }
#[test] fn case8_1_8() { cell(T3N, "*(2)", "b a"); }
#[test] fn case8_1_9() { cell(T3N, "(0)*", "b c"); }
#[test] fn case8_1_10() { cell(T3N, "(1)*", "c"); }
#[test] fn case8_1_11() { cell(T3N, "(2)*", ""); }
#[test] fn case8_1_12() { cell(T3N, "[0]", "a"); }
#[test] fn case8_1_13() { cell(T3N, "[1]", "b"); }
#[test] fn case8_1_14() { cell(T3N, "[2]", "c"); }
#[test] fn case8_1_15() { cell(T3N, "[0]*", "b c"); }
#[test] fn case8_1_16() { cell(T3N, "[1]*", "c"); }
#[test] fn case8_1_17() { cell(T3N, "[2]*", ""); }
#[test] fn case8_1_18() { cell(T3N, "*[0]", ""); }
#[test] fn case8_1_19() { cell(T3N, "*[1]", "a"); }
#[test] fn case8_1_20() { cell(T3N, "*[2]", "b a"); }

#[test]
fn case8_1_21() {
	let (m, _u) = coverage(T3N, INV3);
	assert_eq!(
		m,
		"0 objects:\n\t\u{2205}\n1 object:\n\ta\n\tb\n\tc\n2 objects:\n\ta b\n\tb c\nmatched: 6/8"
	);
}

#[test]
fn case8_1_22() {
	let (_m, u) = coverage(T3N, INV3);
	assert_eq!(u, "2 objects:\n\ta c\n3 objects:\n\ta b c\nunmatched: 2/8");
}

#[test] fn case8_2_1() { cell(T5N, "*c", "c a"); }
#[test] fn case8_2_2() { cell(T5N, "c*", "c d"); }
#[test] fn case8_2_3() { cell(T5N, "(0)", "a e"); }
#[test] fn case8_2_4() { cell(T5N, "(1)", "b c"); }
#[test] fn case8_2_5() { cell(T5N, "(2)", "d"); }
#[test] fn case8_2_6() { cell(T5N, "*(0)", ""); }
#[test] fn case8_2_7() { cell(T5N, "*(1)", "a e"); }
#[test] fn case8_2_8() { cell(T5N, "*(2)", "b c a e"); }
#[test] fn case8_2_9() { cell(T5N, "(0)*", "b c d"); }
#[test] fn case8_2_10() { cell(T5N, "(1)*", "d"); }
#[test] fn case8_2_11() { cell(T5N, "(2)*", ""); }
#[test] fn case8_2_12() { cell(T5N, "[0]", "a"); }
#[test] fn case8_2_13() { cell(T5N, "[1]", "b"); }
#[test] fn case8_2_14() { cell(T5N, "[2]", "c"); }
#[test] fn case8_2_15() { cell(T5N, "[3]", "d"); }
#[test] fn case8_2_16() { cell(T5N, "[4]", "e"); }
#[test] fn case8_2_17() { cell(T5N, "[0]*", "b c d e"); }
#[test] fn case8_2_18() { cell(T5N, "[1]*", "c d e"); }
#[test] fn case8_2_19() { cell(T5N, "[2]*", "d e"); }
#[test] fn case8_2_20() { cell(T5N, "[3]*", "e"); }
#[test] fn case8_2_21() { cell(T5N, "[4]*", ""); }
#[test] fn case8_2_22() { cell(T5N, "*[0]", ""); }
#[test] fn case8_2_23() { cell(T5N, "*[1]", "a"); }
#[test] fn case8_2_24() { cell(T5N, "*[2]", "b a"); }
#[test] fn case8_2_25() { cell(T5N, "*[3]", "c b a"); }
#[test] fn case8_2_26() { cell(T5N, "*[4]", "d c b a"); }

#[test]
fn case8_2_27() {
	let (m, _u) = coverage(T5N, INV5);
	assert_eq!(
		m,
		"0 objects:\n\t\u{2205}\n1 object:\n\ta\n\tb\n\tc\n\td\n\te\n2 objects:\n\ta b\n\ta c\n\ta e\n\tb c\n\tc d\n\td e\n3 objects:\n\ta b c\n\tb c d\n\tc d e\n4 objects:\n\ta b c d\n\ta b c e\n\tb c d e\nmatched: 18/32"
	);
}

#[test]
fn case8_2_28() {
	let (_m, u) = coverage(T5N, INV5);
	assert_eq!(
		u,
		"2 objects:\n\ta d\n\tb d\n\tb e\n\tc e\n3 objects:\n\ta b d\n\ta b e\n\ta c d\n\ta c e\n\ta d e\n\tb c e\n\tb d e\n4 objects:\n\ta b d e\n\ta c d e\n5 objects:\n\ta b c d e\nunmatched: 14/32"
	);
}

#[test] fn case8_3_1() { cell(TQ, "*a", "a Q"); }
#[test] fn case8_3_2() { cell(TQ, "a*", "a r"); }
#[test] fn case8_3_3() { cell(TQ, "(0)", "Q k"); }
#[test] fn case8_3_4() { cell(TQ, "(1)", "u a"); }
#[test] fn case8_3_5() { cell(TQ, "(2)", "r"); }
#[test] fn case8_3_6() { cell(TQ, "*(0)", ""); }
#[test] fn case8_3_7() { cell(TQ, "*(1)", "Q k"); }
#[test] fn case8_3_8() { cell(TQ, "*(2)", "u a Q k"); }
#[test] fn case8_3_9() { cell(TQ, "(0)*", "u a r"); }
#[test] fn case8_3_10() { cell(TQ, "(1)*", "r"); }
#[test] fn case8_3_11() { cell(TQ, "(2)*", ""); }
#[test] fn case8_3_12() { cell(TQ, "[0]", "Q"); }
#[test] fn case8_3_13() { cell(TQ, "[1]", "u"); }
#[test] fn case8_3_14() { cell(TQ, "[2]", "a"); }
#[test] fn case8_3_15() { cell(TQ, "[3]", "r"); }
#[test] fn case8_3_16() { cell(TQ, "[4]", "k"); }
#[test] fn case8_3_17() { cell(TQ, "[0]*", "u a r k"); }
#[test] fn case8_3_18() { cell(TQ, "[1]*", "a r k"); }
#[test] fn case8_3_19() { cell(TQ, "[2]*", "r k"); }
#[test] fn case8_3_20() { cell(TQ, "[3]*", "k"); }
#[test] fn case8_3_21() { cell(TQ, "[4]*", ""); }
#[test] fn case8_3_22() { cell(TQ, "*[0]", ""); }
#[test] fn case8_3_23() { cell(TQ, "*[1]", "Q"); }
#[test] fn case8_3_24() { cell(TQ, "*[2]", "u Q"); }
#[test] fn case8_3_25() { cell(TQ, "*[3]", "a u Q"); }
#[test] fn case8_3_26() { cell(TQ, "*[4]", "r a u Q"); }

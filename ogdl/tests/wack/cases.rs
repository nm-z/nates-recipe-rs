//! Adversarial probes: expressions the elicited law underdetermines, each
//! encoded with my best-guess expectation. A red here is a question for Nate,
//! answered by telling me what actually happens and why.

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

const DUP: &str = "x\n\ty\nx\n\tz";
const RIDER: &str = "w\t0.1\nv";
const FIVE: &str = "a\n\tb\n\tc\n\t\td\ne";
const DEEP: &str = "g\n\th\n\t\ti";
const FORK: &str = "a\n\tb\n\tc\n\t\td\ne\n\tf";

#[test]
fn w1_duplicate_name_matches_every_occurrence() {
	assert_eq!(sel(DUP, "x"), "x x");
	assert_eq!(sel(DUP, "*y"), "y x");
	assert_eq!(sel(DUP, "x*"), "x y x z");
}

#[test]
fn w2_riders_do_not_occupy_rows() {
	assert_eq!(sel(RIDER, "[1]"), "v");
	assert_eq!(sel(RIDER, "(0)"), "w v");
}

#[test]
fn w3_name_level_juxtaposition() {
	assert_eq!(sel(FIVE, "c(1)"), "c");
	assert_eq!(sel(FIVE, "c(0)"), "");
}

#[test]
fn w4_contradictory_juxtaposition_is_empty_not_identity() {
	assert_eq!(sel(FIVE, "[2](0)"), "");
}

#[test]
fn w5_out_of_range_operand_drops_as_identity() {
	assert_eq!(sel(FIVE, "(9)[0]"), "a");
}

#[test]
fn w6_reversed_bounds_is_outward_union() {
	assert_eq!(sel(FIVE, "*(2).(0)*"), "b c a e d");
}

#[test]
fn w7_shared_star_between_names() {
	assert_eq!(sel(DEEP, "g*h"), "g h");
}

#[test]
fn w8_route_revisits_dedup() {
	assert_eq!(sel(FIVE, "a.a"), "a");
}

#[test]
fn w9_downstream_star_is_one_step_not_subtree() {
	assert_eq!(sel(DEEP, "g*"), "g h");
	assert_eq!(sel(DEEP, "g*.h*"), "h i");
}

#[test]
fn w10_disjoint_lineage_bounds_have_no_facing() {
	assert_eq!(sel(FORK, "b*.*e"), "");
}

#[test]
fn w11_deep_name_route_collects() {
	assert_eq!(sel(DEEP, "g.h.i"), "g h i");
}

#[test]
fn w12_level_of_empty_beyond_depth_in_route() {
	assert_eq!(sel(FIVE, "(9).d"), "d");
}

//! The glob grammar through its real door: the `ogdl!` macro, including the
//! loop-through forms with runtime coordinates. One test fn: the macro works
//! on the process-global graph, so this file owns its process.

use ogdl::ogdl;
use std::fmt::Display;

fn s(b: impl Display) -> String {
	let t = format!("{b}");
	return t.split_whitespace().collect::<Vec<_>>().join(" ");
}

#[test]
fn macro_speaks_glob() {
	ogdl!("a\n\tb\n\tc\n\t\td\ne");

	let lvl = ["a e", "b c", "d"];
	let lvl_left = ["", "a e", "b c a e"];
	let lvl_right = ["b c d", "d", ""];
	for i in 0..3 {
		assert_eq!(s(ogdl!((i))), lvl[i], "({i})");
		assert_eq!(s(ogdl!(*(i))), lvl_left[i], "*({i})");
		assert_eq!(s(ogdl!((i)*)), lvl_right[i], "({i})*");
	}

	let row = ["a", "b", "c", "d", "e"];
	let row_up = ["", "a", "b a", "c b a", "d c b a"];
	let row_down = ["b c d e", "c d e", "d e", "e", ""];
	for i in 0..5 {
		assert_eq!(s(ogdl!([i])), row[i], "[{i}]");
		assert_eq!(s(ogdl!(*[i])), row_up[i], "*[{i}]");
		assert_eq!(s(ogdl!([i]*)), row_down[i], "[{i}]*");
	}

	assert_eq!(s(ogdl!(*c)), "c a");
	assert_eq!(s(ogdl!(c*)), "c d");
	assert_eq!(s(ogdl!((0)*.*(2))), "b c");
	assert_eq!(s(ogdl!([0]*.*[4])), "b c d");
}

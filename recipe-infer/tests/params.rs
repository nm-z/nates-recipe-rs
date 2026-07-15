use recipe_infer::{Saved, load_ogdl, load_ogdl_str};
use std::env;
use std::fs;

#[test]
fn ogdl_format_roundtrips_host_side() {
	let path = env::temp_dir().join("nrs_ogdl_roundtrip.ogdl");
	let text = "\
r2\t0.42
embed
\t0\t-0.0312\t0.1847\t-0.0551
\t1\t0.0892\t-0.2104\t0.0033
attn
\twq\t1\t2\t3\t4
\twk\t5\t6\t7\t8
\twv\t9\t10\t11\t12
\two\t13\t14\t15\t16
\tbq\t0\t0
\tbk\t0\t0
\tbv\t0\t0
\tbo\t0\t0
z1
\tw\t0.01\t-0.02\t0.03
\tb\t0.001
z2
\tw\t0.04\t0.05\t0.06
\ta\t0.25
\tb\t0.002
";
	fs::write(&path, text).expect("write tmp ogdl");
	let parsed = load_ogdl(path.to_str().expect("utf8 path")).expect("load_ogdl");
	fs::remove_file(&path).ok();
	assert_eq!(parsed.len(), 4);
	assert_eq!(
		parsed[0],
		Saved::Embed(vec![-0.0312, 0.1847, -0.0551, 0.0892, -0.2104, 0.0033])
	);
	assert_eq!(
		parsed[1],
		Saved::Attn {
			wq: vec![1.0, 2.0, 3.0, 4.0],
			wk: vec![5.0, 6.0, 7.0, 8.0],
			wv: vec![9.0, 10.0, 11.0, 12.0],
			wo: vec![13.0, 14.0, 15.0, 16.0],
			bq: vec![0.0, 0.0],
			bk: vec![0.0, 0.0],
			bv: vec![0.0, 0.0],
			bo: vec![0.0, 0.0],
		}
	);
	assert_eq!(
		parsed[2],
		Saved::Dense {
			w: vec![0.01, -0.02, 0.03],
			b: 0.001,
			a: None
		}
	);
	assert_eq!(
		parsed[3],
		Saved::Dense {
			w: vec![0.04, 0.05, 0.06],
			b: 0.002,
			a: Some(0.25)
		}
	);
}

#[test]
fn dump_add_api_roundtrips() {
	let text = recipe_infer::params::ogdl_text(|g| {
		drop(g.add(0.42f64, "r2"));
		drop(g.add(vec![0.1f64, 0.2, 0.3], "z1.w"));
		drop(g.add(0.05f64, "z1.a"));
		drop(g.add(0.01f64, "z1.b"));
		drop(g.add(vec![-0.4f64, 0.5], "z2.w"));
		drop(g.add(0.02f64, "z2.b"));
	});
	let saved = load_ogdl_str(&text).expect("load_ogdl_str");
	assert_eq!(saved.len(), 2, "two dense neurons, metric header skipped");
	assert_eq!(
		saved[0],
		Saved::Dense {
			w: vec![0.1, 0.2, 0.3],
			b: 0.01,
			a: Some(0.05)
		}
	);
	assert_eq!(
		saved[1],
		Saved::Dense {
			w: vec![-0.4, 0.5],
			b: 0.02,
			a: None
		}
	);
}

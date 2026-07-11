use recipe_infer::{Saved, load_ogdl, load_ogdl_str};

#[test]
fn ogdl_format_roundtrips_host_side() {
	let path = std::env::temp_dir().join("nrs_ogdl_roundtrip.ogdl");
	let text = "\
r2=0.42
embed
    0=-0.0312 0.1847 -0.0551
    1=0.0892 -0.2104 0.0033
attn
    wq=1 2 3 4
    wk=5 6 7 8
    wv=9 10 11 12
    wo=13 14 15 16
    bq=0 0
    bk=0 0
    bv=0 0
    bo=0 0
z1
    w=0.01 -0.02 0.03
    b=0.001
z2
    w=0.04 0.05 0.06
    a=0.25
    b=0.002
";
	std::fs::write(&path, text).expect("write tmp ogdl");
	let parsed = load_ogdl(path.to_str().expect("utf8 path")).expect("load_ogdl");
	std::fs::remove_file(&path).ok();
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
		g.add(0.42_f64, "r2");
		g.add(vec![0.1_f64, 0.2, 0.3], "z1.w");
		g.add(0.05_f64, "z1.a");
		g.add(0.01_f64, "z1.b");
		g.add(vec![-0.4_f64, 0.5], "z2.w");
		g.add(0.02_f64, "z2.b");
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

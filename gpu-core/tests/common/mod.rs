use std::env;
use std::fs;
use std::process;
use std::sync::OnceLock;

static DIR: OnceLock<String> = OnceLock::new();

pub fn inventory_dir() -> String {
	DIR.get_or_init(hydrate).clone()
}

fn hydrate() -> String {
	let db_path = format!("{}/../recipe.db", env!("CARGO_MANIFEST_DIR"));
	let out = env::temp_dir().join(format!("ki_inv_{}", process::id()));
	fs::create_dir_all(&out).expect("create temp inventory dir");
	let conn =
		rusqlite::Connection::open(&db_path).unwrap_or_else(|e| panic!("open {db_path}: {e}"));

	let sources: Vec<String> = {
		let mut st = conn.prepare("SELECT DISTINCT source FROM kernels").unwrap();
		let rows = st.query_map([], |r| r.get::<_, String>(0)).unwrap();
		rows.filter_map(Result::ok).collect()
	};

	for src in &sources {
		let mut st = conn
                  .prepare("SELECT name, category, signature, description, dtypes, fused, vendor_backend, library, url, trivial FROM kernels WHERE source = ?1")
                  .unwrap();
		let kernels: Vec<serde_json::Value> = st
                  .query_map([src], |r| {
                        let dtypes: String = r.get(4)?;
                        let vb: String = r.get(6)?;
                        Ok(serde_json::json!({
                              "name": r.get::<_, String>(0)?,
                              "category": r.get::<_, String>(1)?,
                              "signature": r.get::<_, String>(2)?,
                              "description": r.get::<_, String>(3)?,
                              "dtypes": if dtypes.is_empty() { Vec::<String>::new() } else { dtypes.split('|').map(String::from).collect() },
                              "fused": r.get::<_, i64>(5)? != 0,
                              "vendor_backend": if vb.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(vb) },
                              "library": r.get::<_, String>(7)?,
                              "url": r.get::<_, String>(8)?,
                              "trivial": r.get::<_, i64>(9)? != 0,
                        }))
                  })
                  .unwrap()
                  .filter_map(Result::ok)
                  .collect();
		let doc = serde_json::json!({ "source": src, "kernels": kernels });
		fs::write(
			out.join(format!("{src}.json")),
			serde_json::to_string(&doc).unwrap(),
		)
		.expect("write inventory json");
	}
	out.to_string_lossy().into_owned()
}

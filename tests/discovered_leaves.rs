use recipe::*;
use std::any::Any;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> std::path::PathBuf {
	let nonce = SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock precedes Unix epoch").as_nanos();
	let path = std::env::temp_dir().join(format!("recipe-discovered-leaves-{}-{nonce}", std::process::id()));
	std::fs::create_dir_all(&path).unwrap_or_else(|error| panic!("cannot create {}: {error}", path.display()));
	std::fs::write(path.join("measurements.csv"), "feature,target\n0,0\n1,1\n2,2\n").unwrap_or_else(|error| panic!("cannot write data fixture: {error}"));
	std::fs::write(path.join("outside.json"), "{\"outside\":{\"nested\":true}}\n").unwrap_or_else(|error| panic!("cannot write malformed fixture: {error}"));
	path
}

fn train(source: &Path) {
	let data = recipe.data(source.to_string_lossy().as_ref()).target("target");
	let model = recipe.model().layer(1).loss(mse);
	let _ = recipe.train().epochs(1).run(&model, &data);
}

fn panic_text(error: Box<dyn Any + Send>) -> String {
	match error.downcast::<String>() {
		Ok(message) => *message,
		Err(error) => error.downcast::<&str>().map_or_else(|_| "non-string panic".to_owned(), |message| (*message).to_owned()),
	}
}

#[test]
fn discovered_malformed_table_leaf_is_skipped_but_named_leaf_errors() {
	let path = fixture();
	let result = std::panic::catch_unwind(|| train(&path));
	assert!(result.is_ok(), "a discovered malformed JSON leaf prevented the valid table from training");

	let error = std::panic::catch_unwind(|| train(&path.join("outside.json"))).expect_err("a caller-named malformed JSON leaf must fail");
	assert!(panic_text(error).contains("JSON records expect a top-level array"));
	std::fs::remove_dir_all(&path).unwrap_or_else(|error| panic!("cannot remove {}: {error}", path.display()));
}

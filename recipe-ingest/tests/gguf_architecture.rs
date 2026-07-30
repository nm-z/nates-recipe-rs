use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use recipe_ingest::{GgufLimits, inspect_gguf_model_stream};

fn fixture_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("examples")
		.join("datasets")
		.join("llamacpp-archs-seed42")
}

fn limits(file_bytes: u64) -> GgufLimits {
	let bound = file_bytes.max(1);
	GgufLimits::new(bound, bound, bound, 4, bound, bound, bound).expect("fixture-sized GGUF limits")
}

#[test]
fn checked_in_model_corpus_exposes_exact_nonempty_architecture_identity() {
	let root = fixture_root();
	let mut paths = fs::read_dir(&root)
		.unwrap_or_else(|error| panic!("read {}: {error}", root.display()))
		.map(|entry| entry.expect("read architecture fixture entry").path())
		.filter(|path| {
			path.extension()
				.is_some_and(|extension| extension == "gguf")
		})
		.collect::<Vec<_>>();
	paths.sort();
	assert!(
		paths.len() >= 100,
		"architecture corpus unexpectedly shrank to {} files",
		paths.len()
	);

	let mut architectures = BTreeSet::new();
	for path in &paths {
		let file = File::open(path).unwrap_or_else(|error| panic!("open {}: {error}", path.display()));
		let file_bytes = file
			.metadata()
			.unwrap_or_else(|error| panic!("inspect {}: {error}", path.display()))
			.len();
		let mut file = BufReader::new(file);
		let descriptor = inspect_gguf_model_stream(&mut file, limits(file_bytes))
			.unwrap_or_else(|error| panic!("inspect architecture for {}: {error}", path.display()));
		assert!(
			!descriptor.architecture().is_empty(),
			"{} has an empty architecture",
			path.display()
		);
		assert!(
			descriptor.tensor_count() > 0,
			"{} has no model tensors",
			path.display()
		);
		architectures.insert(descriptor.architecture().to_owned());
	}

	assert!(
		architectures.contains("llama"),
		"corpus did not exercise llama architecture identity"
	);
	assert!(
		architectures.contains("qwen3"),
		"corpus did not exercise qwen3 architecture identity"
	);
}

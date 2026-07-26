use std::cell::RefCell;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use recipe_ingest::{
	AmbiguousVectorModel, CategoricalEncodingModel, DatasetSourceErrorKind, IngestLimits, PreparationRequest,
	SemanticType, TrainFraction, VectorEvidence, VectorMetadata, distill_dataset, distill_datasets,
};
use zip::write::SimpleFileOptions;

#[derive(Debug)]
struct TestDirectory(PathBuf);

impl TestDirectory {
	fn new() -> Self {
		static NEXT: AtomicU64 = AtomicU64::new(1);
		for _ in 0..64 {
			let path = std::env::temp_dir().join(format!(
				"recipe-ingest-dataset-{}-{}",
				std::process::id(),
				NEXT.fetch_add(1, Ordering::Relaxed)
			));
			match fs::create_dir(&path) {
				Ok(()) => return Self(path),
				Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
				Err(error) => panic!("create test directory: {error}"),
			}
		}
		panic!("could not allocate test directory");
	}

	fn path(&self) -> &Path {
		&self.0
	}
}

impl Drop for TestDirectory {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.0);
	}
}

fn limits() -> IngestLimits {
	IngestLimits::new(1 << 20, 1_000, 128, 1 << 20).unwrap()
}

fn jpeg(width: u16, height: u16) -> Vec<u8> {
	let mut bytes = b"\xff\xd8\xff\xe0\x00\x07JFIF\0\xff\xc0\x00\x11\x08".to_vec();
	bytes.extend_from_slice(&height.to_be_bytes());
	bytes.extend_from_slice(&width.to_be_bytes());
	bytes.extend_from_slice(&[3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0]);
	bytes
}

#[derive(Clone, Copy)]
struct TextModel;

impl AmbiguousVectorModel for TextModel {
	fn classify(&self, _evidence: VectorEvidence) -> SemanticType {
		SemanticType::Text
	}
}

#[test]
fn headerless_extension_is_distilled_without_inventing_a_header_row() {
	let directory = TestDirectory::new();
	let path = directory.path().join("samples.data");
	fs::write(&path, b"1,red\n2,blue\n3,red\n").unwrap();

	let dataset = distill_dataset(&path, limits()).unwrap();
	assert_eq!(dataset.file_count(), 1);
	assert_eq!(dataset.sample_count(), 3);
	assert!(
		!dataset
			.table()
			.headers()
			.iter()
			.any(|header| header == b"source_path")
	);
	assert!(
		dataset
			.table()
			.headers()
			.iter()
			.any(|header| header == b"col1")
	);
	assert!(
		dataset
			.table()
			.headers()
			.iter()
			.any(|header| header == b"col2")
	);
	let inferred = dataset.infer_vectors(&CategoricalEncodingModel).unwrap();
	assert_eq!(
		inferred
			.vectors()
			.iter()
			.filter(|vector| matches!(vector.name(), b"col1" | b"col2"))
			.map(|vector| vector.semantic_type())
			.collect::<Vec<_>>(),
		[SemanticType::Numeric, SemanticType::Categorical]
	);
}

#[test]
fn zip_members_keep_folder_filename_hash_and_sample_semantics() {
	let directory = TestDirectory::new();
	let path = directory.path().join("images.zip");
	let file = fs::File::create(&path).unwrap();
	let mut archive = zip::ZipWriter::new(file);
	let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
	archive.start_file("cats/000001.png", options).unwrap();
	archive.write_all(b"\x89PNG\r\n\x1a\ncat").unwrap();
	archive
		.start_file("dogs/7f83b1657ff1fc53b92dc18148a1d65dfa13514f.png", options)
		.unwrap();
	archive.write_all(b"\x89PNG\r\n\x1a\ndog").unwrap();
	archive.finish().unwrap();

	let dataset = distill_dataset(&path, limits()).unwrap();
	assert_eq!(dataset.file_count(), 2);
	assert_eq!(dataset.sample_count(), 2);
	let inferred = dataset.infer_vectors(&CategoricalEncodingModel).unwrap();
	let semantic = |name: &[u8]| {
		inferred
			.vectors()
			.iter()
			.find(|vector| vector.name() == name)
			.map(|vector| vector.semantic_type())
	};
	assert_eq!(semantic(b"folder"), Some(SemanticType::Categorical));
	assert_eq!(semantic(b"file_name"), Some(SemanticType::Categorical));
	assert_eq!(semantic(b"file_stem"), Some(SemanticType::Categorical));
	assert_eq!(semantic(b"content_sha256"), Some(SemanticType::Categorical));
	assert_eq!(semantic(b"sample_index"), Some(SemanticType::Numeric));
	assert_eq!(semantic(b"sample_count"), Some(SemanticType::Numeric));
	assert_eq!(semantic(b"image"), Some(SemanticType::Image));

	let modeled = dataset.infer_vectors(&TextModel).unwrap();
	let modeled_semantic = |name: &[u8]| {
		modeled
			.vectors()
			.iter()
			.find(|vector| vector.name() == name)
			.map(|vector| vector.semantic_type())
	};
	assert_eq!(modeled_semantic(b"file_stem"), Some(SemanticType::Text));
	assert_eq!(
		modeled_semantic(b"content_sha256"),
		Some(SemanticType::Text)
	);
	assert_eq!(
		modeled_semantic(b"sample_count"),
		Some(SemanticType::Numeric)
	);
	assert_eq!(modeled_semantic(b"image"), Some(SemanticType::Image));
}

#[test]
fn directory_members_are_sorted_and_distilled_as_samples() {
	let directory = TestDirectory::new();
	let first = directory.path().join("cats");
	let second = directory.path().join("dogs");
	fs::create_dir_all(&first).unwrap();
	fs::create_dir_all(&second).unwrap();
	fs::write(first.join("000002.png"), b"\x89PNG\r\n\x1a\nsecond").unwrap();
	fs::write(first.join("000001.png"), b"\x89PNG\r\n\x1a\nfirst").unwrap();
	fs::write(second.join("hash.png"), b"\x89PNG\r\n\x1a\nthird").unwrap();

	let dataset = distill_dataset(directory.path(), limits()).unwrap();
	assert_eq!(dataset.file_count(), 3);
	assert_eq!(dataset.sample_count(), 3);
	let file_name = dataset
		.table()
		.headers()
		.iter()
		.position(|header| header == b"file_name")
		.unwrap();
	assert_eq!(
		dataset
			.table()
			.rows()
			.iter()
			.map(|row| row[file_name].as_slice())
			.collect::<Vec<_>>(),
		[
			b"000001.png".as_slice(),
			b"000002.png".as_slice(),
			b"hash.png".as_slice()
		]
	);
}

#[test]
fn distilled_prepare_preserves_exact_image_rules_and_fits_classified_state_on_train_only() {
	#[derive(Debug, Default)]
	struct RecordingModel {
		evidence: RefCell<Vec<VectorEvidence>>,
	}

	impl AmbiguousVectorModel for RecordingModel {
		fn classify(&self, evidence: VectorEvidence) -> SemanticType {
			self.evidence.borrow_mut().push(evidence);
			SemanticType::Categorical
		}
	}

	let directory = TestDirectory::new();
	for (name, width) in [("a.jpg", 10), ("b.jpg", 11), ("c.jpg", 20), ("d.jpg", 21)] {
		fs::write(directory.path().join(name), jpeg(width, 7)).unwrap();
	}
	let dataset = distill_dataset(directory.path(), limits()).unwrap();
	let model = RecordingModel::default();
	let prepared = dataset
		.prepare(
			&PreparationRequest::new(["sample_index"], TrainFraction::new(1, 2).unwrap()),
			&model,
		)
		.unwrap();
	let image = prepared
		.vectors()
		.iter()
		.find(|vector| vector.name() == b"image")
		.unwrap();
	assert_eq!(image.semantic_type(), SemanticType::Image);
	let VectorMetadata::Image { encoded_variants } = image.metadata() else {
		panic!("distilled exact image rule must remain an image schema");
	};
	assert_eq!(
		encoded_variants
			.iter()
			.map(|variant| (variant.width(), variant.height()))
			.collect::<Vec<_>>(),
		[(10, 7), (11, 7)]
	);
	assert!(!model.evidence.borrow().is_empty());
	assert!(
		model.evidence
			.borrow()
			.iter()
			.all(|evidence| evidence.values == 2)
	);
}

#[test]
fn declared_sources_append_rows_in_order_and_retain_source_identity() {
	let directory = TestDirectory::new();
	let first = directory.path().join("first.csv");
	let second = directory.path().join("second.csv");
	fs::write(&first, b"value,label\n10,yes\n11,no\n").unwrap();
	fs::write(&second, b"value,label\n20,no\n21,yes\n").unwrap();

	let dataset = distill_datasets([first.as_path(), second.as_path()], limits()).unwrap();
	assert_eq!(dataset.file_count(), 2);
	assert_eq!(dataset.sample_count(), 4);
	assert_eq!(
		dataset
			.infer_vectors(&CategoricalEncodingModel)
			.unwrap()
			.vectors()
			.iter()
			.find(|vector| vector.name() == b"source_index")
			.unwrap()
			.semantic_type(),
		SemanticType::Categorical
	);
	let column = |name: &[u8]| {
		dataset
			.table()
			.headers()
			.iter()
			.position(|header| header == name)
			.unwrap()
	};
	let source_index = column(b"source_index");
	let source_path = column(b"source_path");
	let value = column(b"value");
	assert_eq!(
		dataset
			.table()
			.rows()
			.iter()
			.map(|row| row[source_index].as_slice())
			.collect::<Vec<_>>(),
		[
			b"0".as_slice(),
			b"0".as_slice(),
			b"1".as_slice(),
			b"1".as_slice()
		]
	);
	assert_eq!(
		dataset
			.table()
			.rows()
			.iter()
			.map(|row| row[source_path].as_slice())
			.collect::<Vec<_>>(),
		[
			first.to_string_lossy().as_bytes(),
			first.to_string_lossy().as_bytes(),
			second.to_string_lossy().as_bytes(),
			second.to_string_lossy().as_bytes(),
		]
	);
	assert_eq!(
		dataset
			.table()
			.rows()
			.iter()
			.map(|row| row[value].as_slice())
			.collect::<Vec<_>>(),
		[
			b"10".as_slice(),
			b"11".as_slice(),
			b"20".as_slice(),
			b"21".as_slice()
		]
	);

	let duplicate = distill_datasets([first.as_path(), first.as_path()], limits()).unwrap();
	let source_index = duplicate
		.table()
		.headers()
		.iter()
		.position(|header| header == b"source_index")
		.unwrap();
	assert_eq!(
		duplicate
			.table()
			.rows()
			.iter()
			.map(|row| row[source_index].as_slice())
			.collect::<Vec<_>>(),
		[
			b"0".as_slice(),
			b"0".as_slice(),
			b"1".as_slice(),
			b"1".as_slice()
		]
	);
}

#[test]
fn declared_source_collection_rejects_an_empty_list() {
	let paths: [&Path; 0] = [];
	let error = distill_datasets(paths, limits()).unwrap_err();
	assert_eq!(error.kind, DatasetSourceErrorKind::EmptySource);
}

#[test]
fn declared_files_directories_and_zips_share_one_ordered_source_list() {
	let root = TestDirectory::new();
	let first = root.path().join("first.csv");
	fs::write(&first, b"value,label\n10,yes\n").unwrap();

	let folder = root.path().join("folder");
	fs::create_dir(&folder).unwrap();
	fs::write(folder.join("second.csv"), b"value,label\n20,no\n").unwrap();

	let zip_path = root.path().join("third.zip");
	let file = fs::File::create(&zip_path).unwrap();
	let mut archive = zip::ZipWriter::new(file);
	archive
		.start_file("nested/third.csv", SimpleFileOptions::default())
		.unwrap();
	archive.write_all(b"value,label\n30,yes\n").unwrap();
	archive.finish().unwrap();

	let dataset = distill_datasets(
		[first.as_path(), folder.as_path(), zip_path.as_path()],
		limits(),
	)
	.unwrap();
	assert_eq!(dataset.file_count(), 3);
	assert_eq!(dataset.sample_count(), 3);
	let source_index = dataset
		.table()
		.headers()
		.iter()
		.position(|header| header == b"source_index")
		.unwrap();
	let value = dataset
		.table()
		.headers()
		.iter()
		.position(|header| header == b"value")
		.unwrap();
	assert_eq!(
		dataset
			.table()
			.rows()
			.iter()
			.map(|row| (row[source_index].as_slice(), row[value].as_slice()))
			.collect::<Vec<_>>(),
		[
			(b"0".as_slice(), b"10".as_slice()),
			(b"1".as_slice(), b"20".as_slice()),
			(b"2".as_slice(), b"30".as_slice()),
		]
	);
}

#[test]
fn encoded_model_files_become_named_vectors_without_cpu_numeric_decoding() {
	let directory = TestDirectory::new();
	let safetensors = directory.path().join("model.safetensors");
	let header = r#"{
		"__metadata__":{"format":"recipe-test"},
		"weights":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}
	}"#;
	let mut image = Vec::new();
	image.extend_from_slice(&u64::try_from(header.len()).unwrap().to_le_bytes());
	image.extend_from_slice(header.as_bytes());
	image.extend_from_slice(&1.25_f32.to_le_bytes());
	image.extend_from_slice(&(-2.5_f32).to_le_bytes());
	fs::write(&safetensors, image).unwrap();

	let dataset = distill_dataset(&safetensors, limits()).unwrap();
	let inferred = dataset.infer_vectors(&CategoricalEncodingModel).unwrap();
	assert!(
		inferred.vectors().iter().any(|vector| {
			vector.name() == b"tensor_name" && vector.semantic_type() == SemanticType::Categorical
		})
	);
	assert!(
		inferred
			.vectors()
			.iter()
			.any(|vector| vector.name() == b"binary" && vector.semantic_type() == SemanticType::Binary)
	);

	let disguised_zip = directory.path().join("pytorch_model.bin");
	let file = fs::File::create(&disguised_zip).unwrap();
	let mut archive = zip::ZipWriter::new(file);
	archive
		.start_file("weights/data", SimpleFileOptions::default())
		.unwrap();
	archive.write_all(b"\x00\x01\x02\x03").unwrap();
	archive.finish().unwrap();
	let dataset = distill_dataset(&disguised_zip, limits()).unwrap();
	assert_eq!(dataset.file_count(), 1);
	assert!(
		dataset
			.table()
			.headers()
			.iter()
			.any(|header| header == b"file_name")
	);
	assert!(
		dataset
			.infer_vectors(&CategoricalEncodingModel)
			.unwrap()
			.vectors()
			.iter()
			.any(|vector| vector.name() == b"binary" && vector.semantic_type() == SemanticType::Binary)
	);
}

#[test]
fn repository_formats_distill_into_nonempty_vector_lists() {
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/datasets");
	let limits = IngestLimits::new(64 << 20, 1_000_000, 16_384, 16 << 20).unwrap();
	for relative in [
		"uci-sonar/sonar.all-data",
		"llm-classification-finetuning/sample_submission.csv",
		"uci-airfoil/airfoil_self_noise.dat",
		"uci-wine/wine.data",
		"uci-german-numeric/german.data-numeric",
		"cosmetics/makeup_data.json",
		"gguf-test/ggml-vocab-gemma-4.gguf",
		"llamacpp-vocabs-tokenizer/ggml-vocab-gemma-4.gguf.inp",
		"llamacpp-vocabs-tokenizer/ggml-vocab-bert-bge.gguf.out",
		"lendingclub-2007-2011/Data_Dictionary.xlsx",
		"llamacpp-archs-seed42/qwen35-dense.logits",
		"llamacpp-ci-models/test-llama-archs-logits-generator.patch",
		"predict-the-handwriting-images/train_images/train_0086.png",
		"rogii-wellbore-geology-prediction/AI_wellbore_geology_prediction_task_en.pptx",
		"sandp500/merge.sh",
		"tokenizers/t5-small/spiece.model",
		"uci-optdigits/optdigits.tra",
		"uci-satimage/sat.trn",
		"llamacpp-archs-seed42/tokens.txt",
		"uci-sms-tab/SMSSpamCollection",
		"uci-sms-tab/smsspamcollection.zip",
		"uci-bank-semicolon/bank.zip",
	] {
		let dataset = distill_dataset(&root.join(relative), limits).unwrap_or_else(|error| {
			panic!("distill {relative}: {error}");
		});
		assert!(dataset.sample_count() > 0, "{relative}");
		assert!(dataset.vector_count() > 0, "{relative}");
		dataset
			.infer_vectors(&CategoricalEncodingModel)
			.unwrap_or_else(|error| panic!("infer {relative}: {error}"));
	}
}

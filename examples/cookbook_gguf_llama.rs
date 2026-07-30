//! Execute the checked-in dense-F32 llama GGUF instrument on exact token IDs.
//!
//! Run `cargo run --bin recipe -- probe` once, then
//! `cargo run --example cookbook_gguf_llama`.

use recipe::*;

const CORPUS: &str = "examples/datasets/llamacpp-archs-seed42";
const TOKENS: &str = "examples/datasets/llamacpp-archs-seed42/tokens.txt";
const MODEL: &str = "examples/datasets/llamacpp-archs-seed42/llama-dense.gguf";
const REFERENCE: &str = "examples/datasets/llamacpp-archs-seed42/llama-dense.logits";

fn main() -> Result<(), Box<dyn std::error::Error>> {
	recipe.data(TOKENS);
	recipe.model().load(MODEL);
	let report = recipe.infer().log([Time, Device]).evaluate()?;
	assert_eq!(report.kind(), InferenceModelKind::GgufLlama);
	let actual = report.values().collect::<Vec<_>>();
	let expected = read_reference_logits(REFERENCE)?;
	assert_eq!(actual.len(), 128 * 128);
	assert_eq!(actual.len(), expected.len());

	let mut squared_error = 0.0_f64;
	let mut reference_energy = 0.0_f64;
	let mut maximum_error = 0.0_f32;
	for (actual, expected) in actual.iter().zip(&expected) {
		let error = actual - expected;
		squared_error += f64::from(error) * f64::from(error);
		reference_energy += f64::from(*expected) * f64::from(*expected);
		maximum_error = maximum_error.max(error.abs());
	}
	let normalized_mean_square_error = squared_error / reference_energy;
	println!(
		"llama.cpp parity\tcorpus\t{CORPUS}\tnmse\t{normalized_mean_square_error:.9e}\tmax_abs\t{maximum_error:.9e}"
	);
	assert!(
		normalized_mean_square_error < 1.0e-3,
		"GGUF llama logits diverged from the checked-in llama.cpp CPU oracle"
	);
	Ok(())
}

fn read_reference_logits(path: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
	let bytes = std::fs::read(path)?;
	if bytes.get(..4) != Some(b"LGT0") || bytes.len() < 12 {
		return Err("reference logits do not have an LGT0 header".into());
	}
	let positions = u32::from_le_bytes(bytes[4..8].try_into()?) as usize;
	let vocabulary = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
	let values = bytes[12..]
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one complete LGT0 f32")))
		.collect::<Vec<_>>();
	if values.len()
		!= positions
			.checked_mul(vocabulary)
			.ok_or("LGT0 shape overflowed")?
	{
		return Err("reference logit count differs from its LGT0 header".into());
	}
	Ok(values)
}

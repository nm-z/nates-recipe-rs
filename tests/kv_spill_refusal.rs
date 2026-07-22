
use gpu_core::memory::{USER_GB, carve_miss_message};
use recipe_infer::human_bytes;

#[test]
fn sized_refusal_renders_in_gb_not_raw_bytes() {
	let one_gb = human_bytes(1 << 30);
	assert!(
		one_gb.ends_with("GB"),
		"1 GiB rendered as {one_gb:?}, expected a GB magnitude"
	);
	assert!(
		one_gb.contains("1.00"),
		"1 GiB rendered as {one_gb:?}, expected two-decimal 1.00"
	);
	let three_and_half = human_bytes((7u64 << 29) as usize);
	assert_eq!(
		three_and_half, "3.50 GB",
		"3.5 GiB must render as a two-decimal GB magnitude"
	);
	let big = human_bytes(1usize << 40);
	assert!(
		big.ends_with("GB"),
		"1 TiB rendered as {big:?}, expected a GB magnitude"
	);
	let raw = format!("{}", 3u64 << 30);
	assert_ne!(
		three_and_half, raw,
		"the sized message must not be a bare byte count"
	);
	eprintln!("sized rendering: {one_gb}, {three_and_half}, {big}");
}

#[test]
fn reserve_is_one_gib_not_a_magic_threshold() {
	assert_eq!(
		USER_GB,
		1 << 30,
		"USER_GB is the single headroom constant the refusal subtracts; a shortfall computed against anything else is a magic threshold"
	);
	eprintln!("USER_GB = {} bytes ({})", USER_GB, human_bytes(USER_GB));
}

#[test]
fn carve_miss_refusal_is_a_sized_message_not_a_fault() {
	let msg = carve_miss_message(64usize << 30);
	assert!(
		msg.contains("GB"),
		"carve-miss refusal {msg:?} carries no GB magnitude — the refusal must be sized"
	);
	assert!(
		msg.contains("VRAM"),
		"carve-miss refusal {msg:?} does not name the tier it exhausted"
	);
	assert!(
		!msg.is_empty(),
		"carve-miss refusal produced an empty message"
	);
	eprintln!("carve-miss refusal message:\n{msg}");
}

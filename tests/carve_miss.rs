use gpu_core::memory::{
	GpuBuffer, arena_remaining, carve_miss_message, claim_device_arena_bytes, release_device_arena,
};

#[test]
fn carve_miss_returns_error_with_human_message() {
	let remain_bytes = 9_090_304usize;
	let asked_bytes = 11_730_944usize;
	let Some(slab) = claim_device_arena_bytes(remain_bytes) else {
		panic!("could not claim a {remain_bytes}-byte arena for the carve-miss test");
	};
	assert_eq!(arena_remaining(), remain_bytes);
	let carve = GpuBuffer::alloc_bytes(asked_bytes);
	assert!(
		carve.is_err(),
		"an over-claim carve must return Err, not an allocation"
	);
	let msg = carve_miss_message(asked_bytes);
	eprintln!("{msg}");
	assert!(
		msg.contains("11.19 MB"),
		"needs magnitude in message: {msg}"
	);
	assert!(
		msg.contains("8.67 MB"),
		"remaining magnitude in message: {msg}"
	);
	assert!(
		msg.contains("2.52 MB short"),
		"short magnitude in message: {msg}"
	);
	assert!(
		msg.contains("GB free /"),
		"VRAM context line in message: {msg}"
	);
	release_device_arena(slab);
}

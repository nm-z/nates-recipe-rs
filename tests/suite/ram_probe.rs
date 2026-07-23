use gpu_core::memory::ram_probe_ask;
use std::fs;

#[test]
fn ram_probe_measures_mem_free() {
	let measured = ram_probe_ask();
	let text = match fs::read_to_string("/proc/meminfo") {
		Ok(t) => t,
		Err(e) => panic!("/proc/meminfo: {e}"),
	};
	let Some(total) = text
		.lines()
		.find_map(|l| return l.strip_prefix("MemTotal:"))
		.and_then(|rest| {
			return rest.trim().trim_end_matches("kB").trim().parse::<usize>().ok();
		})
		.map(|kb| return kb.saturating_mul(1024))
	else {
		panic!("MemTotal missing from /proc/meminfo");
	};
	assert!(measured > 0, "MemAvailable measured as 0 bytes");
	assert!(
		measured <= total,
		"MemAvailable {measured} exceeds MemTotal {total}"
	);
}

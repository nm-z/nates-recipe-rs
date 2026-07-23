use gpu_core::memory::GpuBuffer;
use recipe_infer::llm::{FWD_DT, HostStore};
use std::{env, fs, process};

#[test]
fn ram_boundary_then_disk_roundtrip_and_stage() {
	let es = FWD_DT.elem_size();
	let w = 8usize;
	let path = env::temp_dir().join(format!("recipe-kv-test-{}.spill", process::id()));
	let mut st = HostStore::new(3 * w * es, path.clone());
	let row = |r: usize| -> Vec<f32> {
		return (0..2 * w)
			.map(|i| return (r * 100 + i) as f32 * 0.5)
			.collect();
	};
	st.append_f32(&row(0)).expect("append rows 0-1");
	assert_eq!(st.ram.len(), 2 * w * es, "first append stays in RAM");
	assert_eq!(st.file_len, 0);
	st.append_f32(&row(2)).expect("append rows 2-3");
	assert_eq!(
		st.ram.len(),
		2 * w * es,
		"RAM frozen at the boundary append"
	);
	assert_eq!(
		st.file_len,
		2 * w * es,
		"over-budget append went whole to disk"
	);
	assert!(fs::metadata(&path).is_ok(), "spill file exists on disk");
	st.append_f32(&row(4)).expect("append rows 4-5");
	assert_eq!(st.len(), 6 * w * es);
	let mut expect: Vec<f32> = Vec::new();
	for r in [0usize, 2, 4] {
		expect.extend_from_slice(&row(r));
	}
	let dst = GpuBuffer::alloc_ty(6 * w, FWD_DT).expect("stage dst");
	let mut scratch = Vec::new();
	st.stage_into(0, 6 * w * es, &dst, &mut scratch)
		.expect("stage all 6 rows");
	let mut got = vec![0.0f32; 6 * w];
	dst.download_f32(&mut got).expect("download");
	assert_eq!(
		got, expect,
		"full-range stage must return the exact stored bytes"
	);
	let dst2 = GpuBuffer::alloc_ty(3 * w, FWD_DT).expect("straddle dst");
	st.stage_into(w * es, 3 * w * es, &dst2, &mut scratch)
		.expect("stage rows 1..4");
	let mut got2 = vec![0.0f32; 3 * w];
	dst2.download_f32(&mut got2).expect("download straddle");
	assert_eq!(
		got2,
		expect[w..4 * w],
		"RAM/file straddling stage must be byte-exact"
	);
	st.append_f32(&row(6))
		.expect("append rows 6-7 after a read");
	let dst3 = GpuBuffer::alloc_ty(2 * w, FWD_DT).expect("post-append dst");
	st.stage_into(6 * w * es, 2 * w * es, &dst3, &mut scratch)
		.expect("stage rows 6..8 through the cached read descriptor");
	let mut got3 = vec![0.0f32; 2 * w];
	dst3.download_f32(&mut got3).expect("download post-append");
	assert_eq!(
		got3,
		row(6),
		"bytes written through the write descriptor must be visible through the independent read descriptor"
	);
	st.truncate(3 * w * es);
	assert_eq!(st.len(), 3 * w * es);
	assert_eq!(
		st.file_len,
		w * es,
		"truncate above the watermark shrinks only the file"
	);
	st.truncate(w * es);
	assert_eq!(
		st.file_len, 0,
		"truncate below the watermark drops the file"
	);
	assert!(
		fs::metadata(&path).is_err(),
		"spill file deleted on truncate below RAM"
	);
	st.append_f32(&row(9)).expect("append after truncate");
	assert_eq!(
		st.ram.len(),
		3 * w * es,
		"RAM regrows to its budget once the file is gone"
	);
	drop(st);
	assert!(
		fs::metadata(&path).is_err(),
		"no spill file left after drop"
	);
}

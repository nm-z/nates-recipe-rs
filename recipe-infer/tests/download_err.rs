use recipe_infer::{GpuBuffer, download_vec};

#[test]
fn download_from_null_buffer_errs_not_aborts() {
	let bogus = GpuBuffer::borrow(std::ptr::null_mut(), 8);
	let got = download_vec(&bogus, 1);
	assert!(
		got.is_err(),
		"download from a null device pointer must surface as Err, not SIGABRT"
	);
}

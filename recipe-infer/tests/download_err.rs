use recipe_infer::{GpuBuffer, download_vec};

/// A D2H download from an invalid (null) device pointer used to `process::abort()`
/// inside `download_vec`; it now stamps an `ERROR:` line and returns the failure up
/// the forward graph. This drives that exact converted path: a null-borrowed buffer
/// forces `hipMemcpyAsync` to reject at API validation (no kernel dispatch, no page
/// fault, no real device memory touched), so the process must survive and hand back
/// an `Err` instead of killing itself.
#[test]
fn download_from_null_buffer_errs_not_aborts() {
	let bogus = GpuBuffer::borrow(std::ptr::null_mut(), 8);
	let got = download_vec(&bogus, 1);
	assert!(
		got.is_err(),
		"download from a null device pointer must surface as Err, not SIGABRT"
	);
}

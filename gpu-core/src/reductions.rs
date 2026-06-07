use crate::hip::HipError;
use crate::kernels::{check_launch, safe_i32};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_sum_all_workspace_bytes(n: i32) -> usize;
	fn launch_sum_all(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_max_all_workspace_bytes(n: i32) -> usize;
	fn launch_max_all(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_min_all_workspace_bytes(n: i32) -> usize;
	fn launch_min_all(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_mean_all_workspace_bytes(n: i32) -> usize;
	fn launch_mean_all(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_l2_norm_workspace_bytes(n: i32) -> usize;
	fn launch_l2_norm(
		x: *const c_void,
		sq: *mut c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_dot_workspace_bytes(n: i32) -> usize;
	fn launch_dot(
		a: *const c_void,
		b: *const c_void,
		prod: *mut c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_fill_sentinel(
		data: *mut c_void,
		real_n: i32,
		padded_n: i32,
		sentinel: f64,
		stream: *mut c_void,
	);
	fn launch_init_idx(idx: *mut c_void, n: i32, stream: *mut c_void);
	fn launch_bitonic_step(data: *mut c_void, j: i32, k: i32, padded_n: i32, stream: *mut c_void);
	fn launch_bitonic_step_idx(
		keys: *mut c_void,
		vals: *mut c_void,
		j: i32,
		k: i32,
		padded_n: i32,
		stream: *mut c_void,
	);
	fn launch_bitonic_step_dd(
		keys: *mut c_void,
		vals: *mut c_void,
		j: i32,
		k: i32,
		padded_n: i32,
		stream: *mut c_void,
	);

	fn launch_segment_sort(
		data: *const c_void,
		seg_offsets: *const c_void,
		out: *mut c_void,
		n: i32,
		n_segs: i32,
		stream: *mut c_void,
	);

	fn launch_cumsum_rows(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_cumsum_cols(
		x: *const c_void,
		out: *mut c_void,
		rows: i32,
		cols: i32,
		stream: *mut c_void,
	);
	fn launch_cumprod_workspace_bytes(n: i32) -> usize;
	fn launch_cumprod(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);
	fn launch_cummax_workspace_bytes(n: i32) -> usize;
	fn launch_cummax(
		x: *const c_void,
		out: *mut c_void,
		workspace: *mut c_void,
		workspace_bytes: usize,
		n: i32,
		stream: *mut c_void,
	);

	fn launch_segment_sum(
		vals: *const c_void,
		seg_ids: *const c_void,
		out: *mut c_void,
		n: i32,
		n_segs: i32,
		stream: *mut c_void,
	);
	fn launch_segment_max(
		vals: *const c_void,
		seg_ids: *const c_void,
		out: *mut c_void,
		n: i32,
		n_segs: i32,
		stream: *mut c_void,
	);

	fn launch_scan_linear_recurrence(
		a: *const c_void,
		b: *const c_void,
		states: *mut c_void,
		n_steps: i32,
		dim: i32,
		stream: *mut c_void,
	);
}

fn next_pow2(n: usize) -> usize {
	let mut p = 1usize;
	while p < n {
		p <<= 1;
	}
	p
}

fn scalar_reduce(
	wbytes: unsafe extern "C" fn(i32) -> usize,
	f: unsafe extern "C" fn(*const c_void, *mut c_void, *mut c_void, usize, i32, *mut c_void),
	x: &GpuBuffer,
	n: usize,
) -> Result<f64, HipError> {
	let ni = safe_i32(n);
	let wb = unsafe { wbytes(ni) };
	let ws = GpuBuffer::alloc_bytes(wb)?;
	let out = GpuBuffer::alloc(1)?;
	unsafe {
		f(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			ws.ptr_raw(),
			wb,
			ni,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0.0f64];
	out.download(&mut v)?;
	Ok(v[0])
}

pub fn gpu_sum_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	scalar_reduce(launch_sum_all_workspace_bytes, launch_sum_all, x, n)
}

pub fn gpu_max_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	scalar_reduce(launch_max_all_workspace_bytes, launch_max_all, x, n)
}

pub fn gpu_min_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	scalar_reduce(launch_min_all_workspace_bytes, launch_min_all, x, n)
}

pub fn gpu_mean_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	let s = scalar_reduce(launch_mean_all_workspace_bytes, launch_mean_all, x, n)?;
	Ok(s / n as f64)
}

pub fn gpu_l2_norm(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	let ni = safe_i32(n);
	let wb = unsafe { launch_l2_norm_workspace_bytes(ni) };
	let ws = GpuBuffer::alloc_bytes(wb)?;
	let sq = GpuBuffer::alloc(n)?;
	let out = GpuBuffer::alloc(1)?;
	unsafe {
		launch_l2_norm(
			x.ptr_raw() as *const c_void,
			sq.ptr_raw(),
			out.ptr_raw(),
			ws.ptr_raw(),
			wb,
			ni,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0.0f64];
	out.download(&mut v)?;
	Ok(v[0].sqrt())
}

pub fn gpu_dot(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<f64, HipError> {
	let ni = safe_i32(n);
	let wb = unsafe { launch_dot_workspace_bytes(ni) };
	let ws = GpuBuffer::alloc_bytes(wb)?;
	let prod = GpuBuffer::alloc(n)?;
	let out = GpuBuffer::alloc(1)?;
	unsafe {
		launch_dot(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			prod.ptr_raw(),
			out.ptr_raw(),
			ws.ptr_raw(),
			wb,
			ni,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	let mut v = [0.0f64];
	out.download(&mut v)?;
	Ok(v[0])
}

pub fn gpu_sort(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let pn = next_pow2(n);
	let mut work = GpuBuffer::alloc(pn)?;
	work.copy_from(x, n * 8)?;
	if pn > n {
		unsafe {
			launch_fill_sentinel(
				work.ptr_raw(),
				safe_i32(n),
				safe_i32(pn),
				f64::MAX,
				std::ptr::null_mut(),
			);
		}
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			unsafe {
				launch_bitonic_step(
					work.ptr_raw(),
					safe_i32(j),
					safe_i32(k),
					safe_i32(pn),
					std::ptr::null_mut(),
				);
			}
			j >>= 1;
		}
		k <<= 1;
	}
	check_launch();
	let mut out = GpuBuffer::alloc(n)?;
	out.copy_from(&work, n * 8)?;
	Ok(out)
}

pub fn gpu_argsort(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let pn = next_pow2(n);
	let mut keys = GpuBuffer::alloc(pn)?;
	let vals = GpuBuffer::alloc_bytes(pn * 4)?;
	keys.copy_from(x, n * 8)?;
	unsafe {
		launch_init_idx(vals.ptr_raw(), safe_i32(pn), std::ptr::null_mut());
	}
	if pn > n {
		unsafe {
			launch_fill_sentinel(
				keys.ptr_raw(),
				safe_i32(n),
				safe_i32(pn),
				f64::MAX,
				std::ptr::null_mut(),
			);
		}
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			unsafe {
				launch_bitonic_step_idx(
					keys.ptr_raw(),
					vals.ptr_raw(),
					safe_i32(j),
					safe_i32(k),
					safe_i32(pn),
					std::ptr::null_mut(),
				);
			}
			j >>= 1;
		}
		k <<= 1;
	}
	check_launch();
	let mut out = GpuBuffer::alloc_bytes(n * 4)?;
	out.copy_from(&vals, n * 4)?;
	Ok(out)
}

pub fn gpu_sort_by_key(
	keys: &GpuBuffer,
	vals: &GpuBuffer,
	n: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
	let pn = next_pow2(n);
	let mut wk = GpuBuffer::alloc(pn)?;
	let mut wv = GpuBuffer::alloc(pn)?;
	wk.copy_from(keys, n * 8)?;
	wv.copy_from(vals, n * 8)?;
	if pn > n {
		unsafe {
			launch_fill_sentinel(
				wk.ptr_raw(),
				safe_i32(n),
				safe_i32(pn),
				f64::MAX,
				std::ptr::null_mut(),
			);
		}
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			unsafe {
				launch_bitonic_step_dd(
					wk.ptr_raw(),
					wv.ptr_raw(),
					safe_i32(j),
					safe_i32(k),
					safe_i32(pn),
					std::ptr::null_mut(),
				);
			}
			j >>= 1;
		}
		k <<= 1;
	}
	check_launch();
	let mut out_keys = GpuBuffer::alloc(n)?;
	let mut out_vals = GpuBuffer::alloc(n)?;
	out_keys.copy_from(&wk, n * 8)?;
	out_vals.copy_from(&wv, n * 8)?;
	Ok((out_keys, out_vals))
}

pub fn gpu_segment_sort(
	data: &GpuBuffer,
	seg_offsets: &GpuBuffer,
	n: usize,
	n_segs: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_segment_sort(
			data.ptr_raw() as *const c_void,
			seg_offsets.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			safe_i32(n_segs),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_cumsum_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(rows * cols)?;
	unsafe {
		launch_cumsum_rows(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(rows),
			safe_i32(cols),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_cumsum_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(rows * cols)?;
	unsafe {
		launch_cumsum_cols(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(rows),
			safe_i32(cols),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_cumprod(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let ni = safe_i32(n);
	let wb = unsafe { launch_cumprod_workspace_bytes(ni) };
	let ws = GpuBuffer::alloc_bytes(wb)?;
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_cumprod(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			ws.ptr_raw(),
			wb,
			ni,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_cummax(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let ni = safe_i32(n);
	let wb = unsafe { launch_cummax_workspace_bytes(ni) };
	let ws = GpuBuffer::alloc_bytes(wb)?;
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_cummax(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			ws.ptr_raw(),
			wb,
			ni,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_segment_sum(
	vals: &GpuBuffer,
	seg_ids: &GpuBuffer,
	n: usize,
	n_segs: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::zeros_bytes(n_segs * 8)?;
	unsafe {
		launch_segment_sum(
			vals.ptr_raw() as *const c_void,
			seg_ids.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			safe_i32(n_segs),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_segment_max(
	vals: &GpuBuffer,
	seg_ids: &GpuBuffer,
	n: usize,
	n_segs: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n_segs)?;
	unsafe {
		launch_segment_max(
			vals.ptr_raw() as *const c_void,
			seg_ids.ptr_raw() as *const c_void,
			out.ptr_raw(),
			safe_i32(n),
			safe_i32(n_segs),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

pub fn gpu_scan_linear_recurrence(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n_steps: usize,
	dim: usize,
) -> Result<GpuBuffer, HipError> {
	let states = GpuBuffer::alloc(n_steps * dim)?;
	unsafe {
		launch_scan_linear_recurrence(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			states.ptr_raw(),
			safe_i32(n_steps),
			safe_i32(dim),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(states)
}

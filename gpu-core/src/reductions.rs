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
		sentinel: *const c_void,
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

pub fn gpu_sum_all_workspace_bytes(n: usize) -> usize {
	unsafe { launch_sum_all_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_max_all_workspace_bytes(n: usize) -> usize {
	unsafe { launch_max_all_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_min_all_workspace_bytes(n: usize) -> usize {
	unsafe { launch_min_all_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_mean_all_workspace_bytes(n: usize) -> usize {
	unsafe { launch_mean_all_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_l2_norm_workspace_bytes(n: usize) -> usize {
	unsafe { launch_l2_norm_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_dot_workspace_bytes(n: usize) -> usize {
	unsafe { launch_dot_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_cumprod_workspace_bytes(n: usize) -> usize {
	unsafe { launch_cumprod_workspace_bytes(safe_i32(n)) }
}
pub fn gpu_cummax_workspace_bytes(n: usize) -> usize {
	unsafe { launch_cummax_workspace_bytes(safe_i32(n)) }
}

fn scalar_reduce(
	f: unsafe extern "C" fn(*const c_void, *mut c_void, *mut c_void, usize, i32, *mut c_void),
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		f(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			workspace.ptr_raw(),
			workspace.len(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sum_all(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	scalar_reduce(launch_sum_all, x, workspace, n, out)
}

pub fn gpu_max_all(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	scalar_reduce(launch_max_all, x, workspace, n, out)
}

pub fn gpu_min_all(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	scalar_reduce(launch_min_all, x, workspace, n, out)
}

pub fn gpu_mean_all(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	scalar_reduce(launch_mean_all, x, workspace, n, out)
}

pub fn gpu_l2_norm(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	sq: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_l2_norm(
			x.ptr_raw() as *const c_void,
			sq.ptr_raw(),
			out.ptr_raw(),
			workspace.ptr_raw(),
			workspace.len(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_dot(
	a: &GpuBuffer,
	b: &GpuBuffer,
	workspace: &GpuBuffer,
	prod: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_dot(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			prod.ptr_raw(),
			out.ptr_raw(),
			workspace.ptr_raw(),
			workspace.len(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}


pub fn gpu_fill_sentinel(
	data: &GpuBuffer,
	real_n: usize,
	padded_n: usize,
	sentinel: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_fill_sentinel(
			data.ptr_raw(),
			safe_i32(real_n),
			safe_i32(padded_n),
			sentinel.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_init_idx(n: usize, idx: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_init_idx(idx.ptr_raw(), safe_i32(n), std::ptr::null_mut());
	}
	check_launch();
	Ok(())
}

pub fn gpu_bitonic_step(
	j: usize,
	k: usize,
	padded_n: usize,
	data: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bitonic_step(
			data.ptr_raw(),
			safe_i32(j),
			safe_i32(k),
			safe_i32(padded_n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_bitonic_step_idx(
	j: usize,
	k: usize,
	padded_n: usize,
	keys: &GpuBuffer,
	vals: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bitonic_step_idx(
			keys.ptr_raw(),
			vals.ptr_raw(),
			safe_i32(j),
			safe_i32(k),
			safe_i32(padded_n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_bitonic_step_dd(
	j: usize,
	k: usize,
	padded_n: usize,
	keys: &GpuBuffer,
	vals: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_bitonic_step_dd(
			keys.ptr_raw(),
			vals.ptr_raw(),
			safe_i32(j),
			safe_i32(k),
			safe_i32(padded_n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_sort(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let pn = next_pow2(n);
	let mut work = GpuBuffer::alloc(pn)?;
	work.copy_from(x, n * 8)?;
	if pn > n {
		let sentinel = GpuBuffer::alloc(1)?;
		sentinel.load(&[f64::MAX])?;
		gpu_fill_sentinel(&work, n, pn, &sentinel)?;
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			gpu_bitonic_step(j, k, pn, &work)?;
			j >>= 1;
		}
		k <<= 1;
	}
	let mut dst = GpuBuffer::borrow(out.ptr_raw(), out.len());
	dst.copy_from(&work, n * 8)?;
	Ok(())
}

pub fn gpu_argsort(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let pn = next_pow2(n);
	let mut keys = GpuBuffer::alloc(pn)?;
	let vals = GpuBuffer::alloc_bytes(pn * 4)?;
	keys.copy_from(x, n * 8)?;
	gpu_init_idx(pn, &vals)?;
	if pn > n {
		let sentinel = GpuBuffer::alloc(1)?;
		sentinel.load(&[f64::MAX])?;
		gpu_fill_sentinel(&keys, n, pn, &sentinel)?;
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			gpu_bitonic_step_idx(j, k, pn, &keys, &vals)?;
			j >>= 1;
		}
		k <<= 1;
	}
	let mut dst = GpuBuffer::borrow(out.ptr_raw(), out.len());
	dst.copy_from(&vals, n * 4)?;
	Ok(())
}

pub fn gpu_sort_by_key(
	keys: &GpuBuffer,
	vals: &GpuBuffer,
	n: usize,
	out_keys: &GpuBuffer,
	out_vals: &GpuBuffer,
) -> Result<(), HipError> {
	let pn = next_pow2(n);
	let mut wk = GpuBuffer::alloc(pn)?;
	let mut wv = GpuBuffer::alloc(pn)?;
	wk.copy_from(keys, n * 8)?;
	wv.copy_from(vals, n * 8)?;
	if pn > n {
		let sentinel = GpuBuffer::alloc(1)?;
		sentinel.load(&[f64::MAX])?;
		gpu_fill_sentinel(&wk, n, pn, &sentinel)?;
	}
	let mut k = 2usize;
	while k <= pn {
		let mut j = k >> 1;
		while j > 0 {
			gpu_bitonic_step_dd(j, k, pn, &wk, &wv)?;
			j >>= 1;
		}
		k <<= 1;
	}
	let mut dk = GpuBuffer::borrow(out_keys.ptr_raw(), out_keys.len());
	dk.copy_from(&wk, n * 8)?;
	let mut dv = GpuBuffer::borrow(out_vals.ptr_raw(), out_vals.len());
	dv.copy_from(&wv, n * 8)?;
	Ok(())
}

pub fn gpu_segment_sort(
	data: &GpuBuffer,
	seg_offsets: &GpuBuffer,
	n: usize,
	n_segs: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

pub fn gpu_cumsum_rows(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

pub fn gpu_cumsum_cols(
	x: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

pub fn gpu_cumprod(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_cumprod(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			workspace.ptr_raw(),
			workspace.len(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_cummax(
	x: &GpuBuffer,
	workspace: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_cummax(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			workspace.ptr_raw(),
			workspace.len(),
			safe_i32(n),
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

pub fn gpu_segment_sum(
	vals: &GpuBuffer,
	seg_ids: &GpuBuffer,
	n: usize,
	n_segs: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

pub fn gpu_segment_max(
	vals: &GpuBuffer,
	seg_ids: &GpuBuffer,
	n: usize,
	n_segs: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

pub fn gpu_scan_linear_recurrence(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n_steps: usize,
	dim: usize,
	states: &GpuBuffer,
) -> Result<(), HipError> {
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
	Ok(())
}

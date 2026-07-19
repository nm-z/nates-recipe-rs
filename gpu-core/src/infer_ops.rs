use crate::HipError;
use crate::callspy;
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::log::Write;
use crate::memory::{Dtype, GpuBuffer};
use core::ffi::c_void;
use core::ptr;

fn cl() -> Result<(), HipError> {
	callspy::tick(&callspy::LAUNCH);
	callspy::tick(&callspy::GET_LAST_ERROR);
	// SAFETY: hipGetLastError only reads and clears the thread-local HIP error state; it dereferences no memory.
	return check(unsafe { hipGetLastError() });
}

unsafe extern "C" {
	fn launch_convert(
		src: *const c_void,
		dst: *mut c_void,
		n: i64,
		scale: f64,
		sdt: i32,
		ddt: i32,
		stream: *mut c_void,
	);
	fn launch_embed_blend(
		rowsrc: *const c_void,
		w: *const c_void,
		out: *mut c_void,
		rows: i64,
		k: i32,
		ne: i32,
		scale: f64,
		sdt: i32,
		ddt: i32,
		stream: *mut c_void,
	);
	fn launch_normx_rmsnorm(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_flash_gqa(
		q: *const c_void,
		kc: *const c_void,
		vc: *const c_void,
		out: *mut c_void,
		t_q: i32,
		t_kv: i32,
		nqh: i32,
		nkv: i32,
		hd: i32,
		max_bias: f64,
		p_base: i32,
		causal_below: i32,
		m_io: *mut c_void,
		l_io: *mut c_void,
		acc_io: *mut c_void,
		kv_off: i32,
		finalize: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_flash_mla(
		q: *const c_void,
		kc: *const c_void,
		vc: *const c_void,
		out: *mut c_void,
		t_q: i32,
		t_kv: i32,
		nqh: i32,
		nkv: i32,
		hdk: i32,
		hdv: i32,
		p_base: i32,
		causal_below: i32,
		m_io: *mut c_void,
		l_io: *mut c_void,
		acc_io: *mut c_void,
		kv_off: i32,
		finalize: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_gelu_mul(
		a: *const c_void,
		b: *const c_void,
		out: *mut c_void,
		n: i64,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_glu_gelu(
		input: *const c_void,
		out: *mut c_void,
		rows: i32,
		half: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_rope_partial(
		buf: *mut c_void,
		rows: i32,
		head_dim: i32,
		rotary_dim: i32,
		heads_per_tok: i32,
		theta: *const c_void,
		factors: *const c_void,
		pos_base: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_gemm_bt(
		a: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_gemm_bt_tiles(
		a: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		tile_m: i32,
		tile_n: i32,
		tile_k: i32,
		stream: *mut c_void,
		dtype: i32,
	);
	fn launch_scale_f64(x: *mut c_void, scalar: *const c_void, n: i64, stream: *mut c_void, dtype: i32);
	fn launch_glu_silu(
		input: *const c_void,
		out: *mut c_void,
		rows: i32,
		half: i32,
		stream: *mut c_void,
		dtype: i32,
	);
}

#[inline]
pub fn gpu_rope_partial(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buf and theta are live GpuBuffer allocations and the dims are range-checked i32; the launcher only reads them.
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			ci(rows)?,
			ci(head_dim)?,
			ci(rotary_dim)?,
			ci(heads_per_tok)?,
			theta.ptr_raw().cast_const(),
			ptr::null(),
			0,
			ptr::null_mut(),
			buf.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_rope_partial_pos(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	pos_base: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buf and theta are live GpuBuffer allocations and the dims are range-checked i32; the launcher only reads them.
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			ci(rows)?,
			ci(head_dim)?,
			ci(rotary_dim)?,
			ci(heads_per_tok)?,
			theta.ptr_raw().cast_const(),
			ptr::null(),
			ci(pos_base)?,
			ptr::null_mut(),
			buf.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_rope_partial_factors(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	factors: &GpuBuffer,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buf, theta and factors are live GpuBuffer allocations and the dims are range-checked i32; the launcher only reads them.
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			ci(rows)?,
			ci(head_dim)?,
			ci(rotary_dim)?,
			ci(heads_per_tok)?,
			theta.ptr_raw().cast_const(),
			factors.ptr_raw().cast_const(),
			0,
			ptr::null_mut(),
			buf.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_rope_partial_factors_pos(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	factors: &GpuBuffer,
	pos_base: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: buf, theta and factors are live GpuBuffer allocations and the dims are range-checked i32; the launcher only reads them.
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			ci(rows)?,
			ci(head_dim)?,
			ci(rotary_dim)?,
			ci(heads_per_tok)?,
			theta.ptr_raw().cast_const(),
			factors.ptr_raw().cast_const(),
			ci(pos_base)?,
			ptr::null_mut(),
			buf.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_embed_blend(
	rowsrc: &GpuBuffer,
	w: &GpuBuffer,
	out: &GpuBuffer,
	rows: usize,
	k: usize,
	ne: usize,
	scale: f64,
) -> Result<(), HipError> {
	let (s, d) = (rowsrc.dtype(), out.dtype());
	if s.conv() < 0 || d.conv() < 0 || d.block_elems() > 1 {
		Write::error(format!("gpu_embed_blend: no kernel for {s:?} -> {d:?}"));
		return Err(HipError(1));
	}
	let rows64 = i64::try_from(rows).map_err(|_e| return HipError(1))?;
	// SAFETY: rowsrc holds rows*k rows of ne elements at its dtype, w holds rows*k weights, and out holds rows*ne elements at its dtype.
	unsafe {
		launch_embed_blend(
			rowsrc.ptr_raw().cast_const(),
			w.ptr_raw().cast_const(),
			out.ptr_raw(),
			rows64,
			ci(k)?,
			ci(ne)?,
			scale,
			s.conv(),
			d.conv(),
			ptr::null_mut(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_convert(src: &GpuBuffer, dst: &GpuBuffer, n: usize, scale: f64) -> Result<(), HipError> {
	let n64 = i64::try_from(n).map_err(|_e| return HipError(1))?;
	let (s, d) = (src.dtype(), dst.dtype());
	if s.conv() < 0 || d.conv() < 0 {
		Write::error(format!("gpu_convert: no kernel for {s:?} -> {d:?}"));
		return Err(HipError(1));
	}
	if d.block_elems() > 1 && d != Dtype::Q8_0 {
		Write::error(format!("gpu_convert: no encoder for {d:?}"));
		return Err(HipError(1));
	}
	if !n.is_multiple_of(s.block_elems()) || !n.is_multiple_of(d.block_elems()) {
		Write::error(format!(
			"gpu_convert: {n} elements is not whole blocks of {s:?} ({}) -> {d:?} ({})",
			s.block_elems(),
			d.block_elems()
		));
		return Err(HipError(1));
	}
	// SAFETY: src holds at least n elements at its dtype and dst n elements at its dtype; the launcher only reads/writes within them.
	unsafe {
		launch_convert(
			src.ptr_raw().cast_const(),
			dst.ptr_raw(),
			n64,
			scale,
			s.conv(),
			d.conv(),
			ptr::null_mut(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_rmsnorm_f64(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: x, gamma, eps, and out are live GpuBuffer allocations and the launcher only reads/writes within them.
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			gamma.ptr_raw().cast_const(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_rmsnorm_f64_nogamma(
	x: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: launcher reads valid device buffers with matching dims on the default stream.
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw().cast_const(),
			out.ptr_raw(),
			ptr::null(),
			ci(rows)?,
			ci(cols)?,
			eps.ptr_raw().cast_const(),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}


static FLASH_RAN: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static L2_TILED: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

#[must_use]
pub fn flash_ran() -> bool {
	return FLASH_RAN.load(core::sync::atomic::Ordering::Relaxed);
}

#[must_use]
pub fn l2_tiled() -> bool {
	return L2_TILED.load(core::sync::atomic::Ordering::Relaxed);
}

fn note_flash(t_kv: usize, kv_row_bytes: usize, resident_bytes: usize) {
	FLASH_RAN.store(true, core::sync::atomic::Ordering::Relaxed);
	let tile = (4_194_304usize.saturating_sub(resident_bytes) / kv_row_bytes.max(1)).max(1);
	if t_kv > tile {
		L2_TILED.store(true, core::sync::atomic::Ordering::Relaxed);
	}
}

#[inline]
pub fn gpu_flash_gqa(
	q: &GpuBuffer,
	kc: &GpuBuffer,
	vc: &GpuBuffer,
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	max_bias: f64,
	p_base: usize,
	causal_below: usize,
	out: &GpuBuffer,
	m_io: &GpuBuffer,
	l_io: &GpuBuffer,
	acc_io: &GpuBuffer,
	kv_off: usize,
	finalize: bool,
) -> Result<(), HipError> {
	let es = out.dtype().elem_size();
	note_flash(t_kv, 2 * nkv * hd * es, 2 * hd * es);
	// SAFETY: launcher reads valid device buffers with matching dims on the default stream; carry buffers are null or live.
	unsafe {
		launch_flash_gqa(
			q.ptr_raw().cast_const(),
			kc.ptr_raw().cast_const(),
			vc.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(t_q)?,
			ci(t_kv)?,
			ci(nqh)?,
			ci(nkv)?,
			ci(hd)?,
			max_bias,
			ci(p_base)?,
			ci(causal_below)?,
			m_io.ptr_raw(),
			l_io.ptr_raw(),
			acc_io.ptr_raw(),
			ci(kv_off)?,
			i32::from(finalize),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_flash_mla(
	q: &GpuBuffer,
	kc: &GpuBuffer,
	vc: &GpuBuffer,
	t_q: usize,
	t_kv: usize,
	nqh: usize,
	nkv: usize,
	hdk: usize,
	hdv: usize,
	p_base: usize,
	causal_below: usize,
	out: &GpuBuffer,
	m_io: &GpuBuffer,
	l_io: &GpuBuffer,
	acc_io: &GpuBuffer,
	kv_off: usize,
	finalize: bool,
) -> Result<(), HipError> {
	let es = out.dtype().elem_size();
	note_flash(t_kv, nkv * (hdk + hdv) * es, (hdk + hdv) * es);
	// SAFETY: launcher reads valid device buffers with matching dims on the default stream; carry buffers are null or live.
	unsafe {
		launch_flash_mla(
			q.ptr_raw().cast_const(),
			kc.ptr_raw().cast_const(),
			vc.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(t_q)?,
			ci(t_kv)?,
			ci(nqh)?,
			ci(nkv)?,
			ci(hdk)?,
			ci(hdv)?,
			ci(p_base)?,
			ci(causal_below)?,
			m_io.ptr_raw(),
			l_io.ptr_raw(),
			acc_io.ptr_raw(),
			ci(kv_off)?,
			i32::from(finalize),
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_gelu_mul(
	a: &GpuBuffer,
	b: &GpuBuffer,
	n: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: a, b, and out outlive this synchronous launch and n matches their element counts.
	unsafe {
		launch_gelu_mul(
			a.ptr_raw().cast_const(),
			b.ptr_raw().cast_const(),
			out.ptr_raw(),
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_glu_gelu(
	input: &GpuBuffer,
	rows: usize,
	half: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: input and out outlive this synchronous launch and rows and half match their sizes.
	unsafe {
		launch_glu_gelu(
			input.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(rows)?,
			ci(half)?,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_glu_silu(
	input: &GpuBuffer,
	rows: usize,
	half: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: input and out outlive this synchronous launch and rows and half match their sizes.
	unsafe {
		launch_glu_silu(
			input.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(rows)?,
			ci(half)?,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_gemm_bt(
	lhs: &GpuBuffer,
	rhs: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: lhs, rhs, and out outlive this synchronous launch and m, n, k match their shapes.
	unsafe {
		launch_gemm_bt(
			lhs.ptr_raw().cast_const(),
			rhs.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_gemm_bt_tiles(
	lhs: &GpuBuffer,
	rhs: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	tile_m: usize,
	tile_n: usize,
	tile_k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: lhs, rhs, and out outlive this synchronous launch and m, n, k match their shapes.
	unsafe {
		launch_gemm_bt_tiles(
			lhs.ptr_raw().cast_const(),
			rhs.ptr_raw().cast_const(),
			out.ptr_raw(),
			ci(m)?,
			ci(n)?,
			ci(k)?,
			ci(tile_m)?,
			ci(tile_n)?,
			ci(tile_k)?,
			ptr::null_mut(),
			out.dtype().ffi(),
		);
	}
	return cl();
}

#[inline]
pub fn gpu_scale_f64_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	// SAFETY: all pointers reference live GpuBuffers valid for the kernel launch.
	unsafe {
		launch_scale_f64(
			x.ptr_raw(),
			scalar.ptr_raw().cast_const(),
			i64::try_from(n).map_err(|_e| return HipError(1))?,
			ptr::null_mut(),
			x.dtype().ffi(),
		);
	}
	return cl();
}

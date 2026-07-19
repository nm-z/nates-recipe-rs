use crate::HipError;
use crate::callspy;
use crate::hip::{check, hipGetLastError};
use crate::kernels::ci;
use crate::memory::{Dtype, GpuBuffer};
use core::ffi::c_void;
use core::ptr;

/// Checks the last HIP launch for an error, ticking the launch callspy counters.
fn cl() -> Result<(), HipError> {
	callspy::tick(&callspy::LAUNCH);
	callspy::tick(&callspy::GET_LAST_ERROR);
	// SAFETY: hipGetLastError only reads and clears the thread-local HIP error state; it dereferences no memory.
	return check(unsafe { hipGetLastError() });
}

unsafe extern "C" {
	fn launch_widen_bf16_f64(input: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_widen_bf16_f32(input: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_widen_bf16_scaled(
		input: *const c_void,
		out: *mut c_void,
		n: i64,
		scale: f64,
		stream: *mut c_void,
		f32_out: i32,
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
	fn launch_gemm_bt_f64(
		a: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
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

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
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

/// NeoX RoPE applying absolute positions offset by `pos_base` (KV-cache decode:
/// the new rows sit at absolute positions `pos_base + row`). `pos_base == 0` is the
/// prefill/full-forward case, identical to [`gpu_rope_partial`].
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
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

/// LongRoPE NeoX RoPE with per-pair frequency factors (`factors` is `[rotary_dim/2]`,
/// dividing the pair-i angle; minicpm3 `rope_ext` with `freq_factors`).
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
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

/// LongRoPE per-pair-factor NeoX RoPE at absolute positions offset by `pos_base`
/// (KV-cache decode). `pos_base == 0` matches [`gpu_rope_partial_factors`].
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
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

/// Widens `n` bf16 values in `raw` into `out` (f32 or f64 by `out`'s dtype),
/// multiplying each by `scale` in f64 before any narrowing — bit-identical to
/// the host `(f64)bf16 * scale` path it replaces. The GPU-side embedding
/// gather: token rows upload as raw bf16 bytes and widen on device, no
/// per-element host loop.
///
/// # Errors
/// Returns [`HipError`] if `n` overflows `i64` or the kernel launch fails.
#[inline]
pub fn gpu_widen_bf16_scaled(
	raw: &GpuBuffer,
	n: usize,
	scale: f64,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	let n64 = i64::try_from(n).map_err(|_e| return HipError(1))?;
	let f32_out = i32::from(out.dtype() == Dtype::F32);
	// SAFETY: raw holds at least n bf16 values and out n elements at its dtype; the launcher only reads/writes within them.
	unsafe {
		launch_widen_bf16_scaled(
			raw.ptr_raw().cast_const(),
			out.ptr_raw(),
			n64,
			scale,
			ptr::null_mut(),
			f32_out,
		);
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if `n` overflows `i64` or the kernel launch fails.
#[inline]
pub fn gpu_widen_bf16(raw: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	let n64 = i64::try_from(n).map_err(|_e| return HipError(1))?;
	// SAFETY: raw and out are live GpuBuffer allocations sized for n elements at out's dtype; the launcher only reads/writes within them.
	unsafe {
		if out.dtype() == Dtype::F32 {
			launch_widen_bf16_f32(raw.ptr_raw().cast_const(), out.ptr_raw(), n64, ptr::null_mut());
		} else {
			launch_widen_bf16_f64(raw.ptr_raw().cast_const(), out.ptr_raw(), n64, ptr::null_mut());
		}
	}
	return cl();
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the kernel launch fails.
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

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the launch fails.
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


/// Batched flash attention (GQA/MQA) — the sole attention path. `q` is `[t_q,nqh,hd]`
/// (the NEW query rows, pre-scaled by `1/sqrt(hd)`), `kc`/`vc` are `[t_kv,nkv,hd]`
/// (the K/V cache, or a per-forward K/V buffer), `out` is `[t_q,nqh,hd]`. Online
/// softmax, never materializes the score matrix; `t_q == 1` and `t_q > 1` run the
/// identical algorithm.
///
/// `causal_below` is a POSITION BOUND, never a flag: key `sp` is masked out for the
/// query at absolute position `p = p_base+i` exactly when `p < causal_below && sp > p`.
/// Pass `t_kv` for a fully causal pass (every query masks its future), `0` for a
/// fully bidirectional pass (nothing masked), or the prompt length for the diffusion
/// canvas (causal prompt, bidirectional canvas). Passing `1` here would make only
/// position 0 causal — never pass a bool.
///
/// Set the first time a flash kernel actually launched / a launch's host-side
/// tile solve actually segmented its K/V sweep (`t_kv > tile`). Read by the
/// chat capability line — proven-ran flags, never build-time claims.
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

/// Host mirror of the kernel's L2 tile solve: rows per tile such that the K+V
/// working set at one sequence offset fits the 4 MiB L2 (same inequality the
/// kernel doc states), clamped to at least 1.
fn note_flash(t_kv: usize, kv_row_bytes: usize, resident_bytes: usize) {
	FLASH_RAN.store(true, core::sync::atomic::Ordering::Relaxed);
	let tile = (4_194_304usize.saturating_sub(resident_bytes) / kv_row_bytes.max(1)).max(1);
	if t_kv > tile {
		L2_TILED.store(true, core::sync::atomic::Ordering::Relaxed);
	}
}

/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the launch fails.
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

/// Batched flash MLA — distinct key/value head dims (`hdk` dot, `hdv` gather), MQA.
/// `q` pre-scaled. See [`gpu_flash_gqa`]; `kc` is `[t_kv,nkv,hdk]`, `vc` is
/// `[t_kv,nkv,hdv]`, `out` is `[t_q,nqh,hdv]`.
///
/// # Errors
/// Returns [`HipError`] if a dimension overflows `i32` or the launch fails.
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

/// # Errors
/// Returns [`HipError`] if `n` overflows `i64` or the kernel launch fails.
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

/// # Errors
/// Returns [`HipError`] if `rows` or `half` overflows `i32` or the launch fails.
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

/// # Errors
/// Returns [`HipError`] if `rows` or `half` overflows `i32` or the launch fails.
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

/// # Errors
/// Returns [`HipError`] if `m`, `n`, or `k` overflows `i32` or the launch fails.
#[inline]
pub fn gpu_gemm_bt_f64(
	lhs: &GpuBuffer,
	rhs: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	// SAFETY: lhs, rhs, and out outlive this synchronous launch and m, n, k match their shapes.
	unsafe {
		launch_gemm_bt_f64(
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

/// # Errors
/// Returns [`HipError`] if the kernel launch fails.
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

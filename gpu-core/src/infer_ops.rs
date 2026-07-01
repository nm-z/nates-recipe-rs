//! f64 streaming-inference ops: widen bf16 weights to f64 in VRAM, fused f64
//! RMSNorm (optional gamma), GQA attention with a mixed causal/bidirectional
//! prefix mask, and gated-GELU fusions. General and model-agnostic — a bf16
//! transformer's forward composes from these plus the hipBLAS GEMMs.

use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	fn launch_widen_bf16_f64(input: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_normx_rmsnorm(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		rows: i32,
		cols: i32,
		eps: f64,
		stream: *mut c_void,
	);
	fn launch_gqa_masked_attn(
		q: *const c_void,
		k: *const c_void,
		v: *const c_void,
		out: *mut c_void,
		t: i32,
		nqh: i32,
		nkv: i32,
		hd: i32,
		prefix: i32,
		stream: *mut c_void,
	);
	fn launch_gelu_mul(a: *const c_void, b: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_glu_gelu(input: *const c_void, out: *mut c_void, rows: i32, half: i32, stream: *mut c_void);
	fn launch_rope_partial(
		buf: *mut c_void,
		rows: i32,
		head_dim: i32,
		rotary_dim: i32,
		heads_per_tok: i32,
		theta: f64,
		stream: *mut c_void,
	);
}

/// NeoX partial rotary embedding, in-place on `buf` `(rows, head_dim)`. The
/// first `rotary_dim` dims of each head rotate (rotate-half); the rest pass
/// through. Row `r`'s position is `r / heads_per_tok`. `rotary_dim == head_dim`
/// gives full rotary.
pub fn gpu_rope_partial(
	buf: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	theta: f64,
) {
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			rows as i32,
			head_dim as i32,
			rotary_dim as i32,
			heads_per_tok as i32,
			theta,
			std::ptr::null_mut(),
		);
	}
	check_launch();
}

/// Widen `n` contiguous bf16 halves (raw little-endian u16 bytes in `raw`,
/// length `2*n` bytes) into a fresh f64 buffer of `n` elements. Exact pad.
pub fn gpu_widen_bf16(raw: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_widen_bf16_f64(raw.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	check_launch();
	Ok(out)
}

/// Widen `n` bf16 halves from `raw` into an existing f64 buffer `out` (reuse
/// path — no allocation). `out` must hold at least `n` f64.
pub fn gpu_widen_bf16_into(raw: &GpuBuffer, out: &GpuBuffer, n: usize) {
	unsafe {
		launch_widen_bf16_f64(raw.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	check_launch();
}

/// Fused RMSNorm: `out[r,j] = x[r,j] / sqrt(mean_j(x^2) + eps) * gamma[j]`, or
/// without the gamma factor when `gamma` is `None` (the scale-less variant).
pub fn gpu_rmsnorm_f64(
	x: &GpuBuffer,
	gamma: Option<&GpuBuffer>,
	rows: usize,
	cols: usize,
	eps: f64,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(rows * cols)?;
	let gptr = gamma.map(|g| g.ptr_raw() as *const c_void).unwrap_or(std::ptr::null());
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			gptr,
			rows as i32,
			cols as i32,
			eps,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

/// GQA attention. `q` is `(t, nqh*hd)`, `k`/`v` are `(t, nkv*hd)`, all f64
/// row-major. kq_scale = 1.0. Prompt rows (`p < prefix`) are causal; canvas rows
/// are bidirectional. Returns attention output `(t, nqh*hd)`.
pub fn gpu_gqa_attn(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	t: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	prefix: usize,
) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(t * nqh * hd)?;
	unsafe {
		launch_gqa_masked_attn(
			q.ptr_raw() as *const c_void,
			k.ptr_raw() as *const c_void,
			v.ptr_raw() as *const c_void,
			out.ptr_raw(),
			t as i32,
			nqh as i32,
			nkv as i32,
			hd as i32,
			prefix as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(out)
}

/// Elementwise `out = gelu(a) * b` (tanh-approx GELU), `n` elements.
pub fn gpu_gelu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	unsafe {
		launch_gelu_mul(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	check_launch();
	Ok(out)
}

/// Fused gate|up split: `in` is `(rows, 2*half)` = `[gate | up]` per row;
/// returns `(rows, half)` with `gelu(gate) * up`.
pub fn gpu_glu_gelu(input: &GpuBuffer, rows: usize, half: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(rows * half)?;
	unsafe {
		launch_glu_gelu(input.ptr_raw() as *const c_void, out.ptr_raw(), rows as i32, half as i32, std::ptr::null_mut());
	}
	check_launch();
	Ok(out)
}

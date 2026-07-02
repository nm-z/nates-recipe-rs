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
	fn launch_gemm_bt_f64(
		a: *const c_void,
		b: *const c_void,
		c: *mut c_void,
		m: i32,
		n: i32,
		k: i32,
		stream: *mut c_void,
	);
	fn launch_scale_f64(x: *mut c_void, scalar: f64, n: i64, stream: *mut c_void);
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
	gpu_rmsnorm_f64_into(x, gamma, &out, rows, cols, eps);
	Ok(out)
}

/// RMSNorm into a caller-owned `out` (no alloc). Aliasing `out == x` is safe:
/// every thread reads all its `x` columns into the sum-of-squares before the
/// block barrier, and only then writes `out`, so an in-place norm is well-defined.
pub fn gpu_rmsnorm_f64_into(
	x: &GpuBuffer,
	gamma: Option<&GpuBuffer>,
	out: &GpuBuffer,
	rows: usize,
	cols: usize,
	eps: f64,
) {
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
	gpu_gqa_attn_into(q, k, v, &out, t, nqh, nkv, hd, prefix);
	Ok(out)
}

/// GQA attention into a caller-owned `out` `(t, nqh*hd)` (no alloc). `out` must
/// be distinct from `q`/`k`/`v` — each block reads the whole q/k/v sequence.
pub fn gpu_gqa_attn_into(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	out: &GpuBuffer,
	t: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	prefix: usize,
) {
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
}

/// Elementwise `out = gelu(a) * b` (tanh-approx GELU), `n` elements.
pub fn gpu_gelu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(n)?;
	gpu_gelu_mul_into(a, b, &out, n);
	Ok(out)
}

/// `out = gelu(a) * b` into a caller-owned buffer (no alloc). Aliasing `out == a`
/// or `out == b` is safe — thread `i` reads `a[i]`/`b[i]` before writing `out[i]`.
pub fn gpu_gelu_mul_into(a: &GpuBuffer, b: &GpuBuffer, out: &GpuBuffer, n: usize) {
	unsafe {
		launch_gelu_mul(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	check_launch();
}

/// Fused gate|up split: `in` is `(rows, 2*half)` = `[gate | up]` per row;
/// returns `(rows, half)` with `gelu(gate) * up`.
pub fn gpu_glu_gelu(input: &GpuBuffer, rows: usize, half: usize) -> Result<GpuBuffer, HipError> {
	let out = GpuBuffer::alloc(rows * half)?;
	gpu_glu_gelu_into(input, &out, rows, half);
	Ok(out)
}

/// Gated-GELU split into a caller-owned `out` `(rows, half)` (no alloc). `out`
/// must be distinct from `input` (different shape: input is `(rows, 2*half)`).
pub fn gpu_glu_gelu_into(input: &GpuBuffer, out: &GpuBuffer, rows: usize, half: usize) {
	unsafe {
		launch_glu_gelu(input.ptr_raw() as *const c_void, out.ptr_raw(), rows as i32, half as i32, std::ptr::null_mut());
	}
	check_launch();
}

/// Custom f64 GEMM-BT (no hipBLAS): `out(m,n) = a(m,k) . b(n,k)^T`, all
/// row-major, into a caller-owned `out` (no alloc). `out` must be distinct
/// from `a`/`b`. Replaces `gpu_gemm_bt_into` in the inference forward.
pub fn gpu_gemm_bt_f64_into(a: &GpuBuffer, b: &GpuBuffer, out: &GpuBuffer, m: usize, n: usize, k: usize) {
	unsafe {
		launch_gemm_bt_f64(
			a.ptr_raw() as *const c_void,
			b.ptr_raw() as *const c_void,
			out.ptr_raw(),
			m as i32,
			n as i32,
			k as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
}

/// In-place scale `x *= scalar` (no alloc, no copy). Replaces `gpu_scale_inplace`.
pub fn gpu_scale_f64_inplace(x: &GpuBuffer, scalar: f64, n: usize) {
	unsafe {
		launch_scale_f64(x.ptr_raw(), scalar, n as i64, std::ptr::null_mut());
	}
	check_launch();
}

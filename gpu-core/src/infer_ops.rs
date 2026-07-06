//! f64 streaming-inference ops: widen bf16 weights to f64 in VRAM, fused f64
//! RMSNorm (gamma / no-gamma variants), GQA attention with a mixed causal/
//! bidirectional prefix mask, and gated-GELU fusions. General and model-agnostic
//! — a bf16 transformer's forward composes from these plus the GEMMs.

use crate::hip::{HipError, check};
use crate::memory::GpuBuffer;
use std::ffi::c_void;

fn cl() -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::LAUNCH);
	crate::callspy::tick(&crate::callspy::GET_LAST_ERROR);
	check(unsafe { crate::hip::hipGetLastError() })
}

unsafe extern "C" {
	fn launch_widen_bf16_f64(input: *const c_void, out: *mut c_void, n: i64, stream: *mut c_void);
	fn launch_normx_rmsnorm(
		x: *const c_void,
		out: *mut c_void,
		gamma: *const c_void,
		rows: i32,
		cols: i32,
		eps: *const c_void,
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
		theta: *const c_void,
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
	fn launch_scale_f64(x: *mut c_void, scalar: *const c_void, n: i64, stream: *mut c_void);
}

/// NeoX partial rotary embedding, in-place on `buf` `(rows, head_dim)`. The
/// first `rotary_dim` dims of each head rotate (rotate-half); the rest pass
/// through. Row `r`'s position is `r / heads_per_tok`. `rotary_dim == head_dim`
/// gives full rotary. `theta` is a caller-supplied 1-elem device buffer.
// in-place: writes x (rotary embedding over leading dims)
pub fn gpu_rope_partial(
	theta: &GpuBuffer,
	rows: usize,
	head_dim: usize,
	rotary_dim: usize,
	heads_per_tok: usize,
	buf: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_rope_partial(
			buf.ptr_raw(),
			rows as i32,
			head_dim as i32,
			rotary_dim as i32,
			heads_per_tok as i32,
			theta.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

/// Widen `n` bf16 halves (raw little-endian u16 bytes in `raw`, length `2*n`
/// bytes) into a caller-owned f64 buffer `out` of `n` elements. Exact pad.
pub fn gpu_widen_bf16(raw: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_widen_bf16_f64(raw.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	cl()
}

/// Fused RMSNorm with per-column `gamma`: `out[r,j] = x[r,j] / sqrt(mean_j(x^2)
/// + eps) * gamma[j]`. `eps` is a caller-supplied 1-elem device buffer. Aliasing
/// `out == x` is safe: every thread reads all its `x` columns into the
/// sum-of-squares before the block barrier, and only then writes `out`.
pub fn gpu_rmsnorm_f64(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			gamma.ptr_raw() as *const c_void,
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

/// Scale-less RMSNorm (no gamma factor): `out[r,j] = x[r,j] / sqrt(mean_j(x^2)
/// + eps)`. `eps` is a caller-supplied 1-elem device buffer. Aliasing `out == x`
/// is safe (same read-before-write ordering as `gpu_rmsnorm_f64`).
pub fn gpu_rmsnorm_f64_nogamma(
	x: &GpuBuffer,
	eps: &GpuBuffer,
	rows: usize,
	cols: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_normx_rmsnorm(
			x.ptr_raw() as *const c_void,
			out.ptr_raw(),
			std::ptr::null(),
			rows as i32,
			cols as i32,
			eps.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	cl()
}

/// GQA attention into a caller-owned `out` `(t, nqh*hd)`. `q` is `(t, nqh*hd)`,
/// `k`/`v` are `(t, nkv*hd)`, all f64 row-major. kq_scale = 1.0. Prompt rows
/// (`p < prefix`) are causal; canvas rows are bidirectional. `out` must be
/// distinct from `q`/`k`/`v` — each block reads the whole q/k/v sequence.
pub fn gpu_gqa_attn(
	q: &GpuBuffer,
	k: &GpuBuffer,
	v: &GpuBuffer,
	t: usize,
	nqh: usize,
	nkv: usize,
	hd: usize,
	prefix: usize,
	out: &GpuBuffer,
) -> Result<(), HipError> {
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
	cl()
}

/// Elementwise `out = gelu(a) * b` (tanh-approx GELU), `n` elements, into a
/// caller-owned buffer. Aliasing `out == a` or `out == b` is safe — thread `i`
/// reads `a[i]`/`b[i]` before writing `out[i]`.
pub fn gpu_gelu_mul(a: &GpuBuffer, b: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_gelu_mul(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), n as i64, std::ptr::null_mut());
	}
	cl()
}

/// Fused gate|up split into a caller-owned `out` `(rows, half)`: `input` is
/// `(rows, 2*half)` = `[gate | up]` per row; `out = gelu(gate) * up`. `out` must
/// be distinct from `input` (different shape).
pub fn gpu_glu_gelu(input: &GpuBuffer, rows: usize, half: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_glu_gelu(input.ptr_raw() as *const c_void, out.ptr_raw(), rows as i32, half as i32, std::ptr::null_mut());
	}
	cl()
}

/// Custom f64 GEMM-BT (no hipBLAS): `out(m,n) = a(m,k) . b(n,k)^T`, all
/// row-major, into a caller-owned `out` (no alloc). `out` must be distinct
/// from `a`/`b`.
pub fn gpu_gemm_bt_f64(a: &GpuBuffer, b: &GpuBuffer, m: usize, n: usize, k: usize, out: &GpuBuffer) -> Result<(), HipError> {
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
	cl()
}

/// In-place scale `x *= scalar` (no alloc, no copy). `scalar` is a caller-
/// supplied 1-elem device buffer.
// in-place: writes x (x *= scalar)
pub fn gpu_scale_f64_inplace(scalar: &GpuBuffer, n: usize, x: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_scale_f64(x.ptr_raw(), scalar.ptr_raw() as *const c_void, n as i64, std::ptr::null_mut());
	}
	cl()
}

use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::{HipError, check};

// ── rocBLAS f32 ────────────────────────────────────────────────────────────
// Same column-major conventions as kernels.rs dgemm section.
const ROCBLAS_OPERATION_NONE: u32 = 111;

unsafe extern "C" {
      fn rocblas_sgemm(
            handle: *mut c_void,
            transA: u32, transB: u32,
            m: i32, n: i32, k: i32,
            alpha: *const f32,
            A: *const f32, lda: i32,
            B: *const f32, ldb: i32,
            beta: *const f32,
            C: *mut f32, ldc: i32,
      ) -> i32;

      fn rocblas_saxpy(
            handle: *mut c_void,
            n: i32,
            alpha: *const f32,
            x: *const f32, incx: i32,
            y: *mut f32, incy: i32,
      ) -> i32;

      // f32 kernels
      fn launch_relu_f32(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_relu_backward_f32(grad: *const c_void, act: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_gelu_f32(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_gelu_backward_f32(grad: *const c_void, x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_bias_add_f32(x: *const c_void, bias: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
      fn launch_repeat_rows_f32(src: *const c_void, dst: *mut c_void, cols: i32, total: i32, stream: *mut c_void);
      fn launch_layernorm_f32(x: *const c_void, out: *mut c_void, gamma: *const c_void, beta: *const c_void, rows: i32, cols: i32, eps: f32, stream: *mut c_void);
      fn launch_layernorm_backward_f32(grad_y: *const c_void, x: *const c_void, gamma: *const c_void, grad_x: *mut c_void, grad_gamma: *mut c_void, grad_beta: *mut c_void, rows: i32, cols: i32, eps: f32, stream: *mut c_void);
      fn launch_avg_pool_2d_f32(input: *const c_void, output: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
      fn launch_avg_pool_2d_backward_f32(grad_out: *const c_void, grad_in: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
      fn launch_max_pool_2d_f32(input: *const c_void, out_vals: *mut c_void, out_idx: *mut c_void, n: i32, c: i32, h: i32, w: i32, kh: i32, kw: i32, sh: i32, sw: i32, out_h: i32, out_w: i32, stream: *mut c_void);
      fn launch_max_pool_2d_backward_f32(grad_out: *const c_void, indices: *const c_void, grad_in: *mut c_void, n: i32, c: i32, out_h: i32, out_w: i32, h: i32, w: i32, stream: *mut c_void);
      fn launch_lstm_cell_f32(gates: *const c_void, c: *mut c_void, h: *mut c_void, n: i32, hs: i32, stream: *mut c_void);
      fn launch_gru_cell_f32(gates: *const c_void, h: *const c_void, h_new: *mut c_void, n: i32, hs: i32, stream: *mut c_void);

      // f16 kernels — buffers are alloc_bytes(n*2), cast as raw u16 bit patterns
      fn launch_relu_f16(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_gelu_f16(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_add_f16(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_mul_f16(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
}

fn check_launch() {
      let err = unsafe { crate::hip::hipGetLastError() };
      assert!(err == 0, "HIP kernel launch failed with error code {}", err);
}

fn safe_i32(v: usize) -> i32 {
      assert!(v <= i32::MAX as usize, "size {} overflows i32", v);
      v as i32
}

// ── gpu_linear_f32 ─────────────────────────────────────────────────────────
// out = X @ W + bias. X is (m,k) f32, W is (k,n) f32, bias is (n,) f32.
// Prefills out with bias broadcast, then sgemm with beta=1.0 adds the matmul.
pub fn gpu_linear_f32(x: &GpuBuffer, w: &GpuBuffer, bias: &GpuBuffer, m: usize, n: usize, k: usize) -> Result<GpuBuffer, HipError> {
      let c = GpuBuffer::zeros_f32(m * n)?;
      unsafe {
            launch_repeat_rows_f32(
                  bias.ptr_raw() as *const c_void,
                  c.ptr_raw(),
                  safe_i32(n),
                  safe_i32(m * n),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      let alpha = 1.0_f32;
      let beta = 1.0_f32;
      let status = unsafe {
            rocblas_sgemm(
                  crate::kernels::rocblas_handle(),
                  ROCBLAS_OPERATION_NONE, ROCBLAS_OPERATION_NONE,
                  safe_i32(n), safe_i32(m), safe_i32(k),
                  &alpha,
                  w.ptr_raw() as *const f32, safe_i32(n),
                  x.ptr_raw() as *const f32, safe_i32(k),
                  &beta,
                  c.ptr_raw() as *mut f32, safe_i32(n),
            )
      };
      check(status)?;
      Ok(c)
}

// ── gpu_relu_f32 / backward ────────────────────────────────────────────────
pub fn gpu_relu_f32(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n)?;
      unsafe { launch_relu_f32(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_relu_backward_f32(grad: &GpuBuffer, act: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n)?;
      unsafe { launch_relu_backward_f32(grad.ptr_raw() as *const c_void, act.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

// ── gpu_gelu_f32 / backward ────────────────────────────────────────────────
// backward takes pre-activation x (not the output), matching the f64 convention
pub fn gpu_gelu_f32(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n)?;
      unsafe { launch_gelu_f32(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_gelu_backward_f32(grad: &GpuBuffer, x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n)?;
      unsafe { launch_gelu_backward_f32(grad.ptr_raw() as *const c_void, x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

// ── gpu_layernorm_f32 / backward ───────────────────────────────────────────
pub fn gpu_layernorm_f32(x: &GpuBuffer, gamma: &GpuBuffer, beta: &GpuBuffer, rows: usize, cols: usize, eps: f32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(rows * cols)?;
      unsafe {
            launch_layernorm_f32(
                  x.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  gamma.ptr_raw() as *const c_void,
                  beta.ptr_raw() as *const c_void,
                  safe_i32(rows), safe_i32(cols),
                  eps,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_layernorm_backward_f32(
      grad_y: &GpuBuffer, x: &GpuBuffer, gamma: &GpuBuffer,
      rows: usize, cols: usize, eps: f32,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let grad_x = GpuBuffer::zeros_f32(rows * cols)?;
      let grad_gamma = GpuBuffer::zeros_f32(cols)?;
      let grad_beta = GpuBuffer::zeros_f32(cols)?;
      unsafe {
            launch_layernorm_backward_f32(
                  grad_y.ptr_raw() as *const c_void,
                  x.ptr_raw() as *const c_void,
                  gamma.ptr_raw() as *const c_void,
                  grad_x.ptr_raw(),
                  grad_gamma.ptr_raw(),
                  grad_beta.ptr_raw(),
                  safe_i32(rows), safe_i32(cols),
                  eps,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok((grad_x, grad_gamma, grad_beta))
}

// ── gpu_bias_add_f32 ───────────────────────────────────────────────────────
pub fn gpu_bias_add_f32(x: &GpuBuffer, bias: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(rows * cols)?;
      unsafe {
            launch_bias_add_f32(
                  x.ptr_raw() as *const c_void,
                  bias.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(rows), safe_i32(cols),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_avg_pool_2d_f32 / backward ────────────────────────────────────────
// Input is NCHW layout. outH = (H-kH)/sH+1, outW = (W-kW)/sW+1.
pub fn gpu_avg_pool_2d_f32(
      input: &GpuBuffer,
      n_batch: usize, c: usize, h: usize, w: usize,
      kh: usize, kw: usize, sh: usize, sw: usize,
) -> Result<GpuBuffer, HipError> {
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let out = GpuBuffer::zeros_f32(n_batch * c * out_h * out_w)?;
      unsafe {
            launch_avg_pool_2d_f32(
                  input.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(n_batch), safe_i32(c), safe_i32(h), safe_i32(w),
                  safe_i32(kh), safe_i32(kw), safe_i32(sh), safe_i32(sw),
                  safe_i32(out_h), safe_i32(out_w),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_avg_pool_2d_backward_f32(
      grad_out: &GpuBuffer,
      n_batch: usize, c: usize, h: usize, w: usize,
      kh: usize, kw: usize, sh: usize, sw: usize,
      out_h: usize, out_w: usize,
) -> Result<GpuBuffer, HipError> {
      let grad_in = GpuBuffer::zeros_f32(n_batch * c * h * w)?;
      unsafe {
            launch_avg_pool_2d_backward_f32(
                  grad_out.ptr_raw() as *const c_void,
                  grad_in.ptr_raw(),
                  safe_i32(n_batch), safe_i32(c), safe_i32(h), safe_i32(w),
                  safe_i32(kh), safe_i32(kw), safe_i32(sh), safe_i32(sw),
                  safe_i32(out_h), safe_i32(out_w),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(grad_in)
}

// ── gpu_max_pool_2d_f32 / backward ────────────────────────────────────────
// Returns (pooled_values, argmax_indices) both as f32 GpuBuffers.
// out_idx stores the flat intra-channel index (ih*W+iw) as f32.
pub fn gpu_max_pool_2d_f32(
      input: &GpuBuffer,
      n_batch: usize, c: usize, h: usize, w: usize,
      kh: usize, kw: usize, sh: usize, sw: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let out_vals = GpuBuffer::zeros_f32(n_batch * c * out_h * out_w)?;
      let out_idx = GpuBuffer::zeros_f32(n_batch * c * out_h * out_w)?;
      unsafe {
            launch_max_pool_2d_f32(
                  input.ptr_raw() as *const c_void,
                  out_vals.ptr_raw(),
                  out_idx.ptr_raw(),
                  safe_i32(n_batch), safe_i32(c), safe_i32(h), safe_i32(w),
                  safe_i32(kh), safe_i32(kw), safe_i32(sh), safe_i32(sw),
                  safe_i32(out_h), safe_i32(out_w),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok((out_vals, out_idx))
}

pub fn gpu_max_pool_2d_backward_f32(
      grad_out: &GpuBuffer, indices: &GpuBuffer,
      n_batch: usize, c: usize, h: usize, w: usize,
      out_h: usize, out_w: usize,
) -> Result<GpuBuffer, HipError> {
      let grad_in = GpuBuffer::zeros_f32(n_batch * c * h * w)?;
      unsafe {
            launch_max_pool_2d_backward_f32(
                  grad_out.ptr_raw() as *const c_void,
                  indices.ptr_raw() as *const c_void,
                  grad_in.ptr_raw(),
                  safe_i32(n_batch), safe_i32(c),
                  safe_i32(out_h), safe_i32(out_w),
                  safe_i32(h), safe_i32(w),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(grad_in)
}

// ── gpu_lstm_cell_f32 ─────────────────────────────────────────────────────
// gates: (n, 4*hs) f32, layout [forget|input|cell_cand|output] per sample.
// c and h are updated in-place (n, hs).
pub fn gpu_lstm_cell_f32(gates: &GpuBuffer, c: &GpuBuffer, h: &GpuBuffer, n: usize, hs: usize) {
      unsafe {
            launch_lstm_cell_f32(
                  gates.ptr_raw() as *const c_void,
                  c.ptr_raw(),
                  h.ptr_raw(),
                  safe_i32(n), safe_i32(hs),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
}

// ── gpu_gru_cell_f32 ──────────────────────────────────────────────────────
// gates: (n, 4*hs) f32, layout [z_pre|r_pre|n_x|n_h] per sample.
// h: previous hidden (n, hs). Returns new hidden (n, hs).
pub fn gpu_gru_cell_f32(gates: &GpuBuffer, h: &GpuBuffer, n: usize, hs: usize) -> Result<GpuBuffer, HipError> {
      let h_new = GpuBuffer::zeros_f32(n * hs)?;
      unsafe {
            launch_gru_cell_f32(
                  gates.ptr_raw() as *const c_void,
                  h.ptr_raw() as *const c_void,
                  h_new.ptr_raw(),
                  safe_i32(n), safe_i32(hs),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(h_new)
}

// ── f16 kernels ───────────────────────────────────────────────────────────
// Buffers hold raw __half bit patterns. Allocate with alloc_bytes(n * 2).
// Upload via upload_u8 (reinterpret &[u16] as &[u8]) or via half::f16 helpers.

pub fn gpu_relu_f16(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * 2)?;
      unsafe { launch_relu_f16(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_gelu_f16(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * 2)?;
      unsafe { launch_gelu_f16(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_add_f16(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * 2)?;
      unsafe { launch_add_f16(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_mul_f16(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * 2)?;
      unsafe { launch_mul_f16(a.ptr_raw() as *const c_void, b.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

// ── gpu_sgd_update_f32 ────────────────────────────────────────────────────
// In-place: weights -= lr * grad, via rocblas_saxpy with negated lr.
pub fn gpu_sgd_update_f32(weights: &GpuBuffer, grad: &GpuBuffer, lr: f32, n: usize) {
      let neg_lr = -lr;
      let status = unsafe {
            rocblas_saxpy(
                  crate::kernels::rocblas_handle(),
                  safe_i32(n),
                  &neg_lr,
                  grad.ptr_raw() as *const f32, 1,
                  weights.ptr_raw() as *mut f32, 1,
            )
      };
      assert_eq!(status, 0, "rocblas_saxpy failed with status {}", status);
}

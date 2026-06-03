use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

// FFI declarations — slot-for-slot with launchers in attention.hip
//
// C: launch_scaled_dot_product_attention(q, k, v, out, n_rows, seq, dim, causal, stream)
//    const float*, const float*, const float*, float*, int, int, int, int, hipStream_t
// C: launch_causal_softmax_rows(x, rows, cols, stream)
//    float*, int, int, hipStream_t
// C: launch_mha_split(x, out, seq, n_heads, head_dim, stream)
//    const float*, float*, int, int, int, hipStream_t
// C: launch_mha_merge(x, out, seq, n_heads, head_dim, stream)
//    const float*, float*, int, int, int, hipStream_t
// C: launch_rope(x, out, seq, dim, base, stream)
//    const float*, float*, int, int, float, hipStream_t
// C: launch_positional_encoding(out, seq, dim, stream)
//    float*, int, int, hipStream_t
// C: launch_rmsnorm(x, gamma, out, rows, cols, eps, stream)
//    const float*, const float*, float*, int, int, float, hipStream_t
// C: launch_rmsnorm_backward(grad_out, x, gamma, grad_x, grad_gamma, rows, cols, eps, stream)
//    const float*, const float*, const float*, float*, float*, int, int, float, hipStream_t
// C: launch_im2col_2d_ext(x, patches, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w, out_h, out_w, stream)
//    const float*, float*, int, int, int, int, int, int, int, int, int, int, int, int, int, int, hipStream_t
// C: launch_col2im_2d_ext(patches, x, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w, out_h, out_w, stream)
//    const float*, float*, int, int, int, int, int, int, int, int, int, int, int, int, int, int, hipStream_t
// C: launch_embedding_backward(grad_out, indices, grad_table, n, cols, vocab, stream)
//    const float*, const int*, float*, int, int, int, hipStream_t
// C: launch_bn_update_running(run_mean, run_var, save_mean, save_var, momentum, c, stream)
//    float*, float*, const float*, const float*, float, int, hipStream_t

unsafe extern "C" {
      fn launch_scaled_dot_product_attention(
            q: *const c_void, k: *const c_void, v: *const c_void, out: *mut c_void,
            n_rows: i32, seq: i32, dim: i32, causal: i32,
            stream: *mut c_void,
      );
      fn launch_causal_softmax_rows(x: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
      fn launch_mha_split(
            x: *const c_void, out: *mut c_void,
            seq: i32, n_heads: i32, head_dim: i32,
            stream: *mut c_void,
      );
      fn launch_mha_merge(
            x: *const c_void, out: *mut c_void,
            seq: i32, n_heads: i32, head_dim: i32,
            stream: *mut c_void,
      );
      fn launch_rope(
            x: *const c_void, out: *mut c_void,
            seq: i32, dim: i32, base: f32,
            stream: *mut c_void,
      );
      fn launch_positional_encoding(out: *mut c_void, seq: i32, dim: i32, stream: *mut c_void);
      fn launch_rmsnorm(
            x: *const c_void, gamma: *const c_void, out: *mut c_void,
            rows: i32, cols: i32, eps: f32,
            stream: *mut c_void,
      );
      fn launch_rmsnorm_backward(
            grad_out: *const c_void, x: *const c_void, gamma: *const c_void,
            grad_x: *mut c_void, grad_gamma: *mut c_void,
            rows: i32, cols: i32, eps: f32,
            stream: *mut c_void,
      );
      fn launch_im2col_2d_ext(
            x: *const c_void, patches: *mut c_void,
            n: i32, c: i32, h: i32, w: i32,
            kh: i32, kw: i32,
            sh: i32, sw: i32,
            pad_h: i32, pad_w: i32,
            dil_h: i32, dil_w: i32,
            out_h: i32, out_w: i32,
            stream: *mut c_void,
      );
      fn launch_col2im_2d_ext(
            patches: *const c_void, x: *mut c_void,
            n: i32, c: i32, h: i32, w: i32,
            kh: i32, kw: i32,
            sh: i32, sw: i32,
            pad_h: i32, pad_w: i32,
            dil_h: i32, dil_w: i32,
            out_h: i32, out_w: i32,
            stream: *mut c_void,
      );
      fn launch_embedding_backward(
            grad_out: *const c_void, indices: *const c_void, grad_table: *mut c_void,
            n: i32, cols: i32, vocab: i32,
            stream: *mut c_void,
      );
      fn launch_bn_update_running(
            run_mean: *mut c_void, run_var: *mut c_void,
            save_mean: *const c_void, save_var: *const c_void,
            momentum: f32, c: i32,
            stream: *mut c_void,
      );
}

// ── gpu_scaled_dot_product_attention ─────────────────────────────────────
// Q, K, V: f32 buffers shaped (n_rows, seq, dim). out: same shape.
// causal=true: query i only attends to keys j <= i.
pub fn gpu_scaled_dot_product_attention(
      q: &GpuBuffer, k: &GpuBuffer, v: &GpuBuffer,
      n_rows: usize, seq: usize, dim: usize,
      causal: bool,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n_rows * seq * dim)?;
      unsafe {
            launch_scaled_dot_product_attention(
                  q.ptr_raw() as *const c_void,
                  k.ptr_raw() as *const c_void,
                  v.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n_rows as i32, seq as i32, dim as i32,
                  causal as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_causal_softmax_rows ───────────────────────────────────────────────
// In-place: upper triangle (j > i) masked to 0 before normalization.
// x: f32 buffer shaped (rows, cols), modified in place.
pub fn gpu_causal_softmax_rows(x: &GpuBuffer, rows: usize, cols: usize) {
      unsafe {
            launch_causal_softmax_rows(
                  x.ptr_raw(),
                  rows as i32, cols as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
}

// ── gpu_mha_split ─────────────────────────────────────────────────────────
// x: (seq, n_heads*head_dim) → out: (n_heads, seq, head_dim). f32.
pub fn gpu_mha_split(
      x: &GpuBuffer, seq: usize, n_heads: usize, head_dim: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(n_heads * seq * head_dim)?;
      unsafe {
            launch_mha_split(
                  x.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  seq as i32, n_heads as i32, head_dim as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_mha_merge ─────────────────────────────────────────────────────────
// x: (n_heads, seq, head_dim) → out: (seq, n_heads*head_dim). f32.
pub fn gpu_mha_merge(
      x: &GpuBuffer, seq: usize, n_heads: usize, head_dim: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(seq * n_heads * head_dim)?;
      unsafe {
            launch_mha_merge(
                  x.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  seq as i32, n_heads as i32, head_dim as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_rope ─────────────────────────────────────────────────────────────
// x: f32 (seq, dim). Rotary positional embedding with given base frequency.
// base: typically 10000.0. Uses sinf/cosf device intrinsics — no external dep.
pub fn gpu_rope(
      x: &GpuBuffer, seq: usize, dim: usize, base: f64,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(seq * dim)?;
      unsafe {
            launch_rope(
                  x.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  seq as i32, dim as i32,
                  base as f32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_positional_encoding ───────────────────────────────────────────────
// Returns f32 sinusoidal table of shape (seq, dim).
pub fn gpu_positional_encoding(seq: usize, dim: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(seq * dim)?;
      unsafe {
            launch_positional_encoding(
                  out.ptr_raw(),
                  seq as i32, dim as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_rmsnorm ──────────────────────────────────────────────────────────
// x: f32 (rows, cols). gamma: f32 (cols,). eps: small positive stabilizer.
pub fn gpu_rmsnorm(
      x: &GpuBuffer, gamma: &GpuBuffer,
      rows: usize, cols: usize, eps: f64,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::zeros_f32(rows * cols)?;
      unsafe {
            launch_rmsnorm(
                  x.ptr_raw() as *const c_void,
                  gamma.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  rows as i32, cols as i32,
                  eps as f32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_rmsnorm_backward ─────────────────────────────────────────────────
// Returns (grad_x, grad_gamma). grad_gamma is zeroed before launch.
pub fn gpu_rmsnorm_backward(
      grad_out: &GpuBuffer, x: &GpuBuffer, gamma: &GpuBuffer,
      rows: usize, cols: usize, eps: f64,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let grad_x = GpuBuffer::zeros_f32(rows * cols)?;
      let grad_gamma = GpuBuffer::zeros_f32(cols)?;
      unsafe {
            launch_rmsnorm_backward(
                  grad_out.ptr_raw() as *const c_void,
                  x.ptr_raw() as *const c_void,
                  gamma.ptr_raw() as *const c_void,
                  grad_x.ptr_raw(),
                  grad_gamma.ptr_raw(),
                  rows as i32, cols as i32,
                  eps as f32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok((grad_x, grad_gamma))
}

// ── gpu_im2col_2d_ext ─────────────────────────────────────────────────────
// x: f32 NCHW (n, c, h, w).
// out_h = (h + 2*pad_h - dil_h*(kh-1) - 1) / sh + 1
// out_w = (w + 2*pad_w - dil_w*(kw-1) - 1) / sw + 1
// Output: f32 (n * out_h * out_w, c * kh * kw). Zero-pads out-of-bounds.
pub fn gpu_im2col_2d_ext(
      x: &GpuBuffer,
      n: usize, c: usize, h: usize, w: usize,
      kh: usize, kw: usize,
      sh: usize, sw: usize,
      pad_h: usize, pad_w: usize,
      dil_h: usize, dil_w: usize,
) -> Result<GpuBuffer, HipError> {
      let out_h = (h + 2 * pad_h - dil_h * (kh - 1) - 1) / sh + 1;
      let out_w = (w + 2 * pad_w - dil_w * (kw - 1) - 1) / sw + 1;
      let patches = GpuBuffer::zeros_f32(n * out_h * out_w * c * kh * kw)?;
      unsafe {
            launch_im2col_2d_ext(
                  x.ptr_raw() as *const c_void,
                  patches.ptr_raw(),
                  n as i32, c as i32, h as i32, w as i32,
                  kh as i32, kw as i32,
                  sh as i32, sw as i32,
                  pad_h as i32, pad_w as i32,
                  dil_h as i32, dil_w as i32,
                  out_h as i32, out_w as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(patches)
}

// ── gpu_col2im_2d_ext ─────────────────────────────────────────────────────
// Inverse of gpu_im2col_2d_ext. Accumulates patch gradients into image grad.
// out: zeroed f32 NCHW (n, c, h, w). Formula same as im2col_2d_ext.
pub fn gpu_col2im_2d_ext(
      patches: &GpuBuffer,
      n: usize, c: usize, h: usize, w: usize,
      kh: usize, kw: usize,
      sh: usize, sw: usize,
      pad_h: usize, pad_w: usize,
      dil_h: usize, dil_w: usize,
) -> Result<GpuBuffer, HipError> {
      let out_h = (h + 2 * pad_h - dil_h * (kh - 1) - 1) / sh + 1;
      let out_w = (w + 2 * pad_w - dil_w * (kw - 1) - 1) / sw + 1;
      let out = GpuBuffer::zeros_f32(n * c * h * w)?;
      unsafe {
            launch_col2im_2d_ext(
                  patches.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32, c as i32, h as i32, w as i32,
                  kh as i32, kw as i32,
                  sh as i32, sw as i32,
                  pad_h as i32, pad_w as i32,
                  dil_h as i32, dil_w as i32,
                  out_h as i32, out_w as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

// ── gpu_embedding_backward ────────────────────────────────────────────────
// grad_out: f32 (n, cols). indices: i32 (n,) token ids in [0, vocab).
// Returns grad_table f32 (vocab, cols) — zeroed, then accumulated via atomicAdd.
// Atomic path: every (row, col) writes to grad_table[indices[row], col].
// Concurrent same-token rows accumulate correctly; no dedup required.
pub fn gpu_embedding_backward(
      grad_out: &GpuBuffer, indices: &GpuBuffer,
      n: usize, cols: usize, vocab: usize,
) -> Result<GpuBuffer, HipError> {
      let grad_table = GpuBuffer::zeros_f32(vocab * cols)?;
      unsafe {
            launch_embedding_backward(
                  grad_out.ptr_raw() as *const c_void,
                  indices.ptr_raw() as *const c_void,
                  grad_table.ptr_raw(),
                  n as i32, cols as i32, vocab as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(grad_table)
}

// ── gpu_bn_update_running ─────────────────────────────────────────────────
// In-place update: run = (1-momentum)*run + momentum*save.
// run_mean, run_var: mutable f32 (c,). save_mean, save_var: f32 (c,) from forward.
pub fn gpu_bn_update_running(
      run_mean: &GpuBuffer, run_var: &GpuBuffer,
      save_mean: &GpuBuffer, save_var: &GpuBuffer,
      momentum: f64, c: usize,
) {
      unsafe {
            launch_bn_update_running(
                  run_mean.ptr_raw(),
                  run_var.ptr_raw(),
                  save_mean.ptr_raw() as *const c_void,
                  save_var.ptr_raw() as *const c_void,
                  momentum as f32, c as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
}

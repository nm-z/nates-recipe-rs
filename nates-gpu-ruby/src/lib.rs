use magnus::prelude::*;
use magnus::{function, method, Error, RArray, Ruby, Value, TryConvert};
use gpu_core::memory::GpuBuffer;
use gpu_core::kernels;
use std::sync::Arc;
use std::sync::Mutex;
static CATBOOST_MODELS: Mutex<Vec<catboost_rs::Model>> = Mutex::new(Vec::new());
static XGBOOST_MODELS: Mutex<Vec<xgboost_rs::Model>> = Mutex::new(Vec::new());
static LIGHTGBM_MODELS: Mutex<Vec<lightgbm_rs::Model>> = Mutex::new(Vec::new());

#[magnus::wrap(class = "NatesGpu::GpuBuffer")]
struct RubyGpuBuffer {
      buf: Arc<GpuBuffer>,
      rows: usize,
      cols: usize,
}

impl RubyGpuBuffer {
      fn new(buf: GpuBuffer, rows: usize, cols: usize) -> Self {
            Self { buf: Arc::new(buf), rows, cols }
      }

      fn rows(&self) -> usize { self.rows }
      fn cols(&self) -> usize { self.cols }
      fn len(&self) -> usize { self.rows * self.cols }

      fn to_s(&self) -> String {
            format!("GpuBuffer[{}, {}]", self.rows, self.cols)
      }

      fn ptr_addr(&self) -> String {
            format!("0x{:x}", self.buf.ptr_addr())
      }

      fn reuse(&self) -> RubyGpuBuffer {
            RubyGpuBuffer { buf: Arc::clone(&self.buf), rows: self.rows, cols: self.cols }
      }

      fn unique(&self) -> bool { Arc::strong_count(&self.buf) == 1 }

      fn op_add(ruby: &Ruby, rb_self: &RubyGpuBuffer, other: Value) -> Result<RubyGpuBuffer, Error> {
            if let Ok(s) = f64::try_convert(other) {
                  if rb_self.unique() {
                        kernels::gpu_add_scalar_inplace(&rb_self.buf, s, rb_self.len());
                        return Ok(rb_self.reuse());
                  }
                  let buf = kernels::gpu_add_scalar(&rb_self.buf, s, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  return Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols));
            }
            let b: &RubyGpuBuffer = TryConvert::try_convert(other)?;
            if b.rows == 1 && rb_self.rows > 1 {
                  let buf = kernels::gpu_bias_add(&rb_self.buf, &b.buf, rb_self.rows, rb_self.cols).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            } else if rb_self.unique() {
                  kernels::gpu_add_inplace(&rb_self.buf, &b.buf, rb_self.len());
                  Ok(rb_self.reuse())
            } else {
                  let buf = kernels::gpu_add(&rb_self.buf, &b.buf, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            }
      }

      fn op_sub(ruby: &Ruby, rb_self: &RubyGpuBuffer, other: Value) -> Result<RubyGpuBuffer, Error> {
            if let Ok(s) = f64::try_convert(other) {
                  if rb_self.unique() {
                        kernels::gpu_add_scalar_inplace(&rb_self.buf, -s, rb_self.len());
                        return Ok(rb_self.reuse());
                  }
                  let buf = kernels::gpu_add_scalar(&rb_self.buf, -s, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  return Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols));
            }
            let b: &RubyGpuBuffer = TryConvert::try_convert(other)?;
            if b.rows == 1 && rb_self.rows > 1 {
                  let buf = kernels::gpu_broadcast_sub(&rb_self.buf, &b.buf, rb_self.len(), rb_self.cols).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            } else if rb_self.unique() {
                  kernels::gpu_sub_inplace(&rb_self.buf, &b.buf, rb_self.len());
                  Ok(rb_self.reuse())
            } else {
                  let buf = kernels::gpu_sub(&rb_self.buf, &b.buf, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            }
      }

      fn op_mul(ruby: &Ruby, rb_self: &RubyGpuBuffer, other: Value) -> Result<RubyGpuBuffer, Error> {
            if let Ok(s) = f64::try_convert(other) {
                  if rb_self.unique() {
                        kernels::gpu_scale_inplace(&rb_self.buf, s, rb_self.len());
                        return Ok(rb_self.reuse());
                  }
                  let buf = kernels::gpu_scale(&rb_self.buf, s, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  return Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols));
            }
            let b: &RubyGpuBuffer = TryConvert::try_convert(other)?;
            if b.rows == 1 && rb_self.rows > 1 {
                  let buf = kernels::gpu_broadcast_mul(&rb_self.buf, &b.buf, rb_self.len(), rb_self.cols).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            } else if rb_self.unique() {
                  kernels::gpu_mul_inplace(&rb_self.buf, &b.buf, rb_self.len());
                  Ok(rb_self.reuse())
            } else {
                  let buf = kernels::gpu_mul(&rb_self.buf, &b.buf, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            }
      }

      fn op_div(ruby: &Ruby, rb_self: &RubyGpuBuffer, other: Value) -> Result<RubyGpuBuffer, Error> {
            if let Ok(s) = f64::try_convert(other) {
                  let buf = kernels::gpu_scale(&rb_self.buf, 1.0 / s, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  return Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols));
            }
            let b: &RubyGpuBuffer = TryConvert::try_convert(other)?;
            if b.rows == 1 && rb_self.rows > 1 {
                  let buf = kernels::gpu_broadcast_div(&rb_self.buf, &b.buf, rb_self.len(), rb_self.cols).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            } else {
                  let buf = kernels::gpu_div(&rb_self.buf, &b.buf, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
                  Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
            }
      }

      fn op_neg(ruby: &Ruby, rb_self: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
            let buf = kernels::gpu_neg(&rb_self.buf, rb_self.len()).map_err(|e| hip_err(ruby, e))?;
            Ok(RubyGpuBuffer::new(buf, rb_self.rows, rb_self.cols))
      }
}

fn gc_hook() {
      unsafe { rb_sys::rb_gc_start(); }
      gpu_core::hip::device_synchronize().ok();
}

fn hip_err(ruby: &Ruby, e: gpu_core::hip::HipError) -> Error {
      Error::new(ruby.exception_runtime_error(), format!("{e}"))
}

fn gpu_gc(ruby: &Ruby) -> Result<(), Error> {
      unsafe { rb_sys::rb_gc_start() };
      gpu_core::hip::device_synchronize().map_err(|e| hip_err(ruby, e))?;
      Ok(())
}

fn gpu_stats(_ruby: &Ruby) -> Result<RArray, Error> {
      let (free, total) = gpu_core::hip::mem_info().unwrap_or((0, 0));
      let ruby = unsafe { Ruby::get_unchecked() };
      let arr = ruby.ary_new();
      arr.push(free / 1048576).ok();
      arr.push(total / 1048576).ok();
      Ok(arr)
}

fn alloc_count_reset(_ruby: &Ruby) -> usize {
      gpu_core::memory::alloc_count_reset()
}

// ── Data transfer ───────────────────────────────────────────────────────────

fn upload(ruby: &Ruby, data: RArray, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let v: Vec<f64> = data.to_vec()?;
      if v.len() != rows * cols {
            return Err(Error::new(
                  ruby.exception_runtime_error(),
                  format!("upload: data len {} != rows*cols {}*{}={}", v.len(), rows, cols, rows * cols),
            ));
      }
      let buf = GpuBuffer::upload(&v).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn upload_u8(ruby: &Ruby, data: Vec<i64>, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let v: Vec<u8> = data.iter().map(|&x| x as u8).collect();
      let buf = GpuBuffer::upload_u8(&v).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn upload_i32(ruby: &Ruby, data: Vec<i64>, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let v: Vec<i32> = data.iter().map(|&x| x as i32).collect();
      let bytes = v.len() * 4;
      let buf = GpuBuffer::alloc_bytes(bytes).map_err(|e| hip_err(ruby, e))?;
      gpu_core::hip::check(unsafe {
            gpu_core::hip::hipMemcpy(buf.ptr_raw(), v.as_ptr() as *const std::ffi::c_void, bytes, gpu_core::hip::HIP_MEMCPY_H2D)
      }).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn upload_f32(ruby: &Ruby, data: Vec<f64>, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let v: Vec<f32> = data.iter().map(|&x| x as f32).collect();
      let buf = GpuBuffer::upload_f32(&v).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn zeros_u8(ruby: &Ruby, n: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = GpuBuffer::zeros_bytes(n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, 1))
}

fn zeros_f32(ruby: &Ruby, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = GpuBuffer::zeros_bytes(rows * cols * 4).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn download_f32(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RArray, Error> {
      let n = x.buf.len() / 4;
      let mut dst = vec![0.0f32; n];
      gpu_core::hip::check(unsafe {
            gpu_core::hip::hipMemcpy(dst.as_mut_ptr() as *mut std::ffi::c_void, x.buf.ptr_raw(), x.buf.len(), gpu_core::hip::HIP_MEMCPY_D2H)
      }).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      for v in dst { arr.push(v as f64).ok(); }
      Ok(arr)
}

fn download(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RArray, Error> {
      let mut dst = vec![0.0f64; x.len()];
      x.buf.download(&mut dst).map_err(|e| hip_err(ruby, e))?;
      Ok(ruby.ary_from_vec(dst))
}

// ── BLAS ────────────────────────────────────────────────────────────────────

fn gemm(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer, ta: String, tb: String) -> Result<RubyGpuBuffer, Error> {
      let (m, n, k, buf) = match (ta.as_str(), tb.as_str()) {
            ("N", "N") => (a.rows, b.cols, a.cols, kernels::gpu_gemm(&a.buf, &b.buf, a.rows, b.cols, a.cols).map_err(|e| hip_err(ruby, e))?),
            ("T", "N") => (a.cols, b.cols, a.rows, kernels::gpu_gemm_at(&a.buf, &b.buf, a.cols, b.cols, a.rows).map_err(|e| hip_err(ruby, e))?),
            ("N", "T") => (a.rows, b.rows, a.cols, kernels::gpu_gemm_bt(&a.buf, &b.buf, a.rows, b.rows, a.cols).map_err(|e| hip_err(ruby, e))?),
            _ => return Err(Error::new(ruby.exception_runtime_error(), format!("gemm: invalid transpose flags ({ta}, {tb}), expected N or T"))),
      };
      let _ = k;
      Ok(RubyGpuBuffer::new(buf, m, n))
}

fn cholesky_solve(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer, n: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_cholesky_solve(&a.buf, &b.buf, n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, 1))
}

fn cholesky_inv(ruby: &Ruby, a: &RubyGpuBuffer, n: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_cholesky_inv(&a.buf, n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, n))
}

fn solve(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = a.rows;
      let nrhs = b.cols;
      let buf = kernels::gpu_solve(&a.buf, &b.buf, n, nrhs).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, nrhs))
}

fn cholesky(ruby: &Ruby, a: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = a.rows;
      let buf = kernels::gpu_cholesky(&a.buf, n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, n))
}

fn tri_solve(ruby: &Ruby, l: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = l.rows;
      let nrhs = b.cols;
      let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, false).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, nrhs))
}

fn tri_solve_t(ruby: &Ruby, l: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = l.rows;
      let nrhs = b.cols;
      let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, true).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, nrhs))
}

// ── Elementwise binary ──────────────────────────────────────────────────────

fn add(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_add(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}

fn sub(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sub(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}

fn mul(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_mul(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}

fn scale(ruby: &Ruby, x: &RubyGpuBuffer, s: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_scale(&x.buf, s, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn sub_scale(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer, s: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sub_scale(&a.buf, &b.buf, a.len(), s).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}

fn fma(ruby: &Ruby, x: &RubyGpuBuffer, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_fma(&x.buf, &a.buf, &b.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── In-place ────────────────────────────────────────────────────────────────

fn shape_err(ruby: &Ruby, msg: String) -> Error {
      Error::new(ruby.exception_runtime_error(), msg)
}

fn linear_into(ruby: &Ruby, x: &RubyGpuBuffer, w: &RubyGpuBuffer, b: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      if x.cols != w.rows { return Err(shape_err(ruby, format!("linear_into!: x.cols({}) != w.rows({})", x.cols, w.rows))); }
      if out.rows != x.rows || out.cols != w.cols { return Err(shape_err(ruby, format!("linear_into!: out shape [{},{}] expected [{},{}]", out.rows, out.cols, x.rows, w.cols))); }
      kernels::gpu_linear_into(&x.buf, &w.buf, &b.buf, &out.buf, x.rows, w.cols, x.cols);
      Ok(())
}

fn layernorm_into(ruby: &Ruby, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, beta: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      if out.len() != x.len() { return Err(shape_err(ruby, format!("layernorm_into!: out.len({}) != x.len({})", out.len(), x.len()))); }
      kernels::gpu_layernorm_into(&x.buf, &out.buf, Some(&gamma.buf), Some(&beta.buf), x.rows, x.cols);
      Ok(())
}

fn gelu_into(ruby: &Ruby, x: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      if out.len() != x.len() { return Err(shape_err(ruby, format!("gelu_into!: out.len({}) != x.len({})", out.len(), x.len()))); }
      kernels::gpu_gelu_into(&x.buf, &out.buf, x.len());
      Ok(())
}

fn gelu_backward_into(ruby: &Ruby, grad: &RubyGpuBuffer, x: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      if grad.len() != x.len() || out.len() != x.len() { return Err(shape_err(ruby, format!("gelu_backward_into!: shape mismatch grad={} x={} out={}", grad.len(), x.len(), out.len()))); }
      kernels::gpu_gelu_backward_into(&grad.buf, &x.buf, &out.buf, grad.len());
      Ok(())
}

fn dropout_into(ruby: &Ruby, x: &RubyGpuBuffer, mask: &RubyGpuBuffer, p: f64, out: &RubyGpuBuffer) -> Result<(), Error> {
      if mask.len() != x.len() || out.len() != x.len() { return Err(shape_err(ruby, format!("dropout_into!: shape mismatch x={} mask={} out={}", x.len(), mask.len(), out.len()))); }
      kernels::gpu_dropout_into(&x.buf, &mask.buf, &out.buf, x.len(), p);
      Ok(())
}

fn dropout_u8_into(ruby: &Ruby, x: &RubyGpuBuffer, mask: &RubyGpuBuffer, p: f64, out: &RubyGpuBuffer) -> Result<(), Error> {
      let n = x.len();
      if out.len() != n { return Err(shape_err(ruby, format!("dropout_u8_into!: shape mismatch x={} out={}", n, out.len()))); }
      kernels::gpu_dropout_u8_into(&x.buf, &mask.buf, &out.buf, n, p);
      Ok(())
}

fn bernoulli_u8_into(_ruby: &Ruby, mask: &RubyGpuBuffer, seed: u32, p: f64) -> Result<(), Error> {
      kernels::gpu_bernoulli_u8(&mask.buf, mask.len(), seed, p);
      Ok(())
}

fn rand_uniform_into(_ruby: &Ruby, out: &RubyGpuBuffer, seed: u32) -> Result<(), Error> {
      kernels::gpu_rand_uniform_into(&out.buf, out.len(), seed);
      Ok(())
}

fn linear_backward_into(ruby: &Ruby, grad: &RubyGpuBuffer, input: &RubyGpuBuffer, weight: &RubyGpuBuffer, grad_input: &RubyGpuBuffer) -> Result<RArray, Error> {
      let (gw, gb) = kernels::gpu_linear_backward_into(&grad.buf, &input.buf, &weight.buf, &grad_input.buf, grad.rows, grad.cols, input.cols).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gw, input.cols, grad.cols)).ok();
      arr.push(RubyGpuBuffer::new(gb, 1, grad.cols)).ok();
      Ok(arr)
}

fn layernorm_backward_into(ruby: &Ruby, grad: &RubyGpuBuffer, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, eps: f64, grad_x: &RubyGpuBuffer) -> Result<RArray, Error> {
      let (gg, gb) = kernels::gpu_layernorm_backward_into(&grad.buf, &x.buf, &gamma.buf, &grad_x.buf, grad.rows, grad.cols, eps).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gg, 1, grad.cols)).ok();
      arr.push(RubyGpuBuffer::new(gb, 1, grad.cols)).ok();
      Ok(arr)
}

// ── Oblivious tree ─────────────────────────────────────────────────────────

fn mse_grad_into(_ruby: &Ruby, pred: &RubyGpuBuffer, target: &RubyGpuBuffer, grad: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_mse_grad_into(&pred.buf, &target.buf, &grad.buf, pred.len());
      Ok(())
}

fn softmax_ce_class_grad_f32_into(ruby: &Ruby, preds: RArray, target: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, k: usize, n: usize) -> Result<(), Error> {
      let nc = preds.len();
      let mut ptrs: Vec<*mut std::ffi::c_void> = Vec::with_capacity(nc);
      for i in 0..nc {
            let buf: &RubyGpuBuffer = preds.entry(i as isize).map_err(|e| shape_err(ruby, format!("softmax_ce_class_grad_f32: bad preds[{}]: {}", i, e)))?;
            ptrs.push(buf.buf.ptr_raw());
      }
      kernels::gpu_softmax_ce_class_grad_f32(&ptrs, &target.buf, &grad.buf, &hess.buf, k, n);
      Ok(())
}

fn logloss_grad_f32_into(_ruby: &Ruby, pred: &RubyGpuBuffer, target: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_logloss_grad_f32(&pred.buf, &target.buf, &grad.buf, &hess.buf, pred.len());
      Ok(())
}

fn argmax_f32_into(_ruby: &Ruby, data: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_argmax_f32(&data.buf, &out.buf, data.len());
      Ok(())
}

fn fill_f32_val(_ruby: &Ruby, out: &RubyGpuBuffer, val: f64) -> Result<(), Error> {
      kernels::gpu_fill_f32(&out.buf, val as f32, out.len());
      Ok(())
}

fn argmax_write_split_into(_ruby: &Ruby, gain: &RubyGpuBuffer, sf: &RubyGpuBuffer, sb: &RubyGpuBuffer, best_idx: &RubyGpuBuffer, n_bins: usize, d: usize) -> Result<(), Error> {
      let n_features = gain.len() / n_bins;
      kernels::gpu_argmax_write_split(&gain.buf, &sf.buf, &sb.buf, &best_idx.buf, n_features, n_bins, d);
      Ok(())
}

fn gpu_sync(_ruby: &Ruby) -> Result<f64, Error> {
      gpu_core::hip::check(unsafe { gpu_core::hip::hipDeviceSynchronize() }).ok();
      Ok(0.0)
}

fn write_split_into(_ruby: &Ruby, sf: &RubyGpuBuffer, sb: &RubyGpuBuffer, feat: usize, bin: usize, d: usize) -> Result<(), Error> {
      kernels::gpu_write_split(&sf.buf, &sb.buf, feat, bin as u8, d);
      Ok(())
}

fn download_i32_scalar(ruby: &Ruby, buf: &RubyGpuBuffer) -> Result<i64, Error> {
      let mut v = [0i32; 1];
      gpu_core::hip::check(unsafe {
            gpu_core::hip::hipMemcpy(v.as_mut_ptr() as *mut std::ffi::c_void, buf.buf.ptr_raw(), 4, gpu_core::hip::HIP_MEMCPY_D2H)
      }).map_err(|e| hip_err(ruby, e))?;
      Ok(v[0] as i64)
}

fn oblivious_histogram_into(_ruby: &Ruby, bins_fm: &RubyGpuBuffer, node_idx: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, grad_hist: &RubyGpuBuffer, hess_hist: &RubyGpuBuffer, n_bins: usize, n_nodes: usize) -> Result<(), Error> {
      let n_rows = grad.len();
      let n_features = bins_fm.len() / n_rows;
      kernels::gpu_oblivious_histogram(&bins_fm.buf, &node_idx.buf, &grad.buf, &hess.buf, &grad_hist.buf, &hess_hist.buf, n_rows, n_features, n_bins, n_nodes);
      Ok(())
}

fn oblivious_route_step_into(_ruby: &Ruby, bins_rm: &RubyGpuBuffer, node_in: &RubyGpuBuffer, node_out: &RubyGpuBuffer, split_feat: usize, split_bin: usize, depth: usize) -> Result<(), Error> {
      let n_rows = node_in.len();
      let n_features = bins_rm.len() / n_rows;
      kernels::gpu_oblivious_route_step(&bins_rm.buf, &node_in.buf, &node_out.buf, split_feat, split_bin as u8, depth, n_rows, n_features);
      Ok(())
}

fn oblivious_route_step_dev_into(_ruby: &Ruby, bins_rm: &RubyGpuBuffer, node_in: &RubyGpuBuffer, node_out: &RubyGpuBuffer, split_feat_arr: &RubyGpuBuffer, split_bin_arr: &RubyGpuBuffer, depth: usize) -> Result<(), Error> {
      let n_rows = node_in.len();
      let n_features = bins_rm.len() / n_rows;
      kernels::gpu_oblivious_route_step_dev(&bins_rm.buf, &node_in.buf, &node_out.buf, &split_feat_arr.buf, &split_bin_arr.buf, depth, n_rows, n_features);
      Ok(())
}

fn oblivious_route_full_into(_ruby: &Ruby, bins_rm: &RubyGpuBuffer, split_feat: &RubyGpuBuffer, split_bin: &RubyGpuBuffer, leaf_idx: &RubyGpuBuffer, depth: usize) -> Result<(), Error> {
      let n_rows = leaf_idx.len();
      let n_features = bins_rm.len() / n_rows;
      kernels::gpu_oblivious_route_full(&bins_rm.buf, &split_feat.buf, &split_bin.buf, &leaf_idx.buf, n_rows, n_features, depth);
      Ok(())
}

fn scatter_add_by_leaf(_ruby: &Ruby, pred: &RubyGpuBuffer, leaf_idx: &RubyGpuBuffer, leaf_value: &RubyGpuBuffer, lr: f64) -> Result<(), Error> {
      kernels::gpu_scatter_add_by_leaf(&pred.buf, &leaf_idx.buf, &leaf_value.buf, lr as f32, pred.len());
      Ok(())
}

fn softmax_inplace_mc(_ruby: &Ruby, pred: &RubyGpuBuffer, n_classes: usize) -> Result<(), Error> {
      let n_rows = pred.len() / n_classes;
      kernels::gpu_softmax_inplace(&pred.buf, n_rows, n_classes);
      Ok(())
}

fn logloss_grad_mc_into(_ruby: &Ruby, pred: &RubyGpuBuffer, tgt: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, n_classes: usize) -> Result<(), Error> {
      let n_rows = tgt.len();
      kernels::gpu_logloss_grad_mc(&pred.buf, &tgt.buf, &grad.buf, &hess.buf, n_rows, n_classes);
      Ok(())
}

fn gpu_accuracy(_ruby: &Ruby, pred: &RubyGpuBuffer, tgt: &RubyGpuBuffer, n_classes: usize) -> Result<f64, Error> {
      let n_rows = tgt.len();
      let out = GpuBuffer::zeros_bytes(4).expect("alloc");
      kernels::gpu_accuracy(&pred.buf, &tgt.buf, &out, n_rows, n_classes);
      let mut v = [0f32; 1];
      gpu_core::hip::check(unsafe {
            gpu_core::hip::hipMemcpy(v.as_mut_ptr() as *mut std::ffi::c_void, out.ptr_raw(), 4, gpu_core::hip::HIP_MEMCPY_D2H)
      }).ok();
      Ok(v[0] as f64 / n_rows as f64)
}

fn scatter_add_by_leaf_col(_ruby: &Ruby, pred: &RubyGpuBuffer, leaf_idx: &RubyGpuBuffer, leaf_value: &RubyGpuBuffer, lr: f64, n_classes: usize, col: usize) -> Result<(), Error> {
      let n_rows = leaf_idx.len();
      kernels::gpu_scatter_add_by_leaf_col(&pred.buf, &leaf_idx.buf, &leaf_value.buf, lr as f32, n_rows, n_classes, col);
      Ok(())
}

fn leaf_reduce_into(_ruby: &Ruby, leaf_idx: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, leaf_grad: &RubyGpuBuffer, leaf_hess: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_leaf_reduce(&leaf_idx.buf, &grad.buf, &hess.buf, &leaf_grad.buf, &leaf_hess.buf, grad.len());
      Ok(())
}

fn leaf_finalize_into(_ruby: &Ruby, leaf_grad: &RubyGpuBuffer, leaf_hess: &RubyGpuBuffer, leaf_value: &RubyGpuBuffer, lambda: f64) -> Result<(), Error> {
      kernels::gpu_leaf_finalize(&leaf_grad.buf, &leaf_hess.buf, &leaf_value.buf, lambda as f32, leaf_value.len());
      Ok(())
}

fn oblivious_split_eval_into(_ruby: &Ruby, grad_hist: &RubyGpuBuffer, hess_hist: &RubyGpuBuffer, gain_out: &RubyGpuBuffer, n_nodes: usize, n_bins: usize, lambda: f64) -> Result<(), Error> {
      let n_features = gain_out.len() / n_bins;
      kernels::gpu_oblivious_split_eval(&grad_hist.buf, &hess_hist.buf, &gain_out.buf, n_nodes, n_features, n_bins, lambda as f32, 1e-3);
      Ok(())
}

fn softmax_into(_ruby: &Ruby, x: &RubyGpuBuffer, out: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_softmax_rows_into(&x.buf, &out.buf, x.rows, x.cols);
      Ok(())
}

fn softmax_ce_grad_into(_ruby: &Ruby, logits: &RubyGpuBuffer, targets: &RubyGpuBuffer, weights: &RubyGpuBuffer, grad_out: &RubyGpuBuffer, scale: f64) -> Result<(), Error> {
      kernels::gpu_softmax_ce_grad_into(&logits.buf, &targets.buf, &weights.buf, &grad_out.buf, logits.rows, logits.cols, scale);
      Ok(())
}

fn linear_backward_weights_only_into(_ruby: &Ruby, grad: &RubyGpuBuffer, input: &RubyGpuBuffer, grad_w: &RubyGpuBuffer, grad_b: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_linear_backward_weights_only_into(&grad.buf, &input.buf, &grad_w.buf, &grad_b.buf, grad.rows, grad.cols, input.cols);
      Ok(())
}

fn grad_clip_norm_scratch(_ruby: &Ruby, x: &RubyGpuBuffer, max_norm: f64, tmp: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_grad_clip_norm_with_tmp(&x.buf, &tmp.buf, x.len(), max_norm);
      Ok(())
}

fn linear_backward_weights_only(ruby: &Ruby, grad: &RubyGpuBuffer, input: &RubyGpuBuffer) -> Result<RArray, Error> {
      let (gw, gb) = kernels::gpu_linear_backward_weights_only(&grad.buf, &input.buf, grad.rows, grad.cols, input.cols).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gw, input.cols, grad.cols)).ok();
      arr.push(RubyGpuBuffer::new(gb, 1, grad.cols)).ok();
      Ok(arr)
}

fn linear_backward_full_into(_ruby: &Ruby, grad: &RubyGpuBuffer, input: &RubyGpuBuffer, weight: &RubyGpuBuffer, grad_input: &RubyGpuBuffer, grad_w: &RubyGpuBuffer, grad_b: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_linear_backward_full_into(&grad.buf, &input.buf, &weight.buf, &grad_input.buf, &grad_w.buf, &grad_b.buf, grad.rows, grad.cols, input.cols);
      Ok(())
}

fn layernorm_backward_full_into(_ruby: &Ruby, grad: &RubyGpuBuffer, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, eps: f64, grad_x: &RubyGpuBuffer, grad_gamma: &RubyGpuBuffer, grad_beta: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_layernorm_backward_full_into(&grad.buf, &x.buf, &gamma.buf, &grad_x.buf, &grad_gamma.buf, &grad_beta.buf, grad.rows, grad.cols, eps);
      Ok(())
}

fn bernoulli_into(_ruby: &Ruby, out: &RubyGpuBuffer, p: f64, seed: u32) -> Result<(), Error> {
      kernels::gpu_bernoulli_into(&out.buf, out.len(), p, seed);
      Ok(())
}

fn grad_hess_into(ruby: &Ruby, probs: &RubyGpuBuffer, targets: &RubyGpuBuffer, weights: &RubyGpuBuffer, mask: &RubyGpuBuffer, grad_out: &RubyGpuBuffer, hess_out: &RubyGpuBuffer, k: usize) -> Result<(), Error> {
      if k >= probs.cols { return Err(shape_err(ruby, format!("grad_hess_into!: k({}) >= probs.cols({})", k, probs.cols))); }
      if grad_out.rows != probs.rows || hess_out.rows != probs.rows { return Err(shape_err(ruby, format!("grad_hess_into!: output rows mismatch"))); }
      kernels::gpu_grad_hess_into(&probs.buf, &targets.buf, &weights.buf, &mask.buf, &grad_out.buf, &hess_out.buf, probs.rows, probs.cols, k);
      Ok(())
}

fn tree_build_into(_ruby: &Ruby, tr_bins: &RubyGpuBuffer, te_bins: &RubyGpuBuffer, g: &RubyGpuBuffer, h: &RubyGpuBuffer, n_bins: usize, depth: usize, lambda: f64, mcw: f64, tr_pred: &RubyGpuBuffer, te_pred: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_tree_build_into(&tr_bins.buf, &te_bins.buf, &g.buf, &h.buf, tr_bins.rows, te_bins.rows, tr_bins.cols, n_bins, depth, lambda, mcw, &tr_pred.buf, &te_pred.buf);
      Ok(())
}

fn zero_buf(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<(), Error> {
      gpu_core::hip::check(unsafe { gpu_core::hip::hipMemset(x.buf.ptr_raw(), 0, x.buf.len()) }).map_err(|e| hip_err(ruby, e))?;
      Ok(())
}

fn add_inplace(_ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_add_inplace(&a.buf, &b.buf, a.len());
      Ok(())
}

fn mul_inplace(_ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<(), Error> {
      kernels::gpu_mul_inplace(&a.buf, &b.buf, a.len());
      Ok(())
}

fn add_col_scaled_inplace(ruby: &Ruby, matrix: &RubyGpuBuffer, k: usize, col: &RubyGpuBuffer, scale: f64) -> Result<(), Error> {
      if k >= matrix.cols { return Err(shape_err(ruby, format!("add_col!: k({}) >= cols({})", k, matrix.cols))); }
      if col.rows != matrix.rows { return Err(shape_err(ruby, format!("add_col!: col.rows({}) != matrix.rows({})", col.rows, matrix.rows))); }
      kernels::gpu_add_col_scaled_inplace(&matrix.buf, matrix.rows, matrix.cols, k, &col.buf, scale);
      Ok(())
}

fn scale_inplace(_ruby: &Ruby, x: &RubyGpuBuffer, s: f64) -> Result<(), Error> {
      kernels::gpu_scale_inplace(&x.buf, s, x.len());
      Ok(())
}

fn diag_add(_ruby: &Ruby, a: &RubyGpuBuffer, val: f64) -> Result<RubyGpuBuffer, Error> {
      kernels::gpu_add_diag(&a.buf, a.rows, val);
      Ok(RubyGpuBuffer { buf: Arc::clone(&a.buf), rows: a.rows, cols: a.cols })
}

fn sgd_update(_ruby: &Ruby, w: &RubyGpuBuffer, grad: &RubyGpuBuffer, lr: f64) -> Result<(), Error> {
      kernels::gpu_sgd_update(&w.buf, &grad.buf, lr, w.len());
      Ok(())
}

// ── Activations ─────────────────────────────────────────────────────────────

fn sigmoid(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sigmoid(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn sigmoid_backward(ruby: &Ruby, grad: &RubyGpuBuffer, act: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sigmoid_backward(&grad.buf, &act.buf, grad.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

fn tanh_act(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_tanh(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn tanh_backward(ruby: &Ruby, grad: &RubyGpuBuffer, act: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_tanh_backward(&grad.buf, &act.buf, grad.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

fn relu(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_relu(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn relu_backward(ruby: &Ruby, grad: &RubyGpuBuffer, act: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_relu_backward(&grad.buf, &act.buf, grad.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

fn leaky_relu(ruby: &Ruby, x: &RubyGpuBuffer, alpha: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_leaky_relu(&x.buf, x.len(), alpha).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn leaky_relu_backward(ruby: &Ruby, grad: &RubyGpuBuffer, act: &RubyGpuBuffer, alpha: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_leaky_relu_backward(&grad.buf, &act.buf, grad.len(), alpha).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

fn softmax(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_softmax_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn scaled_exp(ruby: &Ruby, x: &RubyGpuBuffer, s: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_scaled_exp(&x.buf, x.len(), s).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Normalization ───────────────────────────────────────────────────────────

fn layernorm(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_layernorm(&x.buf, x.rows, x.cols, None, None).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn layernorm_affine(ruby: &Ruby, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, beta: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_layernorm(&x.buf, x.rows, x.cols, Some(&gamma.buf), Some(&beta.buf)).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn dropout(ruby: &Ruby, x: &RubyGpuBuffer, mask: &RubyGpuBuffer, p: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_dropout(&x.buf, &mask.buf, x.len(), p).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Reductions ──────────────────────────────────────────────────────────────

fn reduce_sum_cols(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_sum_cols(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, 1, x.cols))
}

fn reduce_sum_rows(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_sum_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn reduce_mean_cols(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_mean_cols(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, 1, x.cols))
}

fn reduce_var_cols(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_var_cols(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, 1, x.cols))
}

// ── Bias ────────────────────────────────────────────────────────────────────

fn bias_add(ruby: &Ruby, x: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_bias_add(&x.buf, &b.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Distance / Sorting ──────────────────────────────────────────────────────

fn pairwise_l2(ruby: &Ruby, q: &RubyGpuBuffer, t: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_pairwise_l2(&q.buf, &t.buf, q.rows, t.rows, q.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, q.rows, t.rows))
}

fn argmin_rows(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_argmin_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn argmax_rows(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_argmax_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn topk_per_row(ruby: &Ruby, x: &RubyGpuBuffer, k: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_topk_per_row(&x.buf, x.rows, x.cols, k).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, k))
}

fn partial_argsort(ruby: &Ruby, data: &RubyGpuBuffer, k: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_partial_argsort(&data.buf, data.len(), k).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, k, 1))
}

// ── Convolution ─────────────────────────────────────────────────────────────

fn im2col_1d(ruby: &Ruby, x: &RubyGpuBuffer, ks: usize) -> Result<RubyGpuBuffer, Error> {
      let n = x.rows;
      let p = x.cols;
      let out_len = p - ks + 1;
      let buf = kernels::gpu_im2col_1d(&x.buf, n, p, ks).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * out_len, ks))
}

// im2col_2d(x, shape=[c,h,w], kernel=[kh,kw])
fn im2col_2d(ruby: &Ruby, x: &RubyGpuBuffer, shape: RArray, kernel: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      if sv.len() != 3 {
            return Err(Error::new(ruby.exception_runtime_error(), "im2col_2d: shape must be [c, h, w]"));
      }
      if kv.len() != 2 {
            return Err(Error::new(ruby.exception_runtime_error(), "im2col_2d: kernel must be [kh, kw]"));
      }
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let n = x.len() / (c * h * w);
      let out_h = h - kh + 1;
      let out_w = w - kw + 1;
      let buf = kernels::gpu_im2col_2d(&x.buf, n, c, h, w, kh, kw).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * out_h * out_w, c * kh * kw))
}

fn avg_pool_1d(ruby: &Ruby, x: &RubyGpuBuffer, out_len: usize, n_filters: usize) -> Result<RubyGpuBuffer, Error> {
      let n = x.len() / (out_len * n_filters);
      let buf = kernels::gpu_avg_pool_1d(&x.buf, n, out_len, n_filters).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, n_filters))
}

fn pool_grad_expand(ruby: &Ruby, grad: &RubyGpuBuffer, out_len: usize, n_filters: usize) -> Result<RubyGpuBuffer, Error> {
      let n = grad.len() / n_filters;
      let buf = kernels::gpu_pool_grad_expand(&grad.buf, n, out_len, n_filters).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * out_len, n_filters))
}

// ── Clustering ──────────────────────────────────────────────────────────────

fn centroid_update(ruby: &Ruby, x: &RubyGpuBuffer, assignments: &RubyGpuBuffer, dim: usize, k: usize) -> Result<RArray, Error> {
      let n = x.rows;
      let (centroids, counts) = kernels::gpu_centroid_update(&x.buf, &assignments.buf, n, dim, k).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(centroids, k, dim)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(counts, k, 1)).map_err(|e| e)?;
      Ok(arr)
}

fn gaussian_ll(ruby: &Ruby, x: &RubyGpuBuffer, means: &RubyGpuBuffer, vars: &RubyGpuBuffer, log_priors: &RubyGpuBuffer, k: usize) -> Result<RubyGpuBuffer, Error> {
      let n = x.rows;
      let p = x.cols;
      let buf = kernels::gpu_gaussian_ll(&x.buf, &means.buf, &vars.buf, &log_priors.buf, n, k, p).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, k))
}

// ── Sequence ────────────────────────────────────────────────────────────────

fn lstm_cell(_ruby: &Ruby, gates: &RubyGpuBuffer, c: &RubyGpuBuffer, h: &RubyGpuBuffer, hs: usize) -> Result<RArray, Error> {
      let n = c.rows;
      kernels::gpu_lstm_cell(&gates.buf, &c.buf, &h.buf, n, hs);
      let ruby = unsafe { Ruby::get_unchecked() };
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer { buf: Arc::clone(&c.buf), rows: c.rows, cols: c.cols }).ok();
      arr.push(RubyGpuBuffer { buf: Arc::clone(&h.buf), rows: h.rows, cols: h.cols }).ok();
      Ok(arr)
}

// ── VAE ─────────────────────────────────────────────────────────────────────

fn reparameterize(ruby: &Ruby, mu: &RubyGpuBuffer, log_var: &RubyGpuBuffer, eps: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reparameterize(&mu.buf, &log_var.buf, &eps.buf, mu.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, mu.rows, mu.cols))
}

fn kl_div(ruby: &Ruby, mu: &RubyGpuBuffer, log_var: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_kl_div(&mu.buf, &log_var.buf, mu.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, mu.rows, mu.cols))
}

fn vae_backward_latent(ruby: &Ruby, grad_z: &RubyGpuBuffer, mu: &RubyGpuBuffer, log_var: &RubyGpuBuffer, eps: &RubyGpuBuffer, kl_weight: f64) -> Result<RArray, Error> {
      let n = mu.len();
      let (grad_mu, grad_lv) = kernels::gpu_vae_backward_latent(&grad_z.buf, &mu.buf, &log_var.buf, &eps.buf, n, kl_weight).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(grad_mu, mu.rows, mu.cols)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(grad_lv, mu.rows, mu.cols)).map_err(|e| e)?;
      Ok(arr)
}

fn log_det_cholesky(ruby: &Ruby, l: &RubyGpuBuffer) -> Result<f64, Error> {
      kernels::gpu_log_det_cholesky(&l.buf, l.rows).map_err(|e| hip_err(ruby, e))
}

// ── Misc ────────────────────────────────────────────────────────────────────

fn concat(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_concat(&a.buf, &b.buf, a.rows, a.cols, b.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols + b.cols))
}

// ── Elementwise unary ──────────────────────────────────────────────────────

fn exp(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_exp(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn log(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_log(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn sqrt(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sqrt(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn abs(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_abs(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn neg(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_neg(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn pow(ruby: &Ruby, x: &RubyGpuBuffer, p: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_pow(&x.buf, x.len(), p).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn clamp(ruby: &Ruby, x: &RubyGpuBuffer, lo: f64, hi: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_clamp(&x.buf, x.len(), lo, hi).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Structural ─────────────────────────────────────────────────────────────

fn transpose(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_transpose(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.cols, x.rows))
}

fn eye(ruby: &Ruby, n: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_eye(n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, n))
}

fn copy(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_copy(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn where_mask(ruby: &Ruby, cond: &RubyGpuBuffer, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_where_mask(&cond.buf, &a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}

fn slice_rows(ruby: &Ruby, x: &RubyGpuBuffer, start: usize, count: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_slice_rows(&x.buf, start, count, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, count, x.cols))
}

fn sign(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_sign(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Init helpers ────────────────────────────────────────────────────────────

fn randn(ruby: &Ruby, rows: usize, cols: usize, seed: u64) -> Result<RubyGpuBuffer, Error> {
      let n = rows * cols;
      let buf = kernels::gpu_randn(n, seed as u32).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn zeros(ruby: &Ruby, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let data = vec![0.0f64; rows * cols];
      let buf = GpuBuffer::upload(&data).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn ones(ruby: &Ruby, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      let data = vec![1.0f64; rows * cols];
      let buf = GpuBuffer::upload(&data).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn broadcast_sub(ruby: &Ruby, x: &RubyGpuBuffer, v: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_broadcast_sub(&x.buf, &v.buf, x.len(), x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn broadcast_mul(ruby: &Ruby, x: &RubyGpuBuffer, v: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_broadcast_mul(&x.buf, &v.buf, x.len(), x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn broadcast_div(ruby: &Ruby, x: &RubyGpuBuffer, v: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_broadcast_div(&x.buf, &v.buf, x.len(), x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Softmax backward / log-softmax / cross-entropy ─────────────────────────

fn softmax_backward(ruby: &Ruby, grad: &RubyGpuBuffer, sm: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_softmax_backward(&grad.buf, &sm.buf, grad.rows, grad.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

fn log_softmax(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_log_softmax_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn cross_entropy(ruby: &Ruby, logits: &RubyGpuBuffer, targets: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_cross_entropy(&logits.buf, &targets.buf, logits.rows, logits.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, logits.rows, 1))
}

// ── Gather / Scatter ───────────────────────────────────────────────────────

fn gather_rows(ruby: &Ruby, table: &RubyGpuBuffer, indices: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = indices.rows;
      let cols = table.cols;
      let buf = kernels::gpu_gather_rows(&table.buf, &indices.buf, n, cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, cols))
}

fn scatter_add(_ruby: &Ruby, target: &RubyGpuBuffer, indices: &RubyGpuBuffer, src: &RubyGpuBuffer) -> Result<(), Error> {
      let n = indices.rows;
      let cols = target.cols;
      kernels::gpu_scatter_add(&target.buf, &indices.buf, &src.buf, n, cols);
      Ok(())
}

// ── Conv backward ──────────────────────────────────────────────────────────

fn col2im_1d(ruby: &Ruby, patches: &RubyGpuBuffer, n: usize, p: usize) -> Result<RubyGpuBuffer, Error> {
      let ks = patches.cols;
      let buf = kernels::gpu_col2im_1d(&patches.buf, n, p, ks).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, p))
}

fn col2im_2d(ruby: &Ruby, patches: &RubyGpuBuffer, shape: RArray, kernel: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      if sv.len() != 3 {
            return Err(Error::new(ruby.exception_runtime_error(), "col2im_2d: shape must be [c, h, w]"));
      }
      if kv.len() != 2 {
            return Err(Error::new(ruby.exception_runtime_error(), "col2im_2d: kernel must be [kh, kw]"));
      }
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let out_h = h - kh + 1;
      let out_w = w - kw + 1;
      let n = patches.rows / (out_h * out_w);
      let buf = kernels::gpu_col2im_2d(&patches.buf, n, c, h, w, kh, kw).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, c * h * w))
}

// ── Max pool 1D ────────────────────────────────────────────────────────────

fn max_pool_1d(ruby: &Ruby, x: &RubyGpuBuffer, out_len: usize, n_filters: usize) -> Result<RArray, Error> {
      let n = x.len() / (out_len * n_filters);
      let (vals, idx) = kernels::gpu_max_pool_1d(&x.buf, n, out_len, n_filters).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(vals, n, n_filters)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(idx, n, n_filters)).map_err(|e| e)?;
      Ok(arr)
}

fn max_pool_1d_backward(ruby: &Ruby, grad: &RubyGpuBuffer, indices: &RubyGpuBuffer, out_len: usize, n_filters: usize) -> Result<RubyGpuBuffer, Error> {
      let n = grad.rows;
      let buf = kernels::gpu_max_pool_1d_backward(&grad.buf, &indices.buf, n, out_len, n_filters).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * out_len, n_filters))
}

// ── Pool 2D ────────────────────────────────────────────────────────────────

fn avg_pool_2d(ruby: &Ruby, x: &RubyGpuBuffer, shape: RArray, kernel: RArray, stride: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      let stv: Vec<usize> = stride.to_vec()?;
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let (sh, sw) = (stv[0], stv[1]);
      let n = x.len() / (c * h * w);
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let buf = kernels::gpu_avg_pool_2d(&x.buf, n, c, h, w, kh, kw, sh, sw).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * c * out_h, out_w))
}

fn avg_pool_2d_backward(ruby: &Ruby, grad: &RubyGpuBuffer, shape: RArray, kernel: RArray, stride: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      let stv: Vec<usize> = stride.to_vec()?;
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let (sh, sw) = (stv[0], stv[1]);
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let n = grad.len() / (c * out_h * out_w);
      let buf = kernels::gpu_avg_pool_2d_backward(&grad.buf, n, c, h, w, kh, kw, sh, sw).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * c * h, w))
}

fn max_pool_2d(ruby: &Ruby, x: &RubyGpuBuffer, shape: RArray, kernel: RArray, stride: RArray) -> Result<RArray, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      let stv: Vec<usize> = stride.to_vec()?;
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let (sh, sw) = (stv[0], stv[1]);
      let n = x.len() / (c * h * w);
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let (vals, idx) = kernels::gpu_max_pool_2d(&x.buf, n, c, h, w, kh, kw, sh, sw).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(vals, n * c * out_h, out_w)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(idx, n * c * out_h, out_w)).map_err(|e| e)?;
      Ok(arr)
}

fn max_pool_2d_backward(ruby: &Ruby, grad: &RubyGpuBuffer, indices: &RubyGpuBuffer, shape: RArray, kernel: RArray, stride: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let kv: Vec<usize> = kernel.to_vec()?;
      let stv: Vec<usize> = stride.to_vec()?;
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (kh, kw) = (kv[0], kv[1]);
      let (sh, sw) = (stv[0], stv[1]);
      let out_h = (h - kh) / sh + 1;
      let out_w = (w - kw) / sw + 1;
      let _ = (kh, kw, sh, sw);
      let n = grad.len() / (c * out_h * out_w);
      let buf = kernels::gpu_max_pool_2d_backward(&grad.buf, &indices.buf, n, c, h, w, out_h, out_w).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * c * h, w))
}

// ── Reduce max/min ─────────────────────────────────────────────────────────

fn reduce_max_rows(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_max_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn reduce_max_cols(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_max_cols(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, 1, x.cols))
}

fn reduce_min_rows(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_min_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn reduce_min_cols(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_reduce_min_cols(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, 1, x.cols))
}

// ── Reshape (zero-copy) ───────────────────────────────────────────────────

fn reshape(ruby: &Ruby, x: &RubyGpuBuffer, rows: usize, cols: usize) -> Result<RubyGpuBuffer, Error> {
      if x.len() != rows * cols {
            return Err(Error::new(
                  ruby.exception_runtime_error(),
                  format!("reshape: len {} != {}*{}={}", x.len(), rows, cols, rows * cols),
            ));
      }
      Ok(RubyGpuBuffer { buf: Arc::clone(&x.buf), rows, cols })
}

// ── Linear (fused) ─────────────────────────────────────────────────────────

fn linear(ruby: &Ruby, x: &RubyGpuBuffer, w: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let m = x.rows;
      let k = x.cols;
      let n = w.cols;
      let buf = kernels::gpu_linear(&x.buf, &w.buf, &b.buf, m, n, k).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, m, n))
}

fn linear_backward(ruby: &Ruby, grad: &RubyGpuBuffer, input: &RubyGpuBuffer, weight: &RubyGpuBuffer) -> Result<RArray, Error> {
      let m = grad.rows;
      let n = grad.cols;
      let k = input.cols;
      let (gi, gw, gb) = kernels::gpu_linear_backward(&grad.buf, &input.buf, &weight.buf, m, n, k).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gi, m, k)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gw, k, n)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gb, 1, n)).map_err(|e| e)?;
      Ok(arr)
}

// ── Comparisons ────────────────────────────────────────────────────────────

fn gt(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_gt(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}
fn lt(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_lt(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}
fn eq_op(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_eq(&a.buf, &b.buf, a.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows, a.cols))
}
fn gt_scalar(ruby: &Ruby, x: &RubyGpuBuffer, val: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_gt_scalar(&x.buf, x.len(), val).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}
fn lt_scalar(ruby: &Ruby, x: &RubyGpuBuffer, val: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_lt_scalar(&x.buf, x.len(), val).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── GELU / SiLU ───────────────────────────────────────────────────────────

fn gelu(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_gelu(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}
fn gelu_backward(ruby: &Ruby, grad: &RubyGpuBuffer, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_gelu_backward(&grad.buf, &x.buf, grad.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}
fn silu(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_silu(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}
fn silu_backward(ruby: &Ruby, grad: &RubyGpuBuffer, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_silu_backward(&grad.buf, &x.buf, grad.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, grad.rows, grad.cols))
}

// ── BatchNorm ──────────────────────────────────────────────────────────────

fn batchnorm_forward(ruby: &Ruby, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, beta: &RubyGpuBuffer, eps: f64) -> Result<RArray, Error> {
      let n = x.rows;
      let c = x.cols;
      let (out, mean, inv_std) = kernels::gpu_batchnorm_forward(&x.buf, &gamma.buf, &beta.buf, n, c, eps).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(out, n, c)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(mean, 1, c)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(inv_std, 1, c)).map_err(|e| e)?;
      Ok(arr)
}

fn batchnorm_inference(ruby: &Ruby, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, beta: &RubyGpuBuffer, run_mean: &RubyGpuBuffer, run_var: &RubyGpuBuffer, eps: f64) -> Result<RubyGpuBuffer, Error> {
      let n = x.rows;
      let c = x.cols;
      let buf = kernels::gpu_batchnorm_inference(&x.buf, &gamma.buf, &beta.buf, &run_mean.buf, &run_var.buf, n, c, eps).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, c))
}

fn batchnorm_backward(ruby: &Ruby, grad_y: &RubyGpuBuffer, x: &RubyGpuBuffer, save_mean: &RubyGpuBuffer, save_inv_std: &RubyGpuBuffer, gamma: &RubyGpuBuffer) -> Result<RArray, Error> {
      let n = x.rows;
      let c = x.cols;
      let (gx, gg, gb) = kernels::gpu_batchnorm_backward(&grad_y.buf, &x.buf, &save_mean.buf, &save_inv_std.buf, &gamma.buf, n, c).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gx, n, c)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gg, 1, c)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gb, 1, c)).map_err(|e| e)?;
      Ok(arr)
}

// ── LayerNorm backward ────────────────────────────────────────────────────

fn layernorm_backward(ruby: &Ruby, grad_y: &RubyGpuBuffer, x: &RubyGpuBuffer, gamma: &RubyGpuBuffer, eps: f64) -> Result<RArray, Error> {
      let (gx, gg, gb) = kernels::gpu_layernorm_backward(&grad_y.buf, &x.buf, &gamma.buf, x.rows, x.cols, eps).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gx, x.rows, x.cols)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gg, 1, x.cols)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(gb, 1, x.cols)).map_err(|e| e)?;
      Ok(arr)
}

// ── Adam / AdamW ──────────────────────────────────────────────────────────

fn adam_update(_ruby: &Ruby, w: &RubyGpuBuffer, m: &RubyGpuBuffer, v: &RubyGpuBuffer, grad: &RubyGpuBuffer, lr: f64, beta1: f64, beta2: f64, eps: f64, t: usize) -> Result<(), Error> {
      kernels::gpu_adam_update(&w.buf, &m.buf, &v.buf, &grad.buf, lr, beta1, beta2, eps, t, w.len());
      Ok(())
}

fn adamw_update(_ruby: &Ruby, w: &RubyGpuBuffer, m: &RubyGpuBuffer, v: &RubyGpuBuffer, grad: &RubyGpuBuffer, lr: f64, beta1: f64, beta2: f64, eps: f64, wd: f64, t: usize) -> Result<(), Error> {
      kernels::gpu_adamw_update(&w.buf, &m.buf, &v.buf, &grad.buf, lr, beta1, beta2, eps, wd, t, w.len());
      Ok(())
}

// ── GRU ───────────────────────────────────────────────────────────────────

fn gru_cell(ruby: &Ruby, gates: &RubyGpuBuffer, h: &RubyGpuBuffer, hs: usize) -> Result<RubyGpuBuffer, Error> {
      let n = h.rows;
      let buf = kernels::gpu_gru_cell(&gates.buf, &h.buf, n, hs).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, hs))
}

// ── Structural ────────────────────────────────────────────────────────────

fn vconcat(ruby: &Ruby, a: &RubyGpuBuffer, b: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      if a.cols != b.cols {
            return Err(Error::new(ruby.exception_runtime_error(), format!("vconcat: cols mismatch {} vs {}", a.cols, b.cols)));
      }
      let buf = kernels::gpu_vconcat(&a.buf, &b.buf, a.len(), b.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, a.rows + b.rows, a.cols))
}

fn slice_cols(ruby: &Ruby, x: &RubyGpuBuffer, start: usize, count: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_slice_cols(&x.buf, x.rows, x.cols, start, count).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, count))
}

fn tril_mask(ruby: &Ruby, n: usize, fill_val: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_tril_mask(n, fill_val).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, n))
}

fn fill(ruby: &Ruby, rows: usize, cols: usize, val: f64) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_fill(rows * cols, val).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn repeat_rows(ruby: &Ruby, x: &RubyGpuBuffer, repeats: usize) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_repeat_rows(&x.buf, x.len(), repeats).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows * repeats, x.cols))
}

fn upsample_nearest_2d(ruby: &Ruby, x: &RubyGpuBuffer, shape: RArray, scale: RArray) -> Result<RubyGpuBuffer, Error> {
      let sv: Vec<usize> = shape.to_vec()?;
      let scv: Vec<usize> = scale.to_vec()?;
      let (c, h, w) = (sv[0], sv[1], sv[2]);
      let (sh, sw) = (scv[0], scv[1]);
      let n = x.len() / (c * h * w);
      let buf = kernels::gpu_upsample_nearest_2d(&x.buf, n, c, h, w, sh, sw).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n * c * h * sh, w * sw))
}

// ── Reductions ────────────────────────────────────────────────────────────

fn log_sum_exp(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_log_sum_exp_rows(&x.buf, x.rows, x.cols).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, 1))
}

fn grad_clip_norm(ruby: &Ruby, x: &RubyGpuBuffer, max_norm: f64) -> Result<(), Error> {
      kernels::gpu_grad_clip_norm(&x.buf, x.len(), max_norm).map_err(|e| hip_err(ruby, e))?;
      Ok(())
}

// ── Random (CPU-side, like randn) ─────────────────────────────────────────

fn rand_uniform(ruby: &Ruby, rows: usize, cols: usize, seed: u64) -> Result<RubyGpuBuffer, Error> {
      let n = rows * cols;
      let buf = kernels::gpu_rand_uniform(n, seed as u32).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

fn bernoulli(ruby: &Ruby, rows: usize, cols: usize, p: f64, seed: u64) -> Result<RubyGpuBuffer, Error> {
      let n = rows * cols;
      let buf = kernels::gpu_bernoulli(n, p, seed as u32).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, rows, cols))
}

// ── Prefix sum ────────────────────────────────────────────────────────────

fn prefix_sum_inclusive(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_prefix_sum_inclusive(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

fn prefix_sum_exclusive(ruby: &Ruby, x: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let buf = kernels::gpu_prefix_sum_exclusive(&x.buf, x.len()).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, x.rows, x.cols))
}

// ── Tree ──────────────────────────────────────────────────────────────────

fn histogram_build(ruby: &Ruby, bins: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, mask: &RubyGpuBuffer, n_bins: usize) -> Result<RArray, Error> {
      let n = bins.rows;
      let p = bins.cols;
      let (gh, hh, ch) = kernels::gpu_histogram_build(&bins.buf, &grad.buf, &hess.buf, &mask.buf, n, p, n_bins).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(gh, p, n_bins)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(hh, p, n_bins)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(ch, p, n_bins)).map_err(|e| e)?;
      Ok(arr)
}

fn split_eval(ruby: &Ruby, grad_hist: &RubyGpuBuffer, hess_hist: &RubyGpuBuffer, lambda: f64, min_child_weight: f64) -> Result<RArray, Error> {
      let p = grad_hist.rows;
      let n_bins = grad_hist.cols;
      let (bg, bb) = kernels::gpu_split_eval(&grad_hist.buf, &hess_hist.buf, p, n_bins, lambda, min_child_weight).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(bg, p, 1)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(bb, p, 1)).map_err(|e| e)?;
      Ok(arr)
}

fn data_partition(ruby: &Ruby, bins: &RubyGpuBuffer, node_mask: &RubyGpuBuffer, split_feature: usize, split_bin: usize) -> Result<RArray, Error> {
      let n = bins.rows;
      let p = bins.cols;
      let (left, right) = kernels::gpu_data_partition(&bins.buf, &node_mask.buf, n, p, split_feature, split_bin).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(left, n, 1)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(right, n, 1)).map_err(|e| e)?;
      Ok(arr)
}

fn tree_build(ruby: &Ruby, tr_bins: &RubyGpuBuffer, te_bins: &RubyGpuBuffer, grad: &RubyGpuBuffer, hess: &RubyGpuBuffer, n_bins: usize, max_depth: usize, lambda: f64, min_cw: f64) -> Result<RArray, Error> {
      let n_tr = tr_bins.rows;
      let n_te = te_bins.rows;
      let p = tr_bins.cols;
      let (tr_pred, te_pred) = kernels::gpu_tree_build(&tr_bins.buf, &te_bins.buf, &grad.buf, &hess.buf, n_tr, n_te, p, n_bins, max_depth, lambda, min_cw).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      arr.push(RubyGpuBuffer::new(tr_pred, n_tr, 1)).map_err(|e| e)?;
      arr.push(RubyGpuBuffer::new(te_pred, n_te, 1)).map_err(|e| e)?;
      Ok(arr)
}

fn grad(ruby: &Ruby, probs: &RubyGpuBuffer, targets: &RubyGpuBuffer, weights: &RubyGpuBuffer,
        k: usize) -> Result<RubyGpuBuffer, Error> {
      let n = probs.rows; let nc = probs.cols;
      let g = kernels::gpu_grad(&probs.buf, &targets.buf, &weights.buf, n, nc, k)
            .map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(g, n, 1))
}

fn hessian(ruby: &Ruby, probs: &RubyGpuBuffer, weights: &RubyGpuBuffer,
        k: usize) -> Result<RubyGpuBuffer, Error> {
      let n = probs.rows; let nc = probs.cols;
      let h = kernels::gpu_hessian(&probs.buf, &weights.buf, n, nc, k)
            .map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(h, n, 1))
}

/// add_col(matrix, k, col) → new matrix with column k updated
fn add_col(ruby: &Ruby, matrix: &RubyGpuBuffer, k: usize, col: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n = matrix.rows; let cols = matrix.cols;
      let buf = kernels::gpu_add_col(&matrix.buf, n, cols, k, &col.buf).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n, cols))
}

/// report(logits, val_targets_array, round) → Float (balanced accuracy)
fn report(ruby: &Ruby, logits: &RubyGpuBuffer, val_targets: Vec<i64>, round: usize) -> Result<f64, Error> {
      let n = logits.rows; let nc = logits.cols;
      let vt: Vec<i32> = val_targets.iter().map(|&v| v as i32).collect();
      kernels::gpu_report(&logits.buf, &vt, n, nc, round).map_err(|e| hip_err(ruby, e))
}

// ── DTW ───────────────────────────────────────────────────────────────────

fn dtw(ruby: &Ruby, cost: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let m = cost.rows;
      let n = cost.cols;
      let buf = kernels::gpu_dtw(&cost.buf, m, n).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, m + 1, n + 1))
}

// ── Apriori ───────────────────────────────────────────────────────────────

fn itemset_support(ruby: &Ruby, trans: &RubyGpuBuffer, cands: &RubyGpuBuffer) -> Result<RubyGpuBuffer, Error> {
      let n_trans = trans.rows;
      let n_items = trans.cols;
      let n_cands = cands.rows;
      let k = cands.cols;
      let buf = kernels::gpu_itemset_support(&trans.buf, &cands.buf, n_trans, n_items, n_cands, k).map_err(|e| hip_err(ruby, e))?;
      Ok(RubyGpuBuffer::new(buf, n_cands, 1))
}

fn candidate_generate(ruby: &Ruby, freq: &RubyGpuBuffer) -> Result<RArray, Error> {
      let n_freq = freq.rows;
      let k = freq.cols;
      let (buf, n_gen) = kernels::gpu_candidate_generate(&freq.buf, n_freq, k).map_err(|e| hip_err(ruby, e))?;
      let arr = ruby.ary_new();
      if n_gen > 0 {
            arr.push(RubyGpuBuffer::new(buf, n_gen, k + 1)).map_err(|e| e)?;
      } else {
            arr.push(RubyGpuBuffer::new(buf, 0, k + 1)).map_err(|e| e)?;
      }
      arr.push(ruby.integer_from_u64(n_gen as u64).as_value()).map_err(|e| e)?;
      Ok(arr)
}

// ── CatBoost bridge ────────────────────────────────────────────────────────

fn catboost_train(ruby: &Ruby, x: RArray, y: RArray, n: usize, p: usize, n_classes: usize,
      cat_features: RArray, params_hash: Value) -> Result<usize, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let y_vec: Vec<usize> = y.to_vec()?;
      let cat_vec: Vec<usize> = cat_features.to_vec()?;

      let mut params = catboost_rs::Params {
            cat_features: cat_vec,
            ..Default::default()
      };

      if let Ok(h) = magnus::RHash::try_convert(params_hash) {
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("iterations")) { params.iterations = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("depth")) { params.depth = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("lr")) { params.learning_rate = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("l2_reg")) { params.l2_reg = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("n_permutations")) { params.n_permutations = v; }
            if let Ok(v) = h.fetch::<_, u64>(ruby.to_symbol("seed")) { params.seed = v; }
      }

      let model = catboost_rs::train(&x_vec, &y_vec, n, p, n_classes, &params)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;

      let mut models = CATBOOST_MODELS.lock().unwrap();
      let id = models.len();
      models.push(model);
      Ok(id)
}

fn catboost_predict(ruby: &Ruby, model_id: usize, x: RArray, n: usize) -> Result<RArray, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let models = CATBOOST_MODELS.lock().unwrap();
      let model = models.get(model_id)
            .ok_or_else(|| Error::new(ruby.exception_runtime_error(), format!("no catboost model {model_id}")))?;
      let probs = catboost_rs::predict(model, &x_vec, n)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      let arr = ruby.ary_new();
      for v in probs { arr.push(v).map_err(|e| e)?; }
      Ok(arr)
}

// ── XGBoost bridge ─────────────────────────────────────────────────────────

fn xgb_train_multiclass(ruby: &Ruby, x: RArray, y: RArray, n: usize, p: usize, n_classes: usize, params_hash: Value) -> Result<usize, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let y_vec: Vec<usize> = y.to_vec()?;
      let mut params = xgboost_rs::Params {
            n_estimators: 100, max_depth: 6, learning_rate: 0.1, l2_reg: 1.0,
            min_child_weight: 1.0, subsample: 1.0, colsample_bytree: 1.0,
            n_bins: 256, seed: 42,
      };
      if let Ok(h) = magnus::RHash::try_convert(params_hash) {
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("n_estimators")) { params.n_estimators = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("max_depth")) { params.max_depth = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("lr")) { params.learning_rate = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("l2_reg")) { params.l2_reg = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("min_child_weight")) { params.min_child_weight = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("subsample")) { params.subsample = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("colsample_bytree")) { params.colsample_bytree = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("n_bins")) { params.n_bins = v; }
            if let Ok(v) = h.fetch::<_, u64>(ruby.to_symbol("seed")) { params.seed = v; }
      }
      let model = xgboost_rs::train_multiclass(&x_vec, &y_vec, n, p, n_classes, &params)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      let mut models = XGBOOST_MODELS.lock().unwrap();
      let id = models.len();
      models.push(model);
      Ok(id)
}

fn xgb_predict_proba(ruby: &Ruby, model_id: usize, x: RArray, n: usize) -> Result<RArray, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let models = XGBOOST_MODELS.lock().unwrap();
      let model = models.get(model_id)
            .ok_or_else(|| Error::new(ruby.exception_runtime_error(), format!("no xgb model {model_id}")))?;
      let probs = xgboost_rs::predict_proba(model, &x_vec, n)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      let arr = ruby.ary_new();
      for v in probs { arr.push(v).map_err(|e| e)?; }
      Ok(arr)
}

// ── LightGBM bridge ────────────────────────────────────────────────────────

fn lgbm_train_multiclass(ruby: &Ruby, x: RArray, y: RArray, n: usize, p: usize, n_classes: usize, params_hash: Value) -> Result<usize, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let y_vec: Vec<usize> = y.to_vec()?;
      let mut params = lightgbm_rs::Params {
            n_estimators: 100, num_leaves: 31, max_depth: 0, learning_rate: 0.1,
            l2_reg: 0.0, min_child_weight: 1e-3, min_gain_to_split: 0.0,
            n_bins: 256, goss_a: 0.0, goss_b: 0.0,
            use_efb: false, efb_max_conflict: 0.0, seed: 42,
      };
      if let Ok(h) = magnus::RHash::try_convert(params_hash) {
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("n_estimators")) { params.n_estimators = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("num_leaves")) { params.num_leaves = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("max_depth")) { params.max_depth = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("lr")) { params.learning_rate = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("l2_reg")) { params.l2_reg = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("min_child_weight")) { params.min_child_weight = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("min_gain_to_split")) { params.min_gain_to_split = v; }
            if let Ok(v) = h.fetch::<_, usize>(ruby.to_symbol("n_bins")) { params.n_bins = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("goss_a")) { params.goss_a = v; }
            if let Ok(v) = h.fetch::<_, f64>(ruby.to_symbol("goss_b")) { params.goss_b = v; }
            if let Ok(v) = h.fetch::<_, bool>(ruby.to_symbol("use_efb")) { params.use_efb = v; }
            if let Ok(v) = h.fetch::<_, u64>(ruby.to_symbol("seed")) { params.seed = v; }
      }
      let model = lightgbm_rs::train_multiclass(&x_vec, &y_vec, n, p, n_classes, &params)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      let mut models = LIGHTGBM_MODELS.lock().unwrap();
      let id = models.len();
      models.push(model);
      Ok(id)
}

fn lgbm_predict_proba(ruby: &Ruby, model_id: usize, x: RArray, n: usize) -> Result<RArray, Error> {
      let x_vec: Vec<f64> = x.to_vec()?;
      let models = LIGHTGBM_MODELS.lock().unwrap();
      let model = models.get(model_id)
            .ok_or_else(|| Error::new(ruby.exception_runtime_error(), format!("no lgbm model {model_id}")))?;
      let probs = lightgbm_rs::predict_proba(model, &x_vec, n)
            .map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      let arr = ruby.ary_new();
      for v in probs { arr.push(v).map_err(|e| e)?; }
      Ok(arr)
}

// ── Module init ─────────────────────────────────────────────────────────────

#[magnus::init(name = "nates_gpu")]
fn init(ruby: &Ruby) -> Result<(), Error> {
      gpu_core::memory::set_gc_hook(gc_hook);

      let module = ruby.define_module("NatesGpu")?;
      // Auto-include so functions are top-level (no `include NatesGpu` needed)
      ruby.define_global_const("NATES_GPU_LOADED", true)?;
      ruby.eval::<magnus::Value>("include NatesGpu").map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;

      let class = module.define_class("GpuBuffer", ruby.class_object())?;
      class.define_method("rows", method!(RubyGpuBuffer::rows, 0))?;
      class.define_method("cols", method!(RubyGpuBuffer::cols, 0))?;
      class.define_method("len", method!(RubyGpuBuffer::len, 0))?;
      class.define_method("to_s", method!(RubyGpuBuffer::to_s, 0))?;
      class.define_method("inspect", method!(RubyGpuBuffer::to_s, 0))?;
      class.define_method("ptr_addr", method!(RubyGpuBuffer::ptr_addr, 0))?;
      class.define_method("+", method!(RubyGpuBuffer::op_add, 1))?;
      class.define_method("-", method!(RubyGpuBuffer::op_sub, 1))?;
      class.define_method("*", method!(RubyGpuBuffer::op_mul, 1))?;
      class.define_method("/", method!(RubyGpuBuffer::op_div, 1))?;
      class.define_method("-@", method!(RubyGpuBuffer::op_neg, 0))?;

      // Data transfer
      module.define_module_function("upload", function!(upload, 3))?;
      module.define_module_function("upload_u8", function!(upload_u8, 3))?;
      module.define_module_function("upload_i32", function!(upload_i32, 3))?;
      module.define_module_function("upload_f32", function!(upload_f32, 3))?;
      module.define_module_function("zeros_u8", function!(zeros_u8, 1))?;
      module.define_module_function("zeros_f32", function!(zeros_f32, 2))?;
      module.define_module_function("download", function!(download, 1))?;
      module.define_module_function("download_f32", function!(download_f32, 1))?;

      // BLAS
      module.define_module_function("gemm", function!(gemm, 4))?;
      module.define_module_function("cholesky_solve", function!(cholesky_solve, 3))?;
      module.define_module_function("cholesky_inv", function!(cholesky_inv, 2))?;
      module.define_module_function("solve", function!(solve, 2))?;
      module.define_module_function("cholesky", function!(cholesky, 1))?;
      module.define_module_function("tri_solve", function!(tri_solve, 2))?;
      module.define_module_function("tri_solve_t", function!(tri_solve_t, 2))?;

      // Elementwise binary
      module.define_module_function("add", function!(add, 2))?;
      module.define_module_function("sub", function!(sub, 2))?;
      module.define_module_function("mul", function!(mul, 2))?;
      module.define_module_function("scale", function!(scale, 2))?;
      module.define_module_function("sub_scale", function!(sub_scale, 3))?;
      module.define_module_function("fma", function!(fma, 3))?;

      // In-place / scratch
      module.define_module_function("zero!", function!(zero_buf, 1))?;
      module.define_module_function("mul!", function!(mul_inplace, 2))?;
      module.define_module_function("add_inplace!", function!(add_inplace, 2))?;
      module.define_module_function("add_col!", function!(add_col_scaled_inplace, 4))?;
      // Oblivious tree helpers
      module.define_module_function("mse_grad_into!", function!(mse_grad_into, 3))?;
      module.define_module_function("softmax_ce_class_grad_f32!", function!(softmax_ce_class_grad_f32_into, 6))?;
      module.define_module_function("logloss_grad_f32!", function!(logloss_grad_f32_into, 4))?;
      module.define_module_function("argmax_f32_into!", function!(argmax_f32_into, 2))?;
      module.define_module_function("fill_f32!", function!(fill_f32_val, 2))?;
      module.define_module_function("write_split_into!", function!(write_split_into, 5))?;
      module.define_module_function("argmax_write_split_into!", function!(argmax_write_split_into, 6))?;
      module.define_module_function("gpu_sync", function!(gpu_sync, 0))?;
      module.define_module_function("download_i32_scalar", function!(download_i32_scalar, 1))?;

      // Oblivious tree
      module.define_module_function("oblivious_histogram_into!", function!(oblivious_histogram_into, 8))?;
      module.define_module_function("oblivious_route_step_into!", function!(oblivious_route_step_into, 6))?;
      module.define_module_function("oblivious_route_step_dev_into!", function!(oblivious_route_step_dev_into, 6))?;
      module.define_module_function("oblivious_route_full_into!", function!(oblivious_route_full_into, 5))?;
      module.define_module_function("scatter_add_by_leaf!", function!(scatter_add_by_leaf, 4))?;
      module.define_module_function("scatter_add_by_leaf_col!", function!(scatter_add_by_leaf_col, 6))?;
      module.define_module_function("leaf_reduce_into!", function!(leaf_reduce_into, 5))?;
      module.define_module_function("leaf_finalize_into!", function!(leaf_finalize_into, 4))?;
      module.define_module_function("oblivious_split_eval_into!", function!(oblivious_split_eval_into, 6))?;
      module.define_module_function("softmax_inplace_mc!", function!(softmax_inplace_mc, 2))?;
      module.define_module_function("logloss_grad_mc_into!", function!(logloss_grad_mc_into, 5))?;
      module.define_module_function("accuracy", function!(gpu_accuracy, 3))?;

      module.define_module_function("softmax_into!", function!(softmax_into, 2))?;
      module.define_module_function("bernoulli_into!", function!(bernoulli_into, 3))?;
      module.define_module_function("grad_hess_into!", function!(grad_hess_into, 7))?;
      module.define_module_function("tree_build_into!", function!(tree_build_into, 10))?;
      module.define_module_function("linear_into!", function!(linear_into, 4))?;
      module.define_module_function("layernorm_into!", function!(layernorm_into, 4))?;
      module.define_module_function("gelu_into!", function!(gelu_into, 2))?;
      module.define_module_function("gelu_backward_into!", function!(gelu_backward_into, 3))?;
      module.define_module_function("dropout_into!", function!(dropout_into, 4))?;
      module.define_module_function("dropout_u8_into!", function!(dropout_u8_into, 4))?;
      module.define_module_function("bernoulli_u8_into!", function!(bernoulli_u8_into, 3))?;
      module.define_module_function("rand_uniform_into!", function!(rand_uniform_into, 2))?;
      module.define_module_function("linear_backward_into!", function!(linear_backward_into, 4))?;
      module.define_module_function("layernorm_backward_into!", function!(layernorm_backward_into, 5))?;
      module.define_module_function("softmax_ce_grad_into!", function!(softmax_ce_grad_into, 5))?;
      module.define_module_function("linear_backward_weights_only", function!(linear_backward_weights_only, 2))?;
      module.define_module_function("linear_backward_weights_only_into!", function!(linear_backward_weights_only_into, 4))?;
      module.define_module_function("grad_clip_norm_scratch", function!(grad_clip_norm_scratch, 3))?;
      module.define_module_function("linear_backward_full_into!", function!(linear_backward_full_into, 6))?;
      module.define_module_function("layernorm_backward_full_into!", function!(layernorm_backward_full_into, 7))?;
      module.define_module_function("scale_inplace", function!(scale_inplace, 2))?;
      module.define_module_function("diag_add", function!(diag_add, 2))?;
      module.define_module_function("sgd_update", function!(sgd_update, 3))?;

      // Activations
      module.define_module_function("sigmoid", function!(sigmoid, 1))?;
      module.define_module_function("sigmoid_backward", function!(sigmoid_backward, 2))?;
      module.define_module_function("tanh_act", function!(tanh_act, 1))?;
      module.define_module_function("tanh_backward", function!(tanh_backward, 2))?;
      module.define_module_function("relu", function!(relu, 1))?;
      module.define_module_function("relu_backward", function!(relu_backward, 2))?;
      module.define_module_function("leaky_relu", function!(leaky_relu, 2))?;
      module.define_module_function("leaky_relu_backward", function!(leaky_relu_backward, 3))?;
      module.define_module_function("softmax", function!(softmax, 1))?;
      module.define_module_function("scaled_exp", function!(scaled_exp, 2))?;

      // Normalization
      module.define_module_function("layernorm", function!(layernorm, 1))?;
      module.define_module_function("layernorm_affine", function!(layernorm_affine, 3))?;
      module.define_module_function("dropout", function!(dropout, 3))?;

      // Reductions
      module.define_module_function("reduce_sum_cols", function!(reduce_sum_cols, 1))?;
      module.define_module_function("reduce_sum_rows", function!(reduce_sum_rows, 1))?;
      module.define_module_function("reduce_mean_cols", function!(reduce_mean_cols, 1))?;
      module.define_module_function("reduce_var_cols", function!(reduce_var_cols, 1))?;

      // Bias
      module.define_module_function("bias_add", function!(bias_add, 2))?;

      // Distance / Sorting
      module.define_module_function("pairwise_l2", function!(pairwise_l2, 2))?;
      module.define_module_function("argmin_rows", function!(argmin_rows, 1))?;
      module.define_module_function("argmax_rows", function!(argmax_rows, 1))?;
      module.define_module_function("topk_per_row", function!(topk_per_row, 2))?;
      module.define_module_function("partial_argsort", function!(partial_argsort, 2))?;

      // Convolution
      module.define_module_function("im2col_1d", function!(im2col_1d, 2))?;
      module.define_module_function("im2col_2d", function!(im2col_2d, 3))?;
      module.define_module_function("avg_pool_1d", function!(avg_pool_1d, 3))?;
      module.define_module_function("pool_grad_expand", function!(pool_grad_expand, 3))?;

      // Clustering
      module.define_module_function("centroid_update", function!(centroid_update, 4))?;
      module.define_module_function("gaussian_ll", function!(gaussian_ll, 5))?;

      // Sequence
      module.define_module_function("lstm_cell", function!(lstm_cell, 4))?;

      // VAE
      module.define_module_function("reparameterize", function!(reparameterize, 3))?;
      module.define_module_function("kl_div", function!(kl_div, 2))?;
      module.define_module_function("vae_backward_latent", function!(vae_backward_latent, 5))?;
      module.define_module_function("log_det_cholesky", function!(log_det_cholesky, 1))?;

      // Elementwise unary
      module.define_module_function("exp", function!(exp, 1))?;
      module.define_module_function("log", function!(log, 1))?;
      module.define_module_function("sqrt", function!(sqrt, 1))?;
      module.define_module_function("abs", function!(abs, 1))?;
      module.define_module_function("neg", function!(neg, 1))?;
      module.define_module_function("pow", function!(pow, 2))?;
      module.define_module_function("clamp", function!(clamp, 3))?;

      // Structural
      module.define_module_function("transpose", function!(transpose, 1))?;
      module.define_module_function("eye", function!(eye, 1))?;
      module.define_module_function("copy", function!(copy, 1))?;
      module.define_module_function("where_mask", function!(where_mask, 3))?;
      module.define_module_function("slice_rows", function!(slice_rows, 3))?;

      // Misc
      module.define_module_function("concat", function!(concat, 2))?;
      module.define_module_function("sign", function!(sign, 1))?;

      // Broadcast
      module.define_module_function("broadcast_sub", function!(broadcast_sub, 2))?;
      module.define_module_function("broadcast_mul", function!(broadcast_mul, 2))?;
      module.define_module_function("broadcast_div", function!(broadcast_div, 2))?;

      // Init helpers
      module.define_module_function("randn", function!(randn, 3))?;
      module.define_module_function("zeros", function!(zeros, 2))?;
      module.define_module_function("ones", function!(ones, 2))?;

      // Softmax backward / log-softmax / cross-entropy
      module.define_module_function("softmax_backward", function!(softmax_backward, 2))?;
      module.define_module_function("log_softmax", function!(log_softmax, 1))?;
      module.define_module_function("cross_entropy", function!(cross_entropy, 2))?;

      // Gather / scatter
      module.define_module_function("gather_rows", function!(gather_rows, 2))?;
      module.define_module_function("scatter_add", function!(scatter_add, 3))?;

      // Conv backward
      module.define_module_function("col2im_1d", function!(col2im_1d, 3))?;
      module.define_module_function("col2im_2d", function!(col2im_2d, 3))?;

      // Max pool 1D
      module.define_module_function("max_pool_1d", function!(max_pool_1d, 3))?;
      module.define_module_function("max_pool_1d_backward", function!(max_pool_1d_backward, 4))?;

      // Pool 2D
      module.define_module_function("avg_pool_2d", function!(avg_pool_2d, 4))?;
      module.define_module_function("avg_pool_2d_backward", function!(avg_pool_2d_backward, 4))?;
      module.define_module_function("max_pool_2d", function!(max_pool_2d, 4))?;
      module.define_module_function("max_pool_2d_backward", function!(max_pool_2d_backward, 5))?;

      // Reduce max/min
      module.define_module_function("reduce_max_rows", function!(reduce_max_rows, 1))?;
      module.define_module_function("reduce_max_cols", function!(reduce_max_cols, 1))?;
      module.define_module_function("reduce_min_rows", function!(reduce_min_rows, 1))?;
      module.define_module_function("reduce_min_cols", function!(reduce_min_cols, 1))?;

      // Reshape (zero-copy)
      module.define_module_function("reshape", function!(reshape, 3))?;

      // Linear (fused)
      module.define_module_function("linear", function!(linear, 3))?;
      module.define_module_function("linear_backward", function!(linear_backward, 3))?;

      // Comparisons
      module.define_module_function("gt", function!(gt, 2))?;
      module.define_module_function("lt", function!(lt, 2))?;
      module.define_module_function("eq_op", function!(eq_op, 2))?;
      module.define_module_function("gt_scalar", function!(gt_scalar, 2))?;
      module.define_module_function("lt_scalar", function!(lt_scalar, 2))?;

      // GELU / SiLU
      module.define_module_function("gelu", function!(gelu, 1))?;
      module.define_module_function("gelu_backward", function!(gelu_backward, 2))?;
      module.define_module_function("silu", function!(silu, 1))?;
      module.define_module_function("silu_backward", function!(silu_backward, 2))?;

      // BatchNorm
      module.define_module_function("batchnorm_forward", function!(batchnorm_forward, 4))?;
      module.define_module_function("batchnorm_inference", function!(batchnorm_inference, 6))?;
      module.define_module_function("batchnorm_backward", function!(batchnorm_backward, 5))?;

      // LayerNorm backward
      module.define_module_function("layernorm_backward", function!(layernorm_backward, 4))?;

      // Adam / AdamW
      module.define_module_function("adam_update", function!(adam_update, 9))?;
      module.define_module_function("adamw_update", function!(adamw_update, 10))?;

      // GRU
      module.define_module_function("gru_cell", function!(gru_cell, 3))?;

      // Structural
      module.define_module_function("vconcat", function!(vconcat, 2))?;
      module.define_module_function("slice_cols", function!(slice_cols, 3))?;
      module.define_module_function("tril_mask", function!(tril_mask, 2))?;
      module.define_module_function("fill", function!(fill, 3))?;
      module.define_module_function("repeat_rows", function!(repeat_rows, 2))?;
      module.define_module_function("upsample_nearest_2d", function!(upsample_nearest_2d, 3))?;

      // Reductions
      module.define_module_function("log_sum_exp", function!(log_sum_exp, 1))?;
      module.define_module_function("grad_clip_norm", function!(grad_clip_norm, 2))?;

      // Memory
      module.define_module_function("gpu_gc", function!(gpu_gc, 0))?;
      module.define_module_function("gpu_stats", function!(gpu_stats, 0))?;
      module.define_module_function("alloc_count_reset", function!(alloc_count_reset, 0))?;

      // Random
      module.define_module_function("rand_uniform", function!(rand_uniform, 3))?;
      module.define_module_function("bernoulli", function!(bernoulli, 4))?;

      // Prefix sum
      module.define_module_function("prefix_sum_inclusive", function!(prefix_sum_inclusive, 1))?;
      module.define_module_function("prefix_sum_exclusive", function!(prefix_sum_exclusive, 1))?;

      // Tree
      module.define_module_function("histogram_build", function!(histogram_build, 5))?;
      module.define_module_function("split_eval", function!(split_eval, 4))?;
      module.define_module_function("data_partition", function!(data_partition, 4))?;
      module.define_module_function("tree_build", function!(tree_build, 8))?;
      module.define_module_function("grad", function!(grad, 4))?;
      module.define_module_function("hessian", function!(hessian, 3))?;
      module.define_module_function("add_col", function!(add_col, 3))?;
      module.define_module_function("report", function!(report, 3))?;

      // DTW
      module.define_module_function("dtw", function!(dtw, 1))?;

      // Apriori
      module.define_module_function("itemset_support", function!(itemset_support, 2))?;
      module.define_module_function("candidate_generate", function!(candidate_generate, 1))?;

      // CatBoost
      module.define_module_function("catboost_train", function!(catboost_train, 7))?;
      module.define_module_function("catboost_predict", function!(catboost_predict, 3))?;
      module.define_module_function("xgb_train_multiclass", function!(xgb_train_multiclass, 6))?;
      module.define_module_function("xgb_predict_proba", function!(xgb_predict_proba, 3))?;
      module.define_module_function("lgbm_train_multiclass", function!(lgbm_train_multiclass, 6))?;
      module.define_module_function("lgbm_predict_proba", function!(lgbm_predict_proba, 3))?;

      // Named parameter wrappers — generated from .rbs signatures
      ruby.eval::<magnus::Value>(r#"
module NatesGpu
  class << self
    {upload: %w[data rows cols], download: %w[buf],
     gemm: %w[a b transA transB], solve: %w[a b], cholesky: %w[a],
     cholesky_solve: %w[a b n], cholesky_inv: %w[a n],
     tri_solve: %w[l b], tri_solve_t: %w[l b],
     add: %w[a b], sub: %w[a b], mul: %w[a b], scale: %w[x s],
     sub_scale: %w[a b s], fma: %w[x a b],
     scale_inplace: %w[x s], diag_add: %w[a val], sgd_update: %w[w grad lr],
     exp: %w[x], log: %w[x], sqrt: %w[x], abs: %w[x], neg: %w[x],
     pow: %w[x p], clamp: %w[x lo hi], sign: %w[x],
     sigmoid: %w[x], sigmoid_backward: %w[grad act],
     tanh_act: %w[x], tanh_backward: %w[grad act],
     relu: %w[x], relu_backward: %w[grad act],
     leaky_relu: %w[x alpha], leaky_relu_backward: %w[grad act alpha],
     softmax: %w[x], scaled_exp: %w[x s],
     layernorm: %w[x], dropout: %w[x mask p],
     reduce_sum_cols: %w[x], reduce_sum_rows: %w[x],
     reduce_mean_cols: %w[x], reduce_var_cols: %w[x],
     bias_add: %w[x b],
     pairwise_l2: %w[q t], argmin_rows: %w[x], argmax_rows: %w[x],
     topk_per_row: %w[x k], partial_argsort: %w[data k],
     im2col_1d: %w[x ks], im2col_2d: %w[x img_dims kernel_dims],
     avg_pool_1d: %w[x out_len n_filters], pool_grad_expand: %w[grad out_len n_filters],
     centroid_update: %w[x assignments dim k], gaussian_ll: %w[x means vars log_priors k],
     lstm_cell: %w[gates c h hs],
     reparameterize: %w[mu log_var eps], kl_div: %w[mu log_var],
     vae_backward_latent: %w[grad_z mu log_var eps kl_weight],
     log_det_cholesky: %w[l],
     transpose: %w[x], eye: %w[n], copy: %w[x],
     where_mask: %w[cond a b], slice_rows: %w[x start count],
     concat: %w[a b],
     broadcast_sub: %w[x v], broadcast_mul: %w[x v], broadcast_div: %w[x v],
     randn: %w[rows cols seed], zeros: %w[rows cols], ones: %w[rows cols],
     softmax_backward: %w[grad sm], log_softmax: %w[x], cross_entropy: %w[logits targets],
     gather_rows: %w[table indices], scatter_add: %w[target indices src],
     col2im_1d: %w[patches n p], col2im_2d: %w[patches shape kernel],
     max_pool_1d: %w[x out_len n_filters], max_pool_1d_backward: %w[grad indices out_len n_filters],
     avg_pool_2d: %w[x shape kernel stride], avg_pool_2d_backward: %w[grad shape kernel stride],
     max_pool_2d: %w[x shape kernel stride], max_pool_2d_backward: %w[grad indices shape kernel stride],
     reduce_max_rows: %w[x], reduce_max_cols: %w[x],
     reduce_min_rows: %w[x], reduce_min_cols: %w[x],
     reshape: %w[x rows cols],
     linear: %w[x w b], linear_backward: %w[grad input weight],
     gt: %w[a b], lt: %w[a b], eq_op: %w[a b],
     gt_scalar: %w[x val], lt_scalar: %w[x val],
     gelu: %w[x], gelu_backward: %w[grad x],
     silu: %w[x], silu_backward: %w[grad x],
     batchnorm_forward: %w[x gamma beta eps],
     batchnorm_inference: %w[x gamma beta run_mean run_var eps],
     batchnorm_backward: %w[grad_y x save_mean save_inv_std gamma],
     layernorm_backward: %w[grad_y x gamma eps],
     adam_update: %w[w m v grad lr beta1 beta2 eps t],
     adamw_update: %w[w m v grad lr beta1 beta2 eps wd t],
     gru_cell: %w[gates h hs],
     vconcat: %w[a b], slice_cols: %w[x start count],
     tril_mask: %w[n fill_val], fill: %w[rows cols val],
     repeat_rows: %w[x repeats],
     upsample_nearest_2d: %w[x shape scale],
     log_sum_exp: %w[x], grad_clip_norm: %w[x max_norm],
     rand_uniform: %w[rows cols seed], bernoulli: %w[rows cols p seed],
     prefix_sum_inclusive: %w[x], prefix_sum_exclusive: %w[x],
     histogram_build: %w[bins grad hess mask n_bins],
     split_eval: %w[grad_hist hess_hist lambda min_child_weight],
     data_partition: %w[bins node_mask split_feature split_bin],
     dtw: %w[cost],
     itemset_support: %w[trans cands],
     candidate_generate: %w[freq]
    }.each do |name, params|
      alias_method :"_#{name}", name
      param_list = params.join(", ")
      class_eval "def #{name}(#{param_list}); _#{name}(#{param_list}); end"
    end
  end
end
module NatesGpu
  def self.mean(x, axis: :x)
    raise "mean axis: :y not implemented" if axis == :y
    reduce_mean_cols(x)
  end
  def self.var(x, axis: :x)
    raise "var axis: :y not implemented" if axis == :y
    reduce_var_cols(x)
  end
  def self.sum(x, axis: :x)
    axis == :y ? reduce_sum_rows(x) : reduce_sum_cols(x)
  end
  def self.max(x, axis: :x)
    axis == :y ? reduce_max_rows(x) : reduce_max_cols(x)
  end
  def self.min(x, axis: :x)
    axis == :y ? reduce_min_rows(x) : reduce_min_cols(x)
  end
  def self.slice(x, start, count, axis: :x)
    axis == :y ? slice_rows(x, start, count) : slice_cols(x, start, count)
  end
end
def mean(x, axis: :x); NatesGpu.mean(x, axis: axis); end
def var(x, axis: :x); NatesGpu.var(x, axis: axis); end
def sum(x, axis: :x); NatesGpu.sum(x, axis: axis); end
def max(x, axis: :x); NatesGpu.max(x, axis: axis); end
def min(x, axis: :x); NatesGpu.min(x, axis: axis); end
def slice(x, start, count, axis: :x); NatesGpu.slice(x, start, count, axis: axis); end
      "#).map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;
      ruby.eval::<magnus::Value>("include NatesGpu").map_err(|e| Error::new(ruby.exception_runtime_error(), format!("{e}")))?;

      Ok(())
}

use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use mlua::prelude::*;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::Mutex;

static CATBOOST_MODELS: Mutex<Vec<catboost_rs::Model>> = Mutex::new(Vec::new());

#[derive(Clone)]
pub struct LuaGpuBuffer {
      pub buf: Arc<GpuBuffer>,
      pub rows: usize,
      pub cols: usize,
}

impl LuaGpuBuffer {
      pub fn new(buf: GpuBuffer, rows: usize, cols: usize) -> Self {
            Self {
                  buf: Arc::new(buf),
                  rows,
                  cols,
            }
      }

      pub fn len(&self) -> usize {
            self.rows * self.cols
      }
}

impl LuaUserData for LuaGpuBuffer {
      fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
            methods.add_method("rows", |_, this, ()| Ok(this.rows));
            methods.add_method("cols", |_, this, ()| Ok(this.cols));
            methods.add_method("len", |_, this, ()| Ok(this.len()));
      }
}

impl FromLua for LuaGpuBuffer {
      fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
            match value {
                  LuaValue::UserData(ud) => ud.borrow::<Self>().map(|b| b.clone()),
                  _ => Err(LuaError::runtime("expected LuaGpuBuffer")),
            }
      }
}

pub fn register_types(_lua: &Lua) -> LuaResult<()> {
      Ok(())
}

pub fn register_upload_download(lua: &Lua) -> LuaResult<()> {
      let g = lua.globals();

      let upload_fn = lua.create_function(|_, (data, rows, cols): (Vec<f64>, usize, usize)| {
            if data.len() != rows * cols {
                  return Err(LuaError::runtime(format!(
                        "upload: data len {} != rows*cols {}*{}={}",
                        data.len(),
                        rows,
                        cols,
                        rows * cols
                  )));
            }
            let buf = GpuBuffer::upload(&data).map_err(|e| LuaError::runtime(e.to_string()))?;
            Ok(LuaGpuBuffer::new(buf, rows, cols))
      })?;
      g.set("upload", upload_fn)?;

      let download_fn = lua.create_function(|_, buf: LuaGpuBuffer| {
            let mut dst = vec![0.0f64; buf.len()];
            buf.buf
                  .download(&mut dst)
                  .map_err(|e| LuaError::runtime(e.to_string()))?;
            Ok(dst)
      })?;
      g.set("download", download_fn)?;

      g.set(
            "upload_u8",
            lua.create_function(|_, (data, rows, cols): (Vec<i64>, usize, usize)| {
                  let v: Vec<u8> = data.iter().map(|&x| x as u8).collect();
                  let buf = GpuBuffer::upload_u8(&v).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "upload_i32",
            lua.create_function(|_, (data, rows, cols): (Vec<i64>, usize, usize)| {
                  let v: Vec<i32> = data.iter().map(|&x| x as i32).collect();
                  let bytes = v.len() * 4;
                  let buf = GpuBuffer::alloc_bytes(bytes).map_err(hip_err)?;
                  gpu_core::hip::check(unsafe {
                        gpu_core::hip::hipMemcpy(
                              buf.ptr_raw(),
                              v.as_ptr() as *const c_void,
                              bytes,
                              gpu_core::hip::HIP_MEMCPY_H2D,
                        )
                  })
                  .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "upload_f32",
            lua.create_function(|_, (data, rows, cols): (Vec<f64>, usize, usize)| {
                  let v: Vec<f32> = data.iter().map(|&x| x as f32).collect();
                  let buf = GpuBuffer::upload_f32(&v).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "zeros_u8",
            lua.create_function(|_, n: usize| {
                  let buf = GpuBuffer::zeros_bytes(n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, 1))
            })?,
      )?;

      g.set(
            "zeros_f32",
            lua.create_function(|_, (rows, cols): (usize, usize)| {
                  let buf = GpuBuffer::zeros_bytes(rows * cols * 4).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "download_f32",
            lua.create_function(|lua, x: LuaGpuBuffer| {
                  let n = x.buf.len() / 4;
                  let mut dst = vec![0.0f32; n];
                  gpu_core::hip::check(unsafe {
                        gpu_core::hip::hipMemcpy(
                              dst.as_mut_ptr() as *mut c_void,
                              x.buf.ptr_raw(),
                              x.buf.len(),
                              gpu_core::hip::HIP_MEMCPY_D2H,
                        )
                  })
                  .map_err(hip_err)?;
                  let arr = lua.create_table()?;
                  for (idx, v) in dst.iter().enumerate() {
                        arr.raw_set(idx + 1, *v as f64)?;
                  }
                  Ok(arr)
            })?,
      )?;

      g.set(
            "download_i32_scalar",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let mut v = [0i32; 1];
                  gpu_core::hip::check(unsafe {
                        gpu_core::hip::hipMemcpy(
                              v.as_mut_ptr() as *mut c_void,
                              x.buf.ptr_raw(),
                              4,
                              gpu_core::hip::HIP_MEMCPY_D2H,
                        )
                  })
                  .map_err(hip_err)?;
                  Ok(v[0] as i64)
            })?,
      )?;

      Ok(())
}

/// Map a HipError to mlua::Error
fn hip_err(e: gpu_core::hip::HipError) -> LuaError {
      LuaError::runtime(format!("{e}"))
}

pub fn register_composites(lua: &Lua) -> LuaResult<()> {
      let g = lua.globals();

      // ── BLAS ──────────────────────────────────────────────────────────────

      // gemm(A, B, tA, tB) — tA/tB are "N" or "T"
      g.set(
            "gemm",
            lua.create_function(
                  |_, (a, b, ta, tb): (LuaGpuBuffer, LuaGpuBuffer, String, String)| {
                        let (m, n, k, buf) = match (ta.as_str(), tb.as_str()) {
                              ("N", "N") => {
                                    // A[m,k] @ B[k,n] → [m,n]
                                    (
                                          a.rows,
                                          b.cols,
                                          a.cols,
                                          kernels::gpu_gemm(&a.buf, &b.buf, a.rows, b.cols, a.cols)
                                                .map_err(hip_err)?,
                                    )
                              }
                              ("T", "N") => {
                                    // A^T[m,k] where A is [k,m] @ B[k,n] → [m,n]
                                    (
                                          a.cols,
                                          b.cols,
                                          a.rows,
                                          kernels::gpu_gemm_at(&a.buf, &b.buf, a.cols, b.cols, a.rows)
                                                .map_err(hip_err)?,
                                    )
                              }
                              ("N", "T") => {
                                    // A[m,k] @ B^T[k,n] where B is [n,k] → [m,n]
                                    (
                                          a.rows,
                                          b.rows,
                                          a.cols,
                                          kernels::gpu_gemm_bt(&a.buf, &b.buf, a.rows, b.rows, a.cols)
                                                .map_err(hip_err)?,
                                    )
                              }
                              _ => {
                                    return Err(LuaError::runtime(format!(
                                          "gemm: invalid transpose flags ({ta}, {tb}), expected N or T"
                                    )));
                              }
                        };
                        Ok(LuaGpuBuffer::new(buf, m, n))
                  },
            )?,
      )?;

      // cholesky_solve(A, b, n) — GPU-resident, no CPU round-trip
      g.set(
            "cholesky_solve",
            lua.create_function(|_, (a, b, n): (LuaGpuBuffer, LuaGpuBuffer, usize)| {
                  let buf = kernels::gpu_cholesky_solve(&a.buf, &b.buf, n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, 1))
            })?,
      )?;

      // cholesky_inv(A, n) — GPU-resident, no CPU round-trip
      g.set(
            "cholesky_inv",
            lua.create_function(|_, (a, n): (LuaGpuBuffer, usize)| {
                  let buf = kernels::gpu_cholesky_inv(&a.buf, n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, n))
            })?,
      )?;

      // solve(A, b) — general LU solve, A*X = B
      g.set(
            "solve",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let n = a.rows;
                  let nrhs = b.cols;
                  let buf = kernels::gpu_solve(&a.buf, &b.buf, n, nrhs).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, nrhs))
            })?,
      )?;

      // cholesky(A) — standalone Cholesky factorization, returns L
      g.set(
            "cholesky",
            lua.create_function(|_, a: LuaGpuBuffer| {
                  let n = a.rows;
                  let buf = kernels::gpu_cholesky(&a.buf, n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, n))
            })?,
      )?;

      // tri_solve(L, b) — triangular solve L*X = B
      g.set(
            "tri_solve",
            lua.create_function(|_, (l, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let n = l.rows;
                  let nrhs = b.cols;
                  let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, false).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, nrhs))
            })?,
      )?;

      // tri_solve_t(L, b) — triangular solve L^T*X = B
      g.set(
            "tri_solve_t",
            lua.create_function(|_, (l, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let n = l.rows;
                  let nrhs = b.cols;
                  let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, true).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, nrhs))
            })?,
      )?;

      // ── Elementwise binary ────────────────────────────────────────────────

      g.set(
            "add",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_add(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "sub",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_sub(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "mul",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_mul(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "scale",
            lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_scale(&x.buf, s, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "sub_scale",
            lua.create_function(|_, (a, b, s): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_sub_scale(&a.buf, &b.buf, a.len(), s).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "fma",
            lua.create_function(|_, (x, a, b): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_fma(&x.buf, &a.buf, &b.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── In-place ──────────────────────────────────────────────────────────

      g.set(
            "scale_inplace",
            lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
                  kernels::gpu_scale_inplace(&x.buf, s, x.len());
                  Ok(x)
            })?,
      )?;

      g.set(
            "diag_add",
            lua.create_function(|_, (a, val): (LuaGpuBuffer, f64)| {
                  kernels::gpu_add_diag(&a.buf, a.rows, val);
                  Ok(a)
            })?,
      )?;

      g.set(
            "sgd_update",
            lua.create_function(|_, (w, grad, lr): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
                  kernels::gpu_sgd_update(&w.buf, &grad.buf, lr, w.len());
                  Ok(w)
            })?,
      )?;

      // ── Activations ───────────────────────────────────────────────────────

      g.set(
            "sigmoid",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_sigmoid(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "sigmoid_backward",
            lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_sigmoid_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "tanh_act",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_tanh(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "tanh_backward",
            lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_tanh_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "relu",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_relu(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "relu_backward",
            lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_relu_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "leaky_relu",
            lua.create_function(|_, (x, alpha): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_leaky_relu(&x.buf, x.len(), alpha).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "leaky_relu_backward",
            lua.create_function(|_, (grad, act, alpha): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_leaky_relu_backward(&grad.buf, &act.buf, grad.len(), alpha)
                        .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "softmax",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_softmax_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "scaled_exp",
            lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_scaled_exp(&x.buf, x.len(), s).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── Normalization ─────────────────────────────────────────────────────

      g.set(
            "layernorm",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf =
                        kernels::gpu_layernorm(&x.buf, x.rows, x.cols, None, None).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "dropout",
            lua.create_function(|_, (x, mask, p): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_dropout(&x.buf, &mask.buf, x.len(), p).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── Reductions ────────────────────────────────────────────────────────

      g.set(
            "reduce_sum_cols",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_sum_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, 1, x.cols))
            })?,
      )?;

      g.set(
            "reduce_sum_rows",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_sum_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "reduce_mean_cols",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_mean_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, 1, x.cols))
            })?,
      )?;

      g.set(
            "reduce_var_cols",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_var_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, 1, x.cols))
            })?,
      )?;

      // ── Bias ──────────────────────────────────────────────────────────────

      g.set(
            "bias_add",
            lua.create_function(|_, (x, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_bias_add(&x.buf, &b.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── Distance / Sorting ────────────────────────────────────────────────

      g.set(
            "pairwise_l2",
            lua.create_function(|_, (q, t): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_pairwise_l2(&q.buf, &t.buf, q.rows, t.rows, q.cols)
                        .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, q.rows, t.rows))
            })?,
      )?;

      g.set(
            "argmin_rows",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_argmin_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "argmax_rows",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_argmax_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "topk_per_row",
            lua.create_function(|_, (x, k): (LuaGpuBuffer, usize)| {
                  let buf = kernels::gpu_topk_per_row(&x.buf, x.rows, x.cols, k).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, k))
            })?,
      )?;

      g.set(
            "partial_argsort",
            lua.create_function(|_, (data, k): (LuaGpuBuffer, usize)| {
                  let buf = kernels::gpu_partial_argsort(&data.buf, data.len(), k).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, k, 1))
            })?,
      )?;

      // ── Convolution ───────────────────────────────────────────────────────

      g.set(
            "im2col_1d",
            lua.create_function(|_, (x, ks): (LuaGpuBuffer, usize)| {
                  let n = x.rows;
                  let p = x.cols;
                  let out_len = p - ks + 1;
                  let buf = kernels::gpu_im2col_1d(&x.buf, n, p, ks).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n * out_len, ks))
            })?,
      )?;

      g.set(
            "im2col_2d",
            lua.create_function(
                  |_, (x, c, h, w, kh, kw): (LuaGpuBuffer, usize, usize, usize, usize, usize)| {
                        let n = x.len() / (c * h * w);
                        let out_h = h - kh + 1;
                        let out_w = w - kw + 1;
                        let buf = kernels::gpu_im2col_2d(&x.buf, n, c, h, w, kh, kw).map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * out_h * out_w, c * kh * kw))
                  },
            )?,
      )?;

      g.set(
            "avg_pool_1d",
            lua.create_function(|_, (x, out_len, n_filters): (LuaGpuBuffer, usize, usize)| {
                  let n = x.len() / (out_len * n_filters);
                  let buf = kernels::gpu_avg_pool_1d(&x.buf, n, out_len, n_filters).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, n_filters))
            })?,
      )?;

      g.set(
            "pool_grad_expand",
            lua.create_function(
                  |_, (grad, out_len, n_filters): (LuaGpuBuffer, usize, usize)| {
                        let n = grad.len() / n_filters;
                        let buf = kernels::gpu_pool_grad_expand(&grad.buf, n, out_len, n_filters)
                              .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * out_len, n_filters))
                  },
            )?,
      )?;

      // ── Clustering ────────────────────────────────────────────────────────

      // centroid_update returns {centroids, counts} table
      g.set(
            "centroid_update",
            lua.create_function(
                  |lua, (x, assignments, dim, k): (LuaGpuBuffer, LuaGpuBuffer, usize, usize)| {
                        let n = x.rows;
                        let (centroids, counts) =
                              kernels::gpu_centroid_update(&x.buf, &assignments.buf, n, dim, k)
                                    .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(centroids, k, dim))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(counts, k, 1))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "gaussian_ll",
            lua.create_function(
                  |_,
                  (x, means, vars, log_priors, k): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                  )| {
                        let n = x.rows;
                        let p = x.cols;
                        let buf = kernels::gpu_gaussian_ll(
                              &x.buf,
                              &means.buf,
                              &vars.buf,
                              &log_priors.buf,
                              n,
                              k,
                              p,
                        )
                        .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n, k))
                  },
            )?,
      )?;

      // ── Sequence ──────────────────────────────────────────────────────────

      g.set(
            "lstm_cell",
            lua.create_function(
                  |_, (gates, c, h, hs): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, usize)| {
                        let n = c.rows;
                        kernels::gpu_lstm_cell(&gates.buf, &c.buf, &h.buf, n, hs);
                        Ok((c, h))
                  },
            )?,
      )?;

      // ── VAE ───────────────────────────────────────────────────────────────

      g.set(
            "reparameterize",
            lua.create_function(
                  |_, (mu, log_var, eps): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        let buf = kernels::gpu_reparameterize(&mu.buf, &log_var.buf, &eps.buf, mu.len())
                              .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, mu.rows, mu.cols))
                  },
            )?,
      )?;

      g.set(
            "kl_div",
            lua.create_function(|_, (mu, log_var): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_kl_div(&mu.buf, &log_var.buf, mu.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, mu.rows, mu.cols))
            })?,
      )?;

      // vae_backward_latent returns {grad_mu, grad_lv} table
      g.set(
            "vae_backward_latent",
            lua.create_function(
                  |lua,
                  (grad_z, mu, log_var, eps, kl_weight): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                  )| {
                        let n = mu.len();
                        let (grad_mu, grad_lv) = kernels::gpu_vae_backward_latent(
                              &grad_z.buf,
                              &mu.buf,
                              &log_var.buf,
                              &eps.buf,
                              n,
                              kl_weight,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(grad_mu, mu.rows, mu.cols))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(grad_lv, mu.rows, mu.cols))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "log_det_cholesky",
            lua.create_function(|_, l: LuaGpuBuffer| {
                  let val = kernels::gpu_log_det_cholesky(&l.buf, l.rows).map_err(hip_err)?;
                  Ok(val)
            })?,
      )?;

      // ── Misc ──────────────────────────────────────────────────────────────

      g.set(
            "concat",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_concat(&a.buf, &b.buf, a.rows, a.cols, b.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols + b.cols))
            })?,
      )?;

      // sign — no GPU kernel, download/signum/upload fallback
      g.set(
            "sign",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let mut data = vec![0.0f64; x.len()];
                  x.buf.download(&mut data).map_err(hip_err)?;
                  for v in &mut data {
                        *v = v.signum();
                  }
                  let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── Init helpers ──────────────────────────────────────────────────────

      g.set(
            "randn",
            lua.create_function(|_, (rows, cols, seed): (usize, usize, u64)| {
                  let n = rows * cols;
                  let buf = kernels::gpu_randn(n, seed as u32).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "zeros",
            lua.create_function(|_, (rows, cols): (usize, usize)| {
                  let data = vec![0.0f64; rows * cols];
                  let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "ones",
            lua.create_function(|_, (rows, cols): (usize, usize)| {
                  let data = vec![1.0f64; rows * cols];
                  let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      // ── Elementwise unary/parameterized ─────────────────────────────────

      g.set(
            "exp",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_exp(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "log",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_log(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "sqrt",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_sqrt(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "abs",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_abs(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "neg",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_neg(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "pow",
            lua.create_function(|_, (x, p): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_pow(&x.buf, x.len(), p).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "clamp",
            lua.create_function(|_, (x, lo, hi): (LuaGpuBuffer, f64, f64)| {
                  let buf = kernels::gpu_clamp(&x.buf, x.len(), lo, hi).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      // ── Structural ────────────────────────────────────────────────────────

      g.set(
            "transpose",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_transpose(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.cols, x.rows))
            })?,
      )?;

      g.set(
            "eye",
            lua.create_function(|_, n: usize| {
                  let buf = kernels::gpu_eye(n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, n))
            })?,
      )?;

      g.set(
            "copy",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_copy(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "where_mask",
            lua.create_function(
                  |_, (cond, a, b): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        let buf =
                              kernels::gpu_where_mask(&cond.buf, &a.buf, &b.buf, a.len()).map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
                  },
            )?,
      )?;

      g.set(
            "slice_rows",
            lua.create_function(|_, (x, start, count): (LuaGpuBuffer, usize, usize)| {
                  let buf = kernels::gpu_slice_rows(&x.buf, start, count, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, count, x.cols))
            })?,
      )?;

      g.set(
            "broadcast_sub",
            lua.create_function(|_, (x, v): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_broadcast_sub(&x.buf, &v.buf, x.len(), x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "broadcast_mul",
            lua.create_function(|_, (x, v): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_broadcast_mul(&x.buf, &v.buf, x.len(), x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "broadcast_div",
            lua.create_function(|_, (x, v): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_broadcast_div(&x.buf, &v.buf, x.len(), x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "layernorm_affine",
            lua.create_function(
                  |_, (x, gamma, beta): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        let buf = kernels::gpu_layernorm(
                              &x.buf,
                              x.rows,
                              x.cols,
                              Some(&gamma.buf),
                              Some(&beta.buf),
                        )
                        .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
                  },
            )?,
      )?;

      g.set(
            "softmax_backward",
            lua.create_function(|_, (grad, sm): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_softmax_backward(&grad.buf, &sm.buf, grad.rows, grad.cols)
                        .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "log_softmax",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_log_softmax_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "cross_entropy",
            lua.create_function(|_, (logits, targets): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf =
                        kernels::gpu_cross_entropy(&logits.buf, &targets.buf, logits.rows, logits.cols)
                              .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, logits.rows, 1))
            })?,
      )?;

      g.set(
            "gather_rows",
            lua.create_function(|_, (table, indices): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let n = indices.rows;
                  let cols = table.cols;
                  let buf =
                        kernels::gpu_gather_rows(&table.buf, &indices.buf, n, cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, cols))
            })?,
      )?;

      g.set(
            "scatter_add",
            lua.create_function(
                  |_, (target, indices, src): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        let n = indices.rows;
                        let cols = target.cols;
                        kernels::gpu_scatter_add(&target.buf, &indices.buf, &src.buf, n, cols);
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "col2im_1d",
            lua.create_function(|_, (patches, n, p): (LuaGpuBuffer, usize, usize)| {
                  let ks = patches.cols;
                  let buf = kernels::gpu_col2im_1d(&patches.buf, n, p, ks).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, p))
            })?,
      )?;

      g.set(
            "col2im_2d",
            lua.create_function(
                  |_, (patches, shape, kernel): (LuaGpuBuffer, Vec<usize>, Vec<usize>)| {
                        if shape.len() != 3 {
                              return Err(LuaError::runtime("col2im_2d: shape must be [c,h,w]"));
                        }
                        if kernel.len() != 2 {
                              return Err(LuaError::runtime("col2im_2d: kernel must be [kh,kw]"));
                        }
                        let (c, h, w) = (shape[0], shape[1], shape[2]);
                        let (kh, kw) = (kernel[0], kernel[1]);
                        let out_h = h - kh + 1;
                        let out_w = w - kw + 1;
                        let n = patches.rows / (out_h * out_w);
                        let buf =
                              kernels::gpu_col2im_2d(&patches.buf, n, c, h, w, kh, kw).map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n, c * h * w))
                  },
            )?,
      )?;

      g.set(
            "max_pool_1d",
            lua.create_function(
                  |lua, (x, out_len, n_filters): (LuaGpuBuffer, usize, usize)| {
                        let n = x.len() / (out_len * n_filters);
                        let (vals, idx) =
                              kernels::gpu_max_pool_1d(&x.buf, n, out_len, n_filters).map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(vals, n, n_filters))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(idx, n, n_filters))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "max_pool_1d_backward",
            lua.create_function(
                  |_, (grad, indices, out_len, n_filters): (LuaGpuBuffer, LuaGpuBuffer, usize, usize)| {
                        let n = grad.rows;
                        let buf = kernels::gpu_max_pool_1d_backward(
                              &grad.buf,
                              &indices.buf,
                              n,
                              out_len,
                              n_filters,
                        )
                        .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * out_len, n_filters))
                  },
            )?,
      )?;

      g.set(
            "avg_pool_2d",
            lua.create_function(
                  |_, (x, shape, kernel, stride): (LuaGpuBuffer, Vec<usize>, Vec<usize>, Vec<usize>)| {
                        let (c, h, w) = (shape[0], shape[1], shape[2]);
                        let (kh, kw) = (kernel[0], kernel[1]);
                        let (sh, sw) = (stride[0], stride[1]);
                        let n = x.len() / (c * h * w);
                        let out_h = (h - kh) / sh + 1;
                        let out_w = (w - kw) / sw + 1;
                        let buf = kernels::gpu_avg_pool_2d(&x.buf, n, c, h, w, kh, kw, sh, sw)
                              .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * c * out_h, out_w))
                  },
            )?,
      )?;

      g.set("avg_pool_2d_backward", lua.create_function(|_, (grad, shape, kernel, stride): (LuaGpuBuffer, Vec<usize>, Vec<usize>, Vec<usize>)| {
                  let (c, h, w) = (shape[0], shape[1], shape[2]);
                  let (kh, kw) = (kernel[0], kernel[1]);
                  let (sh, sw) = (stride[0], stride[1]);
                  let out_h = (h - kh) / sh + 1;
                  let out_w = (w - kw) / sw + 1;
                  let n = grad.len() / (c * out_h * out_w);
                  let buf = kernels::gpu_avg_pool_2d_backward(&grad.buf, n, c, h, w, kh, kw, sh, sw).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n * c * h, w))
         })?)?;

      g.set("max_pool_2d", lua.create_function(|lua, (x, shape, kernel, stride): (LuaGpuBuffer, Vec<usize>, Vec<usize>, Vec<usize>)| {
                  let (c, h, w) = (shape[0], shape[1], shape[2]);
                  let (kh, kw) = (kernel[0], kernel[1]);
                  let (sh, sw) = (stride[0], stride[1]);
                  let n = x.len() / (c * h * w);
                  let out_h = (h - kh) / sh + 1;
                  let out_w = (w - kw) / sw + 1;
                  let (vals, idx) = kernels::gpu_max_pool_2d(&x.buf, n, c, h, w, kh, kw, sh, sw).map_err(hip_err)?;
                  let tbl = lua.create_table()?;
                  tbl.raw_set(1, LuaGpuBuffer::new(vals, n * c * out_h, out_w))?;
                  tbl.raw_set(2, LuaGpuBuffer::new(idx, n * c * out_h, out_w))?;
                  Ok(tbl)
         })?)?;

      g.set(
            "max_pool_2d_backward",
            lua.create_function(
                  |_,
                  (grad, indices, shape, kernel, stride): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        Vec<usize>,
                        Vec<usize>,
                        Vec<usize>,
                  )| {
                        let (c, h, w) = (shape[0], shape[1], shape[2]);
                        let (kh, kw) = (kernel[0], kernel[1]);
                        let (sh, sw) = (stride[0], stride[1]);
                        let out_h = (h - kh) / sh + 1;
                        let out_w = (w - kw) / sw + 1;
                        let n = grad.len() / (c * out_h * out_w);
                        let buf = kernels::gpu_max_pool_2d_backward(
                              &grad.buf,
                              &indices.buf,
                              n,
                              c,
                              h,
                              w,
                              out_h,
                              out_w,
                        )
                        .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * c * h, w))
                  },
            )?,
      )?;

      g.set(
            "reduce_max_rows",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_max_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "reduce_max_cols",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_max_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, 1, x.cols))
            })?,
      )?;

      g.set(
            "reduce_min_rows",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_min_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "reduce_min_cols",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_reduce_min_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, 1, x.cols))
            })?,
      )?;

      g.set(
            "reshape",
            lua.create_function(|_, (x, rows, cols): (LuaGpuBuffer, usize, usize)| {
                  if x.len() != rows * cols {
                        return Err(LuaError::runtime(format!(
                              "reshape: len {} != {}*{}={}",
                              x.len(),
                              rows,
                              cols,
                              rows * cols
                        )));
                  }
                  Ok(LuaGpuBuffer {
                        buf: Arc::clone(&x.buf),
                        rows,
                        cols,
                  })
            })?,
      )?;

      g.set(
            "mean",
            lua.create_function(|_, (x, axis): (LuaGpuBuffer, Option<String>)| {
                  match axis.as_deref() {
                        Some("y") => Err(LuaError::runtime("mean axis: y not implemented")),
                        _ => {
                              let buf =
                                    kernels::gpu_reduce_mean_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, 1, x.cols))
                        }
                  }
            })?,
      )?;

      g.set(
            "var",
            lua.create_function(|_, (x, axis): (LuaGpuBuffer, Option<String>)| {
                  match axis.as_deref() {
                        Some("y") => Err(LuaError::runtime("var axis: y not implemented")),
                        _ => {
                              let buf =
                                    kernels::gpu_reduce_var_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, 1, x.cols))
                        }
                  }
            })?,
      )?;

      g.set(
            "sum",
            lua.create_function(|_, (x, axis): (LuaGpuBuffer, Option<String>)| {
                  match axis.as_deref() {
                        Some("y") => {
                              let buf =
                                    kernels::gpu_reduce_sum_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, x.rows, 1))
                        }
                        _ => {
                              let buf =
                                    kernels::gpu_reduce_sum_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, 1, x.cols))
                        }
                  }
            })?,
      )?;

      g.set(
            "max",
            lua.create_function(|_, (x, axis): (LuaGpuBuffer, Option<String>)| {
                  match axis.as_deref() {
                        Some("y") => {
                              let buf =
                                    kernels::gpu_reduce_max_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, x.rows, 1))
                        }
                        _ => {
                              let buf =
                                    kernels::gpu_reduce_max_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, 1, x.cols))
                        }
                  }
            })?,
      )?;

      g.set(
            "min",
            lua.create_function(|_, (x, axis): (LuaGpuBuffer, Option<String>)| {
                  match axis.as_deref() {
                        Some("y") => {
                              let buf =
                                    kernels::gpu_reduce_min_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, x.rows, 1))
                        }
                        _ => {
                              let buf =
                                    kernels::gpu_reduce_min_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, 1, x.cols))
                        }
                  }
            })?,
      )?;

      g.set(
            "slice",
            lua.create_function(
                  |_, (x, start, count, axis): (LuaGpuBuffer, usize, usize, Option<String>)| match axis
                        .as_deref()
                  {
                        Some("y") => {
                              let buf =
                                    kernels::gpu_slice_rows(&x.buf, start, count, x.cols).map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, count, x.cols))
                        }
                        _ => {
                              let buf = kernels::gpu_slice_cols(&x.buf, x.rows, x.cols, start, count)
                                    .map_err(hip_err)?;
                              Ok(LuaGpuBuffer::new(buf, x.rows, count))
                        }
                  },
            )?,
      )?;

      g.set(
            "gpu_gc",
            lua.create_function(|_, ()| {
                  gpu_core::hip::device_synchronize().map_err(hip_err)?;
                  Ok(())
            })?,
      )?;

      g.set(
            "gpu_stats",
            lua.create_function(|lua, ()| {
                  let (free, total) = gpu_core::hip::mem_info().unwrap_or((0, 0));
                  let tbl = lua.create_table()?;
                  tbl.raw_set(1, free / 1_048_576)?;
                  tbl.raw_set(2, total / 1_048_576)?;
                  Ok(tbl)
            })?,
      )?;

      g.set(
            "alloc_count_reset",
            lua.create_function(|_, ()| Ok(gpu_core::memory::alloc_count_reset()))?,
      )?;

      g.set(
            "gpu_sync",
            lua.create_function(|_, ()| {
                  gpu_core::hip::check(unsafe { gpu_core::hip::hipDeviceSynchronize() }).ok();
                  Ok(0.0f64)
            })?,
      )?;

      g.set(
            "zero!",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  gpu_core::hip::check(unsafe {
                        gpu_core::hip::hipMemset(x.buf.ptr_raw(), 0, x.buf.len())
                  })
                  .map_err(hip_err)?;
                  Ok(())
            })?,
      )?;

      g.set(
            "mul!",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  kernels::gpu_mul_inplace(&a.buf, &b.buf, a.len());
                  Ok(())
            })?,
      )?;

      g.set(
            "add_inplace!",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  kernels::gpu_add_inplace(&a.buf, &b.buf, a.len());
                  Ok(())
            })?,
      )?;

      g.set(
            "add_col!",
            lua.create_function(
                  |_, (matrix, k, col, scale): (LuaGpuBuffer, usize, LuaGpuBuffer, f64)| {
                        kernels::gpu_add_col_scaled_inplace(
                              &matrix.buf,
                              matrix.rows,
                              matrix.cols,
                              k,
                              &col.buf,
                              scale,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "linear",
            lua.create_function(|_, (x, w, b): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                  let m = x.rows;
                  let k = x.cols;
                  let n = w.cols;
                  let buf = kernels::gpu_linear(&x.buf, &w.buf, &b.buf, m, n, k).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, m, n))
            })?,
      )?;

      g.set(
            "linear_backward",
            lua.create_function(
                  |lua, (grad, input, weight): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        let m = grad.rows;
                        let n = grad.cols;
                        let k = input.cols;
                        let (gi, gw, gb) =
                              kernels::gpu_linear_backward(&grad.buf, &input.buf, &weight.buf, m, n, k)
                                    .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gi, m, k))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(gw, k, n))?;
                        tbl.raw_set(3, LuaGpuBuffer::new(gb, 1, n))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "gt",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_gt(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "lt",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_lt(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "eq_op",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_eq(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
            })?,
      )?;

      g.set(
            "gt_scalar",
            lua.create_function(|_, (x, val): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_gt_scalar(&x.buf, x.len(), val).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "lt_scalar",
            lua.create_function(|_, (x, val): (LuaGpuBuffer, f64)| {
                  let buf = kernels::gpu_lt_scalar(&x.buf, x.len(), val).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "gelu",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_gelu(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "gelu_backward",
            lua.create_function(|_, (grad, x): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_gelu_backward(&grad.buf, &x.buf, grad.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "silu",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_silu(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "silu_backward",
            lua.create_function(|_, (grad, x): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let buf = kernels::gpu_silu_backward(&grad.buf, &x.buf, grad.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
            })?,
      )?;

      g.set(
            "batchnorm_forward",
            lua.create_function(
                  |lua, (x, gamma, beta, eps): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, f64)| {
                        let n = x.rows;
                        let c = x.cols;
                        let (out, mean, inv_std) =
                              kernels::gpu_batchnorm_forward(&x.buf, &gamma.buf, &beta.buf, n, c, eps)
                                    .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(out, n, c))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(mean, 1, c))?;
                        tbl.raw_set(3, LuaGpuBuffer::new(inv_std, 1, c))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "batchnorm_inference",
            lua.create_function(
                  |_,
                  (x, gamma, beta, run_mean, run_var, eps): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                  )| {
                        let n = x.rows;
                        let c = x.cols;
                        let buf = kernels::gpu_batchnorm_inference(
                              &x.buf,
                              &gamma.buf,
                              &beta.buf,
                              &run_mean.buf,
                              &run_var.buf,
                              n,
                              c,
                              eps,
                        )
                        .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n, c))
                  },
            )?,
      )?;

      g.set(
            "batchnorm_backward",
            lua.create_function(
                  |lua,
                  (grad_y, x, save_mean, save_inv_std, gamma): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        let n = x.rows;
                        let c = x.cols;
                        let (gx, gg, gb) = kernels::gpu_batchnorm_backward(
                              &grad_y.buf,
                              &x.buf,
                              &save_mean.buf,
                              &save_inv_std.buf,
                              &gamma.buf,
                              n,
                              c,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gx, n, c))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(gg, 1, c))?;
                        tbl.raw_set(3, LuaGpuBuffer::new(gb, 1, c))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "layernorm_backward",
            lua.create_function(
                  |lua, (grad_y, x, gamma, eps): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, f64)| {
                        let (gx, gg, gb) = kernels::gpu_layernorm_backward(
                              &grad_y.buf,
                              &x.buf,
                              &gamma.buf,
                              x.rows,
                              x.cols,
                              eps,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gx, x.rows, x.cols))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(gg, 1, x.cols))?;
                        tbl.raw_set(3, LuaGpuBuffer::new(gb, 1, x.cols))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "adam_update",
            lua.create_function(
                  |_,
                  (w, m, v, grad, lr, beta1, beta2, eps, t): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                        f64,
                        f64,
                        f64,
                        usize,
                  )| {
                        kernels::gpu_adam_update(
                              &w.buf,
                              &m.buf,
                              &v.buf,
                              &grad.buf,
                              lr,
                              beta1,
                              beta2,
                              eps,
                              t,
                              w.len(),
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "adamw_update",
            lua.create_function(
                  |_,
                  (w, m, v, grad, lr, beta1, beta2, eps, wd, t): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                        f64,
                        f64,
                        f64,
                        f64,
                        usize,
                  )| {
                        kernels::gpu_adamw_update(
                              &w.buf,
                              &m.buf,
                              &v.buf,
                              &grad.buf,
                              lr,
                              beta1,
                              beta2,
                              eps,
                              wd,
                              t,
                              w.len(),
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "gru_cell",
            lua.create_function(|_, (gates, h, hs): (LuaGpuBuffer, LuaGpuBuffer, usize)| {
                  let n = h.rows;
                  let buf = kernels::gpu_gru_cell(&gates.buf, &h.buf, n, hs).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, hs))
            })?,
      )?;

      g.set(
            "vconcat",
            lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
                  if a.cols != b.cols {
                        return Err(LuaError::runtime(format!(
                              "vconcat: cols mismatch {} vs {}",
                              a.cols, b.cols
                        )));
                  }
                  let buf = kernels::gpu_vconcat(&a.buf, &b.buf, a.len(), b.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, a.rows + b.rows, a.cols))
            })?,
      )?;

      g.set(
            "slice_cols",
            lua.create_function(|_, (x, start, count): (LuaGpuBuffer, usize, usize)| {
                  let buf =
                        kernels::gpu_slice_cols(&x.buf, x.rows, x.cols, start, count).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, count))
            })?,
      )?;

      g.set(
            "tril_mask",
            lua.create_function(|_, (n, fill_val): (usize, f64)| {
                  let buf = kernels::gpu_tril_mask(n, fill_val).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, n))
            })?,
      )?;

      g.set(
            "fill",
            lua.create_function(|_, (rows, cols, val): (usize, usize, f64)| {
                  let buf = kernels::gpu_fill(rows * cols, val).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "repeat_rows",
            lua.create_function(|_, (x, repeats): (LuaGpuBuffer, usize)| {
                  let buf = kernels::gpu_repeat_rows(&x.buf, x.len(), repeats).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows * repeats, x.cols))
            })?,
      )?;

      g.set(
            "upsample_nearest_2d",
            lua.create_function(
                  |_, (x, shape, scale): (LuaGpuBuffer, Vec<usize>, Vec<usize>)| {
                        let (c, h, w) = (shape[0], shape[1], shape[2]);
                        let (sh, sw) = (scale[0], scale[1]);
                        let n = x.len() / (c * h * w);
                        let buf = kernels::gpu_upsample_nearest_2d(&x.buf, n, c, h, w, sh, sw)
                              .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(buf, n * c * h * sh, w * sw))
                  },
            )?,
      )?;

      g.set(
            "log_sum_exp",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_log_sum_exp_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, 1))
            })?,
      )?;

      g.set(
            "grad_clip_norm",
            lua.create_function(|_, (x, max_norm): (LuaGpuBuffer, f64)| {
                  kernels::gpu_grad_clip_norm(&x.buf, x.len(), max_norm).map_err(hip_err)?;
                  Ok(())
            })?,
      )?;

      g.set(
            "grad_clip_norm_scratch",
            lua.create_function(|_, (x, max_norm, tmp): (LuaGpuBuffer, f64, LuaGpuBuffer)| {
                  kernels::gpu_grad_clip_norm_with_tmp(&x.buf, &tmp.buf, x.len(), max_norm);
                  Ok(())
            })?,
      )?;

      g.set(
            "rand_uniform",
            lua.create_function(|_, (rows, cols, seed): (usize, usize, u64)| {
                  let n = rows * cols;
                  let buf = kernels::gpu_rand_uniform(n, seed as u32).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "bernoulli",
            lua.create_function(|_, (rows, cols, p, seed): (usize, usize, f64, u64)| {
                  let n = rows * cols;
                  let buf = kernels::gpu_bernoulli(n, p, seed as u32).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, rows, cols))
            })?,
      )?;

      g.set(
            "prefix_sum_inclusive",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_prefix_sum_inclusive(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "prefix_sum_exclusive",
            lua.create_function(|_, x: LuaGpuBuffer| {
                  let buf = kernels::gpu_prefix_sum_exclusive(&x.buf, x.len()).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
            })?,
      )?;

      g.set(
            "histogram_build",
            lua.create_function(
                  |lua,
                  (bins, grad, hess, mask, n_bins): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                  )| {
                        let n = bins.rows;
                        let p = bins.cols;
                        let (gh, hh, ch) = kernels::gpu_histogram_build(
                              &bins.buf, &grad.buf, &hess.buf, &mask.buf, n, p, n_bins,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gh, p, n_bins))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(hh, p, n_bins))?;
                        tbl.raw_set(3, LuaGpuBuffer::new(ch, p, n_bins))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "split_eval",
            lua.create_function(
                  |lua,
                  (grad_hist, hess_hist, lambda, min_child_weight): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                        f64,
                  )| {
                        let p = grad_hist.rows;
                        let n_bins = grad_hist.cols;
                        let (bg, bb) = kernels::gpu_split_eval(
                              &grad_hist.buf,
                              &hess_hist.buf,
                              p,
                              n_bins,
                              lambda,
                              min_child_weight,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(bg, p, 1))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(bb, p, 1))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "data_partition",
            lua.create_function(
                  |lua,
                  (bins, node_mask, split_feature, split_bin): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                  )| {
                        let n = bins.rows;
                        let p = bins.cols;
                        let (left, right) = kernels::gpu_data_partition(
                              &bins.buf,
                              &node_mask.buf,
                              n,
                              p,
                              split_feature,
                              split_bin,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(left, n, 1))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(right, n, 1))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "tree_build",
            lua.create_function(
                  |lua,
                  (tr_bins, te_bins, grad, hess, n_bins, max_depth, lambda, min_cw): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                        f64,
                        f64,
                  )| {
                        let n_tr = tr_bins.rows;
                        let n_te = te_bins.rows;
                        let p = tr_bins.cols;
                        let (tr_pred, te_pred) = kernels::gpu_tree_build(
                              &tr_bins.buf,
                              &te_bins.buf,
                              &grad.buf,
                              &hess.buf,
                              n_tr,
                              n_te,
                              p,
                              n_bins,
                              max_depth,
                              lambda,
                              min_cw,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(tr_pred, n_tr, 1))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(te_pred, n_te, 1))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "grad",
            lua.create_function(
                  |_, (probs, targets, weights, k): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, usize)| {
                        let n = probs.rows;
                        let nc = probs.cols;
                        let gbuf = kernels::gpu_grad(&probs.buf, &targets.buf, &weights.buf, n, nc, k)
                              .map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(gbuf, n, 1))
                  },
            )?,
      )?;

      g.set(
            "hessian",
            lua.create_function(
                  |_, (probs, weights, k): (LuaGpuBuffer, LuaGpuBuffer, usize)| {
                        let n = probs.rows;
                        let nc = probs.cols;
                        let hbuf =
                              kernels::gpu_hessian(&probs.buf, &weights.buf, n, nc, k).map_err(hip_err)?;
                        Ok(LuaGpuBuffer::new(hbuf, n, 1))
                  },
            )?,
      )?;

      g.set(
            "add_col",
            lua.create_function(|_, (matrix, k, col): (LuaGpuBuffer, usize, LuaGpuBuffer)| {
                  let n = matrix.rows;
                  let cols = matrix.cols;
                  let buf = kernels::gpu_add_col(&matrix.buf, n, cols, k, &col.buf).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n, cols))
            })?,
      )?;

      g.set(
            "report",
            lua.create_function(
                  |_, (logits, val_targets, round): (LuaGpuBuffer, Vec<i64>, usize)| {
                        let n = logits.rows;
                        let nc = logits.cols;
                        let vt: Vec<i32> = val_targets.iter().map(|&v| v as i32).collect();
                        let score = kernels::gpu_report(&logits.buf, &vt, n, nc, round).map_err(hip_err)?;
                        Ok(score)
                  },
            )?,
      )?;

      g.set(
            "dtw",
            lua.create_function(|_, cost: LuaGpuBuffer| {
                  let m = cost.rows;
                  let n = cost.cols;
                  let buf = kernels::gpu_dtw(&cost.buf, m, n).map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, m + 1, n + 1))
            })?,
      )?;

      g.set(
            "itemset_support",
            lua.create_function(|_, (trans, cands): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let n_trans = trans.rows;
                  let n_items = trans.cols;
                  let n_cands = cands.rows;
                  let k = cands.cols;
                  let buf =
                        kernels::gpu_itemset_support(&trans.buf, &cands.buf, n_trans, n_items, n_cands, k)
                              .map_err(hip_err)?;
                  Ok(LuaGpuBuffer::new(buf, n_cands, 1))
            })?,
      )?;

      g.set(
            "candidate_generate",
            lua.create_function(|lua, freq: LuaGpuBuffer| {
                  let n_freq = freq.rows;
                  let k = freq.cols;
                  let (buf, n_gen) =
                        kernels::gpu_candidate_generate(&freq.buf, n_freq, k).map_err(hip_err)?;
                  let tbl = lua.create_table()?;
                  tbl.raw_set(1, LuaGpuBuffer::new(buf, n_gen, k + 1))?;
                  tbl.raw_set(2, n_gen)?;
                  Ok(tbl)
            })?,
      )?;

      g.set(
            "linear_into!",
            lua.create_function(
                  |_, (x, w, b, out): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        kernels::gpu_linear_into(&x.buf, &w.buf, &b.buf, &out.buf, x.rows, w.cols, x.cols);
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "layernorm_into!",
            lua.create_function(
                  |_, (x, gamma, beta, out): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        kernels::gpu_layernorm_into(
                              &x.buf,
                              &out.buf,
                              Some(&gamma.buf),
                              Some(&beta.buf),
                              x.rows,
                              x.cols,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "gelu_into!",
            lua.create_function(|_, (x, out): (LuaGpuBuffer, LuaGpuBuffer)| {
                  kernels::gpu_gelu_into(&x.buf, &out.buf, x.len());
                  Ok(())
            })?,
      )?;

      g.set(
            "gelu_backward_into!",
            lua.create_function(
                  |_, (grad, x, out): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        kernels::gpu_gelu_backward_into(&grad.buf, &x.buf, &out.buf, grad.len());
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "dropout_into!",
            lua.create_function(
                  |_, (x, mask, p, out): (LuaGpuBuffer, LuaGpuBuffer, f64, LuaGpuBuffer)| {
                        kernels::gpu_dropout_into(&x.buf, &mask.buf, &out.buf, x.len(), p);
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "rand_uniform_into!",
            lua.create_function(|_, (out, seed): (LuaGpuBuffer, u32)| {
                  kernels::gpu_rand_uniform_into(&out.buf, out.len(), seed);
                  Ok(())
            })?,
      )?;

      g.set(
            "linear_backward_into!",
            lua.create_function(
                  |lua,
                  (grad, input, weight, grad_input): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        let (gw, gb) = kernels::gpu_linear_backward_into(
                              &grad.buf,
                              &input.buf,
                              &weight.buf,
                              &grad_input.buf,
                              grad.rows,
                              grad.cols,
                              input.cols,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gw, input.cols, grad.cols))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(gb, 1, grad.cols))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "layernorm_backward_into!",
            lua.create_function(
                  |lua,
                  (grad, x, gamma, eps, grad_x): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                        LuaGpuBuffer,
                  )| {
                        let (gg, gb) = kernels::gpu_layernorm_backward_into(
                              &grad.buf,
                              &x.buf,
                              &gamma.buf,
                              &grad_x.buf,
                              grad.rows,
                              grad.cols,
                              eps,
                        )
                        .map_err(hip_err)?;
                        let tbl = lua.create_table()?;
                        tbl.raw_set(1, LuaGpuBuffer::new(gg, 1, grad.cols))?;
                        tbl.raw_set(2, LuaGpuBuffer::new(gb, 1, grad.cols))?;
                        Ok(tbl)
                  },
            )?,
      )?;

      g.set(
            "softmax_ce_grad_into!",
            lua.create_function(
                  |_,
                  (logits, targets, weights, grad_out, scale): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                  )| {
                        kernels::gpu_softmax_ce_grad_into(
                              &logits.buf,
                              &targets.buf,
                              &weights.buf,
                              &grad_out.buf,
                              logits.rows,
                              logits.cols,
                              scale,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "linear_backward_weights_only",
            lua.create_function(|lua, (grad, input): (LuaGpuBuffer, LuaGpuBuffer)| {
                  let (gw, gb) = kernels::gpu_linear_backward_weights_only(
                        &grad.buf, &input.buf, grad.rows, grad.cols, input.cols,
                  )
                  .map_err(hip_err)?;
                  let tbl = lua.create_table()?;
                  tbl.raw_set(1, LuaGpuBuffer::new(gw, input.cols, grad.cols))?;
                  tbl.raw_set(2, LuaGpuBuffer::new(gb, 1, grad.cols))?;
                  Ok(tbl)
            })?,
      )?;

      g.set(
            "linear_backward_weights_only_into!",
            lua.create_function(
                  |_,
                  (grad, input, grad_w, grad_b): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        kernels::gpu_linear_backward_weights_only_into(
                              &grad.buf,
                              &input.buf,
                              &grad_w.buf,
                              &grad_b.buf,
                              grad.rows,
                              grad.cols,
                              input.cols,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "linear_backward_full_into!",
            lua.create_function(
                  |_,
                  (grad, input, weight, grad_input, grad_w, grad_b): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        kernels::gpu_linear_backward_full_into(
                              &grad.buf,
                              &input.buf,
                              &weight.buf,
                              &grad_input.buf,
                              &grad_w.buf,
                              &grad_b.buf,
                              grad.rows,
                              grad.cols,
                              input.cols,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "layernorm_backward_full_into!",
            lua.create_function(
                  |_,
                  (grad, x, gamma, eps, grad_x, grad_gamma, grad_beta): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        kernels::gpu_layernorm_backward_full_into(
                              &grad.buf,
                              &x.buf,
                              &gamma.buf,
                              &grad_x.buf,
                              &grad_gamma.buf,
                              &grad_beta.buf,
                              grad.rows,
                              grad.cols,
                              eps,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "bernoulli_into!",
            lua.create_function(|_, (out, p, seed): (LuaGpuBuffer, f64, u32)| {
                  kernels::gpu_bernoulli_into(&out.buf, out.len(), p, seed);
                  Ok(())
            })?,
      )?;

      g.set(
            "grad_hess_into!",
            lua.create_function(
                  |_,
                  (probs, targets, weights, mask, grad_out, hess_out, k): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                  )| {
                        kernels::gpu_grad_hess_into(
                              &probs.buf,
                              &targets.buf,
                              &weights.buf,
                              &mask.buf,
                              &grad_out.buf,
                              &hess_out.buf,
                              probs.rows,
                              probs.cols,
                              k,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "tree_build_into!",
            lua.create_function(
                  |_,
                  (tr_bins, te_bins, gbuf, hbuf, n_bins, depth, lambda, mcw, tr_pred, te_pred): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                        f64,
                        f64,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        kernels::gpu_tree_build_into(
                              &tr_bins.buf,
                              &te_bins.buf,
                              &gbuf.buf,
                              &hbuf.buf,
                              tr_bins.rows,
                              te_bins.rows,
                              tr_bins.cols,
                              n_bins,
                              depth,
                              lambda,
                              mcw,
                              &tr_pred.buf,
                              &te_pred.buf,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "mse_grad_into!",
            lua.create_function(
                  |_, (pred, target, grad): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
                        kernels::gpu_mse_grad_into(&pred.buf, &target.buf, &grad.buf, pred.len());
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "argmax_f32_into!",
            lua.create_function(|_, (data, out): (LuaGpuBuffer, LuaGpuBuffer)| {
                  kernels::gpu_argmax_f32(&data.buf, &out.buf, data.len());
                  Ok(())
            })?,
      )?;

      g.set(
            "fill_f32!",
            lua.create_function(|_, (out, val): (LuaGpuBuffer, f64)| {
                  kernels::gpu_fill_f32(&out.buf, val as f32, out.len());
                  Ok(())
            })?,
      )?;

      g.set(
            "write_split_into!",
            lua.create_function(
                  |_, (sf, sb, feat, bin, d): (LuaGpuBuffer, LuaGpuBuffer, usize, usize, usize)| {
                        kernels::gpu_write_split(&sf.buf, &sb.buf, feat, bin as u8, d);
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "argmax_write_split_into!",
            lua.create_function(
                  |_,
                  (gain, sf, sb, best_idx, n_bins, d): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                  )| {
                        let n_features = gain.len() / n_bins;
                        kernels::gpu_argmax_write_split(
                              &gain.buf,
                              &sf.buf,
                              &sb.buf,
                              &best_idx.buf,
                              n_features,
                              n_bins,
                              d,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "oblivious_histogram_into!",
            lua.create_function(
                  |_,
                  (bins_fm, node_idx, grad, hess, grad_hist, hess_hist, n_bins, n_nodes): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                  )| {
                        let n_rows = grad.len();
                        let n_features = bins_fm.len() / n_rows;
                        kernels::gpu_oblivious_histogram(
                              &bins_fm.buf,
                              &node_idx.buf,
                              &grad.buf,
                              &hess.buf,
                              &grad_hist.buf,
                              &hess_hist.buf,
                              n_rows,
                              n_features,
                              n_bins,
                              n_nodes,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "oblivious_route_step_into!",
            lua.create_function(
                  |_,
                  (bins_rm, node_in, node_out, split_feat, split_bin, depth): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                        usize,
                  )| {
                        let n_rows = node_in.len();
                        let n_features = bins_rm.len() / n_rows;
                        kernels::gpu_oblivious_route_step(
                              &bins_rm.buf,
                              &node_in.buf,
                              &node_out.buf,
                              split_feat,
                              split_bin as u8,
                              depth,
                              n_rows,
                              n_features,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "oblivious_route_full_into!",
            lua.create_function(
                  |_,
                  (bins_rm, split_feat, split_bin, leaf_idx, depth): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                  )| {
                        let n_rows = leaf_idx.len();
                        let n_features = bins_rm.len() / n_rows;
                        kernels::gpu_oblivious_route_full(
                              &bins_rm.buf,
                              &split_feat.buf,
                              &split_bin.buf,
                              &leaf_idx.buf,
                              n_rows,
                              n_features,
                              depth,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set("scatter_add_by_leaf!", lua.create_function(|_, (pred, leaf_idx, leaf_value, lr): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, f64)| {
                  kernels::gpu_scatter_add_by_leaf(&pred.buf, &leaf_idx.buf, &leaf_value.buf, lr as f32, pred.len());
                  Ok(())
         })?)?;

      g.set(
            "leaf_reduce_into!",
            lua.create_function(
                  |_,
                  (leaf_idx, grad, hess, leaf_grad, leaf_hess): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                  )| {
                        kernels::gpu_leaf_reduce(
                              &leaf_idx.buf,
                              &grad.buf,
                              &hess.buf,
                              &leaf_grad.buf,
                              &leaf_hess.buf,
                              grad.len(),
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "leaf_finalize_into!",
            lua.create_function(
                  |_,
                  (leaf_grad, leaf_hess, leaf_value, lambda): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        f64,
                  )| {
                        kernels::gpu_leaf_finalize(
                              &leaf_grad.buf,
                              &leaf_hess.buf,
                              &leaf_value.buf,
                              lambda as f32,
                              leaf_value.len(),
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "oblivious_split_eval_into!",
            lua.create_function(
                  |_,
                  (grad_hist, hess_hist, gain_out, n_nodes, n_bins, lambda): (
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        LuaGpuBuffer,
                        usize,
                        usize,
                        f64,
                  )| {
                        let n_features = gain_out.len() / n_bins;
                        kernels::gpu_oblivious_split_eval(
                              &grad_hist.buf,
                              &hess_hist.buf,
                              &gain_out.buf,
                              n_nodes,
                              n_features,
                              n_bins,
                              lambda as f32,
                              1e-3,
                        );
                        Ok(())
                  },
            )?,
      )?;

      g.set(
            "softmax_into!",
            lua.create_function(|_, (x, out): (LuaGpuBuffer, LuaGpuBuffer)| {
                  kernels::gpu_softmax_rows_into(&x.buf, &out.buf, x.rows, x.cols);
                  Ok(())
            })?,
      )?;

      g.set(
            "catboost_train",
            lua.create_function(
                  |_,
                  (x, y, n, p, n_classes, cat_features, params): (
                        Vec<f64>,
                        Vec<usize>,
                        usize,
                        usize,
                        usize,
                        Vec<usize>,
                        Option<LuaTable>,
                  )| {
                        let mut train_params = catboost_rs::Params {
                              cat_features,
                              ..Default::default()
                        };
                        if let Some(tbl) = params {
                              if let Ok(v) = tbl.get::<usize>("iterations") {
                                    train_params.iterations = v;
                              }
                              if let Ok(v) = tbl.get::<usize>("depth") {
                                    train_params.depth = v;
                              }
                              if let Ok(v) = tbl.get::<f64>("lr") {
                                    train_params.learning_rate = v;
                              }
                              if let Ok(v) = tbl.get::<f64>("l2_reg") {
                                    train_params.l2_reg = v;
                              }
                              if let Ok(v) = tbl.get::<usize>("n_permutations") {
                                    train_params.n_permutations = v;
                              }
                              if let Ok(v) = tbl.get::<u64>("seed") {
                                    train_params.seed = v;
                              }
                        }
                        let model = catboost_rs::train(&x, &y, n, p, n_classes, &train_params)
                              .map_err(|e| LuaError::runtime(e.to_string()))?;
                        let mut models = CATBOOST_MODELS.lock().unwrap();
                        let id = models.len();
                        models.push(model);
                        Ok(id)
                  },
            )?,
      )?;

      g.set(
            "catboost_predict",
            lua.create_function(|_, (model_id, x, n): (usize, Vec<f64>, usize)| {
                  let models = CATBOOST_MODELS.lock().unwrap();
                  let model = models
                        .get(model_id)
                        .ok_or_else(|| LuaError::runtime(format!("no catboost model {model_id}")))?;
                  let probs =
                        catboost_rs::predict(model, &x, n).map_err(|e| LuaError::runtime(e.to_string()))?;
                  Ok(probs)
            })?,
      )?;

      Ok(())
}

/// Initialize Lua with GPU buffer types, upload/download, and all GPU kernel composites.
pub fn init(lua: &Lua) -> LuaResult<()> {
      register_types(lua)?;
      register_upload_download(lua)?;
      register_composites(lua)?;
      Ok(())
}

/// Entry point for `require("nates_gpu")` from system Lua.
#[mlua::lua_module]
fn nates_gpu(lua: &Lua) -> LuaResult<LuaTable> {
      init(lua)?;
      Ok(lua.globals().clone())
}

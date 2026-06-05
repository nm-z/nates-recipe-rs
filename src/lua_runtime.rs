use mlua::prelude::*;
use gpu_core::memory::GpuBuffer;
use gpu_core::kernels;
use std::sync::Arc;

#[derive(Clone)]
pub struct LuaGpuBuffer {
      pub buf: Arc<GpuBuffer>,
      pub rows: usize,
      pub cols: usize,
}

impl LuaGpuBuffer {
      pub fn new(buf: GpuBuffer, rows: usize, cols: usize) -> Self {
            Self { buf: Arc::new(buf), rows, cols }
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
      let upload_fn = lua.create_function(|_, (data, rows, cols): (Vec<f64>, usize, usize)| {
            if data.len() != rows * cols {
                  return Err(LuaError::runtime(format!(
                        "upload: data len {} != rows*cols {}*{}={}",
                        data.len(), rows, cols, rows * cols
                  )));
            }
            let buf = GpuBuffer::upload(&data)
                  .map_err(|e| LuaError::runtime(e.to_string()))?;
            Ok(LuaGpuBuffer::new(buf, rows, cols))
      })?;
      lua.globals().set("upload", upload_fn)?;

      let download_fn = lua.create_function(|_, buf: LuaGpuBuffer| {
            let mut dst = vec![0.0f64; buf.len()];
            buf.buf.download(&mut dst)
                  .map_err(|e| LuaError::runtime(e.to_string()))?;
            Ok(dst)
      })?;
      lua.globals().set("download", download_fn)?;

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
      g.set("gemm", lua.create_function(|_, (a, b, ta, tb): (LuaGpuBuffer, LuaGpuBuffer, String, String)| {
            let (m, n, _k, buf) = match (ta.as_str(), tb.as_str()) {
                  ("N", "N") => {
                        // A[m,k] @ B[k,n] → [m,n]
                        (a.rows, b.cols, a.cols, kernels::gpu_gemm(&a.buf, &b.buf, a.rows, b.cols, a.cols).map_err(hip_err)?)
                  }
                  ("T", "N") => {
                        // A^T[m,k] where A is [k,m] @ B[k,n] → [m,n]
                        (a.cols, b.cols, a.rows, kernels::gpu_gemm_at(&a.buf, &b.buf, a.cols, b.cols, a.rows).map_err(hip_err)?)
                  }
                  ("N", "T") => {
                        // A[m,k] @ B^T[k,n] where B is [n,k] → [m,n]
                        (a.rows, b.rows, a.cols, kernels::gpu_gemm_bt(&a.buf, &b.buf, a.rows, b.rows, a.cols).map_err(hip_err)?)
                  }
                  _ => return Err(LuaError::runtime(format!("gemm: invalid transpose flags ({ta}, {tb}), expected N or T"))),
            };
            Ok(LuaGpuBuffer::new(buf, m, n))
      })?)?;

      // cholesky_solve(A, b, n) — GPU-resident, no CPU round-trip
      g.set("cholesky_solve", lua.create_function(|_, (a, b, n): (LuaGpuBuffer, LuaGpuBuffer, usize)| {
            let buf = kernels::gpu_cholesky_solve(&a.buf, &b.buf, n).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, 1))
      })?)?;

      // cholesky_inv(A, n) — GPU-resident, no CPU round-trip
      g.set("cholesky_inv", lua.create_function(|_, (a, n): (LuaGpuBuffer, usize)| {
            let buf = kernels::gpu_cholesky_inv(&a.buf, n).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, n))
      })?)?;

      // solve(A, b) — general LU solve, A*X = B
      g.set("solve", lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let n = a.rows;
            let nrhs = b.cols;
            let buf = kernels::gpu_solve(&a.buf, &b.buf, n, nrhs).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, nrhs))
      })?)?;

      // cholesky(A) — standalone Cholesky factorization, returns L
      g.set("cholesky", lua.create_function(|_, a: LuaGpuBuffer| {
            let n = a.rows;
            let buf = kernels::gpu_cholesky(&a.buf, n).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, n))
      })?)?;

      // tri_solve(L, b) — triangular solve L*X = B
      g.set("tri_solve", lua.create_function(|_, (l, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let n = l.rows;
            let nrhs = b.cols;
            let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, false).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, nrhs))
      })?)?;

      // tri_solve_t(L, b) — triangular solve L^T*X = B
      g.set("tri_solve_t", lua.create_function(|_, (l, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let n = l.rows;
            let nrhs = b.cols;
            let buf = kernels::gpu_tri_solve(&l.buf, &b.buf, n, nrhs, true).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, nrhs))
      })?)?;

      // ── Elementwise binary ────────────────────────────────────────────────

      g.set("add", lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_add(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
      })?)?;

      g.set("sub", lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_sub(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
      })?)?;

      g.set("mul", lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_mul(&a.buf, &b.buf, a.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
      })?)?;

      g.set("scale", lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_scale(&x.buf, s, x.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("sub_scale", lua.create_function(|_, (a, b, s): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_sub_scale(&a.buf, &b.buf, a.len(), s).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, a.rows, a.cols))
      })?)?;

      g.set("fma", lua.create_function(|_, (x, a, b): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_fma(&x.buf, &a.buf, &b.buf, x.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      // ── In-place ──────────────────────────────────────────────────────────

      g.set("scale_inplace", lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
            kernels::gpu_scale_inplace(&x.buf, s, x.len());
            Ok(x)
      })?)?;

      g.set("diag_add", lua.create_function(|_, (a, val): (LuaGpuBuffer, f64)| {
            kernels::gpu_add_diag(&a.buf, a.rows, val);
            Ok(a)
      })?)?;

      g.set("sgd_update", lua.create_function(|_, (w, grad, lr): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
            kernels::gpu_sgd_update(&w.buf, &grad.buf, lr, w.len());
            Ok(w)
      })?)?;

      // ── Activations ───────────────────────────────────────────────────────

      g.set("sigmoid", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_sigmoid(&x.buf, x.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("sigmoid_backward", lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_sigmoid_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
      })?)?;

      g.set("tanh_act", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_tanh(&x.buf, x.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("tanh_backward", lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_tanh_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
      })?)?;

      g.set("relu", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_relu(&x.buf, x.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("relu_backward", lua.create_function(|_, (grad, act): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_relu_backward(&grad.buf, &act.buf, grad.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
      })?)?;

      g.set("leaky_relu", lua.create_function(|_, (x, alpha): (LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_leaky_relu(&x.buf, x.len(), alpha).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("leaky_relu_backward", lua.create_function(|_, (grad, act, alpha): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_leaky_relu_backward(&grad.buf, &act.buf, grad.len(), alpha).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, grad.rows, grad.cols))
      })?)?;

      g.set("softmax", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_softmax_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("scaled_exp", lua.create_function(|_, (x, s): (LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_scaled_exp(&x.buf, x.len(), s).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      // ── Normalization ─────────────────────────────────────────────────────

      g.set("layernorm", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_layernorm(&x.buf, x.rows, x.cols, None, None).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      g.set("dropout", lua.create_function(|_, (x, mask, p): (LuaGpuBuffer, LuaGpuBuffer, f64)| {
            let buf = kernels::gpu_dropout(&x.buf, &mask.buf, x.len(), p).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      // ── Reductions ────────────────────────────────────────────────────────

      g.set("reduce_sum_cols", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_reduce_sum_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, 1, x.cols))
      })?)?;

      g.set("reduce_sum_rows", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_reduce_sum_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, 1))
      })?)?;

      g.set("reduce_mean_cols", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_reduce_mean_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, 1, x.cols))
      })?)?;

      g.set("reduce_var_cols", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_reduce_var_cols(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, 1, x.cols))
      })?)?;

      // ── Bias ──────────────────────────────────────────────────────────────

      g.set("bias_add", lua.create_function(|_, (x, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_bias_add(&x.buf, &b.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      // ── Distance / Sorting ────────────────────────────────────────────────

      g.set("pairwise_l2", lua.create_function(|_, (q, t): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_pairwise_l2(&q.buf, &t.buf, q.rows, t.rows, q.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, q.rows, t.rows))
      })?)?;

      g.set("argmin_rows", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_argmin_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, 1))
      })?)?;

      g.set("argmax_rows", lua.create_function(|_, x: LuaGpuBuffer| {
            let buf = kernels::gpu_argmax_rows(&x.buf, x.rows, x.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, 1))
      })?)?;

      g.set("topk_per_row", lua.create_function(|_, (x, k): (LuaGpuBuffer, usize)| {
            let buf = kernels::gpu_topk_per_row(&x.buf, x.rows, x.cols, k).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, k))
      })?)?;

      g.set("partial_argsort", lua.create_function(|_, (data, k): (LuaGpuBuffer, usize)| {
            let buf = kernels::gpu_partial_argsort(&data.buf, data.len(), k).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, k, 1))
      })?)?;

      // ── Convolution ───────────────────────────────────────────────────────

      g.set("im2col_1d", lua.create_function(|_, (x, ks): (LuaGpuBuffer, usize)| {
            let n = x.rows;
            let p = x.cols;
            let out_len = p - ks + 1;
            let buf = kernels::gpu_im2col_1d(&x.buf, n, p, ks).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n * out_len, ks))
      })?)?;

      g.set("im2col_2d", lua.create_function(|_, (x, c, h, w, kh, kw): (LuaGpuBuffer, usize, usize, usize, usize, usize)| {
            let n = x.len() / (c * h * w);
            let out_h = h - kh + 1;
            let out_w = w - kw + 1;
            let buf = kernels::gpu_im2col_2d(&x.buf, n, c, h, w, kh, kw).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n * out_h * out_w, c * kh * kw))
      })?)?;

      g.set("avg_pool_1d", lua.create_function(|_, (x, out_len, n_filters): (LuaGpuBuffer, usize, usize)| {
            let n = x.len() / (out_len * n_filters);
            let buf = kernels::gpu_avg_pool_1d(&x.buf, n, out_len, n_filters).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, n_filters))
      })?)?;

      g.set("pool_grad_expand", lua.create_function(|_, (grad, out_len, n_filters): (LuaGpuBuffer, usize, usize)| {
            let n = grad.len() / n_filters;
            let buf = kernels::gpu_pool_grad_expand(&grad.buf, n, out_len, n_filters).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n * out_len, n_filters))
      })?)?;

      // ── Clustering ────────────────────────────────────────────────────────

      // centroid_update returns {centroids, counts} table
      g.set("centroid_update", lua.create_function(|lua, (x, assignments, dim, k): (LuaGpuBuffer, LuaGpuBuffer, usize, usize)| {
            let n = x.rows;
            let (centroids, counts) = kernels::gpu_centroid_update(&x.buf, &assignments.buf, n, dim, k).map_err(hip_err)?;
            let tbl = lua.create_table()?;
            tbl.raw_set(1, LuaGpuBuffer::new(centroids, k, dim))?;
            tbl.raw_set(2, LuaGpuBuffer::new(counts, k, 1))?;
            Ok(tbl)
      })?)?;

      g.set("gaussian_ll", lua.create_function(|_, (x, means, vars, log_priors, k): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, usize)| {
            let n = x.rows;
            let p = x.cols;
            let buf = kernels::gpu_gaussian_ll(&x.buf, &means.buf, &vars.buf, &log_priors.buf, n, k, p).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, n, k))
      })?)?;

      // ── Sequence ──────────────────────────────────────────────────────────

      g.set("lstm_cell", lua.create_function(|_, (gates, c, h, hs): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, usize)| {
            let n = c.rows;
            kernels::gpu_lstm_cell(&gates.buf, &c.buf, &h.buf, n, hs);
            Ok((c, h))
      })?)?;

      // ── VAE ───────────────────────────────────────────────────────────────

      g.set("reparameterize", lua.create_function(|_, (mu, log_var, eps): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_reparameterize(&mu.buf, &log_var.buf, &eps.buf, mu.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, mu.rows, mu.cols))
      })?)?;

      g.set("kl_div", lua.create_function(|_, (mu, log_var): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_kl_div(&mu.buf, &log_var.buf, mu.len()).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, mu.rows, mu.cols))
      })?)?;

      // vae_backward_latent returns {grad_mu, grad_lv} table
      g.set("vae_backward_latent", lua.create_function(|lua, (grad_z, mu, log_var, eps, kl_weight): (LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, LuaGpuBuffer, f64)| {
            let n = mu.len();
            let (grad_mu, grad_lv) = kernels::gpu_vae_backward_latent(&grad_z.buf, &mu.buf, &log_var.buf, &eps.buf, n, kl_weight).map_err(hip_err)?;
            let tbl = lua.create_table()?;
            tbl.raw_set(1, LuaGpuBuffer::new(grad_mu, mu.rows, mu.cols))?;
            tbl.raw_set(2, LuaGpuBuffer::new(grad_lv, mu.rows, mu.cols))?;
            Ok(tbl)
      })?)?;

      g.set("log_det_cholesky", lua.create_function(|_, l: LuaGpuBuffer| {
            let val = kernels::gpu_log_det_cholesky(&l.buf, l.rows).map_err(hip_err)?;
            Ok(val)
      })?)?;

      // ── Misc ──────────────────────────────────────────────────────────────

      g.set("concat", lua.create_function(|_, (a, b): (LuaGpuBuffer, LuaGpuBuffer)| {
            let buf = kernels::gpu_concat(&a.buf, &b.buf, a.rows, a.cols, b.cols).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, a.rows, a.cols + b.cols))
      })?)?;

      // sign — no GPU kernel, download/signum/upload fallback
      g.set("sign", lua.create_function(|_, x: LuaGpuBuffer| {
            let mut data = vec![0.0f64; x.len()];
            x.buf.download(&mut data).map_err(hip_err)?;
            for v in &mut data { *v = v.signum(); }
            let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, x.rows, x.cols))
      })?)?;

      // ── Init helpers ──────────────────────────────────────────────────────

      g.set("randn", lua.create_function(|_, (rows, cols, seed): (usize, usize, u64)| {
            let n = rows * cols;
            let buf = kernels::gpu_randn(n, seed as u32).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, rows, cols))
      })?)?;

      g.set("zeros", lua.create_function(|_, (rows, cols): (usize, usize)| {
            let data = vec![0.0f64; rows * cols];
            let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, rows, cols))
      })?)?;

      g.set("ones", lua.create_function(|_, (rows, cols): (usize, usize)| {
            let data = vec![1.0f64; rows * cols];
            let buf = GpuBuffer::upload(&data).map_err(hip_err)?;
            Ok(LuaGpuBuffer::new(buf, rows, cols))
      })?)?;

      Ok(())
}

/// Initialize Lua with GPU buffer types, upload/download, and all GPU kernel composites.
pub fn init(lua: &Lua) -> LuaResult<()> {
      register_types(lua)?;
      register_upload_download(lua)?;
      register_composites(lua)?;
      Ok(())
}

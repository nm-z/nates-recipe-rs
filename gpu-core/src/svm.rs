use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;

unsafe extern "C" {
      // launch_kernel_matrix(x, k_out, n, dim, kind, gamma, coef0, degree, stream)
      fn launch_kernel_matrix(
            x:      *const c_void,
            k_out:  *mut   c_void,
            n:      i32,
            dim:    i32,
            kind:   i32,
            gamma:  f64,
            coef0:  f64,
            degree: f64,
            stream: *mut c_void,
      );

      // launch_smo_update_gradient(grad, K, n, row_i, row_j, delta_i, delta_j, stream)
      fn launch_smo_update_gradient(
            grad:    *mut   c_void,
            k:       *const c_void,
            n:       i32,
            row_i:   i32,
            row_j:   i32,
            delta_i: f64,
            delta_j: f64,
            stream:  *mut c_void,
      );

      // launch_smo_kkt_score(grad, alpha, y, score_i, score_j, n, C, stream)
      fn launch_smo_kkt_score(
            grad:    *const c_void,
            alpha:   *const c_void,
            y:       *const c_void,
            score_i: *mut   c_void,
            score_j: *mut   c_void,
            n:       i32,
            c:       f64,
            stream:  *mut c_void,
      );
}

// Compute the n×n kernel matrix for the n training samples in x (n×dim, row-major).
// kind: 0=linear, 1=rbf, 2=poly, 3=sigmoid.
// gamma, coef0, degree are kernel hyperparameters (full real-line valid; unused params ignored).
// Returns K[n*n] on GPU.
pub fn gpu_kernel_matrix(
      x:      &GpuBuffer,
      n:      usize,
      dim:    usize,
      kind:   i32,
      gamma:  f64,
      coef0:  f64,
      degree: f64,
) -> Result<GpuBuffer, HipError> {
      let k_out = GpuBuffer::alloc(n * n)?;
      unsafe {
            launch_kernel_matrix(
                  x.ptr_raw()     as *const c_void,
                  k_out.ptr_raw(),
                  n   as i32,
                  dim as i32,
                  kind,
                  gamma,
                  coef0,
                  degree,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(k_out)
}

// Working-set SMO training for binary SVM.
//
// Host/device split:
//   GPU: kernel matrix K (precomputed, n×n), gradient vector G, alpha vector, KKT score arrays.
//   Host: working-set selection (argmax of score_i, argmax of score_j), alpha pair update,
//         bias accumulation, convergence check.
//
// Each iteration:
//   1. GPU: launch_smo_kkt_score — writes per-sample I_up/I_down violation scores.
//   2. Host: download score_i, score_j (n doubles each); pick i = argmax(score_i),
//            j = argmax(score_j); check stopping: max_i(score_i[i]) - score_j[j] < tol → done.
//   3. Host: download K[i,i], K[i,j], K[j,j] (3 scalars, via row download).
//            Compute new alpha_i, alpha_j analytically (standard SMO closed-form).
//            Clip to box [0,C].
//   4. GPU: launch_smo_update_gradient with delta_i = y[i]*(new_i - old_i),
//            delta_j = y[j]*(new_j - old_j).
//   5. Host: update alpha host vector; update bias b estimate from newly-on-bound samples.
//
// y must be in {-1.0, +1.0}. C, tol, max_iter: full valid ranges (C>0, tol>0, max_iter>0).
// Returns (alpha[n], b) where b is the decision threshold.
pub fn gpu_smo_train(
      k:        &GpuBuffer,  // n×n kernel matrix on GPU
      y_pm1:    &[f64],      // labels in {-1,+1}, length n
      c:        f64,
      tol:      f64,
      max_iter: i32,
      n:        usize,
) -> Result<(Vec<f64>, f64), HipError> {
      // Upload label vector and initial alpha=0 to GPU.
      let y_buf     = GpuBuffer::upload(y_pm1)?;
      let alpha_buf = GpuBuffer::alloc(n)?;
      alpha_buf.memset_zero(n * std::mem::size_of::<f64>())?;

      // Gradient G[t] = -1 initially (all alphas = 0, so G[t] = sum_s 0 - 1 = -1).
      let neg_one: Vec<f64> = vec![-1.0_f64; n];
      let grad_buf = GpuBuffer::upload(&neg_one)?;

      let score_i_buf = GpuBuffer::alloc(n)?;
      let score_j_buf = GpuBuffer::alloc(n)?;

      let mut alpha_host = vec![0.0_f64; n];
      let mut b          = 0.0_f64;
      let mut b_count    = 0_usize;

      // Download the full K once for cheap per-iteration K[i,j] access.
      // n×n doubles; for large n the host loop dominates anyway.
      let mut k_host = vec![0.0_f64; n * n];
      unsafe {
            let src = k.ptr_raw() as *const c_void;
            let dst = k_host.as_mut_ptr() as *mut c_void;
            let bytes = n * n * std::mem::size_of::<f64>();
            crate::hip::check(crate::hip::hipMemcpy(dst, src, bytes, crate::hip::HIP_MEMCPY_D2H))?;
      }

      let mut score_i_host = vec![0.0_f64; n];
      let mut score_j_host = vec![0.0_f64; n];

      for _iter in 0..max_iter {
            // Score every sample for KKT violation.
            unsafe {
                  launch_smo_kkt_score(
                        grad_buf.ptr_raw()    as *const c_void,
                        alpha_buf.ptr_raw()   as *const c_void,
                        y_buf.ptr_raw()       as *const c_void,
                        score_i_buf.ptr_raw(),
                        score_j_buf.ptr_raw(),
                        n as i32,
                        c,
                        std::ptr::null_mut(),
                  );
            }
            crate::kernels::check_launch();

            score_i_buf.download(&mut score_i_host)?;
            score_j_buf.download(&mut score_j_host)?;

            // Pick working set: i maximises upper-set violation, j maximises lower-set violation.
            let i = score_i_host.iter().enumerate()
                  .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                  .map(|(idx, _)| idx)
                  .unwrap_or(0);
            let j = score_j_host.iter().enumerate()
                  .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                  .map(|(idx, _)| idx)
                  .unwrap_or(0);

            // Stopping condition: optimality gap < tol.
            let gap = score_i_host[i] - score_j_host[j];
            if gap < tol {
                  break;
            }

            let yi = y_pm1[i];
            let yj = y_pm1[j];

            let kii = k_host[i * n + i];
            let kjj = k_host[j * n + j];
            let kij = k_host[i * n + j];
            let eta = kii + kjj - 2.0 * kij;

            let old_ai = alpha_host[i];
            let old_aj = alpha_host[j];

            // Raw unconstrained step in the j direction.
            let grad_diff = -(score_i_host[i] - score_j_host[j]);
            let new_aj_raw = if eta.abs() > 1e-12 {
                  old_aj + yj * grad_diff / eta
            } else {
                  old_aj
            };

            // Box constraints [L, H] for alpha_j.
            let (lo, hi) = if (yi - yj).abs() < 1e-9 {
                  // Same sign: alpha_i + alpha_j = constant
                  let s = old_ai + old_aj;
                  (f64::max(0.0, s - c), f64::min(c, s))
            } else {
                  // Different sign: alpha_i - alpha_j = constant  (sign adjusted)
                  let s = old_ai - old_aj;
                  (f64::max(0.0, -s), f64::min(c, c - s))
            };

            let new_aj = new_aj_raw.clamp(lo, hi);
            let new_ai = old_ai + yi * yj * (old_aj - new_aj);
            let new_ai = new_ai.clamp(0.0, c);

            let delta_ai = new_ai - old_ai;
            let delta_aj = new_aj - old_aj;

            if delta_ai.abs() < 1e-12 && delta_aj.abs() < 1e-12 {
                  break;
            }

            // GPU gradient update: G[t] += yi*delta_ai * K[i,t] + yj*delta_aj * K[j,t]
            unsafe {
                  launch_smo_update_gradient(
                        grad_buf.ptr_raw(),
                        k.ptr_raw()  as *const c_void,
                        n as i32,
                        i as i32,
                        j as i32,
                        yi * delta_ai,
                        yj * delta_aj,
                        std::ptr::null_mut(),
                  );
            }
            crate::kernels::check_launch();

            alpha_host[i] = new_ai;
            alpha_host[j] = new_aj;

            // Bias: use newly on-bound samples for a running average.
            // b = -G[t]/y[t] for free support vectors (0 < alpha < C).
            // We read the updated gradient for i and j from GPU for the bias estimate.
            let mut g_ij = [0.0_f64; 2];
            {
                  let mut g_row = vec![0.0_f64; n];
                  grad_buf.download(&mut g_row)?;
                  g_ij[0] = g_row[i];
                  g_ij[1] = g_row[j];
            }
            if new_ai > 0.0 && new_ai < c {
                  b += -g_ij[0] / yi;
                  b_count += 1;
            }
            if new_aj > 0.0 && new_aj < c {
                  b += -g_ij[1] / yj;
                  b_count += 1;
            }
      }

      let b_final = if b_count > 0 { b / b_count as f64 } else { 0.0 };
      Ok((alpha_host, b_final))
}

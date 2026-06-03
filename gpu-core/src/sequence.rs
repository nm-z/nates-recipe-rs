use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

unsafe extern "C" {
      // log_trans [S*S], log_emit [T*S]
      // log_alpha [T*S] out, log_beta [T*S] out, log_gamma [T*S] out
      fn launch_forward_backward(
            log_trans: *const c_void,
            log_emit:  *const c_void,
            log_alpha: *mut c_void,
            log_beta:  *mut c_void,
            log_gamma: *mut c_void,
            n_states: i32,
            t_len:    i32,
            stream:   *mut c_void,
      );

      // log_trans [S*S], log_emit [T*S]
      // delta [T*S] scratch, backptr [T*S] i32 scratch, best_path [T] i32 out
      fn launch_viterbi(
            log_trans: *const c_void,
            log_emit:  *const c_void,
            delta:     *mut c_void,
            backptr:   *mut c_void,
            best_path: *mut c_void,
            n_states:  i32,
            t_len:     i32,
            stream:    *mut c_void,
      );
}

/// HMM forward-backward in log space.
///
/// log_trans: [n_states * n_states] — log_trans[s * n_states + s2] = log P(s2 | s)
/// log_emit:  [t_len * n_states]   — log_emit[t * n_states + s]   = log P(obs_t | s)
///
/// Returns (log_alpha, log_beta, log_gamma) each of length t_len * n_states.
pub fn gpu_forward_backward(
      log_trans: &GpuBuffer,
      log_emit:  &GpuBuffer,
      n_states:  usize,
      t_len:     usize,
) -> Result<(GpuBuffer, GpuBuffer, GpuBuffer), HipError> {
      let ts = t_len * n_states;
      let log_alpha = GpuBuffer::alloc(ts)?;
      let log_beta  = GpuBuffer::alloc(ts)?;
      let log_gamma = GpuBuffer::alloc(ts)?;

      unsafe {
            launch_forward_backward(
                  log_trans.ptr_raw() as *const c_void,
                  log_emit.ptr_raw()  as *const c_void,
                  log_alpha.ptr_raw(),
                  log_beta.ptr_raw(),
                  log_gamma.ptr_raw(),
                  n_states as i32,
                  t_len    as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok((log_alpha, log_beta, log_gamma))
}

/// Viterbi decoding in log space.
///
/// log_trans: [n_states * n_states]
/// log_emit:  [t_len * n_states]
///
/// Returns best_path as GpuBuffer containing i32[t_len].
/// Download with download_i32 into a &mut [i32] of length t_len.
pub fn gpu_viterbi(
      log_trans: &GpuBuffer,
      log_emit:  &GpuBuffer,
      n_states:  usize,
      t_len:     usize,
) -> Result<GpuBuffer, HipError> {
      let ts = t_len * n_states;
      // delta: f64 scratch
      let delta    = GpuBuffer::alloc(ts)?;
      // backptr: i32 scratch — allocate as bytes (4 bytes per i32)
      let backptr  = GpuBuffer::alloc_bytes(ts * 4)?;
      // best_path: i32 output
      let best_path = GpuBuffer::alloc_bytes(t_len * 4)?;

      unsafe {
            launch_viterbi(
                  log_trans.ptr_raw() as *const c_void,
                  log_emit.ptr_raw()  as *const c_void,
                  delta.ptr_raw(),
                  backptr.ptr_raw(),
                  best_path.ptr_raw(),
                  n_states as i32,
                  t_len    as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(best_path)
}

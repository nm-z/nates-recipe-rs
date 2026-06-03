use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

unsafe extern "C" {
      fn launch_discounted_returns(
            rewards: *const c_void,
            returns: *mut c_void,
            gamma:   f64,
            t_len:   i32,
            stream:  *mut c_void,
      );

      fn launch_gae(
            rewards:    *const c_void,
            values:     *const c_void,
            advantages: *mut c_void,
            gamma:      f64,
            lam:        f64,
            t_len:      i32,
            stream:     *mut c_void,
      );

      fn launch_td_targets(
            rewards:     *const c_void,
            values_next: *const c_void,
            done_mask:   *const c_void,
            targets:     *mut c_void,
            gamma:       f64,
            n:           i32,
            stream:      *mut c_void,
      );

      fn launch_categorical_logprob(
            logits:  *const c_void,
            actions: *const c_void,
            logp:    *mut c_void,
            n:       i32,
            n_actions: i32,
            stream:  *mut c_void,
      );

      fn launch_gaussian_logprob(
            mu:      *const c_void,
            log_std: *const c_void,
            actions: *const c_void,
            logp:    *mut c_void,
            n:       i32,
            dim:     i32,
            stream:  *mut c_void,
      );
}

/// Discounted returns for a single trajectory (reverse scan on one thread).
/// rewards: [t_len], gamma: discount factor.
/// Returns G[t_len] where G_t = r_t + gamma * G_{t+1}.
pub fn gpu_discounted_returns(
      rewards: &GpuBuffer,
      gamma:   f64,
      t_len:   usize,
) -> Result<GpuBuffer, HipError> {
      let returns = GpuBuffer::alloc(t_len)?;
      unsafe {
            launch_discounted_returns(
                  rewards.ptr_raw() as *const c_void,
                  returns.ptr_raw(),
                  gamma,
                  t_len as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(returns)
}

/// Generalized Advantage Estimation (reverse scan on one thread).
/// rewards: [t_len], values: [t_len], gamma, lam: GAE lambda.
/// Returns advantages[t_len].
pub fn gpu_gae(
      rewards: &GpuBuffer,
      values:  &GpuBuffer,
      gamma:   f64,
      lam:     f64,
      t_len:   usize,
) -> Result<GpuBuffer, HipError> {
      let advantages = GpuBuffer::alloc(t_len)?;
      unsafe {
            launch_gae(
                  rewards.ptr_raw()    as *const c_void,
                  values.ptr_raw()     as *const c_void,
                  advantages.ptr_raw(),
                  gamma,
                  lam,
                  t_len as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(advantages)
}

/// TD targets: targets[i] = r[i] + gamma * V_next[i] * (1 - done[i]).
/// done_mask: f64 buffer with 0.0 (not done) or 1.0 (done).
pub fn gpu_td_targets(
      rewards:     &GpuBuffer,
      values_next: &GpuBuffer,
      gamma:       f64,
      done_mask:   &GpuBuffer,
      n:           usize,
) -> Result<GpuBuffer, HipError> {
      let targets = GpuBuffer::alloc(n)?;
      unsafe {
            launch_td_targets(
                  rewards.ptr_raw()     as *const c_void,
                  values_next.ptr_raw() as *const c_void,
                  done_mask.ptr_raw()   as *const c_void,
                  targets.ptr_raw(),
                  gamma,
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(targets)
}

/// Categorical log-prob: logp[i] = log_softmax(logits[i])[actions[i]].
/// logits: [n * n_actions] f64.
/// actions_i32: [n] i32 buffer (upload_i32).
pub fn gpu_categorical_logprob(
      logits:     &GpuBuffer,
      actions_i32: &GpuBuffer,
      n:          usize,
      n_actions:  usize,
) -> Result<GpuBuffer, HipError> {
      let logp = GpuBuffer::alloc(n)?;
      unsafe {
            launch_categorical_logprob(
                  logits.ptr_raw()      as *const c_void,
                  actions_i32.ptr_raw() as *const c_void,
                  logp.ptr_raw(),
                  n as i32,
                  n_actions as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(logp)
}

/// Gaussian log-prob: logp[i] = sum_d [ -0.5*((a-mu)/sigma)^2 - log_std - 0.5*log(2pi) ].
/// mu, log_std, actions: [n * dim] f64.
pub fn gpu_gaussian_logprob(
      mu:      &GpuBuffer,
      log_std: &GpuBuffer,
      actions: &GpuBuffer,
      n:       usize,
      dim:     usize,
) -> Result<GpuBuffer, HipError> {
      let logp = GpuBuffer::alloc(n)?;
      unsafe {
            launch_gaussian_logprob(
                  mu.ptr_raw()      as *const c_void,
                  log_std.ptr_raw() as *const c_void,
                  actions.ptr_raw() as *const c_void,
                  logp.ptr_raw(),
                  n as i32,
                  dim as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(logp)
}

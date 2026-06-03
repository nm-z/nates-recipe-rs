use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

unsafe extern "C" {
      fn launch_momentum_update(
            w: *mut c_void, v: *mut c_void, g: *const c_void,
            lr: f64, momentum: f64, n: i32, stream: *mut c_void,
      );
      fn launch_rmsprop_update(
            w: *mut c_void, cache: *mut c_void, g: *const c_void,
            lr: f64, decay: f64, eps: f64, n: i32, stream: *mut c_void,
      );
      fn launch_adagrad_update(
            w: *mut c_void, accum: *mut c_void, g: *const c_void,
            lr: f64, eps: f64, n: i32, stream: *mut c_void,
      );
      fn launch_lamb_phase1(
            w: *mut c_void, m: *mut c_void, v: *mut c_void, g: *const c_void,
            b1: f64, b2: f64, eps: f64, wd: f64, t: i32, n: i32,
            tmp_upd: *mut c_void, w_norm_sq: *mut c_void, u_norm_sq: *mut c_void,
            stream: *mut c_void,
      );
      fn launch_lamb_phase2(
            w: *mut c_void, tmp_upd: *const c_void,
            lr: f64, w_norm_sq: f64, u_norm_sq: f64,
            n: i32, stream: *mut c_void,
      );
      fn launch_lion_update(
            w: *mut c_void, m: *mut c_void, g: *const c_void,
            lr: f64, b1: f64, b2: f64, wd: f64, n: i32, stream: *mut c_void,
      );
      fn launch_nadam_update(
            w: *mut c_void, m: *mut c_void, v: *mut c_void, g: *const c_void,
            lr: f64, b1: f64, b2: f64, eps: f64, t: i32, n: i32, stream: *mut c_void,
      );
      fn launch_clip_value(
            x: *mut c_void, lo: f64, hi: f64, n: i32, stream: *mut c_void,
      );
}

/// Momentum SGD: v = momentum*v - lr*g; w += v (in-place, updates both w and v).
pub fn gpu_momentum_update(w: &GpuBuffer, v: &GpuBuffer, g: &GpuBuffer, lr: f64, momentum: f64, n: usize) {
      unsafe {
            launch_momentum_update(
                  w.ptr_raw(), v.ptr_raw(), g.ptr_raw() as *const c_void,
                  lr, momentum, n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
}

/// RMSProp: cache = decay*cache + (1-decay)*g^2; w -= lr*g/(sqrt(cache)+eps) (in-place).
pub fn gpu_rmsprop_update(w: &GpuBuffer, cache: &GpuBuffer, g: &GpuBuffer, lr: f64, decay: f64, eps: f64, n: usize) {
      unsafe {
            launch_rmsprop_update(
                  w.ptr_raw(), cache.ptr_raw(), g.ptr_raw() as *const c_void,
                  lr, decay, eps, n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
}

/// Adagrad: accum += g^2; w -= lr*g/(sqrt(accum)+eps) (in-place).
pub fn gpu_adagrad_update(w: &GpuBuffer, accum: &GpuBuffer, g: &GpuBuffer, lr: f64, eps: f64, n: usize) {
      unsafe {
            launch_adagrad_update(
                  w.ptr_raw(), accum.ptr_raw(), g.ptr_raw() as *const c_void,
                  lr, eps, n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
}

/// LAMB: Adam moments + trust ratio = ||w|| / ||update|| (with 0-norm guard).
/// Two-phase: phase1 computes moments and accumulates norms on GPU, sync+download,
/// phase2 applies the trust-ratio-scaled update in-place.
pub fn gpu_lamb_update(
      w: &GpuBuffer, m: &GpuBuffer, v: &GpuBuffer, g: &GpuBuffer,
      lr: f64, b1: f64, b2: f64, eps: f64, wd: f64, t: i32, n: usize,
) -> Result<(), HipError> {
      let tmp_upd  = GpuBuffer::alloc(n)?;
      let w_norm_sq = GpuBuffer::alloc(1)?;
      let u_norm_sq = GpuBuffer::alloc(1)?;
      w_norm_sq.memset_zero(8)?;
      u_norm_sq.memset_zero(8)?;

      unsafe {
            launch_lamb_phase1(
                  w.ptr_raw(), m.ptr_raw(), v.ptr_raw(), g.ptr_raw() as *const c_void,
                  b1, b2, eps, wd, t, n as i32,
                  tmp_upd.ptr_raw(), w_norm_sq.ptr_raw(), u_norm_sq.ptr_raw(),
                  std::ptr::null_mut(),
            );
      }
      check_launch();

      crate::hip::device_synchronize()?;

      let mut wn = [0.0f64];
      let mut un = [0.0f64];
      w_norm_sq.download(&mut wn)?;
      u_norm_sq.download(&mut un)?;

      unsafe {
            launch_lamb_phase2(
                  w.ptr_raw(), tmp_upd.ptr_raw() as *const c_void,
                  lr, wn[0], un[0], n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(())
}

/// Lion: update = sign(b1*m + (1-b1)*g); w -= lr*(update + wd*w); m = b2*m + (1-b2)*g (in-place).
pub fn gpu_lion_update(w: &GpuBuffer, m: &GpuBuffer, g: &GpuBuffer, lr: f64, b1: f64, b2: f64, wd: f64, n: usize) {
      unsafe {
            launch_lion_update(
                  w.ptr_raw(), m.ptr_raw(), g.ptr_raw() as *const c_void,
                  lr, b1, b2, wd, n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
}

/// Nadam (Nesterov-accelerated Adam): uses next-step bias-corrected first moment.
pub fn gpu_nadam_update(w: &GpuBuffer, m: &GpuBuffer, v: &GpuBuffer, g: &GpuBuffer, lr: f64, b1: f64, b2: f64, eps: f64, t: i32, n: usize) {
      unsafe {
            launch_nadam_update(
                  w.ptr_raw(), m.ptr_raw(), v.ptr_raw(), g.ptr_raw() as *const c_void,
                  lr, b1, b2, eps, t, n as i32, std::ptr::null_mut(),
            );
      }
      check_launch();
}

/// In-place elementwise clamp: x[i] = clamp(x[i], lo, hi).
pub fn gpu_clip_value(x: &GpuBuffer, lo: f64, hi: f64, n: usize) {
      unsafe {
            launch_clip_value(x.ptr_raw(), lo, hi, n as i32, std::ptr::null_mut());
      }
      check_launch();
}

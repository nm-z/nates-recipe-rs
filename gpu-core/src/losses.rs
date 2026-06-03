use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;

unsafe extern "C" {
      fn launch_mae_grad(
            pred: *const c_void, target: *const c_void, out: *mut c_void,
            n: i32, stream: *mut c_void,
      );
      fn launch_huber_grad(
            pred: *const c_void, target: *const c_void, out: *mut c_void,
            n: i32, delta: f64, stream: *mut c_void,
      );
      fn launch_bce_logits(
            z: *const c_void, y: *const c_void,
            loss_out: *mut c_void, grad_out: *mut c_void,
            n: i32, stream: *mut c_void,
      );
      fn launch_focal_loss(
            prob: *const c_void, target: *const c_void,
            loss_out: *mut c_void, grad_out: *mut c_void,
            n: i32, gamma: f64, alpha: f64, stream: *mut c_void,
      );
      fn launch_kl_div_loss(
            log_p: *const c_void, target: *const c_void, out: *mut c_void,
            n: i32, stream: *mut c_void,
      );
      fn launch_hinge(
            scores: *const c_void, labels: *const c_void,
            loss_out: *mut c_void, grad_out: *mut c_void,
            n: i32, stream: *mut c_void,
      );
      fn launch_cosine_emb(
            a: *const c_void, b: *const c_void, label: *const c_void, out: *mut c_void,
            n: i32, dim: i32, margin: f64, stream: *mut c_void,
      );
      fn launch_triplet(
            anchor: *const c_void, pos: *const c_void, neg: *const c_void, out: *mut c_void,
            n: i32, dim: i32, margin: f64, stream: *mut c_void,
      );
      fn launch_contrastive(
            a: *const c_void, b: *const c_void, label: *const c_void, out: *mut c_void,
            n: i32, dim: i32, margin: f64, stream: *mut c_void,
      );
}

pub fn gpu_mae_grad(pred: &GpuBuffer, target: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_mae_grad(
                  pred.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

pub fn gpu_huber_grad(
      pred: &GpuBuffer, target: &GpuBuffer, delta: f64, n: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_huber_grad(
                  pred.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  delta,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

pub fn gpu_bce_with_logits(
      logits: &GpuBuffer, target: &GpuBuffer, n: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let loss = GpuBuffer::alloc(n)?;
      let grad = GpuBuffer::alloc(n)?;
      unsafe {
            launch_bce_logits(
                  logits.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  loss.ptr_raw(),
                  grad.ptr_raw(),
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok((loss, grad))
}

pub fn gpu_focal_loss(
      prob: &GpuBuffer, target: &GpuBuffer, gamma: f64, alpha: f64, n: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let loss = GpuBuffer::alloc(n)?;
      let grad = GpuBuffer::alloc(n)?;
      unsafe {
            launch_focal_loss(
                  prob.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  loss.ptr_raw(),
                  grad.ptr_raw(),
                  n as i32,
                  gamma,
                  alpha,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok((loss, grad))
}

pub fn gpu_kl_div_loss(
      log_p: &GpuBuffer, target: &GpuBuffer, n: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_kl_div_loss(
                  log_p.ptr_raw() as *const c_void,
                  target.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

pub fn gpu_hinge_loss(
      scores: &GpuBuffer, labels: &GpuBuffer, n: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let loss = GpuBuffer::alloc(n)?;
      let grad = GpuBuffer::alloc(n)?;
      unsafe {
            launch_hinge(
                  scores.ptr_raw() as *const c_void,
                  labels.ptr_raw() as *const c_void,
                  loss.ptr_raw(),
                  grad.ptr_raw(),
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok((loss, grad))
}

pub fn gpu_cosine_embedding_loss(
      a: &GpuBuffer, b: &GpuBuffer, label: &GpuBuffer,
      n: usize, dim: usize, margin: f64,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_cosine_emb(
                  a.ptr_raw() as *const c_void,
                  b.ptr_raw() as *const c_void,
                  label.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  dim as i32,
                  margin,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

pub fn gpu_triplet_loss(
      anchor: &GpuBuffer, pos: &GpuBuffer, neg: &GpuBuffer,
      n: usize, dim: usize, margin: f64,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_triplet(
                  anchor.ptr_raw() as *const c_void,
                  pos.ptr_raw() as *const c_void,
                  neg.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  dim as i32,
                  margin,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

pub fn gpu_contrastive_loss(
      a: &GpuBuffer, b: &GpuBuffer, label: &GpuBuffer,
      n: usize, dim: usize, margin: f64,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_contrastive(
                  a.ptr_raw() as *const c_void,
                  b.ptr_raw() as *const c_void,
                  label.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  dim as i32,
                  margin,
                  std::ptr::null_mut(),
            );
      }
      crate::kernels::check_launch();
      Ok(out)
}

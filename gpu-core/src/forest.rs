use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::{check_launch, gpu_rand_uniform};

unsafe extern "C" {
      fn launch_floor_scale_to_idx(
            uniform: *const c_void,
            idx_out: *mut c_void,
            n_samples: i32,
            n: i32,
            stream: *mut c_void,
      );
      fn launch_feature_subset(
            idx_out: *mut c_void,
            n_features: i32,
            k: i32,
            seed: u32,
            stream: *mut c_void,
      );
      fn launch_random_threshold_split(
            col: *const c_void,
            threshold_out: *mut c_void,
            n: i32,
            seed: u32,
            stream: *mut c_void,
      );
      fn launch_oob_mask(
            bootstrap_idx: *const c_void,
            oob_out: *mut c_void,
            n_samples: i32,
            n: i32,
            stream: *mut c_void,
      );
}

pub fn gpu_bootstrap_sample(n: usize, n_samples: usize, seed: u32) -> Result<GpuBuffer, HipError> {
      let uniform = gpu_rand_uniform(n_samples, seed)?;
      let out = GpuBuffer::alloc_bytes(n_samples * 4)?;
      unsafe {
            launch_floor_scale_to_idx(
                  uniform.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n_samples as i32,
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_feature_subset(n_features: usize, k: usize, seed: u32) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n_features * 4)?;
      unsafe {
            launch_feature_subset(
                  out.ptr_raw(),
                  n_features as i32,
                  k as i32,
                  seed,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_random_threshold_split(feature_col: &GpuBuffer, n: usize, seed: u32) -> Result<f64, HipError> {
      let out = GpuBuffer::alloc(1)?;
      unsafe {
            launch_random_threshold_split(
                  feature_col.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n as i32,
                  seed,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      let mut result = [0.0f64];
      out.download(&mut result)?;
      Ok(result[0])
}

pub fn gpu_oob_mask(bootstrap_idx_i32: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let n_samples = bootstrap_idx_i32.len() / 4;
      let out = GpuBuffer::zeros_bytes(n)?;
      unsafe {
            launch_oob_mask(
                  bootstrap_idx_i32.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  n_samples as i32,
                  n as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

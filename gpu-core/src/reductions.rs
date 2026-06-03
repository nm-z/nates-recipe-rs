use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::{check_launch, safe_i32};

unsafe extern "C" {
      fn launch_sum_all(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_max_all(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_min_all(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_mean_all(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_l2_norm(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_dot(a: *const c_void, b: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

      fn launch_sort(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_argsort(x: *const c_void, out_idx: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_sort_by_key(
            keys: *const c_void, vals: *const c_void,
            out_keys: *mut c_void, out_vals: *mut c_void,
            n: i32, stream: *mut c_void,
      );
      fn launch_segment_sort(
            data: *const c_void, seg_offsets: *const c_void,
            out: *mut c_void, n: i32, n_segs: i32, stream: *mut c_void,
      );

      fn launch_cumsum_rows(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
      fn launch_cumsum_cols(x: *const c_void, out: *mut c_void, rows: i32, cols: i32, stream: *mut c_void);
      fn launch_cumprod(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);
      fn launch_cummax(x: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

      fn launch_segment_sum(
            vals: *const c_void, seg_ids: *const c_void,
            out: *mut c_void, n: i32, n_segs: i32, stream: *mut c_void,
      );
      fn launch_segment_max(
            vals: *const c_void, seg_ids: *const c_void,
            out: *mut c_void, n: i32, n_segs: i32, stream: *mut c_void,
      );

      fn launch_scan_linear_recurrence(
            a: *const c_void, b: *const c_void, states: *mut c_void,
            n_steps: i32, dim: i32, stream: *mut c_void,
      );
}

fn scalar_reduce(
      f: unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void),
      x: &GpuBuffer,
      n: usize,
) -> Result<f64, HipError> {
      let out = GpuBuffer::alloc(1)?;
      unsafe { f(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      let mut v = [0.0f64];
      out.download(&mut v)?;
      Ok(v[0])
}

pub fn gpu_sum_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      scalar_reduce(launch_sum_all, x, n)
}

pub fn gpu_max_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      scalar_reduce(launch_max_all, x, n)
}

pub fn gpu_min_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      scalar_reduce(launch_min_all, x, n)
}

pub fn gpu_mean_all(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      let s = scalar_reduce(launch_mean_all, x, n)?;
      Ok(s / n as f64)
}

pub fn gpu_l2_norm(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      let sum_sq = scalar_reduce(launch_l2_norm, x, n)?;
      Ok(sum_sq.sqrt())
}

pub fn gpu_dot(a: &GpuBuffer, b: &GpuBuffer, n: usize) -> Result<f64, HipError> {
      let out = GpuBuffer::alloc(1)?;
      unsafe {
            launch_dot(
                  a.ptr_raw() as *const c_void,
                  b.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(n),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      let mut v = [0.0f64];
      out.download(&mut v)?;
      Ok(v[0])
}

pub fn gpu_sort(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_sort(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_argsort(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc_bytes(n * 4)?;
      unsafe { launch_argsort(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_sort_by_key(
      keys: &GpuBuffer, vals: &GpuBuffer, n: usize,
) -> Result<(GpuBuffer, GpuBuffer), HipError> {
      let out_keys = GpuBuffer::alloc(n)?;
      let out_vals = GpuBuffer::alloc(n)?;
      unsafe {
            launch_sort_by_key(
                  keys.ptr_raw() as *const c_void,
                  vals.ptr_raw() as *const c_void,
                  out_keys.ptr_raw(),
                  out_vals.ptr_raw(),
                  safe_i32(n),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok((out_keys, out_vals))
}

pub fn gpu_segment_sort(
      data: &GpuBuffer, seg_offsets: &GpuBuffer, n: usize, n_segs: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe {
            launch_segment_sort(
                  data.ptr_raw() as *const c_void,
                  seg_offsets.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(n),
                  safe_i32(n_segs),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_cumsum_rows(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * cols)?;
      unsafe {
            launch_cumsum_rows(
                  x.ptr_raw() as *const c_void, out.ptr_raw(),
                  safe_i32(rows), safe_i32(cols), std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_cumsum_cols(x: &GpuBuffer, rows: usize, cols: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(rows * cols)?;
      unsafe {
            launch_cumsum_cols(
                  x.ptr_raw() as *const c_void, out.ptr_raw(),
                  safe_i32(rows), safe_i32(cols), std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_cumprod(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_cumprod(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_cummax(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n)?;
      unsafe { launch_cummax(x.ptr_raw() as *const c_void, out.ptr_raw(), safe_i32(n), std::ptr::null_mut()); }
      check_launch();
      Ok(out)
}

pub fn gpu_segment_sum(
      vals: &GpuBuffer, seg_ids: &GpuBuffer, n: usize, n_segs: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n_segs)?;
      unsafe {
            launch_segment_sum(
                  vals.ptr_raw() as *const c_void,
                  seg_ids.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(n),
                  safe_i32(n_segs),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_segment_max(
      vals: &GpuBuffer, seg_ids: &GpuBuffer, n: usize, n_segs: usize,
) -> Result<GpuBuffer, HipError> {
      let out = GpuBuffer::alloc(n_segs)?;
      unsafe {
            launch_segment_max(
                  vals.ptr_raw() as *const c_void,
                  seg_ids.ptr_raw() as *const c_void,
                  out.ptr_raw(),
                  safe_i32(n),
                  safe_i32(n_segs),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(out)
}

pub fn gpu_scan_linear_recurrence(
      a: &GpuBuffer, b: &GpuBuffer, n_steps: usize, dim: usize,
) -> Result<GpuBuffer, HipError> {
      let states = GpuBuffer::alloc(n_steps * dim)?;
      unsafe {
            launch_scan_linear_recurrence(
                  a.ptr_raw() as *const c_void,
                  b.ptr_raw() as *const c_void,
                  states.ptr_raw(),
                  safe_i32(n_steps),
                  safe_i32(dim),
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(states)
}

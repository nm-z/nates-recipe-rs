use std::ffi::c_void;
use crate::memory::GpuBuffer;
use crate::hip::HipError;
use crate::kernels::check_launch;

unsafe extern "C" {
      fn launch_fixed_radius_neighbors(
            points: *const c_void,
            mask: *mut c_void,
            count: *mut c_void,
            n: i32,
            dim: i32,
            eps: f64,
            stream: *mut c_void,
      );

      fn launch_union_find_cc(
            edge_src: *const c_void,
            edge_dst: *const c_void,
            parent_buf: *mut c_void,
            n_nodes: i32,
            n_edges: i32,
            stream: *mut c_void,
      );

      fn launch_boruvka_mst(
            edge_src: *const c_void,
            edge_dst: *const c_void,
            edge_w: *const c_void,
            in_mst: *mut c_void,
            total_weight_buf: *mut c_void,
            n_nodes: i32,
            n_edges: i32,
            stream: *mut c_void,
      );

      fn launch_core_distance(
            points: *const c_void,
            core_dist: *mut c_void,
            n: i32,
            dim: i32,
            min_pts: i32,
            stream: *mut c_void,
      );
}

pub struct FixedRadiusResult {
      pub neighbor_count: GpuBuffer,
      pub within_mask: GpuBuffer,
}

pub fn gpu_fixed_radius_neighbors(
      points: &GpuBuffer,
      n: usize,
      dim: usize,
      eps: f64,
) -> Result<FixedRadiusResult, HipError> {
      let count = GpuBuffer::alloc_bytes(n * std::mem::size_of::<i32>())?;
      let mask = GpuBuffer::zeros_bytes(n * n)?;
      unsafe {
            launch_fixed_radius_neighbors(
                  points.ptr_raw() as *const c_void,
                  mask.ptr_raw(),
                  count.ptr_raw(),
                  n as i32,
                  dim as i32,
                  eps,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(FixedRadiusResult { neighbor_count: count, within_mask: mask })
}

pub fn gpu_union_find_cc(
      edge_src: &GpuBuffer,
      edge_dst: &GpuBuffer,
      n_nodes: usize,
      n_edges: usize,
) -> Result<GpuBuffer, HipError> {
      let labels = GpuBuffer::alloc_bytes(n_nodes * std::mem::size_of::<i32>())?;
      unsafe {
            launch_union_find_cc(
                  edge_src.ptr_raw() as *const c_void,
                  edge_dst.ptr_raw() as *const c_void,
                  labels.ptr_raw(),
                  n_nodes as i32,
                  n_edges as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(labels)
}

pub struct BoruvkaResult {
      pub in_mst: GpuBuffer,
      pub total_weight: f64,
}

pub fn gpu_boruvka_mst(
      edge_src: &GpuBuffer,
      edge_dst: &GpuBuffer,
      edge_w: &GpuBuffer,
      n_nodes: usize,
      n_edges: usize,
) -> Result<BoruvkaResult, HipError> {
      let in_mst = GpuBuffer::zeros_bytes(n_edges)?;
      let weight_buf = GpuBuffer::alloc(1)?;
      unsafe {
            launch_boruvka_mst(
                  edge_src.ptr_raw() as *const c_void,
                  edge_dst.ptr_raw() as *const c_void,
                  edge_w.ptr_raw() as *const c_void,
                  in_mst.ptr_raw(),
                  weight_buf.ptr_raw(),
                  n_nodes as i32,
                  n_edges as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      let mut tw = [0.0f64];
      weight_buf.download(&mut tw)?;
      Ok(BoruvkaResult { in_mst, total_weight: tw[0] })
}

pub fn gpu_core_distance(
      points: &GpuBuffer,
      n: usize,
      dim: usize,
      min_pts: usize,
) -> Result<GpuBuffer, HipError> {
      let core_dist = GpuBuffer::alloc(n)?;
      unsafe {
            launch_core_distance(
                  points.ptr_raw() as *const c_void,
                  core_dist.ptr_raw(),
                  n as i32,
                  dim as i32,
                  min_pts as i32,
                  std::ptr::null_mut(),
            );
      }
      check_launch();
      Ok(core_dist)
}

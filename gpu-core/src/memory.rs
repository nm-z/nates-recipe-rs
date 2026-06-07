use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::hip::*;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn alloc_count_reset() -> usize {
      ALLOC_COUNT.swap(0, Ordering::Relaxed)
}

const ARENA_ALIGN: usize = 256;
static ARENA_BASE: AtomicUsize = AtomicUsize::new(0);
static ARENA_SIZE: AtomicUsize = AtomicUsize::new(0);
static ARENA_OFFSET: AtomicUsize = AtomicUsize::new(0);

pub struct GpuBuffer {
      pub(crate) ptr: *mut c_void,
      len: usize,
      owned: bool,
}

unsafe impl Send for GpuBuffer {}
unsafe impl Sync for GpuBuffer {}

impl GpuBuffer {
      pub fn borrow(ptr: *mut c_void, len: usize) -> Self {
            Self { ptr, len, owned: false }
      }

      pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
            Self::alloc_bytes(n_floats * std::mem::size_of::<f64>())
      }

      pub fn alloc_checked(n_bytes: usize) -> Result<(), HipError> {
            let mut free: usize = 0;
            let mut total: usize = 0;
            check(unsafe { hipMemGetInfo(&mut free, &mut total) })?;
            if (total - free) + n_bytes > total * 90 / 100 {
                  return Err(HipError(2));
            }
            let mut ptr: *mut c_void = std::ptr::null_mut();
            check(unsafe { hipMalloc(&mut ptr, n_bytes) })?;
            ARENA_BASE.store(ptr as usize, Ordering::Relaxed);
            ARENA_SIZE.store(n_bytes, Ordering::Relaxed);
            ARENA_OFFSET.store(0, Ordering::Relaxed);
            Ok(())
      }

      pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            let base = ARENA_BASE.load(Ordering::Relaxed);
            if base != 0 {
                  let size = ARENA_SIZE.load(Ordering::Relaxed);
                  let aligned = (n_bytes + ARENA_ALIGN - 1) & !(ARENA_ALIGN - 1);
                  let mut off = ARENA_OFFSET.load(Ordering::Relaxed);
                  while off + aligned <= size {
                        match ARENA_OFFSET.compare_exchange_weak(
                              off, off + aligned, Ordering::Relaxed, Ordering::Relaxed) {
                              Ok(_) => {
                                    let ptr = unsafe { (base as *mut u8).add(off) as *mut c_void };
                                    return Ok(Self { ptr, len: n_bytes, owned: false });
                              }
                              Err(cur) => off = cur,
                        }
                  }
            }
            let mut ptr: *mut c_void = std::ptr::null_mut();
            check(unsafe { hipMalloc(&mut ptr, n_bytes) })?;
            Ok(Self { ptr, len: n_bytes, owned: true })
      }

      pub fn upload(data: &[f64]) -> Result<Self, HipError> {
            let buf = Self::alloc(data.len())?;
            let bytes = data.len() * std::mem::size_of::<f64>();
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn upload_u8(data: &[u8]) -> Result<Self, HipError> {
            let buf = Self::alloc_bytes(data.len())?;
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, data.len(), HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
            let bytes = data.len() * 4;
            let buf = Self::alloc_bytes(bytes)?;
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
            let bytes = data.len() * 4;
            let buf = Self::alloc_bytes(bytes)?;
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn zeros_bytes(n_bytes: usize) -> Result<Self, HipError> {
            let buf = Self::alloc_bytes(n_bytes)?;
            check(unsafe { hipMemset(buf.ptr, 0, n_bytes) })?;
            Ok(buf)
      }

      pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
            Self::zeros_bytes(n * 4)
      }

      pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
            check(unsafe { hipMemset(self.ptr, 0, n_bytes) })
      }

      pub fn download(&self, dst: &mut [f64]) -> Result<(), HipError> {
            let bytes = dst.len() * std::mem::size_of::<f64>();
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H)
            })
      }

      pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
            let bytes = dst.len() * 4;
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H)
            })
      }

      pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr, dst.len(), HIP_MEMCPY_D2H)
            })
      }

      pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
            let bytes = dst.len() * 4;
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H)
            })
      }

      pub fn len(&self) -> usize { self.len }
      pub fn n_floats(&self) -> usize { self.len / std::mem::size_of::<f64>() }
      pub fn ptr_addr(&self) -> usize { self.ptr as usize }
      pub fn ptr_raw(&self) -> *mut c_void { self.ptr }

      pub fn is_empty(&self) -> bool { self.len == 0 }

      pub fn as_ptr_offset(&self, n_floats: usize) -> *mut c_void {
            unsafe { (self.ptr as *mut u8).add(n_floats * 8) as *mut c_void }
      }

      pub fn view(&self, offset_floats: usize, len_floats: usize) -> GpuBuffer {
            GpuBuffer::borrow(self.as_ptr_offset(offset_floats), len_floats * 8)
      }

      pub fn copy_from(&mut self, src: &GpuBuffer, n_bytes: usize) -> Result<(), HipError> {
            check(unsafe { hipMemcpy(self.ptr, src.ptr as *const c_void, n_bytes, HIP_MEMCPY_D2D) })
      }

      pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
            check(unsafe { hipMemset(self.ptr, value as i32, n_bytes) })
      }

      pub fn upload_async(data: &[f64], stream: *mut c_void) -> Result<Self, HipError> {
            let bytes = data.len() * std::mem::size_of::<f64>();
            let buf = Self::alloc(data.len())?;
            check(unsafe {
                  hipMemcpyAsync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, stream)
            })?;
            Ok(buf)
      }

      pub fn download_async(&self, dst: &mut [f64], stream: *mut c_void) -> Result<(), HipError> {
            let bytes = dst.len() * std::mem::size_of::<f64>();
            check(unsafe {
                  hipMemcpyAsync(dst.as_mut_ptr() as *mut c_void, self.ptr as *const c_void, bytes, HIP_MEMCPY_D2H, stream)
            })
      }

      pub fn download_vec(&self) -> Result<Vec<f64>, HipError> {
            let mut v = vec![0.0f64; self.n_floats()];
            self.download(&mut v)?;
            Ok(v)
      }

      pub fn download_vec_f32(&self) -> Result<Vec<f32>, HipError> {
            let mut v = vec![0.0f32; self.len / 4];
            self.download_f32(&mut v)?;
            Ok(v)
      }

      pub fn upload_f16(data: &[half::f16]) -> Result<Self, HipError> {
            let bytes = data.len() * 2;
            let buf = Self::alloc_bytes(bytes)?;
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
            let bytes = dst.len() * 2;
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr as *const c_void, bytes, HIP_MEMCPY_D2H)
            })
      }

      pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
            let bytes = data.len() * 2;
            let buf = Self::alloc_bytes(bytes)?;
            check(unsafe {
                  hipMemcpy(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D)
            })?;
            Ok(buf)
      }

      pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
            let bytes = dst.len() * 2;
            check(unsafe {
                  hipMemcpy(dst.as_mut_ptr() as *mut c_void, self.ptr as *const c_void, bytes, HIP_MEMCPY_D2H)
            })
      }
}

impl Drop for GpuBuffer {
      fn drop(&mut self) {
            if self.owned && !self.ptr.is_null() {
                  unsafe { hipFree(self.ptr) };
                  self.ptr = std::ptr::null_mut();
            }
      }
}

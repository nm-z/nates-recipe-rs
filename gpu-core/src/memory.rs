use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::hip::*;

static SPILL_MODE: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn alloc_count_reset() -> usize {
      ALLOC_COUNT.swap(0, Ordering::Relaxed)
}

const HIP_MEM_ATTACH_GLOBAL: u32 = 0x01;

unsafe extern "C" {
      fn hipMallocManaged(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
}

static GC_HOOK: OnceLock<fn()> = OnceLock::new();

pub fn set_gc_hook(hook: fn()) {
      let _ = GC_HOOK.set(hook);
}

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

      pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let mut free: usize = 0;
            let mut total: usize = 0;
            unsafe { hipMemGetInfo(&mut free, &mut total) };
            let mut used = total - free;

            if SPILL_MODE.load(Ordering::Relaxed) {
                  if used < total * 70 / 100 {
                        SPILL_MODE.store(false, Ordering::Relaxed);
                  } else {
                        check(unsafe { hipMallocManaged(&mut ptr, n_bytes, HIP_MEM_ATTACH_GLOBAL) })?;
                        return Ok(Self { ptr, len: n_bytes, owned: true });
                  }
            }

            if used + n_bytes <= total * 90 / 100 {
                  check(unsafe { hipMalloc(&mut ptr, n_bytes) })?;
                  return Ok(Self { ptr, len: n_bytes, owned: true });
            }

            if let Some(gc) = GC_HOOK.get() {
                  gc();
            }
            unsafe { hipMemGetInfo(&mut free, &mut total) };
            used = total - free;

            if used + n_bytes <= total * 90 / 100 {
                  check(unsafe { hipMalloc(&mut ptr, n_bytes) })?;
                  return Ok(Self { ptr, len: n_bytes, owned: true });
            }

            SPILL_MODE.store(true, Ordering::Relaxed);
            check(unsafe { hipMallocManaged(&mut ptr, n_bytes, HIP_MEM_ATTACH_GLOBAL) })?;
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
}

impl Drop for GpuBuffer {
      fn drop(&mut self) {
            if self.owned && !self.ptr.is_null() {
                  unsafe { hipFree(self.ptr) };
                  self.ptr = std::ptr::null_mut();
            }
      }
}

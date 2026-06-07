use std::ffi::{CStr, c_void};
use std::fmt;

#[derive(Debug)]
pub struct HipError(pub i32);

impl fmt::Display for HipError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		unsafe {
			let name_ptr = hipGetErrorName(self.0);
			let str_ptr = hipGetErrorString(self.0);
			if !name_ptr.is_null() && !str_ptr.is_null() {
				let name = CStr::from_ptr(name_ptr).to_string_lossy();
				let msg = CStr::from_ptr(str_ptr).to_string_lossy();
				write!(f, "{}: {} (code {})", name, msg, self.0)
			} else {
				write!(f, "HIP error code {}", self.0)
			}
		}
	}
}
impl std::error::Error for HipError {}

pub fn check(code: i32) -> Result<(), HipError> {
	if code == 0 {
		Ok(())
	} else {
		Err(HipError(code))
	}
}

pub const HIP_MEMCPY_H2D: i32 = 1;
pub const HIP_MEMCPY_D2H: i32 = 2;
pub const HIP_MEMCPY_D2D: i32 = 3;

unsafe extern "C" {
	pub fn hipMalloc(ptr: *mut *mut c_void, size: usize) -> i32;
	pub fn hipFree(ptr: *mut c_void) -> i32;
	pub fn hipMemcpy(dst: *mut c_void, src: *const c_void, size: usize, kind: i32) -> i32;
	pub fn hipMemset(dst: *mut c_void, value: i32, size: usize) -> i32;
	pub fn hipGetLastError() -> i32;
	pub fn hipDeviceSynchronize() -> i32;
	pub fn hipEventCreate(event: *mut *mut c_void) -> i32;
	pub fn hipEventDestroy(event: *mut c_void) -> i32;
	pub fn hipEventRecord(event: *mut c_void, stream: *mut c_void) -> i32;
	pub fn hipEventSynchronize(event: *mut c_void) -> i32;
	pub fn hipEventElapsedTime(ms: *mut f32, start: *mut c_void, stop: *mut c_void) -> i32;
	pub fn hipSetDevice(device: i32) -> i32;
	pub fn hipStreamCreate(stream: *mut *mut c_void) -> i32;
	pub fn hipStreamSynchronize(stream: *mut c_void) -> i32;
	pub fn hipStreamDestroy(stream: *mut c_void) -> i32;
	pub fn hipMemGetInfo(free: *mut usize, total: *mut usize) -> i32;
	// Error string helpers — used by HipError::Display
	pub fn hipGetErrorName(error: i32) -> *const i8;
	pub fn hipGetErrorString(error: i32) -> *const i8;
	// Peek at last error without clearing it
	pub fn hipPeekAtLastError() -> i32;
	// Async transfers
	pub fn hipMemcpyAsync(
		dst: *mut c_void,
		src: *const c_void,
		size: usize,
		kind: i32,
		stream: *mut c_void,
	) -> i32;
	pub fn hipMemsetAsync(dst: *mut c_void, value: i32, size: usize, stream: *mut c_void) -> i32;
	// Pinned host memory
	pub fn hipHostMalloc(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
	pub fn hipHostFree(ptr: *mut c_void) -> i32;
	pub fn hipHostRegister(ptr: *mut c_void, size: usize, flags: u32) -> i32;
	pub fn hipHostUnregister(ptr: *mut c_void) -> i32;
	// Device count and attributes
	// Note: hipDeviceProp_t is a ~800-byte struct; we expose hipDeviceGetAttribute instead
	// to avoid defining it in Rust and to allow querying individual fields by attribute enum int.
	pub fn hipGetDeviceCount(count: *mut i32) -> i32;
	pub fn hipDeviceGetAttribute(pi: *mut i32, attr: i32, device_id: i32) -> i32;
	// Peer access
	pub fn hipDeviceCanAccessPeer(
		can_access_peer: *mut i32,
		device_id: i32,
		peer_device_id: i32,
	) -> i32;
	pub fn hipDeviceEnablePeerAccess(peer_device_id: i32, flags: u32) -> i32;
	pub fn hipMemcpyPeer(
		dst: *mut c_void,
		dst_device: i32,
		src: *const c_void,
		src_device: i32,
		size: usize,
	) -> i32;
	// Stream-ordered allocation
	pub fn hipMallocAsync(dev_ptr: *mut *mut c_void, size: usize, stream: *mut c_void) -> i32;
	pub fn hipFreeAsync(dev_ptr: *mut c_void, stream: *mut c_void) -> i32;
	// Managed (unified) memory
	pub fn hipMallocManaged(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
	// rocBLAS — matrix-vector multiply (out_dim == 1 fast path)
	pub fn rocblas_dgemv(
		handle: *mut c_void,
		trans: u32,
		m: i32,
		n: i32,
		alpha: *const f64,
		A: *const f64,
		lda: i32,
		x: *const f64,
		incx: i32,
		beta: *const f64,
		y: *mut f64,
		incy: i32,
	) -> i32;
	// rocBLAS — rank-1 update: A = alpha * x * yᵀ + A (column-major)
	pub fn rocblas_dger(
		handle: *mut c_void,
		m: i32,
		n: i32,
		alpha: *const f64,
		x: *const f64,
		incx: i32,
		y: *const f64,
		incy: i32,
		A: *mut f64,
		lda: i32,
	) -> i32;
}

pub fn mem_info() -> Result<(usize, usize), HipError> {
	let mut free: usize = 0;
	let mut total: usize = 0;
	check(unsafe { hipMemGetInfo(&mut free, &mut total) })?;
	Ok((free, total))
}

pub fn device_synchronize() -> Result<(), HipError> {
	check(unsafe { hipDeviceSynchronize() })
}

pub fn set_device(device: i32) -> Result<(), HipError> {
	check(unsafe { hipSetDevice(device) })
}

pub fn peek_last_error() -> i32 {
	unsafe { hipPeekAtLastError() }
}

pub fn device_count() -> Result<i32, HipError> {
	let mut count: i32 = 0;
	check(unsafe { hipGetDeviceCount(&mut count) })?;
	Ok(count)
}

pub fn device_attribute(attr: i32, device: i32) -> Result<i32, HipError> {
	let mut val: i32 = 0;
	check(unsafe { hipDeviceGetAttribute(&mut val, attr, device) })?;
	Ok(val)
}

pub unsafe fn memcpy_async(
	dst: *mut c_void,
	src: *const c_void,
	size: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipMemcpyAsync(dst, src, size, kind, stream) })
}

pub unsafe fn memset_async(
	dst: *mut c_void,
	value: i32,
	size: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipMemsetAsync(dst, value, size, stream) })
}

pub unsafe fn memcpy_dtod(
	dst: *mut c_void,
	src: *const c_void,
	size: usize,
) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipMemcpy(dst, src, size, HIP_MEMCPY_D2D) })
}

pub fn host_malloc(size: usize, flags: u32) -> Result<*mut c_void, HipError> {
	let mut ptr: *mut c_void = std::ptr::null_mut();
	check(unsafe { hipHostMalloc(&mut ptr, size, flags) })?;
	Ok(ptr)
}

pub unsafe fn host_free(ptr: *mut c_void) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipHostFree(ptr) })
}

pub unsafe fn host_register(ptr: *mut c_void, size: usize, flags: u32) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipHostRegister(ptr, size, flags) })
}

pub unsafe fn host_unregister(ptr: *mut c_void) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipHostUnregister(ptr) })
}

pub fn can_access_peer(device: i32, peer: i32) -> Result<bool, HipError> {
	let mut val: i32 = 0;
	check(unsafe { hipDeviceCanAccessPeer(&mut val, device, peer) })?;
	Ok(val != 0)
}

pub fn enable_peer_access(peer: i32, flags: u32) -> Result<(), HipError> {
	check(unsafe { hipDeviceEnablePeerAccess(peer, flags) })
}

pub unsafe fn memcpy_peer(
	dst: *mut c_void,
	dst_device: i32,
	src: *const c_void,
	src_device: i32,
	size: usize,
) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipMemcpyPeer(dst, dst_device, src, src_device, size) })
}

pub unsafe fn malloc_async(size: usize, stream: *mut c_void) -> Result<*mut c_void, HipError> {
	let mut ptr: *mut c_void = std::ptr::null_mut();
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipMallocAsync(&mut ptr, size, stream) })?;
	Ok(ptr)
}

pub unsafe fn free_async(ptr: *mut c_void, stream: *mut c_void) -> Result<(), HipError> {
	// SAFETY: FFI call — caller must ensure pointer validity and size.
	check(unsafe { hipFreeAsync(ptr, stream) })
}

/// RAII wrapper for a HIP stream.
pub struct Stream {
	raw: *mut c_void,
}

// SAFETY: HIP device pointers are thread-safe; the runtime serializes kernel launches per-stream.
unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}

impl Stream {
	pub fn new() -> Result<Self, HipError> {
		let mut raw: *mut c_void = std::ptr::null_mut();
		check(unsafe { hipStreamCreate(&mut raw) })?;
		Ok(Stream { raw })
	}

	pub fn raw(&self) -> *mut c_void {
		self.raw
	}

	pub fn synchronize(&self) -> Result<(), HipError> {
		check(unsafe { hipStreamSynchronize(self.raw) })
	}
}

impl Drop for Stream {
	fn drop(&mut self) {
		unsafe {
			hipStreamDestroy(self.raw);
		}
	}
}

/// RAII wrapper for a HIP event.
pub struct Event {
	raw: *mut c_void,
}

// SAFETY: HIP device pointers are thread-safe; the runtime serializes kernel launches per-stream.
unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
	pub fn new() -> Result<Self, HipError> {
		let mut raw: *mut c_void = std::ptr::null_mut();
		check(unsafe { hipEventCreate(&mut raw) })?;
		Ok(Event { raw })
	}

	pub unsafe fn record(&self, stream: *mut c_void) -> Result<(), HipError> {
		// SAFETY: FFI call — caller must ensure pointer validity and size.
		check(unsafe { hipEventRecord(self.raw, stream) })
	}

	pub fn synchronize(&self) -> Result<(), HipError> {
		check(unsafe { hipEventSynchronize(self.raw) })
	}
}

impl Drop for Event {
	fn drop(&mut self) {
		unsafe {
			hipEventDestroy(self.raw);
		}
	}
}

pub fn elapsed_ms(start: &Event, stop: &Event) -> Result<f32, HipError> {
	let mut ms: f32 = 0.0;
	check(unsafe { hipEventElapsedTime(&mut ms, start.raw, stop.raw) })?;
	Ok(ms)
}

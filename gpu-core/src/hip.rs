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

/// Multiprocessor count of the current device (hipGetDeviceProperties →
/// prop.multiProcessorCount), cached after the first query. Used to size GPU
/// launches to the real hardware instead of a hardcoded CU count. No fallback:
/// if the query fails (e.g. called before the device is initialized) it panics
/// with a clear cause rather than returning a silent wrong value.
pub fn cu_count() -> usize {
	use std::sync::atomic::{AtomicUsize, Ordering};
	static CU: AtomicUsize = AtomicUsize::new(0);
	let cached = CU.load(Ordering::Relaxed);
	if cached != 0 {
		return cached;
	}
	let n = unsafe { hip_multiprocessor_count() };
	assert!(
		n > 0,
		"hipGetDeviceProperties returned multiProcessorCount={n} — initialize the device (set_device) before sizing GPU launches"
	);
	CU.store(n as usize, Ordering::Relaxed);
	n as usize
}

pub const HIP_MEMCPY_H2D: i32 = 1;
pub const HIP_MEMCPY_D2H: i32 = 2;
pub const HIP_MEMCPY_D2D: i32 = 3;

unsafe extern "C" {
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
	pub(crate) fn hipMemcpyAsync(
		dst: *mut c_void,
		src: *const c_void,
		size: usize,
		kind: i32,
		stream: *mut c_void,
	) -> i32;
	pub fn hipMemsetAsync(dst: *mut c_void, value: i32, size: usize, stream: *mut c_void) -> i32;
	// Pinned host memory
	pub(crate) fn hipHostMalloc(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
	pub(crate) fn hipHostFree(ptr: *mut c_void) -> i32;
	pub fn hipHostRegister(ptr: *mut c_void, size: usize, flags: u32) -> i32;
	pub fn hipHostUnregister(ptr: *mut c_void) -> i32;
	// Device count and attributes
	// Note: hipDeviceProp_t is a ~800-byte struct; we expose hipDeviceGetAttribute instead
	// to avoid defining it in Rust and to allow querying individual fields by attribute enum int.
	pub fn hipGetDeviceCount(count: *mut i32) -> i32;
	pub fn hipDeviceGetAttribute(pi: *mut i32, attr: i32, device_id: i32) -> i32;
	// Defined in kernels/math.hip: hipGetDeviceProperties → prop.multiProcessorCount,
	// returned as a plain int so we never bind the hipDeviceProp_t struct in Rust.
	pub fn hip_multiprocessor_count() -> i32;
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
	pub(crate) fn hipMallocAsync(dev_ptr: *mut *mut c_void, size: usize, stream: *mut c_void) -> i32;
	pub(crate) fn hipFreeAsync(dev_ptr: *mut c_void, stream: *mut c_void) -> i32;
	pub fn hipDeviceGetDefaultMemPool(pool: *mut *mut c_void, device: i32) -> i32;
	pub fn hipMemPoolSetAttribute(pool: *mut c_void, attr: i32, value: *mut c_void) -> i32;
	pub fn hipMemPoolGetAttribute(pool: *mut c_void, attr: i32, value: *mut c_void) -> i32;
	pub fn hipMemPoolTrimTo(pool: *mut c_void, min_bytes_to_hold: usize) -> i32;
	// Managed (unified) memory
	pub fn hipMallocManaged(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
	// VRAM tier of the tiered buffer — VMM wrappers (src/kernels/vmm.hip). Handles
	// are opaque, carried as *mut c_void.
	pub fn vmm_granularity(out: *mut usize) -> i32;
	pub fn vmm_create(handle_out: *mut *mut c_void, size: usize) -> i32;
	pub fn vmm_reserve(va_out: *mut *mut c_void, size: usize) -> i32;
	pub fn vmm_map_at(va: *mut c_void, size: usize, handle: *mut c_void) -> i32;
	pub fn vmm_unmap(va: *mut c_void, size: usize) -> i32;
	pub fn vmm_release(handle: *mut c_void) -> i32;
	pub fn vmm_addr_free(va: *mut c_void, size: usize) -> i32;
	// hipBLAS — matrix-vector multiply (out_dim == 1 fast path)
	pub fn hipblasDgemv(
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
	// hipBLAS — rank-1 update: A = alpha * x * yᵀ + A (column-major)
	pub fn hipblasDger(
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

// The SDMA copy engine is incoherent with the gfx L2 on reused hipMallocAsync
// pool pages (ROCm 7.2.1 / gfx1101): an SDMA H2D lands in memory while compute
// reads stale L2 lines (silent wrong gemm results) or a stale mapping
// (intermittent "page not present" fault). Measured on inventory_proof: ~55%
// failure with SDMA, 8/8 clean without. Force blit-kernel copies — coherent
// with gfx L2 by construction — unless the user set the variable themselves.
// Must run before the first HIP call of the process; both GPU entry funnels
// (set_device, first allocation) call it.
pub(crate) fn disable_sdma_once() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		if std::env::var_os("HSA_ENABLE_SDMA").is_none() {
			// SAFETY: first GPU touch is effectively single-threaded, before HSA init.
			unsafe { std::env::set_var("HSA_ENABLE_SDMA", "0") };
		}
	});
}

pub fn set_device(device: i32) -> Result<(), HipError> {
	disable_sdma_once();
	check(unsafe { hipSetDevice(device) })
}

/// Make the default stream-ordered memory pool retain freed memory instead of
/// releasing it to the OS (release threshold = u64::MAX). Without this the pool
/// unmaps freed pages on sync and a later hipMallocAsync can hand back an
/// address whose backing is not yet remapped when a kernel touches it — an
/// intermittent GPU page fault under heavy alloc/free churn (weight streaming).
/// Bytes the default pool has reserved from the driver but not handed out —
/// growth for a new allocation comes on top of this, so a "how much can I
/// still ask for" computation must subtract it.
pub fn pool_slack(device: i32) -> Result<usize, HipError> {
	const RESERVED_MEM_CURRENT: i32 = 0x5;
	const USED_MEM_CURRENT: i32 = 0x7;
	let mut pool: *mut c_void = std::ptr::null_mut();
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	let mut reserved: u64 = 0;
	let mut used: u64 = 0;
	check(unsafe { hipMemPoolGetAttribute(pool, RESERVED_MEM_CURRENT, &mut reserved as *mut u64 as *mut c_void) })?;
	check(unsafe { hipMemPoolGetAttribute(pool, USED_MEM_CURRENT, &mut used as *mut u64 as *mut c_void) })?;
	Ok(reserved.saturating_sub(used) as usize)
}

/// Physical VRAM the kernel reports free across ALL clients (compositor
/// included) — `hipMemGetInfo` only sees KFD's own accounting, and an ask
/// beyond real physical free is an uncatchable `VmHeap::MapPhysMemory` abort,
/// so the slab pre-check needs the amdgpu sysfs ground truth.
pub fn sysfs_vram_free() -> Option<usize> {
	for card in std::fs::read_dir("/sys/class/drm").ok()? {
		let dev = card.ok()?.path().join("device");
		let read = |f: &str| -> Option<usize> {
			std::fs::read_to_string(dev.join(f)).ok()?.trim().parse().ok()
		};
		if let (Some(total), Some(used)) = (read("mem_info_vram_total"), read("mem_info_vram_used")) {
			return Some(total.saturating_sub(used));
		}
	}
	None
}

pub(crate) fn set_pool_retain(device: i32) -> Result<(), HipError> {
	const HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD: i32 = 4;
	let mut pool: *mut c_void = std::ptr::null_mut();
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	let mut threshold: u64 = u64::MAX;
	check(unsafe {
		hipMemPoolSetAttribute(
			pool,
			HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD,
			&mut threshold as *mut u64 as *mut c_void,
		)
	})
}

/// Pin the pool's release threshold and warm it (commit + retain a chunk) so no
/// async copy faults on an uncommitted page. Idempotent — funnels through the
/// same one-time warm that the first allocation triggers, so calling it from
/// `init()` and allocating without `init()` both warm the pool exactly once.
pub fn retain_mempool(_device: i32) -> Result<(), HipError> {
	crate::memory::ensure_pool_warmed();
	Ok(())
}

/// Release all retained pool VRAM back to the driver. Retention (threshold=max)
/// must not outlive the process: teardown reclaim is asynchronous, so a process
/// launched milliseconds later (cargo's next test binary) can touch pages whose
/// remap is still in flight — an intermittent gfxhub fault in the FIRST heavy
/// test of the next binary. Called from gpu_shutdown's atexit hook.
pub(crate) fn trim_mempool(device: i32) -> Result<(), HipError> {
	let mut pool: *mut c_void = std::ptr::null_mut();
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	check(unsafe { hipMemPoolTrimTo(pool, 0) })
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

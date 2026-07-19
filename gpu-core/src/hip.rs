use crate::callspy;
use crate::gate;
use crate::hw;
use crate::log::Write;
use core::cmp;
use core::error::Error;
use core::ffi::{CStr, c_void};
use core::fmt;
use core::mem;
use core::num::NonZeroUsize;
use core::ptr;
use core::time::Duration;
use std::env;
use std::fs;
use std::sync::Once;
use std::thread;

use crate::HipError;

impl fmt::Display for HipError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let &Self(code) = self;
		callspy::tick(&callspy::GET_ERROR_NAME);
		callspy::tick(&callspy::GET_ERROR_STRING);
		// SAFETY: hipGetErrorName accepts any status code and returns a static string pointer.
		let name_ptr = unsafe { hipGetErrorName(code) };
		// SAFETY: hipGetErrorString accepts any status code and returns a static string pointer.
		let str_ptr = unsafe { hipGetErrorString(code) };
		match ptr::NonNull::new(name_ptr.cast_mut()) {
			Some(name_nn) => match ptr::NonNull::new(str_ptr.cast_mut()) {
				Some(str_nn) => {
					// SAFETY: name_nn is non-null and points to a NUL-terminated C string.
					let name =
						unsafe { CStr::from_ptr(name_nn.as_ptr()) }.to_string_lossy();
					// SAFETY: str_nn is non-null and points to a NUL-terminated C string.
					let msg =
						unsafe { CStr::from_ptr(str_nn.as_ptr()) }.to_string_lossy();
					return write!(f, "{name}: {msg} (code {code})");
				}
				None => return write!(f, "HIP error code {code}"),
			},
			None => return write!(f, "HIP error code {code}"),
		}
	}
}
impl Error for HipError {}

#[inline]
pub const fn check(code: i32) -> Result<(), HipError> {
	match code {
		0 => return Ok(()),
		nonzero => return Err(HipError(nonzero)),
	}
}

#[inline]
pub fn cu_count() -> usize {
	use core::sync::atomic::{AtomicUsize, Ordering};
	static CU: AtomicUsize = AtomicUsize::new(0);
	return NonZeroUsize::new(CU.load(Ordering::Relaxed)).map_or_else(
		|| {
			gate::acquire();
			callspy::tick(&callspy::GET_DEVICE_PROPERTIES);
			// SAFETY: hip_multiprocessor_count takes no arguments and only reads the cached device multiprocessor count.
			let n = unsafe { hip_multiprocessor_count() };
			if n <= 0i32 {
				Write::error(format!(
					"hipGetDeviceProperties returned multiProcessorCount={n} — initialize the device (set_device) before sizing GPU launches"
				));
				return 0;
			}
			let count = usize::try_from(n).unwrap_or_else(|_| {
				Write::error(format!("multiProcessorCount={n} does not fit in usize"));
				return 0;
			});
			CU.store(count, Ordering::Relaxed);
			return count;
		},
		|cached| return cached.get(),
	);
}

pub fn gcn_arch() -> &'static str {
	use std::sync::OnceLock;
	static ARCH: OnceLock<String> = OnceLock::new();
	if let Some(a) = ARCH.get() {
		return a;
	}
	let mut buf = [0u8; 64];
	callspy::tick(&callspy::GET_DEVICE_PROPERTIES);
	// SAFETY: buf is a writable 64-byte out-slot; the shim NUL-terminates within cap.
	let n = unsafe { hip_gcn_arch(buf.as_mut_ptr().cast::<i8>(), 64) };
	if n <= 0i32 {
		return "";
	}
	let s = String::from_utf8_lossy(&buf[..n.unsigned_abs() as usize]).into_owned();
	return ARCH.get_or_init(|| return s);
}

pub fn mall_present() -> bool {
	let a = gcn_arch().strip_prefix("gfx").unwrap_or("");
	let digits: String = a.chars().take_while(char::is_ascii_digit).collect();
	return digits.parse::<u32>().is_ok_and(|n| return n >= 1030);
}

pub const HIP_MEMCPY_H2D: i32 = 1;
pub const HIP_MEMCPY_D2H: i32 = 2;
pub const HIP_MEMCPY_D2D: i32 = 3;

unsafe extern "C" {
	pub fn hip_gcn_arch(buf: *mut i8, cap: i32) -> i32;
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
	pub fn hipGetErrorName(error: i32) -> *const i8;
	pub fn hipGetErrorString(error: i32) -> *const i8;
	pub fn hipPeekAtLastError() -> i32;
	pub(crate) fn hipMemcpyAsync(
		dst: *mut c_void,
		src: *const c_void,
		size: usize,
		kind: i32,
		stream: *mut c_void,
	) -> i32;
	pub(crate) fn hipMemsetAsync(
		dst: *mut c_void,
		value: i32,
		size: usize,
		stream: *mut c_void,
	) -> i32;
	pub(crate) fn hipHostMalloc(ptr: *mut *mut c_void, size: usize, flags: u32) -> i32;
	pub(crate) fn hipHostFree(ptr: *mut c_void) -> i32;
	pub fn hipGetDeviceCount(count: *mut i32) -> i32;
	pub fn hipDeviceGetAttribute(pi: *mut i32, attr: i32, device_id: i32) -> i32;
	pub fn hip_multiprocessor_count() -> i32;
	pub fn hipDeviceCanAccessPeer(
		can_access_peer: *mut i32,
		device_id: i32,
		peer_device_id: i32,
	) -> i32;
	pub fn hipDeviceEnablePeerAccess(peer_device_id: i32, flags: u32) -> i32;
	pub(crate) fn hipMallocAsync(
		dev_ptr: *mut *mut c_void,
		size: usize,
		stream: *mut c_void,
	) -> i32;
	pub(crate) fn hipFreeAsync(dev_ptr: *mut c_void, stream: *mut c_void) -> i32;
	pub fn hipDeviceGetDefaultMemPool(pool: *mut *mut c_void, device: i32) -> i32;
	pub(crate) fn hipMemPoolSetAttribute(pool: *mut c_void, attr: i32, value: *mut c_void)
	-> i32;
	pub fn hipMemPoolGetAttribute(pool: *mut c_void, attr: i32, value: *mut c_void) -> i32;
	pub(crate) fn hipMemPoolTrimTo(pool: *mut c_void, min_bytes_to_hold: usize) -> i32;
	pub fn vmm_granularity(out: *mut usize) -> i32;
	pub fn vmm_create(handle_out: *mut *mut c_void, size: usize) -> i32;
	pub fn vmm_reserve(va_out: *mut *mut c_void, size: usize) -> i32;
	pub fn vmm_map_at(va: *mut c_void, size: usize, handle: *mut c_void) -> i32;
	pub fn vmm_unmap(va: *mut c_void, size: usize) -> i32;
	pub fn vmm_release(handle: *mut c_void) -> i32;
	pub fn vmm_addr_free(va: *mut c_void, size: usize) -> i32;
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

pub struct MemInfo {
	pub free: usize,
	pub total: usize,
}

#[inline]
pub fn mem_info() -> Result<MemInfo, HipError> {
	gate::acquire();
	let mut free: usize = 0;
	let mut total: usize = 0;
	callspy::tick(&callspy::MEM_GET_INFO);
	// SAFETY: free and total are initialized local usize slots that hipMemGetInfo writes.
	check(unsafe { hipMemGetInfo(&raw mut free, &raw mut total) })?;
	return Ok(MemInfo { free, total });
}

#[inline]
pub fn device_synchronize() -> Result<(), HipError> {
	gate::acquire();
	callspy::tick(&callspy::DEVICE_SYNCHRONIZE);
	// SAFETY: hipDeviceSynchronize takes no arguments and only blocks the host until the device is idle.
	return check(unsafe { hipDeviceSynchronize() });
}

pub(crate) fn disable_sdma_once() {
	static ONCE: Once = Once::new();
	ONCE.call_once(|| {
		if env::var_os("HSA_ENABLE_SDMA").is_none() {
			// SAFETY: runs once under Once during single-threaded init, before any thread reads the environment.
			unsafe {
				env::set_var("HSA_ENABLE_SDMA", "0");
			}
		}
	});
}

#[inline]
pub fn set_device(device: i32) -> Result<(), HipError> {
	gate::acquire();
	disable_sdma_once();
	register_fault_autopsy_once();
	hw::install_fast_death();
	callspy::tick(&callspy::SET_DEVICE);
	// SAFETY: device is a plain integer index passed by value; hipSetDevice reads no caller memory.
	return check(unsafe { hipSetDevice(device) });
}

#[repr(C)]
pub struct HsaAmdEvent {
	event_type: u32,
	agent: u64,
	virtual_address: u64,
	fault_reason_mask: u32,
}

struct FaultReason {
	bit: u32,
	name: &'static str,
}

#[inline]
pub unsafe extern "C" fn fault_autopsy(event: *const HsaAmdEvent, _data: *mut c_void) -> i32 {
	use crate::memory;
	// SAFETY: event is the non-null HsaAmdEvent pointer the HSA runtime passes to this registered handler.
	let e = unsafe { &*event };
	if e.event_type == 0 {
		const REASONS: [FaultReason; 8] = [
			FaultReason {
				bit: 1 << 0,
				name: "page-not-present",
			},
			FaultReason {
				bit: 1 << 1,
				name: "read-only",
			},
			FaultReason {
				bit: 1 << 2,
				name: "nx",
			},
			FaultReason {
				bit: 1 << 3,
				name: "host-only",
			},
			FaultReason {
				bit: 1 << 4,
				name: "dram-ecc",
			},
			FaultReason {
				bit: 1 << 5,
				name: "imprecise",
			},
			FaultReason {
				bit: 1 << 6,
				name: "sram-ecc",
			},
			FaultReason {
				bit: 1 << 31,
				name: "hang",
			},
		];
		let mut why = String::new();
		for reason in REASONS {
			if e.fault_reason_mask & reason.bit != 0 {
				if !why.is_empty() {
					why.push('+');
				}
				why.push_str(reason.name);
			}
		}
		let va = usize::try_from(e.virtual_address).unwrap_or_default();
		let placed = memory::bounce_range().map_or_else(
			|| return "bounce not yet allocated".to_owned(),
			|r| match va.cmp(&r.base) {
				cmp::Ordering::Less => {
					return format!("outside bounce (bounce base 0x{:x})", r.base);
				}
				cmp::Ordering::Equal | cmp::Ordering::Greater => match va
					.cmp(&(r.base + r.len))
				{
					cmp::Ordering::Less => {
						return format!(
							"INSIDE pinned h2d bounce (+0x{:x})",
							va - r.base
						);
					}
					cmp::Ordering::Equal | cmp::Ordering::Greater => {
						return format!("outside bounce (bounce base 0x{:x})", r.base);
					}
				},
			},
		);
		let located = memory::locate_va(va).map_or_else(
			|| return format!("{placed}; va in NO recorded allocation"),
			|hit| return format!("{placed}; {hit}"),
		);
		Write::error(format!(
			"gpu fault autopsy  va=0x{:x}  reason={why}  {located}\n{}",
			e.virtual_address,
			memory::ledger_report(),
		));

		thread::sleep(Duration::from_millis(150));
	}
	return 1;
}

pub(crate) fn register_fault_autopsy_once() {
	use core::sync::atomic::{AtomicUsize, Ordering};
	static REGISTERED: AtomicUsize = AtomicUsize::new(0);
	if REGISTERED.load(Ordering::Relaxed) == 0 {
		// SAFETY: dlsym takes RTLD_DEFAULT and a valid NUL-terminated C string; it returns a function pointer or null, and null is checked below.
		let sym = unsafe {
			libc::dlsym(
				libc::RTLD_DEFAULT,
				c"hsa_amd_register_system_event_handler".as_ptr(),
			)
		};
		if let Some(found) = ptr::NonNull::new(sym) {
			type Register = extern "C" fn(
				unsafe extern "C" fn(*const HsaAmdEvent, *mut c_void) -> i32,
				*mut c_void,
			) -> i32;
			// SAFETY: the resolved symbol is hsa_amd_register_system_event_handler, whose C ABI matches the Register fn-pointer type.
			let register = unsafe { mem::transmute::<*mut c_void, Register>(found.as_ptr()) };
			if register(fault_autopsy, ptr::null_mut()) == 0i32 {
				REGISTERED.store(1, Ordering::Relaxed);
			}
		}
	}
}

#[inline]
pub fn pool_slack(device: i32) -> Result<usize, HipError> {
	use crate::{callspy, gate};
	const RESERVED_MEM_CURRENT: i32 = 0x5;
	const USED_MEM_CURRENT: i32 = 0x7;
	gate::acquire();

	let mut pool: *mut c_void = ptr::null_mut();
	callspy::tick(&callspy::GET_DEFAULT_MEMPOOL);
	// SAFETY: pool is a valid local out-pointer and device is a valid ordinal; the call only writes the pool handle.
	check(unsafe { hipDeviceGetDefaultMemPool(&raw mut pool, device) })?;
	let mut reserved: u64 = 0;
	let mut used: u64 = 0;
	callspy::tick(&callspy::MEMPOOL_GET_ATTRIBUTE);
	// SAFETY: pool is the live default mempool and the out arg points at an owned u64 sized for the attribute.
	check(unsafe {
		hipMemPoolGetAttribute(
			pool,
			RESERVED_MEM_CURRENT,
			(&raw mut reserved).cast::<c_void>(),
		)
	})?;
	callspy::tick(&callspy::MEMPOOL_GET_ATTRIBUTE);
	// SAFETY: pool is the live default mempool and the out arg points at an owned u64 sized for the attribute.
	check(unsafe {
		hipMemPoolGetAttribute(pool, USED_MEM_CURRENT, (&raw mut used).cast::<c_void>())
	})?;
	return usize::try_from(reserved.saturating_sub(used)).map_err(|_err| return HipError(1));
}

#[inline]
#[must_use]
pub fn sysfs_vram_free() -> Option<usize> {
	for card in fs::read_dir("/sys/class/drm").ok()? {
		let dev = card.ok()?.path().join("device");
		let read = |f: &str| -> Option<usize> {
			return fs::read_to_string(dev.join(f)).ok()?.trim().parse().ok();
		};
		let total = read("mem_info_vram_total");
		let used = read("mem_info_vram_used");
		if let (Some(got_total), Some(got_used)) = (total, used) {
			return Some(got_total.saturating_sub(got_used));
		}
	}
	return None;
}

#[inline]
pub fn set_pool_retain(device: i32) -> Result<(), HipError> {
	const HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD: i32 = 4;
	let mut pool: *mut c_void = ptr::null_mut();
	callspy::tick(&callspy::GET_DEFAULT_MEMPOOL);
	// SAFETY: pool is a valid owned slot and hipDeviceGetDefaultMemPool writes the device's default pool handle.
	check(unsafe { hipDeviceGetDefaultMemPool(&raw mut pool, device) })?;
	let mut threshold = u64::MAX;
	callspy::tick(&callspy::MEMPOOL_SET_ATTRIBUTE);
	// SAFETY: pool is the live default mempool and the out arg points at an owned u64 holding the release threshold.
	return check(unsafe {
		hipMemPoolSetAttribute(
			pool,
			HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD,
			(&raw mut threshold).cast::<c_void>(),
		)
	});
}

#[inline]
pub fn retain_mempool(_device: i32) -> Result<(), HipError> {
	use crate::memory;
	memory::device_init_once();
	return Ok(());
}

pub(crate) fn trim_mempool(device: i32) -> Result<(), HipError> {
	let mut pool: *mut c_void = ptr::null_mut();
	callspy::tick(&callspy::GET_DEFAULT_MEMPOOL);
	// SAFETY: pool is a valid local out-pointer and device is a valid ordinal for the FFI query.
	check(unsafe { hipDeviceGetDefaultMemPool(&raw mut pool, device) })?;
	callspy::tick(&callspy::MEMPOOL_TRIM_TO);
	// SAFETY: pool was obtained from the default-mempool query above and is valid to trim.
	return check(unsafe { hipMemPoolTrimTo(pool, 0) });
}

#[inline]
#[must_use]
pub fn peek_last_error() -> i32 {
	callspy::tick(&callspy::PEEK_AT_LAST_ERROR);
	// SAFETY: hipPeekAtLastError only reads the thread-local last error and cannot fault.
	unsafe { return hipPeekAtLastError() }
}

#[inline]
pub fn device_count() -> Result<i32, HipError> {
	gate::acquire();
	let mut count: i32 = 0;
	callspy::tick(&callspy::GET_DEVICE_COUNT);
	// SAFETY: count is a valid local out-pointer for the FFI query.
	check(unsafe { hipGetDeviceCount(&raw mut count) })?;
	return Ok(count);
}

#[inline]
pub fn device_attribute(attr: i32, device: i32) -> Result<i32, HipError> {
	gate::acquire();
	let mut val: i32 = 0;
	callspy::tick(&callspy::DEVICE_GET_ATTRIBUTE);
	// SAFETY: val is a valid local out-pointer; attr and device are plain scalars for the FFI query.
	check(unsafe { hipDeviceGetAttribute(&raw mut val, attr, device) })?;
	return Ok(val);
}

#[inline]
pub fn host_malloc(size: usize, flags: u32) -> Result<*mut c_void, HipError> {
	use crate::{callspy, gate, memory};
	gate::acquire();
	let mut ptr: *mut c_void = ptr::null_mut();
	callspy::tick(&callspy::HOST_MALLOC);
	// SAFETY: ptr is a valid local out-pointer; hipHostMalloc writes the allocation handle into it.
	check(unsafe { hipHostMalloc(&raw mut ptr, size, flags) })?;
	memory::note_range(ptr.addr(), size, "pinned-host");
	return Ok(ptr);
}

#[inline]
pub unsafe fn host_free(ptr: *mut c_void) -> Result<(), HipError> {
	use crate::callspy;
	callspy::tick(&callspy::HOST_FREE);
	// SAFETY: ptr was returned by hipHostMalloc and is freed exactly once by the caller.
	return check(unsafe { hipHostFree(ptr) });
}

#[inline]
pub fn can_access_peer(device: i32, peer: i32) -> Result<bool, HipError> {
	let mut val: i32 = 0;
	callspy::tick(&callspy::DEVICE_CAN_ACCESS_PEER);
	// SAFETY: val is a valid local out-pointer the runtime writes the peer-access flag into.
	check(unsafe { hipDeviceCanAccessPeer(&raw mut val, device, peer) })?;
	return Ok(val != 0);
}

#[inline]
pub fn enable_peer_access(peer: i32, flags: u32) -> Result<(), HipError> {
	use crate::callspy;
	callspy::tick(&callspy::DEVICE_ENABLE_PEER_ACCESS);
	// SAFETY: peer is a caller-validated device ordinal; hipDeviceEnablePeerAccess reads only scalars.
	return check(unsafe { hipDeviceEnablePeerAccess(peer, flags) });
}

pub struct Stream {
	raw: *mut c_void,
}

// SAFETY: Stream owns an opaque HIP stream handle that is safe to move between threads.
unsafe impl Send for Stream {}
// SAFETY: HIP stream handles may be referenced concurrently; the runtime serializes access.
unsafe impl Sync for Stream {}

impl Stream {
	#[inline]
	pub fn new() -> Result<Self, HipError> {
		use crate::{callspy, gate};
		gate::acquire();
		let mut raw: *mut c_void = ptr::null_mut();
		callspy::tick(&callspy::STREAM_CREATE);
		// SAFETY: raw is a valid local out-pointer hipStreamCreate writes the new stream handle into.
		check(unsafe { hipStreamCreate(&raw mut raw) })?;
		return Ok(Self { raw });
	}

	#[must_use]
	#[inline]
	pub const fn raw(&self) -> *mut c_void {
		return self.raw;
	}

	#[inline]
	pub fn synchronize(&self) -> Result<(), HipError> {
		callspy::tick(&callspy::STREAM_SYNCHRONIZE);
		// SAFETY: self.raw is a live HIP stream handle owned by this Stream for its lifetime.
		return check(unsafe { hipStreamSynchronize(self.raw) });
	}
}

impl Drop for Stream {
	#[inline]
	fn drop(&mut self) {
		// SAFETY: self.raw is the live HIP stream handle owned by this Stream, destroyed exactly once here.
		unsafe {
			callspy::tick(&callspy::STREAM_DESTROY);
			hipStreamDestroy(self.raw);
		}
	}
}

pub struct Event {
	raw: *mut c_void,
}

// SAFETY: Event owns a raw HIP event handle with no thread-affine state, so moving it across threads is sound.
unsafe impl Send for Event {}
// SAFETY: shared access exposes only an immutable raw handle; concurrent device use is externally synchronized.
unsafe impl Sync for Event {}

impl Event {
	#[inline]
	pub fn new() -> Result<Self, HipError> {
		gate::acquire();
		let mut raw: *mut c_void = ptr::null_mut();
		callspy::tick(&callspy::EVENT_CREATE);
		// SAFETY: &raw mut raw is a valid out-pointer to a null-initialized handle that hipEventCreate fills.
		check(unsafe { hipEventCreate(&raw mut raw) })?;
		return Ok(Self { raw });
	}

	#[inline]
	pub unsafe fn record(&self, stream: *mut c_void) -> Result<(), HipError> {
		callspy::tick(&callspy::EVENT_RECORD);
		// SAFETY: self.raw is a live HIP event handle; stream is a caller-supplied valid HIP stream or null.
		return check(unsafe { hipEventRecord(self.raw, stream) });
	}

	#[inline]
	pub fn record_default(&self) -> Result<(), HipError> {
		// SAFETY: the null (default) stream is always a valid stream argument.
		return unsafe { self.record(ptr::null_mut()) };
	}

	#[inline]
	pub fn synchronize(&self) -> Result<(), HipError> {
		callspy::tick(&callspy::EVENT_SYNCHRONIZE);
		// SAFETY: self.raw is a live HIP event handle owned by this Event for its lifetime.
		return check(unsafe { hipEventSynchronize(self.raw) });
	}
}

impl Drop for Event {
	#[inline]
	fn drop(&mut self) {
		// SAFETY: self.raw is the live HIP event handle owned by this Event, destroyed exactly once here.
		unsafe {
			callspy::tick(&callspy::EVENT_DESTROY);
			hipEventDestroy(self.raw);
		}
	}
}

#[inline]
pub fn elapsed_ms(start: &Event, stop: &Event) -> Result<f32, HipError> {
	let mut ms: f32 = 0.0;
	callspy::tick(&callspy::EVENT_ELAPSED_TIME);
	// SAFETY: ms is a valid stack f32 out-param; start.raw and stop.raw are live event handles.
	check(unsafe { hipEventElapsedTime(&raw mut ms, start.raw, stop.raw) })?;
	return Ok(ms);
}

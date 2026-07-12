use std::ffi::{CStr, c_void};
use std::fmt;

#[derive(Debug)]
pub struct HipError(pub i32);

impl fmt::Display for HipError {
	fn fmt<'a>(&self, f: &mut fmt::Formatter<'a>) -> fmt::Result {
		let &HipError(code) = self;
		unsafe {
			crate::callspy::tick(&crate::callspy::GET_ERROR_NAME);
			crate::callspy::tick(&crate::callspy::GET_ERROR_STRING);
			let name_ptr = hipGetErrorName(code);
			let str_ptr = hipGetErrorString(code);
			match std::ptr::NonNull::new(name_ptr.cast_mut()) {
				Some(name_nn) => match std::ptr::NonNull::new(str_ptr.cast_mut()) {
					Some(str_nn) => {
						let name = CStr::from_ptr(name_nn.as_ptr()).to_string_lossy();
						let msg = CStr::from_ptr(str_nn.as_ptr()).to_string_lossy();
						write!(f, "{}: {} (code {})", name, msg, code)
					}
					None => write!(f, "HIP error code {}", code),
				},
				None => write!(f, "HIP error code {}", code),
			}
		}
	}
}
impl std::error::Error for HipError {}

pub fn check(code: i32) -> Result<(), HipError> {
	match code {
		0 => Ok(()),
		nonzero => Err(HipError(nonzero)),
	}
}

pub fn cu_count() -> usize {
	use std::sync::atomic::{AtomicUsize, Ordering};
	static CU: AtomicUsize = AtomicUsize::new(0);
	match std::num::NonZeroUsize::new(CU.load(Ordering::Relaxed)) {
		Some(cached) => cached.get(),
		None => {
			crate::gate::acquire();
			crate::callspy::tick(&crate::callspy::GET_DEVICE_PROPERTIES);
			let n = unsafe { hip_multiprocessor_count() };
			assert!(
				n > 0,
				"hipGetDeviceProperties returned multiProcessorCount={n} — initialize the device (set_device) before sizing GPU launches"
			);
			CU.store(n as usize, Ordering::Relaxed);
			n as usize
		}
	}
}

pub const HIP_MEMCPY_H2D: i32 = 1;
pub const HIP_MEMCPY_D2H: i32 = 2;
pub const HIP_MEMCPY_D2D: i32 = 3;

unsafe extern "C" {
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

pub fn mem_info() -> Result<MemInfo, HipError> {
	crate::gate::acquire();
	let mut free: usize = 0;
	let mut total: usize = 0;
	crate::callspy::tick(&crate::callspy::MEM_GET_INFO);
	check(unsafe { hipMemGetInfo(&mut free, &mut total) })?;
	Ok(MemInfo { free, total })
}

pub fn device_synchronize() -> Result<(), HipError> {
	crate::gate::acquire();
	crate::callspy::tick(&crate::callspy::DEVICE_SYNCHRONIZE);
	check(unsafe { hipDeviceSynchronize() })
}

pub(crate) fn disable_sdma_once() {
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		for _absent in Some(())
			.filter(|_u| std::env::var_os("HSA_ENABLE_SDMA").is_none())
			.into_iter()
		{
			unsafe { std::env::set_var("HSA_ENABLE_SDMA", "0") };
		}
	});
}

pub fn set_device(device: i32) -> Result<(), HipError> {
	crate::gate::acquire();
	disable_sdma_once();
	register_fault_autopsy_once();
	crate::hw::install_fast_death();
	crate::callspy::tick(&crate::callspy::SET_DEVICE);
	check(unsafe { hipSetDevice(device) })
}

#[repr(C)]
struct HsaAmdEvent {
	event_type: u32,
	agent: u64,
	virtual_address: u64,
	fault_reason_mask: u32,
}

struct FaultReason {
	bit: u32,
	name: &'static str,
}

extern "C" fn fault_autopsy(event: *const HsaAmdEvent, _data: *mut c_void) -> i32 {
	let e = unsafe { &*event };
	for _fault in Some(()).filter(|_u| e.event_type == 0).into_iter() {
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
			for _set in Some(())
				.filter(|_u| e.fault_reason_mask & reason.bit != 0)
				.into_iter()
			{
				for _sep in Some(()).filter(|_u| !why.is_empty()).into_iter() {
					why.push('+');
				}
				why.push_str(reason.name);
			}
		}
		let va = e.virtual_address as usize;
		let locate = match crate::memory::bounce_range() {
			Some(r) => match va.cmp(&r.base) {
				std::cmp::Ordering::Less => {
					format!("outside bounce (bounce base 0x{:x})", r.base)
				}
				std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
					match va.cmp(&(r.base + r.len)) {
						std::cmp::Ordering::Less => {
							format!("INSIDE pinned h2d bounce (+0x{:x})", va - r.base)
						}
						std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
							format!("outside bounce (bounce base 0x{:x})", r.base)
						}
					}
				}
			},
			None => "bounce not yet allocated".to_string(),
		};
		let locate = match crate::memory::locate_va(va) {
			Some(hit) => format!("{locate}; {hit}"),
			None => format!("{locate}; va in NO recorded allocation"),
		};
		crate::log::Write::err(&format!(
			"gpu fault autopsy  va=0x{:x}  reason={why}  {locate}\n{}",
			e.virtual_address,
			crate::memory::ledger_report(),
		));
		std::thread::sleep(std::time::Duration::from_millis(150));
	}
	1
}

pub(crate) fn register_fault_autopsy_once() {
	use std::sync::atomic::{AtomicUsize, Ordering};
	static REGISTERED: AtomicUsize = AtomicUsize::new(0);
	for _first in Some(())
		.filter(|_u| REGISTERED.load(Ordering::Relaxed) == 0)
		.into_iter()
	{
		let sym = unsafe {
			libc::dlsym(
				libc::RTLD_DEFAULT,
				c"hsa_amd_register_system_event_handler".as_ptr(),
			)
		};
		for found in std::ptr::NonNull::new(sym).into_iter() {
			type Register = extern "C" fn(
				extern "C" fn(*const HsaAmdEvent, *mut c_void) -> i32,
				*mut c_void,
			) -> i32;
			let register =
				unsafe { std::mem::transmute::<*mut c_void, Register>(found.as_ptr()) };
			for _ok in Some(())
				.filter(|_u| register(fault_autopsy, std::ptr::null_mut()) == 0)
				.into_iter()
			{
				REGISTERED.store(1, Ordering::Relaxed);
			}
		}
	}
}

pub fn pool_slack(device: i32) -> Result<usize, HipError> {
	crate::gate::acquire();
	const RESERVED_MEM_CURRENT: i32 = 0x5;
	const USED_MEM_CURRENT: i32 = 0x7;
	let mut pool: *mut c_void = std::ptr::null_mut();
	crate::callspy::tick(&crate::callspy::GET_DEFAULT_MEMPOOL);
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	let mut reserved: u64 = 0;
	let mut used: u64 = 0;
	crate::callspy::tick(&crate::callspy::MEMPOOL_GET_ATTRIBUTE);
	check(unsafe {
		hipMemPoolGetAttribute(
			pool,
			RESERVED_MEM_CURRENT,
			&mut reserved as *mut u64 as *mut c_void,
		)
	})?;
	crate::callspy::tick(&crate::callspy::MEMPOOL_GET_ATTRIBUTE);
	check(unsafe {
		hipMemPoolGetAttribute(pool, USED_MEM_CURRENT, &mut used as *mut u64 as *mut c_void)
	})?;
	Ok(reserved.saturating_sub(used) as usize)
}

pub fn sysfs_vram_free() -> Option<usize> {
	for card in std::fs::read_dir("/sys/class/drm").ok()? {
		let dev = card.ok()?.path().join("device");
		let read = |f: &str| -> Option<usize> {
			std::fs::read_to_string(dev.join(f))
				.ok()?
				.trim()
				.parse()
				.ok()
		};
		let total = read("mem_info_vram_total");
		let used = read("mem_info_vram_used");
		if let (Some(got_total), Some(got_used)) = (total, used) {
			return Some(got_total.saturating_sub(got_used));
		}
	}
	None
}

pub(crate) fn set_pool_retain(device: i32) -> Result<(), HipError> {
	const HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD: i32 = 4;
	let mut pool: *mut c_void = std::ptr::null_mut();
	crate::callspy::tick(&crate::callspy::GET_DEFAULT_MEMPOOL);
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	let mut threshold: u64 = u64::MAX;
	crate::callspy::tick(&crate::callspy::MEMPOOL_SET_ATTRIBUTE);
	check(unsafe {
		hipMemPoolSetAttribute(
			pool,
			HIP_MEM_POOL_ATTR_RELEASE_THRESHOLD,
			&mut threshold as *mut u64 as *mut c_void,
		)
	})
}

pub fn retain_mempool(_device: i32) -> Result<(), HipError> {
	crate::memory::device_init_once();
	Ok(())
}

pub(crate) fn trim_mempool(device: i32) -> Result<(), HipError> {
	let mut pool: *mut c_void = std::ptr::null_mut();
	crate::callspy::tick(&crate::callspy::GET_DEFAULT_MEMPOOL);
	check(unsafe { hipDeviceGetDefaultMemPool(&mut pool, device) })?;
	crate::callspy::tick(&crate::callspy::MEMPOOL_TRIM_TO);
	check(unsafe { hipMemPoolTrimTo(pool, 0) })
}

pub fn peek_last_error() -> i32 {
	crate::callspy::tick(&crate::callspy::PEEK_AT_LAST_ERROR);
	unsafe { hipPeekAtLastError() }
}

pub fn device_count() -> Result<i32, HipError> {
	crate::gate::acquire();
	let mut count: i32 = 0;
	crate::callspy::tick(&crate::callspy::GET_DEVICE_COUNT);
	check(unsafe { hipGetDeviceCount(&mut count) })?;
	Ok(count)
}

pub fn device_attribute(attr: i32, device: i32) -> Result<i32, HipError> {
	crate::gate::acquire();
	let mut val: i32 = 0;
	crate::callspy::tick(&crate::callspy::DEVICE_GET_ATTRIBUTE);
	check(unsafe { hipDeviceGetAttribute(&mut val, attr, device) })?;
	Ok(val)
}

pub fn host_malloc(size: usize, flags: u32) -> Result<*mut c_void, HipError> {
	crate::gate::acquire();
	let mut ptr: *mut c_void = std::ptr::null_mut();
	crate::callspy::tick(&crate::callspy::HOST_MALLOC);
	check(unsafe { hipHostMalloc(&mut ptr, size, flags) })?;
	crate::memory::note_range(ptr as usize, size, "pinned-host");
	Ok(ptr)
}

pub unsafe fn host_free(ptr: *mut c_void) -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::HOST_FREE);
	check(unsafe { hipHostFree(ptr) })
}

pub fn can_access_peer(device: i32, peer: i32) -> Result<bool, HipError> {
	let mut val: i32 = 0;
	crate::callspy::tick(&crate::callspy::DEVICE_CAN_ACCESS_PEER);
	check(unsafe { hipDeviceCanAccessPeer(&mut val, device, peer) })?;
	Ok(val != 0)
}

pub fn enable_peer_access(peer: i32, flags: u32) -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::DEVICE_ENABLE_PEER_ACCESS);
	check(unsafe { hipDeviceEnablePeerAccess(peer, flags) })
}

pub struct Stream {
	raw: *mut c_void,
}

unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}

impl Stream {
	pub fn new() -> Result<Self, HipError> {
		crate::gate::acquire();
		let mut raw: *mut c_void = std::ptr::null_mut();
		crate::callspy::tick(&crate::callspy::STREAM_CREATE);
		check(unsafe { hipStreamCreate(&mut raw) })?;
		Ok(Stream { raw })
	}

	pub fn raw(&self) -> *mut c_void {
		self.raw
	}

	pub fn synchronize(&self) -> Result<(), HipError> {
		crate::callspy::tick(&crate::callspy::STREAM_SYNCHRONIZE);
		check(unsafe { hipStreamSynchronize(self.raw) })
	}
}

impl Drop for Stream {
	fn drop(&mut self) {
		unsafe {
			crate::callspy::tick(&crate::callspy::STREAM_DESTROY);
			hipStreamDestroy(self.raw);
		}
	}
}

pub struct Event {
	raw: *mut c_void,
}

unsafe impl Send for Event {}
unsafe impl Sync for Event {}

impl Event {
	pub fn new() -> Result<Self, HipError> {
		crate::gate::acquire();
		let mut raw: *mut c_void = std::ptr::null_mut();
		crate::callspy::tick(&crate::callspy::EVENT_CREATE);
		check(unsafe { hipEventCreate(&mut raw) })?;
		Ok(Event { raw })
	}

	pub unsafe fn record(&self, stream: *mut c_void) -> Result<(), HipError> {
		crate::callspy::tick(&crate::callspy::EVENT_RECORD);
		check(unsafe { hipEventRecord(self.raw, stream) })
	}

	pub fn synchronize(&self) -> Result<(), HipError> {
		crate::callspy::tick(&crate::callspy::EVENT_SYNCHRONIZE);
		check(unsafe { hipEventSynchronize(self.raw) })
	}
}

impl Drop for Event {
	fn drop(&mut self) {
		unsafe {
			crate::callspy::tick(&crate::callspy::EVENT_DESTROY);
			hipEventDestroy(self.raw);
		}
	}
}

pub fn elapsed_ms(start: &Event, stop: &Event) -> Result<f32, HipError> {
	let mut ms: f32 = 0.0;
	crate::callspy::tick(&crate::callspy::EVENT_ELAPSED_TIME);
	check(unsafe { hipEventElapsedTime(&mut ms, start.raw, stop.raw) })?;
	Ok(ms)
}

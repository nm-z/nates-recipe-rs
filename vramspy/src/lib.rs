//! LD_PRELOAD interposer for exactly four HSA allocation entry points:
//! `hsa_amd_memory_pool_{allocate,free}` and `hsa_memory_{allocate,free}`.
//! Every allocation is classified by which AGENT owns its pool (device vs.
//! host), not by guessing from SEGMENT/GLOBAL_FLAGS alone — both device and
//! pinned-host pools report segment GLOBAL, so ownership is the only
//! reliable discriminator. Interposition works because the calling code
//! (libamdhip64.so, the vendor BLAS lib) is a separate DSO from
//! libhsa-runtime64.so and reaches these symbols through the dynamic symbol
//! table — LD_PRELOAD makes this library resolve first.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub const KIND_DEVICE: u8 = 0;
pub const KIND_HOST_PINNED: u8 = 1;
pub const KIND_KERNARG: u8 = 2;
pub const KIND_OTHER: u8 = 3;
const N_KINDS: usize = 4;

// hsa.h:126 — HSA_STATUS_SUCCESS = 0x0.
const HSA_STATUS_SUCCESS: i32 = 0;
// hsa.h:1026 — HSA_AGENT_INFO_DEVICE = 17; value type is hsa_device_type_t.
const HSA_AGENT_INFO_DEVICE: i32 = 17;
// hsa.h:808/812 — HSA_DEVICE_TYPE_CPU = 0, HSA_DEVICE_TYPE_GPU = 1.
const HSA_DEVICE_TYPE_GPU: u32 = 1;
// hsa_ext_amd.h:1403 — HSA_AMD_SEGMENT_GLOBAL = 0 (the only segment that ever
// holds a real device/host allocation; READONLY=1/PRIVATE=2/GROUP=3 do not).
const HSA_AMD_SEGMENT_GLOBAL: u32 = 0;
// hsa_ext_amd.h:1493 — HSA_AMD_MEMORY_POOL_INFO_SEGMENT = 0.
const HSA_AMD_MEMORY_POOL_INFO_SEGMENT: i32 = 0;
// hsa_ext_amd.h:1502 — HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS = 1.
const HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS: i32 = 1;
// hsa_ext_amd.h:1454 — HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT = 1 (bit 0).
// hsa_amd_segment_t (hsa_ext_amd.h:1402-1418) defines only 4 values (0-3); there
// is no segment value 4. Kernarg pools are segment GLOBAL(0) with this flag bit
// set — confirmed by reading the header, not assumed from the spec's "value 4".
const HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT: u32 = 1;

static LIVE: [AtomicU64; N_KINDS] =
	[AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static PEAK: [AtomicU64; N_KINDS] =
	[AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static ALLOCS: [AtomicU64; N_KINDS] =
	[AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static FREES: [AtomicU64; N_KINDS] =
	[AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
static UNKNOWN_FREES: AtomicU64 = AtomicU64::new(0);

// ptr -> (size, kind). Plain std Mutex/HashMap: the hooks never wrap malloc,
// so allocating inside a hook cannot recurse into a hook.
static MAP: OnceLock<Mutex<HashMap<usize, (u64, u8)>>> = OnceLock::new();

fn map() -> &'static Mutex<HashMap<usize, (u64, u8)>> {
	MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_map() -> std::sync::MutexGuard<'static, HashMap<usize, (u64, u8)>> {
	match map().lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	}
}

fn record_alloc(ptr: usize, size: u64, kind: u8) {
	let k = kind as usize;
	lock_map().insert(ptr, (size, kind));
	let live = LIVE[k].fetch_add(size, Ordering::Relaxed) + size;
	ALLOCS[k].fetch_add(1, Ordering::Relaxed);
	PEAK[k].fetch_max(live, Ordering::Relaxed);
}

fn record_free(ptr: usize) {
	match lock_map().remove(&ptr) {
		Some((size, kind)) => {
			let k = kind as usize;
			LIVE[k].fetch_sub(size, Ordering::Relaxed);
			FREES[k].fetch_add(1, Ordering::Relaxed);
		}
		None => {
			UNKNOWN_FREES.fetch_add(1, Ordering::Relaxed);
		}
	}
}

// ── real-symbol resolution (dlsym RTLD_NEXT, resolved once, abort on miss) ──

struct Real {
	pool_allocate: unsafe extern "C" fn(u64, usize, u32, *mut *mut c_void) -> i32,
	pool_free: unsafe extern "C" fn(*mut c_void) -> i32,
	mem_allocate: unsafe extern "C" fn(u64, usize, *mut *mut c_void) -> i32,
	mem_free: unsafe extern "C" fn(*mut c_void) -> i32,
}
// SAFETY: plain C function pointers into a shared library — safe to share across threads.
unsafe impl Sync for Real {}

fn resolve_next(name: &CStr) -> usize {
	// SAFETY: dlsym with a valid NUL-terminated name; RTLD_NEXT is well-defined
	// when this library was loaded via LD_PRELOAD.
	let p = unsafe { libc::dlsym(libc::RTLD_NEXT, name.as_ptr()) };
	if p.is_null() {
		eprintln!("vramspy: RTLD_NEXT resolution failed for {}", name.to_string_lossy());
		// SAFETY: abort() takes no arguments and never returns.
		unsafe { libc::abort() };
	}
	p as usize
}

fn real() -> &'static Real {
	static REAL: OnceLock<Real> = OnceLock::new();
	REAL.get_or_init(|| {
		// SAFETY: each transmute target is the exact C signature documented for
		// the resolved symbol; resolve_next() guarantees a non-null pointer.
		unsafe {
			Real {
				pool_allocate: std::mem::transmute(resolve_next(c"hsa_amd_memory_pool_allocate")),
				pool_free: std::mem::transmute(resolve_next(c"hsa_amd_memory_pool_free")),
				mem_allocate: std::mem::transmute(resolve_next(c"hsa_memory_allocate")),
				mem_free: std::mem::transmute(resolve_next(c"hsa_memory_free")),
			}
		}
	})
}

// ── pool classification: which agent (CPU/GPU) owns the pool ───────────────

fn resolve_next_or_default(name: &CStr) -> Option<usize> {
	// SAFETY: dlsym with a valid NUL-terminated name.
	let mut p = unsafe { libc::dlsym(libc::RTLD_NEXT, name.as_ptr()) };
	if p.is_null() {
		// SAFETY: same call, different pseudo-handle.
		p = unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
	}
	if p.is_null() { None } else { Some(p as usize) }
}

struct BuildCtx<'a> {
	pools: &'a mut HashMap<u64, u8>,
	agent_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
	iterate_pools: unsafe extern "C" fn(u64, extern "C" fn(u64, *mut c_void) -> i32, *mut c_void) -> i32,
	pool_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
}

struct PoolCtx<'a> {
	pools: &'a mut HashMap<u64, u8>,
	is_gpu: bool,
	pool_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
}

extern "C" fn pool_cb(pool: u64, data: *mut c_void) -> i32 {
	// SAFETY: data is a live &mut PoolCtx for the duration of the enclosing
	// hsa_amd_agent_iterate_memory_pools call (set up in pool_kinds).
	let ctx = unsafe { &mut *(data as *mut PoolCtx) };
	let mut segment: u32 = u32::MAX;
	// SAFETY: FFI query; segment is a valid out-param for the call's duration.
	let r1 = unsafe {
		(ctx.pool_get_info)(pool, HSA_AMD_MEMORY_POOL_INFO_SEGMENT, &mut segment as *mut u32 as *mut c_void)
	};
	if r1 != HSA_STATUS_SUCCESS || segment != HSA_AMD_SEGMENT_GLOBAL {
		ctx.pools.insert(pool, KIND_OTHER);
		return HSA_STATUS_SUCCESS;
	}
	let mut flags: u32 = 0;
	// SAFETY: FFI query; flags is a valid out-param for the call's duration.
	let r2 = unsafe {
		(ctx.pool_get_info)(pool, HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS, &mut flags as *mut u32 as *mut c_void)
	};
	let kernarg = r2 == HSA_STATUS_SUCCESS && (flags & HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT) != 0;
	let kind = if kernarg {
		KIND_KERNARG
	} else if ctx.is_gpu {
		KIND_DEVICE
	} else {
		KIND_HOST_PINNED
	};
	ctx.pools.insert(pool, kind);
	HSA_STATUS_SUCCESS
}

extern "C" fn agent_cb(agent: u64, data: *mut c_void) -> i32 {
	// SAFETY: data is a live &mut BuildCtx for the duration of the enclosing
	// hsa_iterate_agents call (set up in pool_kinds).
	let ctx = unsafe { &mut *(data as *mut BuildCtx) };
	let mut dev_type: u32 = u32::MAX;
	// SAFETY: FFI query; dev_type is a valid out-param for the call's duration.
	let r = unsafe { (ctx.agent_get_info)(agent, HSA_AGENT_INFO_DEVICE, &mut dev_type as *mut u32 as *mut c_void) };
	if r != HSA_STATUS_SUCCESS {
		return HSA_STATUS_SUCCESS; // skip this agent, keep iterating
	}
	let mut pool_ctx =
		PoolCtx { pools: ctx.pools, is_gpu: dev_type == HSA_DEVICE_TYPE_GPU, pool_get_info: ctx.pool_get_info };
	// SAFETY: FFI iterate call; pool_ctx outlives the call (stack frame does not
	// return until hsa_amd_agent_iterate_memory_pools does).
	unsafe { (ctx.iterate_pools)(agent, pool_cb, &mut pool_ctx as *mut PoolCtx as *mut c_void) };
	HSA_STATUS_SUCCESS
}

fn pool_kinds() -> &'static HashMap<u64, u8> {
	static MAP: OnceLock<HashMap<u64, u8>> = OnceLock::new();
	MAP.get_or_init(|| {
		let mut pools = HashMap::new();
		let (Some(ia), Some(agi), Some(ip), Some(pgi)) = (
			resolve_next_or_default(c"hsa_iterate_agents"),
			resolve_next_or_default(c"hsa_agent_get_info"),
			resolve_next_or_default(c"hsa_amd_agent_iterate_memory_pools"),
			resolve_next_or_default(c"hsa_amd_memory_pool_get_info"),
		) else {
			eprintln!("vramspy: agent/pool classification symbols unavailable — all pools classify OTHER");
			return pools;
		};
		// SAFETY: each transmute target is the exact C signature documented for
		// the resolved symbol.
		let iterate_agents: unsafe extern "C" fn(
			extern "C" fn(u64, *mut c_void) -> i32,
			*mut c_void,
		) -> i32 = unsafe { std::mem::transmute(ia) };
		let mut ctx = BuildCtx {
			pools: &mut pools,
			// SAFETY: same as above.
			agent_get_info: unsafe { std::mem::transmute(agi) },
			iterate_pools: unsafe { std::mem::transmute(ip) },
			pool_get_info: unsafe { std::mem::transmute(pgi) },
		};
		// SAFETY: agent_cb matches the callback signature hsa_iterate_agents expects;
		// ctx outlives this call.
		unsafe { iterate_agents(agent_cb, &mut ctx as *mut BuildCtx as *mut c_void) };
		pools
	})
}

fn classify_pool(pool: u64) -> u8 {
	*pool_kinds().get(&pool).unwrap_or(&KIND_OTHER)
}

// ── interposed entry points ─────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_amd_memory_pool_allocate(
	pool: u64,
	size: usize,
	flags: u32,
	ptr: *mut *mut c_void,
) -> i32 {
	// SAFETY: forwarding the caller's arguments unchanged to the real HSA runtime.
	let status = unsafe { (real().pool_allocate)(pool, size, flags, ptr) };
	if status == HSA_STATUS_SUCCESS {
		// SAFETY: status==SUCCESS guarantees the real allocator wrote a valid pointer.
		let p = unsafe { *ptr };
		if !p.is_null() {
			record_alloc(p as usize, size as u64, classify_pool(pool));
		}
	}
	status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_amd_memory_pool_free(ptr: *mut c_void) -> i32 {
	// SAFETY: forwarding the caller's pointer unchanged to the real HSA runtime.
	let status = unsafe { (real().pool_free)(ptr) };
	if status == HSA_STATUS_SUCCESS {
		record_free(ptr as usize);
	}
	status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_memory_allocate(region: u64, size: usize, ptr: *mut *mut c_void) -> i32 {
	// SAFETY: forwarding the caller's arguments unchanged to the real HSA runtime.
	let status = unsafe { (real().mem_allocate)(region, size, ptr) };
	if status == HSA_STATUS_SUCCESS {
		// SAFETY: status==SUCCESS guarantees the real allocator wrote a valid pointer.
		let p = unsafe { *ptr };
		if !p.is_null() {
			// Legacy region API: ROCr on AMD uses the pool API for everything that
			// matters, so this path is classified Other unconditionally (bytes
			// still counted).
			record_alloc(p as usize, size as u64, KIND_OTHER);
		}
	}
	status
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_memory_free(ptr: *mut c_void) -> i32 {
	// SAFETY: forwarding the caller's pointer unchanged to the real HSA runtime.
	let status = unsafe { (real().mem_free)(ptr) };
	if status == HSA_STATUS_SUCCESS {
		record_free(ptr as usize);
	}
	status
}

// ── C-ABI query surface (dlsym'd by gpu-core at report time) ────────────────

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_loaded() -> u32 {
	1
}

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_live(kind: u32) -> u64 {
	if kind > 3 { 0 } else { LIVE[kind as usize].load(Ordering::Relaxed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_peak(kind: u32) -> u64 {
	if kind > 3 { 0 } else { PEAK[kind as usize].load(Ordering::Relaxed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_allocs(kind: u32) -> u64 {
	if kind > 3 { 0 } else { ALLOCS[kind as usize].load(Ordering::Relaxed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_frees(kind: u32) -> u64 {
	if kind > 3 { 0 } else { FREES[kind as usize].load(Ordering::Relaxed) }
}

#[unsafe(no_mangle)]
pub extern "C" fn vramspy_unknown_frees() -> u64 {
	UNKNOWN_FREES.load(Ordering::Relaxed)
}

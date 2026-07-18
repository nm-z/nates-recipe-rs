#![allow(unsafe_code)]
#![allow(non_snake_case, reason = "interposed HSA symbols carry C ABI names")]
//! `LD_PRELOAD` interposer for exactly four HSA allocation entry points:
//! `hsa_amd_memory_pool_{allocate,free}` and `hsa_memory_{allocate,free}`.
//! Every allocation is classified by which AGENT owns its pool (device vs.
//! host), not by guessing from `SEGMENT/GLOBAL_FLAGS` alone — both device and
//! pinned-host pools report segment GLOBAL, so ownership is the only
//! reliable discriminator. Interposition works because the calling code
//! (libamdhip64.so, the vendor BLAS lib) is a separate DSO from
//! libhsa-runtime64.so and reaches these symbols through the dynamic symbol
//! table — `LD_PRELOAD` makes this library resolve first.


use core::ffi::{CStr, c_void};
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, Ordering};
use log::Write;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Device-owned VRAM.
pub const KIND_DEVICE: u8 = 0;
/// Pinned host memory.
pub const KIND_HOST_PINNED: u8 = 1;
/// Kernarg memory.
pub const KIND_KERNARG: u8 = 2;
/// Unclassified memory.
pub const KIND_OTHER: u8 = 3;
/// Number of allocation kinds tracked.
const N_KINDS: usize = 4;

/// `HSA_STATUS_SUCCESS`.
const HSA_STATUS_SUCCESS: i32 = 0;
/// `HSA_AGENT_INFO_DEVICE`; value type is `hsa_device_type_t`.
const HSA_AGENT_INFO_DEVICE: i32 = 17;
/// `HSA_DEVICE_TYPE_GPU`; CPU is 0.
const HSA_DEVICE_TYPE_GPU: u32 = 1;
/// `HSA_AMD_SEGMENT_GLOBAL`; the only segment holding real allocations.
const HSA_AMD_SEGMENT_GLOBAL: u32 = 0;
/// `HSA_AMD_MEMORY_POOL_INFO_SEGMENT`.
const HSA_AMD_MEMORY_POOL_INFO_SEGMENT: i32 = 0;
/// `HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS`.
const HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS: i32 = 1;
/// `HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT` bit.
const HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT: u32 = 1;

/// Live bytes per kind.
static LIVE: [AtomicU64; N_KINDS] = [
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
];
/// Peak bytes per kind.
static PEAK: [AtomicU64; N_KINDS] = [
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
];
/// Allocation count per kind.
static ALLOCS: [AtomicU64; N_KINDS] = [
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
];
/// Free count per kind.
static FREES: [AtomicU64; N_KINDS] = [
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
	AtomicU64::new(0),
];
/// Frees of pointers never recorded.
static UNKNOWN_FREES: AtomicU64 = AtomicU64::new(0);

/// A recorded allocation.
struct Alloc {
	/// Byte size.
	size: u64,
	/// Kind index.
	kind: u8,
}

/// Pointer-to-record map.
static MAP: OnceLock<Mutex<HashMap<usize, Alloc>>> = OnceLock::new();

/// Locks the allocation map, recovering from poisoning.
fn lock_map() -> MutexGuard<'static, HashMap<usize, Alloc>> {
	let guard = match MAP.get_or_init(|| return Mutex::new(HashMap::new())).lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	return guard;
}

/// Records an allocation of `size` bytes at `ptr` classified as `kind`.
fn record_alloc(ptr: usize, size: usize, kind: u8) {
	let k = usize::from(kind);
	let bytes = u64::try_from(size).unwrap_or(u64::MAX);
	lock_map().insert(ptr, Alloc { size: bytes, kind });
	let live = LIVE[k].fetch_add(bytes, Ordering::Relaxed) + bytes;
	ALLOCS[k].fetch_add(1, Ordering::Relaxed);
	PEAK[k].fetch_max(live, Ordering::Relaxed);
}

/// Records the free of `ptr`.
fn record_free(ptr: usize) {
	let removed = lock_map().remove(&ptr);
	match removed {
		Some(a) => {
			let k = usize::from(a.kind);
			LIVE[k].fetch_sub(a.size, Ordering::Relaxed);
			FREES[k].fetch_add(1, Ordering::Relaxed);
		}
		None => {
			UNKNOWN_FREES.fetch_add(1, Ordering::Relaxed);
		}
	}
}

/// Resolved real HSA allocation entry points.
struct Real {
	/// Real `hsa_amd_memory_pool_allocate`.
	pool_allocate: unsafe extern "C" fn(u64, usize, u32, *mut *mut c_void) -> i32,
	/// Real `hsa_amd_memory_pool_free`.
	pool_free: unsafe extern "C" fn(*mut c_void) -> i32,
	/// Real `hsa_memory_allocate`.
	mem_allocate: unsafe extern "C" fn(u64, usize, *mut *mut c_void) -> i32,
	/// Real `hsa_memory_free`.
	mem_free: unsafe extern "C" fn(*mut c_void) -> i32,
}
// SAFETY: plain C function pointers into a shared library, safe to share across threads.
unsafe impl Sync for Real {}

/// Resolves `name` via `RTLD_NEXT`, returning 0 on miss (never aborts the host).
fn resolve_next(name: &CStr) -> usize {
	// SAFETY: dlsym with a valid NUL-terminated name; RTLD_NEXT is well-defined under LD_PRELOAD.
	let p = unsafe { libc::dlsym(libc::RTLD_NEXT, name.as_ptr()) };
	let Some(nn) = NonNull::new(p) else {
		Write::error(format!(
			"vramspy: RTLD_NEXT resolution failed for {}",
			name.to_string_lossy()
		));
		return 0;
	};
	return nn.as_ptr().addr();
}

/// Real entry points, resolved once.
fn real() -> &'static Real {
	static REAL: OnceLock<Real> = OnceLock::new();
	return REAL.get_or_init(|| {
		// SAFETY: transmute to the exact documented C signature; resolve_next is non-null.
		let pool_allocate = unsafe {
			mem::transmute::<
				usize,
				unsafe extern "C" fn(u64, usize, u32, *mut *mut c_void) -> i32,
			>(resolve_next(c"hsa_amd_memory_pool_allocate"))
		};
		// SAFETY: transmute to the exact documented C signature; resolve_next is non-null.
		let pool_free = unsafe {
			mem::transmute::<usize, unsafe extern "C" fn(*mut c_void) -> i32>(resolve_next(
				c"hsa_amd_memory_pool_free",
			))
		};
		// SAFETY: transmute to the exact documented C signature; resolve_next is non-null.
		let mem_allocate = unsafe {
			mem::transmute::<usize, unsafe extern "C" fn(u64, usize, *mut *mut c_void) -> i32>(
				resolve_next(c"hsa_memory_allocate"),
			)
		};
		// SAFETY: transmute to the exact documented C signature; resolve_next is non-null.
		let mem_free = unsafe {
			mem::transmute::<usize, unsafe extern "C" fn(*mut c_void) -> i32>(resolve_next(
				c"hsa_memory_free",
			))
		};
		return Real {
			pool_allocate,
			pool_free,
			mem_allocate,
			mem_free,
		};
	});
}

/// Resolves `name` via `RTLD_NEXT` then `RTLD_DEFAULT`.
fn resolve_next_or_default(name: &CStr) -> Option<usize> {
	// SAFETY: dlsym with a valid NUL-terminated name.
	let p = unsafe { libc::dlsym(libc::RTLD_NEXT, name.as_ptr()) };
	let q = NonNull::new(p).map_or_else(
		|| {
			// SAFETY: same call, different pseudo-handle.
			return unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) };
		},
		|nn| return nn.as_ptr(),
	);
	return NonNull::new(q).map(|nn| return nn.as_ptr().addr());
}

/// Owning-agent class of a pool.
enum DevKind {
	/// GPU agent.
	Gpu,
	/// Any non-GPU agent.
	Other,
}

/// Agent-iteration context.
struct BuildCtx<'ctx> {
	/// Accumulated pool classifications.
	pools: &'ctx mut HashMap<u64, u8>,
	/// Real `hsa_agent_get_info`.
	agent_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
	/// Real `hsa_amd_agent_iterate_memory_pools`.
	iterate_pools:
		unsafe extern "C" fn(u64, extern "C" fn(u64, *mut c_void) -> i32, *mut c_void) -> i32,
	/// Real `hsa_amd_memory_pool_get_info`.
	pool_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
}

/// Pool-iteration context for one agent.
struct PoolCtx<'ctx> {
	/// Accumulated pool classifications.
	pools: &'ctx mut HashMap<u64, u8>,
	/// Owning-agent class.
	dev: DevKind,
	/// Real `hsa_amd_memory_pool_get_info`.
	pool_get_info: unsafe extern "C" fn(u64, i32, *mut c_void) -> i32,
}

/// Resolved classification symbol addresses.
struct Syms {
	/// `hsa_iterate_agents`.
	ia: usize,
	/// `hsa_agent_get_info`.
	agi: usize,
	/// `hsa_amd_agent_iterate_memory_pools`.
	ip: usize,
	/// `hsa_amd_memory_pool_get_info`.
	pgi: usize,
}

/// Classifies one pool into its iteration context.
#[inline]
pub extern "C" fn pool_cb(pool: u64, data: *mut c_void) -> i32 {
	// SAFETY: data is a live &mut PoolCtx for the duration of the enclosing iterate call.
	let ctx = unsafe { &mut *data.cast::<PoolCtx>() };
	let mut segment = u32::MAX;
	// SAFETY: FFI query; segment is a valid out-param for the call's duration.
	let r1 = unsafe {
		(ctx.pool_get_info)(
			pool,
			HSA_AMD_MEMORY_POOL_INFO_SEGMENT,
			(&raw mut segment).cast::<c_void>(),
		)
	};
	let kind = if r1 == HSA_STATUS_SUCCESS && segment == HSA_AMD_SEGMENT_GLOBAL {
		let mut flags = 0u32;
		// SAFETY: FFI query; flags is a valid out-param for the call's duration.
		let r2 = unsafe {
			(ctx.pool_get_info)(
				pool,
				HSA_AMD_MEMORY_POOL_INFO_GLOBAL_FLAGS,
				(&raw mut flags).cast::<c_void>(),
			)
		};
		if r2 == HSA_STATUS_SUCCESS
			&& (flags & HSA_AMD_MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT) != 0
		{
			KIND_KERNARG
		} else {
			match ctx.dev {
				DevKind::Gpu => KIND_DEVICE,
				DevKind::Other => KIND_HOST_PINNED,
			}
		}
	} else {
		KIND_OTHER
	};
	ctx.pools.insert(pool, kind);
	return HSA_STATUS_SUCCESS;
}

/// Iterates one agent's pools, classifying each.
#[inline]
pub extern "C" fn agent_cb(agent: u64, data: *mut c_void) -> i32 {
	// SAFETY: data is a live &mut BuildCtx for the duration of the enclosing iterate call.
	let ctx = unsafe { &mut *data.cast::<BuildCtx>() };
	let mut dev_type = u32::MAX;
	// SAFETY: FFI query; dev_type is a valid out-param for the call's duration.
	let r = unsafe {
		(ctx.agent_get_info)(
			agent,
			HSA_AGENT_INFO_DEVICE,
			(&raw mut dev_type).cast::<c_void>(),
		)
	};
	let Some(dt) = (r == HSA_STATUS_SUCCESS).then_some(dev_type) else {
		return HSA_STATUS_SUCCESS;
	};
	let dev = if dt == HSA_DEVICE_TYPE_GPU {
		DevKind::Gpu
	} else {
		DevKind::Other
	};
	let mut pool_ctx = PoolCtx {
		pools: ctx.pools,
		dev,
		pool_get_info: ctx.pool_get_info,
	};
	// SAFETY: FFI iterate call; pool_ctx outlives the call.
	unsafe {
		(ctx.iterate_pools)(agent, pool_cb, (&raw mut pool_ctx).cast::<c_void>());
	}
	return HSA_STATUS_SUCCESS;
}

/// # Safety
///
/// `ptr` must satisfy the real `hsa_amd_memory_pool_allocate` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_amd_memory_pool_allocate(
	pool: u64,
	size: usize,
	flags: u32,
	ptr: *mut *mut c_void,
) -> i32 {
	static POOLS: OnceLock<HashMap<u64, u8>> = OnceLock::new();
	// SAFETY: forwarding the caller's arguments unchanged to the real HSA runtime.
	let status = unsafe { (real().pool_allocate)(pool, size, flags, ptr) };
	if status != HSA_STATUS_SUCCESS {
		return status;
	}
	// SAFETY: status==SUCCESS guarantees the real allocator wrote a valid pointer.
	let p = unsafe { *ptr };
	let Some(nn) = NonNull::new(p) else {
		return status;
	};
	let kinds = POOLS.get_or_init(|| {
		let mut pools = HashMap::new();
		let Some(syms) = (|| {
			return Some(Syms {
				ia: resolve_next_or_default(c"hsa_iterate_agents")?,
				agi: resolve_next_or_default(c"hsa_agent_get_info")?,
				ip: resolve_next_or_default(c"hsa_amd_agent_iterate_memory_pools")?,
				pgi: resolve_next_or_default(c"hsa_amd_memory_pool_get_info")?,
			});
		})() else {
			Write::error(
				"vramspy: agent/pool classification symbols unavailable \u{2014} all pools classify OTHER",
			);
			return pools;
		};
		// SAFETY: transmute to the exact documented C signature.
		let iterate_agents = unsafe {
			mem::transmute::<
				usize,
				unsafe extern "C" fn(
					extern "C" fn(u64, *mut c_void) -> i32,
					*mut c_void,
				) -> i32,
			>(syms.ia)
		};
		// SAFETY: transmute to the exact documented C signature.
		let agent_get_info = unsafe {
			mem::transmute::<usize, unsafe extern "C" fn(u64, i32, *mut c_void) -> i32>(
				syms.agi,
			)
		};
		// SAFETY: transmute to the exact documented C signature.
		let iterate_pools = unsafe {
			mem::transmute::<
				usize,
				unsafe extern "C" fn(
					u64,
					extern "C" fn(u64, *mut c_void) -> i32,
					*mut c_void,
				) -> i32,
			>(syms.ip)
		};
		// SAFETY: transmute to the exact documented C signature.
		let pool_get_info = unsafe {
			mem::transmute::<usize, unsafe extern "C" fn(u64, i32, *mut c_void) -> i32>(
				syms.pgi,
			)
		};
		let mut ctx = BuildCtx {
			pools: &mut pools,
			agent_get_info,
			iterate_pools,
			pool_get_info,
		};
		// SAFETY: agent_cb matches the callback signature; ctx outlives the call.
		unsafe {
			iterate_agents(agent_cb, (&raw mut ctx).cast::<c_void>());
		}
		return pools;
	});
	let kind = *kinds.get(&pool).unwrap_or(&KIND_OTHER);
	record_alloc(nn.as_ptr().addr(), size, kind);
	return status;
}

/// # Safety
///
/// `ptr` must satisfy the real `hsa_amd_memory_pool_free` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_amd_memory_pool_free(ptr: *mut c_void) -> i32 {
	// SAFETY: forwarding the caller's pointer unchanged to the real HSA runtime.
	let status = unsafe { (real().pool_free)(ptr) };
	if status != HSA_STATUS_SUCCESS {
		return status;
	}
	record_free(ptr.addr());
	return status;
}

/// # Safety
///
/// `ptr` must satisfy the real `hsa_memory_allocate` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_memory_allocate(
	region: u64,
	size: usize,
	ptr: *mut *mut c_void,
) -> i32 {
	// SAFETY: forwarding the caller's arguments unchanged to the real HSA runtime.
	let status = unsafe { (real().mem_allocate)(region, size, ptr) };
	if status != HSA_STATUS_SUCCESS {
		return status;
	}
	// SAFETY: status==SUCCESS guarantees the real allocator wrote a valid pointer.
	let p = unsafe { *ptr };
	let Some(nn) = NonNull::new(p) else {
		return status;
	};
	record_alloc(nn.as_ptr().addr(), size, KIND_OTHER);
	return status;
}

/// # Safety
///
/// `ptr` must satisfy the real `hsa_memory_free` contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsa_memory_free(ptr: *mut c_void) -> i32 {
	// SAFETY: forwarding the caller's pointer unchanged to the real HSA runtime.
	let status = unsafe { (real().mem_free)(ptr) };
	if status != HSA_STATUS_SUCCESS {
		return status;
	}
	record_free(ptr.addr());
	return status;
}

/// Whether the interposer is loaded.
#[unsafe(no_mangle)]
pub const extern "C" fn vramspy_loaded() -> u32 {
	return 1;
}

/// Loads an atomic counter for `kind`.
fn load_kind(arr: &[AtomicU64; N_KINDS], kind: u32) -> u64 {
	let Ok(i) = usize::try_from(kind) else {
		return 0;
	};
	return arr.get(i).map_or(0, |a| return a.load(Ordering::Relaxed));
}

/// Live bytes for `kind`.
#[unsafe(no_mangle)]
pub extern "C" fn vramspy_live(kind: u32) -> u64 {
	return load_kind(&LIVE, kind);
}

/// Peak bytes for `kind`.
#[unsafe(no_mangle)]
pub extern "C" fn vramspy_peak(kind: u32) -> u64 {
	return load_kind(&PEAK, kind);
}

/// Allocation count for `kind`.
#[unsafe(no_mangle)]
pub extern "C" fn vramspy_allocs(kind: u32) -> u64 {
	return load_kind(&ALLOCS, kind);
}

/// Free count for `kind`.
#[unsafe(no_mangle)]
pub extern "C" fn vramspy_frees(kind: u32) -> u64 {
	return load_kind(&FREES, kind);
}

/// Count of frees of unrecorded pointers.
#[unsafe(no_mangle)]
pub extern "C" fn vramspy_unknown_frees() -> u64 {
	return UNKNOWN_FREES.load(Ordering::Relaxed);
}

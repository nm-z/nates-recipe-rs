use crate::hip::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

// Cumulative device-pool alloc/free counts (the two choke sites below). Never
// reset — they answer "how many live device buffers" at any point.
static ALLOC_TOTAL: AtomicUsize = AtomicUsize::new(0);
static FREE_TOTAL: AtomicUsize = AtomicUsize::new(0);

// Cumulative transfer ledger, bumped by the ONE xfer site before each copy is
// enqueued. Bytes + call count per direction — the exact "how much moved and
// which way" that the OOM autopsy and ledger_report read back.
static H2D_BYTES: AtomicUsize = AtomicUsize::new(0);
static D2H_BYTES: AtomicUsize = AtomicUsize::new(0);
static D2D_BYTES: AtomicUsize = AtomicUsize::new(0);
static H2D_CALLS: AtomicUsize = AtomicUsize::new(0);
static D2H_CALLS: AtomicUsize = AtomicUsize::new(0);
static D2D_CALLS: AtomicUsize = AtomicUsize::new(0);

// When set, drain the device after each hipMallocAsync so the pool commits the
// new buffer's pages before anything (notably an SDMA host->device copy, which
// runs on a queue not ordered with the alloc) writes into it. Off by default —
// training uploads once and never churns; streaming inference (fresh alloc +
// immediate copy, thousands of times) needs it to avoid GPU page faults.
static ALLOC_SYNC: AtomicBool = AtomicBool::new(false);

/// Enable/disable the post-allocation device sync (see `ALLOC_SYNC`).
pub fn set_alloc_sync(on: bool) {
	ALLOC_SYNC.store(on, Ordering::Relaxed);
}

// Live device bytes per purpose-tag, and the high-water peak per tag. On a VRAM
// OOM we dump the live map so the failure names what is on the card (data /
// weights / scratch / tiered-vram / other), not just a size; peak survives frees
// so ledger_report shows the worst-case footprint each tag ever reached.
static TAG_BYTES: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
static TAG_PEAK: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());

thread_local! {
	static CURRENT_TAG: Cell<&'static str> = const { Cell::new("other") };
}

/// Sets the purpose-tag for every allocation made while it is alive; restores
/// the previous tag on drop. Wrap an allocation phase: `let _t = tag_scope("data");`.
pub struct TagScope(&'static str);

pub fn tag_scope(name: &'static str) -> TagScope {
	let prev = CURRENT_TAG.with(|t| t.replace(name));
	TagScope(prev)
}

impl Drop for TagScope {
	fn drop(&mut self) {
		CURRENT_TAG.with(|t| t.set(self.0));
	}
}

fn tag_add(tag: &'static str, n: usize) {
	let live = if let Ok(mut m) = TAG_BYTES.lock() {
		let e = m.entry(tag).or_insert(0);
		*e += n;
		*e
	} else {
		return;
	};
	if let Ok(mut p) = TAG_PEAK.lock() {
		let e = p.entry(tag).or_insert(0);
		if live > *e {
			*e = live;
		}
	}
}

fn tag_sub(tag: &'static str, n: usize) {
	if let Ok(mut m) = TAG_BYTES.lock() {
		let e = m.entry(tag).or_insert(0);
		*e = e.saturating_sub(n);
	}
}

/// Record a device allocation the choke point cannot see — the tiered buffer's
/// VMM-mapped VRAM handles, which live outside the stream-ordered pool. Bytes
/// still land in the same live/peak ledger under their tag.
pub(crate) fn tag_note_alloc(tag: &'static str, n: usize) {
	tag_add(tag, n);
}

pub(crate) fn tag_note_free(tag: &'static str, n: usize) {
	tag_sub(tag, n);
}

fn fmt_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	if f >= K * K * K {
		format!("{:.2} GB", f / (K * K * K))
	} else if f >= K * K {
		format!("{:.2} MB", f / (K * K))
	} else if f >= K {
		format!("{:.2} KB", f / K)
	} else {
		format!("{b} B")
	}
}

fn oom_pair(name: &str, val: &str) -> String {
	format!("\x1b[1;31m{name}:\x1b[0m \x1b[1m{val}\x1b[0m")
}

// One-line VRAM autopsy: live tags largest-first, then request/free/total/over,
// followed by the cumulative transfer totals (how much has moved each way).
fn oom_report(req: usize) {
	let (free, total) = crate::hip::mem_info().unwrap_or((0, 0));
	let mut autopsy: Vec<(&'static str, usize)> = TAG_BYTES
		.lock()
		.map(|m| m.iter().map(|(k, v)| (*k, *v)).filter(|(_, v)| *v > 0).collect())
		.unwrap_or_default();
	autopsy.sort_by(|a, b| b.1.cmp(&a.1));
	let mut line: Vec<String> = autopsy.iter().map(|(k, v)| oom_pair(k, &fmt_bytes(*v))).collect();
	line.push(oom_pair("req", &fmt_bytes(req)));
	line.push(oom_pair("free", &fmt_bytes(free)));
	line.push(oom_pair("total", &fmt_bytes(total)));
	line.push(oom_pair("over", &fmt_bytes(req.saturating_sub(free))));
	eprintln!("{}", line.join(", "));
	eprintln!(
		"{}, {}, {}",
		oom_pair("H2D", &fmt_bytes(H2D_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2H", &fmt_bytes(D2H_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2D", &fmt_bytes(D2D_BYTES.load(Ordering::Relaxed))),
	);
}

// ── Global (cross-library) tracking via the vramspy LD_PRELOAD shim ────────
// The choke points above see only bytes this crate's own code path allocates.
// The HIP/HSA runtime and the vendor BLAS lib allocate more underneath them
// (code objects, workspaces, queues) that never touches this file. vramspy
// interposes the four HSA allocation entry points and exposes per-kind
// counters via a C ABI, present only when the process was started under
// `LD_PRELOAD=libvramspy.so`. Resolved by name (RTLD_DEFAULT) at report time
// so this crate never links against vramspy directly.

/// Kernel-attributed VRAM/GTT (KiB, summed) from `/proc/self/fdinfo/*`. Every
/// DRM fd carries a `drm-client-id:` line; the same client appears under
/// multiple fds (dup'd handles), so entries are deduped by that id before
/// summing `drm-memory-vram:` / `drm-memory-gtt:` — the ground truth
/// vramspy's userspace count must reconcile against. `None` means the
/// directory itself was unreadable (not "no GPU fd open", which is a
/// legitimate all-zero result).
fn kernel_fdinfo() -> Option<(u64, u64)> {
	let entries = std::fs::read_dir("/proc/self/fdinfo").ok()?;
	let mut by_client: BTreeMap<String, (u64, u64)> = BTreeMap::new();
	for e in entries.flatten() {
		let Ok(info) = std::fs::read_to_string(e.path()) else {
			continue;
		};
		let mut client_id: Option<String> = None;
		let mut vram_kib = 0u64;
		let mut gtt_kib = 0u64;
		for line in info.lines() {
			if let Some(v) = line.strip_prefix("drm-client-id:") {
				client_id = Some(v.trim().to_string());
			} else if let Some(v) = line.strip_prefix("drm-memory-vram:") {
				vram_kib = v.trim().trim_end_matches("KiB").trim().parse().unwrap_or(0);
			} else if let Some(v) = line.strip_prefix("drm-memory-gtt:") {
				gtt_kib = v.trim().trim_end_matches("KiB").trim().parse().unwrap_or(0);
			}
		}
		if let Some(id) = client_id {
			by_client.entry(id).or_insert((vram_kib, gtt_kib));
		}
	}
	let vram_total: u64 = by_client.values().map(|(v, _)| v).sum();
	let gtt_total: u64 = by_client.values().map(|(_, g)| g).sum();
	Some((vram_total, gtt_total))
}

/// Exact device-memory ledger as a human table: live + peak bytes per purpose
/// tag, cumulative transfer bytes/calls per direction, and device alloc/free
/// counts. One call answers "how many GBs and for exactly what".
pub fn ledger_report() -> String {
	let mut live: Vec<(&'static str, usize)> = TAG_BYTES
		.lock()
		.map(|m| m.iter().map(|(k, v)| (*k, *v)).collect())
		.unwrap_or_default();
	live.sort_by(|a, b| b.1.cmp(&a.1));
	let peak = TAG_PEAK.lock().map(|m| m.clone()).unwrap_or_default();
	let mut s = String::from("──────── GPU MEMORY LEDGER ────────\n");
	let mut total_live = 0usize;
	for (tag, v) in &live {
		total_live += *v;
		let pk = peak.get(tag).copied().unwrap_or(0);
		s += &format!("  {tag:<14} live {:>11}  peak {:>11}\n", fmt_bytes(*v), fmt_bytes(pk));
	}
	s += &format!("  {:<14} live {:>11}\n", "TOTAL", fmt_bytes(total_live));
	s += &format!(
		"  transfers  H2D {} ({} calls)  D2H {} ({} calls)  D2D {} ({} calls)\n",
		fmt_bytes(H2D_BYTES.load(Ordering::Relaxed)),
		H2D_CALLS.load(Ordering::Relaxed),
		fmt_bytes(D2H_BYTES.load(Ordering::Relaxed)),
		D2H_CALLS.load(Ordering::Relaxed),
		fmt_bytes(D2D_BYTES.load(Ordering::Relaxed)),
		D2D_CALLS.load(Ordering::Relaxed),
	);
	let (a, f) = (ALLOC_TOTAL.load(Ordering::Relaxed), FREE_TOTAL.load(Ordering::Relaxed));
	s += &format!("  device     allocs {a}  frees {f}  live-buffers {}\n", a.saturating_sub(f));

	// SAFETY: RTLD_DEFAULT + a literal NUL-terminated name is always well-defined.
	let live_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_live".as_ptr()) };
	if live_sym.is_null() {
		s += "  global: vramspy NOT loaded (LD_PRELOAD libvramspy.so)\n";
	} else {
		let sym = |name: &std::ffi::CStr| -> *mut c_void {
			// SAFETY: RTLD_DEFAULT + a valid NUL-terminated name.
			unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
		};
		let to_u32_u64 = |p: *mut c_void| -> extern "C" fn(u32) -> u64 {
			// SAFETY: p was resolved from a vramspy_* symbol documented with this signature.
			unsafe { std::mem::transmute::<*mut c_void, extern "C" fn(u32) -> u64>(p) }
		};
		let live_fn = to_u32_u64(live_sym);
		let peak_fn = to_u32_u64(sym(c"vramspy_peak"));
		let allocs_fn = to_u32_u64(sym(c"vramspy_allocs"));
		let frees_fn = to_u32_u64(sym(c"vramspy_frees"));
		// SAFETY: same as above, different fixed signature (no kind argument).
		let unknown_frees_fn = unsafe {
			std::mem::transmute::<*mut c_void, extern "C" fn() -> u64>(sym(c"vramspy_unknown_frees"))
		};

		s += "  global (vramspy, every byte incl. runtime+libs)\n";
		let mut device_live = 0u64;
		for (kind, label) in [(0u32, "device"), (1, "pinned"), (2, "kernarg"), (3, "other")] {
			let (live, peak, al, fr) = (live_fn(kind), peak_fn(kind), allocs_fn(kind), frees_fn(kind));
			if kind == 0 {
				device_live = live;
			}
			s += &format!(
				"    {label:<10} live {:>11}  peak {:>11}  (allocs {al} frees {fr})\n",
				fmt_bytes(live as usize),
				fmt_bytes(peak as usize)
			);
		}
		let delta = (device_live as usize).saturating_sub(total_live);
		s += &format!("    library delta: {} (unknown frees: {})\n", fmt_bytes(delta), unknown_frees_fn());
	}

	match kernel_fdinfo() {
		Some((vram_kib, gtt_kib)) => {
			s += &format!(
				"  kernel (fdinfo)  vram {}  gtt {}\n",
				fmt_bytes((vram_kib * 1024) as usize),
				fmt_bytes((gtt_kib * 1024) as usize)
			);
		}
		None => {
			s += "  kernel (fdinfo): unreadable\n";
		}
	}
	s += "───────────────────────────────────";
	s
}

pub fn mark_shutting_down() {
      SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

thread_local! {
	static ALLOC_FROZEN: Cell<bool> = const { Cell::new(false) };
}

pub fn alloc_count_reset() -> usize {
	ALLOC_COUNT.swap(0, Ordering::Relaxed)
}

/// Cumulative count of real device-pool allocations (`hipMallocAsync`) since
/// process start. Steady-state proof for the streaming forward: identical before
/// step 0 and after the last step ⇒ zero allocations churned in the hot loop.
pub fn device_alloc_count() -> usize {
	ALLOC_TOTAL.load(Ordering::Relaxed)
}

pub fn device_free_count() -> usize {
	FREE_TOTAL.load(Ordering::Relaxed)
}

pub fn alloc_freeze() {
	ALLOC_FROZEN.with(|f| f.set(true));
}

pub fn alloc_unfreeze() {
	ALLOC_FROZEN.with(|f| f.set(false));
}

pub struct AllocGuard(std::marker::PhantomData<*const ()>);

impl AllocGuard {
      pub fn freeze() -> Self {
            alloc_freeze();
            AllocGuard(std::marker::PhantomData)
      }
}

impl Drop for AllocGuard {
      fn drop(&mut self) {
            alloc_unfreeze();
      }
}

const ARENA_ALIGN: usize = 256;
static ARENA_BASE: AtomicUsize = AtomicUsize::new(0);
static ARENA_SIZE: AtomicUsize = AtomicUsize::new(0);
static ARENA_OFFSET: AtomicUsize = AtomicUsize::new(0);

// ── The three device choke points ───────────────────────────────────────────
// Exactly one hipMemcpyAsync call, one hipMemsetAsync call, one hipMallocAsync
// call (in alloc_bytes), and one hipFreeAsync call (in Drop) exist in the whole
// codebase — all below. Every byte that moves or lives on the card passes here.

/// THE single hipMemcpyAsync call site. Counts the transfer (bytes + calls, by
/// direction) into the ledger, then enqueues it on `stream` — async, no host
/// sync. The streaming-inference path calls this directly and syncs on its own
/// schedule; the blocking `*_sync` shim below adds a default-stream wait.
pub unsafe fn xfer(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	let (b, c) = match kind {
		HIP_MEMCPY_H2D => (&H2D_BYTES, &H2D_CALLS),
		HIP_MEMCPY_D2H => (&D2H_BYTES, &D2H_CALLS),
		_ => (&D2D_BYTES, &D2D_CALLS),
	};
	b.fetch_add(bytes, Ordering::Relaxed);
	c.fetch_add(1, Ordering::Relaxed);
	if kind == HIP_MEMCPY_H2D {
		// H2D goes through a pinned bounce: with SDMA disabled (gfx-L2 staleness
		// on reused pool pages — see hip::disable_sdma_once) the blit engine does
		// the copy, and blit reads of PAGEABLE host memory fault on unmapped host
		// pages under large streamed uploads. Staging through pinned memory is
		// the sanctioned path for either engine. The bounce holds its lock across
		// the stream sync of each chunk so the arena is never overwritten while a
		// copy is in flight — H2D is therefore always synchronous.
		return unsafe { h2d_pinned(dst, src, bytes, stream) };
	}
	unsafe { dev_copy(dst, src, bytes, kind, stream) }
}

/// THE raw device-copy call — the only `hipMemcpyAsync` in the codebase. Both
/// the counted `xfer` paths above funnel here; nothing else may call it.
unsafe fn dev_copy(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	// SAFETY: caller guarantees dst/src validity and that both span `bytes`.
	crate::callspy::tick(&crate::callspy::MEMCPY_ASYNC);
	check(unsafe { hipMemcpyAsync(dst, src, bytes, kind, stream) })
}

// Host-side copies saturate all cores: a single-thread memcpy runs ~5 GB/s
// while 12 threads reach ~4x that — whenever RAM is moving, CPU is at 100%.
/// Fault a fresh allocation's pages in on every core (one write per 4 KiB
/// page) — lazy zero-pages otherwise materialize serially on first touch.
pub fn par_touch(v: &mut [u8]) {
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
	let per = v.len().div_ceil(threads).div_ceil(4096) * 4096;
	std::thread::scope(|sc| {
		for ch in v.chunks_mut(per.max(4096)) {
			sc.spawn(|| {
				for i in (0..ch.len()).step_by(4096) {
					unsafe { std::ptr::write_volatile(ch.as_mut_ptr().add(i), 0) };
				}
			});
		}
	});
}

pub fn par_copy(dst: *mut u8, src: *const u8, bytes: usize) {
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
	let per = bytes.div_ceil(threads);
	let (d, s0) = (dst as usize, src as usize);
	std::thread::scope(|sc| {
		for t in 0..threads {
			let off = t * per;
			if off >= bytes {
				break;
			}
			let len = per.min(bytes - off);
			sc.spawn(move || unsafe {
				std::ptr::copy_nonoverlapping((s0 as *const u8).add(off), (d as *mut u8).add(off), len);
			});
		}
	});
}

const BOUNCE_BYTES: usize = 64 << 20;
static BOUNCE: Mutex<usize> = Mutex::new(0);

/// Explicitly release the pinned bounce at shutdown (exit frees ALL RAM).
pub(crate) fn free_bounce() {
	let mut guard = match BOUNCE.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if *guard != 0 {
		let _ = unsafe { crate::hip::host_free(*guard as *mut c_void) };
		*guard = 0;
	}
}

unsafe fn h2d_pinned(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	let mut guard = match BOUNCE.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if *guard == 0 {
		*guard = crate::hip::host_malloc(BOUNCE_BYTES, 0)? as usize;
	}
	let pin = *guard as *mut u8;
	let mut done = 0usize;
	while done < bytes {
		let chunk = BOUNCE_BYTES.min(bytes - done);
		// SAFETY: caller guarantees src spans `bytes`; pin spans BOUNCE_BYTES.
		par_copy(pin, unsafe { (src as *const u8).add(done) }, chunk);
		unsafe {
			dev_copy(
				(dst as *mut u8).add(done) as *mut c_void,
				pin as *const c_void,
				chunk,
				HIP_MEMCPY_H2D,
				stream,
			)
		}?;
		crate::callspy::tick(&crate::callspy::STREAM_SYNCHRONIZE);
		check(unsafe { hipStreamSynchronize(stream) })?;
		done += chunk;
	}
	Ok(())
}

/// Blocking transfer: enqueue on the default stream, then wait on that stream —
/// the drop-in for the old synchronous `hipMemcpy`. Fresh buffers come from the
/// warmed, page-committed pool (see `ensure_pool_warmed`), so the async SDMA copy
/// never touches an uncommitted page.
pub(crate) unsafe fn xfer_sync(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
) -> Result<(), HipError> {
	// Synchronous D2H to pageable memory otherwise crawls through the driver's
	// single-threaded internal staging — bounce it through the pinned arena and
	// fan the host copy across all cores. The ASYNC D2H path (deferred metric
	// scalars, one sync per epoch) is untouched.
	if kind == HIP_MEMCPY_D2H {
		D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
		D2H_CALLS.fetch_add(1, Ordering::Relaxed);
		let mut guard = match BOUNCE.lock() {
			Ok(g) => g,
			Err(p) => p.into_inner(),
		};
		if *guard == 0 {
			*guard = crate::hip::host_malloc(BOUNCE_BYTES, 0)? as usize;
		}
		let pin = *guard as *mut u8;
		let mut done = 0usize;
		while done < bytes {
			let chunk = BOUNCE_BYTES.min(bytes - done);
			// SAFETY: caller guarantees src spans `bytes`; pin spans BOUNCE_BYTES.
			unsafe {
				dev_copy(
					pin as *mut c_void,
					(src as *const u8).add(done) as *const c_void,
					chunk,
					HIP_MEMCPY_D2H,
					std::ptr::null_mut(),
				)
			}?;
			crate::callspy::tick(&crate::callspy::STREAM_SYNCHRONIZE);
			check(unsafe { hipStreamSynchronize(std::ptr::null_mut()) })?;
			par_copy(unsafe { (dst as *mut u8).add(done) }, pin, chunk);
			done += chunk;
		}
		return Ok(());
	}
	// SAFETY: forwarded from the caller's validated pointers.
	unsafe { xfer(dst, src, bytes, kind, std::ptr::null_mut()) }?;
	crate::callspy::tick(&crate::callspy::STREAM_SYNCHRONIZE);
	check(unsafe { hipStreamSynchronize(std::ptr::null_mut()) })
}


/// THE single hipMemsetAsync call site. Enqueues on `stream`, no host sync.
pub(crate) unsafe fn memset_dev(
	dst: *mut c_void,
	value: i32,
	bytes: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	// SAFETY: caller guarantees dst spans `bytes`.
	crate::callspy::tick(&crate::callspy::MEMSET_ASYNC);
	check(unsafe { hipMemsetAsync(dst, value, bytes, stream) })
}

/// Blocking device memset: enqueue then wait — drop-in for the old `hipMemset`.
pub(crate) unsafe fn memset_sync(dst: *mut c_void, value: i32, bytes: usize) -> Result<(), HipError> {
	// SAFETY: forwarded from the caller's validated pointer.
	unsafe { memset_dev(dst, value, bytes, std::ptr::null_mut()) }?;
	crate::callspy::tick(&crate::callspy::STREAM_SYNCHRONIZE);
	check(unsafe { hipStreamSynchronize(std::ptr::null_mut()) })
}

static POOL_INIT: std::sync::Once = std::sync::Once::new();

thread_local! {
	// Set only on the thread currently running the warm, so its own re-entrant
	// 1 GiB alloc skips the warm. Other threads must NOT skip — they block in
	// call_once until the warm finishes, then allocate from a warmed pool.
	static WARMING: Cell<bool> = const { Cell::new(false) };
}

/// Warm the stream-ordered pool exactly once — on the first allocation of the
/// process, or when `init()` calls `retain_mempool`, whichever comes first. Pins
/// the pool's release threshold and force-commits a chunk so that async device
/// copies (the only copy primitive now) never touch an uncommitted page and
/// fault "page not present". Re-entrant-safe: the warm's own 1 GiB alloc re-enters
/// `alloc_bytes` → here, but the thread-local WARMING guard short-circuits it.
pub(crate) fn ensure_pool_warmed() {
	if POOL_INIT.is_completed() || WARMING.with(|w| w.get()) {
		return;
	}
	POOL_INIT.call_once(|| {
		crate::hip::disable_sdma_once();
		if WARM_SKIP.load(Ordering::Relaxed) {
			return;
		}
		WARMING.with(|w| w.set(true));
		if let Err(e) = crate::hip::set_pool_retain(0).and_then(|_| warm_pool()) {
			eprintln!("GPU pool warm failed: {e}");
		}
		WARMING.with(|w| w.set(false));
	});
}

static WARM_SKIP: AtomicBool = AtomicBool::new(false);

// Bytes currently held by pool (owned) allocations, and the co-mapped total a
// disposable child has PROVEN the system can hold alongside this process.
static POOL_LIVE: AtomicUsize = AtomicUsize::new(0);
static POOL_VERIFIED: AtomicUsize = AtomicUsize::new(0);

/// One-claim lifecycle mode (the waterfall claim is the process's ONLY pool
/// allocation): skip the churn-protection warm — there is no churn to protect,
/// and the retained warm gigabyte would just shrink the claim.
pub fn skip_pool_warm() {
	WARM_SKIP.store(true, Ordering::Relaxed);
}

/// Hand the process its ONE pre-claimed device block: every subsequent
/// `GpuBuffer` allocation carves from it (non-owning, no pool traffic) until
/// it is exhausted. init → one claim, exit → one free — the lifecycle law.
/// The backing allocation must be tagged "unclaimed"; carves move their bytes
/// from that tag to the caller's current tag so the ledger stays exact.
pub fn set_device_arena(base: *mut c_void, size: usize) {
	ARENA_OFFSET.store(0, Ordering::Relaxed);
	ARENA_SIZE.store(size, Ordering::Relaxed);
	ARENA_BASE.store(base as usize, Ordering::Relaxed);
}

/// Bytes still carvable from the claimed block (0 when no claim is active).
pub fn arena_remaining() -> usize {
	if ARENA_BASE.load(Ordering::Relaxed) == 0 {
		return 0;
	}
	ARENA_SIZE.load(Ordering::Relaxed).saturating_sub(ARENA_OFFSET.load(Ordering::Relaxed))
}

/// Force-commit a 1 GiB buffer (so its pages are backed), zero it, then free it.
/// With the pool's release threshold pinned the pages stay resident, so later
/// allocs reuse already-mapped memory. Runs through the choke points against the
/// ledger under tag "warmup"; freed immediately.
pub(crate) fn warm_pool() -> Result<(), HipError> {
	let _t = tag_scope("warmup");
	let warm: usize = 1usize << 30; // 1 GiB
	let buf = GpuBuffer::alloc_bytes(warm)?;
	buf.memset_zero(warm)?;
	crate::hip::device_synchronize()?;
	drop(buf);
	crate::hip::device_synchronize()
}

pub struct GpuBuffer {
	pub(crate) ptr: *mut c_void,
	len: usize,
	owned: bool,
	tag: &'static str,
}

// SAFETY: HIP device pointers are thread-safe; the runtime serializes kernel launches per-stream.
unsafe impl Send for GpuBuffer {}
unsafe impl Sync for GpuBuffer {}

impl GpuBuffer {
	pub fn borrow(ptr: *mut c_void, len: usize) -> Self {
		Self {
			ptr,
			len,
			owned: false,
			tag: "borrow",
		}
	}

	pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
		Self::alloc_bytes(n_floats * std::mem::size_of::<f64>())
	}

	/// Waterfall probe: allocate or quietly refuse. A `None` is the "VRAM is
	/// full" signal the waterfall fills against — no autopsy spam, no error.
	/// Routes through the same single `hipMallocAsync` site as `alloc_bytes`.
	pub fn try_alloc_bytes(n_bytes: usize) -> Option<Self> {
		match Self::alloc_bytes_inner(n_bytes, true) {
			Ok(buf) => Some(buf),
			Err(_) => None,
		}
	}

	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		Self::alloc_bytes_inner(n_bytes, false)
	}

	fn alloc_bytes_inner(n_bytes: usize, quiet_oom: bool) -> Result<Self, HipError> {
		ensure_pool_warmed();
		ALLOC_FROZEN.with(|f| {
			assert!(
				!f.get(),
				"GPU allocation inside frozen training loop (requested {n_bytes} bytes)"
			)
		});
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(|t| t.get());
		let base = ARENA_BASE.load(Ordering::Relaxed);
		if base != 0 {
			let size = ARENA_SIZE.load(Ordering::Relaxed);
			let aligned = (n_bytes + ARENA_ALIGN - 1) & !(ARENA_ALIGN - 1);
			let mut off = ARENA_OFFSET.load(Ordering::Relaxed);
			while off + aligned <= size {
				match ARENA_OFFSET.compare_exchange_weak(
					off,
					off + aligned,
					Ordering::Relaxed,
					Ordering::Relaxed,
				) {
					Ok(_) => {
						let ptr = unsafe { (base as *mut u8).add(off) as *mut c_void };
						tag_sub("unclaimed", aligned);
						tag_add(tag, n_bytes);
						return Ok(Self {
							ptr,
							len: n_bytes,
							owned: false,
							tag,
						});
					}
					Err(cur) => off = cur,
				}
			}
		}
		// Growing the pool past what has been PROVEN co-mappable risks the
		// driver's uncatchable VmHeap::MapPhysMemory assert. Two regimes, two
		// checks, both required before the map is attempted:
		//   1. Counters (heavy-parent regime): the growth cannot exceed
		//      min(hip_free, sysfs_free) − pool_slack. A fresh probe process
		//      would pass here anyway — its mapping gets eviction-assisted —
		//      so counters, not a probe, are the honest signal once this
		//      process is the one holding the card.
		//   2. Probe (fresh/light regime): counters over-report near the
		//      ceiling for big asks, so a disposable child takes the assert
		//      risk and its exit status is the verdict.
		// Re-asks under the proven peak skip both — the pool never shrinks
		// mid-run (retain threshold = max), so no new mapping is needed.
		let live = POOL_LIVE.load(Ordering::Relaxed);
		let projected = live + n_bytes;
		if projected > POOL_VERIFIED.load(Ordering::Relaxed) {
			let hip_free = crate::hip::mem_info().map(|(f, _)| f).unwrap_or(0);
			let sys_free = crate::hip::sysfs_vram_free().unwrap_or(hip_free);
			let slack = crate::hip::pool_slack(0).unwrap_or(0);
			let remaining = hip_free.min(sys_free).saturating_sub(slack);
			// THE law: 1 GB of VRAM belongs to the user, always. Growth is
			// refused inside that band — which also covers the counters'
			// over-report of the true VmHeap ceiling (observed well under
			// 1 GB), so no per-ask probe children, no ratios, no guesses.
			if n_bytes > remaining.saturating_sub(1 << 30) {
				if !quiet_oom {
					oom_report(n_bytes);
				}
				return Err(HipError(2));
			}
		}
		let mut ptr: *mut c_void = std::ptr::null_mut();
		crate::callspy::tick(&crate::callspy::MALLOC_ASYNC);
		let code = unsafe { hipMallocAsync(&mut ptr, n_bytes, std::ptr::null_mut()) };
		if code == 2 && !quiet_oom {
			oom_report(n_bytes);
		}
		check(code)?;
		ALLOC_TOTAL.fetch_add(1, Ordering::Relaxed);
		if ALLOC_SYNC.load(Ordering::Relaxed) {
			crate::callspy::tick(&crate::callspy::DEVICE_SYNCHRONIZE);
			check(unsafe { hipDeviceSynchronize() })?;
		}
		tag_add(tag, n_bytes);
		let live = POOL_LIVE.fetch_add(n_bytes, Ordering::Relaxed) + n_bytes;
		POOL_VERIFIED.fetch_max(live, Ordering::Relaxed);
		Ok(Self {
			ptr,
			len: n_bytes,
			owned: true,
			tag,
		})
	}

	pub fn upload(data: &[f64]) -> Result<Self, HipError> {
		let buf = Self::alloc(data.len())?;
		let bytes = std::mem::size_of_val(data);
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	pub fn upload_u8(data: &[u8]) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(data.len())?;
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, data.len(), HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	/// Copy host bytes into this (already-allocated) buffer — the reuse path for
	/// a persistent staging window, avoiding a fresh alloc per upload.
	pub fn write_u8(&self, data: &[u8]) -> Result<(), HipError> {
		assert!(
			data.len() <= self.len,
			"write_u8: {} bytes into a {}-byte buffer",
			data.len(),
			self.len
		);
		unsafe { xfer_sync(self.ptr, data.as_ptr() as *const c_void, data.len(), HIP_MEMCPY_H2D) }
	}

	/// Overwrite this buffer's device bytes with host f64 data (H2D into the
	/// existing allocation — no fresh alloc). Length must fit.
	pub fn load(&self, data: &[f64]) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(data);
		assert!(bytes <= self.len, "load: {bytes} bytes into a {}-byte buffer", self.len);
		unsafe { xfer_sync(self.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }
	}

	pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	pub fn zeros_bytes(n_bytes: usize) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(n_bytes)?;
		unsafe { memset_sync(buf.ptr, 0, n_bytes) }?;
		Ok(buf)
	}

	pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
		Self::zeros_bytes(n * 4)
	}

	pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_sync(self.ptr, 0, n_bytes) }
	}

	pub fn download(&self, dst: &mut [f64]) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H) }
	}

	pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H) }
	}

	pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, dst.len(), HIP_MEMCPY_D2H) }
	}

	pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H) }
	}

	pub fn len(&self) -> usize {
		self.len
	}
	pub fn n_floats(&self) -> usize {
		self.len / std::mem::size_of::<f64>()
	}
	pub fn ptr_addr(&self) -> usize {
		self.ptr as usize
	}
	pub fn ptr_raw(&self) -> *mut c_void {
		self.ptr
	}

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	pub fn as_ptr_offset(&self, n_floats: usize) -> *mut c_void {
		assert!(
			n_floats * 8 <= self.len,
			"as_ptr_offset: offset {} bytes exceeds buffer len {}",
			n_floats * 8,
			self.len
		);
		unsafe { (self.ptr as *mut u8).add(n_floats * 8) as *mut c_void }
	}

	pub fn view(&self, offset_floats: usize, len_floats: usize) -> GpuBuffer {
		GpuBuffer::borrow(self.as_ptr_offset(offset_floats), len_floats * 8)
	}

	pub fn copy_from(&mut self, src: &GpuBuffer, n_bytes: usize) -> Result<(), HipError> {
		unsafe { xfer_sync(self.ptr, src.ptr as *const c_void, n_bytes, HIP_MEMCPY_D2D) }
	}

	pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_sync(self.ptr, value as i32, n_bytes) }
	}

	pub unsafe fn upload_async(data: &[f64], stream: *mut c_void) -> Result<Self, HipError> {
		let bytes = std::mem::size_of_val(data);
		let buf = Self::alloc(data.len())?;
		// SAFETY: FFI transfer — caller ensures pointer validity and size.
		unsafe { xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, stream) }?;
		Ok(buf)
	}

	pub unsafe fn download_async(
		&self,
		dst: &mut [f64],
		stream: *mut c_void,
	) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
		// SAFETY: FFI transfer — caller ensures pointer validity and size.
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H, stream) }
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
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H) }
	}

	pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe { xfer_sync(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D) }?;
		Ok(buf)
	}

	pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe { xfer_sync(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H) }
	}
}

impl Drop for GpuBuffer {
	fn drop(&mut self) {
		if self.owned && !self.ptr.is_null() && !SHUTTING_DOWN.load(Ordering::Relaxed) {
			tag_sub(self.tag, self.len);
			POOL_LIVE.fetch_sub(self.len, Ordering::Relaxed);
			FREE_TOTAL.fetch_add(1, Ordering::Relaxed);
			crate::callspy::tick(&crate::callspy::FREE_ASYNC);
			unsafe { hipFreeAsync(self.ptr, std::ptr::null_mut()) };
			self.ptr = std::ptr::null_mut();
		}
	}
}

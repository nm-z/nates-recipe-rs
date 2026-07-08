use crate::hip::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

// Cumulative device-pool free count (the free choke site below). Never reset —
// with the real map count (callspy MALLOC_ASYNC) it answers "how many live
// device buffers" at any point.
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
	// Pool + arena state: the bytes the tag map cannot see. slack = mapped but
	// idle (a growth map cannot use it); arena = live claim left.
	line.push(oom_pair("slack", &fmt_bytes(crate::hip::pool_slack(0).unwrap_or(0))));
	line.push(oom_pair("arena", &fmt_bytes(arena_remaining())));
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
	let a = crate::callspy::MALLOC_ASYNC.load(Ordering::Relaxed) as usize;
	let f = FREE_TOTAL.load(Ordering::Relaxed);
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
	crate::callspy::MALLOC_ASYNC.load(Ordering::Relaxed) as usize
}

pub fn device_free_count() -> usize {
	FREE_TOTAL.load(Ordering::Relaxed)
}

/// Cumulative transfer ledger bytes as numbers: (H2D, D2H, D2D). Snapshot before
/// and after a phase to attribute the delta to that phase's PCIe traffic.
pub fn xfer_bytes() -> (usize, usize, usize) {
	(
		H2D_BYTES.load(Ordering::Relaxed),
		D2H_BYTES.load(Ordering::Relaxed),
		D2D_BYTES.load(Ordering::Relaxed),
	)
}

/// Cumulative transfer ledger CALL counts: (H2D, D2H, D2D). One increment per
/// `xfer_sync`/`xfer` regardless of the pinned bounce's 64 MB chunking (callspy
/// MEMCPY_ASYNC counts chunks; these count logical transfers). Snapshot around a
/// phase to prove "one upload / one download" — the authoritative single-transfer
/// check for the one-claim lifecycle.
pub fn xfer_calls() -> (usize, usize, usize) {
	(
		H2D_CALLS.load(Ordering::Relaxed),
		D2H_CALLS.load(Ordering::Relaxed),
		D2D_CALLS.load(Ordering::Relaxed),
	)
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

// Per-tag bytes carved from the live claim (and the aligned total), so
// `release_device_arena` can return every carve's ledger bytes to "unclaimed"
// before the slab's own drop subtracts the full claim — the ledger reads exact
// at every point of the claim→carve→release lifecycle.
static ARENA_CARVED: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
static ARENA_CARVED_ALIGNED: AtomicUsize = AtomicUsize::new(0);

fn arena_note_carve(tag: &'static str, n_bytes: usize, aligned: usize) {
	if let Ok(mut m) = ARENA_CARVED.lock() {
		*m.entry(tag).or_insert(0) += n_bytes;
	}
	ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
}

/// Bump-carve the run's init-image prefix off the arena FRONT (kernel-clean,
/// ledgered "weights") so every later scratch carve lands past it. No HIP call —
/// the image bytes themselves are written into the arena base by the owning
/// claim/adopt op's `commit_with_image`; this only advances the offset. The carve
/// is non-owning (dropping it is a no-op), so the reservation persists.
fn carve_image_front(image: &[f64]) {
	if image.is_empty() {
		return;
	}
	let _t = tag_scope("weights");
	let _img = GpuBuffer::alloc(image.len()).expect("arena image carve");
}

/// The ONE blocking drain of a claim/adopt op: zero the WHOLE slab, then (if the
/// run has a composed init image) stage it through the run pin and enqueue ONE
/// H2D into the arena base, then EXACTLY ONE device drain. Null-stream ordering
/// serializes the memset before the image copy; the drain lets the KFD page-table
/// update settle before the first kernel touches the slab. This is the claim op's
/// sole `hipDeviceSynchronize` — the map (`claim_map_bytes`) does none.
fn commit_with_image(base: *mut c_void, size: usize, image: &[f64]) -> Result<(), HipError> {
	// SAFETY: base spans `size` bytes (the just-mapped slab).
	unsafe { memset_dev(base, 0, size, std::ptr::null_mut())? };
	if !image.is_empty() {
		let bytes = std::mem::size_of_val(image);
		H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
		H2D_CALLS.fetch_add(1, Ordering::Relaxed);
		// The pin is image-sized, so ONE enqueue moves any size (no chunk loop).
		let pin = run_pin(bytes);
		par_copy(pin, image.as_ptr() as *const u8, bytes);
		// SAFETY: pin spans `bytes`; base spans `size` >= bytes; null-stream ordered
		// after the memset above.
		unsafe { dev_copy(base, pin as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())? };
	}
	crate::hip::device_synchronize()
}

/// The run's ONE claim: everything the driver reports free minus the user's
/// gigabyte, memset-committed and registered as the process device arena.
/// Every later `GpuBuffer` allocation bump-carves from it (non-owning, zero
/// pool traffic) until `release_device_arena` — init → one claim, exit → one
/// free. Returns None when even 1 MB cannot be mapped.
pub fn claim_device_arena() -> Option<GpuBuffer> {
	claim_device_arena_with_image(&[])
}

/// Claim exactly `want` bytes (walking down 1/16 per refusal, waterfall-style).
pub fn claim_device_arena_bytes(want: usize) -> Option<GpuBuffer> {
	claim_device_arena_bytes_with_image(want, &[])
}

/// Claim the process arena AND commit the run's composed init image to its front
/// in one operation — the H2D rides the claim (no standalone upload). Sizes the
/// ask as the growable headroom (device free base minus the user's gigabyte),
/// rounded down to a 2 MB boundary. Returns the slab (image resident at offset 0,
/// arena offset past it); None if even the minimum arena cannot be mapped.
pub fn claim_device_arena_with_image(image: &[f64]) -> Option<GpuBuffer> {
	let grow = vram_free_base().saturating_sub(USER_GB);
	claim_device_arena_bytes_with_image(grow & !((1 << 21) - 1), image)
}

/// Claim exactly `want` bytes AND commit the image to the arena front in one
/// operation (the detector's sized claim + fused init image). The map
/// (`claim_map_bytes`, no drain) walks `want` down 1/16 per refusal; on success
/// the arena is registered, the image prefix is bump-carved, and `commit_with_image`
/// does the op's single memset+image+drain. Returns the slab (image resident at
/// offset 0); None if even 1 MB cannot be mapped.
pub fn claim_device_arena_bytes_with_image(mut want: usize, image: &[f64]) -> Option<GpuBuffer> {
	assert_eq!(
		ARENA_BASE.load(Ordering::Relaxed),
		0,
		"claim_device_arena: a device arena is already active"
	);
	let _t = tag_scope("unclaimed");
	while want > (1 << 20) {
		match GpuBuffer::claim_map_bytes(want) {
			Some(slab) => {
				set_device_arena(slab.ptr_raw(), want);
				carve_image_front(image);
				commit_with_image(slab.ptr_raw(), want, image).expect("claim commit");
				return Some(slab);
			}
			None => want -= want / 16,
		}
	}
	None
}

/// The run's ONE free: drain the device (no carve may be in flight), unregister
/// the arena, sweep every carve's ledger bytes back to "unclaimed", then drop
/// the slab — the single `hipFreeAsync` of the run's exit block.
pub fn release_device_arena(slab: GpuBuffer) {
	assert_eq!(
		ARENA_BASE.load(Ordering::Relaxed),
		slab.ptr_addr(),
		"release_device_arena: slab is not the active claim"
	);
	crate::hip::device_synchronize().expect("arena release sync");
	set_device_arena(std::ptr::null_mut(), 0);
	if let Ok(mut m) = ARENA_CARVED.lock() {
		for (tag, bytes) in m.iter() {
			tag_sub(tag, *bytes);
		}
		m.clear();
	}
	tag_add("unclaimed", ARENA_CARVED_ALIGNED.swap(0, Ordering::Relaxed));
	drop(slab);
}

// The previous run's backing block (arena slab, or the arena backing of
// an out-of-core run), kept alive after the run returns so the params/stat
// views stored in the model registry stay valid for eval/save. The NEXT run's
// entry releases it — the deferred "one free at exit" of the run lifecycle.
static PARKED: Mutex<Option<GpuBuffer>> = Mutex::new(None);

// Monotonic id stamped on each parked backing, and the id of the one CURRENTLY
// parked (0 = nothing parked). A model records the id its param views were
// carved from; before a forward-only read it compares against the live id, and
// a mismatch means a later run freed that backing (the params must be rebuilt
// from the host weight mirror rather than dereferenced).
static PARK_GEN: AtomicUsize = AtomicUsize::new(0);
static PARKED_GEN: AtomicUsize = AtomicUsize::new(0);

pub fn park_run_backing(buf: GpuBuffer) {
	let mut g = match PARKED.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	assert!(g.is_none(), "park_run_backing: a parked run backing already exists");
	*g = Some(buf);
	PARKED_GEN.store(PARK_GEN.fetch_add(1, Ordering::Relaxed) + 1, Ordering::Relaxed);
}

/// Id of the backing currently parked (the arena slab whose carves the last
/// in-VRAM run left resident), or None when nothing is parked. A model records
/// this at park time; when it no longer matches, a later run freed that slab
/// and the model's device weights are gone.
pub fn live_parked_gen() -> Option<usize> {
	let g = PARKED_GEN.load(Ordering::Relaxed);
	(g != 0).then_some(g)
}

/// Take the parked backing and re-arm it as THIS run's arena when it can hold
/// `need` bytes: no free, no realloc, no unmap — the freeAsync lazy-unmap /
/// VA-reuse race and the post-free counter depression (driver still counting the
/// freed slab, refusing even 8-byte growth) structurally cannot fire. The prior
/// run's carve ledger is swept and the bump pointer rewound. NO memset, NO drain
/// here — the caller's `commit_with_image` does the op's single memset+image+drain
/// (accumulator carves rely on the claim-time zero fill). Returns None when
/// nothing is parked (first run: caller claims fresh) or the parked slab is too
/// small / not the registered arena (released here, keeping its existing syncs;
/// caller claims fresh).
fn adopt_run_backing_inner(need: usize) -> Option<GpuBuffer> {
	let parked = match PARKED.lock() {
		Ok(mut g) => g.take(),
		Err(p) => p.into_inner().take(),
	}?;
	PARKED_GEN.store(0, Ordering::Relaxed);
	let registered = ARENA_BASE.load(Ordering::Relaxed) == parked.ptr_addr();
	if !registered || parked.len() < need {
		if registered {
			release_device_arena(parked);
		} else {
			crate::hip::device_synchronize().expect("parked release sync");
			drop(parked);
			pool_trim();
		}
		return None;
	}
	if let Ok(mut m) = ARENA_CARVED.lock() {
		for (tag, bytes) in m.iter() {
			tag_sub(tag, *bytes);
		}
		m.clear();
	}
	tag_add("unclaimed", ARENA_CARVED_ALIGNED.swap(0, Ordering::Relaxed));
	ARENA_OFFSET.store(0, Ordering::Relaxed);
	Some(parked)
}

/// Re-arm the previous run's parked backing as THIS run's arena (no image): the
/// inner take/validate/rewind, then `commit_with_image` re-zeroes the whole slab
/// with the op's single memset + drain. Prior params carved from the slab are
/// invalidated via the park generation — their models rebuild from the host
/// weight mirror. None when nothing is parked or the slab is too small.
pub fn adopt_run_backing(need: usize) -> Option<GpuBuffer> {
	adopt_run_backing_with_image(need, &[])
}

/// Adopt the parked backing AND commit the run's composed init image to its front
/// in one operation — the H2D rides the adopt (no standalone upload). ONE drain
/// total (`commit_with_image`). Returns the re-armed slab (image resident at
/// offset 0); None when nothing is parked or the parked slab is too small (caller
/// claims fresh with the image).
pub fn adopt_run_backing_with_image(need: usize, image: &[f64]) -> Option<GpuBuffer> {
	let slab = adopt_run_backing_inner(need)?;
	carve_image_front(image);
	commit_with_image(slab.ptr_raw(), slab.len(), image).expect("adopt commit");
	Some(slab)
}

/// Release the previous run's parked backing (no-op when none). Views into it
/// — a prior fit's weights — are dead after this; the caller owns that
/// invalidation (each run re-composes its inputs from host state).
pub fn release_run_backing() {
	let parked = match PARKED.lock() {
		Ok(mut g) => g.take(),
		Err(p) => p.into_inner().take(),
	};
	PARKED_GEN.store(0, Ordering::Relaxed);
	if let Some(b) = parked {
		if ARENA_BASE.load(Ordering::Relaxed) == b.ptr_addr() {
			release_device_arena(b);
		} else {
			crate::hip::device_synchronize().expect("parked release sync");
			drop(b);
		}
	}
}

/// AOT init composer: every host-sourced block a run needs on the device is
/// pushed (real bytes) or reserved (device-filled later: randn weights, zscore
/// stats, metric ring) during init planning, then `upload()` moves the WHOLE
/// image with one carve + ONE sync H2D — the run's single upload. Offsets are
/// in f64s, each block 256-byte aligned so any view is kernel-clean.
pub struct Stage {
	host: Vec<f64>,
}

const STAGE_ALIGN_F64: usize = ARENA_ALIGN / 8;

impl Default for Stage {
	fn default() -> Self {
		Self::new()
	}
}

impl Stage {
	pub fn new() -> Self {
		Stage { host: Vec::new() }
	}

	fn pad(&mut self) {
		let rem = self.host.len() % STAGE_ALIGN_F64;
		if rem != 0 {
			self.host.resize(self.host.len() + STAGE_ALIGN_F64 - rem, 0.0);
		}
	}

	/// Append `data`, returning its offset in f64s.
	pub fn push(&mut self, data: &[f64]) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.extend_from_slice(data);
		off
	}

	/// Append a zero-filled slot the device fills later (randn init, zscore
	/// stats, metric ring), returning its offset in f64s.
	pub fn reserve(&mut self, n_floats: usize) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + n_floats, 0.0);
		off
	}

	pub fn len_floats(&self) -> usize {
		self.host.len()
	}

	/// Consume the Stage and hand back the composed host image — the caller uploads
	/// it as part of the arena claim/adopt (`claim_device_arena_with_image`) so the
	/// H2D rides the claim instead of a standalone `upload`.
	pub fn into_host(self) -> Vec<f64> {
		self.host
	}

	pub fn is_empty(&self) -> bool {
		self.host.is_empty()
	}

}

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
	if kind != HIP_MEMCPY_H2D {
		// Standalone async transfer call — the run state table's `async` cell.
		// H2D is excluded (the bounce makes it blocking; its chunk waits land in
		// the sync cell), as are the claim/park riding paths (they never enter
		// this choke — the transfer is part of the claim/park op itself).
		crate::callspy::tick(&crate::callspy::XFER_ASYNC);
	}
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

/// Right-sized pinned staging owned by the claim/adopt ops: the init-image H2D
/// (`commit_with_image`) and the exit prefix D2H (`exit_d2h_enqueue`) ride it,
/// replacing the fixed 64 MB BOUNCE for those two op paths. (ptr, bytes) — grows
/// to fit the largest image/prefix asked, never shrinks; freed at shutdown.
static RUN_PIN: Mutex<(usize, usize)> = Mutex::new((0, 0));

/// Grow the (already-locked) run pin to hold at least `bytes` and return its
/// base. The old pin is host-freed before the larger one is allocated, so at most
/// one pin is live. host_malloc/host_free tick HOST_MALLOC/HOST_FREE (wanted).
fn pin_ensure(g: &mut (usize, usize), bytes: usize) -> *mut u8 {
	if g.1 < bytes {
		if g.0 != 0 {
			let _ = unsafe { crate::hip::host_free(g.0 as *mut c_void) };
		}
		g.0 = crate::hip::host_malloc(bytes, 0).expect("run_pin host_malloc") as usize;
		g.1 = bytes;
	}
	g.0 as *mut u8
}

/// Run pin base sized to at least `bytes` (grows on demand). Used by
/// `commit_with_image`, which drains before returning — the lock need not be held
/// across the copy. `exit_d2h_enqueue` instead holds the guard in its handle.
fn run_pin(bytes: usize) -> *mut u8 {
	let mut g = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	pin_ensure(&mut g, bytes)
}

/// Pinned bounce arena [base, base+len) if allocated — the fault autopsy names
/// a faulting VA that lands inside it.
pub(crate) fn bounce_range() -> Option<(usize, usize)> {
	let base = *BOUNCE.lock().ok()?;
	(base != 0).then_some((base, BOUNCE_BYTES))
}

// Ring of the most recent allocation ranges (device pool, arena carves, pinned
// host), so the fault autopsy can name the buffer a faulting VA lands in.
// Entries outlive frees on purpose — a fault in a FREED range is itself the
// diagnosis (use-after-free / reuse-before-remap).
static RECENT_RANGES: Mutex<Vec<(usize, usize, &'static str)>> = Mutex::new(Vec::new());

pub(crate) fn note_range(base: usize, len: usize, what: &'static str) {
	if let Ok(mut r) = RECENT_RANGES.lock() {
		if r.len() >= 256 {
			r.remove(0);
		}
		r.push((base, len, what));
	}
}

/// Name the most recent recorded allocation containing `va` (most recent wins:
/// a reused VA should report its current life, not a prior one).
pub(crate) fn locate_va(va: usize) -> Option<String> {
	let r = RECENT_RANGES.lock().ok()?;
	r.iter()
		.rev()
		.find(|(b, l, _)| va >= *b && va < b + l)
		.map(|(b, l, what)| format!("{what} [base 0x{b:x} len {}] +0x{:x}", fmt_bytes(*l), va - b))
}

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

/// Explicitly release the run pin at shutdown — the companion of `free_bounce`
/// (the claim/adopt ops' staging vs. the generic bounce). Drain first: an
/// in-flight async copy touching the pin would read freed host pages.
pub(crate) fn free_run_pin() {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if guard.0 != 0 {
		let _ = crate::hip::device_synchronize();
		let _ = unsafe { crate::hip::host_free(guard.0 as *mut c_void) };
		*guard = (0, 0);
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

/// Largest image the single-enqueue async transfer paths accept; above it the
/// caller falls back to the chunked synced path (correctness over cell-counts).
pub const BOUNCE_LIMIT: usize = BOUNCE_BYTES;

/// In-flight handle for the exit's two-phase D2H: the device→pin copy is already
/// enqueued (async, no wait) and the run-pin lock is held so nothing else can
/// reuse the pin. The caller drains the device by OTHER means — the Scratch
/// drop's mandated `device_synchronize` — then calls `finish` to fan the pin
/// into the host buffer on all cores. One enqueue, zero dedicated waits.
pub struct ExitD2H {
	_guard: std::sync::MutexGuard<'static, (usize, usize)>,
	pin: usize,
	bytes: usize,
}

impl ExitD2H {
	/// Fan the (already-drained) pin into `dst`. Call ONLY after the device has
	/// been synchronized so the enqueued copy is complete; releases the pin lock.
	pub fn finish(self, dst: &mut [f64]) {
		assert_eq!(std::mem::size_of_val(dst), self.bytes, "ExitD2H::finish size mismatch");
		// SAFETY: pin spans >= bytes; dst spans `bytes`; copy is complete.
		par_copy(dst.as_mut_ptr() as *mut u8, self.pin as *const u8, self.bytes);
	}
}

/// Phase 1 of the blocking-free exit D2H: grow the run pin to fit, enqueue the
/// device→pin copy on the null stream (async, counted as one logical D2H) and
/// hold the pin lock in the returned handle. Any size — the pin grows to `bytes`,
/// so one enqueue moves it. The handle's `finish` completes the transfer
/// host-side after the caller's own device drain.
pub unsafe fn exit_d2h_enqueue(src: *const c_void, bytes: usize) -> Result<ExitD2H, HipError> {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	let pin = pin_ensure(&mut guard, bytes);
	D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
	D2H_CALLS.fetch_add(1, Ordering::Relaxed);
	// SAFETY: caller guarantees src spans `bytes`; pin spans >= bytes.
	unsafe { dev_copy(pin as *mut c_void, src, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
	Ok(ExitD2H { _guard: guard, pin: pin as usize, bytes })
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

static DEVICE_INIT: std::sync::Once = std::sync::Once::new();

/// atexit half of the birth claim: free what you allocate, drained, before death.
extern "C" fn release_birth_claim_at_exit() {
	release_run_backing();
}

/// One-time pool setup on the first allocation of the process: SDMA off,
/// release threshold pinned, fault autopsy + thrash watchdog registered. The
/// old 1 GiB warm is gone — the run's arena claim is larger and commits its own
/// pages (memset + drain inside the claim op), so pre-touching the pool would
/// only shrink the claim (the no-warmup invariant).
pub(crate) fn device_init_once() {
	DEVICE_INIT.call_once(|| {
		crate::hip::disable_sdma_once();
		if let Err(e) = crate::hip::set_pool_retain(0) {
			eprintln!("GPU pool retain failed: {e}");
		}
		crate::hip::register_fault_autopsy_once();
		crate::hw::spawn_thrash_watchdog();
	});
}


/// Release the pool's idle slack back to the driver. An out-of-core fit leaves
/// nearly all of VRAM as freed slack; a later differently-shaped allocation storm
/// forces new physical maps out of the fragmented slack — the uncatchable VmHeap
/// assert, observed at the interrupt teardown. Sync + trim hands the pages back.
pub fn pool_trim() {
	crate::hip::device_synchronize().expect("pool_trim sync");
	crate::hip::trim_mempool(0).expect("pool_trim");
}

/// THE reserve law: exactly 1 GB of each tier belongs to the user, always.
pub const USER_GB: usize = 1 << 30;

/// Counters' view of free device memory: min of HIP's and the kernel's free
/// counts (hipMemGetInfo does not see other processes; sysfs does) minus the
/// pool's idle slack (mapped, but a growth map cannot use it). Crashes on a
/// failed query — a wedged device must never read as "0 free" and quietly
/// reroute callers.
pub fn vram_free_base() -> usize {
	let hip_free = crate::hip::mem_info().expect("hipMemGetInfo").0;
	let sys_free = crate::hip::sysfs_vram_free().unwrap_or(hip_free);
	let slack = crate::hip::pool_slack(0).expect("pool_slack");
	hip_free.min(sys_free).saturating_sub(slack)
}

/// Device bytes claimable under the SAME rules the alloc gate below enforces:
/// arena remainder plus growable headroom (base minus the user's gigabyte).
/// Every "will it fit" decision consults this — never raw hipMemGetInfo.
pub fn claimable_bytes() -> usize {
	arena_remaining() + vram_free_base().saturating_sub(USER_GB)
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

/// Whether a device arena is currently registered — a live claim (a run's own)
/// or a parked training backing still resident between runs. A second claim
/// asserts, so callers that opportunistically claim (the detector) must skip
/// their claim when one is already active and carve from it instead.
pub fn device_arena_active() -> bool {
	ARENA_BASE.load(Ordering::Relaxed) != 0
}

/// Force-commit a 1 GiB buffer (so its pages are backed), zero it, then free it.
/// With the pool's release threshold pinned the pages stay resident, so later
/// allocs reuse already-mapped memory. Runs through the choke points against the
/// ledger under tag "warmup"; freed immediately.
/// Walk the claimable ask down from the counters' guess until a DISPOSABLE
/// probe (spawned by the caller — a child process that attempts exactly one
/// allocation) survives. Both counters over-report the true VmHeap ceiling
/// and an in-process overshoot is an uncatchable `VmHeap::MapPhysMemory`
/// abort (reproduced deterministically at LLM scale — the over-report can
/// exceed the 1 GB band), so the ceiling is MEASURED, never assumed.
pub fn probe_ceiling(mut probe_survives: impl FnMut(usize) -> bool) -> Option<usize> {
	let mut want = vram_free_base().saturating_sub(USER_GB) & !((1 << 21) - 1);
	while want > (1 << 30) {
		if probe_survives(want) {
			eprintln!("claim probe: {:.2} GB (probe-verified)", want as f64 / (1u64 << 30) as f64);
			return Some(want);
		}
		eprintln!("claim probe: {:.2} GB unmappable, backing off", want as f64 / (1u64 << 30) as f64);
		want -= want / 16;
	}
	None
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

	pub fn is_pool_owned(&self) -> bool { self.owned }

	pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
		Self::alloc_bytes(n_floats * std::mem::size_of::<f64>())
	}

	/// Waterfall probe: allocate or quietly refuse. A `None` is the "VRAM is
	/// full" signal the waterfall fills against — no autopsy spam, no error.
	/// Routes through the same single `hipMallocAsync` site as `alloc_bytes`;
	/// quiet by construction (`alloc_bytes_inner` never reports).
	pub fn try_alloc_bytes(n_bytes: usize) -> Option<Self> {
		Self::alloc_bytes_inner(n_bytes).ok()
	}

	/// The loud hand-out: on refusal, print the VRAM autopsy before returning the
	/// error. `alloc_bytes_inner` stays silent so `try_alloc_bytes` (the waterfall
	/// probe) does not spam — the report lives in this wrapper alone.
	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		match Self::alloc_bytes_inner(n_bytes) {
			Ok(buf) => Ok(buf),
			Err(e) => {
				oom_report(n_bytes);
				if device_arena_active() {
					// A must-succeed carve missing a live claim cannot occur in a
					// correct program — placement exceeded the claim. Die loud.
					panic!(
						"arena carve miss: {n_bytes} B asked, {} B remain — placement exceeded the claim",
						arena_remaining()
					);
				}
				Err(e)
			}
		}
	}

	/// THE single `hipMallocAsync` call site: tick, map, check. Nothing else —
	/// no commit, no ledger, no admission. Both the pool hand-out
	/// (`alloc_bytes_inner`) and the claim op's mapping (`claim_map_bytes`) funnel
	/// their one map through here.
	fn map_bytes(n_bytes: usize) -> Result<*mut c_void, HipError> {
		let mut ptr: *mut c_void = std::ptr::null_mut();
		crate::callspy::tick(&crate::callspy::MALLOC_ASYNC);
		check(unsafe { hipMallocAsync(&mut ptr, n_bytes, std::ptr::null_mut()) })?;
		Ok(ptr)
	}

	/// Arena carve. A claimless process's first byte births the process claim
	/// (freed at exit); everything after carves. Refuses SILENTLY (the loud
	/// `alloc_bytes` wrapper reports).
	fn alloc_bytes_inner(n_bytes: usize) -> Result<Self, HipError> {
		device_init_once();
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
						arena_note_carve(tag, n_bytes, aligned);
						note_range(ptr as usize, n_bytes, tag);
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
		if base != 0 {
			// Carve miss on a live claim: the probes' quiet fill signal
			// (try_alloc_bytes → None); must-succeed callers die in alloc_bytes.
			return Err(HipError(2));
		}
		// First device byte of the process: birth the ONE claim, carve from it,
		// free it at exit. No per-buffer pool path exists.
		match claim_device_arena() {
			Some(slab) => {
				park_run_backing(slab);
				unsafe {
					libc::atexit(release_birth_claim_at_exit);
				}
				Self::alloc_bytes_inner(n_bytes)
			}
			None => Err(HipError(2)),
		}
	}

	/// The claim/adopt op's own mapping — the FIRST HALF of a claim op, its own
	/// contract distinct from `alloc_bytes_inner`: the same headroom admission
	/// and `map_bytes`, the same ledger/note_range bookkeeping, but NO
	/// memset and NO drain — the claim op commits the WHOLE slab itself (one
	/// memset + the init image + ONE drain, in `commit_with_image`). A claim can
	/// only happen with no arena active (asserted); there is no bump-carve branch.
	/// Refuses SILENTLY (the claim walks `want` down 1/16 on `None`).
	pub(crate) fn claim_map_bytes(n_bytes: usize) -> Option<Self> {
		device_init_once();
		ALLOC_FROZEN.with(|f| {
			assert!(!f.get(), "GPU claim inside frozen training loop (requested {n_bytes} bytes)")
		});
		assert_eq!(
			ARENA_BASE.load(Ordering::Relaxed),
			0,
			"claim_map_bytes: a device arena is already active"
		);
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(|t| t.get());
		let remaining = vram_free_base();
		if n_bytes > remaining.saturating_sub(USER_GB) {
			return None;
		}
		let ptr = Self::map_bytes(n_bytes).ok()?;
		tag_add(tag, n_bytes);
		note_range(ptr as usize, n_bytes, tag);
		Some(Self {
			ptr,
			len: n_bytes,
			owned: true,
			tag,
		})
	}

	/// Copy host bytes into this (already-carved) buffer — the reuse path for a
	/// persistent staging window, avoiding a fresh alloc per upload. H2D rides the
	/// pinned bounce, which is synchronous on return (the bounce holds its lock
	/// across each chunk's stream sync), so the bytes are resident when this
	/// returns; no caller drain is needed.
	pub fn write_u8(&self, data: &[u8]) -> Result<(), HipError> {
		assert!(
			data.len() <= self.len,
			"write_u8: {} bytes into a {}-byte buffer",
			data.len(),
			self.len
		);
		unsafe {
			xfer(self.ptr, data.as_ptr() as *const c_void, data.len(), HIP_MEMCPY_H2D, std::ptr::null_mut())
		}
	}

	/// Overwrite this buffer's device bytes with host f64 data (H2D into the
	/// existing carve — no fresh alloc). The replacement for the deleted `upload`:
	/// carve the buffer, then `load` into it. H2D is synchronous through the pinned
	/// bounce, so the data is resident on return. Length must fit.
	pub fn load(&self, data: &[f64]) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(data);
		assert!(bytes <= self.len, "load: {bytes} bytes into a {}-byte buffer", self.len);
		unsafe {
			xfer(self.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())
		}
	}

	pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())
		}?;
		Ok(buf)
	}

	pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())
		}?;
		Ok(buf)
	}

	pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(n * 4)?;
		buf.memset_zero(n * 4)?;
		Ok(buf)
	}

	/// Zero this buffer's device bytes: the ONE memset choke (`memset_dev`)
	/// enqueued on the null stream, then the drain — this method is the op that
	/// owns completion, so the memset is done on return. Callers that previously
	/// used the deleted `zeros_bytes` alloc + this method's zero.
	pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_dev(self.ptr, 0, n_bytes, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, dst.len(), HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
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
		unsafe { xfer(self.ptr, src.ptr as *const c_void, n_bytes, HIP_MEMCPY_D2D, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_dev(self.ptr, value as i32, n_bytes, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
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

	pub fn upload_f16(data: &[half::f16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())
		}?;
		Ok(buf)
	}

	pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())
		}?;
		Ok(buf)
	}

	pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe { xfer(dst.as_mut_ptr() as *mut c_void, self.ptr, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}
}

impl Drop for GpuBuffer {
	fn drop(&mut self) {
		if self.owned && !self.ptr.is_null() && !SHUTTING_DOWN.load(Ordering::Relaxed) {
			tag_sub(self.tag, self.len);
			FREE_TOTAL.fetch_add(1, Ordering::Relaxed);
			crate::callspy::tick(&crate::callspy::FREE_ASYNC);
			let code = unsafe { hipFreeAsync(self.ptr, std::ptr::null_mut()) };
			// Drop cannot propagate; a swallowed failed free silently leaks the
			// buffer in the DRIVER's accounting (free counters stay depressed for
			// the rest of the process) — say so loudly instead.
			if code != 0 {
				eprintln!(
					"hipFreeAsync FAILED (code {code}): leaked {} tag '{}' at {:p}",
					self.len, self.tag, self.ptr
				);
			}
			self.ptr = std::ptr::null_mut();
		}
	}
}

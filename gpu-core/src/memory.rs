extern crate alloc;

use crate::HipError;
use crate::callspy;
use crate::hip::{
	HIP_MEMCPY_D2D, HIP_MEMCPY_D2H, HIP_MEMCPY_H2D, MemInfo, check, device_synchronize,
	disable_sdma_once, hipFreeAsync, hipMallocAsync, hipMemcpyAsync, hipMemsetAsync,
	hipStreamSynchronize, host_free, host_malloc, mem_info, pool_slack,
	register_fault_autopsy_once, set_pool_retain, sysfs_vram_free, trim_mempool,
};
use crate::log::{Write, gpu};
use alloc::collections::BTreeMap;
use core::cell::Cell;
use core::cmp;
use core::ffi::{CStr, c_void};
use core::fmt::Write as _;
use core::marker::PhantomData;
use core::mem;
use core::ptr;
use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::fs;
use std::num;
use std::sync::{Mutex, MutexGuard, Once};
use std::thread;

/// Total device allocations requested this process (diagnostic counter).
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
/// Nonzero once shutdown begins; suppresses frees racing HIP's atexit dtor.
static SHUTTING_DOWN: AtomicU8 = AtomicU8::new(0);

/// Cumulative count of device frees.
static FREE_TOTAL: AtomicUsize = AtomicUsize::new(0);

/// Cumulative host-to-device transfer bytes.
static H2D_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Cumulative device-to-host transfer bytes.
static D2H_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Cumulative device-to-device transfer bytes.
static D2D_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Cumulative host-to-device transfer call count.
static H2D_CALLS: AtomicUsize = AtomicUsize::new(0);
/// Cumulative device-to-host transfer call count.
static D2H_CALLS: AtomicUsize = AtomicUsize::new(0);
/// Cumulative device-to-device transfer call count.
static D2D_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Live bytes per allocation tag.
static TAG_BYTES: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
/// Peak live bytes per allocation tag.
static TAG_PEAK: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());

thread_local! {
	static CURRENT_TAG: Cell<&'static str> = const { Cell::new("other") };
}

pub struct TagScope {
	/// Tag active before this scope, restored on drop.
	prev: &'static str,
}

#[must_use]
#[inline]
pub fn tag_scope(name: &'static str) -> TagScope {
	let prev = CURRENT_TAG.with(|t| return t.replace(name));
	return TagScope { prev };
}

impl Drop for TagScope {
	#[inline]
	fn drop(&mut self) {
		CURRENT_TAG.with(|t| t.set(self.prev));
	}
}

/// Add `n` bytes to `tag`'s live total and update its peak.
fn tag_add(tag: &'static str, n: usize) {
	let live = match TAG_BYTES.lock() {
		Ok(mut m) => {
			let e = m.entry(tag).or_insert(0);
			*e += n;
			*e
		}
		Err(_p) => return,
	};
	let Ok(mut p) = TAG_PEAK.lock() else {
		return;
	};
	let e = p.entry(tag).or_insert(0);
	*e = (*e).max(live);
}

/// Subtract `n` bytes from `tag`'s live total (saturating).
fn tag_sub(tag: &'static str, n: usize) {
	let Ok(mut m) = TAG_BYTES.lock() else {
		return;
	};
	let e = m.entry(tag).or_insert(0);
	*e = e.saturating_sub(n);
}

/// Record an allocation of `n` bytes under `tag`.
#[inline]
pub fn tag_note_alloc(tag: &'static str, n: usize) {
	tag_add(tag, n);
}

/// Record a free of `n` bytes under `tag`.
#[inline]
pub fn tag_note_free(tag: &'static str, n: usize) {
	tag_sub(tag, n);
}

/// Format a byte count as a human-readable GB/MB/KB/B string.
fn fmt_bytes(b: usize) -> String {
	const K: usize = 1024;
	const M: usize = K * K;
	const G: usize = K * K * K;
	let (div, unit) = if b >= G {
		(G, "GB")
	} else if b >= M {
		(M, "MB")
	} else if b >= K {
		(K, "KB")
	} else {
		return format!("{b} B");
	};
	let whole = b.div_euclid(div);
	let frac = (b.rem_euclid(div) * 100).div_euclid(div);
	return format!("{whole}.{frac:02} {unit}");
}

/// Format a `name: value` diagnostic pair.
fn oom_pair(name: &str, val: &str) -> String {
	return format!("{name}: {val}");
}

/// A tag paired with its live byte count, for ledger rows.
struct TagBytes {
	/// The allocation tag.
	tag: &'static str,
	/// Live bytes attributed to this tag.
	bytes: usize,
}

/// Writes an out-of-memory autopsy to stderr with per-tag live bytes and transfer totals.
#[inline]
pub fn oom_report(req: usize) {
	let mi = mem_info().unwrap_or(MemInfo { free: 0, total: 0 });
	let free = mi.free;
	let total = mi.total;
	let mut autopsy: Vec<TagBytes> = match TAG_BYTES.lock() {
		Ok(m) => m
			.keys()
			.filter_map(|k| {
				let bytes = m.get(k).copied().unwrap_or(0);
				return Some(TagBytes { tag: k, bytes }).filter(|tb| return tb.bytes > 0);
			})
			.collect(),
		Err(_p) => Vec::new(),
	};
	autopsy.sort_by_key(|b| return cmp::Reverse(b.bytes));
	let mut line: Vec<String> = autopsy
		.iter()
		.map(|tb| return oom_pair(tb.tag, &fmt_bytes(tb.bytes)))
		.collect();
	line.push(oom_pair("req", &fmt_bytes(req)));
	line.push(oom_pair("free", &fmt_bytes(free)));
	line.push(oom_pair("total", &fmt_bytes(total)));
	line.push(oom_pair("over", &fmt_bytes(req.saturating_sub(free))));
	line.push(oom_pair("slack", &fmt_bytes(pool_slack(0).unwrap_or(0))));
	line.push(oom_pair("arena", &fmt_bytes(arena_remaining())));
	Write::error(line.join(", "));
	Write::error(format!(
		"{}, {}, {}",
		oom_pair("H2D", &fmt_bytes(H2D_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2H", &fmt_bytes(D2H_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2D", &fmt_bytes(D2D_BYTES.load(Ordering::Relaxed))),
	));
}

/// Kernel-reported video and system memory usage for this process.
pub struct FdinfoMem {
	/// Video memory in use, in kibibytes.
	vram_kib: u64,
	/// System memory in use, in kibibytes.
	gtt_kib: u64,
}

/// Sums per-client video and system memory from kernel fdinfo, or None if unreadable.
#[must_use]
#[inline]
pub fn kernel_fdinfo() -> Option<FdinfoMem> {
	let entries = fs::read_dir("/proc/self/fdinfo").ok()?;
	let mut by_client: BTreeMap<String, FdinfoMem> = BTreeMap::new();
	for e in entries.flatten() {
		let Ok(info) = fs::read_to_string(e.path()) else {
			continue;
		};
		let mut client_id: Option<String> = None;
		let mut vram_kib = 0u64;
		let mut gtt_kib = 0u64;
		for line in info.lines() {
			match line.strip_prefix("drm-client-id:") {
				Some(v) => client_id = Some(v.trim().to_owned()),
				None => match line.strip_prefix("drm-memory-vram:") {
					Some(v) => {
						vram_kib = v
							.trim()
							.trim_end_matches("KiB")
							.trim()
							.parse()
							.unwrap_or(0);
					}
					None => {
						if let Some(v) = line.strip_prefix("drm-memory-gtt:") {
							gtt_kib = v
								.trim()
								.trim_end_matches("KiB")
								.trim()
								.parse()
								.unwrap_or(0);
						}
					}
				},
			}
		}
		let Some(id) = client_id else {
			continue;
		};
		by_client
			.entry(id)
			.or_insert(FdinfoMem { vram_kib, gtt_kib });
	}
	let vram_total: u64 = by_client.values().map(|cm| return cm.vram_kib).sum();
	let gtt_total: u64 = by_client.values().map(|cm| return cm.gtt_kib).sum();
	return Some(FdinfoMem {
		vram_kib: vram_total,
		gtt_kib: gtt_total,
	});
}

/// Pairs a vramspy allocation-kind code with its display label.
struct KindLabel {
	/// Numeric allocation-kind code.
	kind: u32,
	/// Human-readable name for the kind.
	label: &'static str,
}

/// One per-kind vramspy row: live and peak bytes with alloc and free counts.
struct VramspyRow {
	/// Allocation-kind label.
	label: &'static str,
	/// Currently live bytes for this kind.
	live: u64,
	/// Peak live bytes observed for this kind.
	peak: u64,
	/// Cumulative allocation count.
	al: u64,
	/// Cumulative free count for this VRAM kind.
	fr: u64,
}

/// Formats the vramspy global-memory section, or a not-loaded notice when the library is absent.
#[inline]
pub fn ledger_vramspy_section(total_live: usize) -> String {
	let mut s = String::new();
	// SAFETY: dlsym on RTLD_DEFAULT with a static NUL-terminated name; a missing symbol returns null, handled below.
	let live_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_live".as_ptr()) };
	match ptr::NonNull::new(live_sym) {
		None => {
			s += "  global: vramspy NOT loaded (LD_PRELOAD libvramspy.so)\n";
		}
		Some(_present) => {
			let sym = |name: &CStr| -> *mut c_void {
				// SAFETY: dlsym on RTLD_DEFAULT with a valid NUL-terminated name; returns null when the symbol is absent.
				unsafe { return libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
			};
			let to_u32_u64 = |p: *mut c_void| -> extern "C" fn(u32) -> u64 {
				// SAFETY: p is a dlsym address for a vramspy function of exactly this ABI, so the transmute is valid.
				unsafe {
					return mem::transmute::<*mut c_void, extern "C" fn(u32) -> u64>(p);
				}
			};
			let live_fn = to_u32_u64(live_sym);
			let peak_fn = to_u32_u64(sym(c"vramspy_peak"));
			let allocs_fn = to_u32_u64(sym(c"vramspy_allocs"));
			let frees_fn = to_u32_u64(sym(c"vramspy_frees"));
			// SAFETY: sym returns the dlsym address for vramspy_unknown_frees with exactly this ABI.
			let unknown_frees_fn = unsafe {
				mem::transmute::<*mut c_void, extern "C" fn() -> u64>(sym(
					c"vramspy_unknown_frees",
				))
			};

			s += "  global (vramspy, every byte incl. runtime+libs)\n";
			let kinds = [
				KindLabel {
					kind: 0,
					label: "device",
				},
				KindLabel {
					kind: 1,
					label: "pinned",
				},
				KindLabel {
					kind: 2,
					label: "kernarg",
				},
				KindLabel {
					kind: 3,
					label: "other",
				},
			];
			let rows: Vec<VramspyRow> = kinds
				.into_iter()
				.map(|kl| {
					return VramspyRow {
						label: kl.label,
						live: live_fn(kl.kind),
						peak: peak_fn(kl.kind),
						al: allocs_fn(kl.kind),
						fr: frees_fn(kl.kind),
					};
				})
				.collect();
			let device_live = rows.first().map_or(0, |r| return r.live);
			for r in &rows {
				writeln!(
					s,
					"    {:<10} live {:>11}  peak {:>11}  (allocs {} frees {})",
					r.label,
					fmt_bytes(usize::try_from(r.live).unwrap_or(0)),
					fmt_bytes(usize::try_from(r.peak).unwrap_or(0)),
					r.al,
					r.fr
				)
				.unwrap_or_default();
			}
			let delta = usize::try_from(device_live)
				.unwrap_or(0)
				.saturating_sub(total_live);
			writeln!(
				s,
				"    library delta: {} (unknown frees: {})",
				fmt_bytes(delta),
				unknown_frees_fn()
			)
			.unwrap_or_default();
		}
	}
	return s;
}

#[inline]
pub fn ledger_report() -> String {
	let mut live: Vec<TagBytes> = TAG_BYTES
		.lock()
		.map(|m| {
			return m
				.keys()
				.map(|k| {
					return TagBytes {
						tag: k,
						bytes: m.get(k).copied().unwrap_or(0),
					};
				})
				.collect();
		})
		.unwrap_or_default();
	live.sort_by_key(|b| return cmp::Reverse(b.bytes));
	let peak = TAG_PEAK
		.lock()
		.map(|m| return m.clone())
		.unwrap_or_default();
	let mut s = String::from(
		"\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500} GPU MEMORY LEDGER \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\n",
	);
	let mut total_live = 0usize;
	for tb in &live {
		total_live += tb.bytes;
		let pk = peak.get(tb.tag).copied().unwrap_or(0);
		writeln!(
			s,
			"  {:<14} live {:>11}  peak {:>11}",
			tb.tag,
			fmt_bytes(tb.bytes),
			fmt_bytes(pk)
		)
		.unwrap_or_default();
	}
	writeln!(s, "  {:<14} live {:>11}", "TOTAL", fmt_bytes(total_live)).unwrap_or_default();
	writeln!(
		s,
		"  transfers  H2D {} ({} calls)  D2H {} ({} calls)  D2D {} ({} calls)",
		fmt_bytes(H2D_BYTES.load(Ordering::Relaxed)),
		H2D_CALLS.load(Ordering::Relaxed),
		fmt_bytes(D2H_BYTES.load(Ordering::Relaxed)),
		D2H_CALLS.load(Ordering::Relaxed),
		fmt_bytes(D2D_BYTES.load(Ordering::Relaxed)),
		D2D_CALLS.load(Ordering::Relaxed),
	)
	.unwrap_or_default();
	let a = usize::try_from(callspy::MALLOC_ASYNC.load(Ordering::Relaxed)).unwrap_or(0);
	let f = FREE_TOTAL.load(Ordering::Relaxed);
	writeln!(
		s,
		"  device     allocs {a}  frees {f}  live-buffers {}",
		a.saturating_sub(f)
	)
	.unwrap_or_default();

	s += &ledger_vramspy_section(total_live);

	match kernel_fdinfo() {
		Some(m) => {
			writeln!(
				s,
				"  kernel (fdinfo)  vram {}  gtt {}",
				fmt_bytes(usize::try_from(m.vram_kib * 1024).unwrap_or(usize::MAX)),
				fmt_bytes(usize::try_from(m.gtt_kib * 1024).unwrap_or(usize::MAX))
			)
			.unwrap_or_default();
		}
		None => {
			s += "  kernel (fdinfo): unreadable\n";
		}
	}
	s += "\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}";
	return s;
}

#[inline]
pub fn mark_shutting_down() {
	SHUTTING_DOWN.store(1, Ordering::SeqCst);
}

#[derive(Clone, Copy)]
/// Whether device allocation is frozen inside the training loop.
enum Frozen {
	/// Allocation permitted.
	No,
	/// Allocation forbidden; attempts abort.
	Yes,
}

thread_local! {
	static ALLOC_FROZEN: Cell<Frozen> = const { Cell::new(Frozen::No) };
}

#[inline]
pub fn alloc_count_reset() -> usize {
	return ALLOC_COUNT.swap(0, Ordering::Relaxed);
}

#[inline]
pub fn device_alloc_count() -> usize {
	return usize::try_from(callspy::MALLOC_ASYNC.load(Ordering::Relaxed)).unwrap_or(usize::MAX);
}

#[inline]
pub fn device_free_count() -> usize {
	return FREE_TOTAL.load(Ordering::Relaxed);
}

pub struct XferBytes {
	pub h2d: usize,
	pub d2h: usize,
	pub d2d: usize,
}

#[inline]
pub fn xfer_bytes() -> XferBytes {
	return XferBytes {
		h2d: H2D_BYTES.load(Ordering::Relaxed),
		d2h: D2H_BYTES.load(Ordering::Relaxed),
		d2d: D2D_BYTES.load(Ordering::Relaxed),
	};
}

#[inline]
pub fn xfer_calls() -> XferBytes {
	return XferBytes {
		h2d: H2D_CALLS.load(Ordering::Relaxed),
		d2h: D2H_CALLS.load(Ordering::Relaxed),
		d2d: D2D_CALLS.load(Ordering::Relaxed),
	};
}

#[inline]
pub fn alloc_freeze() {
	ALLOC_FROZEN.with(|f| f.set(Frozen::Yes));
}

#[inline]
pub fn alloc_unfreeze() {
	ALLOC_FROZEN.with(|f| f.set(Frozen::No));
}

pub struct AllocGuard {
	/// Keeps the guard neither Send nor Sync.
	_marker: PhantomData<*const ()>,
}

impl AllocGuard {
	#[must_use]
	#[inline]
	pub fn freeze() -> Self {
		alloc_freeze();
		return Self {
			_marker: PhantomData,
		};
	}
}

impl Drop for AllocGuard {
	#[inline]
	fn drop(&mut self) {
		alloc_unfreeze();
	}
}

/// Byte alignment for every arena carve.
const ARENA_ALIGN: usize = 256;
/// Base address of the active device arena, or 0 when none.
static ARENA_BASE: AtomicUsize = AtomicUsize::new(0);
/// Total byte size of the active device arena.
static ARENA_SIZE: AtomicUsize = AtomicUsize::new(0);
/// Bytes already carved from the active arena.
static ARENA_OFFSET: AtomicUsize = AtomicUsize::new(0);

/// The VMM page handles backing the active [`Ownership::Vmm`] arena, keyed by its
/// base address. One arena exists at a time, so this holds at most one entry; the
/// arena slab's drop takes it and hands the handles to [`vmm_unmap_span`]. Stored
/// as addresses (opaque VMM handles carry no memory provenance) so the mutex stays
/// `Send`.
static ARENA_VMM: Mutex<Option<(usize, Vec<usize>)>> = Mutex::new(None);

/// Per-tag bytes carved from the arena since the last drain.
static ARENA_CARVED: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
/// Total aligned bytes carved from the arena since the last drain.
static ARENA_CARVED_ALIGNED: AtomicUsize = AtomicUsize::new(0);

/// Record a carve of `n_bytes` (aligned rounded up) under tag.
#[inline]
pub fn arena_note_carve(tag: &'static str, n_bytes: usize, aligned: usize) {
	let Ok(mut m) = ARENA_CARVED.lock() else {
		ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
		return;
	};
	*m.entry(tag).or_insert(0) += n_bytes;
	drop(m);
	ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
}

/// Carves the weights image buffer at the arena front so it precedes all later allocations.
fn carve_image_front(image: &[f64]) {
	let Some(_first) = image.first() else {
		return;
	};
	let _t = tag_scope("weights");
	if let Err(e) = GpuBuffer::alloc(image.len()) {
		Write::error(format!("arena image carve: {e}"));
	}
}

/// Zeroes the claimed arena then uploads the weights image into its front.
fn commit_with_image(base: *mut c_void, size: usize, image: &[f64]) -> Result<(), HipError> {
	// SAFETY: base points to the just-claimed device arena of size bytes, a valid target for an async memset.
	unsafe {
		memset_dev(base, 0, size, ptr::null_mut())?;
	}
	if let Some(_first) = image.first() {
		let bytes = mem::size_of_val(image);
		H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
		H2D_CALLS.fetch_add(1, Ordering::Relaxed);
		let pin = run_pin(bytes)?;
		par_copy(pin, image.as_ptr().cast::<u8>(), bytes);
		// SAFETY: base is the freshly mapped arena and pin holds the staged bytes.
		unsafe {
			dev_copy(
				base,
				pin.cast::<c_void>().cast_const(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			)?;
		}
	}
	return device_synchronize();
}

/// Page granularity for a VMM-backed reservation (2 MiB, the ROCm VMM minimum).
const VMM_PAGE: usize = 2 << 20;

/// THE single choke point for a VMM-backed contiguous span: reserves a
/// `ceil(bytes/page)`-page VA range and creates + maps one physical handle per
/// page, so the returned base is DETERMINISTICALLY backed — a first-write memset
/// can never page-fault the way a cold `hipMallocAsync` pool suballocation does.
/// Shared by the arena claim and [`crate::tiered`]; every VMM `hipMem*` call lives
/// here and in [`vmm_unmap_span`], nowhere else. Returns `(base, page-handles)`.
pub fn vmm_map_span(bytes: usize) -> Option<(*mut c_void, Vec<*mut c_void>)> {
	let pages = bytes.div_ceil(VMM_PAGE);
	if pages == 0 {
		return None;
	}
	let mut va: *mut c_void = ptr::null_mut();
	callspy::tick(&callspy::MEM_ADDRESS_RESERVE);
	// SAFETY: &raw mut va is a valid out-slot for the reserved base address.
	if check(unsafe { crate::hip::vmm_reserve(&raw mut va, pages * VMM_PAGE) }).is_err() {
		return None;
	}
	let mut handles: Vec<*mut c_void> = Vec::with_capacity(pages);
	for s in 0..pages {
		let mut h: *mut c_void = ptr::null_mut();
		callspy::tick(&callspy::MEM_CREATE);
		// SAFETY: &raw mut h is a valid out-slot for the new physical handle.
		if check(unsafe { crate::hip::vmm_create(&raw mut h, VMM_PAGE) }).is_err() {
			vmm_unmap_span(va, &handles);
			return None;
		}
		// SAFETY: s < pages, so va + s*page stays within the reserved range.
		let slot = unsafe { va.cast::<u8>().add(s * VMM_PAGE).cast::<c_void>() };
		callspy::tick(&callspy::MEM_MAP);
		callspy::tick(&callspy::MEM_SET_ACCESS);
		// SAFETY: slot is a reserved in-range page and h its freshly created handle.
		if check(unsafe { crate::hip::vmm_map_at(slot, VMM_PAGE, h) }).is_err() {
			// SAFETY: h was created just above and is released once here before bailing.
			unsafe { crate::hip::vmm_release(h) };
			vmm_unmap_span(va, &handles);
			return None;
		}
		handles.push(h);
	}
	return Some((va, handles));
}

/// THE single choke point tearing down a [`vmm_map_span`] region: unmaps each
/// page's slot, releases its handle, then frees the reserved VA span. Reserved-only
/// tails (a mid-map failure) free correctly because the address range is freed by
/// the mapped-page count, matching how the caller reserved and mapped.
pub fn vmm_unmap_span(va: *mut c_void, handles: &[*mut c_void]) {
	for (s, &h) in handles.iter().enumerate() {
		// SAFETY: va + s*page is this slot's live mapped VA; unmap once before release.
		let slot = unsafe { va.cast::<u8>().add(s * VMM_PAGE).cast::<c_void>() };
		callspy::tick(&callspy::MEM_UNMAP);
		// SAFETY: slot is mapped and h backs it; unmap then release, once each.
		unsafe {
			crate::hip::vmm_unmap(slot, VMM_PAGE);
			callspy::tick(&callspy::MEM_RELEASE);
			crate::hip::vmm_release(h);
		}
	}
	if !va.is_null() {
		callspy::tick(&callspy::MEM_ADDRESS_FREE);
		// SAFETY: va is the reserved base of handles.len() contiguous mapped pages.
		unsafe { crate::hip::vmm_addr_free(va, handles.len() * VMM_PAGE) };
	}
}

#[inline]
#[must_use]
pub fn claim_device_arena() -> Option<GpuBuffer> {
	return claim_device_arena_with_image(&[]);
}

#[inline]
#[must_use]
pub fn claim_device_arena_bytes(want: usize) -> Option<GpuBuffer> {
	return claim_device_arena_bytes_with_image(want, &[]);
}

#[inline]
#[must_use]
pub fn claim_device_arena_with_image(image: &[f64]) -> Option<GpuBuffer> {
	let grow = vram_free_base().saturating_sub(USER_GB);
	return claim_device_arena_bytes_with_image(grow & !((1 << 21) - 1), image);
}

#[inline]
pub fn claim_device_arena_bytes_with_image(mut want: usize, image: &[f64]) -> Option<GpuBuffer> {
	if ARENA_BASE.load(Ordering::Relaxed) != 0 {
		Write::error(
			"claim_device_arena: a device arena is already active",
		);
		return None;
	}
	let _t = tag_scope("unclaimed");
	while want > (1 << 20i32) {
		match GpuBuffer::claim_map_bytes(want) {
			Some(slab) => {
				set_device_arena(slab.ptr_raw(), want);
				carve_image_front(image);
				if let Err(e) = commit_with_image(slab.ptr_raw(), want, image) {
					Write::error(format!("claim commit: {e}"));
					return None;
				}
				return Some(slab);
			}
			None => want -= want >> 4i32,
		}
	}
	return None;
}

#[inline]
pub fn release_device_arena(slab: GpuBuffer) {
	if ARENA_BASE.load(Ordering::Relaxed) != slab.ptr_addr() {
		Write::error(
			"release_device_arena: slab is not the active claim",
		);
		return;
	}
	if let Err(e) = device_synchronize() {
		Write::error(format!("arena release sync: {e}"));
	}
	set_device_arena(ptr::null_mut(), 0);
	drain_arena_carve();
	drop(slab);
}

/// Folds all arena carve tallies back into the unclaimed tag and clears the per-tag map.
fn drain_arena_carve() {
	drain_arena_carve_map();
	tag_add("unclaimed", ARENA_CARVED_ALIGNED.swap(0, Ordering::Relaxed));
}

/// Subtracts every recorded arena carve from its tag ledger and empties the carve map.
#[inline]
pub fn drain_arena_carve_map() {
	let Ok(mut m) = ARENA_CARVED.lock() else {
		return;
	};
	for tag in m.keys() {
		let bytes = m.get(tag).copied().unwrap_or(0);
		tag_sub(tag, bytes);
	}
	m.clear();
}

/// The single parked run backing awaiting adoption or release across runs.
static PARKED: Mutex<Option<GpuBuffer>> = Mutex::new(None);

/// Monotonic generation counter, bumped each time a run backing is parked.
static PARK_GEN: AtomicUsize = AtomicUsize::new(0);
/// Generation of the currently parked backing; 0 when nothing is parked.
static PARKED_GEN: AtomicUsize = AtomicUsize::new(0);

#[inline]
pub fn park_run_backing(buf: GpuBuffer) {
	let mut g = match PARKED.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if !g.is_none() {
		Write::error(
			"park_run_backing: a parked run backing already exists",
		);
		return;
	}
	*g = Some(buf);
	drop(g);
	PARKED_GEN.store(
		PARK_GEN.fetch_add(1, Ordering::Relaxed) + 1,
		Ordering::Relaxed,
	);
}

#[inline]
pub fn live_parked_gen() -> Option<usize> {
	return num::NonZeroUsize::new(PARKED_GEN.load(Ordering::Relaxed))
		.map(num::NonZeroUsize::get);
}

/// Classification of a taken parked buffer relative to the active arena.
enum ParkKind {
	/// The parked buffer is the registered device arena.
	Registered,
	/// The parked buffer is a foreign pool allocation, not the arena.
	Foreign,
}

/// Decided action for a parked backing during adoption.
enum Disposition {
	/// Reuse the parked arena in place for this run.
	Adopt,
	/// Release the arena because the parked backing is too small.
	ReleaseArena,
	/// Drop the foreign parked buffer and trim the pool.
	DropForeign,
}

/// Take the parked backing and decide whether to adopt, release, or drop it.
#[inline]
pub fn adopt_run_backing_inner(need: usize) -> Option<GpuBuffer> {
	let parked = match PARKED.lock() {
		Ok(mut g) => g.take(),
		Err(p) => p.into_inner().take(),
	}?;
	PARKED_GEN.store(0, Ordering::Relaxed);
	let kind = match ARENA_BASE.load(Ordering::Relaxed).cmp(&parked.ptr_addr()) {
		cmp::Ordering::Equal => ParkKind::Registered,
		cmp::Ordering::Less | cmp::Ordering::Greater => ParkKind::Foreign,
	};
	let decision = match kind {
		ParkKind::Foreign => Disposition::DropForeign,
		ParkKind::Registered => match parked.len().cmp(&need) {
			cmp::Ordering::Less => Disposition::ReleaseArena,
			cmp::Ordering::Equal | cmp::Ordering::Greater => Disposition::Adopt,
		},
	};
	match decision {
		Disposition::DropForeign => {
			if let Err(e) = device_synchronize() {
				Write::error(format!("parked release sync: {e}"));
			}
			drop(parked);
			pool_trim();
			return None;
		}
		Disposition::ReleaseArena => {
			release_device_arena(parked);
			return None;
		}
		Disposition::Adopt => {
			drain_arena_carve();
			ARENA_OFFSET.store(0, Ordering::Relaxed);
			return Some(parked);
		}
	}
}

#[inline]
#[must_use]
pub fn adopt_run_backing(need: usize) -> Option<GpuBuffer> {
	return adopt_run_backing_with_image(need, &[]);
}

#[inline]
#[must_use]
pub fn adopt_run_backing_with_image(need: usize, image: &[f64]) -> Option<GpuBuffer> {
	let slab = adopt_run_backing_inner(need)?;
	carve_image_front(image);
	if let Err(e) = commit_with_image(slab.ptr_raw(), slab.len(), image) {
		Write::error(format!("adopt commit: {e}"));
		return None;
	}
	return Some(slab);
}

#[inline]
pub fn release_run_backing() {
	let parked = match PARKED.lock() {
		Ok(mut g) => g.take(),
		Err(p) => p.into_inner().take(),
	};
	PARKED_GEN.store(0, Ordering::Relaxed);
	let Some(b) = parked else {
		return;
	};
	match ARENA_BASE.load(Ordering::Relaxed).cmp(&b.ptr_addr()) {
		cmp::Ordering::Equal => release_device_arena(b),
		cmp::Ordering::Less | cmp::Ordering::Greater => {
			if let Err(e) = device_synchronize() {
				Write::error(format!("parked release sync: {e}"));
			}
			drop(b);
		}
	}
}

pub struct Stage {
	/// Host-side f64 staging buffer, alignment-padded per push.
	host: Vec<f64>,
}

/// Arena alignment expressed in f64 elements.
const STAGE_ALIGN_F64: usize = ARENA_ALIGN >> 3;

impl Default for Stage {
	#[inline]
	fn default() -> Self {
		return Self::new();
	}
}

impl Stage {
	#[inline]
	#[must_use]
	pub const fn new() -> Self {
		return Self { host: Vec::new() };
	}

	/// Pads the host buffer up to the next f64 alignment boundary.
	fn pad(&mut self) {
		let rem = self.host.len() & (STAGE_ALIGN_F64 - 1);
		let add = (STAGE_ALIGN_F64 - rem) & (STAGE_ALIGN_F64 - 1);
		self.host.resize(self.host.len() + add, 0.0f64);
	}

	#[inline]
	pub fn push(&mut self, data: &[f64]) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.extend_from_slice(data);
		return off;
	}

	#[inline]
	pub fn reserve(&mut self, n_floats: usize) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + n_floats, 0.0f64);
		return off;
	}

	#[inline]
	#[must_use]
	pub const fn len_floats(&self) -> usize {
		return self.host.len();
	}

	#[must_use]
	#[inline]
	pub fn into_host(self) -> Vec<f64> {
		return self.host;
	}

	#[must_use]
	#[inline]
	pub const fn is_empty(&self) -> bool {
		return self.host.is_empty();
	}
}

/// Direction of a ledgered device transfer.
enum Dir {
	/// Host to device.
	H2D,
	/// Device to host.
	D2H,
	/// Device to device.
	D2D,
}

/// # Errors
/// Returns [`HipError`] if the underlying HIP memcpy fails.
#[inline]
pub unsafe fn xfer(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	let dir = Some(kind)
		.filter(|k| return *k == HIP_MEMCPY_H2D)
		.map(|_k| return Dir::H2D)
		.or_else(|| {
			return Some(kind)
				.filter(|k| return *k == HIP_MEMCPY_D2H)
				.map(|_k| return Dir::D2H);
		})
		.unwrap_or(Dir::D2D);
	match dir {
		Dir::H2D => {
			H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
			H2D_CALLS.fetch_add(1, Ordering::Relaxed);
			// SAFETY: xfer's caller guarantees dst/src are valid for bytes and stream is a live HIP stream.
			unsafe { return h2d_pinned(dst, src, bytes, stream) }
		}
		Dir::D2H => {
			D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
			D2H_CALLS.fetch_add(1, Ordering::Relaxed);
			callspy::tick(&callspy::XFER_ASYNC);
			// SAFETY: xfer's caller guarantees dst/src are valid for bytes and stream is a live HIP stream.
			unsafe { return dev_copy(dst, src, bytes, kind, stream) }
		}
		Dir::D2D => {
			D2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
			D2D_CALLS.fetch_add(1, Ordering::Relaxed);
			callspy::tick(&callspy::XFER_ASYNC);
			// SAFETY: xfer's caller guarantees dst/src are valid for bytes and stream is a live HIP stream.
			unsafe { return dev_copy(dst, src, bytes, kind, stream) }
		}
	}
}

/// Records and issues one ledgered async `hipMemcpyAsync`.
unsafe fn dev_copy(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	callspy::tick(&callspy::MEMCPY_ASYNC);
	// SAFETY: dev_copy's caller guarantees dst/src are valid for bytes and stream is a live HIP stream.
	return check(unsafe { hipMemcpyAsync(dst, src, bytes, kind, stream) });
}

#[inline]
pub fn par_touch(v: &mut [u8]) {
	let threads = thread::available_parallelism().map_or(1, num::NonZeroUsize::get);

	let per = v.len().div_ceil(threads).div_ceil(4096) * 4096;
	thread::scope(|sc| {
		for ch in v.chunks_mut(per.max(4096)) {
			sc.spawn(|| {
				for i in (0..ch.len()).step_by(4096) {
					// SAFETY: i < ch.len() (step_by over 0..ch.len()); .add stays in bounds.
					let p = unsafe { ch.as_mut_ptr().add(i) };
					// SAFETY: p is a valid, in-bounds, writable pointer for one u8.
					unsafe {
						ptr::write_volatile(p, 0);
					}
				}
			});
		}
	});
}

#[inline]
pub fn par_copy(dst: *mut u8, src: *const u8, bytes: usize) {
	let threads = thread::available_parallelism().map_or(1, num::NonZeroUsize::get);

	let per = bytes.div_ceil(threads);
	let d = dst.expose_provenance();
	let s0 = src.expose_provenance();
	thread::scope(|sc| {
		for t in 0..threads {
			let off = t * per;
			let remaining = match off.cmp(&bytes) {
				cmp::Ordering::Less => bytes - off,
				cmp::Ordering::Equal | cmp::Ordering::Greater => break,
			};
			let len = per.min(remaining);
			sc.spawn(move || {
				let sp = ptr::with_exposed_provenance::<u8>(s0 + off);
				let dp = ptr::with_exposed_provenance_mut::<u8>(d + off);
				// SAFETY: sp and dp cover len in-bounds bytes of the src/dst buffers; per-thread ranges are disjoint.
				unsafe {
					ptr::copy_nonoverlapping(sp, dp, len);
				}
			});
		}
	});
}

/// Fixed size of the pinned H2D bounce staging buffer.
const BOUNCE_BYTES: usize = 64 << 20;
/// Address of the lazily allocated pinned bounce buffer, 0 when unset.
struct BounceAddr {
	/// Address of the pinned bounce allocation, 0 when unset.
	addr: usize,
}
/// Serializes access to the single shared pinned H2D bounce buffer.
static BOUNCE: Mutex<BounceAddr> = Mutex::new(BounceAddr { addr: 0 });

/// Reusable pinned host buffer backing out-of-core run transfers.
struct PinBuf {
	/// Address of the pinned allocation, 0 when none.
	ptr: usize,
	/// Capacity of the pinned allocation in bytes.
	cap: usize,
}

/// Process-wide reusable run pin buffer.
static RUN_PIN: Mutex<PinBuf> = Mutex::new(PinBuf { ptr: 0, cap: 0 });

/// Grows the pin buffer to hold `bytes` when it is smaller and returns its pointer.
fn pin_ensure(g: &mut PinBuf, bytes: usize) -> Result<*mut u8, HipError> {
	if g.cap < bytes {
		if let Some(_old) = num::NonZeroUsize::new(g.ptr) {
			// SAFETY: g.ptr is a live host_malloc allocation (NonZero-guarded) freed exactly once here.
			drop(unsafe { host_free(ptr::with_exposed_provenance_mut::<c_void>(g.ptr)) });
		}
		let raw = host_malloc(bytes, 0).map_err(|e| {
			Write::error(format!("run_pin host_malloc: {e}"));
			return e;
		})?;
		g.ptr = raw.expose_provenance();
		g.cap = bytes;
	}
	return Ok(ptr::with_exposed_provenance_mut::<u8>(g.ptr));
}

/// Return the shared run-pin host buffer, grown to at least the requested bytes.
#[inline]
pub fn run_pin(bytes: usize) -> Result<*mut u8, HipError> {
	let mut g = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	return pin_ensure(&mut g, bytes);
}

/// Base address and length of the live bounce buffer.
pub struct BounceRange {
	/// Start address of the bounce buffer.
	pub base: usize,
	/// Length of the bounce buffer in bytes.
	pub len: usize,
}

/// Return the live bounce buffer range, or None when unallocated.
#[inline]
pub fn bounce_range() -> Option<BounceRange> {
	let base = BOUNCE.lock().ok()?.addr;
	return num::NonZeroUsize::new(base).map(|nz| {
		return BounceRange {
			base: nz.get(),
			len: BOUNCE_BYTES,
		};
	});
}

/// A recently noted device VA range, for fault-autopsy lookup.
struct Range {
	/// Start address of the range.
	base: usize,
	/// Length of the range in bytes.
	len: usize,
	/// Human-readable label for the range.
	what: &'static str,
}

/// Ring of recently noted device VA ranges for fault autopsy.
static RECENT_RANGES: Mutex<Vec<Range>> = Mutex::new(Vec::new());

/// Record a device VA range so `locate_va` can later name it.
pub(crate) fn note_range(base: usize, len: usize, what: &'static str) {
	let Ok(mut r) = RECENT_RANGES.lock() else {
		return;
	};
	if let Some(_full) = (r.len() >= 256).then_some(()) {
		r.remove(0);
	}
	r.push(Range { base, len, what });
}

/// Describe which noted range contains the given address, if any.
#[inline]
pub fn locate_va(va: usize) -> Option<String> {
	let r = RECENT_RANGES.lock().ok()?;
	return r
		.iter()
		.rev()
		.find(|range| return va >= range.base && va < range.base + range.len)
		.map(|range| {
			return format!(
				"{} [base 0x{:x} len {}] +0x{:x}",
				range.what,
				range.base,
				fmt_bytes(range.len),
				va - range.base
			);
		});
}

/// Releases the pinned H2D bounce buffer if one was allocated.
#[inline]
pub fn free_bounce() {
	let mut guard = match BOUNCE.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if let Some(_live) = num::NonZeroUsize::new(guard.addr) {
		// SAFETY: guard holds a live pinned allocation from host_malloc, freed once here.
		drop(unsafe { host_free(ptr::with_exposed_provenance_mut::<c_void>(guard.addr)) });
		guard.addr = 0;
	}
}

/// Releases the pinned run staging buffer after a device sync.
#[inline]
pub fn free_run_pin() {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if let Some(_live) = num::NonZeroUsize::new(guard.ptr) {
		drop(device_synchronize());
		// SAFETY: guard.ptr holds a live pinned allocation from host_malloc, freed once here.
		drop(unsafe { host_free(ptr::with_exposed_provenance_mut::<c_void>(guard.ptr)) });
		guard.ptr = 0;
		guard.cap = 0;
	}
}

/// Streams host bytes to device through the pinned bounce buffer in synced chunks.
///
/// # Errors
/// Returns [`HipError`] if a chunk copy or stream sync fails.
///
/// # Safety
/// `dst` and `src` must be valid for `bytes` and `stream` must be a live HIP stream.
#[inline]
pub unsafe fn h2d_pinned(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	let mut guard = match BOUNCE.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	let base = if let Some(nz) = num::NonZeroUsize::new(guard.addr) {
		nz.get()
	} else {
		let fresh = host_malloc(BOUNCE_BYTES, 0)?.expose_provenance();
		guard.addr = fresh;
		fresh
	};

	let pin = ptr::with_exposed_provenance_mut::<u8>(base);
	let mut done = 0usize;
	while done < bytes {
		let chunk = BOUNCE_BYTES.min(bytes - done);
		// SAFETY: done < bytes, so src + done stays within the source allocation.
		par_copy(pin, unsafe { src.cast::<u8>().add(done) }, chunk);
		// SAFETY: done < bytes, so dst + done stays within the destination allocation.
		let dst_off = unsafe { dst.cast::<u8>().add(done) };
		// SAFETY: dst_off and pin are valid for chunk bytes and stream is a live HIP stream.
		unsafe {
			dev_copy(
				dst_off.cast::<c_void>(),
				pin.cast::<c_void>().cast_const(),
				chunk,
				HIP_MEMCPY_H2D,
				stream,
			)
		}?;
		callspy::tick(&callspy::STREAM_SYNCHRONIZE);
		// SAFETY: stream is a valid HIP stream for this run.
		check(unsafe { hipStreamSynchronize(stream) })?;
		done += chunk;
	}
	drop(guard);
	return Ok(());
}

pub const BOUNCE_LIMIT: usize = BOUNCE_BYTES;

pub struct ExitD2H {
	/// Held pin-buffer lock keeping the staging region alive until finish.
	_guard: MutexGuard<'static, PinBuf>,
	/// Address of the pinned staging buffer holding the download.
	pin: usize,
	/// Byte length of the pending download.
	bytes: usize,
}

impl ExitD2H {
	#[inline]
	pub fn finish(self, dst: &mut [f64]) {
		if mem::size_of_val(dst) != self.bytes {
			Write::error(format!(
				"ExitD2H::finish size mismatch: {} vs {}",
				mem::size_of_val(dst),
				self.bytes
			));
			return;
		}
		par_copy(
			dst.as_mut_ptr().cast::<u8>(),
			ptr::with_exposed_provenance::<u8>(self.pin),
			self.bytes,
		);
	}
}

/// # Errors
/// Errors when the device-to-host copy fails to enqueue.
#[inline]
#[expect(
	clippy::significant_drop_tightening,
	reason = "the RUN_PIN guard must outlive the enqueued D2H copy; it rides in the returned ExitD2H, so it cannot drop earlier"
)]
pub unsafe fn exit_d2h_enqueue(src: *const c_void, bytes: usize) -> Result<ExitD2H, HipError> {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	let pin = pin_ensure(&mut guard, bytes)?;
	let pin_addr = guard.ptr;
	D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
	D2H_CALLS.fetch_add(1, Ordering::Relaxed);
	// SAFETY: pin and src are valid pointers for the bytes copied.
	unsafe {
		dev_copy(
			pin.cast::<c_void>(),
			src,
			bytes,
			HIP_MEMCPY_D2H,
			ptr::null_mut(),
		)
	}?;
	return Ok(ExitD2H {
		_guard: guard,
		pin: pin_addr,
		bytes,
	});
}

/// Safe wrapper over [`exit_d2h_enqueue`] taking the source as a buffer plus a
/// byte count. Bounds-checks against the buffer so callers need no unsafe.
///
/// # Errors
/// Returns [`HipError`] if the enqueue fails.
pub fn exit_d2h_enqueue_buf(src: &GpuBuffer, bytes: usize) -> Result<ExitD2H, HipError> {
	assert!(
		bytes <= src.len,
		"exit_d2h_enqueue_buf: {bytes} bytes exceeds buffer len {}",
		src.len
	);
	// SAFETY: the assert proves src covers `bytes`; ptr_raw is a valid device pointer.
	return unsafe { exit_d2h_enqueue(src.ptr_raw().cast_const(), bytes) };
}

/// Process-global two-slot pinned host buffer used for the deferred 8-byte
/// metric-scalar downloads. Stored as an address so it stays `Send`.
static PINNED_PAIR: Mutex<usize> = Mutex::new(0);

/// Address of the pinned two-`f64` pair, allocating it on first use. Returns 0
/// if the pinned allocation fails.
fn pinned_pair_addr() -> usize {
	let mut g = match PINNED_PAIR.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if *g == 0 {
		match crate::hip::host_malloc(16, 0) {
			Ok(p) => *g = p.addr(),
			Err(e) => {
				Write::error(format!("pinned scalars: {e}"));
				return 0;
			}
		}
	}
	return *g;
}

/// Download 8 bytes from `src` into pinned scalar `slot` (0 or 1) on `stream`,
/// deferred (no sync). Safe: `slot` is masked to the two-slot pair and the copy
/// size is fixed at one `f64`.
pub fn pinned_download(slot: usize, src: &GpuBuffer, stream: &crate::hip::Stream) {
	let base = pinned_pair_addr();
	if base == 0 {
		return;
	}
	let dst = (base + (slot & 1) * size_of::<f64>()) as *mut c_void;
	// SAFETY: dst is one f64 slot inside the 16-byte pinned pair; src covers 8
	// bytes (a scalar buffer); stream is a valid HIP stream handle.
	let r = unsafe {
		xfer(
			dst,
			src.ptr_raw().cast_const(),
			size_of::<f64>(),
			HIP_MEMCPY_D2H,
			stream.raw(),
		)
	};
	if let Err(e) = r {
		Write::error(format!("pinned download: {e}"));
	}
}

/// Read pinned scalar `slot` (0 or 1). Returns 0.0 if the pinned pair could not
/// be allocated. The caller must have synchronized the download stream first.
#[must_use]
pub fn pinned_read(slot: usize) -> f64 {
	let base = pinned_pair_addr();
	if base == 0 {
		return 0.0;
	}
	// SAFETY: base points at a live 16-byte pinned allocation; slot&1 stays in it.
	return unsafe { *((base + (slot & 1) * size_of::<f64>()) as *const f64) };
}

/// Free the pinned scalar pair after a device sync. Called during teardown.
pub fn free_pinned_pair() {
	let mut g = match PINNED_PAIR.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if *g != 0 {
		let _ = device_synchronize();
		// SAFETY: the address came from host_malloc and is freed exactly once here.
		let _ = unsafe { crate::hip::host_free(*g as *mut c_void) };
		*g = 0;
	}
}

/// Ledgered async device memset on the given stream.
pub(crate) unsafe fn memset_dev(
	dst: *mut c_void,
	value: i32,
	bytes: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	callspy::tick(&callspy::MEMSET_ASYNC);
	// SAFETY: dst points to the given bytes of device memory on a valid stream.
	return check(unsafe { hipMemsetAsync(dst, value, bytes, stream) });
}

/// One-shot guard for process-wide device initialization.
static DEVICE_INIT: Once = Once::new();

/// atexit hook releasing the parked birth arena claim at process exit.
#[inline]
pub extern "C" fn release_birth_claim_at_exit() {
	release_run_backing();
}

/// Performs process-wide GPU device initialization exactly once.
pub(crate) fn device_init_once() {
	use crate::gate;
	use crate::hw;
	gate::acquire();
	DEVICE_INIT.call_once(|| {
		disable_sdma_once();
		if let Some(e) = set_pool_retain(0).err() {
			Write::error(format!("GPU pool retain failed: {e}"));
		}
		register_fault_autopsy_once();
		hw::spawn_thrash_watchdog();
	});
}

#[inline]
pub fn pool_trim() {
	if let Err(e) = device_synchronize() {
		Write::error(format!("pool_trim sync: {e}"));
	}
	if let Err(e) = trim_mempool(0) {
		Write::error(format!("pool_trim: {e}"));
	}
}

pub const USER_GB: usize = 1 << 30;

#[must_use]
#[inline]
pub fn vram_free_base() -> usize {
	let hip_free = match mem_info() {
		Ok(mi) => mi.free,
		Err(e) => {
			Write::error(format!("hipMemGetInfo: {e}"));
			sysfs_vram_free().unwrap_or(0)
		}
	};
	let sys_free = sysfs_vram_free().unwrap_or(hip_free);
	let slack = match pool_slack(0) {
		Ok(s) => s,
		Err(e) => {
			Write::error(format!("pool_slack: {e}"));
			0
		}
	};
	return hip_free.min(sys_free).saturating_sub(slack);
}

#[must_use]
#[inline]
pub fn claimable_bytes() -> usize {
	return arena_remaining() + vram_free_base().saturating_sub(USER_GB);
}

#[inline]
pub fn set_device_arena(base: *mut c_void, size: usize) {
	ARENA_OFFSET.store(0, Ordering::Relaxed);
	ARENA_SIZE.store(size, Ordering::Relaxed);
	ARENA_BASE.store(base.addr(), Ordering::Relaxed);
}

/// Human-readable arena carve-miss diagnostic: a GPU-busy and VRAM free/total
/// context line followed by the needs/has/short accounting, all in MB/GB to two
/// decimals. Composed where the exact asked size and remaining claim are known
/// so the inference path can propagate it instead of aborting.
#[must_use]
pub fn carve_miss_message(asked_bytes: usize) -> String {
	let mb = |b: usize| return b as f64 / (1u64 << 20i32) as f64;
	let gb = |b: usize| return b as f64 / (1u64 << 30i32) as f64;
	let remain = arena_remaining();
	let short = asked_bytes.saturating_sub(remain);
	let mi = mem_info().unwrap_or(MemInfo { free: 0, total: 0 });
	let busy = gpu_busy_percent().map_or_else(|| return "?".to_owned(), |b| return b.to_string());
	return format!(
		"GPU: {busy}%  VRAM: {:.2} GB free / {:.2} GB total\nmodel needs {:.2} MB, arena has {:.2} MB ({:.2} MB short)",
		gb(mi.free),
		gb(mi.total),
		mb(asked_bytes),
		mb(remain),
		mb(short),
	);
}

/// Reads the first `gpu_busy_percent` under `/sys/class/drm`, or `None` when the
/// counter is unavailable; used only to annotate diagnostics, never for a fit.
#[must_use]
fn gpu_busy_percent() -> Option<u32> {
	for card in fs::read_dir("/sys/class/drm").into_iter().flatten().flatten() {
		let p = card.path().join("device/gpu_busy_percent");
		let Ok(text) = fs::read_to_string(&p) else {
			continue;
		};
		if let Ok(v) = text.trim().parse::<u32>() {
			return Some(v);
		}
	}
	return None;
}

#[inline]
pub fn arena_remaining() -> usize {
	return num::NonZeroUsize::new(ARENA_BASE.load(Ordering::Relaxed)).map_or(0, |_base| {
		return ARENA_SIZE
			.load(Ordering::Relaxed)
			.saturating_sub(ARENA_OFFSET.load(Ordering::Relaxed));
	});
}

#[inline]
pub fn device_arena_active() -> bool {
	return ARENA_BASE.load(Ordering::Relaxed) != 0;
}

#[inline]
pub fn probe_ceiling(mut probe_survives: impl FnMut(usize) -> bool) -> Option<usize> {
	let mut want = vram_free_base().saturating_sub(USER_GB) & !((1 << 21i32) - 1);
	while want > (1 << 30i32) {
		if probe_survives(want) {
			Write::line(
				gpu,
				format!("claim probe: {} (probe-verified)", fmt_bytes(want)),
			);
			return Some(want);
		}
		Write::line(
			gpu,
			format!("claim probe: {} unmappable, backing off", fmt_bytes(want)),
		);
		want -= want >> 4i32;
	}
	return None;
}

/// Child entry for the host-RAM headroom probe. When `RAM_PROBE` names a byte
/// count, commits that many host bytes and touches every page so the commit
/// becomes real resident memory, mirroring the VRAM probe's device allocation.
/// Returns `Some(0)` if the process survived the commit, `Some(2)` on a parse
/// failure, and `None` when the env var is unset (not a probe child). If the
/// kernel OOM-kills the child mid-commit it dies with SIGKILL and the parent's
/// [`probe_ram_ceiling`] loop reads the non-success status as "unmappable".
#[inline]
pub fn ram_probe_ask() -> Option<i32> {
	let sz = std::env::var_os("RAM_PROBE")?;
	let Ok(n) = sz.to_string_lossy().parse::<usize>() else {
		return Some(2);
	};
	let _volunteered = std::fs::write("/proc/self/oom_score_adj", "1000");
	let mut v = vec![0u8; n];
	let mut i = 0usize;
	while i < v.len() {
		v[i] = 1u8;
		i += 4096;
	}
	core::hint::black_box(v.as_ptr());
	drop(v);
	return Some(0);
}

/// Spawns one RAM probe child that must commit and touch `want` bytes, returning
/// whether it survived. Core dumps are disabled in the child so an OOM-kill is
/// silent, exactly as the VRAM claim probe does.
fn spawn_ram_probe(want: usize) -> bool {
	let Ok(exe) = std::env::current_exe() else {
		return false;
	};
	let mut c = std::process::Command::new(exe);
	c.stdin(std::process::Stdio::null());
	c.env("RAM_PROBE", want.to_string());
	crate::sys::disable_core_dumps(&mut c);
	return c.status().map(|s| return s.success()).unwrap_or(false);
}

/// Probe-verified host-RAM budget. Spawns a child per candidate size that must
/// commit and touch that many bytes, backing off by 1/16 until one survives,
/// exactly mirroring [`probe_ceiling`]'s VRAM claim probe. Returns the largest
/// survivable size, or `None` when nothing above 1 GB survives the commit.
#[inline]
pub fn probe_ram_ceiling(guess: usize) -> Option<usize> {
	let mut want = guess & !((1 << 21i32) - 1);
	while want > (1 << 30i32) {
		if spawn_ram_probe(want) {
			Write::line(
				gpu,
				format!("ram probe: {} (probe-verified)", fmt_bytes(want)),
			);
			return Some(want);
		}
		Write::line(
			gpu,
			format!("ram probe: {} unmappable, backing off", fmt_bytes(want)),
		);
		want -= want >> 4i32;
	}
	return None;
}

/// Image packed channel sample format, applied per channel after unpacking.
/// - `Unorm`   raw / (2^bits - 1)
/// - `Snorm`   signext then / (2^(bits-1) - 1), clamped to -1
/// - `Uscaled` raw as f32 (unsigned integer value)
/// - `Sscaled` signext as f32 (signed integer value)
/// - `Uint`    raw as f32
/// - `Sint`    signext as f32
/// - `Float`   IEEE/mini float of the channel width (10/11/16/32)
/// - `Srgb`    `Unorm` then the sRGB EOTF on colour channels (alpha stays linear)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImgFmt {
	Unorm,
	Snorm,
	Uscaled,
	Sscaled,
	Uint,
	Sint,
	Float,
	Srgb,
}

/// Image packed channel layout: the per-channel bit widths, listed low-order
/// channel first. Channels pack little-endian within the block, the first
/// listed channel occupying the least-significant bits.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ImgLayout {
	L8,
	L8_8,
	L8_8_8_8,
	L16,
	L16_16,
	L16_16_16_16,
	L32,
	L32_32,
	L32_32_32,
	L32_32_32_32,
	L5_6_5,
	L1_5_5_5,
	L5_5_5_1,
	L10_10_10_2,
	L2_10_10_10,
	L10_11_11,
	L11_11_10,
}

impl ImgLayout {
	/// Per-channel bit widths, low-order channel first.
	pub fn channels(self) -> &'static [u32] {
		return match self {
			ImgLayout::L8 => &[8],
			ImgLayout::L8_8 => &[8, 8],
			ImgLayout::L8_8_8_8 => &[8, 8, 8, 8],
			ImgLayout::L16 => &[16],
			ImgLayout::L16_16 => &[16, 16],
			ImgLayout::L16_16_16_16 => &[16, 16, 16, 16],
			ImgLayout::L32 => &[32],
			ImgLayout::L32_32 => &[32, 32],
			ImgLayout::L32_32_32 => &[32, 32, 32],
			ImgLayout::L32_32_32_32 => &[32, 32, 32, 32],
			ImgLayout::L5_6_5 => &[5, 6, 5],
			ImgLayout::L1_5_5_5 => &[1, 5, 5, 5],
			ImgLayout::L5_5_5_1 => &[5, 5, 5, 1],
			ImgLayout::L10_10_10_2 => &[10, 10, 10, 2],
			ImgLayout::L2_10_10_10 => &[2, 10, 10, 10],
			ImgLayout::L10_11_11 => &[10, 11, 11],
			ImgLayout::L11_11_10 => &[11, 11, 10],
		};
	}

}

/// THE dtype: every element type any part of the system names — device buffer
/// elements, gguf tensor storage, quant blocks, packed ints, mini-floats,
/// image layouts — is a variant here, in gpu-core, the bottom of the DAG.
/// Adding dtype N+1 costs one variant and one match arm. Kernel dispatch
/// ([`Dtype::ffi`]) covers the compute dtypes; codecs in `recipe-infer`'s
/// dequant map the rest onto them.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Dtype {
	F32,
	#[default]
	F64,
	F16,
	BF16,
	Q4_0,
	Q4_1,
	Q5_0,
	Q5_1,
	Q8_0,
	Q2K,
	Q3K,
	Q4K,
	Q5K,
	Q6K,
	Iq1S,
	Iq1M,
	Iq2Xxs,
	Iq2Xs,
	Iq2S,
	Iq3Xxs,
	Iq3S,
	Iq4Nl,
	Iq4Xs,
	B8,
	B16,
	B32,
	B64,
	B96,
	B128,
	B256,
	B512,
	B16x2,
	B16x3,
	B16x4,
	I4,
	I8,
	I16,
	I24,
	I32,
	I64,
	U4,
	U8,
	U16,
	U24,
	U32,
	U64,
	I4x8,
	U4x8,
	I8x4,
	U8x4,
	I16x2,
	U16x2,
	F4e2m1,
	F6e2m3,
	F6e3m2,
	F8e4m3,
	F8e5m2,
	F8e4m3fnuz,
	F8e5m2fnuz,
	BF8,
	Tf32,
	Xf32,
	F4x2e2m1,
	F4x4e2m1,
	F6x2e2m3,
	F6x4e2m3,
	F6x2e3m2,
	F6x4e3m2,
	F8x2e4m3,
	F8x4e4m3,
	F8x2e5m2,
	F8x4e5m2,
	F8x2e4m3fnuz,
	F8x4e4m3fnuz,
	F8x2e5m2fnuz,
	F8x4e5m2fnuz,
	F16x2,
	BF16x2,
	F32x2,
	F64x2,
	Byte,
	Short,
	Dword,
	Dwordx2,
	Dwordx3,
	Dwordx4,
	Dwordx8,
	Dwordx16,
	Img(ImgLayout, ImgFmt),
}


impl Dtype {
      #[inline]
      #[must_use]
      /// Bytes per element for the scalar compute dtypes. Block-quantized, packed,
      /// and image dtypes are byte-granular here (1): their real geometry lives in
      /// the codec (`block_bytes`/`block_elems`), never in flat element math.
      pub const fn elem_size(self) -> usize {
            return match self {
                  Self::F64 | Self::F64x2 => 8,
                  Self::F32 | Self::Tf32 | Self::Xf32 => 4,
                  Self::F16 | Self::BF16 => 2,
                  _other => 1,
            };
      }

      #[inline]
      #[must_use]
      /// Kernel-ABI dispatch code (dtype_dispatch.h): 0 = f64, 1 = f32. Every
      /// other dtype is NOT kernel-dispatchable — codecs convert it to a compute
      /// dtype first — and returns -1 so a stray launch fails loudly in review,
      /// never silently as f64.
      pub const fn ffi(self) -> i32 {
            return match self {
                  Self::F32 => 1,
                  Self::F64 => 0,
                  _other => -1,
            };
      }

      #[inline]
      #[must_use]
      /// Convert-kernel ABI code (`convert.h`), the dtype half of the
      /// [`crate::infer_ops::gpu_convert`] flipper. Shares 0 = f64 / 1 = f32
      /// with [`Self::ffi`] so a buffer's compute code and convert code never
      /// disagree. `-1` = no kernel reads or writes this dtype yet, which
      /// `gpu_convert` reports by name rather than reinterpreting bytes.
      pub const fn conv(self) -> i32 {
            return match self {
                  Self::F64 => 0,
                  Self::F32 => 1,
                  Self::F16 => 2,
                  Self::BF16 => 3,
                  Self::Q4_0 => 10,
                  Self::Q4_1 => 11,
                  Self::Q5_0 => 12,
                  Self::Q5_1 => 13,
                  Self::Q8_0 => 14,
                  Self::Q2K => 20,
                  Self::Q3K => 21,
                  Self::Q4K => 22,
                  Self::Q5K => 23,
                  Self::Q6K => 24,
                  _other => -1,
            };
      }

      #[inline]
      #[must_use]
      /// Elements per storage block: 1 for the element-granular dtypes, 32 for
      /// the legacy quants, 256 for the K-quant superblocks. The count a
      /// conversion length must divide evenly.
      pub const fn block_elems(self) -> usize {
            return match self {
                  Self::Q4_0 | Self::Q4_1 | Self::Q5_0 | Self::Q5_1 | Self::Q8_0 => 32,
                  Self::Q2K | Self::Q3K | Self::Q4K | Self::Q5K | Self::Q6K => 256,
                  _other => 1,
            };
      }

      #[inline]
      #[must_use]
      /// Bytes per storage block, matching the ggml on-disk layouts byte for
      /// byte. For element-granular dtypes this is [`Self::elem_size`].
      pub const fn block_bytes(self) -> usize {
            return match self {
                  Self::Q4_0 => 18,
                  Self::Q4_1 => 20,
                  Self::Q5_0 => 22,
                  Self::Q5_1 => 24,
                  Self::Q8_0 => 34,
                  Self::Q2K => 84,
                  Self::Q3K => 110,
                  Self::Q4K => 144,
                  Self::Q5K => 176,
                  Self::Q6K => 210,
                  _other => self.elem_size(),
            };
      }
}

/// Whether a buffer owns its device allocation or merely borrows one.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub enum Ownership {
	/// Owns a pool allocation; freed on drop.
	Pool,
	/// Borrows memory owned elsewhere; never freed.
	Borrow,
	/// Owns the VMM-backed device arena ([`vmm_map_span`]); its page handles live
	/// in [`ARENA_VMM`] (one arena at a time) and are unmapped, released, and the
	/// VA freed on drop.
	Vmm,
}

pub struct GpuBuffer {
	/// Raw device allocation pointer, null once dropped.
	pub ptr: *mut c_void,
	/// Length of the allocation in bytes.
	pub len: usize,
	/// Ownership mode governing whether drop frees the pointer.
	pub owned: Ownership,
	/// Ledger tag the buffer's bytes are accounted under.
	pub tag: &'static str,
	/// Element type every byte computation on this buffer derives from.
	pub dt: Dtype,
}

// SAFETY: GpuBuffer owns a raw device pointer with no thread-affine handles, so moving it across threads is sound.
unsafe impl Send for GpuBuffer {}
// SAFETY: shared access exposes only immutable pointer metadata; concurrent device use is externally synchronized.
unsafe impl Sync for GpuBuffer {}

impl GpuBuffer {
	#[inline]
	pub const fn borrow(ptr: *mut c_void, len: usize) -> Self {
		return Self {
			ptr,
			len,
			owned: Ownership::Borrow,
			tag: "borrow",
			dt: Dtype::F64,
		};
	}

	#[inline]
	#[must_use]
	pub const fn dtype(&self) -> Dtype {
		return self.dt;
	}

	#[inline]
	#[must_use]
	pub const fn elem_size(&self) -> usize {
		return self.dt.elem_size();
	}

	#[inline]
	#[must_use]
	pub const fn as_dtype(mut self, dt: Dtype) -> Self {
		self.dt = dt;
		return self;
	}

	/// # Errors
	/// Errors when the underlying device allocation fails.
	#[inline]
	pub fn alloc_ty(n_elems: usize, dt: Dtype) -> Result<Self, HipError> {
		return Ok(Self::alloc_bytes(n_elems * dt.elem_size())?.as_dtype(dt));
	}

	#[must_use]
	#[inline]
	pub const fn is_pool_owned(&self) -> bool {
		return matches!(self.owned, Ownership::Pool);
	}

	/// # Errors
	/// Errors when the underlying device allocation fails.
	#[inline]
	pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
		return Self::alloc_bytes(n_floats * mem::size_of::<f64>());
	}

	#[must_use]
	#[inline]
	pub fn try_alloc_bytes(n_bytes: usize) -> Option<Self> {
		return Self::alloc_bytes_inner(n_bytes).ok();
	}

	/// # Errors
	/// Errors when the device allocation fails and no arena carve remains.
	#[inline]
	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		match Self::alloc_bytes_inner(n_bytes) {
			Ok(buf) => return Ok(buf),
			Err(e) => {
				if device_arena_active() {
					return Err(e);
				}
				oom_report(n_bytes);
				return Err(e);
			}
		}
	}

	/// Maps `n_bytes` of pool memory and returns the raw device pointer.
	///
	/// # Errors
	/// Returns [`HipError`] if the pool allocation fails.
	#[inline]
	pub fn map_bytes(n_bytes: usize) -> Result<*mut c_void, HipError> {
		let mut ptr: *mut c_void = ptr::null_mut();
		callspy::tick(&callspy::MALLOC_ASYNC);
		// SAFETY: &raw mut ptr is a valid out-parameter that hipMallocAsync writes the device allocation pointer into.
		check(unsafe { hipMallocAsync(&raw mut ptr, n_bytes, ptr::null_mut()) })?;
		return Ok(ptr);
	}

	/// Carves `n_bytes` from the active arena, claiming the arena first when none is active.
	fn alloc_bytes_inner(n_bytes: usize) -> Result<Self, HipError> {
		device_init_once();
		if ALLOC_FROZEN.with(|f| return !matches!(f.get(), Frozen::No)) {
			Write::error(format!(
				"GPU allocation inside frozen training loop (requested {n_bytes} bytes)"
			));
			return Err(HipError(2));
		}
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(Cell::get);
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
					Ok(_prev) => {
						// SAFETY: off + aligned <= size holds by the loop guard, so base + off stays within the claimed arena slab.
						let ptr = unsafe {
							ptr::with_exposed_provenance_mut::<u8>(base)
								.add(off)
								.cast::<c_void>()
						};
						tag_sub("unclaimed", aligned);
						tag_add(tag, n_bytes);
						arena_note_carve(tag, n_bytes, aligned);
						note_range(ptr.addr(), n_bytes, tag);
						return Ok(Self {
							ptr,
							len: n_bytes,
							owned: Ownership::Borrow,
							tag,
							dt: Dtype::F64,
						});
					}
					Err(cur) => off = cur,
				}
			}
			return Err(HipError(2));
		}
		return claim_device_arena().map_or(Err(HipError(2)), |slab| {
			park_run_backing(slab);
			// SAFETY: release_birth_claim_at_exit is a valid extern "C" fn pointer with no captured state, callable at process exit.
			unsafe {
				libc::atexit(release_birth_claim_at_exit);
			}
			return Self::alloc_bytes_inner(n_bytes);
		});
	}

	/// Maps a fresh pool-owned slab of `n_bytes` to back the device arena claim.
	#[inline]
	pub fn claim_map_bytes(n_bytes: usize) -> Option<Self> {
		device_init_once();
		if ALLOC_FROZEN.with(|f| return !matches!(f.get(), Frozen::No)) {
			Write::error(format!(
				"GPU claim inside frozen training loop (requested {n_bytes} bytes)"
			));
			return None;
		}
		if ARENA_BASE.load(Ordering::Relaxed) != 0 {
			Write::error(
				"claim_map_bytes: a device arena is already active",
			);
			return None;
		}
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(Cell::get);
		let remaining = vram_free_base();
		let cap = remaining.saturating_sub(USER_GB);
		if n_bytes > cap {
			return None;
		}
		let (ptr, handles) = vmm_map_span(n_bytes)?;
		if let Ok(mut g) = ARENA_VMM.lock() {
			*g = Some((ptr.addr(), handles.iter().map(|h| return h.addr()).collect()));
		}
		tag_add(tag, n_bytes);
		note_range(ptr.addr(), n_bytes, tag);
		return Some(Self {
			ptr,
			len: n_bytes,
			owned: Ownership::Vmm,
			tag,
			dt: Dtype::F64,
		});
	}

	/// # Errors
	/// Returns an error if the host-to-device transfer fails.
	#[inline]
	pub fn write_u8(&self, data: &[u8]) -> Result<(), HipError> {
		if data.len() > self.len {
			Write::error(format!(
				"write_u8: {} bytes into a {}-byte buffer",
				data.len(),
				self.len
			));
			return Err(HipError(1));
		}
		// SAFETY: the bounds check above guarantees data fits within self.len; self.ptr is a live device allocation.
		unsafe {
			return xfer(
				self.ptr,
				data.as_ptr().cast::<c_void>(),
				data.len(),
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			);
		}
	}

	/// # Errors
	/// Returns an error if the host-to-device transfer fails.
	#[inline]
	pub fn load(&self, data: &[f64]) -> Result<(), HipError> {
		if self.dt == Dtype::F32 {
			let narrowed: Vec<f32> = data.iter().map(|&x| x as f32).collect();
			let bytes = narrowed.len() * Dtype::F32.elem_size();
			if bytes > self.len {
				Write::error(format!(
					"load: {bytes} bytes into a {}-byte buffer",
					self.len
				));
				return Err(HipError(1));
			}
			// SAFETY: the bounds check above guarantees the narrowed bytes fit within self.len; self.ptr is a live device allocation.
			unsafe {
				return xfer(
					self.ptr,
					narrowed.as_ptr().cast::<c_void>(),
					bytes,
					HIP_MEMCPY_H2D,
					ptr::null_mut(),
				);
			}
		}
		let bytes = mem::size_of_val(data);
		if bytes > self.len {
			Write::error(format!(
				"load: {bytes} bytes into a {}-byte buffer",
				self.len
			));
			return Err(HipError(1));
		}
		// SAFETY: the bounds check above guarantees data fits within self.len; self.ptr is a live device allocation.
		unsafe {
			return xfer(
				self.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			);
		}
	}

	/// Downloads into an f64 host slice, widening from the buffer's storage
	/// dtype: an F32 buffer is read back and promoted, an F64 buffer copied
	/// directly. The inference forward parks activations at the model's native
	/// width but composes its host-side routing and readout in f64.
	///
	/// # Errors
	/// Errors if the device-to-host copy or the following synchronize fails.
	#[inline]
	pub fn download_host(&self, dst: &mut [f64]) -> Result<(), HipError> {
		if self.dt == Dtype::F32 {
			let mut tmp = vec![0f32; dst.len()];
			self.download_f32(&mut tmp)?;
			for (d, s) in dst.iter_mut().zip(tmp.iter()) {
				*d = f64::from(*s);
			}
			return Ok(());
		}
		let bytes = dst.len() * size_of::<f64>();
		// SAFETY: dst holds bytes of valid host memory and self.ptr is a live device buffer of at least that size.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	/// # Errors
	/// Returns an error if allocation or the host-to-device transfer fails.
	#[inline]
	pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?.as_dtype(Dtype::F32);
		// SAFETY: buf holds bytes just allocated and data points to bytes of valid host memory.
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			)
		}?;
		return Ok(buf);
	}

	/// # Errors
	/// Errors if the device allocation or the host-to-device upload fails.
	#[inline]
	pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		// SAFETY: buf holds bytes just allocated and data points to bytes of valid host memory.
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			)
		}?;
		return Ok(buf);
	}

	/// # Errors
	/// Errors if the device allocation or the zeroing memset fails.
	#[inline]
	pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(n * 4)?.as_dtype(Dtype::F32);
		buf.memset_zero(n * 4)?;
		return Ok(buf);
	}

	/// # Errors
	/// Errors if the memset or the device synchronize fails.
	#[inline]
	pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
		// SAFETY: self.ptr is a valid device buffer of at least n_bytes.
		unsafe { memset_dev(self.ptr, 0, n_bytes, ptr::null_mut()) }?;
		return device_synchronize();
	}

	/// # Errors
	/// Errors if the device-to-host copy or the synchronize fails.
	#[inline]
	pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		// SAFETY: dst holds bytes of valid host memory and self.ptr is a valid device buffer.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	/// # Errors
	/// Errors if the device-to-host copy or the synchronize fails.
	#[inline]
	pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
		// SAFETY: dst holds dst.len() bytes of valid host memory and self.ptr is a valid device buffer.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				dst.len(),
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	/// # Errors
	/// Errors if the device-to-host copy or the synchronize fails.
	#[inline]
	pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		// SAFETY: dst holds bytes of valid host memory and self.ptr is a valid device buffer.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	#[inline]
	#[must_use]
	pub const fn len(&self) -> usize {
		return self.len;
	}
	#[inline]
	#[must_use]
	pub const fn n_floats(&self) -> usize {
		return self.len.wrapping_div(self.dt.elem_size());
	}
	#[inline]
	#[must_use]
	pub fn ptr_addr(&self) -> usize {
		return self.ptr.addr();
	}
	#[inline]
	#[must_use]
	pub const fn ptr_raw(&self) -> *mut c_void {
		return self.ptr;
	}

	#[inline]
	#[must_use]
	pub const fn is_empty(&self) -> bool {
		return self.len == 0;
	}

	/// Borrowed sub-window `[byte_off, byte_off+byte_len)` of this buffer.
	/// The returned buffer does not own the memory; it stays valid only while
	/// `self` does. Panics if the window runs past the end of the buffer.
	#[inline]
	#[must_use]
	pub fn sub_view(&self, byte_off: usize, byte_len: usize) -> Self {
		assert!(
			byte_off.saturating_add(byte_len) <= self.len,
			"sub_view [{byte_off}, {byte_off}+{byte_len}) exceeds buffer len {}",
			self.len
		);
		// SAFETY: the assert above proves byte_off is within the allocation, so
		// the offset pointer stays inside self's device buffer.
		let p = unsafe { self.ptr.cast::<u8>().add(byte_off) }.cast::<c_void>();
		return Self::borrow(p, byte_len);
	}

	#[inline]
	#[must_use]
	pub fn as_ptr_offset(&self, n_floats: usize) -> *mut c_void {
		let es = self.dt.elem_size();
		let want = n_floats * es;
		let off = if want > self.len {
			Write::error(format!(
				"as_ptr_offset: offset {want} bytes exceeds buffer len {}",
				self.len
			));
			self.len
		} else {
			want
		};
		// SAFETY: off is clamped to self.len above, so the offset stays within the allocation.
		unsafe { return self.ptr.cast::<u8>().add(off).cast::<c_void>() }
	}

	#[inline]
	#[must_use]
	pub fn view(&self, offset_floats: usize, len_floats: usize) -> Self {
		return Self::borrow(
			self.as_ptr_offset(offset_floats),
			len_floats * self.dt.elem_size(),
		)
		.as_dtype(self.dt);
	}

	/// # Errors
	/// Returns `HipError` if the device-to-device copy or the following device sync fails.
	#[inline]
	pub fn copy_from(&mut self, src: &Self, n_bytes: usize) -> Result<(), HipError> {
		// SAFETY: self.ptr and src.ptr are valid device pointers spanning n_bytes.
		unsafe {
			xfer(
				self.ptr,
				src.ptr.cast_const(),
				n_bytes,
				HIP_MEMCPY_D2D,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	/// # Errors
	/// Returns [`HipError`] if the device memset fails.
	#[inline]
	pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
		// SAFETY: self.ptr is a valid device pointer spanning n_bytes.
		unsafe { memset_dev(self.ptr, i32::from(value), n_bytes, ptr::null_mut()) }?;
		return device_synchronize();
	}

	/// # Errors
	/// Returns [`HipError`] if the allocation or upload fails.
	#[inline]
	pub unsafe fn upload_async(data: &[f64], stream: *mut c_void) -> Result<Self, HipError> {
		let bytes = mem::size_of_val(data);
		let buf = Self::alloc(data.len())?;
		// SAFETY: buf.ptr is a fresh device buffer of `bytes`; data is valid for `bytes`.
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				stream,
			)
		}?;
		return Ok(buf);
	}

	/// # Errors
	/// Returns [`HipError`] if the download fails.
	#[inline]
	pub unsafe fn download_async(
		&self,
		dst: &mut [f64],
		stream: *mut c_void,
	) -> Result<(), HipError> {
		let bytes = mem::size_of_val(dst);
		// SAFETY: self.ptr is a valid device pointer for `bytes`; dst is valid for `bytes`.
		unsafe {
			return xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				stream,
			);
		}
	}

	/// Download `self` into `dst` on the default stream, then the caller
	/// synchronizes. Safe wrapper over [`Self::download_async`]: the only
	/// invariant the async form needs from a caller is a valid stream, and the
	/// default stream is always valid, so this form carries none.
	///
	/// # Errors
	/// Returns [`HipError`] if the download fails.
	#[inline]
	pub fn download(&self, dst: &mut [f64]) -> Result<(), HipError> {
		// SAFETY: the null (default) stream is always valid; bounds come from dst.
		return unsafe { self.download_async(dst, ptr::null_mut()) };
	}

	/// # Errors
	/// Returns [`HipError`] if the allocation or upload fails.
	#[inline]
	pub fn upload_f16(data: &[half::f16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		// SAFETY: buf.ptr is a fresh device buffer of `bytes`; data is valid for `bytes`.
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			)
		}?;
		return Ok(buf);
	}

	/// # Errors
	/// Returns [`HipError`] if the download fails.
	#[inline]
	pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		// SAFETY: self.ptr is a valid device pointer for `bytes`; dst is valid for `bytes`.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}

	/// # Errors
	/// Returns [`HipError`] if the allocation or upload fails.
	#[inline]
	pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		// SAFETY: buf is freshly allocated for bytes; data is a valid host slice of the same length.
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr().cast::<c_void>(),
				bytes,
				HIP_MEMCPY_H2D,
				ptr::null_mut(),
			)
		}?;
		return Ok(buf);
	}

	/// # Errors
	/// Errors if the device-to-host transfer or the synchronize fails.
	#[inline]
	pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		// SAFETY: self.ptr is a live device buffer; dst is a valid host slice of bytes length.
		unsafe {
			xfer(
				dst.as_mut_ptr().cast::<c_void>(),
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				ptr::null_mut(),
			)
		}?;
		return device_synchronize();
	}
}

impl Drop for GpuBuffer {
	#[inline]
	fn drop(&mut self) {
		if matches!(self.owned, Ownership::Borrow) {
			return;
		}
		let Some(_nn) = ptr::NonNull::new(self.ptr) else {
			return;
		};
		let cmp::Ordering::Equal = SHUTTING_DOWN.load(Ordering::Relaxed).cmp(&0) else {
			return;
		};
		tag_sub(self.tag, self.len);
		FREE_TOTAL.fetch_add(1, Ordering::Relaxed);
		match mem::replace(&mut self.owned, Ownership::Borrow) {
			Ownership::Vmm => {
				let taken = ARENA_VMM.lock().ok().and_then(|mut g| return g.take());
				if let Some((_va, addrs)) = taken {
					let ptrs: Vec<*mut c_void> = addrs
						.iter()
						.map(|&a| return ptr::with_exposed_provenance_mut::<c_void>(a))
						.collect();
					vmm_unmap_span(self.ptr, &ptrs);
				}
			}
			_pool => {
				callspy::tick(&callspy::FREE_ASYNC);
				// SAFETY: self.ptr is a live pool allocation freed exactly once here and nulled below.
				let code = unsafe { hipFreeAsync(self.ptr, ptr::null_mut()) };
				if let Some(_fail) = num::NonZeroI32::new(code) {
					Write::error(format!(
						"hipFreeAsync FAILED (code {code}): leaked {} tag '{}' at {:#x}",
						self.len,
						self.tag,
						self.ptr.addr()
					));
				}
			}
		}
		self.ptr = ptr::null_mut();
	}
}

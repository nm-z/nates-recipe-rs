use crate::log::{Write, gpu};
use crate::hip::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTTING_DOWN: AtomicU8 = AtomicU8::new(0);

static FREE_TOTAL: AtomicUsize = AtomicUsize::new(0);

static H2D_BYTES: AtomicUsize = AtomicUsize::new(0);
static D2H_BYTES: AtomicUsize = AtomicUsize::new(0);
static D2D_BYTES: AtomicUsize = AtomicUsize::new(0);
static H2D_CALLS: AtomicUsize = AtomicUsize::new(0);
static D2H_CALLS: AtomicUsize = AtomicUsize::new(0);
static D2D_CALLS: AtomicUsize = AtomicUsize::new(0);

static TAG_BYTES: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
static TAG_PEAK: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());

thread_local! {
	static CURRENT_TAG: Cell<&'static str> = const { Cell::new("other") };
}

pub struct TagScope {
	prev: &'static str,
}

pub fn tag_scope(name: &'static str) -> TagScope {
	let prev = CURRENT_TAG.with(|t| t.replace(name));
	TagScope { prev }
}

impl Drop for TagScope {
	fn drop(&mut self) {
		CURRENT_TAG.with(|t| t.set(self.prev));
	}
}

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

fn tag_sub(tag: &'static str, n: usize) {
	let Ok(mut m) = TAG_BYTES.lock() else {
		return;
	};
	let e = m.entry(tag).or_insert(0);
	*e = e.saturating_sub(n);
}

pub(crate) fn tag_note_alloc(tag: &'static str, n: usize) {
	tag_add(tag, n);
}

pub(crate) fn tag_note_free(tag: &'static str, n: usize) {
	tag_sub(tag, n);
}

fn fmt_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	let gb = K * K * K;
	let mb = K * K;
	match f.partial_cmp(&gb) {
		Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal) => {
			format!("{:.2} GB", f / gb)
		}
		Some(std::cmp::Ordering::Less) | None => match f.partial_cmp(&mb) {
			Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal) => {
				format!("{:.2} MB", f / mb)
			}
			Some(std::cmp::Ordering::Less) | None => match f.partial_cmp(&K) {
				Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal) => {
					format!("{:.2} KB", f / K)
				}
				Some(std::cmp::Ordering::Less) | None => format!("{b} B"),
			},
		},
	}
}

fn oom_pair(name: &str, val: &str) -> String {
	format!("{name}: {val}")
}

struct TagBytes {
	tag: &'static str,
	bytes: usize,
}

fn oom_report(req: usize) {
	let mi = crate::hip::mem_info().unwrap_or(crate::hip::MemInfo { free: 0, total: 0 });
	let free = mi.free;
	let total = mi.total;
	let mut autopsy: Vec<TagBytes> = match TAG_BYTES.lock() {
		Ok(m) => m
			.keys()
			.filter_map(|k| {
				let bytes = m.get(k).copied().unwrap_or(0);
				Some(TagBytes { tag: k, bytes }).filter(|tb| tb.bytes > 0)
			})
			.collect(),
		Err(_p) => Vec::new(),
	};
	autopsy.sort_by(|a, b| b.bytes.cmp(&a.bytes));
	let mut line: Vec<String> = autopsy
		.iter()
		.map(|tb| oom_pair(tb.tag, &fmt_bytes(tb.bytes)))
		.collect();
	line.push(oom_pair("req", &fmt_bytes(req)));
	line.push(oom_pair("free", &fmt_bytes(free)));
	line.push(oom_pair("total", &fmt_bytes(total)));
	line.push(oom_pair("over", &fmt_bytes(req.saturating_sub(free))));
	line.push(oom_pair(
		"slack",
		&fmt_bytes(crate::hip::pool_slack(0).unwrap_or(0)),
	));
	line.push(oom_pair("arena", &fmt_bytes(arena_remaining())));
	drop(Write::err(&line.join(", ")));
	drop(Write::err(&format!(
		"{}, {}, {}",
		oom_pair("H2D", &fmt_bytes(H2D_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2H", &fmt_bytes(D2H_BYTES.load(Ordering::Relaxed))),
		oom_pair("D2D", &fmt_bytes(D2D_BYTES.load(Ordering::Relaxed))),
	)));
}

struct FdinfoMem {
	vram_kib: u64,
	gtt_kib: u64,
}

fn kernel_fdinfo() -> Option<FdinfoMem> {
	let entries = std::fs::read_dir("/proc/self/fdinfo").ok()?;
	let mut by_client: BTreeMap<String, FdinfoMem> = BTreeMap::new();
	for e in entries.flatten() {
		let Ok(info) = std::fs::read_to_string(e.path()) else {
			continue;
		};
		let mut client_id: Option<String> = None;
		let mut vram_kib = 0u64;
		let mut gtt_kib = 0u64;
		for line in info.lines() {
			match line.strip_prefix("drm-client-id:") {
				Some(v) => client_id = Some(v.trim().to_string()),
				None => match line.strip_prefix("drm-memory-vram:") {
					Some(v) => {
						vram_kib = v
							.trim()
							.trim_end_matches("KiB")
							.trim()
							.parse()
							.unwrap_or(0)
					}
					None => match line.strip_prefix("drm-memory-gtt:") {
						Some(v) => {
							gtt_kib = v
								.trim()
								.trim_end_matches("KiB")
								.trim()
								.parse()
								.unwrap_or(0)
						}
						None => continue,
					},
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
	let vram_total: u64 = by_client.values().map(|cm| cm.vram_kib).sum();
	let gtt_total: u64 = by_client.values().map(|cm| cm.gtt_kib).sum();
	Some(FdinfoMem {
		vram_kib: vram_total,
		gtt_kib: gtt_total,
	})
}

struct KindLabel {
	kind: u32,
	label: &'static str,
}

struct VramspyRow {
	label: &'static str,
	live: u64,
	peak: u64,
	al: u64,
	fr: u64,
}

pub fn ledger_report() -> String {
	let mut live: Vec<TagBytes> = TAG_BYTES
		.lock()
		.map(|m| {
			m.keys()
				.map(|k| TagBytes {
					tag: k,
					bytes: m.get(k).copied().unwrap_or(0),
				})
				.collect()
		})
		.unwrap_or_default();
	live.sort_by(|a, b| b.bytes.cmp(&a.bytes));
	let peak = TAG_PEAK.lock().map(|m| m.clone()).unwrap_or_default();
	let mut s = String::from("──────── GPU MEMORY LEDGER ────────\n");
	let mut total_live = 0usize;
	for tb in &live {
		total_live += tb.bytes;
		let pk = peak.get(tb.tag).copied().unwrap_or(0);
		s += &format!(
			"  {:<14} live {:>11}  peak {:>11}\n",
			tb.tag,
			fmt_bytes(tb.bytes),
			fmt_bytes(pk)
		);
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
	s += &format!(
		"  device     allocs {a}  frees {f}  live-buffers {}\n",
		a.saturating_sub(f)
	);

	let live_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_live".as_ptr()) };
	match std::ptr::NonNull::new(live_sym) {
		None => {
			s += "  global: vramspy NOT loaded (LD_PRELOAD libvramspy.so)\n";
		}
		Some(_present) => {
			let sym = |name: &std::ffi::CStr| -> *mut c_void {
				unsafe { libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr()) }
			};
			let to_u32_u64 = |p: *mut c_void| -> extern "C" fn(u32) -> u64 {
				unsafe { std::mem::transmute::<*mut c_void, extern "C" fn(u32) -> u64>(p) }
			};
			let live_fn = to_u32_u64(live_sym);
			let peak_fn = to_u32_u64(sym(c"vramspy_peak"));
			let allocs_fn = to_u32_u64(sym(c"vramspy_allocs"));
			let frees_fn = to_u32_u64(sym(c"vramspy_frees"));
			let unknown_frees_fn = unsafe {
				std::mem::transmute::<*mut c_void, extern "C" fn() -> u64>(sym(
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
				.map(|kl| VramspyRow {
					label: kl.label,
					live: live_fn(kl.kind),
					peak: peak_fn(kl.kind),
					al: allocs_fn(kl.kind),
					fr: frees_fn(kl.kind),
				})
				.collect();
			let device_live = rows.first().map(|r| r.live).unwrap_or(0);
			for r in &rows {
				s += &format!(
					"    {:<10} live {:>11}  peak {:>11}  (allocs {} frees {})\n",
					r.label,
					fmt_bytes(r.live as usize),
					fmt_bytes(r.peak as usize),
					r.al,
					r.fr
				);
			}
			let delta = (device_live as usize).saturating_sub(total_live);
			s += &format!(
				"    library delta: {} (unknown frees: {})\n",
				fmt_bytes(delta),
				unknown_frees_fn()
			);
		}
	}

	match kernel_fdinfo() {
		Some(m) => {
			s += &format!(
				"  kernel (fdinfo)  vram {}  gtt {}\n",
				fmt_bytes((m.vram_kib * 1024) as usize),
				fmt_bytes((m.gtt_kib * 1024) as usize)
			);
		}
		None => {
			s += "  kernel (fdinfo): unreadable\n";
		}
	}
	s += "───────────────────────────────────";
	s
}

enum Lifecycle {
	Running,
	Down,
}

fn lifecycle() -> Lifecycle {
	match SHUTTING_DOWN.load(Ordering::Relaxed).cmp(&0) {
		std::cmp::Ordering::Equal => Lifecycle::Running,
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => Lifecycle::Down,
	}
}

pub fn mark_shutting_down() {
	SHUTTING_DOWN.store(1, Ordering::SeqCst);
}

#[derive(Clone, Copy)]
enum Frozen {
	No,
	Yes,
}

thread_local! {
	static ALLOC_FROZEN: Cell<Frozen> = const { Cell::new(Frozen::No) };
}

pub fn alloc_count_reset() -> usize {
	ALLOC_COUNT.swap(0, Ordering::Relaxed)
}

pub fn device_alloc_count() -> usize {
	crate::callspy::MALLOC_ASYNC.load(Ordering::Relaxed) as usize
}

pub fn device_free_count() -> usize {
	FREE_TOTAL.load(Ordering::Relaxed)
}

pub struct XferBytes {
	pub h2d: usize,
	pub d2h: usize,
	pub d2d: usize,
}

pub fn xfer_bytes() -> XferBytes {
	XferBytes {
		h2d: H2D_BYTES.load(Ordering::Relaxed),
		d2h: D2H_BYTES.load(Ordering::Relaxed),
		d2d: D2D_BYTES.load(Ordering::Relaxed),
	}
}

pub fn xfer_calls() -> XferBytes {
	XferBytes {
		h2d: H2D_CALLS.load(Ordering::Relaxed),
		d2h: D2H_CALLS.load(Ordering::Relaxed),
		d2d: D2D_CALLS.load(Ordering::Relaxed),
	}
}

pub fn alloc_freeze() {
	ALLOC_FROZEN.with(|f| f.set(Frozen::Yes));
}

pub fn alloc_unfreeze() {
	ALLOC_FROZEN.with(|f| f.set(Frozen::No));
}

pub struct AllocGuard {
	_marker: std::marker::PhantomData<*const ()>,
}

impl AllocGuard {
	pub fn freeze() -> Self {
		alloc_freeze();
		AllocGuard {
			_marker: std::marker::PhantomData,
		}
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

static ARENA_CARVED: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
static ARENA_CARVED_ALIGNED: AtomicUsize = AtomicUsize::new(0);

fn arena_note_carve(tag: &'static str, n_bytes: usize, aligned: usize) {
	let Ok(mut m) = ARENA_CARVED.lock() else {
		ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
		return;
	};
	*m.entry(tag).or_insert(0) += n_bytes;
	drop(m);
	ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
}

fn carve_image_front(image: &[f64]) {
	let Some(_first) = image.first() else {
		return;
	};
	let _t = tag_scope("weights");
	let _img = GpuBuffer::alloc(image.len()).unwrap_or_else(|e| {
		drop(Write::err(&format!("arena image carve: {e}")));
		std::process::abort()
	});
}

fn commit_with_image(base: *mut c_void, size: usize, image: &[f64]) -> Result<(), HipError> {
	unsafe { memset_dev(base, 0, size, std::ptr::null_mut())? };
	for _first in image.first().into_iter() {
		let bytes = std::mem::size_of_val(image);
		H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
		H2D_CALLS.fetch_add(1, Ordering::Relaxed);
		let pin = run_pin(bytes);
		par_copy(pin, image.as_ptr() as *const u8, bytes);
		unsafe {
			dev_copy(
				base,
				pin as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)?
		};
	}
	crate::hip::device_synchronize()
}

pub fn claim_device_arena() -> Option<GpuBuffer> {
	claim_device_arena_with_image(&[])
}

pub fn claim_device_arena_bytes(want: usize) -> Option<GpuBuffer> {
	claim_device_arena_bytes_with_image(want, &[])
}

pub fn claim_device_arena_with_image(image: &[f64]) -> Option<GpuBuffer> {
	let grow = vram_free_base().saturating_sub(USER_GB);
	claim_device_arena_bytes_with_image(grow & !((1 << 21) - 1), image)
}

pub fn claim_device_arena_bytes_with_image(mut want: usize, image: &[f64]) -> Option<GpuBuffer> {
	if ARENA_BASE.load(Ordering::Relaxed) != 0 {
		drop(Write::err("claim_device_arena: a device arena is already active"));
		std::process::abort();
	}
	let _t = tag_scope("unclaimed");
	while want > (1 << 20) {
		match GpuBuffer::claim_map_bytes(want) {
			Some(slab) => {
				set_device_arena(slab.ptr_raw(), want);
				carve_image_front(image);
				commit_with_image(slab.ptr_raw(), want, image).unwrap_or_else(|e| {
					drop(Write::err(&format!("claim commit: {e}")));
					std::process::abort()
				});
				return Some(slab);
			}
			None => want -= want / 16,
		}
	}
	None
}

pub fn release_device_arena(slab: GpuBuffer) {
	if ARENA_BASE.load(Ordering::Relaxed) != slab.ptr_addr() {
		drop(Write::err("release_device_arena: slab is not the active claim"));
		std::process::abort();
	}
	crate::hip::device_synchronize().unwrap_or_else(|e| {
		drop(Write::err(&format!("arena release sync: {e}")));
		std::process::abort()
	});
	set_device_arena(std::ptr::null_mut(), 0);
	drain_arena_carve();
	drop(slab);
}

fn drain_arena_carve() {
	drain_arena_carve_map();
	tag_add("unclaimed", ARENA_CARVED_ALIGNED.swap(0, Ordering::Relaxed));
}

fn drain_arena_carve_map() {
	let Ok(mut m) = ARENA_CARVED.lock() else {
		return;
	};
	for tag in m.keys() {
		let bytes = m.get(tag).copied().unwrap_or(0);
		tag_sub(tag, bytes);
	}
	m.clear();
}

static PARKED: Mutex<Option<GpuBuffer>> = Mutex::new(None);

static PARK_GEN: AtomicUsize = AtomicUsize::new(0);
static PARKED_GEN: AtomicUsize = AtomicUsize::new(0);

pub fn park_run_backing(buf: GpuBuffer) {
	let mut g = match PARKED.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	if !g.is_none() {
		drop(Write::err("park_run_backing: a parked run backing already exists"));
		std::process::abort();
	}
	*g = Some(buf);
	PARKED_GEN.store(
		PARK_GEN.fetch_add(1, Ordering::Relaxed) + 1,
		Ordering::Relaxed,
	);
}

pub fn live_parked_gen() -> Option<usize> {
	std::num::NonZeroUsize::new(PARKED_GEN.load(Ordering::Relaxed))
		.map(std::num::NonZeroUsize::get)
}

enum ParkKind {
	Registered,
	Foreign,
}

enum Disposition {
	Adopt,
	ReleaseArena,
	DropForeign,
}

fn adopt_run_backing_inner(need: usize) -> Option<GpuBuffer> {
	let parked = match PARKED.lock() {
		Ok(mut g) => g.take(),
		Err(p) => p.into_inner().take(),
	}?;
	PARKED_GEN.store(0, Ordering::Relaxed);
	let kind = match ARENA_BASE.load(Ordering::Relaxed).cmp(&parked.ptr_addr()) {
		std::cmp::Ordering::Equal => ParkKind::Registered,
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => ParkKind::Foreign,
	};
	let decision = match kind {
		ParkKind::Foreign => Disposition::DropForeign,
		ParkKind::Registered => match parked.len().cmp(&need) {
			std::cmp::Ordering::Less => Disposition::ReleaseArena,
			std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => Disposition::Adopt,
		},
	};
	match decision {
		Disposition::DropForeign => {
			crate::hip::device_synchronize().unwrap_or_else(|e| {
				drop(Write::err(&format!("parked release sync: {e}")));
				std::process::abort()
			});
			drop(parked);
			pool_trim();
			None
		}
		Disposition::ReleaseArena => {
			release_device_arena(parked);
			None
		}
		Disposition::Adopt => {
			drain_arena_carve();
			ARENA_OFFSET.store(0, Ordering::Relaxed);
			Some(parked)
		}
	}
}

pub fn adopt_run_backing(need: usize) -> Option<GpuBuffer> {
	adopt_run_backing_with_image(need, &[])
}

pub fn adopt_run_backing_with_image(need: usize, image: &[f64]) -> Option<GpuBuffer> {
	let slab = adopt_run_backing_inner(need)?;
	carve_image_front(image);
	commit_with_image(slab.ptr_raw(), slab.len(), image).unwrap_or_else(|e| {
		drop(Write::err(&format!("adopt commit: {e}")));
		std::process::abort()
	});
	Some(slab)
}

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
		std::cmp::Ordering::Equal => release_device_arena(b),
		std::cmp::Ordering::Less | std::cmp::Ordering::Greater => {
			crate::hip::device_synchronize().unwrap_or_else(|e| {
				drop(Write::err(&format!("parked release sync: {e}")));
				std::process::abort()
			});
			drop(b);
		}
	}
}

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
		let add = (STAGE_ALIGN_F64 - rem) % STAGE_ALIGN_F64;
		self.host.resize(self.host.len() + add, 0.0);
	}

	pub fn push(&mut self, data: &[f64]) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.extend_from_slice(data);
		off
	}

	pub fn reserve(&mut self, n_floats: usize) -> usize {
		self.pad();
		let off = self.host.len();
		self.host.resize(off + n_floats, 0.0);
		off
	}

	pub fn len_floats(&self) -> usize {
		self.host.len()
	}

	pub fn into_host(self) -> Vec<f64> {
		self.host
	}

	pub fn is_empty(&self) -> bool {
		self.host.is_empty()
	}
}

enum Dir {
	H2D,
	D2H,
	D2D,
}

pub unsafe fn xfer(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	let dir = Some(kind)
		.filter(|k| *k == HIP_MEMCPY_H2D)
		.map(|_k| Dir::H2D)
		.or_else(|| {
			Some(kind)
				.filter(|k| *k == HIP_MEMCPY_D2H)
				.map(|_k| Dir::D2H)
		})
		.unwrap_or(Dir::D2D);
	match dir {
		Dir::H2D => {
			H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
			H2D_CALLS.fetch_add(1, Ordering::Relaxed);
			unsafe { h2d_pinned(dst, src, bytes, stream) }
		}
		Dir::D2H => {
			D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
			D2H_CALLS.fetch_add(1, Ordering::Relaxed);
			crate::callspy::tick(&crate::callspy::XFER_ASYNC);
			unsafe { dev_copy(dst, src, bytes, kind, stream) }
		}
		Dir::D2D => {
			D2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
			D2D_CALLS.fetch_add(1, Ordering::Relaxed);
			crate::callspy::tick(&crate::callspy::XFER_ASYNC);
			unsafe { dev_copy(dst, src, bytes, kind, stream) }
		}
	}
}

unsafe fn dev_copy(
	dst: *mut c_void,
	src: *const c_void,
	bytes: usize,
	kind: i32,
	stream: *mut c_void,
) -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::MEMCPY_ASYNC);
	check(unsafe { hipMemcpyAsync(dst, src, bytes, kind, stream) })
}

pub fn par_touch(v: &mut [u8]) {
	let threads = std::thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
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
	let threads = std::thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	let per = bytes.div_ceil(threads);
	let d = dst as usize;
	let s0 = src as usize;
	std::thread::scope(|sc| {
		for t in 0..threads {
			let off = t * per;
			let remaining = match off.cmp(&bytes) {
				std::cmp::Ordering::Less => bytes - off,
				std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => break,
			};
			let len = per.min(remaining);
			sc.spawn(move || unsafe {
				std::ptr::copy_nonoverlapping(
					(s0 as *const u8).add(off),
					(d as *mut u8).add(off),
					len,
				);
			});
		}
	});
}

const BOUNCE_BYTES: usize = 64 << 20;
static BOUNCE: Mutex<usize> = Mutex::new(0);

struct PinBuf {
	ptr: usize,
	cap: usize,
}

static RUN_PIN: Mutex<PinBuf> = Mutex::new(PinBuf { ptr: 0, cap: 0 });

fn pin_ensure(g: &mut PinBuf, bytes: usize) -> *mut u8 {
	grow_pin(g, bytes);
	g.ptr as *mut u8
}

fn grow_pin(g: &mut PinBuf, bytes: usize) {
	let std::cmp::Ordering::Less = g.cap.cmp(&bytes) else {
		return;
	};
	for _old in std::num::NonZeroUsize::new(g.ptr).into_iter() {
		unsafe { crate::hip::host_free(g.ptr as *mut c_void) }.ok();
	}
	g.ptr = crate::hip::host_malloc(bytes, 0).unwrap_or_else(|e| {
		drop(Write::err(&format!("run_pin host_malloc: {e}")));
		std::process::abort()
	}) as usize;
	g.cap = bytes;
}

fn run_pin(bytes: usize) -> *mut u8 {
	let mut g = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	pin_ensure(&mut g, bytes)
}

pub(crate) struct BounceRange {
	pub base: usize,
	pub len: usize,
}

pub(crate) fn bounce_range() -> Option<BounceRange> {
	let base = *BOUNCE.lock().ok()?;
	std::num::NonZeroUsize::new(base).map(|nz| BounceRange {
		base: nz.get(),
		len: BOUNCE_BYTES,
	})
}

struct Range {
	base: usize,
	len: usize,
	what: &'static str,
}

static RECENT_RANGES: Mutex<Vec<Range>> = Mutex::new(Vec::new());

pub(crate) fn note_range(base: usize, len: usize, what: &'static str) {
	let Ok(mut r) = RECENT_RANGES.lock() else {
		return;
	};
	for _full in Some(()).filter(|_u| r.len() >= 256).into_iter() {
		r.remove(0);
	}
	r.push(Range { base, len, what });
}

pub(crate) fn locate_va(va: usize) -> Option<String> {
	let r = RECENT_RANGES.lock().ok()?;
	r.iter()
		.rev()
		.find(|range| va >= range.base && va < range.base + range.len)
		.map(|range| {
			format!(
				"{} [base 0x{:x} len {}] +0x{:x}",
				range.what,
				range.base,
				fmt_bytes(range.len),
				va - range.base
			)
		})
}

pub(crate) fn free_bounce() {
	let mut guard = match BOUNCE.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	for _live in std::num::NonZeroUsize::new(*guard).into_iter() {
		unsafe { crate::hip::host_free(*guard as *mut c_void) }.ok();
		*guard = 0;
	}
}

pub(crate) fn free_run_pin() {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	for _live in std::num::NonZeroUsize::new(guard.ptr).into_iter() {
		crate::hip::device_synchronize().ok();
		unsafe { crate::hip::host_free(guard.ptr as *mut c_void) }.ok();
		guard.ptr = 0;
		guard.cap = 0;
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
	let base = match std::num::NonZeroUsize::new(*guard) {
		Some(nz) => nz.get(),
		None => {
			let fresh = crate::hip::host_malloc(BOUNCE_BYTES, 0)? as usize;
			*guard = fresh;
			fresh
		}
	};
	let pin = base as *mut u8;
	let mut done = 0usize;
	while done < bytes {
		let chunk = BOUNCE_BYTES.min(bytes - done);
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

pub const BOUNCE_LIMIT: usize = BOUNCE_BYTES;

pub struct ExitD2H {
	_guard: std::sync::MutexGuard<'static, PinBuf>,
	pin: usize,
	bytes: usize,
}

impl ExitD2H {
	pub fn finish(self, dst: &mut [f64]) {
		if std::mem::size_of_val(dst) != self.bytes {
			drop(Write::err(&format!(
				"ExitD2H::finish size mismatch: {} vs {}",
				std::mem::size_of_val(dst),
				self.bytes
			)));
			std::process::abort();
		}
		par_copy(
			dst.as_mut_ptr() as *mut u8,
			self.pin as *const u8,
			self.bytes,
		);
	}
}

pub unsafe fn exit_d2h_enqueue(src: *const c_void, bytes: usize) -> Result<ExitD2H, HipError> {
	let mut guard = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	let pin = pin_ensure(&mut guard, bytes);
	D2H_BYTES.fetch_add(bytes, Ordering::Relaxed);
	D2H_CALLS.fetch_add(1, Ordering::Relaxed);
	unsafe {
		dev_copy(
			pin as *mut c_void,
			src,
			bytes,
			HIP_MEMCPY_D2H,
			std::ptr::null_mut(),
		)
	}?;
	Ok(ExitD2H {
		_guard: guard,
		pin: pin as usize,
		bytes,
	})
}

pub(crate) unsafe fn memset_dev(
	dst: *mut c_void,
	value: i32,
	bytes: usize,
	stream: *mut c_void,
) -> Result<(), HipError> {
	crate::callspy::tick(&crate::callspy::MEMSET_ASYNC);
	check(unsafe { hipMemsetAsync(dst, value, bytes, stream) })
}

static DEVICE_INIT: std::sync::Once = std::sync::Once::new();

extern "C" fn release_birth_claim_at_exit() {
	release_run_backing();
}

pub(crate) fn device_init_once() {
	crate::gate::acquire();
	DEVICE_INIT.call_once(|| {
		crate::hip::disable_sdma_once();
		for e in crate::hip::set_pool_retain(0).err().into_iter() {
			drop(Write::err(&format!("GPU pool retain failed: {e}")));
		}
		crate::hip::register_fault_autopsy_once();
		crate::hw::spawn_thrash_watchdog();
	});
}

pub fn pool_trim() {
	crate::hip::device_synchronize().unwrap_or_else(|e| {
		drop(Write::err(&format!("pool_trim sync: {e}")));
		std::process::abort()
	});
	crate::hip::trim_mempool(0).unwrap_or_else(|e| {
		drop(Write::err(&format!("pool_trim: {e}")));
		std::process::abort()
	});
}

pub const USER_GB: usize = 1 << 30;

pub fn vram_free_base() -> usize {
	let hip_free = crate::hip::mem_info().unwrap_or_else(|e| {
		drop(Write::err(&format!("hipMemGetInfo: {e}")));
		std::process::abort()
	}).free;
	let sys_free = crate::hip::sysfs_vram_free().unwrap_or(hip_free);
	let slack = crate::hip::pool_slack(0).unwrap_or_else(|e| {
		drop(Write::err(&format!("pool_slack: {e}")));
		std::process::abort()
	});
	hip_free.min(sys_free).saturating_sub(slack)
}

pub fn claimable_bytes() -> usize {
	arena_remaining() + vram_free_base().saturating_sub(USER_GB)
}

pub fn set_device_arena(base: *mut c_void, size: usize) {
	ARENA_OFFSET.store(0, Ordering::Relaxed);
	ARENA_SIZE.store(size, Ordering::Relaxed);
	ARENA_BASE.store(base as usize, Ordering::Relaxed);
}

pub fn arena_remaining() -> usize {
	match std::num::NonZeroUsize::new(ARENA_BASE.load(Ordering::Relaxed)) {
		None => 0,
		Some(_base) => ARENA_SIZE
			.load(Ordering::Relaxed)
			.saturating_sub(ARENA_OFFSET.load(Ordering::Relaxed)),
	}
}

pub fn device_arena_active() -> bool {
	ARENA_BASE.load(Ordering::Relaxed) != 0
}

pub fn probe_ceiling(mut probe_survives: impl FnMut(usize) -> bool) -> Option<usize> {
	let mut want = vram_free_base().saturating_sub(USER_GB) & !((1 << 21) - 1);
	while want > (1 << 30) {
		if probe_survives(want) {
			Write::line(gpu, &format!(
				"claim probe: {:.2} GB (probe-verified)",
				want as f64 / (1u64 << 30) as f64
			));
			return Some(want);
		}
		Write::line(gpu, &format!(
			"claim probe: {:.2} GB unmappable, backing off",
			want as f64 / (1u64 << 30) as f64
		));
		want -= want / 16;
	}
	None
}

#[derive(Clone, Copy)]
enum Ownership {
	Pool,
	Borrow,
}

pub struct GpuBuffer {
	pub(crate) ptr: *mut c_void,
	len: usize,
	owned: Ownership,
	tag: &'static str,
}

unsafe impl Send for GpuBuffer {}
unsafe impl Sync for GpuBuffer {}

impl GpuBuffer {
	pub fn borrow(ptr: *mut c_void, len: usize) -> Self {
		Self {
			ptr,
			len,
			owned: Ownership::Borrow,
			tag: "borrow",
		}
	}

	pub fn is_pool_owned(&self) -> bool {
		matches!(self.owned, Ownership::Pool)
	}

	pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
		Self::alloc_bytes(n_floats * std::mem::size_of::<f64>())
	}

	pub fn try_alloc_bytes(n_bytes: usize) -> Option<Self> {
		Self::alloc_bytes_inner(n_bytes).ok()
	}

	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		match Self::alloc_bytes_inner(n_bytes) {
			Ok(buf) => Ok(buf),
			Err(e) => {
				oom_report(n_bytes);
				match std::num::NonZeroUsize::new(ARENA_BASE.load(Ordering::Relaxed)) {
					Some(_active) => {
						drop(Write::err(&format!(
							"arena carve miss: {n_bytes} B asked, {} B remain — placement exceeded the claim",
							arena_remaining()
						)));
						std::process::abort();
					}
					None => Err(e),
				}
			}
		}
	}

	fn map_bytes(n_bytes: usize) -> Result<*mut c_void, HipError> {
		let mut ptr: *mut c_void = std::ptr::null_mut();
		crate::callspy::tick(&crate::callspy::MALLOC_ASYNC);
		check(unsafe { hipMallocAsync(&mut ptr, n_bytes, std::ptr::null_mut()) })?;
		Ok(ptr)
	}

	fn alloc_bytes_inner(n_bytes: usize) -> Result<Self, HipError> {
		device_init_once();
		ALLOC_FROZEN.with(|f| {
			if !matches!(f.get(), Frozen::No) {
				drop(Write::err(&format!(
					"GPU allocation inside frozen training loop (requested {n_bytes} bytes)"
				)));
				std::process::abort();
			}
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
					Ok(_prev) => {
						let ptr = unsafe { (base as *mut u8).add(off) as *mut c_void };
						tag_sub("unclaimed", aligned);
						tag_add(tag, n_bytes);
						arena_note_carve(tag, n_bytes, aligned);
						note_range(ptr as usize, n_bytes, tag);
						return Ok(Self {
							ptr,
							len: n_bytes,
							owned: Ownership::Borrow,
							tag,
						});
					}
					Err(cur) => off = cur,
				}
			}
			return Err(HipError(2));
		}
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

	pub(crate) fn claim_map_bytes(n_bytes: usize) -> Option<Self> {
		device_init_once();
		ALLOC_FROZEN.with(|f| {
			if !matches!(f.get(), Frozen::No) {
				drop(Write::err(&format!(
					"GPU claim inside frozen training loop (requested {n_bytes} bytes)"
				)));
				std::process::abort();
			}
		});
		if ARENA_BASE.load(Ordering::Relaxed) != 0 {
			drop(Write::err("claim_map_bytes: a device arena is already active"));
			std::process::abort();
		}
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(|t| t.get());
		let remaining = vram_free_base();
		let cap = remaining.saturating_sub(USER_GB);
		if n_bytes > cap {
			return None;
		}
		let ptr = Self::map_bytes(n_bytes).ok()?;
		tag_add(tag, n_bytes);
		note_range(ptr as usize, n_bytes, tag);
		Some(Self {
			ptr,
			len: n_bytes,
			owned: Ownership::Pool,
			tag,
		})
	}

	pub fn write_u8(&self, data: &[u8]) -> Result<(), HipError> {
		if !(data.len() <= self.len) {
			drop(Write::err(&format!(
				"write_u8: {} bytes into a {}-byte buffer",
				data.len(),
				self.len
			)));
			std::process::abort();
		}
		unsafe {
			xfer(
				self.ptr,
				data.as_ptr() as *const c_void,
				data.len(),
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}
	}

	pub fn load(&self, data: &[f64]) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(data);
		if !(bytes <= self.len) {
			drop(Write::err(&format!(
				"load: {bytes} bytes into a {}-byte buffer",
				self.len
			)));
			std::process::abort();
		}
		unsafe {
			xfer(
				self.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}
	}

	pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}?;
		Ok(buf)
	}

	pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}?;
		Ok(buf)
	}

	pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(n * 4)?;
		buf.memset_zero(n * 4)?;
		Ok(buf)
	}

	pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_dev(self.ptr, 0, n_bytes, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				std::ptr::null_mut(),
			)
		}?;
		crate::hip::device_synchronize()
	}

	pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				dst.len(),
				HIP_MEMCPY_D2H,
				std::ptr::null_mut(),
			)
		}?;
		crate::hip::device_synchronize()
	}

	pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				std::ptr::null_mut(),
			)
		}?;
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
		if !(n_floats * 8 <= self.len) {
			drop(Write::err(&format!(
				"as_ptr_offset: offset {} bytes exceeds buffer len {}",
				n_floats * 8,
				self.len
			)));
			std::process::abort();
		}
		unsafe { (self.ptr as *mut u8).add(n_floats * 8) as *mut c_void }
	}

	pub fn view(&self, offset_floats: usize, len_floats: usize) -> GpuBuffer {
		GpuBuffer::borrow(self.as_ptr_offset(offset_floats), len_floats * 8)
	}

	pub fn copy_from(&mut self, src: &GpuBuffer, n_bytes: usize) -> Result<(), HipError> {
		unsafe {
			xfer(
				self.ptr,
				src.ptr as *const c_void,
				n_bytes,
				HIP_MEMCPY_D2D,
				std::ptr::null_mut(),
			)
		}?;
		crate::hip::device_synchronize()
	}

	pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
		unsafe { memset_dev(self.ptr, value as i32, n_bytes, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()
	}

	pub unsafe fn upload_async(data: &[f64], stream: *mut c_void) -> Result<Self, HipError> {
		let bytes = std::mem::size_of_val(data);
		let buf = Self::alloc(data.len())?;
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				stream,
			)
		}?;
		Ok(buf)
	}

	pub unsafe fn download_async(
		&self,
		dst: &mut [f64],
		stream: *mut c_void,
	) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				stream,
			)
		}
	}

	pub fn upload_f16(data: &[half::f16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}?;
		Ok(buf)
	}

	pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				std::ptr::null_mut(),
			)
		}?;
		crate::hip::device_synchronize()
	}

	pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		unsafe {
			xfer(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				std::ptr::null_mut(),
			)
		}?;
		Ok(buf)
	}

	pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		unsafe {
			xfer(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
				std::ptr::null_mut(),
			)
		}?;
		crate::hip::device_synchronize()
	}
}

impl Drop for GpuBuffer {
	fn drop(&mut self) {
		let Ownership::Pool = self.owned else {
			return;
		};
		let Some(_nn) = std::ptr::NonNull::new(self.ptr) else {
			return;
		};
		let Lifecycle::Running = lifecycle() else {
			return;
		};
		tag_sub(self.tag, self.len);
		FREE_TOTAL.fetch_add(1, Ordering::Relaxed);
		crate::callspy::tick(&crate::callspy::FREE_ASYNC);
		let code = unsafe { hipFreeAsync(self.ptr, std::ptr::null_mut()) };
		for _fail in std::num::NonZeroI32::new(code).into_iter() {
			drop(Write::err(&format!(
				"hipFreeAsync FAILED (code {code}): leaked {} tag '{}' at {:p}",
				self.len, self.tag, self.ptr
			)));
		}
		self.ptr = std::ptr::null_mut();
	}
}

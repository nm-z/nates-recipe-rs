use crate::hip::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

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

	let live_sym = unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_live".as_ptr()) };
	if live_sym.is_null() {
		s += "  global: vramspy NOT loaded (LD_PRELOAD libvramspy.so)\n";
	} else {
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

pub fn device_alloc_count() -> usize {
	crate::callspy::MALLOC_ASYNC.load(Ordering::Relaxed) as usize
}

pub fn device_free_count() -> usize {
	FREE_TOTAL.load(Ordering::Relaxed)
}

pub fn xfer_bytes() -> (usize, usize, usize) {
	(
		H2D_BYTES.load(Ordering::Relaxed),
		D2H_BYTES.load(Ordering::Relaxed),
		D2D_BYTES.load(Ordering::Relaxed),
	)
}

pub struct XferBytes {
	pub h2d: usize,
	pub d2h: usize,
	pub d2d: usize,
}

pub fn xfer_bytes_named() -> XferBytes {
	let (h2d, d2h, d2d) = xfer_bytes();
	XferBytes { h2d, d2h, d2d }
}

pub fn xfer_calls() -> (usize, usize, usize) {
	(
		H2D_CALLS.load(Ordering::Relaxed),
		D2H_CALLS.load(Ordering::Relaxed),
		D2D_CALLS.load(Ordering::Relaxed),
	)
}

pub fn xfer_calls_named() -> XferBytes {
	let (h2d, d2h, d2d) = xfer_calls();
	XferBytes { h2d, d2h, d2d }
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

static ARENA_CARVED: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());
static ARENA_CARVED_ALIGNED: AtomicUsize = AtomicUsize::new(0);

fn arena_note_carve(tag: &'static str, n_bytes: usize, aligned: usize) {
	if let Ok(mut m) = ARENA_CARVED.lock() {
		*m.entry(tag).or_insert(0) += n_bytes;
	}
	ARENA_CARVED_ALIGNED.fetch_add(aligned, Ordering::Relaxed);
}

fn carve_image_front(image: &[f64]) {
	if image.is_empty() {
		return;
	}
	let _t = tag_scope("weights");
	let _img = GpuBuffer::alloc(image.len()).expect("arena image carve");
}

fn commit_with_image(base: *mut c_void, size: usize, image: &[f64]) -> Result<(), HipError> {
	unsafe { memset_dev(base, 0, size, std::ptr::null_mut())? };
	if !image.is_empty() {
		let bytes = std::mem::size_of_val(image);
		H2D_BYTES.fetch_add(bytes, Ordering::Relaxed);
		H2D_CALLS.fetch_add(1, Ordering::Relaxed);
		let pin = run_pin(bytes);
		par_copy(pin, image.as_ptr() as *const u8, bytes);
		unsafe { dev_copy(base, pin as *const c_void, bytes, HIP_MEMCPY_H2D, std::ptr::null_mut())? };
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

static PARKED: Mutex<Option<GpuBuffer>> = Mutex::new(None);

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

pub fn live_parked_gen() -> Option<usize> {
	let g = PARKED_GEN.load(Ordering::Relaxed);
	(g != 0).then_some(g)
}

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

pub fn adopt_run_backing(need: usize) -> Option<GpuBuffer> {
	adopt_run_backing_with_image(need, &[])
}

pub fn adopt_run_backing_with_image(need: usize, image: &[f64]) -> Option<GpuBuffer> {
	let slab = adopt_run_backing_inner(need)?;
	carve_image_front(image);
	commit_with_image(slab.ptr_raw(), slab.len(), image).expect("adopt commit");
	Some(slab)
}

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
		crate::callspy::tick(&crate::callspy::XFER_ASYNC);
	}
	if kind == HIP_MEMCPY_H2D {
		return unsafe { h2d_pinned(dst, src, bytes, stream) };
	}
	unsafe { dev_copy(dst, src, bytes, kind, stream) }
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

static RUN_PIN: Mutex<(usize, usize)> = Mutex::new((0, 0));

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

fn run_pin(bytes: usize) -> *mut u8 {
	let mut g = match RUN_PIN.lock() {
		Ok(g) => g,
		Err(p) => p.into_inner(),
	};
	pin_ensure(&mut g, bytes)
}

pub(crate) fn bounce_range() -> Option<(usize, usize)> {
	let base = *BOUNCE.lock().ok()?;
	(base != 0).then_some((base, BOUNCE_BYTES))
}

static RECENT_RANGES: Mutex<Vec<(usize, usize, &'static str)>> = Mutex::new(Vec::new());

pub(crate) fn note_range(base: usize, len: usize, what: &'static str) {
	if let Ok(mut r) = RECENT_RANGES.lock() {
		if r.len() >= 256 {
			r.remove(0);
		}
		r.push((base, len, what));
	}
}

pub(crate) fn locate_va(va: usize) -> Option<String> {
	let r = RECENT_RANGES.lock().ok()?;
	r.iter()
		.rev()
		.find(|(b, l, _)| va >= *b && va < b + l)
		.map(|(b, l, what)| format!("{what} [base 0x{b:x} len {}] +0x{:x}", fmt_bytes(*l), va - b))
}

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
	_guard: std::sync::MutexGuard<'static, (usize, usize)>,
	pin: usize,
	bytes: usize,
}

impl ExitD2H {
	pub fn finish(self, dst: &mut [f64]) {
		assert_eq!(std::mem::size_of_val(dst), self.bytes, "ExitD2H::finish size mismatch");
		par_copy(dst.as_mut_ptr() as *mut u8, self.pin as *const u8, self.bytes);
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
	unsafe { dev_copy(pin as *mut c_void, src, bytes, HIP_MEMCPY_D2H, std::ptr::null_mut()) }?;
	Ok(ExitD2H { _guard: guard, pin: pin as usize, bytes })
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
		if let Err(e) = crate::hip::set_pool_retain(0) {
			eprintln!("GPU pool retain failed: {e}");
		}
		crate::hip::register_fault_autopsy_once();
		crate::hw::spawn_thrash_watchdog();
	});
}


pub fn pool_trim() {
	crate::hip::device_synchronize().expect("pool_trim sync");
	crate::hip::trim_mempool(0).expect("pool_trim");
}

pub const USER_GB: usize = 1 << 30;

pub fn vram_free_base() -> usize {
	let hip_free = crate::hip::mem_info().expect("hipMemGetInfo").0;
	let sys_free = crate::hip::sysfs_vram_free().unwrap_or(hip_free);
	let slack = crate::hip::pool_slack(0).expect("pool_slack");
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
	if ARENA_BASE.load(Ordering::Relaxed) == 0 {
		return 0;
	}
	ARENA_SIZE.load(Ordering::Relaxed).saturating_sub(ARENA_OFFSET.load(Ordering::Relaxed))
}

pub fn device_arena_active() -> bool {
	ARENA_BASE.load(Ordering::Relaxed) != 0
}

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

	pub fn try_alloc_bytes(n_bytes: usize) -> Option<Self> {
		Self::alloc_bytes_inner(n_bytes).ok()
	}

	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		match Self::alloc_bytes_inner(n_bytes) {
			Ok(buf) => Ok(buf),
			Err(e) => {
				oom_report(n_bytes);
				if device_arena_active() {
					panic!(
						"arena carve miss: {n_bytes} B asked, {} B remain — placement exceeded the claim",
						arena_remaining()
					);
				}
				Err(e)
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
		unsafe { xfer(buf.ptr, data.as_ptr() as *const c_void, bytes, HIP_MEMCPY_H2D, stream) }?;
		Ok(buf)
	}

	pub unsafe fn download_async(
		&self,
		dst: &mut [f64],
		stream: *mut c_void,
	) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
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

pub struct Chan<T> {
	pub tx: std::sync::mpsc::SyncSender<T>,
	pub rx: std::sync::mpsc::Receiver<T>,
}

pub fn sync_chan<T>(depth: usize) -> Chan<T> {
	let (tx, rx) = std::sync::mpsc::sync_channel::<T>(depth);
	Chan { tx, rx }
}

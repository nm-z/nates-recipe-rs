//! Out-of-core training: the waterfall applied to the fit loop. When the
//! full-batch scratch exceeds free VRAM, every big buffer gets a HOME laid out
//! strictly VRAM → RAM → DISK (one spill file), and each op streams
//! sample-aligned windows through a fixed pool of device staging buffers. The
//! math stays full-batch: every sample flows through every op each epoch,
//! weight gradients are tiled reductions over windows (the split-K pattern one
//! level up), and SGD updates once per epoch.

use crate::model::ModelInner;
use crate::train::StepScalars;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use recipe_infer::{
	Activation, LayerKind, LayerParams, Loss, Scratch,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// Cumulative spill-file traffic (bytes), bumped at the three I/O sites so each
// sweep line can attribute its disk delta — the NVMe half of the streamed-tier
// roofline (PCIe H2D/D2H deltas come from the gpu-core ledger).
static DISK_R_BYTES: AtomicUsize = AtomicUsize::new(0);
static DISK_W_BYTES: AtomicUsize = AtomicUsize::new(0);
static NET_R_BYTES: AtomicUsize = AtomicUsize::new(0);
static NET_W_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Fixed pool of host window buffers, preallocated and page-touched at build.
/// Every transient host byte this path moves (read-ahead, sync reads, the
/// write-behind queue, commit staging) comes from here — so host RAM is FLAT
/// at exactly the budget instead of fluctuating below it while disk churns
/// (the reserve law: RAM at top−1GB or it's visible laziness). Exhaustion is
/// a counting bug and panics loud.
type HostPool = Arc<Mutex<Vec<Vec<u8>>>>;

fn pool_take(p: &HostPool) -> Vec<u8> {
	p.lock()
		.expect("host pool lock")
		.pop()
		.expect("host pool exhausted — transient window count exceeded POOL_BUFS")
}

fn pool_give(p: &HostPool, v: Vec<u8>) {
	p.lock().expect("host pool lock").push(v);
}

fn mem_available() -> usize {
	std::fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(0, |kb| kb.saturating_mul(1024))
}

fn disk_free(path: &std::path::Path) -> usize {
	let Ok(c) = std::ffi::CString::new(path.as_os_str().as_encoded_bytes()) else {
		return 0;
	};
	let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
	if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
		return 0;
	}
	(st.f_bavail as usize).saturating_mul(st.f_frsize as usize)
}

fn view(b: &GpuBuffer, byte_off: usize, byte_len: usize) -> GpuBuffer {
	GpuBuffer::borrow(
		unsafe { (b.ptr_raw() as *mut u8).add(byte_off) as *mut c_void },
		byte_len,
	)
}

// Spill-file cache discipline: without this the kernel hoards every written
// window as dirty page cache (tens of GB) and the box swaps. Force the range
// to disk and drop it from cache immediately — the spill is streamed, never
// hot.
fn drop_cache(f: &File, off: u64, len: usize) {
	use std::os::unix::io::AsRawFd;
	unsafe {
		libc::sync_file_range(
			f.as_raw_fd(),
			off as i64,
			len as i64,
			libc::SYNC_FILE_RANGE_WAIT_BEFORE | libc::SYNC_FILE_RANGE_WRITE | libc::SYNC_FILE_RANGE_WAIT_AFTER,
		);
		libc::posix_fadvise(f.as_raw_fd(), off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
	}
}

fn interrupted() -> bool {
	crate::train::INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
}

/// VRAM ceiling proven by disposable probe children (the `recipe` binary's
/// VRAM_PROBE branch): both driver counters over-report what VmHeap can
/// physically map and an in-process overshoot is an uncatchable
/// `VmHeap::MapPhysMemory` abort (reproduced 10/10 by SETUP_RACE fill-to-
/// refusal), so the only trustworthy size is one a child SURVIVED claiming
/// alongside this process's current residency.
fn probe_verified_vram() -> usize {
	let exe = std::env::current_exe()
		.ok()
		.and_then(|e| {
			let dir = e.parent()?;
			[dir.join("recipe"), dir.parent()?.join("recipe")]
				.into_iter()
				.find(|p| p.exists())
		})
		.unwrap_or_else(|| panic!("ooc: no `recipe` probe binary beside {:?}", std::env::current_exe()));
	gpu_core::memory::probe_ceiling(|bytes| {
		use std::os::unix::process::CommandExt;
		let mut c = std::process::Command::new(&exe);
		c.env("VRAM_PROBE", bytes.to_string());
		// Cores off — the corpse is the signal, not a bug report.
		unsafe {
			c.pre_exec(|| {
				let z = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
				libc::setrlimit(libc::RLIMIT_CORE, &z);
				Ok(())
			});
		}
		c.status().map(|s| s.success()).unwrap_or(false)
	})
	.unwrap_or_else(|| {
		eprintln!("ooc: no growth band above 1 GiB survived probing");
		0
	})
}

fn chunks(n: usize, c: usize) -> impl Iterator<Item = (usize, usize)> {
	(0..n.div_ceil(c)).map(move |i| (i * c, c.min(n - i * c)))
}

enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk(u64), // byte offset in the shared spill file
	Remote { node: usize, id: u64 }, // blob on net[node] — the tier below disk
}

/// One full-batch logical buffer, [n × spb] f64, homed PER WINDOW by the
/// waterfall — VRAM fills to the gate line at window granularity instead of
/// all-or-nothing per buffer. `spb` (f64s per sample) can change when a
/// ping-pong buffer is reused at another width; each window's backing is sized
/// for the creation width and later widths use its prefix. `chunk` is the
/// window sample count every sweep iterates by, so (s0, cnt) always addresses
/// exactly one window.
struct Paged {
	homes: Vec<Home>,
	spb: usize,
	chunk: usize,
	// Sequential read-ahead, depth 2: every disk read keeps the next TWO
	// disk-resident windows in flight, so the NVMe sees queue depth and
	// streams underneath the GPU instead of taking turns with it.
	ahead: RefCell<std::collections::VecDeque<(usize, usize, std::thread::JoinHandle<Vec<u8>>)>>,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
}

impl Paged {
	fn win(&self, s0: usize, cnt: usize) -> usize {
		assert!(s0 % self.chunk == 0 && cnt <= self.chunk, "ooc access not window-aligned");
		s0 / self.chunk
	}
	fn kick_ahead(&self, s0: usize, cnt: usize, n: usize, spill: &File, host: &HostPool) {
		let mut q = self.ahead.borrow_mut();
		let mut next0 = q.back().map_or(s0 + cnt, |(p0, pc, _)| p0 + pc);
		while q.len() < AHEAD && next0 < n {
			let next_cnt = cnt.min(n - next0);
			let len = next_cnt * self.spb * 8;
			let h = match &self.homes[next0 / self.chunk] {
				Home::Disk(off) => {
					let off = *off;
					let f = spill.try_clone().expect("spill clone");
					let hp = host.clone();
					std::thread::spawn(move || {
						let mut buf = pool_take(&hp);
						f.read_exact_at(&mut buf[..len], off).expect("ooc spill read-ahead");
						DISK_R_BYTES.fetch_add(len, Ordering::Relaxed);
						unsafe {
							use std::os::unix::io::AsRawFd;
							libc::posix_fadvise(f.as_raw_fd(), off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
						}
						buf
					})
				}
				Home::Remote { node, id } => {
					let (node, id) = (*node, *id);
					let nt = Arc::clone(self.net.as_ref().expect("remote home without net"));
					let hp = host.clone();
					std::thread::spawn(move || {
						let mut buf = pool_take(&hp);
						let v = nt[node].fetch(id, 0, len as u64).expect("ooc net read-ahead");
						buf[..len].copy_from_slice(&v);
						NET_R_BYTES.fetch_add(len, Ordering::Relaxed);
						buf
					})
				}
				_ => return, // next window is a fast tier — nothing to hide
			};
			q.push_back((next0, next_cnt, h));
			next0 += next_cnt;
		}
	}
	fn drain_ahead(&self, host: &HostPool) {
		for (_, _, h) in self.ahead.borrow_mut().drain(..) {
			pool_give(host, h.join().expect("read-ahead thread"));
		}
	}
	/// Device view of samples [s0, s0+cnt): VRAM-homed windows return an
	/// offset view (zero copies); RAM/DISK windows stage into `win` first.
	fn read(&self, s0: usize, cnt: usize, win: &GpuBuffer, spill: &File, n: usize, host: &HostPool) -> GpuBuffer {
		let len = cnt * self.spb * 8;
		match &self.homes[self.win(s0, cnt)] {
			Home::Vram(b) => view(b, 0, len),
			Home::Ram(v) => {
				win.write_u8(&v[..len]).expect("ooc H2D");
				view(win, 0, len)
			}
			Home::Disk(off) => {
				let pre = self.ahead.borrow_mut().pop_front();
				let bytes = match pre {
					Some((p0, pc, h)) if p0 == s0 && pc == cnt => h.join().expect("read-ahead thread"),
					other => {
						// Wrong window prefetched (new sweep) — drain the queue, read sync.
						if let Some((_, _, h)) = other {
							pool_give(host, h.join().expect("read-ahead thread"));
						}
						self.drain_ahead(host);
						let mut buf = pool_take(host);
						spill.read_exact_at(&mut buf[..len], *off).expect("ooc spill read");
						DISK_R_BYTES.fetch_add(len, Ordering::Relaxed);
						unsafe {
							use std::os::unix::io::AsRawFd;
							libc::posix_fadvise(spill.as_raw_fd(), *off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
						}
						buf
					}
				};
				self.kick_ahead(s0, cnt, n, spill, host);
				win.write_u8(&bytes[..len]).expect("ooc H2D");
				pool_give(host, bytes);
				view(win, 0, len)
			}
			Home::Remote { node, id } => {
				let pre = self.ahead.borrow_mut().pop_front();
				let bytes = match pre {
					Some((p0, pc, h)) if p0 == s0 && pc == cnt => h.join().expect("read-ahead thread"),
					other => {
						if let Some((_, _, h)) = other {
							pool_give(host, h.join().expect("read-ahead thread"));
						}
						self.drain_ahead(host);
						let mut buf = pool_take(host);
						let nt = self.net.as_ref().expect("remote home without net");
						let v = nt[*node].fetch(*id, 0, len as u64).expect("ooc net read");
						buf[..len].copy_from_slice(&v);
						NET_R_BYTES.fetch_add(len, Ordering::Relaxed);
						buf
					}
				};
				self.kick_ahead(s0, cnt, n, spill, host);
				win.write_u8(&bytes[..len]).expect("ooc H2D");
				pool_give(host, bytes);
				view(win, 0, len)
			}
		}
	}
	/// Device window a kernel writes samples [s0, s0+cnt) into. Pair with
	/// `commit(same view)` afterwards — a no-op for VRAM-homed windows.
	fn write_view(&self, s0: usize, cnt: usize, win: &GpuBuffer) -> GpuBuffer {
		let len = cnt * self.spb * 8;
		match &self.homes[self.win(s0, cnt)] {
			Home::Vram(b) => view(b, 0, len),
			_ => view(win, 0, len),
		}
	}
	fn commit(&mut self, s0: usize, cnt: usize, v: &GpuBuffer, writer: &Writer, host: &HostPool) {
		let len = cnt * self.spb * 8;
		let w = self.win(s0, cnt);
		match &mut self.homes[w] {
			Home::Vram(_) => {}
			Home::Ram(dst) => v.download_u8(&mut dst[..len]).expect("ooc D2H"),
			Home::Disk(off) => {
				let mut buf = pool_take(host);
				v.download_u8(&mut buf[..len]).expect("ooc D2H");
				writer.send(Dest::Disk(*off), buf, len);
			}
			Home::Remote { node, id } => {
				let mut buf = pool_take(host);
				v.download_u8(&mut buf[..len]).expect("ooc D2H");
				writer.send(Dest::Remote { node: *node, id: *id }, buf, len);
			}
		}
	}
}

/// Write-behind: disk commits queue here (bounded, so at most 2 windows of
/// host bytes are in flight) and a worker pwrites + drops the page cache while
/// the GPU moves on to the next window.
/// Write-behind pool: THREE workers, round-robin dispatch, each queue 2 deep
/// — the NVMe sees parallel writes instead of one serial pwrite stream.
#[derive(Clone, Copy)]
enum Dest {
	Disk(u64),
	Remote { node: usize, id: u64 },
}

struct Writer {
	lanes: Vec<(Option<std::sync::mpsc::SyncSender<(Dest, Vec<u8>, usize)>>, Option<std::thread::JoinHandle<()>>)>,
	next: std::cell::Cell<usize>,
	host: HostPool,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
	// Seconds spent draining lanes since the last sweep line — epoch-to-epoch
	// wander on identical work is write-behind drain landing on whichever
	// sweep's barrier catches it; the line surfaces it as `drain Xs`.
	drained: std::cell::Cell<f64>,
}

const W_LANES: usize = 3;
const WQ_DEPTH: usize = 2;
const AHEAD: usize = 2;
// The widest read fan-out of any sweep: flash backward pulls q/k/v/ctx/dctx
// through disk queues in one window body.
const MAX_READERS: usize = 5;
// Exact structural max of concurrently checked-out host windows: every
// mid-sweep reader holds AHEAD queued prefetches (one Paged at a time also
// holds its current buffer: +1), each writer lane holds its queue plus the
// buffer being pwritten, and one commit stages before its send lands.
const POOL_BUFS: usize = MAX_READERS * AHEAD + 1 + W_LANES * (WQ_DEPTH + 1) + 1;

fn spawn_lane(spill: &File, host: &HostPool, net: &Option<Arc<Vec<crate::wire::Conn>>>) -> (Option<std::sync::mpsc::SyncSender<(Dest, Vec<u8>, usize)>>, Option<std::thread::JoinHandle<()>>) {
	let f = spill.try_clone().expect("spill clone");
	let hp = host.clone();
	let nt = net.clone();
	let (tx, rx) = std::sync::mpsc::sync_channel::<(Dest, Vec<u8>, usize)>(WQ_DEPTH);
	let worker = std::thread::spawn(move || {
		for (dest, buf, len) in rx {
			match dest {
				Dest::Disk(off) => {
					f.write_all_at(&buf[..len], off).expect("ooc spill write");
					DISK_W_BYTES.fetch_add(len, Ordering::Relaxed);
					drop_cache(&f, off, len);
				}
				Dest::Remote { node, id } => {
					let nt = nt.as_ref().expect("remote dest without net");
					nt[node].store_from(id, &buf[..len]).expect("ooc net write");
					NET_W_BYTES.fetch_add(len, Ordering::Relaxed);
				}
			}
			pool_give(&hp, buf);
		}
	});
	(Some(tx), Some(worker))
}

impl Writer {
	fn new(spill: &File, host: HostPool, net: Option<Arc<Vec<crate::wire::Conn>>>) -> Writer {
		Writer {
			lanes: (0..W_LANES).map(|_| spawn_lane(spill, &host, &net)).collect(),
			next: std::cell::Cell::new(0),
			host,
			net,
			drained: std::cell::Cell::new(0.0),
		}
	}
	fn send(&self, dest: Dest, buf: Vec<u8>, len: usize) {
		let i = self.next.get();
		self.next.set((i + 1) % W_LANES);
		self.lanes[i].0.as_ref().expect("writer live").send((dest, buf, len)).expect("writer send");
	}
	/// Drain every lane — REQUIRED before any read of a disk home that was
	/// just written this sweep (the next sweep reads it).
	fn barrier(&mut self, spill: &File) {
		let t = std::time::Instant::now();
		for lane in &mut self.lanes {
			if let Some(tx) = lane.0.take() {
				drop(tx);
			}
			if let Some(w) = lane.1.take() {
				w.join().expect("writer join");
			}
			*lane = spawn_lane(spill, &self.host, &self.net);
		}
		self.drained.set(self.drained.get() + t.elapsed().as_secs_f64());
	}
}

pub struct Plan {
	pub vram: usize,
	pub ram: usize,
	pub disk: usize,
	pub remote: usize,
}

/// Combined-ceiling admit in waterfall order from live measurements. `None`
/// only when VRAM+RAM+DISK together cannot hold `need` — the sole abort.
// THE reserve law: exactly 1 GB of each tier belongs to the user; every
// other byte is ours to fill. No ratios, no floors, no guesses. The constant
// lives in gpu-core next to the alloc gate that enforces it.
pub use gpu_core::memory::USER_GB;

pub fn plan(need: usize, net_ram: usize) -> Option<Plan> {
	let vram_avail = gpu_core::memory::claimable_bytes();
	let ram_avail = mem_available().saturating_sub(USER_GB);
	let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
	let disk_avail = disk_free(&cwd).saturating_sub(USER_GB);
	if need > vram_avail + ram_avail + disk_avail + net_ram {
		return None;
	}
	let vram = need.min(vram_avail);
	let ram = (need - vram).min(ram_avail);
	let disk = (need - vram - ram).min(disk_avail);
	Some(Plan { vram, ram, disk, remote: need - vram - ram - disk })
}

/// The out-of-core buffer set + streaming state for one fit.
pub struct Ooc {
	n: usize,
	chunk: usize,
	wins: Vec<GpuBuffer>,
	spill: File,
	writer: Writer,
	host: HostPool,
	acts: Vec<Paged>,
	preacts: Vec<Option<Paged>>,
	a_q: Paged,
	a_k: Paged,
	a_v: Paged,
	a_ctx: Paged,
	a_dctx: Paged,
	a_dq: Paged,
	a_dk: Paged,
	a_dv: Paged,
	concat: Paged,
	da_a: Paged,
	da_b: Paged,
	lse: GpuBuffer,
	dsum: GpuBuffer,
	dw_acc: GpuBuffer,
	db_acc: GpuBuffer,
	dw_tmp: GpuBuffer,
	db_tmp: GpuBuffer,
	scalar_acc: GpuBuffer,
	scalar_tmp: GpuBuffer,
	dwq_acc: GpuBuffer,
	dwk_acc: GpuBuffer,
	dwv_acc: GpuBuffer,
	dw_partials: GpuBuffer,
	reduce_ws: GpuBuffer,
	// Streamed-tier rates measured at build (bytes/sec): one window pushed
	// through each production path — the PCIe/NVMe rooflines every sweep line
	// reports its achieved rate against.
	rate_h2d: f64,
	rate_d2h: f64,
	rate_disk_r: f64,
	rate_disk_w: f64,
	rate_net_r: f64,
	rate_net_w: f64,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
	// ONE probe-verified slab holds every VRAM-homed window as a carved view —
	// the pool is never asked to grow near the top of the card (the gate
	// admits asks VmHeap cannot map there; see probe_verified_vram).
	_slab: GpuBuffer,
}

/// Wall clock + cumulative transfer counters at sweep start, so the sweep line
/// can attribute exactly this sweep's PCIe and disk bytes.
struct SweepStart {
	t: std::time::Instant,
	h2d: usize,
	d2h: usize,
	disk_r: usize,
	disk_w: usize,
	net_r: usize,
	net_w: usize,
}

fn sweep_start() -> SweepStart {
	let (h2d, d2h, _) = gpu_core::memory::xfer_bytes();
	SweepStart {
		t: std::time::Instant::now(),
		h2d,
		d2h,
		disk_r: DISK_R_BYTES.load(Ordering::Relaxed),
		disk_w: DISK_W_BYTES.load(Ordering::Relaxed),
		net_r: NET_R_BYTES.load(Ordering::Relaxed),
		net_w: NET_W_BYTES.load(Ordering::Relaxed),
	}
}

impl Ooc {
	/// Lay every full-batch buffer out across the waterfall and size the
	/// streaming windows. Windows are sample-aligned; the widest op (flash
	/// backward) holds 9 at once, so VRAM's window share is carved into 10.
	pub fn build(params: &[LayerParams], n: usize, concat_ac: Option<(usize, usize)>, net: Option<Arc<Vec<crate::wire::Conn>>>) -> Ooc {
		let attn = params.iter().find(|p| p.kind == LayerKind::Attn);
		let (seq_spb, hs) = attn.map_or((1, 1), |p| (p.in_dim, p.heads * (p.in_dim / p.dim)));
		let max_act_spb = params.iter().map(|p| p.out_dim.max(p.in_dim)).max().unwrap_or(1);
		let (ca, cc) = concat_ac.unwrap_or((0, 0));
		let max_spb = seq_spb.max(max_act_spb).max(ca + cc);

		let spill_path = std::env::current_dir()
			.unwrap_or_else(|_| ".".into())
			.join(".recipe_spill");
		let spill = OpenOptions::new()
			.read(true)
			.write(true)
			.create(true)
			.truncate(true)
			.open(&spill_path)
			.expect("open spill file");
		// Unlink immediately: the fd keeps the file alive, and the space
		// reclaims itself no matter how the process dies (a SIGKILLed run
		// leaked a 40 GB spill on disk once).
		let _ = std::fs::remove_file(&spill_path);

		let max_wt = params
			.iter()
			.map(|p| match p.kind {
				LayerKind::Dense => p.in_dim * p.out_dim,
				LayerKind::Attn => p.dim * p.dim,
				LayerKind::Embed => p.vocab * p.dim,
				LayerKind::Conv => p.in_dim * p.out_dim,
			})
			.max()
			.unwrap_or(1);
		let max_bias = params.iter().map(|p| p.out_dim).max().unwrap_or(1);
		const WINS: usize = 10;
		// Budget from the honest claimability (the alloc gate's own measure)
		// AFTER reserving the fixed residents (lse/dsum/grad accumulators); half
		// of what remains stages windows (+2 windows of margin for the
		// chunk-sized dw_partials/reduce_ws), the waterfall homes fill the rest.
		let fixed_res = (2 * n * hs + 6 * max_wt + 2 * max_bias + 2) * 8;
		let win_budget = gpu_core::memory::claimable_bytes().saturating_sub(fixed_res) / 2;
		let chunk = (win_budget / ((WINS + 2) * max_spb * 8)).clamp(1, n);
		let wins: Vec<GpuBuffer> = (0..WINS)
			.map(|_| GpuBuffer::alloc(chunk * max_spb).expect("ooc window"))
			.collect();
		// Host pool BEFORE the RAM measurement below, faulted in across all
		// cores — its bytes are part of the budget, permanently resident.
		let wbytes = chunk * max_spb * 8;
		let host: HostPool = Arc::new(Mutex::new(
			(0..POOL_BUFS)
				.map(|_| {
					let mut v = vec![0u8; wbytes];
					gpu_core::memory::par_touch(&mut v);
					v
				})
				.collect(),
		));
		let writer = Writer::new(&spill, host.clone(), net.clone());
		// One window through each streamed path, timed — the measured PCIe/NVMe
		// rates the sweep lines report against. Window-sized (the exact unit the
		// epoch streams); the spill region is overwritten by real windows later.
		// Drain the device first: the wins were just hipMallocAsync'd and an
		// immediate copy into uncommitted pool pages GPU-page-faults (the
		// ALLOC_SYNC pattern in gpu-core/memory.rs).
		gpu_core::hip::device_synchronize().expect("ooc calibrate sync");
		let (rate_h2d, rate_d2h, rate_disk_r, rate_disk_w) = {
			let bps = |b: usize, t: std::time::Instant| b as f64 / t.elapsed().as_secs_f64();
			let mut buf = pool_take(&host);
			let t = std::time::Instant::now();
			wins[0].write_u8(&buf[..wbytes]).expect("calibrate h2d");
			let h2d = bps(wbytes, t);
			let t = std::time::Instant::now();
			wins[0].download_u8(&mut buf[..wbytes]).expect("calibrate d2h");
			let d2h = bps(wbytes, t);
			let t = std::time::Instant::now();
			spill.write_all_at(&buf[..wbytes], 0).expect("calibrate spill write");
			drop_cache(&spill, 0, wbytes);
			let w = bps(wbytes, t);
			let t = std::time::Instant::now();
			spill.read_exact_at(&mut buf[..wbytes], 0).expect("calibrate spill read");
			let r = bps(wbytes, t);
			pool_give(&host, buf);
			(h2d, d2h, r, w)
		};
		let seq_rows = attn.map_or(chunk, |p| chunk * (p.in_dim / p.dim));
		let mut dwp = 1usize;
		let mut ws = kernels::gpu_reduce_sum_cols_workspace_bytes(chunk, 1);
		for p in params {
			let e = match p.kind {
				LayerKind::Dense => kernels::gpu_splitk_dw_partials_elems(chunk, p.in_dim, p.out_dim),
				LayerKind::Attn => kernels::gpu_splitk_dw_partials_elems(seq_rows, p.dim, p.dim),
				_ => 0,
			};
			dwp = dwp.max(e);
			let rows = if p.kind == LayerKind::Attn { seq_rows } else { chunk };
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(rows, p.out_dim.max(p.dim)));
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(rows * p.out_dim.max(p.dim), 1));
		}

		// Resident allocations FIRST — the waterfall fill below takes VRAM to
		// refusal, so everything that must stay device-resident is claimed before.
		let lse = GpuBuffer::alloc(n * hs).expect("ooc lse");
		let dsum = GpuBuffer::alloc(n * hs).expect("ooc dsum");
		let dw_acc = GpuBuffer::alloc(max_wt).expect("ooc dw_acc");
		let db_acc = GpuBuffer::alloc(max_bias).expect("ooc db_acc");
		let dw_tmp = GpuBuffer::alloc(max_wt).expect("ooc dw_tmp");
		let db_tmp = GpuBuffer::alloc(max_bias).expect("ooc db_tmp");
		let scalar_acc = GpuBuffer::alloc(1).expect("ooc scalar_acc");
		let scalar_tmp = GpuBuffer::alloc(1).expect("ooc scalar_tmp");
		let dwq_acc = GpuBuffer::alloc(max_wt).expect("ooc dwq_acc");
		let dwk_acc = GpuBuffer::alloc(max_wt).expect("ooc dwk_acc");
		let dwv_acc = GpuBuffer::alloc(max_wt).expect("ooc dwv_acc");
		let dw_partials = GpuBuffer::alloc(dwp).expect("ooc dw_partials");
		let reduce_ws = GpuBuffer::alloc_bytes(ws).expect("ooc reduce_ws");

		// The windows' VRAM share: one probe-verified slab, committed here,
		// carved below. Probing runs with the residents above already mapped,
		// so the child's survival proves exactly the co-mapping this process
		// is about to hold. Walk down on a gate refusal (its measure can sit
		// under the probe's).
		let slab = {
			// Reuse of retained pool slack is already-committed pages (probe-
			// free); only the growth band needs a child's survival proof.
			let growable = gpu_core::memory::vram_free_base().saturating_sub(gpu_core::memory::USER_GB);
			let reusable = gpu_core::memory::claimable_bytes()
				.saturating_sub(gpu_core::memory::arena_remaining())
				.saturating_sub(growable);
			let mut want = reusable + probe_verified_vram();
			loop {
				if want < (1 << 21) {
					// VRAM genuinely full — every window waterfalls to RAM/disk.
					break GpuBuffer::alloc(1).expect("ooc slab stub");
				}
				if let Some(b) = GpuBuffer::try_alloc_bytes(want) {
					b.memset_zero(want).expect("ooc slab commit");
					gpu_core::hip::device_synchronize().expect("ooc slab sync");
					break b;
				}
				want -= want / 16;
			}
		};
		let slab_bytes = if slab.len() > (1 << 21) { slab.len() } else { 0 };
		let mut slab_off = 0usize;

		// RAM accounting must be CUMULATIVE: fresh Vec pages are lazily
		// zero-backed, so mem_available() does not drop until an epoch touches
		// them — re-measuring per placement admits unbounded virtual memory and
		// the OOM killer collects mid-epoch. Every transient host byte comes
		// from the preallocated (touched, already-measured) pool above, so the
		// floor is exactly the user's gigabyte — homes fill RAM to top−1GB and
		// it STAYS there while disk churns.
		let ram_start = mem_available();
		let ram_floor = USER_GB;
		let mut ram_used = 0usize;
		let mut vram_open = true;
		let mut disk_cursor: u64 = 0;
		// Disk fills to ITS reserve line, then windows go remote: each pooled
		// node takes up to its HELLO-reported MemAvailable minus the same 1 GB
		// user reserve (the reserve law applies on the far side too).
		let disk_budget = disk_free(&std::env::current_dir().unwrap_or_else(|_| ".".into()))
			.saturating_sub(USER_GB);
		let net_caps: Vec<usize> = net.as_ref().map_or(Vec::new(), |ns| {
			ns.iter().map(|c| (c.info.ram as usize).saturating_sub(USER_GB)).collect()
		});
		let mut net_used = vec![0usize; net_caps.len()];
		// Blob ids carry a coordinator fingerprint in the high half — two
		// machines pooling the same node must not collide.
		let id_base: u64 = {
			let host_s = std::fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
			let mut h = 0xcbf2_9ce4_8422_2325u64;
			for b in host_s.trim().bytes().chain(std::process::id().to_le_bytes()) {
				h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
			}
			h << 32
		};
		let mut next_id: u64 = 0;
		// Per-WINDOW placement: the waterfall fills VRAM window by window until
		// the gate refuses (top minus the user GB), THEN RAM to its line, THEN
		// disk — not a byte pools in a slower tier while a faster one has room.
		let mut place = |spb: usize| -> Paged {
			let n_wins = n.div_ceil(chunk);
			let mut homes = Vec::with_capacity(n_wins);
			for w in 0..n_wins {
				let cnt = chunk.min(n - w * chunk);
				let bytes = cnt * spb * 8;
				if vram_open {
					// Carve from the committed slab — zero pool traffic; the
					// waterfall's "fill to refusal" is now slab exhaustion.
					let aligned = (bytes + 4095) & !4095;
					if slab_off + aligned <= slab_bytes {
						homes.push(Home::Vram(view(&slab, slab_off, bytes)));
						slab_off += aligned;
						continue;
					}
					vram_open = false;
				}
				if ram_start.saturating_sub(ram_used + bytes) > ram_floor {
					ram_used += bytes;
					let mut v = vec![0u8; bytes];
					// Fault the pages in NOW across all cores — calloc pages are
					// lazy and would otherwise materialize one at a time inside
					// the epoch (RAM climbing at 8% CPU).
					gpu_core::memory::par_touch(&mut v);
					homes.push(Home::Ram(v));
					continue;
				}
				if disk_cursor as usize + bytes <= disk_budget {
					homes.push(Home::Disk(disk_cursor));
					disk_cursor += bytes as u64;
					continue;
				}
				let node = (0..net_caps.len()).find(|&nd| net_used[nd] + bytes <= net_caps[nd]);
				let Some(node) = node else {
					panic!(
						"ooc: window has no home — disk {disk_cursor}B of {disk_budget}B, remote {net_used:?} of {net_caps:?} (admit passed what placement cannot hold)"
					);
				};
				net_used[node] += bytes;
				homes.push(Home::Remote { node, id: id_base | next_id });
				next_id += 1;
			}
			Paged { homes, spb, chunk, ahead: RefCell::new(std::collections::VecDeque::new()), net: net.clone() }
		};

		// Placement in HOTNESS order — the waterfall gives the fastest tier to
		// the buffers touched most per epoch, so disk sees the coldest traffic:
		// da ping-pongs hit every layer (3-4 R/W each), ctx is 1W+3R, q/k/v and
		// acts 1W+2R, concat 1W+2R, the d* grads 1W+1R, preacts 1W+1R.
		let da_a = place(max_spb);
		let da_b = place(max_spb);
		let a_ctx = place(seq_spb);
		let a_q = place(seq_spb);
		let a_k = place(seq_spb);
		let a_v = place(seq_spb);
		let acts: Vec<Paged> = params.iter().map(|p| place(p.out_dim)).collect();
		let concat = place(if ca + cc > 0 { ca + cc } else { 1 });
		let a_dctx = place(seq_spb);
		let a_dq = place(seq_spb);
		let a_dk = place(seq_spb);
		let a_dv = place(seq_spb);
		let preacts: Vec<Option<Paged>> = params
			.iter()
			.map(|p| {
				matches!(
					p.act,
					Activation::Silu
						| Activation::Gelu | Activation::Elu
						| Activation::Selu | Activation::PRelu
				)
				.then(|| place(p.out_dim))
			})
			.collect();
		if disk_cursor > 0 {
			spill.set_len(disk_cursor).expect("size spill file");
		}
		// One window through the wire when any window homed remote — the net
		// roofline the sweep lines report against.
		let (rate_net_r, rate_net_w) = if net_used.iter().any(|u| *u > 0) {
			let ns = net.as_ref().expect("net used without net");
			let cal_id = id_base | 0xffff_ffff;
			let buf = pool_take(&host);
			let bps = |b: usize, t: std::time::Instant| b as f64 / t.elapsed().as_secs_f64();
			let t = std::time::Instant::now();
			ns[0].store_from(cal_id, &buf[..wbytes]).expect("calibrate net write");
			let w = bps(wbytes, t);
			let t = std::time::Instant::now();
			let v = ns[0].fetch(cal_id, 0, wbytes as u64).expect("calibrate net read");
			let r = bps(v.len(), t);
			let _ = ns[0].free(cal_id);
			pool_give(&host, buf);
			(r, w)
		} else {
			(0.0, 0.0)
		};

		Ooc {
			n,
			chunk,
			wins,
			spill,
			writer,
			host,
			acts,
			preacts,
			a_q,
			a_k,
			a_v,
			a_ctx,
			a_dctx,
			a_dq,
			a_dk,
			a_dv,
			concat,
			da_a,
			da_b,
			lse,
			dsum,
			dw_acc,
			db_acc,
			dw_tmp,
			db_tmp,
			scalar_acc,
			scalar_tmp,
			dwq_acc,
			dwk_acc,
			dwv_acc,
			dw_partials,
			reduce_ws,
			rate_h2d,
			rate_d2h,
			rate_disk_r,
			rate_disk_w,
			rate_net_r,
			rate_net_w,
			net,
			_slab: slab,
		}
	}

	pub fn report(&self) {
		let gb = |b: usize| b as f64 / (1u64 << 30) as f64;
		let (mut v, mut r, mut d, mut nt) = (0usize, 0usize, 0usize, 0usize);
		let mut tally = |p: &Paged| {
			for (w, h) in p.homes.iter().enumerate() {
				let cnt = p.chunk.min(self.n - w * p.chunk);
				match h {
					Home::Vram(_) => v += cnt * p.spb * 8,
					Home::Ram(x) => r += x.len(),
					Home::Disk(_) => d += cnt * p.spb * 8,
					Home::Remote { .. } => nt += cnt * p.spb * 8,
				}
			}
		};
		for a in &self.acts {
			tally(a);
		}
		for pa in self.preacts.iter().flatten() {
			tally(pa);
		}
		for b in [
			&self.a_q, &self.a_k, &self.a_v, &self.a_ctx, &self.a_dctx, &self.a_dq, &self.a_dk,
			&self.a_dv, &self.concat, &self.da_a, &self.da_b,
		] {
			tally(b);
		}
		eprintln!(
			"\x1b[33mwaterfall\x1b[0m  scratch homes: VRAM {:.2} GB → RAM {:.2} GB → DISK {:.2} GB → NET {:.2} GB, {}-sample windows",
			gb(v),
			gb(r),
			gb(d),
			gb(nt),
			self.chunk
		);
		eprintln!(
			"\x1b[33mwaterfall\x1b[0m  measured rooflines: gemm {} GF/s  vram {} GB/s  h2d {:.2} GB/s  d2h {:.2} GB/s  disk-r {:.2} GB/s  disk-w {:.2} GB/s",
			recipe_infer::GEMM_GFLOPS,
			recipe_infer::VRAM_GBS,
			self.rate_h2d / 1e9,
			self.rate_d2h / 1e9,
			self.rate_disk_r / 1e9,
			self.rate_disk_w / 1e9,
		);
		if self.rate_net_r > 0.0 {
			eprintln!(
				"\x1b[33mwaterfall\x1b[0m  net roofline: net-r {:.3} GB/s  net-w {:.3} GB/s ({} nodes)",
				self.rate_net_r / 1e9,
				self.rate_net_w / 1e9,
				self.net.as_ref().map_or(0, |n| n.len()),
			);
		}
	}

	/// Full-batch forward swept in windows. `x`/`x_cat`/`sc.acts[last]` are
	/// resident; the last layer writes `sc.acts[last]` so the metric plumbing
	/// works unchanged.
	pub fn forward(
		&mut self,
		params: &[LayerParams],
		x: &GpuBuffer,
		x_cat: Option<&GpuBuffer>,
		sc: &Scratch,
		concat_at: Option<(usize, usize, usize)>,
	) {
		let last = params.len() - 1;
		for (l, p) in params.iter().enumerate() {
			// The concat build is its own sweep with its own line — folded into
			// the consuming dense's timer it read as that layer's slowness.
			if let Some((pf, a, c)) = concat_at
				&& l == pf
			{
				let s_c = sweep_start();
				self.writer.barrier(&self.spill);
				for (s0, cnt) in chunks(self.n, self.chunk) {
					if self.bail() {
						return;
					}
					let prev = self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
					let xc = view(x_cat.expect("x_cat"), s0 * c * 8, cnt * c * 8);
					let out = self.concat.write_view(s0, cnt, &self.wins[1]);
					kernels::gpu_concat_into(&prev, &xc, cnt, a, c, &out).expect("concat");
					self.concat.commit(s0, cnt, &out, &self.writer, &self.host);
				}
				// Pure copy: [n×A]+[n×C] read, [n×(A+C)] written, zero FLOP.
				let cw = recipe_infer::Work { flop: 0.0, bytes: 16.0 * (self.n * (a + c)) as f64 };
				self.sweep_line_work("fwd", l, "concat", &format!("{a}+{c}"), cw, s_c);
			}
			let s_l = sweep_start();
			match p.kind {
				LayerKind::Embed => {
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if self.bail() {
							return;
						}
						let ids = view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8);
						let out = self.acts[l].write_view(s0, cnt, &self.wins[0]);
						kernels::gpu_gather_rows_into(&p.w, &ids, cnt * p.in_dim, p.dim, &out).expect("gather");
						kernels::gpu_broadcast_sub_into(&out, &p.b, cnt * p.out_dim, p.out_dim, &out).expect("pe add");
						self.acts[l].commit(s0, cnt, &out, &self.writer, &self.host);
					}
				}
				LayerKind::Attn => {
					let d = p.dim;
					let heads = p.heads;
					let s = p.in_dim / d;
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if self.bail() {
							return;
						}
						let prev = self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
						let m = cnt * s;
						let q = self.a_q.write_view(s0, cnt, &self.wins[1]);
						let k = self.a_k.write_view(s0, cnt, &self.wins[2]);
						let v = self.a_v.write_view(s0, cnt, &self.wins[3]);
						kernels::gpu_linear_into(&prev, &p.w, &p.b, m, d, d, &q).expect("attn q");
						kernels::gpu_linear_into(&prev, &p.wk, &p.b, m, d, d, &k).expect("attn k");
						kernels::gpu_linear_into(&prev, &p.wv, &p.b, m, d, d, &v).expect("attn v");
						gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_one, &sc.c_rope_theta, m, d, heads, s, &q, &k).expect("rope");
						self.a_q.commit(s0, cnt, &q, &self.writer, &self.host);
						self.a_k.commit(s0, cnt, &k, &self.writer, &self.host);
						self.a_v.commit(s0, cnt, &v, &self.writer, &self.host);
					}
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if self.bail() {
							return;
						}
						let q = self.a_q.read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
						let k = self.a_k.read(s0, cnt, &self.wins[1], &self.spill, self.n, &self.host);
						let v = self.a_v.read(s0, cnt, &self.wins[2], &self.spill, self.n, &self.host);
						let ctx = self.a_ctx.write_view(s0, cnt, &self.wins[3]);
						let lse = view(&self.lse, s0 * heads * s * 8, cnt * heads * s * 8);
						kernels::gpu_flash_attention_train_into(&q, &k, &v, cnt, s, d, heads, &ctx, &lse).expect("flash attn");
						self.a_ctx.commit(s0, cnt, &ctx, &self.writer, &self.host);
					}
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if self.bail() {
							return;
						}
						let ctx = self.a_ctx.read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
						let out = self.acts[l].write_view(s0, cnt, &self.wins[1]);
						kernels::gpu_linear_into(&ctx, &p.wo, &p.b, cnt * s, d, d, &out).expect("attn out");
						self.acts[l].commit(s0, cnt, &out, &self.writer, &self.host);
					}
				}
				LayerKind::Dense | LayerKind::Conv => {
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if self.bail() {
							return;
						}
						let prev = if l == 0 {
							view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8)
						} else if Some(l) == concat_at.map(|t| t.0) {
							self.concat.read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host)
						} else {
							self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host)
						};
						let out = if l == last {
							view(&sc.acts[last], s0 * p.out_dim * 8, cnt * p.out_dim * 8)
						} else {
							self.acts[l].write_view(s0, cnt, &self.wins[1])
						};
						if p.kind == LayerKind::Conv {
							let (cin, kk, stride) = (p.conv_cin, p.conv_k, p.conv_stride);
							let lin = p.in_dim / cin;
							let cout = p.out_dim / ((lin - kk) / stride + 1);
							kernels::gpu_conv1d_into(&prev, &p.w, &p.b, cnt, cin, lin, cout, kk, stride, &out).expect("conv1d");
						} else if p.out_dim == 1 {
							kernels::gpu_matvec_bias_into(&prev, &p.w, &p.b, cnt, p.in_dim, &out).expect("matvec");
						} else {
							kernels::gpu_linear_into(&prev, &p.w, &p.b, cnt, p.out_dim, p.in_dim, &out).expect("linear");
						}
						let m = cnt * p.out_dim;
						if let Some(pa) = self.preacts[l].as_mut() {
							let pre = pa.write_view(s0, cnt, &self.wins[2]);
							kernels::gpu_copy_into(&out, m, &pre).expect("copy preact");
							pa.commit(s0, cnt, &pre, &self.writer, &self.host);
							// PRelu/Elu/Selu/Silu/Gelu apply FROM the saved z.
							match p.act {
								Activation::PRelu => {
									kernels::gpu_leaky_relu_into(&pre, &p.palpha, m, &out).expect("prelu");
								}
								Activation::Elu => gpu_core::k_gapact::gpu_elu(&pre, &sc.c_elu_alpha, m, &out).expect("elu"),
								Activation::Selu => gpu_core::k_gapact::gpu_selu(&pre, &sc.c_selu_alpha, &sc.c_selu_lambda, m, &out).expect("selu"),
								Activation::Silu => kernels::gpu_silu_into(&pre, m, &out).expect("silu"),
								Activation::Gelu => kernels::gpu_gelu_into(&pre, m, &out).expect("gelu"),
								_ => unreachable!("preact only saved for z-based activations"),
							}
						} else {
							match p.act {
								Activation::Relu => kernels::gpu_relu_into(&out, m, &out).expect("relu"),
								Activation::Sigmoid => kernels::gpu_sigmoid_into(&out, m, &out).expect("sigmoid"),
								Activation::LeakyRelu => kernels::gpu_leaky_relu_into(&out, &sc.c_leaky_alpha, m, &out).expect("leaky"),
								Activation::Tanh => kernels::gpu_tanh_into(&out, m, &out).expect("tanh"),
								Activation::Linear => {}
								_ => unreachable!("z-based activation without preact"),
							}
						}
						if l != last {
							self.acts[l].commit(s0, cnt, &out, &self.writer, &self.host);
						}
					}
				}
			}
			self.sweep_line("fwd", l, p, s_l);
		}
	}

	/// Full-batch backward + one SGD update per layer, swept in windows.
	/// Weight grads accumulate across windows into dw_acc/db_acc (tiled
	/// reduction — same math as one pass), then update once.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn backward(
		&mut self,
		params: &[LayerParams],
		x: &GpuBuffer,
		ybuf: &GpuBuffer,
		sc: &Scratch,
		ss: &StepScalars,
		loss: Loss,
		concat_at: Option<(usize, usize, usize)>,
	) {
		let last = params.len() - 1;
		let n = self.n;
		// Loss gradient at the output, windowed into da_a (width = out_dim). The
		// batch-mean scale rides ss.inv_n/two_inv_n (global 1/n) so each window's
		// contribution is already at the full-batch scale — no per-window rescale.
		self.da_a.spb = params[last].out_dim;
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if self.bail() {
				return;
			}
			let k = params[last].out_dim;
			let out = view(&sc.acts[last], s0 * k * 8, cnt * k * 8);
			let y = view(ybuf, s0 * k * 8, cnt * k * 8);
			let da = self.da_a.write_view(s0, cnt, &self.wins[0]);
			ModelInner::loss_grad_into(loss, &out, &y, &da, cnt, cnt * k, sc, ss);
			self.da_a.commit(s0, cnt, &da, &self.writer, &self.host);
		}
		let mut flip = false;
		for l in (0..params.len()).rev() {
			let p = &params[l];
			let s_l = sweep_start();
			let (in_dim, out_dim) = (p.in_dim, p.out_dim);
			match p.kind {
				LayerKind::Embed => {
					kernels::gpu_scale_inplace(&ss.zero, p.vocab * p.dim, &sc.embed_grad).expect("embed zero");
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(n, self.chunk) {
						if self.bail() {
							return;
						}
						let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
						let ids = view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8);
						kernels::gpu_scatter_add(&ids, &da, cnt * p.in_dim, p.dim, &sc.embed_grad).expect("embed scatter");
					}
					kernels::gpu_sgd_update(&sc.embed_grad, &ss.neg_lr, p.vocab * p.dim, &p.w).expect("sgd embed");
					self.sweep_line("bwd", l, p, s_l);
					flip = !flip;
					continue;
				}
				LayerKind::Attn => {
					self.attn_backward(params, l, x, sc, ss, flip);
					self.sweep_line("bwd", l, p, s_l);
					flip = !flip;
					continue;
				}
				LayerKind::Conv => panic!(
					"out-of-core conv backward not implemented — a conv this large has no production caller yet"
				),
				LayerKind::Dense => {}
			}
			// Dense: dz = act'(da) per window; dW/db accumulate; da_below → home.
			kernels::gpu_scale_inplace(&ss.zero, in_dim * out_dim, &self.dw_acc).expect("dw_acc zero");
			kernels::gpu_scale_inplace(&ss.zero, out_dim, &self.db_acc).expect("db_acc zero");
			if p.act == Activation::PRelu {
				kernels::gpu_scale_inplace(&ss.zero, 1, &self.scalar_acc).expect("scalar_acc zero");
			}
			self.da_below(flip).spb = if l > 0 { in_dim } else { 1 };
			self.writer.barrier(&self.spill);
			for (s0, cnt) in chunks(n, self.chunk) {
				if self.bail() {
					return;
				}
				let m = cnt * out_dim;
				let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
				let act_l = if l == last {
					view(&sc.acts[last], s0 * out_dim * 8, cnt * out_dim * 8)
				} else {
					self.acts[l].read(s0, cnt, &self.wins[1], &self.spill, self.n, &self.host)
				};
				let dz = view(&self.wins[2], 0, m * 8);
				let grad: &GpuBuffer = match p.act {
					Activation::Relu => {
						kernels::gpu_relu_backward_into(&da, &act_l, m, &dz).expect("relu bwd");
						&dz
					}
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_backward_into(&da, &act_l, m, &dz).expect("sigmoid bwd");
						&dz
					}
					Activation::LeakyRelu => {
						kernels::gpu_leaky_relu_backward_into(&da, &act_l, &sc.c_leaky_alpha, m, &dz).expect("leaky bwd");
						&dz
					}
					Activation::PRelu => {
						kernels::gpu_leaky_relu_backward_into(&da, &act_l, &p.palpha, m, &dz).expect("prelu bwd");
						let pre = self.preacts[l]
							.as_ref()
							.expect("prelu preact")
							.read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
						let t0 = view(&self.wins[4], 0, m * 8);
						let t1 = view(&self.wins[5], 0, m * 8);
						kernels::gpu_relu_into(&pre, m, &t0).expect("prelu relu");
						kernels::gpu_copy_into(&pre, m, &t1).expect("prelu copy");
						kernels::gpu_sub_inplace(&t0, m, &t1).expect("prelu sub");
						kernels::gpu_mul_inplace(&da, m, &t1).expect("prelu mul");
						kernels::gpu_reduce_sum_cols_into(&t1, &self.reduce_ws, m, 1, &self.scalar_tmp).expect("prelu reduce");
						kernels::gpu_add_inplace(&self.scalar_tmp, 1, &self.scalar_acc).expect("prelu acc");
						&dz
					}
					Activation::Tanh => {
						kernels::gpu_tanh_backward_into(&da, &act_l, m, &dz).expect("tanh bwd");
						&dz
					}
					Activation::Elu => {
						let pre = self.preacts[l].as_ref().expect("elu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
						gpu_core::k_gapact::gpu_elu_backward(&da, &pre, &sc.c_elu_alpha, m, &dz).expect("elu bwd");
						&dz
					}
					Activation::Selu => {
						let pre = self.preacts[l].as_ref().expect("selu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
						gpu_core::k_gapact::gpu_selu_backward(&da, &pre, &sc.c_selu_alpha, &sc.c_selu_lambda, m, &dz).expect("selu bwd");
						&dz
					}
					Activation::Silu => {
						let pre = self.preacts[l].as_ref().expect("silu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
						kernels::gpu_silu_backward_into(&da, &pre, m, &dz).expect("silu bwd");
						&dz
					}
					Activation::Gelu => {
						let pre = self.preacts[l].as_ref().expect("gelu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
						kernels::gpu_gelu_backward_into(&da, &pre, m, &dz).expect("gelu bwd");
						&dz
					}
					Activation::Linear => &da,
				};
				let a_prev = if l == 0 {
					view(x, s0 * in_dim * 8, cnt * in_dim * 8)
				} else if Some(l) == concat_at.map(|t| t.0) {
					self.concat.read(s0, cnt, &self.wins[6], &self.spill, self.n, &self.host)
				} else {
					self.acts[l - 1].read(s0, cnt, &self.wins[6], &self.spill, self.n, &self.host)
				};
				if l > 0 {
					let below_pg = if flip { &mut self.da_a } else { &mut self.da_b };
					let below = below_pg.write_view(s0, cnt, &self.wins[7]);
					kernels::gpu_linear_backward_full_into(
						grad, &a_prev, &p.w, &self.reduce_ws, &self.dw_partials, cnt, out_dim, in_dim,
						&below, &self.dw_tmp, &self.db_tmp,
					).expect("linear full bwd");
					// The layer below the concat only wants the leading A
					// attention columns — compact before committing.
					if let Some((pf, a, c)) = concat_at
						&& l == pf
					{
						let compact = view(&self.wins[8], 0, cnt * a * 8);
						kernels::gpu_slice_lead_into(&below, cnt, a + c, a, &compact).expect("concat slice");
						below_pg.spb = a;
						below_pg.commit(s0, cnt, &compact, &self.writer, &self.host);
						below_pg.spb = a + c;
					} else {
						below_pg.commit(s0, cnt, &below, &self.writer, &self.host);
					}
				} else {
					kernels::gpu_linear_backward_weights_only_into(
						grad, &a_prev, &self.reduce_ws, &self.dw_partials, cnt, out_dim, in_dim,
						&self.dw_tmp, &self.db_tmp,
					).expect("linear weights bwd");
				}
				kernels::gpu_add_inplace(&self.dw_tmp, in_dim * out_dim, &self.dw_acc).expect("dw acc");
				kernels::gpu_add_inplace(&self.db_tmp, out_dim, &self.db_acc).expect("db acc");
			}
			if let Some((pf, a, _)) = concat_at
				&& l == pf
			{
				self.da_below(flip).spb = a;
			}
			kernels::gpu_sgd_update(&self.dw_acc, &ss.neg_lr, in_dim * out_dim, &p.w).expect("sgd w");
			kernels::gpu_sgd_update(&self.db_acc, &ss.neg_lr, out_dim, &p.b).expect("sgd b");
			if p.act == Activation::PRelu {
				kernels::gpu_sgd_update(&self.scalar_acc, &ss.neg_lr, 1, &p.palpha).expect("sgd prelu");
			}
			self.sweep_line("bwd", l, p, s_l);
			flip = !flip;
		}
	}

	// Persistent per-layer heartbeat — an out-of-core epoch moves ~100 GB
	// through the tiers before the first loss line, and silence looks like a
	// hang. One line per layer pass with wall time, the layer's analytic GFLOP
	// and VRAM GB (achieved rate as % of the gemm/vram rooflines), and this
	// sweep's streamed bytes per channel as % of the build-measured PCIe/NVMe
	// rates. Newline-terminated so it survives the dev wrapper own 1 Hz line.
	fn sweep_line(&self, phase: &str, l: usize, p: &LayerParams, s: SweepStart) {
		let kind = match p.kind {
			LayerKind::Embed => "embed",
			LayerKind::Attn => "attn",
			LayerKind::Conv => "conv",
			LayerKind::Dense => "dense",
		};
		let w = if phase == "fwd" {
			recipe_infer::layer_fwd(p, self.n)
		} else {
			recipe_infer::layer_bwd(p, self.n, l == 0)
		};
		self.sweep_line_work(phase, l, kind, &format!("{}→{}", p.in_dim, p.out_dim), w, s);
	}

	fn sweep_line_work(&self, phase: &str, l: usize, kind: &str, dims: &str, w: recipe_infer::Work, s: SweepStart) {
		// Drain the device before reading the clock, so the line includes this
		// sweep's own enqueued kernels instead of leaking the tail into the
		// next layer's line.
		gpu_core::hip::device_synchronize().expect("sweep sync");
		let sec = s.t.elapsed().as_secs_f64();
		let (gfs, gbs) = (w.flop / sec / 1e9, w.bytes / sec / 1e9);
		let mut line = format!(
			"ooc {phase} L{l} {kind} {dims}  {} windows  {sec:.1}s  {:.2} GFLOP {gfs:.1} GF/s {:.0}% gemm  {:.2} GB {gbs:.1} GB/s {:.0}% vram",
			self.n.div_ceil(self.chunk),
			w.flop / 1e9,
			100.0 * gfs / recipe_infer::GEMM_GFLOPS,
			w.bytes / 1e9,
			100.0 * gbs / recipe_infer::VRAM_GBS,
		);
		let (h2d, d2h, _) = gpu_core::memory::xfer_bytes();
		for (label, delta, rate) in [
			("h2d", h2d - s.h2d, self.rate_h2d),
			("d2h", d2h - s.d2h, self.rate_d2h),
			("disk-r", DISK_R_BYTES.load(Ordering::Relaxed) - s.disk_r, self.rate_disk_r),
			("disk-w", DISK_W_BYTES.load(Ordering::Relaxed) - s.disk_w, self.rate_disk_w),
			("net-r", NET_R_BYTES.load(Ordering::Relaxed) - s.net_r, self.rate_net_r),
			("net-w", NET_W_BYTES.load(Ordering::Relaxed) - s.net_w, self.rate_net_w),
		] {
			if delta > 0 {
				let bps = delta as f64 / sec;
				line += &format!(
					"  {label} {:.2} GB {:.2} GB/s {:.0}%",
					delta as f64 / 1e9,
					bps / 1e9,
					100.0 * bps / rate,
				);
			}
		}
		let drain = self.writer.drained.take();
		if drain > 0.0 {
			line += &format!("  drain {drain:.1}s");
		}
		eprintln!("{line}");
	}

	/// Every Paged buffer. The exhaustive destructure (no `..`) makes adding a
	/// field a compile error HERE, so a future Paged cannot silently miss the
	/// interrupt drain in `bail` and strand its read-ahead pool checkouts.
	fn all_paged(&self) -> impl Iterator<Item = &Paged> {
		let Ooc {
			n: _,
			chunk: _,
			wins: _,
			spill: _,
			writer: _,
			host: _,
			acts,
			preacts,
			a_q,
			a_k,
			a_v,
			a_ctx,
			a_dctx,
			a_dq,
			a_dk,
			a_dv,
			concat,
			da_a,
			da_b,
			lse: _,
			dsum: _,
			dw_acc: _,
			db_acc: _,
			dw_tmp: _,
			db_tmp: _,
			scalar_acc: _,
			scalar_tmp: _,
			dwq_acc: _,
			dwk_acc: _,
			dwv_acc: _,
			dw_partials: _,
			reduce_ws: _,
			rate_h2d: _,
			rate_d2h: _,
			rate_disk_r: _,
			rate_disk_w: _,
			rate_net_r: _,
			rate_net_w: _,
			net: _,
			_slab: _,
		} = self;
		acts.iter().chain(preacts.iter().flatten()).chain([
			a_q, a_k, a_v, a_ctx, a_dctx, a_dq, a_dk, a_dv, concat, da_a, da_b,
		])
	}

	/// Interrupt check for every sweep. On ^C the read-ahead queues hold
	/// checked-out pool buffers; drain them back so a later sweep (the
	/// checkpoint forward) starts with the full pool.
	fn bail(&self) -> bool {
		if !interrupted() {
			return false;
		}
		for p in self.all_paged() {
			p.drain_ahead(&self.host);
		}
		true
	}

	fn da(&self, flip: bool) -> &Paged {
		if flip { &self.da_b } else { &self.da_a }
	}
	fn da_below(&mut self, flip: bool) -> &mut Paged {
		if flip { &mut self.da_a } else { &mut self.da_b }
	}

	fn attn_backward(&mut self, params: &[LayerParams], l: usize, x: &GpuBuffer, sc: &Scratch, ss: &StepScalars, flip: bool) {
		let p = &params[l];
		let d = p.dim;
		let heads = p.heads;
		let s = p.in_dim / d;
		let n = self.n;
		// Wo: da → dctx, dWo accumulated over windows.
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dw_acc).expect("dw_acc zero");
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if self.bail() {
				return;
			}
			let m = cnt * s;
			let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
			let ctx = self.a_ctx.read(s0, cnt, &self.wins[1], &self.spill, self.n, &self.host);
			let dctx = self.a_dctx.write_view(s0, cnt, &self.wins[2]);
			kernels::gpu_linear_backward_full_into(
				&da, &ctx, &p.wo, &self.reduce_ws, &self.dw_partials, m, d, d,
				&dctx, &self.dw_tmp, &self.db_tmp,
			).expect("attn wo bwd");
			kernels::gpu_add_inplace(&self.dw_tmp, d * d, &self.dw_acc).expect("dw_acc add");
			self.a_dctx.commit(s0, cnt, &dctx, &self.writer, &self.host);
		}
		kernels::gpu_sgd_update(&self.dw_acc, &ss.neg_lr, d * d, &p.wo).expect("sgd wo");
		// Flash backward per sample chunk: dsum, then dQ/dK/dV; un-rotate RoPE.
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if self.bail() {
				return;
			}
			let q = self.a_q.read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host);
			let k = self.a_k.read(s0, cnt, &self.wins[1], &self.spill, self.n, &self.host);
			let v = self.a_v.read(s0, cnt, &self.wins[2], &self.spill, self.n, &self.host);
			let ctx = self.a_ctx.read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
			let dctx = self.a_dctx.read(s0, cnt, &self.wins[4], &self.spill, self.n, &self.host);
			let lse = view(&self.lse, s0 * heads * s * 8, cnt * heads * s * 8);
			let dsum = view(&self.dsum, s0 * heads * s * 8, cnt * heads * s * 8);
			let dq = self.a_dq.write_view(s0, cnt, &self.wins[5]);
			let dk = self.a_dk.write_view(s0, cnt, &self.wins[6]);
			let dv = self.a_dv.write_view(s0, cnt, &self.wins[7]);
			kernels::gpu_flash_attention_backward_into(
				&q, &k, &v, &ctx, &dctx, &lse, cnt, s, d, heads, &dsum, &dq, &dk, &dv,
			)
			.expect("flash attn bwd");
			gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_neg_one, &sc.c_rope_theta, cnt * s, d, heads, s, &dq, &dk).expect("rope bwd");
			self.a_dq.commit(s0, cnt, &dq, &self.writer, &self.host);
			self.a_dk.commit(s0, cnt, &dk, &self.writer, &self.host);
			self.a_dv.commit(s0, cnt, &dv, &self.writer, &self.host);
		}
		// Wq/Wk/Wv: three projections in one sweep per window so the dH sum
		// stays local; per-projection dW accumulators zeroed up front.
		self.da_below(flip).spb = p.in_dim;
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwq_acc).expect("dwq zero");
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwk_acc).expect("dwk zero");
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwv_acc).expect("dwv zero");
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if self.bail() {
				return;
			}
			let m = cnt * s;
			let h = if l == 0 {
				view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8)
			} else {
				self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n, &self.host)
			};
			let below_pg = if flip { &mut self.da_a } else { &mut self.da_b };
			let below = below_pg.write_view(s0, cnt, &self.wins[1]);
			let dh_tmp = view(&self.wins[2], 0, cnt * p.in_dim * 8);
			for (wi, (w, dbuf)) in [(&p.w, &self.a_dq), (&p.wk, &self.a_dk), (&p.wv, &self.a_dv)]
				.into_iter()
				.enumerate()
			{
				let dg = dbuf.read(s0, cnt, &self.wins[3], &self.spill, self.n, &self.host);
				let dst = if wi == 0 { &below } else { &dh_tmp };
				kernels::gpu_linear_backward_full_into(
					&dg, &h, w, &self.reduce_ws, &self.dw_partials, m, d, d,
					dst, &self.dw_tmp, &self.db_tmp,
				).expect("attn wqkv bwd");
				let acc = match wi {
					0 => &self.dwq_acc,
					1 => &self.dwk_acc,
					_ => &self.dwv_acc,
				};
				kernels::gpu_add_inplace(&self.dw_tmp, d * d, acc).expect("acc add");
				if wi > 0 {
					kernels::gpu_add_inplace(&dh_tmp, cnt * p.in_dim, &below).expect("dh add");
				}
			}
			below_pg.commit(s0, cnt, &below, &self.writer, &self.host);
		}
		kernels::gpu_sgd_update(&self.dwq_acc, &ss.neg_lr, d * d, &p.w).expect("sgd wq");
		kernels::gpu_sgd_update(&self.dwk_acc, &ss.neg_lr, d * d, &p.wk).expect("sgd wk");
		kernels::gpu_sgd_update(&self.dwv_acc, &ss.neg_lr, d * d, &p.wv).expect("sgd wv");
	}
}

impl Drop for Ooc {
	fn drop(&mut self) {
		// Drain in-flight write-behind (the spill fd itself dies with us —
		// the path was unlinked at open). The barrier also lands every queued
		// remote STORE before the blobs are freed off the pooled nodes.
		let Ooc { writer, spill, .. } = &mut *self;
		writer.barrier(spill);
		if let Some(net) = self.net.clone() {
			for p in self.all_paged() {
				for h in &p.homes {
					if let Home::Remote { node, id } = h {
						let _ = net[*node].free(*id);
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use gpu_core::kernels;

	fn cpu_bce_grad(p: &[f64], y: &[f64], n_total: usize) -> Vec<f64> {
		let eps = 1e-7;
		p.iter()
			.zip(y)
			.map(|(&p, &y)| {
				let p = p.clamp(eps, 1.0 - eps);
				(p - y) / (p * (1.0 - p)) / n_total as f64
			})
			.collect()
	}

	fn cpu_focal_grad(p: &[f64], y: &[f64], gamma: f64, alpha: f64, n_total: usize) -> Vec<f64> {
		let eps = 1e-12;
		p.iter()
			.zip(y)
			.map(|(&p, &t)| {
				let p = p.clamp(eps, 1.0 - eps);
				let p_t = if t > 0.5 { p } else { 1.0 - p };
				let wt = 1.0 - p_t;
				let sign_pt = if t > 0.5 { 1.0 } else { -1.0 };
				let g = -alpha
					* (gamma * wt.powf(gamma - 1.0) * (-sign_pt) * p_t.ln()
						+ wt.powf(gamma) * sign_pt / p_t);
				g / n_total as f64
			})
			.collect()
	}

	// Bce/Focal grad kernels take the batch-mean scale as an explicit device
	// scalar (inv_n), uniform with the other losses' ss.inv_n riding. The law
	// this pins: per-window calls with the GLOBAL 1/n compose to exactly the
	// full-batch gradient — ragged last window included (7 = 3+3+1) — and both
	// match the analytic CPU reference. The guard at the end pins the contract
	// direction: a per-window 1/cnt scale (the old kernel-internal semantic)
	// must diverge, so a regression to count-derived scaling cannot pass.
	#[test]
	fn ragged_window_bce_focal_grad_matches_full_batch() {
		gpu_core::hip::set_device(0).expect("set_device");
		let n = 7usize;
		let chunk = 3usize; // windows (0,3) (3,3) (6,1) — ragged
		let p = [0.9, 0.2, 0.7, 0.4, 0.6, 0.15, 0.85];
		let y = [1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0];
		let bp = GpuBuffer::upload(&p).expect("p");
		let by = GpuBuffer::upload(&y).expect("y");
		let inv_n = GpuBuffer::upload(&[1.0 / n as f64]).expect("inv_n");
		let (gamma, alpha) = (2.0, 0.25);
		let bgamma = GpuBuffer::upload(&[gamma]).expect("gamma");
		let balpha = GpuBuffer::upload(&[alpha]).expect("alpha");

		// full batch on GPU vs CPU analytic
		let da_full = GpuBuffer::alloc(n).expect("da");
		kernels::gpu_bce_grad_into(&bp, &by, &inv_n, n, &da_full).expect("bce full");
		let mut got = vec![0.0; n];
		da_full.download(&mut got).expect("dl");
		let want = cpu_bce_grad(&p, &y, n);
		for i in 0..n {
			assert!((got[i] - want[i]).abs() < 1e-12, "bce full[{i}] {} vs {}", got[i], want[i]);
		}

		// windowed with the same global inv_n (the exact Ooc::backward sequence)
		let da_win = GpuBuffer::alloc(n).expect("da_win");
		for (s0, cnt) in chunks(n, chunk) {
			let pw = view(&bp, s0 * 8, cnt * 8);
			let yw = view(&by, s0 * 8, cnt * 8);
			let dw = view(&da_win, s0 * 8, cnt * 8);
			kernels::gpu_bce_grad_into(&pw, &yw, &inv_n, cnt, &dw).expect("bce win");
		}
		da_win.download(&mut got).expect("dl");
		for i in 0..n {
			assert!((got[i] - want[i]).abs() < 1e-12, "bce win[{i}] {} vs {}", got[i], want[i]);
		}

		// contract guard: per-window 1/cnt (the old kernel-internal semantic)
		// must NOT reproduce the full-batch gradient.
		let da_raw = GpuBuffer::alloc(n).expect("da_raw");
		for (s0, cnt) in chunks(n, chunk) {
			let inv_cnt = GpuBuffer::upload(&[1.0 / cnt as f64]).expect("inv_cnt");
			let pw = view(&bp, s0 * 8, cnt * 8);
			let yw = view(&by, s0 * 8, cnt * 8);
			let dw = view(&da_raw, s0 * 8, cnt * 8);
			kernels::gpu_bce_grad_into(&pw, &yw, &inv_cnt, cnt, &dw).expect("bce raw win");
		}
		da_raw.download(&mut got).expect("dl");
		let max_dev = got.iter().zip(&want).map(|(g, w)| (g - w).abs()).fold(0.0, f64::max);
		assert!(max_dev > 1e-3, "per-window 1/cnt unexpectedly matched full batch (max dev {max_dev})");

		// focal: same composition law
		let want_f = cpu_focal_grad(&p, &y, gamma, alpha, n);
		let da_f = GpuBuffer::alloc(n).expect("da_f");
		gpu_core::losses::gpu_focal_grad_into(&bp, &by, &bgamma, &balpha, &inv_n, n, &da_f)
			.expect("focal full");
		da_f.download(&mut got).expect("dl");
		for i in 0..n {
			assert!((got[i] - want_f[i]).abs() < 1e-12, "focal full[{i}] {} vs {}", got[i], want_f[i]);
		}
		let da_fw = GpuBuffer::alloc(n).expect("da_fw");
		for (s0, cnt) in chunks(n, chunk) {
			let pw = view(&bp, s0 * 8, cnt * 8);
			let yw = view(&by, s0 * 8, cnt * 8);
			let dw = view(&da_fw, s0 * 8, cnt * 8);
			gpu_core::losses::gpu_focal_grad_into(&pw, &yw, &bgamma, &balpha, &inv_n, cnt, &dw)
				.expect("focal win");
		}
		da_fw.download(&mut got).expect("dl");
		for i in 0..n {
			assert!((got[i] - want_f[i]).abs() < 1e-12, "focal win[{i}] {} vs {}", got[i], want_f[i]);
		}
	}
}

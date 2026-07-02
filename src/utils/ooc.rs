//! Out-of-core training: the waterfall applied to the fit loop. When the
//! full-batch scratch exceeds free VRAM, every big buffer gets a HOME laid out
//! strictly VRAM → RAM → DISK (one spill file), and each op streams
//! sample-aligned windows through a fixed pool of device staging buffers. The
//! math stays full-batch: every sample flows through every op each epoch,
//! weight gradients are tiled reductions over windows (the split-K pattern one
//! level up), and SGD updates once per epoch.

use crate::model::ModelInner;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use recipe_infer::{
	Activation, ELU_ALPHA, LEAKY_ALPHA, LayerKind, LayerParams, Loss, Scratch, download_scalar,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;

thread_local! {
	static HOST: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
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

// Claimable VRAM the way the claim law demands: hipMemGetInfo does not see
// other processes (the desktop), sysfs does; take the min and subtract the
// pool's idle reservations.
fn vram_avail() -> usize {
	let hip_free = gpu_core::hip::mem_info().map(|(f, _)| f).unwrap_or(0);
	let sys_free = gpu_core::hip::sysfs_vram_free().unwrap_or(hip_free);
	let slack = gpu_core::hip::pool_slack(0).unwrap_or(0);
	hip_free.min(sys_free).saturating_sub(slack)
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

fn chunks(n: usize, c: usize) -> impl Iterator<Item = (usize, usize)> {
	(0..n.div_ceil(c)).map(move |i| (i * c, c.min(n - i * c)))
}

enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk(u64), // byte offset in the shared spill file
}

/// One full-batch logical buffer, [n × spb] f64, homed by the waterfall. `spb`
/// (f64s per sample) can change when a ping-pong buffer is reused at another
/// width — bytes are always packed at the current width.
struct Paged {
	home: Home,
	spb: usize,
	// Sequential read-ahead, depth 2: every disk read keeps the next TWO
	// windows in flight (every sweep walks windows in order), so the NVMe
	// sees queue depth and streams underneath the GPU instead of taking
	// turns with it.
	ahead: RefCell<std::collections::VecDeque<(usize, usize, std::thread::JoinHandle<Vec<u8>>)>>,
}

impl Paged {
	fn bytes(&self, samples: usize) -> usize {
		samples * self.spb * 8
	}
	fn kick_ahead(&self, s0: usize, cnt: usize, n: usize, spill: &File) {
		let base = match &self.home {
			Home::Disk(b) => *b,
			_ => return,
		};
		let mut q = self.ahead.borrow_mut();
		let mut next0 = q.back().map_or(s0 + cnt, |(p0, pc, _)| p0 + pc);
		while q.len() < 2 && next0 < n {
			let next_cnt = cnt.min(n - next0);
			let off = base + self.bytes(next0) as u64;
			let len = self.bytes(next_cnt);
			let f = spill.try_clone().expect("spill clone");
			let h = std::thread::spawn(move || {
				let mut buf = vec![0u8; len];
				f.read_exact_at(&mut buf, off).expect("ooc spill read-ahead");
				unsafe {
					use std::os::unix::io::AsRawFd;
					libc::posix_fadvise(f.as_raw_fd(), off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
				}
				buf
			});
			q.push_back((next0, next_cnt, h));
			next0 += next_cnt;
		}
	}
	/// Device view of samples [s0, s0+cnt): VRAM homes return an offset view
	/// (zero copies); RAM/DISK homes stage into `win` first.
	fn read(&self, s0: usize, cnt: usize, win: &GpuBuffer, spill: &File, n: usize) -> GpuBuffer {
		let (off, len) = (self.bytes(s0), self.bytes(cnt));
		match &self.home {
			Home::Vram(b) => view(b, off, len),
			Home::Ram(v) => {
				win.write_u8(&v[off..off + len]).expect("ooc H2D");
				view(win, 0, len)
			}
			Home::Disk(base) => {
				let pre = self.ahead.borrow_mut().pop_front();
				let bytes = match pre {
					Some((p0, pc, h)) if p0 == s0 && pc == cnt => h.join().expect("read-ahead thread"),
					other => {
						// Wrong window prefetched (new sweep) — drop the queue, read sync.
						drop(other.map(|(_, _, h)| h.join()));
						for (_, _, h) in self.ahead.borrow_mut().drain(..) {
							let _ = h.join();
						}
						let mut buf = vec![0u8; len];
						spill.read_exact_at(&mut buf, base + off as u64).expect("ooc spill read");
						unsafe {
							use std::os::unix::io::AsRawFd;
							libc::posix_fadvise(spill.as_raw_fd(), (base + off as u64) as i64, len as i64, libc::POSIX_FADV_DONTNEED);
						}
						buf
					}
				};
				self.kick_ahead(s0, cnt, n, spill);
				win.write_u8(&bytes).expect("ooc H2D");
				view(win, 0, len)
			}
		}
	}
	/// Device window a kernel writes samples [s0, s0+cnt) into. Pair with
	/// `commit(same view)` afterwards — a no-op for VRAM homes.
	fn write_view(&self, s0: usize, cnt: usize, win: &GpuBuffer) -> GpuBuffer {
		let (off, len) = (self.bytes(s0), self.bytes(cnt));
		match &self.home {
			Home::Vram(b) => view(b, off, len),
			_ => view(win, 0, len),
		}
	}
	fn commit(&mut self, s0: usize, cnt: usize, v: &GpuBuffer, writer: &Writer) {
		let (off, len) = (self.bytes(s0), self.bytes(cnt));
		match &mut self.home {
			Home::Vram(_) => {}
			Home::Ram(dst) => v.download_u8(&mut dst[off..off + len]).expect("ooc D2H"),
			Home::Disk(base) => {
				let mut buf = vec![0u8; len];
				v.download_u8(&mut buf).expect("ooc D2H");
				writer.send(*base + off as u64, buf);
			}
		}
	}
}

/// Write-behind: disk commits queue here (bounded, so at most 2 windows of
/// host bytes are in flight) and a worker pwrites + drops the page cache while
/// the GPU moves on to the next window.
/// Write-behind pool: THREE workers, round-robin dispatch, each queue 2 deep
/// — the NVMe sees parallel writes instead of one serial pwrite stream.
struct Writer {
	lanes: Vec<(Option<std::sync::mpsc::SyncSender<(u64, Vec<u8>)>>, Option<std::thread::JoinHandle<()>>)>,
	next: std::cell::Cell<usize>,
}

const W_LANES: usize = 3;

fn spawn_lane(spill: &File) -> (Option<std::sync::mpsc::SyncSender<(u64, Vec<u8>)>>, Option<std::thread::JoinHandle<()>>) {
	let f = spill.try_clone().expect("spill clone");
	let (tx, rx) = std::sync::mpsc::sync_channel::<(u64, Vec<u8>)>(2);
	let worker = std::thread::spawn(move || {
		for (off, buf) in rx {
			f.write_all_at(&buf, off).expect("ooc spill write");
			drop_cache(&f, off, buf.len());
		}
	});
	(Some(tx), Some(worker))
}

impl Writer {
	fn new(spill: &File) -> Writer {
		Writer {
			lanes: (0..W_LANES).map(|_| spawn_lane(spill)).collect(),
			next: std::cell::Cell::new(0),
		}
	}
	fn send(&self, off: u64, buf: Vec<u8>) {
		let i = self.next.get();
		self.next.set((i + 1) % W_LANES);
		self.lanes[i].0.as_ref().expect("writer live").send((off, buf)).expect("writer send");
	}
	/// Drain every lane — REQUIRED before any read of a disk home that was
	/// just written this sweep (the next sweep reads it).
	fn barrier(&mut self, spill: &File) {
		for lane in &mut self.lanes {
			if let Some(tx) = lane.0.take() {
				drop(tx);
			}
			if let Some(w) = lane.1.take() {
				w.join().expect("writer join");
			}
			*lane = spawn_lane(spill);
		}
	}
}

pub struct Plan {
	pub vram: usize,
	pub ram: usize,
	pub disk: usize,
}

/// Combined-ceiling admit in waterfall order from live measurements. `None`
/// only when VRAM+RAM+DISK together cannot hold `need` — the sole abort.
// THE reserve law: exactly 1 GB of each tier belongs to the user; every
// other byte is ours to fill. No ratios, no floors, no guesses.
pub const USER_GB: usize = 1 << 30;

pub fn plan(need: usize) -> Option<Plan> {
	let vram_avail = vram_avail().saturating_sub(USER_GB);
	let ram_avail = mem_available().saturating_sub(USER_GB);
	let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
	let disk_avail = disk_free(&cwd).saturating_sub(USER_GB);
	if need > vram_avail + ram_avail + disk_avail {
		return None;
	}
	let vram = need.min(vram_avail);
	let ram = (need - vram).min(ram_avail);
	Some(Plan { vram, ram, disk: need - vram - ram })
}

/// The out-of-core buffer set + streaming state for one fit.
pub struct Ooc {
	n: usize,
	chunk: usize,
	wins: Vec<GpuBuffer>,
	spill: File,
	writer: Writer,
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
}

impl Ooc {
	/// Lay every full-batch buffer out across the waterfall and size the
	/// streaming windows. Windows are sample-aligned; the widest op (flash
	/// backward) holds 9 at once, so VRAM's window share is carved into 10.
	pub fn build(params: &[LayerParams], n: usize, concat_ac: Option<(usize, usize)>) -> Ooc {
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
		let writer = Writer::new(&spill);

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
		// Budget from the honest availability (min of hip/sysfs minus pool slack)
		// AFTER reserving the fixed residents (lse/dsum/grad accumulators); half
		// of what remains stages windows (+2 windows of margin for the
		// chunk-sized dw_partials/reduce_ws), the waterfall homes fill the rest.
		let fixed_res = (2 * n * hs + 6 * max_wt + 2 * max_bias + 2) * 8;
		let win_budget = vram_avail().saturating_sub(USER_GB).saturating_sub(fixed_res) / 2;
		let chunk = (win_budget / ((WINS + 2) * max_spb * 8)).clamp(1, n);
		let wins: Vec<GpuBuffer> = (0..WINS)
			.map(|_| GpuBuffer::alloc(chunk * max_spb).expect("ooc window"))
			.collect();
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

		// RAM accounting must be CUMULATIVE: fresh Vec pages are lazily
		// zero-backed, so mem_available() does not drop until an epoch touches
		// them — re-measuring per placement admits unbounded virtual memory and
		// the OOM killer collects mid-epoch. The homes budget must also carry
		// THIS path's own host machinery (read-ahead Vecs — up to 5 buffers
		// concurrently read in flash backward — the 2-deep write-behind queue,
		// the staging bounce and a commit temp: 9 windows), or the box lands
		// exactly on the floor and the OOM killer collects (it did, twice).
		let host_overhead = (2 * 5 + 2 * W_LANES + 2) * chunk * max_spb * 8;
		let ram_start = mem_available();
		let ram_floor = USER_GB + host_overhead;
		let mut ram_used = 0usize;
		let mut vram_open = true;
		let mut disk_cursor: u64 = 0;
		let mut place = |spb: usize| -> Paged {
			let bytes = n * spb * 8;
			if vram_open {
				if let Some(b) = GpuBuffer::try_alloc_bytes(bytes) {
					return Paged { home: Home::Vram(b), spb, ahead: RefCell::new(std::collections::VecDeque::new()) };
				}
				vram_open = false;
			}
			if ram_start.saturating_sub(ram_used + bytes) > ram_floor {
				ram_used += bytes;
				return Paged { home: Home::Ram(vec![0u8; bytes]), spb, ahead: RefCell::new(std::collections::VecDeque::new()) };
			}
			let off = disk_cursor;
			disk_cursor += bytes as u64;
			Paged { home: Home::Disk(off), spb, ahead: RefCell::new(std::collections::VecDeque::new()) }
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

		Ooc {
			n,
			chunk,
			wins,
			spill,
			writer,
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
		}
	}

	pub fn report(&self) {
		let gb = |b: usize| b as f64 / (1u64 << 30) as f64;
		let (mut v, mut r, mut d) = (0usize, 0usize, 0usize);
		let mut tally = |p: &Paged| match &p.home {
			Home::Vram(_) => v += self.n * p.spb * 8,
			Home::Ram(x) => r += x.len(),
			Home::Disk(_) => d += self.n * p.spb * 8,
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
			"\x1b[33mwaterfall\x1b[0m  scratch homes: VRAM {:.2} GB → RAM {:.2} GB → DISK {:.2} GB, {}-sample windows",
			gb(v),
			gb(r),
			gb(d),
			self.chunk
		);
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
			let t_l = std::time::Instant::now();
			if let Some((pf, a, c)) = concat_at
				&& l == pf
			{
				self.writer.barrier(&self.spill);
				for (s0, cnt) in chunks(self.n, self.chunk) {
					if interrupted() {
						return;
					}
					let prev = self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n);
					let xc = view(x_cat.expect("x_cat"), s0 * c * 8, cnt * c * 8);
					let out = self.concat.write_view(s0, cnt, &self.wins[1]);
					kernels::gpu_concat_into(&prev, &xc, &out, cnt, a, c);
					self.concat.commit(s0, cnt, &out, &self.writer);
				}
			}
			match p.kind {
				LayerKind::Embed => {
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if interrupted() {
							return;
						}
						let ids = view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8);
						let out = self.acts[l].write_view(s0, cnt, &self.wins[0]);
						kernels::gpu_gather_rows_into(&p.w, &ids, &out, cnt * p.in_dim, p.dim);
						kernels::gpu_broadcast_sub_into(&out, &p.b, &out, cnt * p.out_dim, p.out_dim);
						self.acts[l].commit(s0, cnt, &out, &self.writer);
					}
				}
				LayerKind::Attn => {
					let d = p.dim;
					let heads = p.heads;
					let s = p.in_dim / d;
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if interrupted() {
							return;
						}
						let prev = self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n);
						let m = cnt * s;
						let q = self.a_q.write_view(s0, cnt, &self.wins[1]);
						let k = self.a_k.write_view(s0, cnt, &self.wins[2]);
						let v = self.a_v.write_view(s0, cnt, &self.wins[3]);
						kernels::gpu_linear_into(&prev, &p.w, &p.b, &q, m, d, d);
						kernels::gpu_linear_into(&prev, &p.wk, &p.b, &k, m, d, d);
						kernels::gpu_linear_into(&prev, &p.wv, &p.b, &v, m, d, d);
						gpu_core::rope::gpu_rope_qk_heads_inplace(&q, &k, m, d, heads, s, 1.0);
						self.a_q.commit(s0, cnt, &q, &self.writer);
						self.a_k.commit(s0, cnt, &k, &self.writer);
						self.a_v.commit(s0, cnt, &v, &self.writer);
					}
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if interrupted() {
							return;
						}
						let q = self.a_q.read(s0, cnt, &self.wins[0], &self.spill, self.n);
						let k = self.a_k.read(s0, cnt, &self.wins[1], &self.spill, self.n);
						let v = self.a_v.read(s0, cnt, &self.wins[2], &self.spill, self.n);
						let ctx = self.a_ctx.write_view(s0, cnt, &self.wins[3]);
						let lse = view(&self.lse, s0 * heads * s * 8, cnt * heads * s * 8);
						kernels::gpu_flash_attention_train_into(&q, &k, &v, &ctx, &lse, cnt, s, d, heads);
						self.a_ctx.commit(s0, cnt, &ctx, &self.writer);
					}
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if interrupted() {
							return;
						}
						let ctx = self.a_ctx.read(s0, cnt, &self.wins[0], &self.spill, self.n);
						let out = self.acts[l].write_view(s0, cnt, &self.wins[1]);
						kernels::gpu_linear_into(&ctx, &p.wo, &p.b, &out, cnt * s, d, d);
						self.acts[l].commit(s0, cnt, &out, &self.writer);
					}
				}
				LayerKind::Dense | LayerKind::Conv => {
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(self.n, self.chunk) {
						if interrupted() {
							return;
						}
						let prev = if l == 0 {
							view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8)
						} else if Some(l) == concat_at.map(|t| t.0) {
							self.concat.read(s0, cnt, &self.wins[0], &self.spill, self.n)
						} else {
							self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n)
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
							kernels::gpu_conv1d_into(&prev, &p.w, &p.b, &out, cnt, cin, lin, cout, kk, stride);
						} else if p.out_dim == 1 {
							kernels::gpu_matvec_bias_into(&prev, &p.w, &p.b, &out, cnt, p.in_dim);
						} else {
							kernels::gpu_linear_into(&prev, &p.w, &p.b, &out, cnt, p.out_dim, p.in_dim);
						}
						let m = cnt * p.out_dim;
						if let Some(pa) = self.preacts[l].as_mut() {
							let pre = pa.write_view(s0, cnt, &self.wins[2]);
							kernels::gpu_copy_into(&out, &pre, m);
							pa.commit(s0, cnt, &pre, &self.writer);
							// PRelu/Elu/Selu/Silu/Gelu apply FROM the saved z.
							match p.act {
								Activation::PRelu => {
									let a = download_scalar(&p.palpha);
									kernels::gpu_leaky_relu_into(&pre, &out, m, a);
								}
								Activation::Elu => gpu_core::k_gapact::gpu_elu_into(&pre, &out, m, ELU_ALPHA),
								Activation::Selu => gpu_core::k_gapact::gpu_selu_into(&pre, &out, m),
								Activation::Silu => kernels::gpu_silu_into(&pre, &out, m),
								Activation::Gelu => kernels::gpu_gelu_into(&pre, &out, m),
								_ => unreachable!("preact only saved for z-based activations"),
							}
						} else {
							match p.act {
								Activation::Relu => kernels::gpu_relu_into(&out, &out, m),
								Activation::Sigmoid => kernels::gpu_sigmoid_into(&out, &out, m),
								Activation::LeakyRelu => kernels::gpu_leaky_relu_into(&out, &out, m, LEAKY_ALPHA),
								Activation::Tanh => kernels::gpu_tanh_into(&out, &out, m),
								Activation::Linear => {}
								_ => unreachable!("z-based activation without preact"),
							}
						}
						if l != last {
							self.acts[l].commit(s0, cnt, &out, &self.writer);
						}
					}
				}
			}
			self.sweep_line("fwd", l, match p.kind { LayerKind::Embed => "embed", LayerKind::Attn => "attn", LayerKind::Conv => "conv", LayerKind::Dense => "dense" }, t_l);
		}
	}

	/// Full-batch backward + one SGD update per layer, swept in windows.
	/// Weight grads accumulate across windows into dw_acc/db_acc (tiled
	/// reduction — same math as one pass), then update once.
	#[allow(clippy::too_many_arguments)]
	pub fn backward(
		&mut self,
		params: &[LayerParams],
		x: &GpuBuffer,
		ybuf: &GpuBuffer,
		sc: &Scratch,
		lr: f64,
		loss: Loss,
		concat_at: Option<(usize, usize, usize)>,
	) {
		let last = params.len() - 1;
		let n = self.n;
		// Loss gradient at the output, windowed into da_a (width = out_dim).
		// loss_grad_into scales by 1/rows-it-was-given, so rescale cnt/n to get
		// the global-batch 1/n.
		self.da_a.spb = params[last].out_dim;
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if interrupted() {
				return;
			}
			let k = params[last].out_dim;
			let out = view(&sc.acts[last], s0 * k * 8, cnt * k * 8);
			let y = view(ybuf, s0 * k * 8, cnt * k * 8);
			let da = self.da_a.write_view(s0, cnt, &self.wins[0]);
			ModelInner::loss_grad_into(loss, &out, &y, &da, cnt, cnt * k);
			kernels::gpu_scale_inplace(&da, cnt as f64 / n as f64, cnt * k);
			self.da_a.commit(s0, cnt, &da, &self.writer);
		}
		let mut flip = false;
		for l in (0..params.len()).rev() {
			let p = &params[l];
			let t_l = std::time::Instant::now();
			let (in_dim, out_dim) = (p.in_dim, p.out_dim);
			match p.kind {
				LayerKind::Embed => {
					kernels::gpu_scale_inplace(&sc.embed_grad, 0.0, p.vocab * p.dim);
					self.writer.barrier(&self.spill);
					for (s0, cnt) in chunks(n, self.chunk) {
						if interrupted() {
							return;
						}
						let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n);
						let ids = view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8);
						kernels::gpu_scatter_add(&sc.embed_grad, &ids, &da, cnt * p.in_dim, p.dim);
					}
					kernels::gpu_sgd_update(&p.w, &sc.embed_grad, lr, p.vocab * p.dim);
					self.sweep_line("bwd", l, "embed", t_l);
					flip = !flip;
					continue;
				}
				LayerKind::Attn => {
					self.attn_backward(params, l, x, lr, flip);
					self.sweep_line("bwd", l, "attn", t_l);
					flip = !flip;
					continue;
				}
				LayerKind::Conv => panic!(
					"out-of-core conv backward not implemented — a conv this large has no production caller yet"
				),
				LayerKind::Dense => {}
			}
			// Dense: dz = act'(da) per window; dW/db accumulate; da_below → home.
			kernels::gpu_scale_inplace(&self.dw_acc, 0.0, in_dim * out_dim);
			kernels::gpu_scale_inplace(&self.db_acc, 0.0, out_dim);
			if p.act == Activation::PRelu {
				kernels::gpu_scale_inplace(&self.scalar_acc, 0.0, 1);
			}
			self.da_below(flip).spb = if l > 0 { in_dim } else { 1 };
			self.writer.barrier(&self.spill);
			for (s0, cnt) in chunks(n, self.chunk) {
				if interrupted() {
					return;
				}
				let m = cnt * out_dim;
				let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n);
				let act_l = if l == last {
					view(&sc.acts[last], s0 * out_dim * 8, cnt * out_dim * 8)
				} else {
					self.acts[l].read(s0, cnt, &self.wins[1], &self.spill, self.n)
				};
				let dz = view(&self.wins[2], 0, m * 8);
				let grad: &GpuBuffer = match p.act {
					Activation::Relu => {
						kernels::gpu_relu_backward_into(&da, &act_l, &dz, m);
						&dz
					}
					Activation::Sigmoid => {
						kernels::gpu_sigmoid_backward_into(&da, &act_l, &dz, m);
						&dz
					}
					Activation::LeakyRelu => {
						kernels::gpu_leaky_relu_backward_into(&da, &act_l, &dz, m, LEAKY_ALPHA);
						&dz
					}
					Activation::PRelu => {
						let a = download_scalar(&p.palpha);
						kernels::gpu_leaky_relu_backward_into(&da, &act_l, &dz, m, a);
						let pre = self.preacts[l]
							.as_ref()
							.expect("prelu preact")
							.read(s0, cnt, &self.wins[3], &self.spill, self.n);
						let t0 = view(&self.wins[4], 0, m * 8);
						let t1 = view(&self.wins[5], 0, m * 8);
						kernels::gpu_relu_into(&pre, &t0, m);
						kernels::gpu_copy_into(&pre, &t1, m);
						kernels::gpu_sub_inplace(&t1, &t0, m);
						kernels::gpu_mul_inplace(&t1, &da, m);
						kernels::gpu_reduce_sum_cols_into(&t1, &self.scalar_tmp, &self.reduce_ws, m, 1);
						kernels::gpu_add_inplace(&self.scalar_acc, &self.scalar_tmp, 1);
						&dz
					}
					Activation::Tanh => {
						kernels::gpu_tanh_backward_into(&da, &act_l, &dz, m);
						&dz
					}
					Activation::Elu => {
						let pre = self.preacts[l].as_ref().expect("elu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n);
						gpu_core::k_gapact::gpu_elu_backward_into(&da, &pre, &dz, m, ELU_ALPHA);
						&dz
					}
					Activation::Selu => {
						let pre = self.preacts[l].as_ref().expect("selu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n);
						gpu_core::k_gapact::gpu_selu_backward_into(&da, &pre, &dz, m);
						&dz
					}
					Activation::Silu => {
						let pre = self.preacts[l].as_ref().expect("silu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n);
						kernels::gpu_silu_backward_into(&da, &pre, &dz, m);
						&dz
					}
					Activation::Gelu => {
						let pre = self.preacts[l].as_ref().expect("gelu preact").read(s0, cnt, &self.wins[3], &self.spill, self.n);
						kernels::gpu_gelu_backward_into(&da, &pre, &dz, m);
						&dz
					}
					Activation::Linear => &da,
				};
				let a_prev = if l == 0 {
					view(x, s0 * in_dim * 8, cnt * in_dim * 8)
				} else if Some(l) == concat_at.map(|t| t.0) {
					self.concat.read(s0, cnt, &self.wins[6], &self.spill, self.n)
				} else {
					self.acts[l - 1].read(s0, cnt, &self.wins[6], &self.spill, self.n)
				};
				if l > 0 {
					let below_pg = if flip { &mut self.da_a } else { &mut self.da_b };
					let below = below_pg.write_view(s0, cnt, &self.wins[7]);
					kernels::gpu_linear_backward_full_into(
						grad, &a_prev, &p.w, &below, &self.dw_tmp, &self.db_tmp,
						&self.reduce_ws, &self.dw_partials, cnt, out_dim, in_dim,
					);
					// The layer below the concat only wants the leading A
					// attention columns — compact before committing.
					if let Some((pf, a, c)) = concat_at
						&& l == pf
					{
						let compact = view(&self.wins[8], 0, cnt * a * 8);
						kernels::gpu_slice_lead_into(&below, &compact, cnt, a + c, a);
						below_pg.spb = a;
						below_pg.commit(s0, cnt, &compact, &self.writer);
						below_pg.spb = a + c;
					} else {
						below_pg.commit(s0, cnt, &below, &self.writer);
					}
				} else {
					kernels::gpu_linear_backward_weights_only_into(
						grad, &a_prev, &self.dw_tmp, &self.db_tmp,
						&self.reduce_ws, &self.dw_partials, cnt, out_dim, in_dim,
					);
				}
				kernels::gpu_add_inplace(&self.dw_acc, &self.dw_tmp, in_dim * out_dim);
				kernels::gpu_add_inplace(&self.db_acc, &self.db_tmp, out_dim);
			}
			if let Some((pf, a, _)) = concat_at
				&& l == pf
			{
				self.da_below(flip).spb = a;
			}
			kernels::gpu_sgd_update(&p.w, &self.dw_acc, lr, in_dim * out_dim);
			kernels::gpu_sgd_update(&p.b, &self.db_acc, lr, out_dim);
			if p.act == Activation::PRelu {
				kernels::gpu_sgd_update(&p.palpha, &self.scalar_acc, lr, 1);
			}
			self.sweep_line("bwd", l, "dense", t_l);
			flip = !flip;
		}
	}

	// Persistent per-layer heartbeat — an out-of-core epoch moves ~100 GB
	// through the tiers before the first loss line, and silence looks like a
	// hang. One line per layer pass with wall time, newline-terminated so it
	// survives the dev wrapper own 1 Hz line.
	fn sweep_line(&self, phase: &str, l: usize, kind: &str, t: std::time::Instant) {
		eprintln!(
			"ooc {phase} L{l} {kind}  {} windows  {:.1}s",
			self.n.div_ceil(self.chunk),
			t.elapsed().as_secs_f64()
		);
	}

	fn da(&self, flip: bool) -> &Paged {
		if flip { &self.da_b } else { &self.da_a }
	}
	fn da_below(&mut self, flip: bool) -> &mut Paged {
		if flip { &mut self.da_a } else { &mut self.da_b }
	}

	fn attn_backward(&mut self, params: &[LayerParams], l: usize, x: &GpuBuffer, lr: f64, flip: bool) {
		let p = &params[l];
		let d = p.dim;
		let heads = p.heads;
		let s = p.in_dim / d;
		let n = self.n;
		// Wo: da → dctx, dWo accumulated over windows.
		kernels::gpu_scale_inplace(&self.dw_acc, 0.0, d * d);
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if interrupted() {
				return;
			}
			let m = cnt * s;
			let da = self.da(flip).read(s0, cnt, &self.wins[0], &self.spill, self.n);
			let ctx = self.a_ctx.read(s0, cnt, &self.wins[1], &self.spill, self.n);
			let dctx = self.a_dctx.write_view(s0, cnt, &self.wins[2]);
			kernels::gpu_linear_backward_full_into(
				&da, &ctx, &p.wo, &dctx, &self.dw_tmp, &self.db_tmp,
				&self.reduce_ws, &self.dw_partials, m, d, d,
			);
			kernels::gpu_add_inplace(&self.dw_acc, &self.dw_tmp, d * d);
			self.a_dctx.commit(s0, cnt, &dctx, &self.writer);
		}
		kernels::gpu_sgd_update(&p.wo, &self.dw_acc, lr, d * d);
		// Flash backward per sample chunk: dsum, then dQ/dK/dV; un-rotate RoPE.
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if interrupted() {
				return;
			}
			let q = self.a_q.read(s0, cnt, &self.wins[0], &self.spill, self.n);
			let k = self.a_k.read(s0, cnt, &self.wins[1], &self.spill, self.n);
			let v = self.a_v.read(s0, cnt, &self.wins[2], &self.spill, self.n);
			let ctx = self.a_ctx.read(s0, cnt, &self.wins[3], &self.spill, self.n);
			let dctx = self.a_dctx.read(s0, cnt, &self.wins[4], &self.spill, self.n);
			let lse = view(&self.lse, s0 * heads * s * 8, cnt * heads * s * 8);
			let dsum = view(&self.dsum, s0 * heads * s * 8, cnt * heads * s * 8);
			let dq = self.a_dq.write_view(s0, cnt, &self.wins[5]);
			let dk = self.a_dk.write_view(s0, cnt, &self.wins[6]);
			let dv = self.a_dv.write_view(s0, cnt, &self.wins[7]);
			kernels::gpu_flash_attention_backward_into(
				&q, &k, &v, &ctx, &dctx, &lse, &dsum, &dq, &dk, &dv, cnt, s, d, heads,
			);
			gpu_core::rope::gpu_rope_qk_heads_inplace(&dq, &dk, cnt * s, d, heads, s, -1.0);
			self.a_dq.commit(s0, cnt, &dq, &self.writer);
			self.a_dk.commit(s0, cnt, &dk, &self.writer);
			self.a_dv.commit(s0, cnt, &dv, &self.writer);
		}
		// Wq/Wk/Wv: three projections in one sweep per window so the dH sum
		// stays local; per-projection dW accumulators zeroed up front.
		self.da_below(flip).spb = p.in_dim;
		kernels::gpu_scale_inplace(&self.dwq_acc, 0.0, d * d);
		kernels::gpu_scale_inplace(&self.dwk_acc, 0.0, d * d);
		kernels::gpu_scale_inplace(&self.dwv_acc, 0.0, d * d);
		self.writer.barrier(&self.spill);
		for (s0, cnt) in chunks(n, self.chunk) {
			if interrupted() {
				return;
			}
			let m = cnt * s;
			let h = if l == 0 {
				view(x, s0 * p.in_dim * 8, cnt * p.in_dim * 8)
			} else {
				self.acts[l - 1].read(s0, cnt, &self.wins[0], &self.spill, self.n)
			};
			let below_pg = if flip { &mut self.da_a } else { &mut self.da_b };
			let below = below_pg.write_view(s0, cnt, &self.wins[1]);
			let dh_tmp = view(&self.wins[2], 0, cnt * p.in_dim * 8);
			for (wi, (w, dbuf)) in [(&p.w, &self.a_dq), (&p.wk, &self.a_dk), (&p.wv, &self.a_dv)]
				.into_iter()
				.enumerate()
			{
				let dg = dbuf.read(s0, cnt, &self.wins[3], &self.spill, self.n);
				let dst = if wi == 0 { &below } else { &dh_tmp };
				kernels::gpu_linear_backward_full_into(
					&dg, &h, w, dst, &self.dw_tmp, &self.db_tmp,
					&self.reduce_ws, &self.dw_partials, m, d, d,
				);
				let acc = match wi {
					0 => &self.dwq_acc,
					1 => &self.dwk_acc,
					_ => &self.dwv_acc,
				};
				kernels::gpu_add_inplace(acc, &self.dw_tmp, d * d);
				if wi > 0 {
					kernels::gpu_add_inplace(&below, &dh_tmp, cnt * p.in_dim);
				}
			}
			below_pg.commit(s0, cnt, &below, &self.writer);
		}
		kernels::gpu_sgd_update(&p.w, &self.dwq_acc, lr, d * d);
		kernels::gpu_sgd_update(&p.wk, &self.dwk_acc, lr, d * d);
		kernels::gpu_sgd_update(&p.wv, &self.dwv_acc, lr, d * d);
	}
}

impl Drop for Ooc {
	fn drop(&mut self) {
		// Drain in-flight write-behind (the spill fd itself dies with us —
		// the path was unlinked at open).
		self.writer.barrier(&self.spill);
	}
}

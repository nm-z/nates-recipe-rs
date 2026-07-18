use crate::train::StepScalars;
use anyhow::Context;
use gpu_core::kernels;
use gpu_core::log::{Write, gpu};
use gpu_core::memory::GpuBuffer;
use recipe_infer::{Activation, LayerKind, LayerParams, Loss, Scratch};
use std::cell::Cell;
use std::cell::RefCell;
use std::cmp;
use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

static DISK_R_BYTES: AtomicUsize = AtomicUsize::new(0);
static DISK_W_BYTES: AtomicUsize = AtomicUsize::new(0);
static NET_R_BYTES: AtomicUsize = AtomicUsize::new(0);
static NET_W_BYTES: AtomicUsize = AtomicUsize::new(0);

type HostPool = Arc<Mutex<Vec<Vec<u8>>>>;

pub struct Window {
	pub s0: usize,
	pub cnt: usize,
}

#[derive(Clone, Copy)]
pub struct ConcatAc {
	pub a: usize,
	pub c: usize,
}

#[derive(Clone, Copy)]
pub struct ConcatFit {
	pub pf: usize,
	pub a: usize,
	pub c: usize,
}

struct Xfer {
	h2d: usize,
	d2h: usize,
}

fn xfer() -> Xfer {
	let b = gpu_core::memory::xfer_bytes();
	Xfer {
		h2d: b.h2d,
		d2h: b.d2h,
	}
}

fn pool_take(p: &HostPool) -> anyhow::Result<Vec<u8>> {
	p.lock()
		.map_err(|_e| anyhow::anyhow!("host pool lock"))?
		.pop()
		.ok_or_else(|| {
			anyhow::anyhow!("host pool exhausted — transient window count exceeded POOL_BUFS")
		})
}

fn pool_give(p: &HostPool, v: Vec<u8>) -> anyhow::Result<()> {
	p.lock()
		.map_err(|_e| anyhow::anyhow!("host pool lock"))?
		.push(v);
	Ok(())
}

fn mem_available() -> usize {
	fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(0, |kb| kb.saturating_mul(1024))
}

fn disk_free(path: &Path) -> usize {
	return gpu_core::sys::disk_free_bytes(path);
}

pub fn view(b: &GpuBuffer, byte_off: usize, byte_len: usize) -> GpuBuffer {
	return b.sub_view(byte_off, byte_len);
}

fn drop_cache(f: &File, off: u64, len: usize) {
	gpu_core::sys::evict_range(f, off, len);
}

enum Interrupt {
	Yes,
	No,
}

enum Flow {
	Go,
	Halt,
}

#[derive(Clone, Copy)]
enum Gate {
	Open,
	Closed,
}

#[derive(Clone, Copy)]
enum Flip {
	A,
	B,
}

impl Flip {
	fn toggle(self) -> Flip {
		match self {
			Flip::A => Flip::B,
			Flip::B => Flip::A,
		}
	}
}

fn prelu_gate(act: &Activation) -> Option<()> {
	match act {
		Activation::PRelu => Some(()),
		_other => None,
	}
}

fn interrupted() -> Interrupt {
	match crate::train::INTERRUPTED.load(Ordering::SeqCst).cmp(&0) {
		cmp::Ordering::Equal => Interrupt::No,
		cmp::Ordering::Less | cmp::Ordering::Greater => Interrupt::Yes,
	}
}

struct BwdWin {
	da: GpuBuffer,
	act_l: GpuBuffer,
	dz: GpuBuffer,
	m: usize,
}

pub fn chunks(n: usize, c: usize) -> impl Iterator<Item = Window> {
	(0..n.div_ceil(c)).map(move |i| Window {
		s0: i * c,
		cnt: c.min(n - i * c),
	})
}

fn open_spill() -> anyhow::Result<File> {
	let path = crate::machine::data_dir()
		.context("spill dir")?
		.join(".recipe_spill");
	let f = recipe_infer::bridge::open_rw(&path).context("open spill file")?;
	drop(fs::remove_file(&path));
	Ok(f)
}

enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk(u64),
	Remote { node: usize, id: u64 },
}

struct Ahead {
	s0: usize,
	cnt: usize,
	handle: thread::JoinHandle<anyhow::Result<Vec<u8>>>,
}

struct Paged {
	homes: Vec<Home>,
	spb: usize,
	chunk: usize,
	ahead: RefCell<VecDeque<Ahead>>,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
}

impl Paged {
	fn win(&self, s0: usize, cnt: usize) -> anyhow::Result<usize> {
		anyhow::ensure!(
			s0.is_multiple_of(self.chunk) && cnt <= self.chunk,
			"ooc access not window-aligned"
		);
		Ok(s0 / self.chunk)
	}
	fn fresh_disk_read(
		&self,
		host: &HostPool,
		f: &File,
		off: u64,
		len: usize,
	) -> anyhow::Result<Vec<u8>> {
		self.drain_ahead(host)?;
		let mut buf = pool_take(host)?;
		f.read_exact_at(&mut buf[..len], off)
			.context("ooc spill read")?;
		DISK_R_BYTES.fetch_add(len, Ordering::Relaxed);
		gpu_core::sys::advise_dontneed(f, off, len);
		Ok(buf)
	}
	fn fresh_net_read(
		&self,
		host: &HostPool,
		node: usize,
		id: u64,
		len: usize,
	) -> anyhow::Result<Vec<u8>> {
		self.drain_ahead(host)?;
		let mut buf = pool_take(host)?;
		let nt = self
			.net
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("remote home without net"))?;
		let v = nt[node].fetch(id, 0, len as u64).context("ooc net read")?;
		buf[..len].copy_from_slice(&v);
		NET_R_BYTES.fetch_add(len, Ordering::Relaxed);
		Ok(buf)
	}
	fn kick_ahead(
		&self,
		s0: usize,
		cnt: usize,
		n: usize,
		spill: Option<&File>,
		host: &HostPool,
	) -> anyhow::Result<()> {
		let mut q = self.ahead.borrow_mut();
		let mut next0 = q.back().map_or(s0 + cnt, |a| a.s0 + a.cnt);
		while q.len() < AHEAD && next0 < n {
			let next_cnt = cnt.min(n - next0);
			let len = next_cnt * self.spb * size_of::<f64>();
			let h = match &self.homes[next0 / self.chunk] {
				Home::Disk(off) => {
					let off = *off;
					let f = spill
						.ok_or_else(|| {
							anyhow::anyhow!("disk read-ahead without spill file")
						})?
						.try_clone()
						.context("spill clone")?;
					let hp = host.clone();
					thread::spawn(move || -> anyhow::Result<Vec<u8>> {
						let mut buf = pool_take(&hp)?;
						f.read_exact_at(&mut buf[..len], off)
							.context("ooc spill read-ahead")?;
						DISK_R_BYTES.fetch_add(len, Ordering::Relaxed);
						gpu_core::sys::advise_dontneed(&f, off, len);
						Ok(buf)
					})
				}
				Home::Remote { node, id } => {
					let node = *node;
					let id = *id;
					let nt =
						Arc::clone(self.net.as_ref().ok_or_else(|| {
							anyhow::anyhow!("remote home without net")
						})?);
					let hp = host.clone();
					thread::spawn(move || -> anyhow::Result<Vec<u8>> {
						let mut buf = pool_take(&hp)?;
						let v = nt[node]
							.fetch(id, 0, len as u64)
							.context("ooc net read-ahead")?;
						buf[..len].copy_from_slice(&v);
						NET_R_BYTES.fetch_add(len, Ordering::Relaxed);
						Ok(buf)
					})
				}
				_resident => return Ok(()),
			};
			q.push_back(Ahead {
				s0: next0,
				cnt: next_cnt,
				handle: h,
			});
			next0 += next_cnt;
		}
		Ok(())
	}
	fn drain_ahead(&self, host: &HostPool) -> anyhow::Result<()> {
		for a in self.ahead.borrow_mut().drain(..) {
			let r = a
				.handle
				.join()
				.map_err(|_e| anyhow::anyhow!("read-ahead thread"))?;
			pool_give(host, r?)?;
		}
		Ok(())
	}
	fn read(
		&self,
		s0: usize,
		cnt: usize,
		win: &GpuBuffer,
		spill: Option<&File>,
		n: usize,
		host: &HostPool,
	) -> anyhow::Result<GpuBuffer> {
		let len = cnt * self.spb * size_of::<f64>();
		Ok(match &self.homes[self.win(s0, cnt)?] {
			Home::Vram(b) => view(b, 0, len),
			Home::Ram(v) => {
				win.write_u8(&v[..len]).context("ooc H2D")?;
				view(win, 0, len)
			}
			Home::Disk(off) => {
				let off = *off;
				let f =
					spill.ok_or_else(|| anyhow::anyhow!("disk read without spill file"))?;
				let pre = self.ahead.borrow_mut().pop_front();
				let bytes = match pre {
					Some(a) => match [a.s0, a.cnt].cmp(&[s0, cnt]) {
						cmp::Ordering::Equal => a
							.handle
							.join()
							.map_err(|_e| anyhow::anyhow!("read-ahead thread"))??,
						_mismatch => {
							let r = a.handle.join().map_err(|_e| {
								anyhow::anyhow!("read-ahead thread")
							})?;
							pool_give(host, r?)?;
							self.fresh_disk_read(host, f, off, len)?
						}
					},
					None => self.fresh_disk_read(host, f, off, len)?,
				};
				self.kick_ahead(s0, cnt, n, spill, host)?;
				win.write_u8(&bytes[..len]).context("ooc H2D")?;
				pool_give(host, bytes)?;
				view(win, 0, len)
			}
			Home::Remote { node, id } => {
				let node = *node;
				let id = *id;
				let pre = self.ahead.borrow_mut().pop_front();
				let bytes = match pre {
					Some(a) => match [a.s0, a.cnt].cmp(&[s0, cnt]) {
						cmp::Ordering::Equal => a
							.handle
							.join()
							.map_err(|_e| anyhow::anyhow!("read-ahead thread"))??,
						_mismatch => {
							let r = a.handle.join().map_err(|_e| {
								anyhow::anyhow!("read-ahead thread")
							})?;
							pool_give(host, r?)?;
							self.fresh_net_read(host, node, id, len)?
						}
					},
					None => self.fresh_net_read(host, node, id, len)?,
				};
				self.kick_ahead(s0, cnt, n, spill, host)?;
				win.write_u8(&bytes[..len]).context("ooc H2D")?;
				pool_give(host, bytes)?;
				view(win, 0, len)
			}
		})
	}
	fn write_view(&self, s0: usize, cnt: usize, win: &GpuBuffer) -> anyhow::Result<GpuBuffer> {
		let len = cnt * self.spb * size_of::<f64>();
		Ok(match &self.homes[self.win(s0, cnt)?] {
			Home::Vram(b) => view(b, 0, len),
			_spilled => view(win, 0, len),
		})
	}
	fn commit(
		&mut self,
		s0: usize,
		cnt: usize,
		v: &GpuBuffer,
		writer: &Writer,
		host: &HostPool,
	) -> anyhow::Result<()> {
		let len = cnt * self.spb * size_of::<f64>();
		let w = self.win(s0, cnt)?;
		match &mut self.homes[w] {
			Home::Vram(_buf) => Ok(()),
			Home::Ram(dst) => v.download_u8(&mut dst[..len]).context("ooc D2H"),
			Home::Disk(off) => {
				let mut buf = pool_take(host)?;
				v.download_u8(&mut buf[..len]).context("ooc D2H")?;
				writer.send(Dest::Disk(*off), buf, len)
			}
			Home::Remote { node, id } => {
				let mut buf = pool_take(host)?;
				v.download_u8(&mut buf[..len]).context("ooc D2H")?;
				writer.send(
					Dest::Remote {
						node: *node,
						id: *id,
					},
					buf,
					len,
				)
			}
		}
	}
}

#[derive(Clone, Copy)]
enum Dest {
	Disk(u64),
	Remote { node: usize, id: u64 },
}

struct WriteMsg {
	dest: Dest,
	buf: Vec<u8>,
	len: usize,
}

struct Lane {
	tx: Option<mpsc::SyncSender<WriteMsg>>,
	worker: Option<thread::JoinHandle<anyhow::Result<()>>>,
}

fn make_chan(depth: usize) -> gpu_core::bridge::Chan<WriteMsg> {
	gpu_core::bridge::sync_chan::<WriteMsg>(depth)
}

struct Writer {
	lanes: Vec<Lane>,
	next: Cell<usize>,
	host: HostPool,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
	pending: Arc<AtomicUsize>,
	drained: Cell<f64>,
}

const W_LANES: usize = 3;
const WQ_DEPTH: usize = 2;
const AHEAD: usize = 2;
const MAX_READERS: usize = 5;
const POOL_BUFS: usize = MAX_READERS * AHEAD + 1 + W_LANES * (WQ_DEPTH + 1) + 1;

fn spawn_lane(
	spill: Option<&File>,
	host: &HostPool,
	net: &Option<Arc<Vec<crate::wire::Conn>>>,
	pending: &Arc<AtomicUsize>,
) -> anyhow::Result<Lane> {
	let f: Option<File> = match spill {
		Some(s) => Some(s.try_clone().context("spill clone")?),
		None => None,
	};
	let hp = host.clone();
	let nt = net.clone();
	let pend = pending.clone();
	let chan = make_chan(WQ_DEPTH);
	let worker = thread::spawn(move || -> anyhow::Result<()> {
		for msg in chan.rx {
			let WriteMsg { dest, buf, len } = msg;
			match dest {
				Dest::Disk(off) => {
					let f = f.as_ref().ok_or_else(|| {
						anyhow::anyhow!("disk dest without spill file")
					})?;
					f.write_all_at(&buf[..len], off)
						.context("ooc spill write")?;
					DISK_W_BYTES.fetch_add(len, Ordering::Relaxed);
					drop_cache(f, off, len);
				}
				Dest::Remote { node, id } => {
					let nt = nt
						.as_ref()
						.ok_or_else(|| anyhow::anyhow!("remote dest without net"))?;
					nt[node]
						.store_from(id, &buf[..len])
						.context("ooc net write")?;
					NET_W_BYTES.fetch_add(len, Ordering::Relaxed);
				}
			}
			pool_give(&hp, buf)?;
			pend.fetch_sub(1, Ordering::Relaxed);
		}
		Ok(())
	});
	Ok(Lane {
		tx: Some(chan.tx),
		worker: Some(worker),
	})
}

impl Writer {
	fn new(
		spill: Option<&File>,
		host: HostPool,
		net: Option<Arc<Vec<crate::wire::Conn>>>,
	) -> anyhow::Result<Writer> {
		let pending = Arc::new(AtomicUsize::new(0));
		Ok(Writer {
			lanes: (0..W_LANES)
				.map(|_lane| spawn_lane(spill, &host, &net, &pending))
				.collect::<anyhow::Result<Vec<Lane>>>()?,
			next: Cell::new(0),
			host,
			net,
			pending,
			drained: Cell::new(0.0),
		})
	}
	fn send(&self, dest: Dest, buf: Vec<u8>, len: usize) -> anyhow::Result<()> {
		self.pending.fetch_add(1, Ordering::Relaxed);
		let i = self.next.get();
		self.next.set((i + 1) % W_LANES);
		self.lanes[i]
			.tx
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("writer live"))?
			.send(WriteMsg { dest, buf, len })
			.map_err(|_e| anyhow::anyhow!("writer send"))?;
		Ok(())
	}
	fn barrier(&mut self, spill: Option<&File>) -> anyhow::Result<()> {
		while self.pending.load(Ordering::Relaxed) > 0 {
			let t = Instant::now();
			for lane in &mut self.lanes {
				drop(lane.tx.take());
				lane.worker
					.take()
					.map(
						|w| {
							w.join()
								.map_err(|_join_err| anyhow::anyhow!("writer join"))
						},
					)
					.transpose()?
					.transpose()?;
				*lane = spawn_lane(spill, &self.host, &self.net, &self.pending)?;
			}
			self.drained
				.set(self.drained.get() + t.elapsed().as_secs_f64());
		}
		Ok(())
	}
}

pub struct Plan {
	pub vram: usize,
	pub ram: usize,
	pub disk: usize,
	pub remote: usize,
}

pub use gpu_core::memory::USER_GB;

pub fn plan(need: usize, net_ram: usize) -> Option<Plan> {
	let vram_avail = gpu_core::memory::claimable_bytes();
	let ram_avail = mem_available().saturating_sub(USER_GB);
	let dir = match crate::machine::data_dir() {
		Ok(v) => v,
		Err(e) => {
			Write::error(format!("data_dir: {e:#}"));
			return None;
		}
	};
	let disk_avail = disk_free(&dir).saturating_sub(USER_GB);
	let vram = need.min(vram_avail);
	let ram = (need - vram).min(ram_avail);
	let disk = (need - vram - ram).min(disk_avail);
	(vram_avail + ram_avail + disk_avail + net_ram)
		.checked_sub(need)
		.map(|_slack| Plan {
			vram,
			ram,
			disk,
			remote: need - vram - ram - disk,
		})
}

pub struct Ooc {
	n: usize,
	chunk: usize,
	wins: Vec<GpuBuffer>,
	spill: Option<File>,
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
	conv_temp: GpuBuffer,
	conv_wg: usize,
	rate_h2d: f64,
	rate_d2h: f64,
	rate_disk_r: f64,
	rate_disk_w: f64,
	rate_net_r: f64,
	rate_net_w: f64,
	net: Option<Arc<Vec<crate::wire::Conn>>>,
}

struct SweepStart {
	t: Instant,
	h2d: usize,
	d2h: usize,
	disk_r: usize,
	disk_w: usize,
	net_r: usize,
	net_w: usize,
}

struct Rate {
	label: &'static str,
	bps: f64,
}

struct Stream {
	label: &'static str,
	delta: usize,
	rate: f64,
}

fn sweep_start() -> SweepStart {
	let x = xfer();
	SweepStart {
		t: Instant::now(),
		h2d: x.h2d,
		d2h: x.d2h,
		disk_r: DISK_R_BYTES.load(Ordering::Relaxed),
		disk_w: DISK_W_BYTES.load(Ordering::Relaxed),
		net_r: NET_R_BYTES.load(Ordering::Relaxed),
		net_w: NET_W_BYTES.load(Ordering::Relaxed),
	}
}

impl Ooc {
	pub fn min_bytes(
		dims: &[recipe_infer::LayerDims],
		n: usize,
		concat_ac: Option<ConcatAc>,
	) -> usize {
		let attn = dims.iter().find(|p| p.kind == LayerKind::Attn);
		let hs = attn.map_or(1, |p| p.heads * (p.in_dim / p.dim));
		let max_act_spb = dims
			.iter()
			.map(|p| p.out_dim.max(p.in_dim))
			.max()
			.unwrap_or(1);
		let seq_spb = attn.map_or(1, |p| p.in_dim);
		let ConcatAc { a: ca, c: cc } = concat_ac.unwrap_or(ConcatAc { a: 0, c: 0 });
		let max_spb = seq_spb.max(max_act_spb).max(ca + cc);
		let max_wt = dims
			.iter()
			.map(|p| match p.kind {
				LayerKind::Dense => p.in_dim * p.out_dim,
				LayerKind::Attn => p.dim * p.dim,
				LayerKind::Embed => p.vocab * p.dim,
				LayerKind::Conv => {
					let lout = (p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1;
					(p.out_dim / lout.max(1)) * p.conv_cin * p.conv_k
				}
			})
			.max()
			.unwrap_or(1);
		let max_bias = dims.iter().map(|p| p.out_dim).max().unwrap_or(1);
		let max_conv_fsz = dims
			.iter()
			.filter(|p| p.kind == LayerKind::Conv)
			.map(|p| {
				let lout = (p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1;
				(p.out_dim / lout.max(1)) * p.conv_cin * p.conv_k
			})
			.max()
			.unwrap_or(0);
		const WINS: usize = 10;
		let chunk = 1usize;
		let conv_wg = match max_conv_fsz.cmp(&0) {
			cmp::Ordering::Greater => chunk,
			cmp::Ordering::Less | cmp::Ordering::Equal => 0,
		};
		let seq_rows = attn.map_or(chunk, |p| chunk * (p.in_dim / p.dim));
		let mut dwp = 1usize;
		let mut ws = kernels::gpu_reduce_sum_cols_workspace_bytes(chunk, 1);
		for p in dims {
			let e = match p.kind {
				LayerKind::Dense => {
					kernels::gpu_splitk_dw_partials_elems(chunk, p.in_dim, p.out_dim)
				}
				LayerKind::Attn => {
					kernels::gpu_splitk_dw_partials_elems(seq_rows, p.dim, p.dim)
				}
				LayerKind::Embed | LayerKind::Conv => 0,
			};
			dwp = dwp.max(e);
			let rows = match p.kind {
				LayerKind::Attn => seq_rows,
				LayerKind::Dense | LayerKind::Embed | LayerKind::Conv => chunk,
			};
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(
				rows,
				p.out_dim.max(p.dim),
			));
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(
				rows * p.out_dim.max(p.dim),
				1,
			));
		}
		ws = ws.max(match max_conv_fsz.cmp(&0) {
			cmp::Ordering::Greater => {
				kernels::gpu_reduce_sum_cols_workspace_bytes(conv_wg, max_conv_fsz)
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => 0,
		});
		let align256 = |b: usize| (b + 255) & !255;
		WINS * align256(chunk * max_spb * size_of::<f64>())
			+ 2 * align256(n * hs * size_of::<f64>())
			+ 5 * align256(max_wt * size_of::<f64>())
			+ 2 * align256(max_bias * size_of::<f64>())
			+ 2 * align256(8)
			+ align256(dwp * size_of::<f64>())
			+ align256(ws) + align256(((conv_wg * max_conv_fsz).max(1)) * size_of::<f64>())
			+ (1 << 20)
	}

	pub fn build(
		params: &[LayerParams],
		n: usize,
		concat_ac: Option<ConcatAc>,
		net: Option<Arc<Vec<crate::wire::Conn>>>,
	) -> anyhow::Result<Ooc> {
		let attn = params.iter().find(|p| p.kind == LayerKind::Attn);
		let seq_spb = attn.map_or(1, |p| p.in_dim);
		let hs = attn.map_or(1, |p| p.heads * (p.in_dim / p.dim));
		let max_act_spb = params
			.iter()
			.map(|p| p.out_dim.max(p.in_dim))
			.max()
			.unwrap_or(1);
		let ConcatAc { a: ca, c: cc } = concat_ac.unwrap_or(ConcatAc { a: 0, c: 0 });
		let max_spb = seq_spb.max(max_act_spb).max(ca + cc);

		if !gpu_core::memory::device_arena_active() {
			Write::err("ooc: build requires the run's claimed arena")?;
		}

		let max_wt = params
			.iter()
			.map(|p| match p.kind {
				LayerKind::Dense => p.in_dim * p.out_dim,
				LayerKind::Attn => p.dim * p.dim,
				LayerKind::Embed => p.vocab * p.dim,
				LayerKind::Conv => {
					let lout = (p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1;
					(p.out_dim / lout.max(1)) * p.conv_cin * p.conv_k
				}
			})
			.max()
			.unwrap_or(1);
		let max_bias = params.iter().map(|p| p.out_dim).max().unwrap_or(1);
		let max_conv_fsz = params
			.iter()
			.filter(|p| p.kind == LayerKind::Conv)
			.map(|p| {
				let lout = (p.in_dim / p.conv_cin - p.conv_k) / p.conv_stride + 1;
				(p.out_dim / lout.max(1)) * p.conv_cin * p.conv_k
			})
			.max()
			.unwrap_or(0);
		const WINS: usize = 10;
		let fixed_res = (2 * n * hs + 6 * max_wt + 2 * max_bias + 2) * size_of::<f64>();
		let win_budget = gpu_core::memory::arena_remaining().saturating_sub(fixed_res) / 2;
		let chunk = (win_budget / (((WINS + 2) * max_spb + max_conv_fsz) * size_of::<f64>())).clamp(1, n);
		let wbytes = chunk * max_spb * size_of::<f64>();
		let conv_wg = match max_conv_fsz.cmp(&0) {
			cmp::Ordering::Greater => chunk,
			cmp::Ordering::Less | cmp::Ordering::Equal => 0,
		};
		let seq_rows = attn.map_or(chunk, |p| chunk * (p.in_dim / p.dim));
		let mut dwp = 1usize;
		let mut ws = kernels::gpu_reduce_sum_cols_workspace_bytes(chunk, 1);
		for p in params {
			let e = match p.kind {
				LayerKind::Dense => {
					kernels::gpu_splitk_dw_partials_elems(chunk, p.in_dim, p.out_dim)
				}
				LayerKind::Attn => {
					kernels::gpu_splitk_dw_partials_elems(seq_rows, p.dim, p.dim)
				}
				LayerKind::Embed | LayerKind::Conv => 0,
			};
			dwp = dwp.max(e);
			let rows = match p.kind {
				LayerKind::Attn => seq_rows,
				LayerKind::Dense | LayerKind::Embed | LayerKind::Conv => chunk,
			};
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(
				rows,
				p.out_dim.max(p.dim),
			));
			ws = ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(
				rows * p.out_dim.max(p.dim),
				1,
			));
		}
		ws = ws.max(match max_conv_fsz.cmp(&0) {
			cmp::Ordering::Greater => {
				kernels::gpu_reduce_sum_cols_workspace_bytes(conv_wg, max_conv_fsz)
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => 0,
		});

		let wins: Vec<GpuBuffer> = (0..WINS)
			.map(|_slot| GpuBuffer::alloc(chunk * max_spb).context("ooc window"))
			.collect::<anyhow::Result<Vec<GpuBuffer>>>()?;

		let lse = GpuBuffer::alloc(n * hs).context("ooc lse")?;
		let dsum = GpuBuffer::alloc(n * hs).context("ooc dsum")?;
		let dw_acc = GpuBuffer::alloc(max_wt).context("ooc dw_acc")?;
		let db_acc = GpuBuffer::alloc(max_bias).context("ooc db_acc")?;
		let dw_tmp = GpuBuffer::alloc(max_wt).context("ooc dw_tmp")?;
		let db_tmp = GpuBuffer::alloc(max_bias).context("ooc db_tmp")?;
		let scalar_acc = GpuBuffer::alloc(1).context("ooc scalar_acc")?;
		let scalar_tmp = GpuBuffer::alloc(1).context("ooc scalar_tmp")?;
		let dwq_acc = GpuBuffer::alloc(max_wt).context("ooc dwq_acc")?;
		let dwk_acc = GpuBuffer::alloc(max_wt).context("ooc dwk_acc")?;
		let dwv_acc = GpuBuffer::alloc(max_wt).context("ooc dwv_acc")?;
		let dw_partials = GpuBuffer::alloc(dwp).context("ooc dw_partials")?;
		let reduce_ws = GpuBuffer::alloc_bytes(ws).context("ooc reduce_ws")?;
		let conv_temp =
			GpuBuffer::alloc((conv_wg * max_conv_fsz).max(1)).context("ooc conv_temp")?;

		let win_region_bytes = gpu_core::memory::arena_remaining();
		let seal =
			GpuBuffer::alloc_bytes(win_region_bytes).context("ooc window-region seal")?;
		let slab_bytes = win_region_bytes;
		let mut slab_off = 0usize;

		let ram_start = mem_available().saturating_sub(POOL_BUFS * wbytes);
		let ram_floor = USER_GB;
		let mut ram_used = 0usize;
		let mut vram_gate = Gate::Open;
		let mut disk_cursor: u64 = 0;
		let mut spill: Option<File> = None;
		let mut nonvram = 0usize;
		let disk_budget = disk_free(&crate::machine::data_dir()?).saturating_sub(USER_GB);
		let net_caps: Vec<usize> = net.as_ref().map_or(Vec::new(), |ns| {
			ns.iter()
				.map(|c| (c.info.ram as usize).saturating_sub(USER_GB))
				.collect()
		});
		let mut net_used = vec![0usize; net_caps.len()];
		let id_base: u64 = {
			let host_s = fs::read_to_string("/proc/sys/kernel/hostname").unwrap_or_default();
			let mut h = 0xcbf2_9ce4_8422_2325u64;
			for b in host_s.trim().bytes().chain(process::id().to_le_bytes()) {
				h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
			}
			h << 32
		};
		let mut next_id: u64 = 0;
		let mut place = |spb: usize| -> anyhow::Result<Paged> {
			let n_wins = n.div_ceil(chunk);
			let mut homes = Vec::with_capacity(n_wins);
			for w in 0..n_wins {
				let cnt = chunk.min(n - w * chunk);
				let bytes = cnt * spb * size_of::<f64>();
				let aligned = (bytes + 4095) & !4095;
				let vram_room = match vram_gate {
					Gate::Open => slab_bytes.checked_sub(slab_off + aligned),
					Gate::Closed => None,
				};
				let home = match vram_room {
					Some(_room) => {
						let h = Home::Vram(view(&seal, slab_off, bytes));
						slab_off += aligned;
						h
					}
					None => {
						vram_gate = Gate::Closed;
						nonvram += 1;
						match ram_start.saturating_sub(ram_used + bytes).cmp(&ram_floor)
						{
							cmp::Ordering::Greater => {
								ram_used += bytes;
								let mut v = vec![0u8; bytes];
								gpu_core::memory::par_touch(&mut v);
								Home::Ram(v)
							}
							cmp::Ordering::Less | cmp::Ordering::Equal => {
								match (disk_cursor as usize + bytes)
									.cmp(&disk_budget)
								{
									cmp::Ordering::Less
									| cmp::Ordering::Equal => {
										spill = Some(match spill.take() {
											Some(existing) => existing,
											None => open_spill()?,
										});
										let h = Home::Disk(disk_cursor);
										disk_cursor += bytes as u64;
										h
									}
									cmp::Ordering::Greater => {
										let node =
											(0..net_caps.len()).find(|&nd| {
												net_used[nd] + bytes
													<= net_caps[nd]
											});
										let Some(node) = node else {
											anyhow::bail!(
												"ooc: window has no home — disk {disk_cursor}B of {disk_budget}B, remote {net_used:?} of {net_caps:?} (admit passed what placement cannot hold)"
											);
										};
										net_used[node] += bytes;
										let h = Home::Remote {
											node,
											id: id_base | next_id,
										};
										next_id += 1;
										h
									}
								}
							}
						}
					}
				};
				homes.push(home);
			}
			Ok(Paged {
				homes,
				spb,
				chunk,
				ahead: RefCell::new(VecDeque::new()),
				net: net.clone(),
			})
		};

		let da_a = place(max_spb)?;
		let da_b = place(max_spb)?;
		let a_ctx = place(seq_spb)?;
		let a_q = place(seq_spb)?;
		let a_k = place(seq_spb)?;
		let a_v = place(seq_spb)?;
		let acts: Vec<Paged> = params
			.iter()
			.map(|p| place(p.out_dim))
			.collect::<anyhow::Result<Vec<Paged>>>()?;
		let concat = place(match (ca + cc).cmp(&0) {
			cmp::Ordering::Greater => ca + cc,
			cmp::Ordering::Less | cmp::Ordering::Equal => 1,
		})?;
		let a_dctx = place(seq_spb)?;
		let a_dq = place(seq_spb)?;
		let a_dk = place(seq_spb)?;
		let a_dv = place(seq_spb)?;
		let preacts: Vec<Option<Paged>> = params
			.iter()
			.map(|p| match p.act {
				Activation::Silu
				| Activation::Gelu
				| Activation::Elu
				| Activation::Selu
				| Activation::PRelu => place(p.out_dim).map(Some),
				Activation::Relu
				| Activation::Sigmoid
				| Activation::LeakyRelu
				| Activation::Tanh
				| Activation::Linear => Ok(None),
			})
			.collect::<anyhow::Result<Vec<Option<Paged>>>>()?;
		spill.as_ref()
			.map(|f| f.set_len(disk_cursor).context("size spill file"))
			.transpose()?;

		let pool_bufs = nonvram.min(POOL_BUFS);
		let host: HostPool = Arc::new(Mutex::new(
			(0..pool_bufs)
				.map(|_buf_idx| {
					let mut v = vec![0u8; wbytes];
					gpu_core::memory::par_touch(&mut v);
					v
				})
				.collect(),
		));
		let writer = Writer::new(spill.as_ref(), host.clone(), net.clone())?;

		let bps = |b: usize, t: Instant| b as f64 / t.elapsed().as_secs_f64();
		let mut rate_h2d = 0.0;
		let mut rate_d2h = 0.0;
		let mut rate_disk_r = 0.0;
		let mut rate_disk_w = 0.0;
		let mut rate_net_r = 0.0;
		let mut rate_net_w = 0.0;
		match nonvram.cmp(&0) {
			cmp::Ordering::Greater => {
				gpu_core::hip::device_synchronize().context("ooc calibrate sync")?;
				let mut buf = pool_take(&host)?;
				let t = Instant::now();
				wins[0].write_u8(&buf[..wbytes]).context("calibrate h2d")?;
				rate_h2d = bps(wbytes, t);
				let t = Instant::now();
				wins[0]
					.download_u8(&mut buf[..wbytes])
					.context("calibrate d2h")?;
				rate_d2h = bps(wbytes, t);
				pool_give(&host, buf)
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => Ok(()),
		}?;
		match disk_cursor.cmp(&0) {
			cmp::Ordering::Greater => {
				let f = spill.as_ref().ok_or_else(|| {
					anyhow::anyhow!("disk calibrate without spill file")
				})?;
				let mut buf = pool_take(&host)?;
				let t = Instant::now();
				f.write_all_at(&buf[..wbytes], 0)
					.context("calibrate spill write")?;
				drop_cache(f, 0, wbytes);
				rate_disk_w = bps(wbytes, t);
				let t = Instant::now();
				f.read_exact_at(&mut buf[..wbytes], 0)
					.context("calibrate spill read")?;
				rate_disk_r = bps(wbytes, t);
				pool_give(&host, buf)
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => Ok(()),
		}?;
		for _net in net_used.iter().find(|u| **u > 0).into_iter() {
			let ns = net
				.as_ref()
				.ok_or_else(|| anyhow::anyhow!("net used without net"))?;
			let cal_id = id_base | 0xffff_ffff;
			let buf = pool_take(&host)?;
			let t = Instant::now();
			ns[0].store_from(cal_id, &buf[..wbytes])
				.context("calibrate net write")?;
			rate_net_w = bps(wbytes, t);
			let t = Instant::now();
			let v = ns[0]
				.fetch(cal_id, 0, wbytes as u64)
				.context("calibrate net read")?;
			rate_net_r = bps(v.len(), t);
			drop(ns[0].free(cal_id));
			pool_give(&host, buf)?;
		}

		Ok(Ooc {
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
			conv_temp,
			conv_wg,
			rate_h2d,
			rate_d2h,
			rate_disk_r,
			rate_disk_w,
			rate_net_r,
			rate_net_w,
			net,
		})
	}

	pub fn report(&self) {
		let gb = |b: usize| b as f64 / (1u64 << 30) as f64;
		let mut v = 0usize;
		let mut r = 0usize;
		let mut d = 0usize;
		let mut nt = 0usize;
		let mut tally = |p: &Paged| {
			for w in 0..p.homes.len() {
				let h = &p.homes[w];
				let cnt = p.chunk.min(self.n - w * p.chunk);
				match h {
					Home::Vram(_buf) => v += cnt * p.spb * size_of::<f64>(),
					Home::Ram(x) => r += x.len(),
					Home::Disk(_off) => d += cnt * p.spb * size_of::<f64>(),
					Home::Remote { .. } => nt += cnt * p.spb * size_of::<f64>(),
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
			&self.a_q,
			&self.a_k,
			&self.a_v,
			&self.a_ctx,
			&self.a_dctx,
			&self.a_dq,
			&self.a_dk,
			&self.a_dv,
			&self.concat,
			&self.da_a,
			&self.da_b,
		] {
			tally(b);
		}
		Write::line(
			gpu,
			&format!(
				"waterfall  scratch homes: VRAM {:.2} GB -> RAM {:.2} GB -> DISK {:.2} GB -> NET {:.2} GB, {}-sample windows",
				gb(v),
				gb(r),
				gb(d),
				gb(nt),
				self.chunk
			),
		);
		let mut roof = format!(
			"waterfall  measured rooflines: gemm {} GF/s  vram {} GB/s",
			recipe_infer::GEMM_GFLOPS,
			recipe_infer::VRAM_GBS,
		);
		for rt in [
			Rate {
				label: "h2d",
				bps: self.rate_h2d,
			},
			Rate {
				label: "d2h",
				bps: self.rate_d2h,
			},
			Rate {
				label: "disk-r",
				bps: self.rate_disk_r,
			},
			Rate {
				label: "disk-w",
				bps: self.rate_disk_w,
			},
			Rate {
				label: "net-r",
				bps: self.rate_net_r,
			},
			Rate {
				label: "net-w",
				bps: self.rate_net_w,
			},
		]
		.into_iter()
		.filter(|rt| rt.bps > 0.0)
		{
			roof += &format!("  {} {:.3} GB/s", rt.label, rt.bps / 1e9);
		}
		Write::line(gpu, &roof);
	}

	pub fn forward(
		&mut self,
		params: &[LayerParams],
		x: &GpuBuffer,
		x_cat: Option<&GpuBuffer>,
		sc: &Scratch,
		concat_at: Option<ConcatFit>,
	) -> anyhow::Result<()> {
		let last = params.len() - 1;
		for l in 0..params.len() {
			let p = &params[l];
			let Flow::Go = self.concat_prefix(l, concat_at, x_cat)? else {
				return Ok(());
			};
			let s_l = sweep_start();
			match p.kind {
				LayerKind::Embed => {
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(self.n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let ids = view(x, s0 * p.in_dim * size_of::<f64>(), cnt * p.in_dim * size_of::<f64>());
						let out = self.acts[l].write_view(s0, cnt, &self.wins[0])?;
						kernels::gpu_gather_rows_into(
							&p.w,
							&ids,
							cnt * p.in_dim,
							p.dim,
							&out,
						)
						.context("gather")?;
						kernels::gpu_broadcast_sub_into(
							&out,
							&p.b,
							cnt * p.out_dim,
							p.out_dim,
							&out,
						)
						.context("pe add")?;
						self.acts[l].commit(s0, cnt, &out, &self.writer, &self.host)?;
					}
				}
				LayerKind::Attn => {
					let d = p.dim;
					let heads = p.heads;
					let s = p.in_dim / d;
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(self.n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let prev = self.acts[l - 1].read(
							s0,
							cnt,
							&self.wins[0],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let m = cnt * s;
						let q = self.a_q.write_view(s0, cnt, &self.wins[1])?;
						let k = self.a_k.write_view(s0, cnt, &self.wins[2])?;
						let v = self.a_v.write_view(s0, cnt, &self.wins[3])?;
						kernels::gpu_linear_into(&prev, &p.w, &p.b, m, d, d, &q)
							.context("attn q")?;
						kernels::gpu_linear_into(&prev, &p.wk, &p.b, m, d, d, &k)
							.context("attn k")?;
						kernels::gpu_linear_into(&prev, &p.wv, &p.b, m, d, d, &v)
							.context("attn v")?;
						gpu_core::rope::gpu_rope_qk_heads_inplace(
							&sc.c_one,
							&sc.c_rope_theta,
							m,
							d,
							heads,
							s,
							&q,
							&k,
						)
						.context("rope")?;
						self.a_q.commit(s0, cnt, &q, &self.writer, &self.host)?;
						self.a_k.commit(s0, cnt, &k, &self.writer, &self.host)?;
						self.a_v.commit(s0, cnt, &v, &self.writer, &self.host)?;
					}
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(self.n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let q = self.a_q.read(
							s0,
							cnt,
							&self.wins[0],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let k = self.a_k.read(
							s0,
							cnt,
							&self.wins[1],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let v = self.a_v.read(
							s0,
							cnt,
							&self.wins[2],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let ctx = self.a_ctx.write_view(s0, cnt, &self.wins[3])?;
						let lse =
							view(&self.lse, s0 * heads * s * size_of::<f64>(), cnt * heads * s * size_of::<f64>());
						kernels::gpu_flash_attention_train_into(
							&q, &k, &v, cnt, s, d, heads, &ctx, &lse,
						)
						.context("flash attn")?;
						self.a_ctx.commit(s0, cnt, &ctx, &self.writer, &self.host)?;
					}
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(self.n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let ctx = self.a_ctx.read(
							s0,
							cnt,
							&self.wins[0],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let out = self.acts[l].write_view(s0, cnt, &self.wins[1])?;
						kernels::gpu_linear_into(
							&ctx,
							&p.wo,
							&p.b,
							cnt * s,
							d,
							d,
							&out,
						)
						.context("attn out")?;
						self.acts[l].commit(s0, cnt, &out, &self.writer, &self.host)?;
					}
				}
				LayerKind::Dense | LayerKind::Conv => {
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(self.n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let concat_here = concat_at.filter(|cf| cf.pf == l);
						let prev = match l {
							0 => view(x, s0 * p.in_dim * size_of::<f64>(), cnt * p.in_dim * size_of::<f64>()),
							_nonzero => match concat_here {
								Some(_cf) => self.concat.read(
									s0,
									cnt,
									&self.wins[0],
									self.spill.as_ref(),
									self.n,
									&self.host,
								)?,
								None => self.acts[l - 1].read(
									s0,
									cnt,
									&self.wins[0],
									self.spill.as_ref(),
									self.n,
									&self.host,
								)?,
							},
						};
						let out = match l.cmp(&last) {
							cmp::Ordering::Equal => view(
								&sc.acts[last],
								s0 * p.out_dim * size_of::<f64>(),
								cnt * p.out_dim * size_of::<f64>(),
							),
							cmp::Ordering::Less | cmp::Ordering::Greater => {
								self.acts[l].write_view(s0, cnt, &self.wins[1])?
							}
						};
						match p.kind {
							LayerKind::Conv => {
								let cin = p.conv_cin;
								let kk = p.conv_k;
								let stride = p.conv_stride;
								let lin = p.in_dim / cin;
								let cout = p.out_dim / ((lin - kk) / stride + 1);
								kernels::gpu_conv1d_into(
									&prev, &p.w, &p.b, cnt, cin, lin, cout, kk,
									stride, &out,
								)
								.context("conv1d")?;
							}
							LayerKind::Dense | LayerKind::Embed | LayerKind::Attn => {
								match p.out_dim.cmp(&1) {
									cmp::Ordering::Equal => {
										kernels::gpu_matvec_bias_into(
											&prev, &p.w, &p.b, cnt, p.in_dim,
											&out,
										)
										.context("matvec")?
									}
									cmp::Ordering::Less
									| cmp::Ordering::Greater => kernels::gpu_linear_into(
										&prev, &p.w, &p.b, cnt, p.out_dim,
										p.in_dim, &out,
									)
									.context("linear")?,
								}
							}
						}
						let m = cnt * p.out_dim;
						let saved = match self.preacts[l].as_mut() {
							Some(pa) => {
								let pre = pa.write_view(s0, cnt, &self.wins[2])?;
								kernels::gpu_copy_into(&out, m, &pre)
									.context("copy preact")?;
								pa.commit(s0, cnt, &pre, &self.writer, &self.host)?;
								Some(pre)
							}
							None => None,
						};
						match p.act {
							Activation::PRelu => {
								let zz = saved.as_ref().ok_or_else(|| {
									anyhow::anyhow!(
										"z-based activation without preact"
									)
								})?;
								kernels::gpu_leaky_relu_into(zz, &p.palpha, m, &out)
									.context("prelu")
							}
							Activation::Elu => {
								let zz = saved.as_ref().ok_or_else(|| {
									anyhow::anyhow!(
										"z-based activation without preact"
									)
								})?;
								gpu_core::k_gapact::gpu_elu(
									zz,
									&sc.c_elu_alpha,
									m,
									&out,
								)
								.context("elu")
							}
							Activation::Selu => {
								let zz = saved.as_ref().ok_or_else(|| {
									anyhow::anyhow!(
										"z-based activation without preact"
									)
								})?;
								gpu_core::k_gapact::gpu_selu(
									zz,
									&sc.c_selu_alpha,
									&sc.c_selu_lambda,
									m,
									&out,
								)
								.context("selu")
							}
							Activation::Silu => {
								let zz = saved.as_ref().ok_or_else(|| {
									anyhow::anyhow!(
										"z-based activation without preact"
									)
								})?;
								kernels::gpu_silu_into(zz, m, &out).context("silu")
							}
							Activation::Gelu => {
								let zz = saved.as_ref().ok_or_else(|| {
									anyhow::anyhow!(
										"z-based activation without preact"
									)
								})?;
								kernels::gpu_gelu_into(zz, m, &out).context("gelu")
							}
							Activation::Relu => kernels::gpu_relu_into(&out, m, &out)
								.context("relu"),
							Activation::Sigmoid => {
								kernels::gpu_sigmoid_into(&out, m, &out)
									.context("sigmoid")
							}
							Activation::LeakyRelu => kernels::gpu_leaky_relu_into(
								&out,
								&sc.c_leaky_alpha,
								m,
								&out,
							)
							.context("leaky"),
							Activation::Tanh => kernels::gpu_tanh_into(&out, m, &out)
								.context("tanh"),
							Activation::Linear => Ok(()),
						}?;
						for _mid in Some(()).filter(|_unit| l != last).into_iter() {
							self.acts[l].commit(
								s0,
								cnt,
								&out,
								&self.writer,
								&self.host,
							)?;
						}
					}
				}
			}
			self.sweep_line("fwd", l, p, s_l)?;
		}
		Ok(())
	}

	fn concat_prefix(
		&mut self,
		l: usize,
		concat_at: Option<ConcatFit>,
		x_cat: Option<&GpuBuffer>,
	) -> anyhow::Result<Flow> {
		let Some(cf) = concat_at.filter(|cf| l == cf.pf) else {
			return Ok(Flow::Go);
		};
		let a = cf.a;
		let c = cf.c;
		let s_c = sweep_start();
		self.writer.barrier(self.spill.as_ref())?;
		for win in chunks(self.n, self.chunk) {
			let s0 = win.s0;
			let cnt = win.cnt;
			let Flow::Go = self.bail()? else {
				return Ok(Flow::Halt);
			};
			let prev = self.acts[l - 1].read(
				s0,
				cnt,
				&self.wins[0],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let xc = view(
				x_cat.ok_or_else(|| anyhow::anyhow!("x_cat"))?,
				s0 * c * size_of::<f64>(),
				cnt * c * size_of::<f64>(),
			);
			let out = self.concat.write_view(s0, cnt, &self.wins[1])?;
			kernels::gpu_concat_into(&prev, &xc, cnt, a, c, &out).context("concat")?;
			self.concat
				.commit(s0, cnt, &out, &self.writer, &self.host)?;
		}
		let cw = recipe_infer::Work {
			flop: 0.0,
			bytes: 16.0 * (self.n * (a + c)) as f64,
		};
		self.sweep_line_work("fwd", l, "concat", &format!("{a}+{c}"), cw, s_c)?;
		Ok(Flow::Go)
	}

	pub(crate) fn backward(
		&mut self,
		params: &[LayerParams],
		x: &GpuBuffer,
		ybuf: &GpuBuffer,
		sc: &Scratch,
		ss: &StepScalars,
		loss: Loss,
		concat_at: Option<ConcatFit>,
	) -> anyhow::Result<()> {
		let last = params.len() - 1;
		let n = self.n;
		self.da_a.spb = params[last].out_dim;
		self.writer.barrier(self.spill.as_ref())?;
		for win in chunks(n, self.chunk) {
			let s0 = win.s0;
			let cnt = win.cnt;
			let Flow::Go = self.bail()? else {
				return Ok(());
			};
			let k = params[last].out_dim;
			let out = view(&sc.acts[last], s0 * k * size_of::<f64>(), cnt * k * size_of::<f64>());
			let y = view(ybuf, s0 * k * size_of::<f64>(), cnt * k * size_of::<f64>());
			let da = self.da_a.write_view(s0, cnt, &self.wins[0])?;
			crate::train::loss_grad_into(loss, &out, &y, &da, cnt, cnt * k, sc, ss)?;
			self.da_a.commit(s0, cnt, &da, &self.writer, &self.host)?;
		}
		let mut flip = Flip::A;
		for l in (0..params.len()).rev() {
			let p = &params[l];
			let s_l = sweep_start();
			let in_dim = p.in_dim;
			let out_dim = p.out_dim;
			match p.kind {
				LayerKind::Embed => {
					kernels::gpu_scale_inplace(&ss.zero, p.vocab * p.dim, &sc.embed_grad)
						.context("embed zero")?;
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let da = self.da(flip).read(
							s0,
							cnt,
							&self.wins[0],
							self.spill.as_ref(),
							self.n,
							&self.host,
						)?;
						let ids = view(x, s0 * p.in_dim * size_of::<f64>(), cnt * p.in_dim * size_of::<f64>());
						kernels::gpu_scatter_add(
							&ids,
							&da,
							cnt * p.in_dim,
							p.dim,
							&sc.embed_grad,
						)
						.context("embed scatter")?;
					}
					kernels::gpu_sgd_update(
						&sc.embed_grad,
						&ss.neg_lr,
						p.vocab * p.dim,
						&p.w,
					)
					.context("sgd embed")?;
					self.sweep_line("bwd", l, p, s_l)?;
					flip = flip.toggle();
				}
				LayerKind::Attn => {
					self.attn_backward(params, l, x, sc, ss, flip)?;
					self.sweep_line("bwd", l, p, s_l)?;
					flip = flip.toggle();
				}
				LayerKind::Conv => {
					let cin = p.conv_cin;
					let kk = p.conv_k;
					let stride = p.conv_stride;
					let lin = in_dim / cin;
					let lout = (lin - kk) / stride + 1;
					let cout = out_dim / lout;
					self.zero_accs(ss, cout * cin * kk, cout, &p.act)?;
					self.set_da_below_spb(
						flip,
						match l {
							0 => 1,
							_positive => in_dim,
						},
					);
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let w = self.bwd_win(flip, l, last, out_dim, s0, cnt, sc)?;
						let grad = self.act_grad(
							p, l, &w.da, &w.act_l, &w.dz, w.m, s0, cnt, sc,
						)?;
						let a_prev = match l {
							0 => view(x, s0 * in_dim * size_of::<f64>(), cnt * in_dim * size_of::<f64>()),
							_positive => self.acts[l - 1].read(
								s0,
								cnt,
								&self.wins[6],
								self.spill.as_ref(),
								self.n,
								&self.host,
							)?,
						};
						kernels::gpu_conv1d_backward_filter_into(
							grad,
							&a_prev,
							&self.conv_temp,
							&self.reduce_ws,
							cnt,
							cin,
							lin,
							cout,
							kk,
							stride,
							self.conv_wg,
							&self.dw_tmp,
						)
						.context("conv filter bwd")?;
						kernels::gpu_add_inplace(
							&self.dw_tmp,
							cout * cin * kk,
							&self.dw_acc,
						)
						.context("conv dw acc")?;
						kernels::gpu_conv1d_backward_bias_into(
							grad,
							cnt,
							cout,
							lout,
							&self.db_acc,
						)
						.context("conv bias bwd")?;
						match l.checked_sub(1) {
							Some(_below_l) => {
								let below_pg = match flip {
									Flip::B => &mut self.da_a,
									Flip::A => &mut self.da_b,
								};
								let below =
									below_pg.write_view(s0, cnt, &self.wins[7])?;
								kernels::gpu_conv1d_backward_data_into(
									grad, &p.w, cnt, cin, lin, cout, kk, stride,
									&below,
								)
								.context("conv data bwd")?;
								below_pg.commit(
									s0,
									cnt,
									&below,
									&self.writer,
									&self.host,
								)
							}
							None => Ok(()),
						}?;
					}
					self.sgd_step(ss, p, cout * cin * kk, cout)?;
					self.sweep_line("bwd", l, p, s_l)?;
					flip = flip.toggle();
				}
				LayerKind::Dense => {
					self.zero_accs(ss, in_dim * out_dim, out_dim, &p.act)?;
					self.set_da_below_spb(
						flip,
						match l {
							0 => 1,
							_positive => in_dim,
						},
					);
					self.writer.barrier(self.spill.as_ref())?;
					for win in chunks(n, self.chunk) {
						let s0 = win.s0;
						let cnt = win.cnt;
						let Flow::Go = self.bail()? else {
							return Ok(());
						};
						let w = self.bwd_win(flip, l, last, out_dim, s0, cnt, sc)?;
						let grad = self.act_grad(
							p, l, &w.da, &w.act_l, &w.dz, w.m, s0, cnt, sc,
						)?;
						let a_prev = match l {
							0 => view(x, s0 * in_dim * size_of::<f64>(), cnt * in_dim * size_of::<f64>()),
							_positive => match concat_at.filter(|cf| cf.pf == l) {
								Some(_cf) => self.concat.read(
									s0,
									cnt,
									&self.wins[6],
									self.spill.as_ref(),
									self.n,
									&self.host,
								)?,
								None => self.acts[l - 1].read(
									s0,
									cnt,
									&self.wins[6],
									self.spill.as_ref(),
									self.n,
									&self.host,
								)?,
							},
						};
						match l {
							0 => {
								kernels::gpu_linear_backward_weights_only_into(
									grad,
									&a_prev,
									&self.reduce_ws,
									&self.dw_partials,
									cnt,
									out_dim,
									in_dim,
									&self.dw_tmp,
									&self.db_tmp,
								)
								.context("linear weights bwd")?;
							}
							_positive => {
								let below_pg = match flip {
									Flip::B => &mut self.da_a,
									Flip::A => &mut self.da_b,
								};
								let below =
									below_pg.write_view(s0, cnt, &self.wins[7])?;
								kernels::gpu_linear_backward_full_into(
									grad,
									&a_prev,
									&p.w,
									&self.reduce_ws,
									&self.dw_partials,
									cnt,
									out_dim,
									in_dim,
									&below,
									&self.dw_tmp,
									&self.db_tmp,
								)
								.context("linear full bwd")?;
								match concat_at.filter(|cf| l == cf.pf) {
									Some(cf) => {
										let a = cf.a;
										let c = cf.c;
										let compact = view(
											&self.wins[8],
											0,
											cnt * a * size_of::<f64>(),
										);
										kernels::gpu_slice_lead_into(
											&below,
											cnt,
											a + c,
											a,
											&compact,
										)
										.context("concat slice")?;
										below_pg.spb = a;
										below_pg.commit(
											s0,
											cnt,
											&compact,
											&self.writer,
											&self.host,
										)?;
										below_pg.spb = a + c;
									}
									None => below_pg.commit(
										s0,
										cnt,
										&below,
										&self.writer,
										&self.host,
									)?,
								}
							}
						}
						kernels::gpu_add_inplace(
							&self.dw_tmp,
							in_dim * out_dim,
							&self.dw_acc,
						)
						.context("dw acc")?;
						kernels::gpu_add_inplace(&self.db_tmp, out_dim, &self.db_acc)
							.context("db acc")?;
					}
					for cf in concat_at.filter(|cf| l == cf.pf).into_iter() {
						self.set_da_below_spb(flip, cf.a);
					}
					self.sgd_step(ss, p, in_dim * out_dim, out_dim)?;
					self.sweep_line("bwd", l, p, s_l)?;
					flip = flip.toggle();
				}
			}
		}
		Ok(())
	}

	fn sweep_line(
		&self,
		phase: &str,
		l: usize,
		p: &LayerParams,
		s: SweepStart,
	) -> anyhow::Result<()> {
		let kind = match p.kind {
			LayerKind::Embed => "embed",
			LayerKind::Attn => "attn",
			LayerKind::Conv => "conv",
			LayerKind::Dense => "dense",
		};
		let w = match phase {
			"fwd" => recipe_infer::layer_fwd(p, self.n),
			_other => recipe_infer::layer_bwd(p, self.n, l == 0),
		};
		self.sweep_line_work(
			phase,
			l,
			kind,
			&format!("{}->{}", p.in_dim, p.out_dim),
			w,
			s,
		)
	}

	fn sweep_line_work(
		&self,
		phase: &str,
		l: usize,
		kind: &str,
		dims: &str,
		w: recipe_infer::Work,
		s: SweepStart,
	) -> anyhow::Result<()> {
		let x = xfer();
		let h2d = x.h2d;
		let d2h = x.d2h;
		let disk_r = DISK_R_BYTES.load(Ordering::Relaxed);
		let disk_w = DISK_W_BYTES.load(Ordering::Relaxed);
		let net_r = NET_R_BYTES.load(Ordering::Relaxed);
		let net_w = NET_W_BYTES.load(Ordering::Relaxed);
		let streamed = (h2d - s.h2d)
			+ (d2h - s.d2h) + (disk_r - s.disk_r)
			+ (disk_w - s.disk_w)
			+ (net_r - s.net_r)
			+ (net_w - s.net_w);
		let cmp::Ordering::Greater = streamed.cmp(&0) else {
			return Ok(());
		};
		gpu_core::hip::device_synchronize().context("sweep sync")?;
		let sec = s.t.elapsed().as_secs_f64();
		let gfs = w.flop / sec / 1e9;
		let gbs = w.bytes / sec / 1e9;
		let mut line = format!(
			"ooc {phase} L{l} {kind} {dims}  {} windows  {sec:.1}s  {:.2} GFLOP {gfs:.1} GF/s {:.0}% gemm  {:.2} GB {gbs:.1} GB/s {:.0}% vram",
			self.n.div_ceil(self.chunk),
			w.flop / 1e9,
			100.0 * gfs / recipe_infer::GEMM_GFLOPS,
			w.bytes / 1e9,
			100.0 * gbs / recipe_infer::VRAM_GBS,
		);
		for st in [
			Stream {
				label: "h2d",
				delta: h2d - s.h2d,
				rate: self.rate_h2d,
			},
			Stream {
				label: "d2h",
				delta: d2h - s.d2h,
				rate: self.rate_d2h,
			},
			Stream {
				label: "disk-r",
				delta: disk_r - s.disk_r,
				rate: self.rate_disk_r,
			},
			Stream {
				label: "disk-w",
				delta: disk_w - s.disk_w,
				rate: self.rate_disk_w,
			},
			Stream {
				label: "net-r",
				delta: net_r - s.net_r,
				rate: self.rate_net_r,
			},
			Stream {
				label: "net-w",
				delta: net_w - s.net_w,
				rate: self.rate_net_w,
			},
		]
		.into_iter()
		.filter(|st| st.delta > 0)
		{
			let bps = st.delta as f64 / sec;
			line += &format!(
				"  {} {:.2} GB {:.2} GB/s {:.0}%",
				st.label,
				st.delta as f64 / 1e9,
				bps / 1e9,
				100.0 * bps / st.rate,
			);
		}
		let drain = self.writer.drained.take();
		for _drained in drain
			.partial_cmp(&0.0)
			.filter(|ord| matches!(ord, cmp::Ordering::Greater))
			.into_iter()
		{
			line += &format!("  drain {drain:.1}s");
		}
		Write::line(gpu, &line);
		Ok(())
	}

	fn all_paged(&self) -> impl Iterator<Item = &Paged> {
		let Ooc {
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
			..
		} = self;
		acts.iter().chain(preacts.iter().flatten()).chain([
			a_q, a_k, a_v, a_ctx, a_dctx, a_dq, a_dk, a_dv, concat, da_a, da_b,
		])
	}

	fn bail(&self) -> anyhow::Result<Flow> {
		let Interrupt::Yes = interrupted() else {
			return Ok(Flow::Go);
		};
		for p in self.all_paged() {
			p.drain_ahead(&self.host)?;
		}
		Ok(Flow::Halt)
	}

	fn da(&self, flip: Flip) -> &Paged {
		match flip {
			Flip::B => &self.da_b,
			Flip::A => &self.da_a,
		}
	}

	fn zero_accs(
		&self,
		ss: &StepScalars,
		dw_len: usize,
		db_len: usize,
		act: &Activation,
	) -> anyhow::Result<()> {
		kernels::gpu_scale_inplace(&ss.zero, dw_len, &self.dw_acc).context("dw_acc zero")?;
		kernels::gpu_scale_inplace(&ss.zero, db_len, &self.db_acc).context("db_acc zero")?;
		match prelu_gate(act) {
			Some(_p) => kernels::gpu_scale_inplace(&ss.zero, 1, &self.scalar_acc)
				.context("scalar_acc zero"),
			None => Ok(()),
		}?;
		Ok(())
	}

	fn sgd_step(
		&self,
		ss: &StepScalars,
		p: &LayerParams,
		dw_len: usize,
		db_len: usize,
	) -> anyhow::Result<()> {
		kernels::gpu_sgd_update(&self.dw_acc, &ss.neg_lr, dw_len, &p.w).context("sgd w")?;
		kernels::gpu_sgd_update(&self.db_acc, &ss.neg_lr, db_len, &p.b).context("sgd b")?;
		match prelu_gate(&p.act) {
			Some(_p) => kernels::gpu_sgd_update(&self.scalar_acc, &ss.neg_lr, 1, &p.palpha)
				.context("sgd prelu"),
			None => Ok(()),
		}?;
		Ok(())
	}

	fn bwd_win(
		&self,
		flip: Flip,
		l: usize,
		last: usize,
		out_dim: usize,
		s0: usize,
		cnt: usize,
		sc: &Scratch,
	) -> anyhow::Result<BwdWin> {
		let m = cnt * out_dim;
		let da = self.da(flip).read(
			s0,
			cnt,
			&self.wins[0],
			self.spill.as_ref(),
			self.n,
			&self.host,
		)?;
		let act_l = match l.cmp(&last) {
			cmp::Ordering::Equal => view(&sc.acts[last], s0 * out_dim * size_of::<f64>(), cnt * out_dim * size_of::<f64>()),
			cmp::Ordering::Less | cmp::Ordering::Greater => self.acts[l].read(
				s0,
				cnt,
				&self.wins[1],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?,
		};
		let dz = view(&self.wins[2], 0, m * size_of::<f64>());
		Ok(BwdWin { da, act_l, dz, m })
	}

	fn act_grad<'g>(
		&self,
		p: &LayerParams,
		l: usize,
		da: &'g GpuBuffer,
		act_l: &GpuBuffer,
		dz: &'g GpuBuffer,
		m: usize,
		s0: usize,
		cnt: usize,
		sc: &Scratch,
	) -> anyhow::Result<&'g GpuBuffer> {
		match p.act {
			Activation::Relu => {
				kernels::gpu_relu_backward_into(da, act_l, m, dz).context("relu bwd")?;
				Ok(dz)
			}
			Activation::Sigmoid => {
				kernels::gpu_sigmoid_backward_into(da, act_l, m, dz)
					.context("sigmoid bwd")?;
				Ok(dz)
			}
			Activation::LeakyRelu => {
				kernels::gpu_leaky_relu_backward_into(da, act_l, &sc.c_leaky_alpha, m, dz)
					.context("leaky bwd")?;
				Ok(dz)
			}
			Activation::PRelu => {
				kernels::gpu_leaky_relu_backward_into(da, act_l, &p.palpha, m, dz)
					.context("prelu bwd")?;
				let pre = self.preacts[l]
					.as_ref()
					.ok_or_else(|| anyhow::anyhow!("prelu preact"))?
					.read(
						s0,
						cnt,
						&self.wins[3],
						self.spill.as_ref(),
						self.n,
						&self.host,
					)?;
				let t0 = view(&self.wins[4], 0, m * size_of::<f64>());
				let t1 = view(&self.wins[5], 0, m * size_of::<f64>());
				kernels::gpu_relu_into(&pre, m, &t0).context("prelu relu")?;
				kernels::gpu_copy_into(&pre, m, &t1).context("prelu copy")?;
				kernels::gpu_sub_inplace(&t0, m, &t1).context("prelu sub")?;
				kernels::gpu_mul_inplace(da, m, &t1).context("prelu mul")?;
				kernels::gpu_reduce_sum_cols_into(
					&t1,
					&self.reduce_ws,
					m,
					1,
					&self.scalar_tmp,
				)
				.context("prelu reduce")?;
				kernels::gpu_add_inplace(&self.scalar_tmp, 1, &self.scalar_acc)
					.context("prelu acc")?;
				Ok(dz)
			}
			Activation::Tanh => {
				kernels::gpu_tanh_backward_into(da, act_l, m, dz).context("tanh bwd")?;
				Ok(dz)
			}
			Activation::Elu => {
				let pre = self.preact(l, "elu")?.read(
					s0,
					cnt,
					&self.wins[3],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?;
				gpu_core::k_gapact::gpu_elu_backward(da, &pre, &sc.c_elu_alpha, m, dz)
					.context("elu bwd")?;
				Ok(dz)
			}
			Activation::Selu => {
				let pre = self.preact(l, "selu")?.read(
					s0,
					cnt,
					&self.wins[3],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?;
				gpu_core::k_gapact::gpu_selu_backward(
					da,
					&pre,
					&sc.c_selu_alpha,
					&sc.c_selu_lambda,
					m,
					dz,
				)
				.context("selu bwd")?;
				Ok(dz)
			}
			Activation::Silu => {
				let pre = self.preact(l, "silu")?.read(
					s0,
					cnt,
					&self.wins[3],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?;
				kernels::gpu_silu_backward_into(da, &pre, m, dz).context("silu bwd")?;
				Ok(dz)
			}
			Activation::Gelu => {
				let pre = self.preact(l, "gelu")?.read(
					s0,
					cnt,
					&self.wins[3],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?;
				kernels::gpu_gelu_backward_into(da, &pre, m, dz).context("gelu bwd")?;
				Ok(dz)
			}
			Activation::Linear => Ok(da),
		}
	}

	fn preact(&self, l: usize, who: &str) -> anyhow::Result<&Paged> {
		self.preacts[l]
			.as_ref()
			.ok_or_else(|| anyhow::anyhow!("{who} preact"))
	}
	fn set_da_below_spb(&mut self, flip: Flip, spb: usize) {
		match flip {
			Flip::B => self.da_a.spb = spb,
			Flip::A => self.da_b.spb = spb,
		}
	}

	fn attn_backward(
		&mut self,
		params: &[LayerParams],
		l: usize,
		x: &GpuBuffer,
		sc: &Scratch,
		ss: &StepScalars,
		flip: Flip,
	) -> anyhow::Result<()> {
		let p = &params[l];
		let d = p.dim;
		let heads = p.heads;
		let s = p.in_dim / d;
		let n = self.n;
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dw_acc).context("dw_acc zero")?;
		self.writer.barrier(self.spill.as_ref())?;
		for win in chunks(n, self.chunk) {
			let s0 = win.s0;
			let cnt = win.cnt;
			let Flow::Go = self.bail()? else {
				return Ok(());
			};
			let m = cnt * s;
			let da = self.da(flip).read(
				s0,
				cnt,
				&self.wins[0],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let ctx = self.a_ctx.read(
				s0,
				cnt,
				&self.wins[1],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let dctx = self.a_dctx.write_view(s0, cnt, &self.wins[2])?;
			kernels::gpu_linear_backward_full_into(
				&da,
				&ctx,
				&p.wo,
				&self.reduce_ws,
				&self.dw_partials,
				m,
				d,
				d,
				&dctx,
				&self.dw_tmp,
				&self.db_tmp,
			)
			.context("attn wo bwd")?;
			kernels::gpu_add_inplace(&self.dw_tmp, d * d, &self.dw_acc)
				.context("dw_acc add")?;
			self.a_dctx
				.commit(s0, cnt, &dctx, &self.writer, &self.host)?;
		}
		kernels::gpu_sgd_update(&self.dw_acc, &ss.neg_lr, d * d, &p.wo).context("sgd wo")?;
		self.writer.barrier(self.spill.as_ref())?;
		for win in chunks(n, self.chunk) {
			let s0 = win.s0;
			let cnt = win.cnt;
			let Flow::Go = self.bail()? else {
				return Ok(());
			};
			let q = self.a_q.read(
				s0,
				cnt,
				&self.wins[0],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let k = self.a_k.read(
				s0,
				cnt,
				&self.wins[1],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let v = self.a_v.read(
				s0,
				cnt,
				&self.wins[2],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let ctx = self.a_ctx.read(
				s0,
				cnt,
				&self.wins[3],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let dctx = self.a_dctx.read(
				s0,
				cnt,
				&self.wins[4],
				self.spill.as_ref(),
				self.n,
				&self.host,
			)?;
			let lse = view(&self.lse, s0 * heads * s * size_of::<f64>(), cnt * heads * s * size_of::<f64>());
			let dsum = view(&self.dsum, s0 * heads * s * size_of::<f64>(), cnt * heads * s * size_of::<f64>());
			let dq = self.a_dq.write_view(s0, cnt, &self.wins[5])?;
			let dk = self.a_dk.write_view(s0, cnt, &self.wins[6])?;
			let dv = self.a_dv.write_view(s0, cnt, &self.wins[7])?;
			kernels::gpu_flash_attention_backward_into(
				&q, &k, &v, &ctx, &dctx, &lse, cnt, s, d, heads, &dsum, &dq, &dk, &dv,
			)
			.context("flash attn bwd")?;
			gpu_core::rope::gpu_rope_qk_heads_inplace(
				&sc.c_neg_one,
				&sc.c_rope_theta,
				cnt * s,
				d,
				heads,
				s,
				&dq,
				&dk,
			)
			.context("rope bwd")?;
			self.a_dq.commit(s0, cnt, &dq, &self.writer, &self.host)?;
			self.a_dk.commit(s0, cnt, &dk, &self.writer, &self.host)?;
			self.a_dv.commit(s0, cnt, &dv, &self.writer, &self.host)?;
		}
		self.set_da_below_spb(flip, p.in_dim);
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwq_acc).context("dwq zero")?;
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwk_acc).context("dwk zero")?;
		kernels::gpu_scale_inplace(&ss.zero, d * d, &self.dwv_acc).context("dwv zero")?;
		self.writer.barrier(self.spill.as_ref())?;
		for win in chunks(n, self.chunk) {
			let s0 = win.s0;
			let cnt = win.cnt;
			let Flow::Go = self.bail()? else {
				return Ok(());
			};
			let m = cnt * s;
			let h = match l {
				0 => view(x, s0 * p.in_dim * size_of::<f64>(), cnt * p.in_dim * size_of::<f64>()),
				_positive => self.acts[l - 1].read(
					s0,
					cnt,
					&self.wins[0],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?,
			};
			let below_pg = match flip {
				Flip::B => &mut self.da_a,
				Flip::A => &mut self.da_b,
			};
			let below = below_pg.write_view(s0, cnt, &self.wins[1])?;
			let dh_tmp = view(&self.wins[2], 0, cnt * p.in_dim * size_of::<f64>());
			let qkv = [
				Qkv {
					w: &p.w,
					dbuf: &self.a_dq,
				},
				Qkv {
					w: &p.wk,
					dbuf: &self.a_dk,
				},
				Qkv {
					w: &p.wv,
					dbuf: &self.a_dv,
				},
			];
			for wi in 0..qkv.len() {
				let w = qkv[wi].w;
				let dbuf = qkv[wi].dbuf;
				let dg = dbuf.read(
					s0,
					cnt,
					&self.wins[3],
					self.spill.as_ref(),
					self.n,
					&self.host,
				)?;
				let dst = match wi {
					0 => &below,
					_other => &dh_tmp,
				};
				kernels::gpu_linear_backward_full_into(
					&dg,
					&h,
					w,
					&self.reduce_ws,
					&self.dw_partials,
					m,
					d,
					d,
					dst,
					&self.dw_tmp,
					&self.db_tmp,
				)
				.context("attn wqkv bwd")?;
				let acc = match wi {
					0 => &self.dwq_acc,
					1 => &self.dwk_acc,
					_other => &self.dwv_acc,
				};
				kernels::gpu_add_inplace(&self.dw_tmp, d * d, acc).context("acc add")?;
				match wi.cmp(&0) {
					cmp::Ordering::Greater => {
						kernels::gpu_add_inplace(&dh_tmp, cnt * p.in_dim, &below)
							.context("dh add")
					}
					cmp::Ordering::Less | cmp::Ordering::Equal => Ok(()),
				}?;
			}
			below_pg.commit(s0, cnt, &below, &self.writer, &self.host)?;
		}
		kernels::gpu_sgd_update(&self.dwq_acc, &ss.neg_lr, d * d, &p.w).context("sgd wq")?;
		kernels::gpu_sgd_update(&self.dwk_acc, &ss.neg_lr, d * d, &p.wk).context("sgd wk")?;
		kernels::gpu_sgd_update(&self.dwv_acc, &ss.neg_lr, d * d, &p.wv).context("sgd wv")?;
		Ok(())
	}
}

struct Qkv<'a> {
	w: &'a GpuBuffer,
	dbuf: &'a Paged,
}

impl Drop for Ooc {
	fn drop(&mut self) {
		let Ooc { writer, spill, .. } = &mut *self;
		let r = writer.barrier(spill.as_ref());
		if !r.is_ok() {
			Write::error(format!(
				"ooc drop barrier: {}",
				r.as_ref()
					.err()
					.map(|e| format!("{e:#}"))
					.unwrap_or_default()
			));
			return;
		}
		let Some(net) = self.net.clone() else {
			return;
		};
		for p in self.all_paged() {
			for h in &p.homes {
				let Home::Remote { node, id } = h else {
					continue;
				};
				drop(net[*node].free(*id));
			}
		}
	}
}

use crate::machine::Machine;
use anyhow::{Result, bail, ensure};
use ogdl::log::{Write, net};
use recipe_infer::bridge::{Chan, chan, recv_from};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::io::{Read, Write as _};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::process;
use std::slice;
use std::str::from_utf8;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub const MAGIC: u32 = 0x5243_5031;
pub const PORT: u16 = 7845;
const HDR: usize = 32;
const NL: char = '\u{a}';
const NLS: &str = "\u{a}";

const BEACON_MAGIC: u32 = 0x5243_5042;
const BEACON_SECS: u64 = 2;
const STALE_MS: u128 = 3 * BEACON_SECS as u128 * 1000;
const CONNECT_TIMEOUT_SECS: u64 = 2;

pub const FN_MOE_FFN: u16 = 1;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
	Hello = 1,
	Store = 2,
	Run = 3,
	Fetch = 4,
	Stat = 5,
	Reply = 6,
	Err = 7,
	Peers = 8,
	Free = 9,
}

impl Op {
	fn from(b: u8) -> Result<Op> {
		const TABLE: [Op; 9] = [
			Op::Hello,
			Op::Store,
			Op::Run,
			Op::Fetch,
			Op::Stat,
			Op::Reply,
			Op::Err,
			Op::Peers,
			Op::Free,
		];
		b.checked_sub(1)
			.and_then(|i| TABLE.get(i as usize))
			.copied()
			.ok_or_else(|| anyhow::anyhow!("wire: bad op byte {b}"))
	}
}

pub struct Frame {
	pub op: Op,
	pub flags: u8,
	pub tag: u16,
	pub seq: u32,
	pub id: u64,
	pub data: Vec<u8>,
}

impl Frame {
	fn new(op: Op, tag: u16, seq: u32, id: u64, data: Vec<u8>) -> Frame {
		Frame {
			op,
			flags: 0,
			tag,
			seq,
			id,
			data,
		}
	}
}

fn write_frame_raw(
	s: &mut TcpStream,
	op: Op,
	flags: u8,
	tag: u16,
	seq: u32,
	id: u64,
	data: &[u8],
) -> Result<()> {
	let mut h = [0u8; HDR];
	h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
	h[4] = op as u8;
	h[5] = flags;
	h[6..8].copy_from_slice(&tag.to_le_bytes());
	h[8..12].copy_from_slice(&seq.to_le_bytes());
	h[12..20].copy_from_slice(&id.to_le_bytes());
	h[20..28].copy_from_slice(&(data.len() as u64).to_le_bytes());
	s.write_all(&h)?;
	for payload in Some(data).filter(|d| !d.is_empty()).into_iter() {
		s.write_all(payload)?;
	}
	s.flush()?;
	Ok(())
}

fn write_frame(s: &mut TcpStream, f: &Frame) -> Result<()> {
	write_frame_raw(s, f.op, f.flags, f.tag, f.seq, f.id, &f.data)
}

fn note_frame_end(e: &anyhow::Error, what: &str) {
	if e.downcast_ref::<std::io::Error>()
		.is_some_and(|io_err| return io_err.kind() == std::io::ErrorKind::UnexpectedEof)
	{
		return;
	}
	Write::error(format!("{what}: {e}"));
}

fn read_frame(s: &mut TcpStream) -> Result<Frame> {
	let mut h = [0u8; HDR];
	s.read_exact(&mut h)?;
	let magic = u32::from_le_bytes(h[0..4].try_into()?);
	ensure!(magic == MAGIC, "wire: bad magic {magic:#010x}");
	let op = Op::from(h[4])?;
	let flags = h[5];
	let tag = u16::from_le_bytes(h[6..8].try_into()?);
	let seq = u32::from_le_bytes(h[8..12].try_into()?);
	let id = u64::from_le_bytes(h[12..20].try_into()?);
	let len = u64::from_le_bytes(h[20..28].try_into()?) as usize;
	let mut data = vec![0u8; len];
	s.read_exact(&mut data)?;
	Ok(Frame {
		op,
		flags,
		tag,
		seq,
		id,
		data,
	})
}

#[derive(Clone, Debug)]
pub struct NodeInfo {
	pub arch: String,
	pub gpus: u32,
	pub vram: u64,
	pub ram: u64,
}

impl NodeInfo {
	pub fn probe() -> NodeInfo {
		let arch = env::var("GPU_ARCH").unwrap_or_else(|_env| "storage".to_string());
		NodeInfo {
			arch,
			gpus: 0,
			vram: 0,
			ram: mem_available(),
		}
	}
	fn encode(&self) -> Vec<u8> {
		format!(
			"{}{NL}{}{NL}{}{NL}{}",
			self.arch, self.gpus, self.vram, self.ram
		)
		.into_bytes()
	}
	fn decode(b: &[u8]) -> Result<NodeInfo> {
		let s = from_utf8(b)?;
		let mut it = s.split(NL);
		let arch = it.next().unwrap_or("").to_string();
		let gpus = it.next().unwrap_or("0").parse()?;
		let vram = it.next().unwrap_or("0").parse()?;
		let ram = it.next().unwrap_or("0").parse()?;
		Ok(NodeInfo {
			arch,
			gpus,
			vram,
			ram,
		})
	}
}

fn mem_available() -> u64 {
	let Ok(s) = fs::read_to_string("/proc/meminfo") else {
		return 0;
	};
	s.lines()
		.find_map(|l| {
			let kb = l.strip_prefix("MemAvailable:")?;
			let kb: u64 = kb.trim().trim_end_matches(" kB").parse().unwrap_or(0);
			Some(kb * 1024)
		})
		.unwrap_or(0)
}

fn hostname() -> String {
	fs::read_to_string("/proc/sys/kernel/hostname")
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

enum Link {
	Wired,
	Wireless,
}

struct Iface {
	ip: Ipv4Addr,
	bcast: Ipv4Addr,
	link: Link,
}

fn ifaces() -> Vec<Iface> {
	let mut out = Vec::new();
	for nic in gpu_core::sys::ipv4_interfaces() {
		let link = match fs::metadata(format!("/sys/class/net/{}/wireless", nic.name)).ok() {
			Some(_meta) => Link::Wireless,
			None => Link::Wired,
		};
		out.push(Iface {
			ip: Ipv4Addr::from(nic.ip),
			bcast: Ipv4Addr::from(nic.ip | !nic.mask),
			link,
		});
	}
	return out;
}

#[derive(Clone)]
struct AddrStamp {
	kind: String,
	addr: String,
	seen: Instant,
}

struct PeerRec {
	host: String,
	info: NodeInfo,
	addrs: Vec<AddrStamp>,
	machine: Option<Machine>,
}

type Registry = Arc<Mutex<HashMap<String, PeerRec>>>;

fn beacon_loop(machine: Option<Arc<Machine>>) {
	let host = hostname();
	let mach_line = machine.as_ref().map(|m| m.beacon_encode());
	loop {
		if pool_deselected().contains(&host) {
			thread::sleep(Duration::from_secs(BEACON_SECS));
			continue;
		}
		let info = NodeInfo::probe();
		for i in ifaces() {
			let bind = SocketAddr::new(IpAddr::V4(i.ip), 0);
			let Ok(s) = UdpSocket::bind(bind) else {
				continue;
			};
			drop(recipe_infer::bridge::broadcast_on(&s));
			let kind = match i.link {
				Link::Wireless => "wlan",
				Link::Wired => "eth",
			};
			let mut body = format!(
				"{host}{NL}{kind}{NL}{PORT}{NL}{}",
				String::from_utf8_lossy(&info.encode())
			);
			for ml in mach_line.as_ref().into_iter() {
				body.push(NL);
				body.push_str(ml);
			}
			let mut buf = BEACON_MAGIC.to_le_bytes().to_vec();
			buf.extend_from_slice(body.as_bytes());
			let dst = SocketAddr::new(IpAddr::V4(i.bcast), PORT);
			drop(s.send_to(&buf, dst));
		}
		thread::sleep(Duration::from_secs(BEACON_SECS));
	}
}

fn listen_loop(reg: Registry, own: Option<Arc<Machine>>) {
	let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), PORT);
	let sock = match UdpSocket::bind(bind) {
		Ok(s) => s,
		Err(e) => {
			Write::error(&format!("discovery listener bind failed: {e}"));
			return;
		}
	};
	let me = hostname();
	let mut buf = [0u8; 2048];
	loop {
		match recv_from(&sock, &mut buf) {
			Ok(r) => {
				let Some(_avail) = r.n.checked_sub(4) else {
					continue;
				};
				let Ordering::Equal = buf[0..4].cmp(BEACON_MAGIC.to_le_bytes().as_slice())
				else {
					continue;
				};
				let Ok(text) = from_utf8(&buf[4..r.n]) else {
					continue;
				};
				let mut it = text.splitn(4, NL);
				let Some(host) = it.next() else { continue };
				let Some(kind) = it.next() else { continue };
				let Some(port) = it.next() else { continue };
				let Some(_ok_host) =
					Some(host).filter(|h| !h.is_empty() && *h != me.as_str())
				else {
					continue;
				};
				let rest = it.next().unwrap_or("");
				let Ok(info) = NodeInfo::decode(rest.as_bytes()) else {
					continue;
				};
				let machine = rest
					.splitn(5, NL)
					.nth(4)
					.and_then(|ml| Machine::beacon_decode(ml).ok());
				let addr = format!("{}:{}", r.from.ip(), port);
				let changed = match reg.lock() {
					Ok(mut g) => {
						let rec =
							g.entry(host.to_string()).or_insert_with(|| PeerRec {
								host: host.to_string(),
								info: info.clone(),
								addrs: Vec::new(),
								machine: None,
							});
						rec.info = info;
						match rec.addrs.iter_mut().find(|e| e.kind == kind) {
							Some(e) => {
								e.addr = addr;
								e.seen = Instant::now();
							}
							None => rec.addrs.push(AddrStamp {
								kind: kind.to_string(),
								addr,
								seen: Instant::now(),
							}),
						}
						let diff = rec.machine != machine;
						match Some(machine).filter(|_moved| diff) {
							Some(moved) => {
								rec.machine = moved;
								Some(())
							}
							None => None,
						}
					}
					Err(p) => {
						Write::error(format!("peer registry poisoned: {p}"));
						None
					}
				};
				for _rw in changed.into_iter() {
					rewrite_config(&reg, &own);
				}
			}
			Err(e) => {
				Write::error(format!("beacon recv: {e}"));
				continue;
			}
		}
	}
}

fn rewrite_config(reg: &Registry, own: &Option<Arc<Machine>>) {
	let mut machines: Vec<Machine> = Vec::new();
	for m in own.iter() {
		machines.push(m.as_ref().clone());
	}
	for g in reg.lock().into_iter() {
		for rec in g.values() {
			for m in rec.machine.iter() {
				machines.push(m.clone());
			}
		}
	}
	for e in crate::machine::write_config_atomic(&machines)
		.err()
		.into_iter()
	{
		Write::error(&format!("config write failed: {e}"));
	}
}

fn encode_peers(reg: &Registry) -> Result<Vec<u8>> {
	let g = reg
		.lock()
		.map_err(|_poison| anyhow::anyhow!("wire: registry poisoned"))?;
	let mut lines = Vec::new();
	for rec in g.values() {
		let mut addrs: Vec<AddrStamp> = rec.addrs.clone();
		addrs.sort_by_key(|e| e.kind != "eth");
		let addrs = addrs
			.iter()
			.map(|e| format!("{}={}@{}", e.kind, e.addr, e.seen.elapsed().as_millis()))
			.collect::<Vec<String>>()
			.join(",");
		lines.push(format!(
			"{}\t{addrs}\t{}\t{}\t{}\t{}",
			rec.host, rec.info.arch, rec.info.gpus, rec.info.vram, rec.info.ram
		));
	}
	Ok(lines.join(NLS).into_bytes())
}

#[derive(Clone, Debug)]
pub struct PeerEntry {
	pub host: String,
	pub addrs: Vec<String>,
	pub info: NodeInfo,
}

fn decode_peers(b: &[u8]) -> Vec<PeerEntry> {
	let mut out = Vec::new();
	for line in String::from_utf8_lossy(b).lines() {
		let f: Vec<&str> = line.split('\t').collect();
		let 6 = f.len() else { continue };
		let addrs: Vec<String> = f[1]
			.split(',')
			.filter_map(|e| {
				let at = e.rfind('@')?;
				let kv = &e[..at];
				let age = &e[at + 1..];
				let eq = kv.find('=')?;
				let addr = &kv[eq + 1..];
				match age.parse::<u128>().ok()?.cmp(&STALE_MS) {
					Ordering::Less => Some(addr.to_string()),
					Ordering::Equal | Ordering::Greater => None,
				}
			})
			.collect();
		let info = NodeInfo {
			arch: f[2].to_string(),
			gpus: f[3].parse().unwrap_or(0),
			vram: f[4].parse().unwrap_or(0),
			ram: f[5].parse().unwrap_or(0),
		};
		let Some(_first_addr) = addrs.first() else {
			continue;
		};
		out.push(PeerEntry {
			host: f[0].to_string(),
			addrs,
			info,
		});
	}
	out
}

pub fn local_peers() -> Result<Vec<PeerEntry>> {
	let c = Conn::connect(&format!("127.0.0.1:{PORT}"))?;
	Ok(decode_peers(&c.call(Op::Peers, 0, 0, Vec::new())?.data))
}

pub fn self_host() -> String {
	hostname()
}

pub fn pool_deselected() -> HashSet<String> {
	let Ok(path) = crate::machine::pool_path() else {
		return HashSet::new();
	};
	let Ok(s) = fs::read_to_string(path) else {
		return HashSet::new();
	};
	s.lines()
		.map(str::trim)
		.filter(|l| !l.is_empty())
		.map(str::to_string)
		.collect()
}

pub fn pool_write(deselected: &[String]) -> Result<()> {
	let path = crate::machine::pool_path()?;
	for parent in path.parent().into_iter() {
		fs::create_dir_all(parent)?;
	}
	let tmp = path.with_extension("ogdl.tmp");
	let mut text = String::new();
	for h in deselected {
		text.push_str(h);
		text.push(NL);
	}
	fs::write(&tmp, text)?;
	fs::rename(&tmp, &path)?;
	Ok(())
}

pub struct Conn {
	pub info: NodeInfo,
	wr: Mutex<TcpStream>,
	pending: Arc<Mutex<HashMap<u32, Sender<Frame>>>>,
	seq: Mutex<u32>,
}

impl Conn {
	pub fn connect(addr: &str) -> Result<Conn> {
		use ToSocketAddrs;
		let sa = addr
			.to_socket_addrs()?
			.next()
			.ok_or_else(|| anyhow::anyhow!("wire: {addr} resolves to nothing"))?;
		let stream =
			TcpStream::connect_timeout(&sa, Duration::from_secs(CONNECT_TIMEOUT_SECS))?;
		recipe_infer::bridge::nodelay_on(&stream)?;
		let reader = stream.try_clone()?;
		let pending: Arc<Mutex<HashMap<u32, Sender<Frame>>>> =
			Arc::new(Mutex::new(HashMap::new()));
		let pend = Arc::clone(&pending);
		thread::spawn(move || {
			let mut r = reader;
			loop {
				match read_frame(&mut r) {
					Ok(f) => {
						let waiter =
							pend.lock().ok().and_then(|mut m| m.remove(&f.seq));
						match waiter {
							Some(tx) => {
								drop(tx.send(f));
							}
							None => continue,
						}
					}
					Err(e) => {
						note_frame_end(&e, "client frame read");
						break;
					}
				}
			}
		});
		let mut c = Conn {
			info: NodeInfo::probe(),
			wr: Mutex::new(stream),
			pending,
			seq: Mutex::new(0),
		};
		let hello = c.call(Op::Hello, 0, 0, Vec::new())?;
		c.info = NodeInfo::decode(&hello.data)?;
		Ok(c)
	}

	fn next_seq(&self) -> Result<u32> {
		let mut g = self
			.seq
			.lock()
			.map_err(|_poison| anyhow::anyhow!("wire: seq poisoned"))?;
		*g = g.wrapping_add(1);
		Ok(*g)
	}

	pub fn send(&self, op: Op, tag: u16, id: u64, data: Vec<u8>) -> Result<Receiver<Frame>> {
		let seq = self.next_seq()?;
		let Chan { tx, rx } = chan::<Frame>();
		self.pending
			.lock()
			.map_err(|_poison| anyhow::anyhow!("wire: pending poisoned"))?
			.insert(seq, tx);
		let mut s = self
			.wr
			.lock()
			.map_err(|_poison| anyhow::anyhow!("wire: writer poisoned"))?;
		write_frame(&mut s, &Frame::new(op, tag, seq, id, data))?;
		Ok(rx)
	}

	pub fn call(&self, op: Op, tag: u16, id: u64, data: Vec<u8>) -> Result<Frame> {
		let f = self.send(op, tag, id, data)?.recv()?;
		ensure!(
			f.op != Op::Err,
			"wire: remote error: {}",
			String::from_utf8_lossy(&f.data)
		);
		Ok(f)
	}

	pub fn store_from(&self, id: u64, data: &[u8]) -> Result<()> {
		let seq = self.next_seq()?;
		let Chan { tx, rx } = chan::<Frame>();
		self.pending
			.lock()
			.map_err(|_poison| anyhow::anyhow!("wire: pending poisoned"))?
			.insert(seq, tx);
		{
			let mut s = self
				.wr
				.lock()
				.map_err(|_poison| anyhow::anyhow!("wire: writer poisoned"))?;
			write_frame_raw(&mut s, Op::Store, 0, 0, seq, id, data)?;
		}
		let f = rx.recv()?;
		ensure!(
			f.op != Op::Err,
			"wire: remote error: {}",
			String::from_utf8_lossy(&f.data)
		);
		Ok(())
	}
	pub fn run(&self, fn_id: u16, id: u64, payload: Vec<u8>) -> Result<Receiver<Frame>> {
		self.send(Op::Run, fn_id, id, payload)
	}
	pub fn fetch(&self, id: u64, off: u64, len: u64) -> Result<Vec<u8>> {
		let mut p = Vec::with_capacity(16);
		p.extend_from_slice(&off.to_le_bytes());
		p.extend_from_slice(&len.to_le_bytes());
		Ok(self.call(Op::Fetch, 0, id, p)?.data)
	}
	pub fn free(&self, id: u64) -> Result<()> {
		self.call(Op::Free, 0, id, Vec::new()).map(|_frame| ())
	}
	pub fn stat(&self) -> Result<String> {
		Ok(
			String::from_utf8_lossy(&self.call(Op::Stat, 0, 0, Vec::new())?.data)
				.into_owned(),
		)
	}
}

pub type RunFn = Arc<dyn Fn(u64, &[u8]) -> Result<Vec<u8>> + Send + Sync>;

type Store = Arc<Mutex<HashMap<u64, Vec<u8>>>>;

#[derive(Clone)]
pub struct Server {
	info: NodeInfo,
	store: Store,
	runners: Arc<HashMap<u16, RunFn>>,
	reg: Registry,
	machine: Option<Arc<Machine>>,
	jobs: Arc<Mutex<()>>,
}

impl Server {
	pub fn new(info: NodeInfo, runners: HashMap<u16, RunFn>) -> Server {
		Server {
			info,
			store: Arc::new(Mutex::new(HashMap::new())),
			runners: Arc::new(runners),
			reg: Arc::new(Mutex::new(HashMap::new())),
			machine: None,
			jobs: Arc::new(Mutex::new(())),
		}
	}

	pub fn machine(mut self, m: Machine) -> Server {
		self.machine = Some(Arc::new(m));
		self
	}

	pub fn bind(addr: impl ToSocketAddrs + fmt::Debug) -> Result<TcpListener> {
		match TcpListener::bind(&addr) {
			Ok(l) => Ok(l),
			Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
				anyhow::bail!("recipe already running ({addr:?})")
			}
			Err(e) => Err(e.into()),
		}
	}

	pub fn serve(self, addr: &str) -> Result<()> {
		self.serve_bound(Self::bind(addr)?)
	}

	pub fn serve_bound(self, listener: TcpListener) -> Result<()> {
		let machine = self.machine.clone();
		for m in machine.iter() {
			for e in crate::machine::write_config_atomic(slice::from_ref(m.as_ref()))
				.err()
				.into_iter()
			{
				Write::error(&format!("config write failed: {e}"));
			}
		}
		let bm = machine.clone();
		thread::spawn(move || beacon_loop(bm));
		let reg = Arc::clone(&self.reg);
		let lm = machine.clone();
		thread::spawn(move || listen_loop(reg, lm));
		Write::line(
			net,
			&format!(
				"{} ({}) on {} (ram {} MiB)",
				self.info.arch,
				hostname(),
				listener
					.local_addr()
					.map(|a| a.to_string())
					.unwrap_or_else(|_addr_err| "?".into()),
				self.info.ram >> 20
			),
		);
		self.serve_on(listener)
	}

	pub fn serve_on(self, listener: TcpListener) -> Result<()> {
		for stream in listener.incoming() {
			let stream = stream?;
			let srv = self.clone();
			thread::spawn(move || {
				for e in srv.handle(stream).err().into_iter() {
					Write::error(&format!("connection ended: {e}"));
				}
			});
		}
		Ok(())
	}

	fn handle(&self, mut s: TcpStream) -> Result<()> {
		recipe_infer::bridge::nodelay_on(&s)?;
		loop {
			let f = match read_frame(&mut s) {
				Ok(f) => f,
				Err(e) => {
					note_frame_end(&e, "serve frame read");
					return Ok(());
				}
			};
			let reply = self.dispatch(&f);
			let out = match reply {
				Ok(data) => Frame::new(Op::Reply, f.tag, f.seq, f.id, data),
				Err(e) => {
					Frame::new(Op::Err, f.tag, f.seq, f.id, e.to_string().into_bytes())
				}
			};
			write_frame(&mut s, &out)?;
		}
	}

	fn dispatch(&self, f: &Frame) -> Result<Vec<u8>> {
		match f.op {
			Op::Hello => Ok(self.info.encode()),
			Op::Store => {
				self.store
					.lock()
					.map_err(|_poison| anyhow::anyhow!("wire: store poisoned"))?
					.insert(f.id, f.data.clone());
				Ok(Vec::new())
			}
			Op::Fetch => {
				let off = u64::from_le_bytes(f.data[0..8].try_into()?) as usize;
				let len = u64::from_le_bytes(f.data[8..16].try_into()?) as usize;
				let g = self
					.store
					.lock()
					.map_err(|_poison| anyhow::anyhow!("wire: store poisoned"))?;
				let blob = g
					.get(&f.id)
					.ok_or_else(|| anyhow::anyhow!("wire: no id {}", f.id))?;
				ensure!(
					off + len <= blob.len(),
					"wire: fetch {off}+{len} past {} for id {}",
					blob.len(),
					f.id
				);
				Ok(blob[off..off + len].to_vec())
			}
			Op::Run => {
				let h = self
					.runners
					.get(&f.tag)
					.ok_or_else(|| anyhow::anyhow!("wire: no runner for fn {}", f.tag))?;
				let _queued = self
					.jobs
					.lock()
					.map_err(|_poison| anyhow::anyhow!("wire: job queue poisoned"))?;
				let _gpu = gpu_core::gate::Lease::new();
				h(f.id, &f.data)
			}
			Op::Stat => {
				let g = self
					.store
					.lock()
					.map_err(|_poison| anyhow::anyhow!("wire: store poisoned"))?;
				let bytes: usize = g.values().map(Vec::len).sum();
				Ok(format!(
					"{}: {} blobs, {} MiB stored",
					self.info.arch,
					g.len(),
					bytes >> 20
				)
				.into_bytes())
			}
			Op::Peers => encode_peers(&self.reg),
			Op::Free => {
				self.store
					.lock()
					.map_err(|_poison| anyhow::anyhow!("wire: store poisoned"))?
					.remove(&f.id);
				Ok(Vec::new())
			}
			op @ (Op::Reply | Op::Err) => bail!("wire: server got client-only op {op:?}"),
		}
	}
}

pub struct Net {
	nodes: Vec<String>,
}

impl Default for Net {
	fn default() -> Self {
		Net::new()
	}
}

impl Net {
	pub fn new() -> Net {
		Net { nodes: Vec::new() }
	}
	pub fn node(mut self, alias: &str) -> Net {
		self.nodes.push(alias.to_string());
		self
	}

	pub fn connect(&self) -> Result<Vec<Conn>> {
		let peers = local_peers().unwrap_or_default();
		self.nodes
			.iter()
			.map(|n| connect_first(n, &candidates(n, &peers)?))
			.collect()
	}

	pub fn all() -> Result<Vec<Conn>> {
		let peers = local_peers().map_err(|e| {
			anyhow::anyhow!("wire: no local daemon for discovery (recipe serve): {e}")
		})?;
		ensure!(!peers.is_empty(), "wire: local daemon hears no peers");
		let off = pool_deselected();
		let picked: Vec<&PeerEntry> = peers.iter().filter(|p| !off.contains(&p.host)).collect();
		ensure!(
			!picked.is_empty(),
			"wire: every live peer is deselected from the pool (recipe peers)"
		);
		picked.iter()
			.map(|p| connect_first(&p.host, &p.addrs))
			.collect()
	}
}

fn connect_first(name: &str, addrs: &[String]) -> Result<Conn> {
	let mut errs = Vec::new();
	for a in addrs {
		match Conn::connect(a) {
			Ok(c) => return Ok(c),
			Err(e) => errs.push(format!("{a}: {e}")),
		}
	}
	bail!("node {name}: no address reachable [{}]", errs.join("; "))
}

fn candidates(alias: &str, peers: &[PeerEntry]) -> Result<Vec<String>> {
	let None = alias.find(':') else {
		return Ok(vec![alias.to_string()]);
	};
	let ssh = process::Command::new("ssh").args(["-G", alias]).output()?;
	let text = String::from_utf8_lossy(&ssh.stdout);
	let host = text
		.lines()
		.find_map(|l| l.strip_prefix("hostname "))
		.map(str::trim)
		.filter(|h| !h.is_empty())
		.unwrap_or(alias);
	ensure!(
		host != alias,
		"wire: '{alias}' is not an ssh alias — add to ~/.ssh/config:{NL}  Host {alias}{NL}      HostName <address>{NL}with keys so `ssh {alias}` works, then retry"
	);
	let ssh_addr = format!("{host}:{PORT}");
	match peers
		.iter()
		.find(|p| p.host == alias || p.addrs.contains(&ssh_addr))
	{
		Some(p) => {
			let mut out = p.addrs.clone();
			out.extend(Some(ssh_addr).filter(|s| !p.addrs.contains(s)));
			Ok(out)
		}
		None => Ok(vec![ssh_addr]),
	}
}

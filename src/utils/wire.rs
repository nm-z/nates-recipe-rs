use crate::probe::Machine;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAGIC: u32 = 0x5243_5031;
pub const PORT: u16 = 7845;
const HDR: usize = 32;

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
		Ok(match b {
			1 => Op::Hello,
			2 => Op::Store,
			3 => Op::Run,
			4 => Op::Fetch,
			5 => Op::Stat,
			6 => Op::Reply,
			7 => Op::Err,
			8 => Op::Peers,
			9 => Op::Free,
			_ => bail!("wire: bad op byte {b}"),
		})
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
		Frame { op, flags: 0, tag, seq, id, data }
	}
}

fn write_frame_raw(s: &mut TcpStream, op: Op, flags: u8, tag: u16, seq: u32, id: u64, data: &[u8]) -> Result<()> {
	let mut h = [0u8; HDR];
	h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
	h[4] = op as u8;
	h[5] = flags;
	h[6..8].copy_from_slice(&tag.to_le_bytes());
	h[8..12].copy_from_slice(&seq.to_le_bytes());
	h[12..20].copy_from_slice(&id.to_le_bytes());
	h[20..28].copy_from_slice(&(data.len() as u64).to_le_bytes());
	s.write_all(&h)?;
	if !data.is_empty() {
		s.write_all(data)?;
	}
	s.flush()?;
	Ok(())
}

fn write_frame(s: &mut TcpStream, f: &Frame) -> Result<()> {
	write_frame_raw(s, f.op, f.flags, f.tag, f.seq, f.id, &f.data)
}

fn read_frame(s: &mut TcpStream) -> Result<Frame> {
	let mut h = [0u8; HDR];
	s.read_exact(&mut h)?;
	let magic = u32::from_le_bytes(h[0..4].try_into()?);
	if magic != MAGIC {
		bail!("wire: bad magic {magic:#010x}");
	}
	let op = Op::from(h[4])?;
	let flags = h[5];
	let tag = u16::from_le_bytes(h[6..8].try_into()?);
	let seq = u32::from_le_bytes(h[8..12].try_into()?);
	let id = u64::from_le_bytes(h[12..20].try_into()?);
	let len = u64::from_le_bytes(h[20..28].try_into()?) as usize;
	let mut data = vec![0u8; len];
	s.read_exact(&mut data)?;
	Ok(Frame { op, flags, tag, seq, id, data })
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
		let arch = std::env::var("GPU_ARCH").unwrap_or_else(|_| "storage".to_string());
		NodeInfo { arch, gpus: 0, vram: 0, ram: mem_available() }
	}
	fn encode(&self) -> Vec<u8> {
		format!("{}\n{}\n{}\n{}", self.arch, self.gpus, self.vram, self.ram).into_bytes()
	}
	fn decode(b: &[u8]) -> Result<NodeInfo> {
		let s = std::str::from_utf8(b)?;
		let mut it = s.split('\n');
		let arch = it.next().unwrap_or("").to_string();
		let gpus = it.next().unwrap_or("0").parse()?;
		let vram = it.next().unwrap_or("0").parse()?;
		let ram = it.next().unwrap_or("0").parse()?;
		Ok(NodeInfo { arch, gpus, vram, ram })
	}
}

fn mem_available() -> u64 {
	let Ok(s) = std::fs::read_to_string("/proc/meminfo") else {
		return 0;
	};
	for l in s.lines() {
		if let Some(kb) = l.strip_prefix("MemAvailable:") {
			let kb: u64 = kb.trim().trim_end_matches(" kB").parse().unwrap_or(0);
			return kb * 1024;
		}
	}
	0
}

fn hostname() -> String {
	std::fs::read_to_string("/proc/sys/kernel/hostname")
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

struct Iface {
	ip: std::net::Ipv4Addr,
	bcast: std::net::Ipv4Addr,
	wireless: bool,
}

fn ifaces() -> Vec<Iface> {
	let mut out = Vec::new();
	unsafe {
		let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
		if libc::getifaddrs(&mut ifap) != 0 {
			return out;
		}
		let mut p = ifap;
		while !p.is_null() {
			let a = &*p;
			let up = a.ifa_flags & (libc::IFF_UP | libc::IFF_RUNNING) as u32
				== (libc::IFF_UP | libc::IFF_RUNNING) as u32;
			let lo = a.ifa_flags & libc::IFF_LOOPBACK as u32 != 0;
			if up && !lo && !a.ifa_addr.is_null() && (*a.ifa_addr).sa_family as i32 == libc::AF_INET {
				let sin = &*(a.ifa_addr as *const libc::sockaddr_in);
				let ip = u32::from_be(sin.sin_addr.s_addr);
				let mask = if a.ifa_netmask.is_null() {
					0
				} else {
					u32::from_be((*(a.ifa_netmask as *const libc::sockaddr_in)).sin_addr.s_addr)
				};
				let name = std::ffi::CStr::from_ptr(a.ifa_name).to_string_lossy().into_owned();
				let wireless = std::path::Path::new(&format!("/sys/class/net/{name}/wireless")).exists();
				out.push(Iface {
					ip: std::net::Ipv4Addr::from(ip),
					bcast: std::net::Ipv4Addr::from(ip | !mask),
					wireless,
				});
			}
			p = a.ifa_next;
		}
		libc::freeifaddrs(ifap);
	}
	out
}

struct PeerRec {
	info: NodeInfo,
	addrs: HashMap<String, (String, std::time::Instant)>,
	machine: Option<Machine>,
}

type Registry = Arc<Mutex<HashMap<String, PeerRec>>>;

fn beacon_loop(machine: Option<Arc<Machine>>) {
	let host = hostname();
	let mach_line = machine.as_ref().map(|m| m.beacon_encode());
	loop {
		let info = NodeInfo::probe();
		for i in ifaces() {
			let Ok(s) = std::net::UdpSocket::bind((i.ip, 0)) else { continue };
			let _ = s.set_broadcast(true);
			let kind = if i.wireless { "wlan" } else { "eth" };
			let mut body =
				format!("{host}\n{kind}\n{PORT}\n{}", String::from_utf8_lossy(&info.encode()));
			if let Some(ml) = &mach_line {
				body.push('\n');
				body.push_str(ml);
			}
			let mut buf = BEACON_MAGIC.to_le_bytes().to_vec();
			buf.extend_from_slice(body.as_bytes());
			let _ = s.send_to(&buf, (i.bcast, PORT));
		}
		thread::sleep(std::time::Duration::from_secs(BEACON_SECS));
	}
}

fn listen_loop(reg: Registry, own: Option<Arc<Machine>>) {
	let sock = match std::net::UdpSocket::bind(("0.0.0.0", PORT)) {
		Ok(s) => s,
		Err(e) => {
			eprintln!("recipe serve: discovery listener bind failed: {e}");
			return;
		}
	};
	let me = hostname();
	let mut buf = [0u8; 2048];
	loop {
		let Ok((n, from)) = sock.recv_from(&mut buf) else { continue };
		if n < 4 || buf[0..4] != BEACON_MAGIC.to_le_bytes() {
			continue;
		}
		let Ok(text) = std::str::from_utf8(&buf[4..n]) else { continue };
		let mut it = text.splitn(4, '\n');
		let (Some(host), Some(kind), Some(port)) = (it.next(), it.next(), it.next()) else {
			continue;
		};
		if host == me || host.is_empty() {
			continue;
		}
		let rest = it.next().unwrap_or("");
		let Ok(info) = NodeInfo::decode(rest.as_bytes()) else { continue };
		let machine = rest.splitn(5, '\n').nth(4).and_then(|ml| Machine::beacon_decode(ml).ok());
		let addr = format!("{}:{}", from.ip(), port);
		let mut changed = false;
		if let Ok(mut g) = reg.lock() {
			let rec = g.entry(host.to_string()).or_insert_with(|| PeerRec {
				info: info.clone(),
				addrs: HashMap::new(),
				machine: None,
			});
			rec.info = info;
			rec.addrs.insert(kind.to_string(), (addr, std::time::Instant::now()));
			if rec.machine != machine {
				rec.machine = machine;
				changed = true;
			}
		}
		if changed {
			rewrite_config(&reg, &own);
		}
	}
}

fn rewrite_config(reg: &Registry, own: &Option<Arc<Machine>>) {
	let mut machines: Vec<Machine> = Vec::new();
	if let Some(m) = own {
		machines.push(m.as_ref().clone());
	}
	if let Ok(g) = reg.lock() {
		for rec in g.values() {
			if let Some(m) = &rec.machine {
				machines.push(m.clone());
			}
		}
	}
	if let Err(e) = crate::probe::write_config_atomic(&machines) {
		eprintln!("recipe serve: config write failed: {e}");
	}
}

fn encode_peers(reg: &Registry) -> Result<Vec<u8>> {
	let g = reg.lock().map_err(|_| anyhow::anyhow!("wire: registry poisoned"))?;
	let mut lines = Vec::new();
	for (host, rec) in g.iter() {
		let mut addrs: Vec<_> = rec
			.addrs
			.iter()
			.map(|(k, (a, t))| (k.clone(), a.clone(), t.elapsed().as_millis()))
			.collect();
		addrs.sort_by_key(|(k, _, _)| k != "eth");
		let addrs =
			addrs.iter().map(|(k, a, ms)| format!("{k}={a}@{ms}")).collect::<Vec<_>>().join(",");
		lines.push(format!(
			"{host}\t{addrs}\t{}\t{}\t{}\t{}",
			rec.info.arch, rec.info.gpus, rec.info.vram, rec.info.ram
		));
	}
	Ok(lines.join("\n").into_bytes())
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
		if f.len() != 6 {
			continue;
		}
		let addrs: Vec<String> = f[1]
			.split(',')
			.filter_map(|e| {
				let (kv, age) = e.rsplit_once('@')?;
				let (_, addr) = kv.split_once('=')?;
				(age.parse::<u128>().ok()? < STALE_MS).then(|| addr.to_string())
			})
			.collect();
		let info = NodeInfo {
			arch: f[2].to_string(),
			gpus: f[3].parse().unwrap_or(0),
			vram: f[4].parse().unwrap_or(0),
			ram: f[5].parse().unwrap_or(0),
		};
		if !addrs.is_empty() {
			out.push(PeerEntry { host: f[0].to_string(), addrs, info });
		}
	}
	out
}

pub fn local_peers() -> Result<Vec<PeerEntry>> {
	let c = Conn::connect(&format!("127.0.0.1:{PORT}"))?;
	Ok(decode_peers(&c.call(Op::Peers, 0, 0, Vec::new())?.data))
}

pub struct Conn {
	pub info: NodeInfo,
	wr: Mutex<TcpStream>,
	pending: Arc<Mutex<HashMap<u32, Sender<Frame>>>>,
	seq: Mutex<u32>,
}

impl Conn {
	pub fn connect(addr: &str) -> Result<Conn> {
		use std::net::ToSocketAddrs;
		let sa = addr
			.to_socket_addrs()?
			.next()
			.ok_or_else(|| anyhow::anyhow!("wire: {addr} resolves to nothing"))?;
		let stream =
			TcpStream::connect_timeout(&sa, std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS))?;
		stream.set_nodelay(true)?;
		let reader = stream.try_clone()?;
		let pending: Arc<Mutex<HashMap<u32, Sender<Frame>>>> = Arc::new(Mutex::new(HashMap::new()));
		let pend = Arc::clone(&pending);
		thread::spawn(move || {
			let mut r = reader;
			loop {
				match read_frame(&mut r) {
					Ok(f) => {
						let waiter = pend.lock().ok().and_then(|mut m| m.remove(&f.seq));
						if let Some(tx) = waiter {
							let _ = tx.send(f);
						}
					}
					Err(_) => break,
				}
			}
		});
		let mut c = Conn { info: NodeInfo::probe(), wr: Mutex::new(stream), pending, seq: Mutex::new(0) };
		let hello = c.call(Op::Hello, 0, 0, Vec::new())?;
		c.info = NodeInfo::decode(&hello.data)?;
		Ok(c)
	}

	fn next_seq(&self) -> Result<u32> {
		let mut g = self.seq.lock().map_err(|_| anyhow::anyhow!("wire: seq poisoned"))?;
		*g = g.wrapping_add(1);
		Ok(*g)
	}

	pub fn send(&self, op: Op, tag: u16, id: u64, data: Vec<u8>) -> Result<Receiver<Frame>> {
		let seq = self.next_seq()?;
		let (tx, rx) = channel();
		self.pending
			.lock()
			.map_err(|_| anyhow::anyhow!("wire: pending poisoned"))?
			.insert(seq, tx);
		let mut s = self.wr.lock().map_err(|_| anyhow::anyhow!("wire: writer poisoned"))?;
		write_frame(&mut s, &Frame::new(op, tag, seq, id, data))?;
		Ok(rx)
	}

	pub fn call(&self, op: Op, tag: u16, id: u64, data: Vec<u8>) -> Result<Frame> {
		let f = self.send(op, tag, id, data)?.recv()?;
		if f.op == Op::Err {
			bail!("wire: remote error: {}", String::from_utf8_lossy(&f.data));
		}
		Ok(f)
	}

	pub fn store_from(&self, id: u64, data: &[u8]) -> Result<()> {
		let seq = self.next_seq()?;
		let (tx, rx) = channel();
		self.pending
			.lock()
			.map_err(|_| anyhow::anyhow!("wire: pending poisoned"))?
			.insert(seq, tx);
		{
			let mut s = self.wr.lock().map_err(|_| anyhow::anyhow!("wire: writer poisoned"))?;
			write_frame_raw(&mut s, Op::Store, 0, 0, seq, id, data)?;
		}
		let f = rx.recv()?;
		if f.op == Op::Err {
			bail!("wire: remote error: {}", String::from_utf8_lossy(&f.data));
		}
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
		self.call(Op::Free, 0, id, Vec::new()).map(|_| ())
	}
	pub fn stat(&self) -> Result<String> {
		Ok(String::from_utf8_lossy(&self.call(Op::Stat, 0, 0, Vec::new())?.data).into_owned())
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

	pub fn serve(self, addr: &str) -> Result<()> {
		self.serve_bound(TcpListener::bind(addr)?)
	}

	pub fn serve_bound(self, listener: TcpListener) -> Result<()> {
		let machine = self.machine.clone();
		if let Some(m) = &machine {
			if let Err(e) = crate::probe::write_config_atomic(std::slice::from_ref(m.as_ref())) {
				eprintln!("recipe serve: config write failed: {e}");
			}
		}
		let bm = machine.clone();
		thread::spawn(move || beacon_loop(bm));
		let reg = Arc::clone(&self.reg);
		let lm = machine.clone();
		thread::spawn(move || listen_loop(reg, lm));
		eprintln!(
			"recipe serve: {} ({}) on {} (ram {} MiB)",
			self.info.arch,
			hostname(),
			listener.local_addr().map(|a| a.to_string()).unwrap_or_else(|_| "?".into()),
			self.info.ram >> 20
		);
		self.serve_on(listener)
	}

	pub fn serve_on(self, listener: TcpListener) -> Result<()> {
		for stream in listener.incoming() {
			let stream = stream?;
			let srv = self.clone();
			thread::spawn(move || {
				if let Err(e) = srv.handle(stream) {
					eprintln!("recipe serve: connection ended: {e}");
				}
			});
		}
		Ok(())
	}

	fn handle(&self, mut s: TcpStream) -> Result<()> {
		s.set_nodelay(true)?;
		loop {
			let f = match read_frame(&mut s) {
				Ok(f) => f,
				Err(_) => return Ok(()),
			};
			let reply = self.dispatch(&f);
			let out = match reply {
				Ok(data) => Frame::new(Op::Reply, f.tag, f.seq, f.id, data),
				Err(e) => Frame::new(Op::Err, f.tag, f.seq, f.id, e.to_string().into_bytes()),
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
					.map_err(|_| anyhow::anyhow!("wire: store poisoned"))?
					.insert(f.id, f.data.clone());
				Ok(Vec::new())
			}
			Op::Fetch => {
				let off = u64::from_le_bytes(f.data[0..8].try_into()?) as usize;
				let len = u64::from_le_bytes(f.data[8..16].try_into()?) as usize;
				let g = self.store.lock().map_err(|_| anyhow::anyhow!("wire: store poisoned"))?;
				let blob = g.get(&f.id).ok_or_else(|| anyhow::anyhow!("wire: no id {}", f.id))?;
				if off + len > blob.len() {
					bail!("wire: fetch {off}+{len} past {} for id {}", blob.len(), f.id);
				}
				Ok(blob[off..off + len].to_vec())
			}
			Op::Run => {
				let h = self
					.runners
					.get(&f.tag)
					.ok_or_else(|| anyhow::anyhow!("wire: no runner for fn {}", f.tag))?;
				let _queued =
					self.jobs.lock().map_err(|_| anyhow::anyhow!("wire: job queue poisoned"))?;
				let _gpu = gpu_core::gate::Lease::new();
				h(f.id, &f.data)
			}
			Op::Stat => {
				let g = self.store.lock().map_err(|_| anyhow::anyhow!("wire: store poisoned"))?;
				let bytes: usize = g.values().map(Vec::len).sum();
				Ok(format!("{}: {} blobs, {} MiB stored", self.info.arch, g.len(), bytes >> 20)
					.into_bytes())
			}
			Op::Peers => encode_peers(&self.reg),
			Op::Free => {
				self.store
					.lock()
					.map_err(|_| anyhow::anyhow!("wire: store poisoned"))?
					.remove(&f.id);
				Ok(Vec::new())
			}
			other => bail!("wire: server got client-only op {other:?}"),
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
		self.nodes.iter().map(|n| connect_first(n, &candidates(n, &peers)?)).collect()
	}

	pub fn all() -> Result<Vec<Conn>> {
		let peers = local_peers()
			.map_err(|e| anyhow::anyhow!("wire: no local daemon for discovery (recipe serve): {e}"))?;
		if peers.is_empty() {
			bail!("wire: local daemon hears no peers");
		}
		peers.iter().map(|p| connect_first(&p.host, &p.addrs)).collect()
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
	if alias.contains(':') {
		return Ok(vec![alias.to_string()]);
	}
	let ssh = std::process::Command::new("ssh").args(["-G", alias]).output()?;
	let text = String::from_utf8_lossy(&ssh.stdout);
	let host = text
		.lines()
		.find_map(|l| l.strip_prefix("hostname "))
		.map(str::trim)
		.filter(|h| !h.is_empty())
		.unwrap_or(alias);
	if host == alias {
		bail!(
			"wire: '{alias}' is not an ssh alias — add to ~/.ssh/config:\n  Host {alias}\n      HostName <address>\nwith keys so `ssh {alias}` works, then retry"
		);
	}
	let ssh_addr = format!("{host}:{PORT}");
	if let Some(p) = peers.iter().find(|p| p.host == alias || p.addrs.contains(&ssh_addr)) {
		let mut out = p.addrs.clone();
		if !out.contains(&ssh_addr) {
			out.push(ssh_addr);
		}
		return Ok(out);
	}
	Ok(vec![ssh_addr])
}

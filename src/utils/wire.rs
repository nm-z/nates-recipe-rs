// ── wire.rs — minimal TCP RPC for distributed inference ──────────────────────
// llama.cpp's rpc-server splits vertically (one machine busy at a time) and
// blocks per-op. This splits horizontally: the coordinator ships weights ONCE
// (STORE, bf16 on the wire), then fires RUN frames for remotely-homed experts
// back-to-back without awaiting replies while the local GPU computes its own —
// local trunk ∥ remote experts ∥ duplex wire, nobody idle. The RUN handler is
// a caller-registered closure, so this module carries zero GPU deps; the gemma
// expert path registers MOE_FFN into the same map.
//
// Frame: 32-byte little-endian header + raw payload.
//   magic u32 | op u8 | flags u8 | tag u16 | seq u32 | id u64 | len u64 | rsvd u32
// Weights ride as bf16 (the on-disk format); widening to f64 happens wherever
// the compute lands, so the wire never carries an f64 weight. All math stays f64.

use anyhow::{bail, Result};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub const MAGIC: u32 = 0x5243_5031; // "RCP1"
pub const PORT: u16 = 7845;
const HDR: usize = 32;

// fn_id values carried in `tag` for RUN. Registered handlers key off these.
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

fn write_frame(s: &mut TcpStream, f: &Frame) -> Result<()> {
	let mut h = [0u8; HDR];
	h[0..4].copy_from_slice(&MAGIC.to_le_bytes());
	h[4] = f.op as u8;
	h[5] = f.flags;
	h[6..8].copy_from_slice(&f.tag.to_le_bytes());
	h[8..12].copy_from_slice(&f.seq.to_le_bytes());
	h[12..20].copy_from_slice(&f.id.to_le_bytes());
	h[20..28].copy_from_slice(&(f.data.len() as u64).to_le_bytes());
	// one syscall for the small frames; payload follows uncopied
	s.write_all(&h)?;
	if !f.data.is_empty() {
		s.write_all(&f.data)?;
	}
	s.flush()?;
	Ok(())
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

// ── node capability descriptor (HELLO payload) ──────────────────────────────
#[derive(Clone, Debug)]
pub struct NodeInfo {
	pub arch: String,
	pub gpus: u32,
	pub vram: u64,
	pub ram: u64,
}

impl NodeInfo {
	// This node's own capabilities. RAM is MemAvailable now; a storage-only
	// node reports gpus=0/vram=0 (genuinely none, not a papered-over default).
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

// ── client connection ───────────────────────────────────────────────────────
// One TCP connection per node. A dedicated reader thread parses replies and
// routes each to the waiter registered under its seq, so RUN frames pipeline:
// fire many, collect out of order, accumulate in expert-index order after.
pub struct Conn {
	pub info: NodeInfo,
	wr: Mutex<TcpStream>,
	pending: Arc<Mutex<HashMap<u32, Sender<Frame>>>>,
	seq: Mutex<u32>,
}

impl Conn {
	pub fn connect(addr: &str) -> Result<Conn> {
		let stream = TcpStream::connect(addr)?;
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
					Err(_) => break, // peer closed
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

	// Fire a frame; return a receiver for its reply (pipelined, non-blocking).
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

	// Fire and block for the reply. Err frames surface as a hard error.
	pub fn call(&self, op: Op, tag: u16, id: u64, data: Vec<u8>) -> Result<Frame> {
		let f = self.send(op, tag, id, data)?.recv()?;
		if f.op == Op::Err {
			bail!("wire: remote error: {}", String::from_utf8_lossy(&f.data));
		}
		Ok(f)
	}

	pub fn store(&self, id: u64, bytes: Vec<u8>) -> Result<()> {
		self.call(Op::Store, 0, id, bytes).map(|_| ())
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
	pub fn stat(&self) -> Result<String> {
		Ok(String::from_utf8_lossy(&self.call(Op::Stat, 0, 0, Vec::new())?.data).into_owned())
	}
}

// ── server ──────────────────────────────────────────────────────────────────
pub type RunFn = Arc<dyn Fn(u64, &[u8]) -> Result<Vec<u8>> + Send + Sync>;

// Remote backing store. Today a RAM map (the remote-RAM tier, one hop below
// local disk on 1GbE); a compute node routes STORE through its own Waterfall so
// the recursive VRAM→RAM→DISK homing applies on the far side too.
type Store = Arc<Mutex<HashMap<u64, Vec<u8>>>>;

#[derive(Clone)]
pub struct Server {
	info: NodeInfo,
	store: Store,
	runners: Arc<HashMap<u16, RunFn>>,
}

impl Server {
	pub fn new(info: NodeInfo, runners: HashMap<u16, RunFn>) -> Server {
		Server { info, store: Arc::new(Mutex::new(HashMap::new())), runners: Arc::new(runners) }
	}

	pub fn serve(self, addr: &str) -> Result<()> {
		let listener = TcpListener::bind(addr)?;
		eprintln!("recipe serve: {} on {addr} (ram {} MiB)", self.info.arch, self.info.ram >> 20);
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
				Err(_) => return Ok(()), // peer closed
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
				h(f.id, &f.data)
			}
			Op::Stat => {
				let g = self.store.lock().map_err(|_| anyhow::anyhow!("wire: store poisoned"))?;
				let bytes: usize = g.values().map(Vec::len).sum();
				Ok(format!("{}: {} blobs, {} MiB stored", self.info.arch, g.len(), bytes >> 20)
					.into_bytes())
			}
			other => bail!("wire: server got client-only op {other:?}"),
		}
	}
}

// ── user API ────────────────────────────────────────────────────────────────
// let net = Net::new().node("archy").node("sentry");
// Aliases resolve through the user's ssh config (`ssh -G`), so the same names
// that work for `ssh archy` work here — no IPs in code.
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

	// HELLO every node; a node that fails to answer fails the run loudly (no
	// silent drop — a distributed run with a missing member is a bug).
	pub fn connect(&self) -> Result<Vec<Conn>> {
		self.nodes
			.iter()
			.map(|n| {
				let addr = resolve(n)?;
				Conn::connect(&addr).map_err(|e| anyhow::anyhow!("node {n} ({addr}): {e}"))
			})
			.collect()
	}
}

fn resolve(alias: &str) -> Result<String> {
	if alias.contains(':') {
		return Ok(alias.to_string());
	}
	// `ssh -G <alias>` prints the effective config; take its hostname line.
	let out = std::process::Command::new("ssh").args(["-G", alias]).output()?;
	let text = String::from_utf8_lossy(&out.stdout);
	let host = text
		.lines()
		.find_map(|l| l.strip_prefix("hostname "))
		.map(str::trim)
		.filter(|h| !h.is_empty())
		.unwrap_or(alias);
	Ok(format!("{host}:{PORT}"))
}

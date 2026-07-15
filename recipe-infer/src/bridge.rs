use std::fs;
use std::io;
use std::net;
use std::path::Path;
use std::sync::mpsc;
pub struct Chan<T> {
	pub tx: mpsc::Sender<T>,
	pub rx: mpsc::Receiver<T>,
}

pub fn chan<T>() -> Chan<T> {
	let (tx, rx) = mpsc::channel();
	Chan { tx, rx }
}

pub struct Recv {
	pub n: usize,
	pub from: net::SocketAddr,
}

pub fn recv_from(sock: &net::UdpSocket, buf: &mut [u8]) -> io::Result<Recv> {
	let (n, from) = sock.recv_from(buf)?;
	Ok(Recv { n, from })
}

pub type Pt = (f64, f64);

pub fn pt(x: f64, y: f64) -> Pt {
	(x, y)
}

pub fn open_rw(path: &Path) -> io::Result<fs::File> {
	fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(path)
}

pub fn broadcast_on(s: &net::UdpSocket) -> io::Result<()> {
	s.set_broadcast(true)
}

pub fn nodelay_on(s: &net::TcpStream) -> io::Result<()> {
	s.set_nodelay(true)
}

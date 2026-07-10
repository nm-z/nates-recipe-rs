pub struct Chan<T> {
	pub tx: std::sync::mpsc::Sender<T>,
	pub rx: std::sync::mpsc::Receiver<T>,
}

pub fn chan<T>() -> Chan<T> {
	let (tx, rx) = std::sync::mpsc::channel();
	Chan { tx, rx }
}

pub struct Recv {
	pub n: usize,
	pub from: std::net::SocketAddr,
}

pub fn recv_from(sock: &std::net::UdpSocket, buf: &mut [u8]) -> std::io::Result<Recv> {
	let (n, from) = sock.recv_from(buf)?;
	Ok(Recv { n, from })
}

pub type Pt = (f64, f64);

pub fn pt(x: f64, y: f64) -> Pt {
	(x, y)
}

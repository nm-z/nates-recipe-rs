#[derive(Clone, Copy, Default)]
pub struct Work {
	pub flop: f64,
	pub bytes: f64,
}

impl Work {
	pub fn add(&mut self, flop: f64, bytes: f64) {
		self.flop += flop;
		self.bytes += bytes;
	}
	pub fn plus(mut self, o: Work) -> Work {
		self.add(o.flop, o.bytes);
		self
	}
}

use super::*;
pub trait Show {
	fn take(self) -> String;
}
#[inline]
pub fn line(f: Flag, t: impl Display) {
	log(&t);
	if on(f) {
		print(&t);
	}
}

impl Show for &str {
	#[inline]
	fn take(self) -> String {
		return self.to_owned();
	}
}

impl Show for String {
	#[inline]
	fn take(self) -> String {
		return self;
	}
}

impl Show for &String {
	#[inline]
	fn take(self) -> String {
		return self.clone();
	}
}

impl Show for crate::Block {
	#[inline]
	fn take(self) -> String {
		return self.show();
	}
}

#[inline]
pub fn block(f: Flag, text: impl Show) {
	let s = text.take();
	let t = s.trim_end();
	log(&t);
	if on(f) {
		print(&t);
	}
}

#[inline]
pub fn always(t: impl Display) {
	let s = t.to_string();
	let trimmed = s.trim_end();
	log(&trimmed);
	print(&trimmed);
}

#[inline]
pub fn wait(t: impl Display) {
	always(t);
}

#[inline]
pub fn unwait() {
	use std::io::IsTerminal as _;
	if io::stderr().is_terminal() {
		drop(write!(io::stderr(), "\u{1b}[1A\u{1b}[2K\r"));
	}
}
pub struct Progress {
	label: String,
	total: u64,
	done: std::sync::atomic::AtomicU64,
	begun: std::time::Instant,
	tty: bool,
}

pub fn progress(label: impl Display, total: u64) -> Progress {
	use std::io::IsTerminal as _;
	let p = Progress {
		label: label.to_string(),
		total,
		done: std::sync::atomic::AtomicU64::new(0),
		begun: std::time::Instant::now(),
		tty: io::stderr().is_terminal(),
	};
	p.draw(0);
	return p;
}

impl Progress {
	pub fn tick(&self) {
		let done = self.done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
		self.draw(done);
	}

	pub fn finish(&self) {
		if self.tty {
			drop(write!(io::stderr(), "\r\u{1b}[2K"));
		}
	}

	fn draw(&self, done: u64) {
		if !self.tty {
			return;
		}
		let secs = self.begun.elapsed().as_secs_f64();
		let rate = done as f64 / secs.max(f64::MIN_POSITIVE);
		let width: usize = 30;
		let fill = ((done.min(self.total) as usize).saturating_mul(width)) / (self.total.max(1) as usize);
		let rest = width - fill;
		drop(write!(
			io::stderr(),
			"\r\u{1b}[2K    {} {:.1}/s {:.1}s [{:=<fill$}{:-<rest$}] {}/{}",
			self.label,
			rate,
			secs,
			"",
			"",
			done,
			self.total,
		));
	}
}

#[inline]
pub fn err(t: impl Display) -> Result<(), Errored> {
	return Result::Err(Errored::new(t));
}

#[inline]
pub fn error(t: impl Display) {
	let _reported = Errored::new(t);
}

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

impl Show for ogdl::Block {
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
#[inline]
pub fn err(t: impl Display) -> Result<(), Errored> {
	return Result::Err(Errored::new(t));
}

#[inline]
pub fn error(t: impl Display) {
	let _reported = Errored::new(t);
}

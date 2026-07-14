#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]
use std::fmt::Display;
use std::fs::File;
use std::io::Write as _;
use std::sync::Mutex;

macro_rules! flags {
	($($f:ident),+) => {
		#[derive(Clone, Copy, Default)]
		pub struct Opt {
			$(pub $f: bool,)+
		}
		#[derive(Clone, Copy, PartialEq)]
		pub enum Flag {
			$($f,)+
		}
		static OPT: Mutex<Opt> = Mutex::new(Opt { $($f: false,)+ });
		fn on(f: Flag) -> bool {
			let o = opt();
			match f {
				$(Flag::$f => o.$f,)+
			}
		}
	};
}

flags!(loss, acc, epoch, lr, time, r2, device, data, gpu, probe, save, net, prompt, chat);
pub use Flag::*;

#[derive(Clone, Copy)]
pub struct Err;
impl Err {
	pub const log: bool = true;
	pub const print: bool = true;
	pub const line: bool = true;
}

pub struct Optusr(pub Opt);
pub struct OptDev(pub Opt, pub Err);
pub struct Option {
	pub user: Optusr,
	pub dev: OptDev,
}

pub fn set_opt(o: Opt) {
	*OPT.lock().unwrap_or_else(|p| p.into_inner()) = o;
}

pub fn opt() -> Opt {
	*OPT.lock().unwrap_or_else(|p| p.into_inner())
}

fn log(t: &impl Display) {
	std::fs::create_dir_all("/tmp/recipe").expect("log: create /tmp/recipe");
	let path = format!("/tmp/recipe/run{:03x}.log", std::process::id() & 0xfff);
	let mut f = File::options()
		.append(true)
		.create(true)
		.open(path)
		.expect("log: open run log");
	writeln!(f, "{t}").expect("log: write run log");
}

fn print(t: &impl Display) {
	drop(writeln!(std::io::stderr(), "{t}"));
}

pub struct Errored(pub String);

impl Errored {
	pub fn new(t: impl Display) -> Errored {
		match Err::log {
			true => log(&t),
			false => {}
		}
		match Err::print {
			true => print(&t),
			false => {}
		}
		Errored(t.to_string())
	}
}

impl Display for Errored {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::fmt::Debug for Errored {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(&self.0)
	}
}

impl std::error::Error for Errored {}

pub mod Write {
	use super::*;
	pub fn line(f: Flag, t: impl Display) {
		log(&t);
		match on(f) {
			true => print(&t),
			false => {}
		}
	}
	pub trait Show {
		fn take(self) -> String;
	}

	impl Show for &str {
		fn take(self) -> String {
			self.to_string()
		}
	}

	impl Show for String {
		fn take(self) -> String {
			self
		}
	}

	impl Show for &String {
		fn take(self) -> String {
			self.clone()
		}
	}

	impl Show for ogdl::Block {
		fn take(self) -> String {
			self.show()
		}
	}

	pub fn block(f: Flag, text: impl Show) {
		let s = text.take();
		let t = s.trim_end();
		log(&t);
		match on(f) {
			true => print(&t),
			false => {}
		}
	}

	pub fn always(t: impl Display) {
		let s = t.to_string();
		let t = s.trim_end();
		log(&t);
		print(&t);
	}

	pub fn wait(t: impl Display) {
		always(t);
	}

	pub fn unwait() {
		use std::io::IsTerminal;
		match std::io::stderr().is_terminal() {
			true => drop(write!(std::io::stderr(), "\u{1b}[1A\u{1b}[2K\r")),
			false => {}
		}
	}
	pub fn err(t: impl Display) -> std::result::Result<(), Errored> {
		std::result::Result::Err(Errored::new(t))
	}
}

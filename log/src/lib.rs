#![allow(
	non_upper_case_globals,
	non_camel_case_types,
	non_snake_case,
	dead_code,
	reason = "log crate deliberately uses lowercase type/const/flag identifiers as its frozen public API"
)]
use core::fmt;
use core::fmt::Display;
use std::error;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write as _;
use std::process;
use std::sync::Mutex;
use std::sync::PoisonError;

/// Declares the flag [`Opt`] struct, the [`Flag`] enum, and the global option state.
macro_rules! flags {
	($($f:ident),+) => {
		#[derive(Clone, Copy, Default)]
		pub struct Opt {
			$(pub $f: bool,)+
		}
		#[derive(Clone, Copy, PartialEq, Eq)]
		#[non_exhaustive]
		pub enum Flag {
			$($f,)+
		}
		static OPT: Mutex<Opt> = Mutex::new(Opt { $($f: false,)+ });
		fn on(f: Flag) -> bool {
			let o = opt();
			match f {
				$(Flag::$f => return o.$f,)+
			}
		}
	};
}

flags!(
	loss, acc, epoch, lr, time, r2, device, data, gpu, probe, save, net, prompt, chat
);
pub use Flag::*;

#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct Err;
impl Err {
	pub const line: bool = true;
	pub const log: bool = true;
	pub const print: bool = true;
}

#[non_exhaustive]
pub struct Optusr(pub Opt);
#[non_exhaustive]
pub struct OptDev(pub Opt, pub Err);
#[non_exhaustive]
pub struct Option {
	pub dev: OptDev,
	pub user: Optusr,
}

#[inline]
pub fn set_opt(o: Opt) {
	*OPT.lock().unwrap_or_else(PoisonError::into_inner) = o;
}

#[inline]
pub fn opt() -> Opt {
	return *OPT.lock().unwrap_or_else(PoisonError::into_inner);
}

/// Appends `t` to this process's run log under `/tmp/recipe`, ignoring any I/O error.
fn log(t: &impl Display) {
	if fs::create_dir_all("/tmp/recipe").is_err() {
		return;
	}
	let path = format!("/tmp/recipe/run{:03x}.log", process::id() & 0xfff);
	let Ok(mut f) = File::options().append(true).create(true).open(path) else {
		return;
	};
	drop(writeln!(f, "{t}"));
}

/// Writes `t` to stderr, ignoring any I/O error.
fn print(t: &impl Display) {
	drop(writeln!(io::stderr(), "{t}"));
}

#[non_exhaustive]
pub struct Errored(pub String);

impl Errored {
	#[inline]
	pub fn new(t: impl Display) -> Self {
		if Err::log {
			log(&t);
		}
		if Err::print {
			print(&t);
		}
		return Self(t.to_string());
	}
}

impl Display for Errored {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		return f.write_str(&self.0);
	}
}

impl fmt::Debug for Errored {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		return f.write_str(&self.0);
	}
}

impl error::Error for Errored {}

pub mod Write;

#![allow(unsafe_code)]
#![allow(
	non_upper_case_globals,
	non_camel_case_types,
	non_snake_case,
	dead_code,
	reason = "log module deliberately uses lowercase type/const/flag identifiers as its frozen public API"
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

pub fn log_path() -> &'static str {
	static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
	return PATH.get_or_init(|| {
		if let Ok(p) = std::env::var("RECIPE_LOG").map(Some).map(|o| o.filter(|p| !p.is_empty()))
			&& let Some(p) = p
		{
			return p;
		}
		let secs = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|d| d.as_secs())
			.unwrap_or(0);
		let p = format!("/tmp/recipe/run-{secs}-{}.log", process::id());
		// SAFETY: first log call happens before this process spawns threads that read the environment; the value is set exactly once.
		unsafe {
			std::env::set_var("RECIPE_LOG", &p);
		}
		return p;
	});
}

fn log(t: &impl Display) {
	if fs::create_dir_all("/tmp/recipe").is_err() {
		return;
	}
	let Ok(mut f) = File::options().append(true).create(true).open(log_path()) else {
		return;
	};
	drop(writeln!(f, "{t}"));
}

fn print(t: &impl Display) {
	drop(writeln!(io::stderr(), "{t}"));
}

#[non_exhaustive]
pub struct Errored(pub String);

impl Errored {
	#[inline]
	pub fn new(t: impl Display) -> Self {
		let msg = format!("ERROR: {t}");
		log(&msg);
		print(&msg);
		return Self(msg);
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

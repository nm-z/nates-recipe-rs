#![allow(
	non_upper_case_globals,
	non_camel_case_types,
	non_snake_case,
	dead_code
)]

use std::{
	fmt::Display,
	fs::File,
	io::{self, Write as _},
	sync::Mutex,
};

macro_rules! flags {
	($($f:ident),+) => {
		#[derive(Clone, Copy, Default)]
		struct Opt {
			$(pub $f: bool,)+
		}
		#[derive(Clone, Copy, PartialEq)]
		pub enum Flag {
			$($f,)+
		}
		static OPT: Mutex<Opt> = Mutex::new(Opt { $($f: false,)+ });
		pub fn select(flags: impl IntoIterator<Item = Flag>) {
			let mut selected = Opt::default();
			for flag in flags {
				match flag {
					$(Flag::$f => selected.$f = true,)+
				}
			}
			*OPT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = selected;
		}
		fn on(f: Flag) -> bool {
			let o = *OPT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
			match f {
				$(Flag::$f => o.$f,)+
			}
		}
	};
}

flags!(
	loss, acc, epoch, lr, time, r2, device, data, gpu, probe, save, net, prompt, chat
);
pub use Flag::*;

fn log(t: &impl Display) {
	std::fs::create_dir_all("/tmp/recipe").expect("write: create /tmp/recipe");
	let path = format!("/tmp/recipe/run{}.log", std::process::id());
	let mut f = File::options()
		.append(true)
		.create(true)
		.open(path)
		.expect("write: open run log");
	writeln!(f, "{t}").expect("write: write run log");
}

fn print(t: &impl Display) { drop(stderr(format!("{t}\n").as_bytes())); }

pub fn stdout(bytes: &[u8]) -> io::Result<()> {
	let mut output = io::stdout().lock();
	output.write_all(bytes)?;
	output.flush()
}

pub fn stderr(bytes: &[u8]) -> io::Result<()> {
	let mut output = io::stderr().lock();
	output.write_all(bytes)?;
	output.flush()
}

#[derive(Clone, PartialEq, Eq)]
pub struct Errored(pub String);

impl Errored {
	pub fn new(t: impl Display) -> Errored {
		log(&t);
		print(&t);
		Errored(t.to_string())
	}
}

impl Display for Errored {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str(&self.0) }
}

impl std::fmt::Debug for Errored {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str(&self.0) }
}

impl std::error::Error for Errored {}

pub fn line(f: Flag, t: impl Display) {
	log(&t);
	if on(f) {
		print(&t);
	}
}

pub fn block(f: Flag, text: &str) {
	let t = text.trim_end();
	log(&t);
	if on(f) {
		print(&t);
	}
}

pub fn always(t: impl Display) {
	let text = t.to_string();
	let text = text.trim_end();
	log(&text);
	print(&text);
}

pub fn err(t: impl Display) -> Result<(), Errored> { Err(Errored::new(t)) }

pub fn error(t: impl Display) { let _reported = Errored::new(t); }

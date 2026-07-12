#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]
use std::fmt::Display;
use std::fs::File;
use std::io::Write as _;
use std::sync::Mutex;

static OPT: Mutex<Opt> = Mutex::new(Opt {
	loss: false,
	acc: false,
	epoch: false,
	lr: false,
	time: false,
	r2: false,
	device: false,
});

static DEV: Mutex<Dev> = Mutex::new(Dev {
	data: false,
	gpu: false,
	probe: false,
	save: false,
	net: false,
	prompt: false,
});

#[derive(Clone, Copy, Default)]
pub struct Opt {
	pub loss: bool,
	pub acc: bool,
	pub epoch: bool,
	pub lr: bool,
	pub time: bool,
	pub r2: bool,
	pub device: bool,
}

#[derive(Clone, Copy, Default)]
pub struct Dev {
	pub data: bool,
	pub gpu: bool,
	pub probe: bool,
	pub save: bool,
	pub net: bool,
	pub prompt: bool,
}

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

pub fn set_dev(d: Dev) {
	*DEV.lock().unwrap_or_else(|p| p.into_inner()) = d;
}

pub fn dev() -> Dev {
	*DEV.lock().unwrap_or_else(|p| p.into_inner())
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
	writeln!(std::io::stderr(), "{t}").expect("log: stderr");
}

pub mod Write {
	use super::*;
	pub fn line(on: bool, t: impl Display) {
		log(&t);
		match on {
			true => print(&t),
			false => {}
		}
	}
	pub fn block(on: bool, graph: &str) {
		log(&graph);
		match on {
			true => print(&graph),
			false => {}
		}
	}
	pub fn err(t: impl Display) {
		match Err::log {
			true => log(&t),
			false => {}
		}
		match Err::print {
			true => print(&t),
			false => {}
		}
	}
}

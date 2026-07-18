#![allow(unsafe_code, reason = "raw-HSA dispatch of loaded assembly")]
//! Load a `.s`/`.hsaco` and dispatch one of its kernels from the command line.
//!
//! ```text
//! asmrun <path.s>                                  # list the kernels
//! asmrun <path.s> <kernel> <gridX> <blockX> [arg..] # dispatch
//! ```
//!
//! Each `arg` is `KIND:VALUE`: `ptr:<bytes>` allocates a zeroed device buffer of
//! that many bytes and passes its pointer; `i32/u32/u64:<v>` and `f64:<v>` pass
//! a scalar. Arguments are packed in declaration order.

use core::ffi::c_void;
use gpu_core::asm::{self, KernArgs};
use gpu_core::log::{Errored, Write};
use std::process;

fn parse_arg(token: &str, args: &mut KernArgs, bufs: &mut Vec<*mut c_void>) -> Result<(), Errored> {
	let (kind, value) = token
		.split_once(':')
		.ok_or_else(|| Errored::new(format!("bad arg '{token}': expected KIND:VALUE")))?;
	match kind {
		"ptr" => {
			let bytes: usize = value
				.parse()
				.map_err(|e| Errored::new(format!("ptr size '{value}': {e}")))?;
			let p = asm::alloc_device(bytes)
				.map_err(|e| Errored::new(format!("alloc_device({bytes}): {e}")))?;
			bufs.push(p);
			args.ptr(p);
		}
		"i32" => {
			let v: i32 = value
				.parse()
				.map_err(|e| Errored::new(format!("i32 '{value}': {e}")))?;
			args.i32(v);
		}
		"u32" => {
			let v: u32 = value
				.parse()
				.map_err(|e| Errored::new(format!("u32 '{value}': {e}")))?;
			args.u32(v);
		}
		"u64" => {
			let v: u64 = value
				.parse()
				.map_err(|e| Errored::new(format!("u64 '{value}': {e}")))?;
			args.u64(v);
		}
		"f64" => {
			let v: f64 = value
				.parse()
				.map_err(|e| Errored::new(format!("f64 '{value}': {e}")))?;
			args.f64(v);
		}
		other => return Err(Errored::new(format!("unknown arg kind '{other}'"))),
	}
	return Ok(());
}

fn run() -> Result<(), Errored> {
	let argv: Vec<String> = std::env::args().collect();
	let path = argv
		.get(1)
		.ok_or_else(|| Errored::new("usage: asmrun <path.s> [kernel gridX blockX arg..]"))?;
	let program =
		asm::load(path).map_err(|e| Errored::new(format!("load {path}: {e}")))?;

	let Some(kernel) = argv.get(2) else {
		let mut names = program.kernels();
		names.sort_unstable();
		Write::error(format!("kernels in {path}:"));
		for name in names {
			Write::error(format!("  {name}"));
		}
		return Ok(());
	};

	let grid_x: u32 = argv
		.get(3)
		.ok_or_else(|| Errored::new("missing gridX"))?
		.parse()
		.map_err(|e| Errored::new(format!("gridX: {e}")))?;
	let block_x: u16 = argv
		.get(4)
		.ok_or_else(|| Errored::new("missing blockX"))?
		.parse()
		.map_err(|e| Errored::new(format!("blockX: {e}")))?;

	let mut args = KernArgs::new();
	let mut bufs: Vec<*mut c_void> = Vec::new();
	for token in argv.iter().skip(5) {
		parse_arg(token, &mut args, &mut bufs)?;
	}

	program
		.launch(kernel, [grid_x, 1, 1], [block_x, 1, 1], &args)
		.map_err(|e| Errored::new(format!("launch {kernel}: {e}")))?;
	Write::error(format!(
		"DONE {kernel} grid={grid_x} block={block_x} kernargs={} bytes",
		args.as_bytes().len()
	));

	for p in bufs {
		// SAFETY: each p was returned by asm::alloc_device above and is freed exactly once here.
		unsafe {
			asm::free_device(p).map_err(|e| Errored::new(format!("free_device: {e}")))?;
		}
	}
	return Ok(());
}

fn main() {
	if let Err(e) = run() {
		Write::error(format!("asmrun: {e}"));
		process::exit(1);
	}
}

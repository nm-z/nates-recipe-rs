#![allow(unsafe_code)]
use anyhow::{Result, anyhow, bail};
use ogdl::log::{Opt, Write, gpu, set_opt};
use std::env;
use std::os::unix::process::CommandExt as _;
use std::process;
use std::process::ExitCode;

fn ensure_vramspy_preloaded() -> Result<()> {
	let loaded =
		!unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_loaded".as_ptr()) }.is_null();
	if loaded {
		return Ok(());
	}
	if env::var_os("VRAMSPY_REEXEC").is_some() {
		bail!("vramspy re-exec loop: libvramspy.so did not load");
	}
	let exe = env::current_exe()?;
	let shim = exe
		.parent()
		.and_then(|p| p.parent())
		.map(|p| p.join("libvramspy.so"))
		.ok_or_else(|| {
			anyhow!(
				"could not resolve libvramspy.so path from current_exe {}",
				exe.display()
			)
		})?;
	if !shim.exists() {
		bail!(
			"vramspy: {} not found — build it with `cargo build --release -p vramspy`",
			shim.display()
		);
	}
	let ld_preload = match env::var("LD_PRELOAD") {
		Ok(existing) if !existing.is_empty() => format!("{existing}:{}", shim.display()),
		_other => shim.display().to_string(),
	};
	unsafe {
		env::set_var("LD_PRELOAD", &ld_preload);
		env::set_var("VRAMSPY_REEXEC", "1");
	}
	Write::line(gpu, "re-exec with vramspy");
	Err(anyhow!(
		process::Command::new(env::current_exe()?)
			.args(env::args_os().skip(1))
			.exec()
	))
}

fn main() -> Result<ExitCode> {
	set_opt(Opt {
		prompt: true,
		gpu: true,
		data: true,
		..Opt::default()
	});
	ensure_vramspy_preloaded()?;

	let gguf_str = env::var("GGUF").unwrap_or_else(|_e| {
		"/home/nate/Desktop/gemma4/gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf".to_owned()
	});
	recipe::cli::tui::chat(&gguf_str);

	Write::line(gpu, gpu_core::memory::ledger_report());
	recipe_infer::shutdown();
	return Ok(ExitCode::SUCCESS);
}

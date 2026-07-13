use anyhow::{Result, anyhow, bail};
use gpu_core::log::{Opt, Write, gpu, prompt as promptf, set_opt};
use recipe_infer::llm::{generate, render_toks};
use std::os::unix::process::CommandExt;
use std::path::Path;

fn ensure_vramspy_preloaded() -> Result<()> {
	let loaded =
		!unsafe { libc::dlsym(libc::RTLD_DEFAULT, c"vramspy_loaded".as_ptr()) }.is_null();
	if loaded {
		return Ok(());
	}
	if std::env::var_os("VRAMSPY_REEXEC").is_some() {
		bail!("vramspy re-exec loop: libvramspy.so did not load");
	}
	let exe = std::env::current_exe()?;
	let shim = exe
		.parent()
		.and_then(|p| p.parent())
		.map(|p| p.join("libvramspy.so"))
		.ok_or_else(|| {
			anyhow!("could not resolve libvramspy.so path from current_exe {}", exe.display())
		})?;
	if !shim.exists() {
		bail!(
			"vramspy: {} not found — build it with `cargo build --release -p vramspy`",
			shim.display()
		);
	}
	let ld_preload = match std::env::var("LD_PRELOAD") {
		Ok(existing) if !existing.is_empty() => format!("{existing}:{}", shim.display()),
		_other => shim.display().to_string(),
	};
	unsafe {
		std::env::set_var("LD_PRELOAD", &ld_preload);
		std::env::set_var("VRAMSPY_REEXEC", "1");
	}
	Write::line(gpu, "re-exec with vramspy");
	Err(anyhow!(
		std::process::Command::new(std::env::current_exe()?)
			.args(std::env::args_os().skip(1))
			.exec()
	))
}

fn main() -> Result<()> {
	if let Some(code) = recipe_infer::llm::vram_probe_ask() {
		std::process::exit(code);
	}
	set_opt(Opt {
		prompt: true,
		gpu: true,
		data: true,
		..Opt::default()
	});
	ensure_vramspy_preloaded()?;

	let gguf = Path::new("/home/nate/Desktop/gemma4/gguf/diffusiongemma-26B-A4B-it-Q4_K_M.gguf");
	let ask = std::env::args()
		.nth(1)
		.unwrap_or_else(|| "The capital of France is".to_string());

	let mut step = 0usize;
	let out = generate(gguf, &ask, &mut |toks| {
		Write::line(promptf, format!("step {step}: {}", render_toks(toks)));
		step += 1;
	})?;

	Write::line(promptf, "");
	Write::line(promptf, "=== OUTPUT ===");
	Write::line(promptf, &out);
	Write::line(gpu, gpu_core::memory::ledger_report());
	recipe_infer::shutdown();
	Ok(())
}

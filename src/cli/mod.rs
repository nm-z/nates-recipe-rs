pub mod tui;

use anyhow::Result;
use ogdl::log::{Opt, Write, net, probe, set_opt};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::DefaultHasher;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process;
use std::slice;
use std::time::UNIX_EPOCH;

fn exit_code(code: i32) -> process::ExitCode {
	return process::ExitCode::from(u8::try_from(code).unwrap_or(1));
}

fn usage(code: i32) -> process::ExitCode {
	Write::always(&format!(
		"recipe {}.{}",
		env!("CARGO_PKG_VERSION"),
		env!("GIT_HASH")
	));
	Write::always("usage: recipe <file.rs> [args]  # compile + run");
	Write::always("       recipe serve            # daemon on 7845");
	Write::always("       recipe peers            # live network view");
	Write::always("       recipe probe            # measure this machine");
	Write::always("       recipe run [name]       # pick a gguf.toml model and chat");
	return exit_code(code);
}

fn gguf_toml_path() -> Result<PathBuf> {
	let cwd = PathBuf::from("gguf.toml");
	if cwd.exists() {
		return Ok(cwd);
	}
	let dir = match env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
		Some(xdg) => PathBuf::from(xdg).join("recipe"),
		None => {
			let home = env::var("HOME").map_err(|_e| anyhow::anyhow!("run: HOME not set"))?;
			PathBuf::from(home).join(".config/recipe")
		}
	};
	Ok(dir.join("gguf.toml"))
}

fn load_models(toml: &Path) -> Result<Vec<(String, String)>> {
	let text = fs::read_to_string(toml)
		.map_err(|e| anyhow::anyhow!("{}: {e}", toml.display()))?;
	let mut models = Vec::new();
	let mut in_models = false;
	for raw in text.lines() {
		let line = raw.split('#').next().unwrap_or("").trim();
		if line.is_empty() {
			continue;
		}
		if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
			in_models = section.trim() == "models";
			continue;
		}
		let Some(_keep) = Some(()).filter(|_probe| in_models) else {
			continue;
		};
		let Some((key, val)) = line.split_once('=') else {
			continue;
		};
		let path = val.trim().trim_matches('"').to_string();
		models.push((key.trim().to_string(), path));
	}
	Ok(models)
}

fn run_chat(name: Option<&str>) -> Result<()> {
	let toml = gguf_toml_path()?;
	let models = load_models(&toml)?;
	anyhow::ensure!(
		!models.is_empty(),
		"run: no [models] entries in {}",
		toml.display()
	);
	let chosen = match name {
		Some(want) => {
			let hit = models.iter().find(|(k, _v)| k == want);
			let Some((_k, path)) = hit else {
				anyhow::bail!("run: no model named {want:?} in {}", toml.display());
			};
			Some(path.clone())
		}
		None => {
			let names: Vec<String> = models.iter().map(|(k, _v)| k.clone()).collect();
			crate::cli::tui::model_picker(&names).map(|i| models[i].1.clone())
		}
	};
	let Some(path) = chosen else {
		return Ok(());
	};
	anyhow::ensure!(
		Path::new(&path).exists(),
		"run: model file not found: {path}"
	);
	crate::cli::tui::chat(&path);
	Ok(())
}

fn run_rs(path: &str, extra: &[String]) -> Result<()> {
	use std::hash::Hasher;
	use std::os::unix::process::CommandExt;
	let root = ["target/release", "target/debug", "/usr/lib/recipe"]
		.into_iter()
		.map(Path::new)
		.find(|d| d.join("librecipe.rlib").exists());
	let Some(root) = root else {
		anyhow::bail!(
			"librecipe.rlib missing: run cargo build (target/release or target/debug), or the recipe package provides /usr/lib/recipe"
		);
	};
	let rlib = root.join("librecipe.rlib");
	let src = fs::read(path).map_err(|e| anyhow::anyhow!("{path}: {e}"))?;
	let mtime = rlib.metadata()?.modified()?;
	let mut h = DefaultHasher::new();
	h.write(&src);
	h.write_u128(mtime.duration_since(UNIX_EPOCH)?.as_nanos());
	let bin = recipe_runtime::machine::data_dir()?.join(format!("{:016x}", h.finish()));
	for _absent in fs::metadata(&bin).err().into_iter() {
		let rocm = env::var_os("ROCM_PATH")
			.filter(|v| !v.is_empty())
			.map(PathBuf::from)
			.unwrap_or_else(|| PathBuf::from("/opt/rocm"));
		let mut cmd = process::Command::new("rustc");
		cmd.arg(path)
			.arg("-L")
			.arg(root)
			.arg("-L")
			.arg(root.join("deps"))
			.arg("-L")
			.arg(rocm.join("lib"))
			.args(["--edition", "2024"])
			.args([
				"-l",
				"amdhip64",
				"-l",
				"hipblas",
				"-l",
				"hipsolver",
				"-l",
				"stdc++",
			])
			.arg("-o")
			.arg(&bin);
		for name in ["recipe", "ogdl", "gpu_core", "pantry", "recipe_infer"] {
			cmd.arg("--extern").arg(format!(
				"{name}={}",
				root.join(format!("lib{name}.rlib")).display()
			));
		}
		let status = cmd.status().map_err(|e| anyhow::anyhow!("rustc: {e}"))?;
		anyhow::ensure!(status.success(), "rustc failed on {path}: {status}");
	}
	Err(process::Command::new(&bin).args(extra).exec().into())
}

pub fn main() -> anyhow::Result<std::process::ExitCode> {
	if let Some(code) = recipe_runtime::machine::gpu_child_ask() {
		return Ok(exit_code(code));
	}
	if let Some(code) = recipe_runtime::memory::vram_probe_ask()? {
		return Ok(exit_code(code));
	}
	if let Some(code) = recipe_runtime::machine::ram_probe_ask() {
		return Ok(exit_code(code));
	}
	if let Some(code) = recipe_runtime::execute::setup_race_ask()? {
		return Ok(exit_code(code));
	}
	let args: Vec<String> = env::args().collect();
	let cmd = match args.get(1) {
		Some(first) => first.as_str(),
		None => return Ok(usage(1)),
	};
	let outcome: Result<()> = match cmd {
		"-h" | "--help" => return Ok(usage(0)),
		"probe" => {
			set_opt(Opt {
				probe: true,
				..Opt::default()
			});
			let machine = recipe_runtime::machine::Machine::probe()?;
			Write::block(
				probe,
				&recipe_runtime::machine::write_config(slice::from_ref(&machine)),
			);
			Ok(())
		}
		"serve" => {
			set_opt(Opt {
				probe: true,
				..Opt::default()
			});
			let bind = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), recipe_runtime::transport::PORT);
			let listener = recipe_runtime::transport::Server::bind(bind)?;
			let machine = recipe_runtime::machine::Machine::probe()?;
			let info = recipe_runtime::transport::NodeInfo::probe();
			let runners = HashMap::new();
			recipe_runtime::transport::Server::new(info, runners)
				.machine(machine)
				.serve_bound(listener)?;
			Ok(())
		}
		"peers" => {
			use std::io::IsTerminal;
			set_opt(Opt {
				net: true,
				..Opt::default()
			});
			let off = recipe_runtime::transport::pool_deselected();
			let me = recipe_runtime::transport::self_host();
			let mine = recipe_runtime::transport::NodeInfo::probe();
			let mut rows = vec![crate::cli::tui::PeerRow {
				detail: format!(
					"{}  {} gpu  {} MiB vram  {} MiB ram",
					mine.arch,
					mine.gpus,
					mine.vram >> 20,
					mine.ram >> 20
				),
				selected: !off.contains(&me),
				host: me,
				local: true,
			}];
			for p in recipe_runtime::transport::local_peers()? {
				rows.push(crate::cli::tui::PeerRow {
					detail: format!(
						"{}  {}  {} gpu  {} MiB vram  {} MiB ram",
						p.addrs.join(","),
						p.info.arch,
						p.info.gpus,
						p.info.vram >> 20,
						p.info.ram >> 20
					),
					selected: !off.contains(&p.host),
					host: p.host,
					local: false,
				});
			}
			match io::stdin().is_terminal() && io::stderr().is_terminal() {
				true => {
					let save = crate::cli::tui::peers_picker(&mut rows);
					for _saved in Some(()).filter(|_u| save).into_iter() {
						let deselected: Vec<String> = rows
							.iter()
							.filter(|r| !r.selected)
							.map(|r| r.host.clone())
							.collect();
						recipe_runtime::transport::pool_write(&deselected)?;
					}
				}
				false => {
					for r in &rows {
						let state = match r.selected {
							true => "on",
							false => "off",
						};
						Write::line(net, &format!("{state}\t{}\t{}", r.host, r.detail));
					}
				}
			}
			Ok(())
		}
		"run" => {
			set_opt(Opt {
				prompt: true,
				gpu: true,
				data: true,
				..Opt::default()
			});
			run_chat(args.get(2).map(String::as_str))
		}
		other => {
			let probed = fs::metadata(other).ok();
			match other.strip_suffix(".rs").and(probed) {
				Some(_meta) => run_rs(other, &args[2..]),
				None => return Ok(usage(1)),
			}
		}
	};
	return outcome.map(|()| process::ExitCode::SUCCESS);
}

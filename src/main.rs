use gpu_core::log::{Errored, Opt, Write, gpu, net, probe, set_opt};
use anyhow::Result;

fn usage(code: i32) -> ! {
	drop(Write::err(&format!("recipe {}", env!("CARGO_PKG_VERSION"))));
	drop(Write::err("usage: recipe <file.rs> [args]  # compile + run"));
	drop(Write::err("       recipe serve            # daemon on 7845"));
	drop(Write::err("       recipe peers            # live network view"));
	drop(Write::err("       recipe probe            # measure this machine"));
	std::process::exit(code);
}

fn run_rs(path: &str, extra: &[String]) -> Result<()> {
	use std::hash::Hasher;
	use std::os::unix::process::CommandExt;
	let rlib = std::path::Path::new("/usr/lib/recipe/librecipe.rlib");
	anyhow::ensure!(
		rlib.exists(),
		"{} missing: the recipe package is not installed",
		rlib.display()
	);
	let src = std::fs::read(path).map_err(|e| anyhow::anyhow!("{path}: {e}"))?;
	let mtime = rlib.metadata()?.modified()?;
	let mut h = std::hash::DefaultHasher::new();
	h.write(&src);
	h.write_u128(mtime.duration_since(std::time::UNIX_EPOCH)?.as_nanos());
	let bin = recipe::probe::data_dir()?.join(format!("{:016x}", h.finish()));
	for _absent in std::fs::metadata(&bin).err().into_iter() {
		let rocm = std::env::var_os("ROCM_PATH")
			.filter(|v| !v.is_empty())
			.map(std::path::PathBuf::from)
			.unwrap_or_else(|| std::path::PathBuf::from("/opt/rocm"));
		let mut cmd = std::process::Command::new("rustc");
		cmd.arg(path)
			.args(["-L", "/usr/lib/recipe", "-L", "/usr/lib/recipe/deps"])
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
			cmd.arg("--extern")
				.arg(format!("{name}=/usr/lib/recipe/lib{name}.rlib"));
		}
		let status = cmd.status().map_err(|e| anyhow::anyhow!("rustc: {e}"))?;
		anyhow::ensure!(status.success(), "rustc failed on {path}: {status}");
	}
	Err(std::process::Command::new(&bin).args(extra).exec().into())
}

fn main() -> Result<()> {
	if let Some(code) = recipe::probe::gpu_child_ask() {
		std::process::exit(code);
	}
	if let Some(sz) = std::env::var_os("VRAM_PROBE") {
		let n: usize = sz
			.to_string_lossy()
			.parse()
			.map_err(|e| Errored::new(format!("VRAM_PROBE parse: {e}")))?;
		let code = match gpu_core::memory::GpuBuffer::try_alloc_bytes(n) {
			Some(held) => {
				drop(held);
				0
			}
			None => 2,
		};
		std::process::exit(code);
	}
	if let Some(it) = std::env::var_os("SETUP_RACE") {
		let iters: usize = it
			.to_string_lossy()
			.parse()
			.map_err(|e| Errored::new(format!("SETUP_RACE parse: {e}")))?;
		gpu_core::hip::set_device(0)?;
		for i in 0..iters {
			let x = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 768), 1.0);
			let cat = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 128), 1.0);
			let mut stage = gpu_core::memory::Stage::new();
			let x_std = x.as_standard_layout();
			let x_off = stage.push(
				x_std
					.as_slice()
					.ok_or_else(|| Errored::new("x contig"))?,
			);
			let cat_std = cat.as_standard_layout();
			let cat_off = stage.push(
				cat_std
					.as_slice()
					.ok_or_else(|| Errored::new("cat contig"))?,
			);
			let host = stage.into_host();
			let staged = gpu_core::memory::GpuBuffer::alloc(host.len().max(1))
				.map_err(|e| Errored::new(format!("setup-race stage: {e}")))?;
			staged
				.load(&host)
				.map_err(|e| Errored::new(format!("setup-race stage: {e}")))?;
			let xraw = staged.view(x_off, x.len());
			let craw = staged.view(cat_off, cat.len());
			let nn = cat.nrows();
			let cc = cat.ncols();
			let eps = {
				let e = gpu_core::memory::GpuBuffer::alloc(1).map_err(|e| Errored::new(format!("eps: {e}")))?;
				e.load(&[recipe_infer::ZSCORE_EPS]).map_err(|e| Errored::new(format!("eps load: {e}")))?;
				e
			};
			let mean = gpu_core::memory::GpuBuffer::alloc(cc).map_err(|e| Errored::new(format!("mean: {e}")))?;
			let std = gpu_core::memory::GpuBuffer::alloc(cc).map_err(|e| Errored::new(format!("std: {e}")))?;
			let xb = gpu_core::memory::GpuBuffer::alloc(nn * cc).map_err(|e| Errored::new(format!("zscored: {e}")))?;
			recipe_infer::zscore_fit_into(&craw, nn, cc, &eps, &mean, &std, &xb)?;
			let lse = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).map_err(|e| Errored::new(format!("lse: {e}")))?;
			let dsum = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).map_err(|e| Errored::new(format!("dsum: {e}")))?;
			let mut fills = Vec::new();
			while let Some(b) = gpu_core::memory::GpuBuffer::try_alloc_bytes(256 << 20) {
				fills.push(b);
			}
			let mut probe1k = vec![0u8; 1024];
			lse.download_u8(&mut probe1k)
				.map_err(|e| Errored::new(format!("d2h under pressure: {e}")))?;
			drop(xraw);
			drop(craw);
			drop(xb);
			drop(lse);
			drop(dsum);
			drop(fills);
			drop(staged);
			drop(eps);
			drop(mean);
			drop(std);
			gpu_core::memory::pool_trim();
			Write::line(gpu, &format!("setup-race iter {i}: clean"));
		}
		gpu_core::kernels::gpu_shutdown();
		std::process::exit(0);
	}
	let args: Vec<String> = std::env::args().collect();
	let cmd = match args.get(1) {
		Some(first) => first.as_str(),
		None => usage(1),
	};
	match cmd {
		"-h" | "--help" => usage(0),
		"probe" => {
			set_opt(Opt {
				probe: true,
				..Opt::default()
			});
			let machine = recipe::probe::Machine::probe()?;
			Write::block(
				probe,
				&recipe::probe::write_config(std::slice::from_ref(&machine)),
			);
			Ok(())
		}
		"serve" => {
			set_opt(Opt {
				probe: true,
				..Opt::default()
			});
			let bind = std::net::SocketAddr::new(
				std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
				recipe::wire::PORT,
			);
			let listener = recipe::wire::Server::bind(bind)?;
			let machine = recipe::probe::Machine::probe()?;
			let info = recipe::wire::NodeInfo::probe();
			let runners = std::collections::HashMap::new();
			recipe::wire::Server::new(info, runners)
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
			let off = recipe::wire::pool_deselected();
			let me = recipe::wire::self_host();
			let mine = recipe::wire::NodeInfo::probe();
			let mut rows = vec![recipe::tui::PeerRow {
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
			for p in recipe::wire::local_peers()? {
				rows.push(recipe::tui::PeerRow {
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
			match std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
				true => {
					let save = recipe::tui::peers_picker(&mut rows);
					for _saved in Some(()).filter(|_u| save).into_iter() {
						let deselected: Vec<String> = rows
							.iter()
							.filter(|r| !r.selected)
							.map(|r| r.host.clone())
							.collect();
						recipe::wire::pool_write(&deselected)?;
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
		other => {
			let probed = std::fs::metadata(other).ok();
			match other.strip_suffix(".rs").and(probed) {
				Some(_meta) => run_rs(other, &args[2..]),
				None => usage(1),
			}
		}
	}
}

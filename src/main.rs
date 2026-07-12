use gpu_core::log::{Opt, Write, gpu, probe, set_opt};
use anyhow::Result;

fn usage(code: i32) -> ! {
	Write::err("usage: recipe <file.rs> [args]  # compile + run");
	Write::err("       recipe serve            # daemon on 7845");
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
	if let Some(d) = std::env::var_os("RECIPE_PROBE_GPU") {
		let card: i32 = d.to_string_lossy().parse().expect("RECIPE_PROBE_GPU parse");
		match recipe::probe::probe_gpu_child_record(card) {
			Ok(rec) => {
				set_opt(Opt {
					probe: true,
					..Opt::default()
				});
				Write::line(probe, &rec);
				std::process::exit(0);
			}
			Err(e) => {
				Write::err(&format!("probe child gpu{card}: {e}"));
				std::process::exit(2);
			}
		}
	}
	if let Some(sz) = std::env::var_os("VRAM_PROBE") {
		let n: usize = sz.to_string_lossy().parse().expect("VRAM_PROBE parse");
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
		let iters: usize = it.to_string_lossy().parse().expect("SETUP_RACE parse");
		gpu_core::hip::set_device(0)?;
		for i in 0..iters {
			let x = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 768), 1.0);
			let cat = ndarray::Array2::<f64>::from_elem(ndarray::Ix2(45982, 128), 1.0);
			let mut stage = gpu_core::memory::Stage::new();
			let x_off = stage.push(x.as_standard_layout().as_slice().expect("x contig"));
			let cat_off =
				stage.push(cat.as_standard_layout().as_slice().expect("cat contig"));
			let host = stage.into_host();
			let staged = gpu_core::memory::GpuBuffer::alloc(host.len().max(1))
				.expect("setup-race stage");
			staged.load(&host).expect("setup-race stage");
			let xraw = staged.view(x_off, x.len());
			let craw = staged.view(cat_off, cat.len());
			let nn = cat.nrows();
			let cc = cat.ncols();
			let eps = {
				let e = gpu_core::memory::GpuBuffer::alloc(1).expect("eps");
				e.load(&[recipe_infer::ZSCORE_EPS]).expect("eps load");
				e
			};
			let mean = gpu_core::memory::GpuBuffer::alloc(cc).expect("mean");
			let std = gpu_core::memory::GpuBuffer::alloc(cc).expect("std");
			let xb = gpu_core::memory::GpuBuffer::alloc(nn * cc).expect("zscored");
			recipe_infer::zscore_fit_into(&craw, nn, cc, &eps, &mean, &std, &xb)?;
			let lse = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).expect("lse");
			let dsum = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).expect("dsum");
			let mut fills = Vec::new();
			while let Some(b) = gpu_core::memory::GpuBuffer::try_alloc_bytes(256 << 20) {
				fills.push(b);
			}
			let mut probe1k = vec![0u8; 1024];
			lse.download_u8(&mut probe1k).expect("d2h under pressure");
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
		"serve" => {
			set_opt(Opt {
				probe: true,
				..Opt::default()
			});
			let bind = std::net::SocketAddr::new(
				std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
				recipe::wire::PORT,
			);
			let listener = std::net::TcpListener::bind(bind)?;
			let machine = recipe::probe::Machine::probe()?;
			let info = recipe::wire::NodeInfo::probe();
			let runners = std::collections::HashMap::new();
			recipe::wire::Server::new(info, runners)
				.machine(machine)
				.serve_bound(listener)?;
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

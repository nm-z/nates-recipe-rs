use anyhow::Result;

fn kind_name(k: usize) -> &'static str {
	match k {
		pantry::KIND_NUMERIC => "Numeric",
		pantry::KIND_TEMPORAL => "Temporal",
		pantry::KIND_CATEGORICAL => "Categorical",
		pantry::KIND_ORDINAL => "Ordinal",
		pantry::KIND_TEXT => "Text",
		_ => "Image",
	}
}

fn run_rs(path: &str, extra: &[String]) -> Result<()> {
	use std::hash::Hasher;
	use std::os::unix::process::CommandExt;
	let rlib = std::path::Path::new("/usr/lib/recipe/librecipe.rlib");
	if !rlib.exists() {
		anyhow::bail!("{} missing: install the recipe package", rlib.display());
	}
	let src = std::fs::read(path).map_err(|e| anyhow::anyhow!("{path}: {e}"))?;
	let mtime = rlib.metadata()?.modified()?;
	let mut h = std::hash::DefaultHasher::new();
	h.write(&src);
	h.write_u128(mtime.duration_since(std::time::UNIX_EPOCH)?.as_nanos());
	let bin = recipe::probe::data_dir()?.join(format!("{:016x}", h.finish()));
	if !bin.exists() {
		let status = std::process::Command::new("rustc")
			.arg(path)
			.args(["-L", "/usr/lib/recipe", "-L", "/usr/lib/recipe/deps", "-L", "/opt/rocm/lib"])
			.args(["--extern", "recipe=/usr/lib/recipe/librecipe.rlib"])
			.args(["--edition", "2024"])
			.args(["-l", "amdhip64", "-l", "hipblas", "-l", "hipsolver", "-l", "stdc++"])
			.arg("-o")
			.arg(&bin)
			.status()
			.map_err(|e| anyhow::anyhow!("rustc: {e}"))?;
		if !status.success() {
			anyhow::bail!("rustc failed on {path}: {status}");
		}
	}
	Err(std::process::Command::new(&bin).args(extra).exec().into())
}

fn main() -> Result<()> {
	if let Some(d) = std::env::var_os("RECIPE_PROBE_GPU") {
		let dev: i32 = d.to_string_lossy().parse().expect("RECIPE_PROBE_GPU parse");
		match recipe::probe::probe_gpu_child_record(dev) {
			Ok(rec) => {
				println!("{rec}");
				std::process::exit(0);
			}
			Err(e) => {
				eprintln!("probe child gpu{dev}: {e}");
				std::process::exit(2);
			}
		}
	}
	if let Some(sz) = std::env::var_os("VRAM_PROBE") {
		let n: usize = sz.to_string_lossy().parse().expect("VRAM_PROBE parse");
		let ok = gpu_core::memory::GpuBuffer::try_alloc_bytes(n).is_some();
		std::process::exit(if ok { 0 } else { 2 });
	}
	if let Some(it) = std::env::var_os("SETUP_RACE") {
		let iters: usize = it.to_string_lossy().parse().expect("SETUP_RACE parse");
		gpu_core::hip::set_device(0)?;
		for i in 0..iters {
			let x = ndarray::Array2::<f64>::from_elem((45982, 768), 1.0);
			let cat = ndarray::Array2::<f64>::from_elem((45982, 128), 1.0);
			let mut stage = gpu_core::memory::Stage::new();
			let x_off = stage.push(x.as_standard_layout().as_slice().expect("x contig"));
			let cat_off = stage.push(cat.as_standard_layout().as_slice().expect("cat contig"));
			let host = stage.into_host();
			let staged = gpu_core::memory::GpuBuffer::alloc(host.len().max(1)).expect("setup-race stage");
			staged.load(&host).expect("setup-race stage");
			let xraw = staged.view(x_off, x.len());
			let craw = staged.view(cat_off, cat.len());
			let (nn, cc) = (cat.nrows(), cat.ncols());
			let eps = {
				let e = gpu_core::memory::GpuBuffer::alloc(1).expect("eps");
				e.load(&[recipe_infer::ZSCORE_EPS]).expect("eps load");
				e
			};
			let mean = gpu_core::memory::GpuBuffer::alloc(cc).expect("mean");
			let std = gpu_core::memory::GpuBuffer::alloc(cc).expect("std");
			let xb = gpu_core::memory::GpuBuffer::alloc(nn * cc).expect("zscored");
			recipe_infer::zscore_fit_into(&craw, nn, cc, &eps, &mean, &std, &xb);
			let lse = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).expect("lse");
			let dsum = gpu_core::memory::GpuBuffer::alloc(45982 * 3072).expect("dsum");
			let mut fills = Vec::new();
			while let Some(b) = gpu_core::memory::GpuBuffer::try_alloc_bytes(256 << 20) {
				fills.push(b);
			}
			let mut probe1k = vec![0u8; 1024];
			lse.download_u8(&mut probe1k).expect("d2h under pressure");
			drop((xraw, craw, xb, lse, dsum, fills, staged, eps, mean, std));
			gpu_core::memory::pool_trim();
			eprintln!("setup-race iter {i}: clean");
		}
		gpu_core::kernels::gpu_shutdown();
		std::process::exit(0);
	}
	let args: Vec<String> = std::env::args().collect();
	if args.len() < 2 {
		eprintln!("usage: recipe <train.csv> [--target <col>]");
		eprintln!("       recipe <file.rs> [args]  # compile user file + run");
		eprintln!("       recipe detect <path>");
		eprintln!("       recipe serve            # discovery + RPC node on 7845");
		eprintln!("       recipe peers            # live network view");
		eprintln!("       recipe install          # probe + config.ogdl + systemd unit");
		std::process::exit(1);
	}

	if args[1] == "serve" {
		let listener = std::net::TcpListener::bind(("0.0.0.0", recipe::wire::PORT))?;
		let machine = recipe::probe::Machine::probe()?;
		let info = recipe::wire::NodeInfo::probe();
		let runners = std::collections::HashMap::new();
		recipe::wire::Server::new(info, runners)
			.machine(machine)
			.serve_bound(listener)?;
		return Ok(());
	}

	if args[1] == "install" {
		let machine = recipe::probe::Machine::probe()?;
		recipe::probe::install(&machine)?;
		return Ok(());
	}

	if args[1] == "peers" {
		for p in recipe::wire::local_peers()? {
			println!(
				"{}\t{}\t{} MiB ram\t{}",
				p.host,
				p.addrs.join(","),
				p.info.ram >> 20,
				p.info.arch
			);
		}
		return Ok(());
	}

	if args[1].ends_with(".rs") {
		return run_rs(&args[1], &args[2..]);
	}

	gpu_core::hip::set_device(0)?;

	if args[1] == "detect" {
		let Some(path) = args.get(2) else {
			eprintln!("usage: recipe detect <path>");
			std::process::exit(1);
		};
		for group in pantry::data::load_groups(path) {
			let pantry::data::DirGroup::Table { name, headers, cells, .. } = group else {
				continue;
			};
			let columns: Vec<Vec<&str>> = (0..headers.len())
				.map(|j| {
					cells
						.iter()
						.filter_map(|r| r.get(j).map(String::as_str))
						.filter(|c| !c.is_empty())
						.collect()
				})
				.collect();
			let kinds = pantry::predict_kinds(&columns)?;
			for (h, k) in headers.iter().zip(kinds) {
				if name.is_empty() {
					println!("{h} -> {}", kind_name(k));
				} else {
					println!("{name}:{h} -> {}", kind_name(k));
				}
			}
		}
		gpu_core::kernels::gpu_shutdown();
		return Ok(());
	}

	let mut target = None;
	let mut path = &args[1];
	let mut i = 2;
	while i < args.len() {
		match args[i].as_str() {
			"--target" => {
				target = Some(args[i + 1].as_str());
				i += 2;
			}
			_ => {
				path = &args[i];
				i += 1;
			}
		}
	}

	let data = if let Some(t) = target {
		recipe::Data::load(path).target(t)
	} else {
		eprintln!("no --target specified");
		recipe::Data::load(path)
	};
	let (set, _test) = data.datasets();
	eprintln!(
		"loaded {} samples × {} features",
		set.x.nrows(),
		set.x.ncols()
	);

	gpu_core::kernels::gpu_shutdown();
	Ok(())
}

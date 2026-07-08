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

fn main() -> Result<()> {
	// Disposable VRAM probe: attempt exactly one allocation and report by exit
	// code. An oversize ask dies in the uncatchable VmHeap abort — in THIS
	// process, whose corpse is the signal (memory::probe_ceiling walks the ask
	// down until a probe survives).
	if let Some(sz) = std::env::var_os("VRAM_PROBE") {
		let n: usize = sz.to_string_lossy().parse().expect("VRAM_PROBE parse");
		let ok = gpu_core::memory::GpuBuffer::try_alloc_bytes(n).is_some();
		std::process::exit(if ok { 0 } else { 2 });
	}
	// Setup-race repro: the exact fit-setup op sequence that faults/wedges at
	// LLM scale (upload → zscore → 1 KB D2H), at the real dims, without the
	// 2.5-minute CSV parse — one attempt ≈ seconds, so the race's probability
	// is measurable instead of anecdotal. SETUP_RACE=<iters>; PRECOMMIT=0
	// skips the probed precommit for A/B.
	if let Some(it) = std::env::var_os("SETUP_RACE") {
		let iters: usize = it.to_string_lossy().parse().expect("SETUP_RACE parse");
		gpu_core::hip::set_device(0)?;
		for i in 0..iters {
			// The LLM scenario's real dims: x [45982×768], cat [45982×128].
			let x = ndarray::Array2::<f64>::from_elem((45982, 768), 1.0);
			let cat = ndarray::Array2::<f64>::from_elem((45982, 128), 1.0);
			// One staged image + ONE H2D for the whole setup (features + categorical),
			// the fit's single-upload arc; xraw/craw are views of that one buffer.
			let mut stage = gpu_core::memory::Stage::new();
			let x_off = stage.push(x.as_standard_layout().as_slice().expect("x contig"));
			let cat_off = stage.push(cat.as_standard_layout().as_slice().expect("cat contig"));
			let staged = stage.upload().expect("setup-race stage");
			let xraw = staged.view(x_off, x.len());
			let craw = staged.view(cat_off, cat.len());
			let (nn, cc) = (cat.nrows(), cat.ncols());
			// z-score the categorical block into carved views (device-only, no readback).
			let eps = {
				let e = gpu_core::memory::GpuBuffer::alloc(1).expect("eps");
				e.load(&[recipe_infer::ZSCORE_EPS]).expect("eps load");
				e
			};
			let mean = gpu_core::memory::GpuBuffer::alloc(cc).expect("mean");
			let std = gpu_core::memory::GpuBuffer::alloc(cc).expect("std");
			let xb = gpu_core::memory::GpuBuffer::alloc(nn * cc).expect("zscored");
			recipe_infer::zscore_fit_into(&craw, nn, cc, &eps, &mean, &std, &xb);
			// The rest of the real setup arc: OOC residents at LLM scale
			// (lse/dsum are n·heads·S ≈ 1.1 GB each), then fill VRAM to refusal
			// in window-sized steps like Ooc::build's place(), then one more
			// transfer under a full card — the exact conditions of the crash.
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
		eprintln!("       recipe detect <path>");
		eprintln!("       recipe serve            # discovery + RPC node on 7845");
		eprintln!("       recipe peers            # live network view");
		eprintln!("       recipe install          # probe + config.ogdl + systemd unit");
		std::process::exit(1);
	}

	// recipe serve — headless RPC + discovery node, ALWAYS on 7845 (no override).
	// Probes this machine's device table once at start, writes + broadcasts it as
	// this host's config.ogdl entry; peers heard append to the same registry file.
	// STORE/FETCH/STAT are live as the remote-storage tier; compute runners
	// (MOE_FFN) register into the same map once the gemma expert path is wired.
	// A storage node (no GPU) probes CPU/disk only — no device is ever inited.
	if args[1] == "serve" {
		// Bind FIRST: a port conflict must fail in milliseconds, not after a
		// full GPU probe (a crashlooping unit re-probing every 2s pegs the card).
		let listener = std::net::TcpListener::bind(("0.0.0.0", recipe::wire::PORT))?;
		let machine = recipe::probe::Machine::probe()?;
		let info = recipe::wire::NodeInfo::probe();
		let runners = std::collections::HashMap::new();
		recipe::wire::Server::new(info, runners)
			.machine(machine)
			.serve_bound(listener)?;
		return Ok(());
	}

	// recipe install — probe this machine, write its config.ogdl entry, generate
	// the systemd user unit from it, and copy the running binary to ~/.local/bin.
	if args[1] == "install" {
		let machine = recipe::probe::Machine::probe()?;
		recipe::probe::install(&machine)?;
		return Ok(());
	}

	// recipe peers — the live network view from this machine's own daemon.
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
			let kinds = pantry::predict_kinds(&columns);
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
		recipe::Data::load().set(path).target(t)
	} else {
		let d = recipe::Data::load().set(path);
		eprintln!("no --target specified");
		d
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

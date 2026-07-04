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
			let scaler = std::cell::RefCell::new(None);
			let x = ndarray::Array2::<f64>::from_elem((45982, 768), 1.0);
			let (xraw, _, _) = recipe_infer::upload(&x);
			let cat = ndarray::Array2::<f64>::from_elem((45982, 128), 1.0);
			let (craw, nn, cc) = recipe_infer::upload(&cat);
			let xb = recipe_infer::zscore_fit(&craw, nn, cc, &scaler);
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
			drop((xraw, craw, xb, lse, dsum, fills));
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
		std::process::exit(1);
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

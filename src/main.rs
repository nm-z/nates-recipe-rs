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
	eprintln!(
		"loaded {} samples × {} features",
		data.set.x.nrows(),
		data.set.x.ncols()
	);

	gpu_core::kernels::gpu_shutdown();
	Ok(())
}

//! Standalone GPU-only column-type detector. Point it at any dataset
//! (CSV / ARFF / dir / zip) and it prints each column → datatype.
//! Links only `pantry` + `recipe-infer` + the embedded detector weights —
//! no training framework.
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
	let Some(path) = args.get(1) else {
		eprintln!("usage: detect <path>   (csv / arff / dir / zip)");
		std::process::exit(1);
	};

	recipe_infer::init()?;

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

	recipe_infer::shutdown();
	Ok(())
}

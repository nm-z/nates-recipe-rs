//! Standalone GPU-only column-type detector. Point it at any dataset
//! (CSV / ARFF / dir / zip) and it prints each column → datatype.
//! Links only `pantry` + `recipe-infer` + the embedded detector weights —
//! no training framework.
use recipe_infer::log::{Opt, Write, data, set_opt};
use anyhow::Result;
use std::env;
use std::process;

fn kind_name(k: usize) -> &'static str {
	["Numeric", "Temporal", "Categorical", "Ordinal", "Text"]
		.get(k)
		.copied()
		.unwrap_or("Image")
}

fn main() -> Result<()> {
	set_opt(Opt { data: true, ..Opt::default() });
	let paths: Vec<String> = env::args().skip(1).collect();
	let Some(_probe) = paths.first() else {
		drop(Write::err("usage: detect <path>...   (csv / arff / dir / zip; globs expand to many)"));
		process::exit(1);
	};

	recipe_infer::init()?;

	let multi = paths.get(1);
	for path in &paths {
		for _extra in multi.into_iter() {
			Write::line(data, "");
			Write::line(data, format!("# {path}"));
		}
		for group in pantry::data::load_groups(path) {
			let pantry::data::DirGroup::Table {
				name,
				headers,
				cells,
				..
			} = group
			else {
				continue;
			};
			let columns: Vec<Vec<&str>> = (0..headers.len())
				.map(|j| {
					cells.iter()
						.filter_map(|r| r.get(j).map(String::as_str))
						.filter(|c| !c.is_empty())
						.collect()
				})
				.collect();
			let kinds = pantry::predict_kinds(&columns)?;
			let prefix = match name.chars().next() {
				Some(_first) => format!("{name}:"),
				None => String::new(),
			};
			for idx in 0..headers.len().min(kinds.len()) {
				let h = &headers[idx];
				let k = kinds[idx];
				Write::line(data, format!("{prefix}{h} -> {}", kind_name(k)));
			}
		}
	}

	recipe_infer::shutdown();
	Ok(())
}

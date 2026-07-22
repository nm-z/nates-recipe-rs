use anyhow::Result;
use ogdl::log::{Opt, Write, data, set_opt};
use std::env;
use std::process::ExitCode;

fn kind_name(k: usize) -> &'static str {
	["Numeric", "Temporal", "Categorical", "Ordinal", "Text"]
		.get(k)
		.copied()
		.unwrap_or("Image")
}

fn main() -> Result<ExitCode> {
	set_opt(Opt {
		data: true,
		..Opt::default()
	});
	let paths: Vec<String> = env::args().skip(1).collect();
	let Some(_probe) = paths.first() else {
		Write::error(
			"usage: detect <path>...   (csv / arff / dir / zip; globs expand to many)",
		);
		return Ok(ExitCode::from(1));
	};

	recipe_infer::init()?;

	let multi = paths.get(1);
	for path in &paths {
		if multi.is_some() {
			Write::line(data, "");
			Write::line(data, format!("# {path}"));
		}
		for group in pantry::data::load_groups(path)? {
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
	return Ok(ExitCode::SUCCESS);
}

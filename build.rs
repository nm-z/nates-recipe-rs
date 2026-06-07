fn main() {
	let rocm = std::env::var("ROCM_PATH").unwrap_or_else(|_| "/opt/rocm".to_string());
	let rocm_extra = std::env::var("ROCM_EXTRA_LIB").unwrap_or_else(|_| format!("{rocm}/lib"));
	println!("cargo:rustc-link-search=native={rocm}/lib");
	println!("cargo:rustc-link-lib=dylib=amdhip64");
	println!("cargo:rustc-link-search=native={rocm_extra}");
	println!("cargo:rustc-link-lib=dylib=rocblas");
	println!("cargo:rustc-link-lib=dylib=rocsolver");
	println!("cargo:rustc-link-lib=dylib=stdc++");
	ban_sync_alloc();
}

fn ban_sync_alloc() {
	let banned = ["hipMalloc(", "hipFree("];
	let allowed = ["hipMallocAsync", "hipFreeAsync", "hipMallocManaged", "fn hipMalloc", "fn hipFree"];
	for entry in walkdir("src") {
		let text = std::fs::read_to_string(&entry).unwrap_or_default();
		for (lineno, line) in text.lines().enumerate() {
			let trimmed = line.trim();
			if trimmed.starts_with("//") { continue; }
			for pat in &banned {
				if line.contains(pat) && !allowed.iter().any(|a| line.contains(a)) {
					panic!(
						"{}:{}: synchronous {} banned in training crate — use hipMallocAsync/hipFreeAsync",
						entry, lineno + 1, pat.trim_end_matches('('),
					);
				}
			}
		}
	}
}

fn walkdir(dir: &str) -> Vec<String> {
	let mut out = Vec::new();
	let Ok(rd) = std::fs::read_dir(dir) else { return out; };
	for e in rd.flatten() {
		let p = e.path();
		if p.is_dir() {
			out.extend(walkdir(p.to_str().unwrap_or_default()));
		} else if p.extension().is_some_and(|e| e == "rs") {
			out.push(p.to_string_lossy().into_owned());
			println!("cargo:rerun-if-changed={}", p.display());
		}
	}
	out
}

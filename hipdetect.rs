// Shared by every build.rs via include!(). Backend truth comes from hipconfig
// alone — runtime detection, not hardware: no env overrides, no filesystem
// probes, no hardcoded default. `hipconfig --platform` names the platform; an
// empty answer defers to `hipconfig --runtime` (rocclr = AMD, cuda = NVIDIA),
// i.e. whichever HIP runtime is installed. Anything else fails the build with
// the raw output.
#[derive(Clone, Copy)]
enum Platform {
	Amd,
	Nvidia,
}

fn hipconfig(flag: &str) -> String {
	let out = std::process::Command::new("hipconfig")
		.arg(flag)
		.output()
		.unwrap_or_else(|e| panic!("hipconfig {flag}: cannot run: {e}"));
	assert!(
		out.status.success(),
		"hipconfig {flag}: {}",
		String::from_utf8_lossy(&out.stderr)
	);
	String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn detect_platform() -> Platform {
	match hipconfig("--platform").as_str() {
		"amd" => Platform::Amd,
		"nvidia" => Platform::Nvidia,
		"" => match hipconfig("--runtime").as_str() {
			"rocclr" => Platform::Amd,
			"cuda" => Platform::Nvidia,
			other => panic!("hipconfig --runtime: unrecognized output {other:?}"),
		},
		other => panic!("hipconfig --platform: unrecognized output {other:?}"),
	}
}

fn rocm_path() -> String {
	let p = hipconfig("--rocmpath");
	assert!(!p.is_empty(), "hipconfig --rocmpath: empty output");
	p
}

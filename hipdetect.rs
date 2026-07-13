// Shared by every build.rs via include!(). Backend truth comes from hipconfig
// alone — runtime detection, not hardware: no env overrides, no filesystem
// probes, no hardcoded default. Installing is the package manager's job, never
// the build's: a missing or undecided HIP runtime fails the build with the
// package names.
#[derive(Clone, Copy)]
enum Platform {
	Amd,
	Nvidia,
}

fn hipconfig(flag: &str) -> String {
	let out = match std::process::Command::new("hipconfig").arg(flag).output() {
		Ok(out) => out,
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
			panic!("hipconfig not found; install hip-runtime-amd or hip-runtime-nvidia")
		}
		Err(e) => panic!("hipconfig {flag}: cannot run: {e}"),
	};
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
		other => panic!(
			"hipconfig --platform returned {other:?}; install hip-runtime-amd or hip-runtime-nvidia"
		),
	}
}

fn rocm_path() -> String {
	let p = hipconfig("--rocmpath");
	assert!(!p.is_empty(), "hipconfig --rocmpath: empty output");
	p
}

fn main() {
	let out = std::process::Command::new("git")
		.args(["rev-parse", "--short", "HEAD"])
		.output()
		.unwrap_or_else(|e| panic!("git rev-parse: {e}"));
	assert!(
		out.status.success(),
		"git rev-parse --short HEAD failed: {}",
		String::from_utf8_lossy(&out.stderr)
	);
	println!(
		"cargo:rustc-env=RECIPE_GIT_HASH={}",
		String::from_utf8_lossy(&out.stdout).trim()
	);
	let gd = std::process::Command::new("git")
		.args(["rev-parse", "--absolute-git-dir"])
		.output()
		.expect("git rev-parse --git-dir");
	println!(
		"cargo:rerun-if-changed={}/HEAD",
		String::from_utf8_lossy(&gd.stdout).trim()
	);
}

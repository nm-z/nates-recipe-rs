#[cfg(feature = "legacy")]
fn main() -> anyhow::Result<std::process::ExitCode> {
	recipe::cli::main()
}

#[cfg(feature = "next")]
fn main() -> std::process::ExitCode {
	recipe::cli::main()
}

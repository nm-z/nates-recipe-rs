//! Machine probe → beacon → config.ogdl registry. The probe MEASURES this box's
//! device table once (sizes are queries, rates are timed one-shots), the beacon
//! (wire.rs) TRANSMITS it, and the daemon WRITES the OGDL tree — a live registry
//! of detected machines, the DHCP-writes-/etc/hosts pattern. Everything here is
//! pure data + timed benches; wire.rs holds/ships a `Machine` without ever
//! touching the GPU (only `Machine::probe` does).

use anyhow::{anyhow, bail, Result};
use gpu_core::memory::{par_copy, par_touch, GpuBuffer};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

// ── device table ─────────────────────────────────────────────────────────────
/// One GPU's measured resources. Sizes queried, rates timed at probe.
#[derive(Clone, Debug, PartialEq)]
/// All rates are PAYLOAD throughput — the number `size / bandwidth = time`
/// divides by (a D2D copy's bus traffic is 2x its payload; the scheduler
/// never needs the bus figure).
pub struct GpuDev {
	pub vram: u64,          // total bytes (hipMemGetInfo)
	pub pcie_gbs: f64,      // H2D upload GB/s
	pub flops_gflops: f64,  // f64 GEMM GFLOP/s
	pub transfer_gbs: f64,  // device-to-device copy GB/s
}

/// One machine's full device table — the per-host entry in config.ogdl. A box
/// with no GPU carries an empty `gpus` (absence, not zeros).
#[derive(Clone, Debug, PartialEq)]
pub struct Machine {
	pub host: String,
	pub gpus: Vec<GpuDev>,
	pub ram: u64,          // MemTotal bytes
	pub ddr5_gbs: f64,     // host memcpy GB/s
	pub cpu_gflops: f64,   // threaded f64 FMA GFLOP/s
	pub disk_size: u64,    // filesystem total bytes (statvfs)
	pub sata_gbs: f64,     // disk read GB/s (post cache-drop)
}

impl Machine {
	/// Measure THIS machine's device table. One-shot at daemon start: sizes are
	/// runtime queries, rates are timed production paths (few iters, best time).
	/// GPU presence is detected via the HIP device count — zero devices means no
	/// GPU block at all, and no device is ever initialised on a storage node.
	pub fn probe() -> Result<Machine> {
		let host = hostname();
		let ngpu = gpu_core::hip::device_count().unwrap_or(0).max(0) as usize;
		let mut gpus = Vec::with_capacity(ngpu);
		for d in 0..ngpu as i32 {
			eprintln!("recipe probe: measuring gpu{d}");
			// Disposable child per device: an unsupported arch can WEDGE inside the
			// driver (ROCm 7 on gfx900 hangs, not errors), so the bench runs in a
			// child the parent can kill — hang/abort/refusal all read the same way:
			// not drivable, storage node. Loud, never silent, never a crashloop.
			match measure_gpu_child(d) {
				Ok(g) => gpus.push(g),
				Err(e) => eprintln!("recipe probe: gpu{d} not drivable by this binary ({e}) — storage node"),
			}
		}
		// Free what the benches allocated: the probe's first alloc birthed the
		// process claim (~all VRAM); an idle daemon must hold ~nothing.
		if ngpu > 0 {
			gpu_core::hip::set_device(0)?;
			// Free the benches' birth claim AND trim: the freed claim otherwise sits
			// as retained pool slack (threshold=max) and the idle daemon squats VRAM.
			gpu_core::memory::release_run_backing();
			gpu_core::memory::pool_trim();
		}
		let ram = mem_total()?;
		eprintln!("recipe probe: measuring cpu (ddr5 + flops)");
		let ddr5_gbs = bench_ddr5();
		let cpu_gflops = bench_cpu_flops();
		let dd = data_dir();
		let disk_size = disk_total(&dd)?;
		eprintln!("recipe probe: measuring disk (sata)");
		let sata_gbs = bench_disk(&dd)?;
		Ok(Machine { host, gpus, ram, ddr5_gbs, cpu_gflops, disk_size, sata_gbs })
	}

	/// Compact single-line beacon body (no interior newline — wire.rs appends it
	/// after the NodeInfo block and splits it back off by line):
	///   host|ram|ddr5|cpu_gflops|disk_size|sata|ngpu|(vram|pcie|flops|transfer)*
	pub fn beacon_encode(&self) -> String {
		let mut parts = vec![
			self.host.clone(),
			self.ram.to_string(),
			self.ddr5_gbs.to_string(),
			self.cpu_gflops.to_string(),
			self.disk_size.to_string(),
			self.sata_gbs.to_string(),
			self.gpus.len().to_string(),
		];
		for g in &self.gpus {
			parts.push(g.vram.to_string());
			parts.push(g.pcie_gbs.to_string());
			parts.push(g.flops_gflops.to_string());
			parts.push(g.transfer_gbs.to_string());
		}
		parts.join("|")
	}

	pub fn beacon_decode(s: &str) -> Result<Machine> {
		let f: Vec<&str> = s.split('|').collect();
		if f.len() < 7 {
			bail!("probe: short beacon machine ({} fields)", f.len());
		}
		let ngpu: usize = f[6].parse()?;
		let mut gpus = Vec::with_capacity(ngpu);
		for i in 0..ngpu {
			let b = 7 + i * 4;
			if b + 4 > f.len() {
				bail!("probe: truncated gpu{i} in beacon");
			}
			gpus.push(GpuDev {
				vram: f[b].parse()?,
				pcie_gbs: f[b + 1].parse()?,
				flops_gflops: f[b + 2].parse()?,
				transfer_gbs: f[b + 3].parse()?,
			});
		}
		Ok(Machine {
			host: f[0].to_string(),
			gpus,
			ram: f[1].parse()?,
			ddr5_gbs: f[2].parse()?,
			cpu_gflops: f[3].parse()?,
			disk_size: f[4].parse()?,
			sata_gbs: f[5].parse()?,
		})
	}
}

// ── local queries ────────────────────────────────────────────────────────────
fn hostname() -> String {
	std::fs::read_to_string("/proc/sys/kernel/hostname")
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

fn mem_total() -> Result<u64> {
	let s = std::fs::read_to_string("/proc/meminfo")?;
	for l in s.lines() {
		if let Some(rest) = l.strip_prefix("MemTotal:") {
			let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse()?;
			return Ok(kb * 1024);
		}
	}
	bail!("probe: MemTotal missing from /proc/meminfo")
}

fn disk_total(path: &Path) -> Result<u64> {
	let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())?;
	let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
	if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
		bail!("probe: statvfs failed for {}", path.display());
	}
	Ok((st.f_blocks as u64).saturating_mul(st.f_frsize as u64))
}

// ── GPU benches (device i current) ───────────────────────────────────────────

/// Run `measure_gpu(dev)` in a disposable child (RECIPE_PROBE_GPU hook in main)
/// with a hard timeout; the child prints "vram|pcie|flops|transfer" on success.
fn measure_gpu_child(dev: i32) -> Result<GpuDev> {
	let exe = std::env::current_exe()?;
	let mut child = std::process::Command::new(exe)
		.env("RECIPE_PROBE_GPU", dev.to_string())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::null())
		.spawn()?;
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
	loop {
		if let Some(status) = child.try_wait()? {
			if !status.success() {
				bail!("probe child exited {status}");
			}
			let mut out = String::new();
			use std::io::Read as _;
			child.stdout.take().expect("probe child stdout").read_to_string(&mut out)?;
			let f: Vec<&str> = out.trim().split('|').collect();
			if f.len() != 4 {
				bail!("probe child output malformed: {out:?}");
			}
			return Ok(GpuDev {
				vram: f[0].parse()?,
				pcie_gbs: f[1].parse()?,
				flops_gflops: f[2].parse()?,
				transfer_gbs: f[3].parse()?,
			});
		}
		if std::time::Instant::now() > deadline {
			let _ = child.kill();
			let _ = child.wait();
			bail!("probe child wedged (120s) — killed");
		}
		std::thread::sleep(std::time::Duration::from_millis(200));
	}
}

/// The RECIPE_PROBE_GPU child body: measure one device, print the fields, exit.
pub fn probe_gpu_child_main(dev: i32) -> ! {
	match measure_gpu(dev) {
		Ok(g) => {
			println!("{}|{}|{}|{}", g.vram, g.pcie_gbs, g.flops_gflops, g.transfer_gbs);
			std::process::exit(0);
		}
		Err(e) => {
			eprintln!("probe child gpu{dev}: {e}");
			std::process::exit(2);
		}
	}
}

fn measure_gpu(dev: i32) -> Result<GpuDev> {
	gpu_core::hip::set_device(dev)?;
	let (_free, total) = gpu_core::hip::mem_info()?;
	Ok(GpuDev {
		vram: total as u64,
		pcie_gbs: bench_pcie_h2d()?,
		flops_gflops: bench_gemm()?,
		transfer_gbs: bench_transfer()?,
	})
}

// One 64 MiB pinned-bounce H2D through the real choke, timed; best of a few.
fn bench_pcie_h2d() -> Result<f64> {
	let bytes = 64usize << 20;
	let host = vec![0u8; bytes];
	let dev = GpuBuffer::alloc_bytes(bytes)?;
	let mut best = f64::INFINITY;
	for _ in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		dev.write_u8(&host)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(bytes as f64 / best / 1e9)
}

// One saturating 2048³ f64 GEMM through the production entry, timed.
fn bench_gemm() -> Result<f64> {
	let (m, n, k) = (2048usize, 2048usize, 2048usize);
	let x = GpuBuffer::alloc(m * k)?;
	let w = GpuBuffer::alloc(k * n)?;
	let bias = GpuBuffer::alloc(n)?;
	let out = GpuBuffer::alloc(m * n)?;
	x.memset_zero(m * k * 8)?;
	w.memset_zero(k * n * 8)?;
	bias.memset_zero(n * 8)?;
	let flop = 2.0 * m as f64 * n as f64 * k as f64;
	let mut best = f64::INFINITY;
	for _ in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		gpu_core::kernels::gpu_linear_into(&x, &w, &bias, m, n, k, &out)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(flop / best / 1e9)
}

// One 256 MiB device-to-device copy (payload throughput), timed.
fn bench_transfer() -> Result<f64> {
	let n = 32usize << 20; // 32M f64 = 256 MiB
	let src = GpuBuffer::alloc(n)?;
	let dst = GpuBuffer::alloc(n)?;
	src.memset_zero(n * 8)?;
	let bytes = n * 8;
	let mut best = f64::INFINITY;
	for _ in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		gpu_core::kernels::gpu_copy_into(&src, n, &dst)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(bytes as f64 / best / 1e9)
}

// ── CPU / disk benches ───────────────────────────────────────────────────────
// Multi-threaded 1 GiB host memcpy, payload GB/s, best of a few.
fn bench_ddr5() -> f64 {
	let bytes = 1usize << 30;
	let mut src = vec![0u8; bytes];
	let mut dst = vec![0u8; bytes];
	par_touch(&mut src);
	par_touch(&mut dst);
	let mut best = f64::INFINITY;
	for _ in 0..5 {
		let t = Instant::now();
		par_copy(dst.as_mut_ptr(), src.as_ptr(), bytes);
		best = best.min(t.elapsed().as_secs_f64());
	}
	bytes as f64 / best / 1e9
}

/// The disk device's ONE home: ~/.cache/recipe — the probe measures THIS
/// filesystem and the runtime spills to it, so SIZE/SATA are the numbers the
/// scheduler's disk tier actually gets, independent of the process cwd.
pub fn data_dir() -> std::path::PathBuf {
	let base = std::env::var("XDG_CACHE_HOME")
		.map(std::path::PathBuf::from)
		.unwrap_or_else(|_| {
			std::path::PathBuf::from(std::env::var("HOME").expect("HOME unset")).join(".cache")
		});
	let dir = base.join("recipe");
	std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("data_dir {}: {e}", dir.display()));
	dir
}

// One f64 FMA per iter across all cores; black_box the operand so the chain is
// not folded away. 2 flops per FMA.
fn bench_cpu_flops() -> f64 {
	// 8 independent FMA lanes per thread: a single serial chain measures FMA
	// LATENCY (one op waiting on the last); independent lanes fill the pipeline
	// and vectorize, measuring the compute the scheduler can actually buy.
	let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
	let iters: u64 = 100_000_000;
	let t = Instant::now();
	let sum: f64 = std::thread::scope(|sc| {
		let handles: Vec<_> = (0..threads)
			.map(|kk| {
				sc.spawn(move || {
					let mut lanes = [0.5_f64 + kk as f64 * 1e-9; 8];
					for (i, l) in lanes.iter_mut().enumerate() {
						*l += i as f64 * 1e-9;
					}
					let b = std::hint::black_box(0.9999999_f64);
					let c = std::hint::black_box(0.0000001_f64);
					for _ in 0..iters {
						for l in lanes.iter_mut() {
							// plain mul+add: baseline x86-64 has no FMA instruction, so
							// mul_add is a libm CALL — this vectorizes, that doesn't.
							*l = *l * b + c;
						}
					}
					lanes.iter().sum::<f64>()
				})
			})
			.collect();
		handles.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
	});
	std::hint::black_box(sum);
	let flops = threads as f64 * iters as f64 * 8.0 * 2.0;
	flops / t.elapsed().as_secs_f64() / 1e9
}

// Write+read 256 MiB with the page cache dropped between, so the read is a real
// disk read. SATA = that sustained read GB/s.
fn bench_disk(dir: &Path) -> Result<f64> {
	let bytes = 256usize << 20;
	let mut buf = vec![0u8; bytes];
	par_touch(&mut buf);
	let path = dir.join(".recipe_probe");
	let f = OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(&path)?;
	let tw = Instant::now();
	f.write_all_at(&buf, 0)?;
	f.sync_all()?;
	drop_cache(&f, 0, bytes);
	let write_gbs = bytes as f64 / tw.elapsed().as_secs_f64() / 1e9;
	let tr = Instant::now();
	f.read_exact_at(&mut buf, 0)?;
	let read_gbs = bytes as f64 / tr.elapsed().as_secs_f64() / 1e9;
	drop(f);
	let _ = std::fs::remove_file(&path);
	eprintln!("recipe probe: disk write {write_gbs:.3} GB/s, read {read_gbs:.3} GB/s");
	Ok(read_gbs)
}

// Force the range to disk and drop it from cache — the probe file is never hot.
fn drop_cache(f: &File, off: u64, len: usize) {
	use std::os::unix::io::AsRawFd;
	unsafe {
		libc::sync_file_range(
			f.as_raw_fd(),
			off as i64,
			len as i64,
			libc::SYNC_FILE_RANGE_WAIT_BEFORE
				| libc::SYNC_FILE_RANGE_WRITE
				| libc::SYNC_FILE_RANGE_WAIT_AFTER,
		);
		libc::posix_fadvise(f.as_raw_fd(), off as i64, len as i64, libc::POSIX_FADV_DONTNEED);
	}
}

// ── config.ogdl registry ─────────────────────────────────────────────────────
// The tab-nested OGDL tree — the human-readable source of truth. Sizes are raw
// bytes (exact, round-trips); rates are GB/s (GFLOP/s for FLOPs). A GPU-less
// machine emits no gpu block.
pub fn write_config(machines: &[Machine]) -> String {
	let mut s = String::from("machines\n");
	for m in machines {
		s.push_str(&format!("\t{}\n", m.host));
		for (i, g) in m.gpus.iter().enumerate() {
			s.push_str(&format!("\t\tgpu{i}\n"));
			s.push_str(&format!("\t\t\tVRAM\t{}\n", g.vram));
			s.push_str(&format!("\t\t\tPCIe\t{:.3}\n", g.pcie_gbs));
			s.push_str(&format!("\t\t\tFLOPs\t{:.1}\n", g.flops_gflops));
			s.push_str(&format!("\t\t\tTransfer\t{:.3}\n", g.transfer_gbs));
		}
		s.push_str("\t\tcpu\n");
		s.push_str(&format!("\t\t\tRAM\t{}\n", m.ram));
		s.push_str(&format!("\t\t\tDDR5\t{:.3}\n", m.ddr5_gbs));
		s.push_str(&format!("\t\t\tFLOPs\t{:.1}\n", m.cpu_gflops));
		s.push_str("\t\tdisk\n");
		s.push_str(&format!("\t\t\tSIZE\t{}\n", m.disk_size));
		s.push_str(&format!("\t\t\tSATA\t{:.3}\n", m.sata_gbs));
	}
	s
}

// Read the tree back into device tables (the scheduler's view). Depth is the
// leading-tab count; a field line is `key<ws>value` (value never has interior
// whitespace here). Malformed lines are skipped — a partial file is not fatal.
pub fn parse_config(text: &str) -> Vec<Machine> {
	enum Sect {
		None,
		Gpu(usize),
		Cpu,
		Disk,
	}
	let mut machines: Vec<Machine> = Vec::new();
	let mut cur: Option<Machine> = None;
	let mut sect = Sect::None;
	for raw in text.lines() {
		if raw.trim().is_empty() {
			continue;
		}
		let depth = raw.chars().take_while(|c| *c == '\t').count();
		let line = raw.trim_start_matches('\t');
		match depth {
			0 => {} // "machines" root
			1 => {
				if let Some(m) = cur.take() {
					machines.push(m);
				}
				cur = Some(Machine {
					host: line.trim().to_string(),
					gpus: Vec::new(),
					ram: 0,
					ddr5_gbs: 0.0,
					cpu_gflops: 0.0,
					disk_size: 0,
					sata_gbs: 0.0,
				});
				sect = Sect::None;
			}
			2 => {
				let name = line.trim();
				if let Some(m) = cur.as_mut() {
					if name == "cpu" {
						sect = Sect::Cpu;
					} else if name == "disk" {
						sect = Sect::Disk;
					} else if name.starts_with("gpu") {
						m.gpus.push(GpuDev {
							vram: 0,
							pcie_gbs: 0.0,
							flops_gflops: 0.0,
							transfer_gbs: 0.0,
						});
						sect = Sect::Gpu(m.gpus.len() - 1);
					}
				}
			}
			_ => {
				let Some((k, v)) = line.split_once(char::is_whitespace) else {
					continue;
				};
				let v = v.trim();
				let Some(m) = cur.as_mut() else { continue };
				match sect {
					Sect::Gpu(i) => {
						if let Some(g) = m.gpus.get_mut(i) {
							match k {
								"VRAM" => g.vram = v.parse().unwrap_or(0),
								"PCIe" => g.pcie_gbs = v.parse().unwrap_or(0.0),
								"FLOPs" => g.flops_gflops = v.parse().unwrap_or(0.0),
								"Transfer" => g.transfer_gbs = v.parse().unwrap_or(0.0),
								_ => {}
							}
						}
					}
					Sect::Cpu => match k {
						"RAM" => m.ram = v.parse().unwrap_or(0),
						"DDR5" => m.ddr5_gbs = v.parse().unwrap_or(0.0),
						"FLOPs" => m.cpu_gflops = v.parse().unwrap_or(0.0),
						_ => {}
					},
					Sect::Disk => match k {
						"SIZE" => m.disk_size = v.parse().unwrap_or(0),
						"SATA" => m.sata_gbs = v.parse().unwrap_or(0.0),
						_ => {}
					},
					Sect::None => {}
				}
			}
		}
	}
	if let Some(m) = cur.take() {
		machines.push(m);
	}
	machines
}

fn config_dir() -> Result<PathBuf> {
	if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
		return Ok(PathBuf::from(xdg).join("recipe"));
	}
	let home = std::env::var("HOME").map_err(|_| anyhow!("probe: HOME not set"))?;
	Ok(PathBuf::from(home).join(".config/recipe"))
}

pub fn config_path() -> Result<PathBuf> {
	Ok(config_dir()?.join("config.ogdl"))
}

/// Rewrite config.ogdl atomically (temp + rename), creating parents.
pub fn write_config_atomic(machines: &[Machine]) -> Result<()> {
	let path = config_path()?;
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	let tmp = path.with_extension("ogdl.tmp");
	std::fs::write(&tmp, write_config(machines))?;
	std::fs::rename(&tmp, &path)?;
	Ok(())
}

// ── systemd unit + install ───────────────────────────────────────────────────
/// The .ogdl → .service converter. Today the unit is the owner's fixed sample
/// (the config is the input so a future version can template limits from it).
pub fn service_unit(_config_ogdl: &str) -> String {
	[
		"[Unit]",
		"Description=recipe distributed training node",
		"After=network-online.target",
		"Wants=network-online.target",
		"",
		"[Service]",
		"ExecStart=%h/.local/bin/recipe serve",
		"Restart=always",
		"RestartSec=2",
		"",
		"[Install]",
		"WantedBy=default.target",
		"",
	]
	.join("\n")
}

fn service_path() -> Result<PathBuf> {
	let home = std::env::var("HOME").map_err(|_| anyhow!("probe: HOME not set"))?;
	Ok(PathBuf::from(home).join(".config/systemd/user/recipe.service"))
}

/// `recipe install`: write this machine's config.ogdl entry, generate the
/// systemd user unit from it, and copy the running binary to ~/.local/bin.
/// A permission failure on the system path is loud with the exact sudo command.
pub fn install(machine: &Machine) -> Result<()> {
	let cfg_text = write_config(std::slice::from_ref(machine));
	write_config_atomic(std::slice::from_ref(machine))?;
	eprintln!("recipe install: wrote {}", config_path()?.display());

	let svc = service_path()?;
	if let Some(p) = svc.parent() {
		std::fs::create_dir_all(p)?;
	}
	std::fs::write(&svc, service_unit(&cfg_text))?;
	eprintln!("recipe install: wrote {}", svc.display());

	let exe = std::env::current_exe()?;
	let home = std::env::var("HOME").expect("HOME unset");
	let bin_dir = Path::new(&home).join(".local/bin");
	std::fs::create_dir_all(&bin_dir).map_err(|e| anyhow::anyhow!("recipe install: mkdir {}: {e}", bin_dir.display()))?;
	let dest = bin_dir.join("recipe");
	let dest = dest.as_path();
	// Running FROM the install path: nothing to copy (self-copy is ETXTBSY).
	if std::fs::canonicalize(&exe).ok() == std::fs::canonicalize(dest).ok() {
		eprintln!("recipe install: {} is already the installed binary", dest.display());
		return Ok(());
	}
	match std::fs::copy(&exe, dest) {
		Ok(_) => eprintln!("recipe install: copied {} -> {}", exe.display(), dest.display()),
		Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => bail!(
			"recipe install: cannot write {} ({e}); run:\n  sudo install -m755 {} {}",
			dest.display(),
			exe.display(),
			dest.display()
		),
		Err(e) => bail!("recipe install: copy {} -> {}: {e}", exe.display(), dest.display()),
	}
	eprintln!("recipe install: done — enable with: systemctl --user enable --now recipe.service");
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample(gpus: Vec<GpuDev>) -> Machine {
		Machine {
			host: "engi".to_string(),
			gpus,
			ram: 67_169_726_464,
			ddr5_gbs: 38.4,
			cpu_gflops: 89.2,
			disk_size: 500_107_862_016,
			sata_gbs: 1.9,
		}
	}

	// Host-only: the compact beacon line round-trips a Machine EXACTLY (full f64
	// precision), including the GPU-less shape (empty gpus, absence not zeros).
	#[test]
	fn beacon_roundtrips_exact() {
		let m = sample(vec![GpuDev {
			vram: 12_884_901_888,
			pcie_gbs: 11.581,
			flops_gflops: 254.7,
			transfer_gbs: 402.3,
		}]);
		assert_eq!(Machine::beacon_decode(&m.beacon_encode()).expect("decode"), m);

		let storage = sample(Vec::new());
		let back = Machine::beacon_decode(&storage.beacon_encode()).expect("decode");
		assert!(back.gpus.is_empty(), "storage node carries no gpu block");
		assert_eq!(back, storage);
	}

	// Host-only: the OGDL tree round-trips at its written precision (values chosen
	// to survive {:.3}/{:.1}). A GPU-less machine emits and parses back no gpu block.
	#[test]
	fn config_ogdl_roundtrips() {
		let a = sample(vec![GpuDev {
			vram: 12_884_901_888,
			pcie_gbs: 11.581,
			flops_gflops: 254.7,
			transfer_gbs: 402.3,
		}]);
		let mut b = sample(Vec::new());
		b.host = "archy".to_string();
		let text = write_config(&[a.clone(), b.clone()]);
		let parsed = parse_config(&text);
		assert_eq!(parsed.len(), 2);
		assert_eq!(parsed[0], a);
		assert_eq!(parsed[1], b);
		assert!(parsed[1].gpus.is_empty());
	}
}

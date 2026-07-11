use anyhow::{Result, anyhow, bail, ensure};
use gpu_core::memory::{GpuBuffer, par_copy, par_touch};
use std::cmp::Ordering;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub struct GpuDev {
	pub vram: u64,
	pub pcie_gbs: f64,
	pub flops_gflops: f64,
	pub transfer_gbs: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Machine {
	pub host: String,
	pub gpus: Vec<GpuDev>,
	pub ram: u64,
	pub ddr5_gbs: f64,
	pub cpu_transfer_gbs: f64,
	pub cpu_gflops: f64,
	pub disk_size: u64,
	pub sata_gbs: f64,
	pub eth_gbs: f64,
}

impl Machine {
	pub fn probe() -> Result<Machine> {
		let host = hostname();
		let job = gpu_core::gate::Lease::new();
		let ngpu = gpu_core::hip::device_count().unwrap_or(0).max(0) as usize;
		let mut gpus = Vec::with_capacity(ngpu);
		for d in 0..ngpu as i32 {
			eprintln!("recipe probe: measuring gpu{d}");
			match measure_gpu_child(d) {
				Ok(g) => gpus.push(g),
				Err(e) => eprintln!(
					"recipe probe: gpu{d} not drivable by this binary ({e}) — storage node"
				),
			}
		}
		for _present in (0..ngpu).next().into_iter() {
			gpu_core::hip::set_device(0)?;
			gpu_core::memory::release_run_backing();
			gpu_core::memory::pool_trim();
		}
		drop(job);
		let ram = mem_total()?;
		eprintln!("recipe probe: measuring cpu (ddr5 + transfer + flops)");
		let ddr5_gbs = bench_ddr5();
		let cpu_transfer_gbs = bench_cpu_read();
		let cpu_gflops = bench_cpu_flops();
		let dd = data_dir()?;
		let disk_size = disk_total(&dd)?;
		eprintln!("recipe probe: measuring disk (sata)");
		let sata_gbs = bench_disk(&dd)?;
		let eth_gbs = link_speed_gbs();
		eprintln!(
			"recipe probe: link {eth_gbs:.3} GB/s ({})",
			eth_label(eth_gbs)
		);
		Ok(Machine {
			host,
			gpus,
			ram,
			ddr5_gbs,
			cpu_transfer_gbs,
			cpu_gflops,
			disk_size,
			sata_gbs,
			eth_gbs,
		})
	}

	pub fn beacon_encode(&self) -> String {
		let mut parts = vec![
			self.host.clone(),
			self.ram.to_string(),
			self.ddr5_gbs.to_string(),
			self.cpu_transfer_gbs.to_string(),
			self.cpu_gflops.to_string(),
			self.disk_size.to_string(),
			self.sata_gbs.to_string(),
			self.eth_gbs.to_string(),
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
		ensure!(
			f.len() >= 9,
			"probe: short beacon machine ({} fields)",
			f.len()
		);
		let ngpu: usize = f[8].parse()?;
		let mut gpus = Vec::with_capacity(ngpu);
		for i in 0..ngpu {
			let b = 9 + i * 4;
			ensure!(b + 4 <= f.len(), "probe: truncated gpu{i} in beacon");
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
			cpu_transfer_gbs: f[3].parse()?,
			cpu_gflops: f[4].parse()?,
			disk_size: f[5].parse()?,
			sata_gbs: f[6].parse()?,
			eth_gbs: f[7].parse()?,
		})
	}
}

fn hostname() -> String {
	std::fs::read_to_string("/proc/sys/kernel/hostname")
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

fn mem_total() -> Result<u64> {
	let s = std::fs::read_to_string("/proc/meminfo")?;
	let rest = s
		.lines()
		.find_map(|l| l.strip_prefix("MemTotal:"))
		.ok_or_else(|| anyhow!("probe: MemTotal missing from /proc/meminfo"))?;
	let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse()?;
	Ok(kb * 1024)
}

fn disk_total(path: &Path) -> Result<u64> {
	let c = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())?;
	let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
	ensure!(
		unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0,
		"probe: statvfs failed for {}",
		path.display()
	);
	Ok((st.f_blocks as u64).saturating_mul(st.f_frsize as u64))
}

fn link_speed_gbs() -> f64 {
	let mut best_mbps = 0i64;
	for entries in std::fs::read_dir("/sys/class/net").into_iter() {
		for e in entries.flatten().filter(|e| e.file_name() != "lo") {
			let sp = e.path().join("speed");
			let m = std::fs::read_to_string(&sp)
				.ok()
				.and_then(|txt| txt.trim().parse::<i64>().ok())
				.unwrap_or(best_mbps);
			best_mbps = best_mbps.max(m);
		}
	}
	match best_mbps.cmp(&0) {
		Ordering::Greater => best_mbps as f64 / 8000.0,
		Ordering::Less | Ordering::Equal => 0.0,
	}
}

fn eth_label(gbs: f64) -> String {
	let mbps = (gbs * 8000.0).round() as i64;
	match mbps {
		100 => "100M".to_string(),
		1000 => "1GbE".to_string(),
		2500 => "2.5GbE".to_string(),
		5000 => "5GbE".to_string(),
		10000 => "10GbE".to_string(),
		25000 => "25GbE".to_string(),
		m => match m.cmp(&0) {
			Ordering::Greater => format!("{m}M"),
			Ordering::Less | Ordering::Equal => "none".to_string(),
		},
	}
}

fn measure_gpu_child(dev: i32) -> Result<GpuDev> {
	let exe = std::env::current_exe()?;
	let mut child = std::process::Command::new(exe)
		.env("RECIPE_PROBE_GPU", dev.to_string())
		.stdout(std::process::Stdio::piped())
		.stderr(std::process::Stdio::null())
		.spawn()?;
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
	loop {
		let Some(status) = child.try_wait()? else {
			match std::time::Instant::now().cmp(&deadline) {
				Ordering::Greater => {
					child.kill().ok();
					child.wait().ok();
					bail!("probe child wedged (120s) — killed");
				}
				Ordering::Less | Ordering::Equal => {
					std::thread::sleep(std::time::Duration::from_millis(200));
					continue;
				}
			}
		};
		ensure!(status.success(), "probe child exited {status}");
		let mut out = String::new();
		use std::io::Read;
		child.stdout
			.take()
			.ok_or_else(|| anyhow::anyhow!("probe child stdout"))?
			.read_to_string(&mut out)?;
		let f: Vec<&str> = out.trim().split('|').collect();
		ensure!(f.len() == 4, "probe child output malformed: {out:?}");
		return Ok(GpuDev {
			vram: f[0].parse()?,
			pcie_gbs: f[1].parse()?,
			flops_gflops: f[2].parse()?,
			transfer_gbs: f[3].parse()?,
		});
	}
}

pub fn probe_gpu_child_record(dev: i32) -> Result<String> {
	let g = measure_gpu(dev)?;
	Ok(format!(
		"{}|{}|{}|{}",
		g.vram, g.pcie_gbs, g.flops_gflops, g.transfer_gbs
	))
}

fn measure_gpu(dev: i32) -> Result<GpuDev> {
	gpu_core::hip::set_device(dev)?;
	let total = gpu_core::hip::mem_info()?.total;
	Ok(GpuDev {
		vram: total as u64,
		pcie_gbs: bench_pcie_h2d()?,
		flops_gflops: bench_gemm()?,
		transfer_gbs: bench_transfer()?,
	})
}

fn bench_pcie_h2d() -> Result<f64> {
	let bytes = 64usize << 20;
	let host = vec![0u8; bytes];
	let dev = GpuBuffer::alloc_bytes(bytes)?;
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		dev.write_u8(&host)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(bytes as f64 / best / 1e9)
}

fn bench_gemm() -> Result<f64> {
	let m = 2048usize;
	let n = 2048usize;
	let k = 2048usize;
	let x = GpuBuffer::alloc(m * k)?;
	let w = GpuBuffer::alloc(k * n)?;
	let bias = GpuBuffer::alloc(n)?;
	let out = GpuBuffer::alloc(m * n)?;
	x.memset_zero(m * k * 8)?;
	w.memset_zero(k * n * 8)?;
	bias.memset_zero(n * 8)?;
	let flop = 2.0 * m as f64 * n as f64 * k as f64;
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		gpu_core::kernels::gpu_linear_into(&x, &w, &bias, m, n, k, &out)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(flop / best / 1e9)
}

fn bench_transfer() -> Result<f64> {
	let n = 32usize << 20;
	let src = GpuBuffer::alloc(n)?;
	let dst = GpuBuffer::alloc(n)?;
	src.memset_zero(n * 8)?;
	let bytes = n * 8;
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		gpu_core::hip::device_synchronize()?;
		let t = Instant::now();
		gpu_core::kernels::gpu_copy_into(&src, n, &dst)?;
		gpu_core::hip::device_synchronize()?;
		best = best.min(t.elapsed().as_secs_f64());
	}
	Ok(bytes as f64 / best / 1e9)
}

fn bench_ddr5() -> f64 {
	let bytes = 1usize << 30;
	let mut src = vec![0u8; bytes];
	let mut dst = vec![0u8; bytes];
	par_touch(&mut src);
	par_touch(&mut dst);
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		let t = Instant::now();
		par_copy(dst.as_mut_ptr(), src.as_ptr(), bytes);
		best = best.min(t.elapsed().as_secs_f64());
	}
	bytes as f64 / best / 1e9
}

fn bench_cpu_read() -> f64 {
	let bytes = 1usize << 30;
	let mut buf = vec![0u8; bytes];
	par_touch(&mut buf);
	let threads = std::thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	let per = bytes.div_ceil(threads);
	let base = buf.as_ptr() as usize;
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		let t = Instant::now();
		let sum: u64 = std::thread::scope(|sc| {
			let mut handles = Vec::new();
			for k in 0..threads {
				handles.push(sc.spawn(move || {
					let off = k * per;
					match off.cmp(&bytes) {
						Ordering::Less => {
							let len = per.min(bytes - off);
							let mut acc = 0u64;
							for i in (off..off + len).step_by(64) {
								acc = acc.wrapping_add(unsafe {
									std::ptr::read_volatile(
										(base + i) as *const u8,
									)
								} as u64);
							}
							acc
						}
						Ordering::Greater | Ordering::Equal => 0u64,
					}
				}));
			}
			handles.into_iter().map(|h| h.join().unwrap_or(0)).sum()
		});
		std::hint::black_box(sum);
		best = best.min(t.elapsed().as_secs_f64());
	}
	bytes as f64 / best / 1e9
}

pub fn data_dir() -> anyhow::Result<std::path::PathBuf> {
	let base = match std::env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
		Some(x) => std::path::PathBuf::from(x),
		None => {
			let home = std::env::var_os("HOME")
				.filter(|v| !v.is_empty())
				.ok_or_else(|| anyhow::anyhow!("neither XDG_CACHE_HOME nor HOME is set"))?;
			std::path::PathBuf::from(home).join(".cache")
		}
	};
	let dir = base.join("recipe");
	std::fs::create_dir_all(&dir)
		.map_err(|e| anyhow::anyhow!("data_dir {}: {e}", dir.display()))?;
	Ok(dir)
}

fn bench_cpu_flops() -> f64 {
	let threads = std::thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	let iters: u64 = 100_000_000;
	let t = Instant::now();
	let sum: f64 = std::thread::scope(|sc| {
		let mut handles = Vec::new();
		for kk in 0..threads {
			handles.push(sc.spawn(move || {
				let mut lanes = [0.5_f64 + kk as f64 * 1e-9; 8];
				for i in 0..lanes.len() {
					lanes[i] += i as f64 * 1e-9;
				}
				let b = std::hint::black_box(0.9999999_f64);
				let c = std::hint::black_box(0.0000001_f64);
				for _iter in 0..iters {
					for l in lanes.iter_mut() {
						*l = *l * b + c;
					}
				}
				lanes.iter().sum::<f64>()
			}));
		}
		handles.into_iter().map(|h| h.join().unwrap_or(0.0)).sum()
	});
	std::hint::black_box(sum);
	let flops = threads as f64 * iters as f64 * 8.0 * 2.0;
	flops / t.elapsed().as_secs_f64() / 1e9
}

fn bench_disk(dir: &Path) -> Result<f64> {
	let bytes = 256usize << 20;
	let mut buf = vec![0u8; bytes];
	par_touch(&mut buf);
	let path = dir.join(".recipe_probe");
	let f = recipe_infer::bridge::open_rw(&path)?;
	let tw = Instant::now();
	f.write_all_at(&buf, 0)?;
	f.sync_all()?;
	drop_cache(&f, 0, bytes);
	let write_gbs = bytes as f64 / tw.elapsed().as_secs_f64() / 1e9;
	let tr = Instant::now();
	f.read_exact_at(&mut buf, 0)?;
	let read_gbs = bytes as f64 / tr.elapsed().as_secs_f64() / 1e9;
	drop(f);
	std::fs::remove_file(&path).ok();
	eprintln!("recipe probe: disk write {write_gbs:.3} GB/s, read {read_gbs:.3} GB/s");
	Ok(read_gbs)
}

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
		libc::posix_fadvise(
			f.as_raw_fd(),
			off as i64,
			len as i64,
			libc::POSIX_FADV_DONTNEED,
		);
	}
}

pub fn write_config(machines: &[Machine]) -> String {
	let mut s = String::from("machines\n");
	for m in machines {
		s.push_str(&format!("\t{}\n", m.host));
		s.push_str("\t\tETH\n");
		s.push_str(&format!(
			"\t\t\t{}\t{:.3}\n",
			eth_label(m.eth_gbs),
			m.eth_gbs
		));
		s.push_str("\t\tDISK\n");
		s.push_str(&format!("\t\t\tSIZE\t{}\n", m.disk_size));
		s.push_str(&format!("\t\t\tSATA\t{:.3}\n", m.sata_gbs));
		for i in 0..m.gpus.len() {
			let g = &m.gpus[i];
			s.push_str(&format!("\t\tGPU{i}\n"));
			s.push_str(&format!("\t\t\tVRAM\t{}\n", g.vram));
			s.push_str(&format!("\t\t\tPCIe\t{:.3}\n", g.pcie_gbs));
			s.push_str(&format!("\t\t\tFLOPs\t{:.1}\n", g.flops_gflops));
			s.push_str(&format!("\t\t\tTransfer\t{:.3}\n", g.transfer_gbs));
		}
		s.push_str("\t\tCPU\n");
		s.push_str(&format!("\t\t\tRAM\t{}\n", m.ram));
		s.push_str(&format!("\t\t\tDDR5\t{:.3}\n", m.ddr5_gbs));
		s.push_str(&format!("\t\t\tFLOPs\t{:.1}\n", m.cpu_gflops));
		s.push_str(&format!("\t\t\tTransfer\t{:.3}\n", m.cpu_transfer_gbs));
	}
	s.push_str("schema\n");
	s.push_str("\tsizes\tbytes\n");
	s.push_str("\tbandwidths\tGB/s\n");
	s.push_str("\tFLOPs\tGFLOP/s\n");
	s
}

pub fn parse_config(text: &str) -> Vec<Machine> {
	enum Sect {
		None,
		Eth,
		Gpu { i: usize },
		Cpu,
		Disk,
	}
	let mut machines: Vec<Machine> = Vec::new();
	let mut cur: Option<Machine> = None;
	let mut sect = Sect::None;
	enum Phase {
		Machines,
		Schema,
	}
	let mut phase = Phase::Machines;
	for raw in text.lines() {
		match phase {
			Phase::Schema => continue,
			Phase::Machines => match raw.trim().chars().next() {
				None => continue,
				Some(_ch) => {
					let depth = raw.chars().take_while(|c| *c == '\t').count();
					let line = raw.trim_start_matches('\t');
					match depth {
						0 => match line.trim() {
							"schema" => {
								for m in cur.take().into_iter() {
									machines.push(m);
								}
								phase = Phase::Schema;
							}
							_keep => continue,
						},
						1 => {
							for m in cur.take().into_iter() {
								machines.push(m);
							}
							cur = Some(Machine {
								host: line.trim().to_string(),
								gpus: Vec::new(),
								ram: 0,
								ddr5_gbs: 0.0,
								cpu_transfer_gbs: 0.0,
								cpu_gflops: 0.0,
								disk_size: 0,
								sata_gbs: 0.0,
								eth_gbs: 0.0,
							});
							sect = Sect::None;
						}
						2 => {
							let name = line.trim().to_ascii_lowercase();
							for m in cur.as_mut().into_iter() {
								let next: Option<Sect> = match name.as_str() {
									"eth" => Some(Sect::Eth),
									"cpu" => Some(Sect::Cpu),
									"disk" => Some(Sect::Disk),
									_other => match name.strip_prefix("gpu") {
										Some(_rest) => {
											m.gpus.push(GpuDev {
												vram: 0,
												pcie_gbs: 0.0,
												flops_gflops: 0.0,
												transfer_gbs: 0.0,
											});
											Some(Sect::Gpu {
												i: m.gpus.len() - 1,
											})
										}
										None => None,
									},
								};
								for s in next.into_iter() {
									sect = s;
								}
							}
						}
						_deep => {
							let Some(sp) = line.find(char::is_whitespace) else {
								continue;
							};
							let k = &line[..sp];
							let v = line[sp..].trim();
							let Some(m) = cur.as_mut() else { continue };
							match sect {
								Sect::Eth => m.eth_gbs = v.parse().unwrap_or(0.0),
								Sect::Gpu { i } => {
									for g in m.gpus.get_mut(i).into_iter() {
										match k {
											"VRAM" => {
												g.vram =
													v.parse().unwrap_or(0)
											}
											"PCIe" => {
												g.pcie_gbs = v
													.parse()
													.unwrap_or(0.0)
											}
											"FLOPs" => {
												g.flops_gflops = v
													.parse()
													.unwrap_or(0.0)
											}
											"Transfer" => {
												g.transfer_gbs = v
													.parse()
													.unwrap_or(0.0)
											}
											_key => continue,
										}
									}
								}
								Sect::Cpu => match k {
									"RAM" => m.ram = v.parse().unwrap_or(0),
									"DDR5" => {
										m.ddr5_gbs = v.parse().unwrap_or(0.0)
									}
									"FLOPs" => {
										m.cpu_gflops = v.parse().unwrap_or(0.0)
									}
									"Transfer" => {
										m.cpu_transfer_gbs =
											v.parse().unwrap_or(0.0)
									}
									_key => continue,
								},
								Sect::Disk => match k {
									"SIZE" => {
										m.disk_size = v.parse().unwrap_or(0)
									}
									"SATA" => {
										m.sata_gbs = v.parse().unwrap_or(0.0)
									}
									_key => continue,
								},
								Sect::None => continue,
							}
						}
					}
				}
			},
		}
	}
	for m in cur.take().into_iter() {
		machines.push(m);
	}
	machines
}

fn config_dir() -> Result<PathBuf> {
	if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
		return Ok(PathBuf::from(xdg).join("recipe"));
	}
	let home = std::env::var("HOME").map_err(|_e| anyhow!("probe: HOME not set"))?;
	Ok(PathBuf::from(home).join(".config/recipe"))
}

pub fn config_path() -> Result<PathBuf> {
	Ok(config_dir()?.join("config.ogdl"))
}

pub fn write_config_atomic(machines: &[Machine]) -> Result<()> {
	let path = config_path()?;
	for parent in path.parent().into_iter() {
		std::fs::create_dir_all(parent)?;
	}
	let tmp = path.with_extension("ogdl.tmp");
	std::fs::write(&tmp, write_config(machines))?;
	std::fs::rename(&tmp, &path)?;
	Ok(())
}

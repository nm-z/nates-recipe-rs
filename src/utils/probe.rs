use anyhow::{Result, anyhow, bail, ensure};
use gpu_core::log::{Opt, Write, probe, set_opt};
use gpu_core::memory::{GpuBuffer, USER_GB, par_copy, par_touch};
use std::cmp::Ordering;
use std::env;
use std::ffi::CString;
use std::fs;
use std::fs::File;
use std::hint;
use std::io::BufReader;
use std::mem;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process;
use std::ptr;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

const NL: char = '\u{a}';

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

pub fn gpu_child_ask() -> Option<i32> {
	let d = env::var_os("RECIPE_PROBE_GPU")?;
	set_opt(Opt {
		probe: true,
		..Opt::default()
	});
	let code = match d.to_string_lossy().parse::<i32>() {
		Ok(card) => match measure_gpu(card) {
			Ok(_dev) => 0,
			Err(e) => {
				drop(Write::err(&format!("probe child gpu{card}: {e}")));
				2
			}
		},
		Err(e) => {
			drop(Write::err(&format!("RECIPE_PROBE_GPU parse: {e}")));
			2
		}
	};
	Some(code)
}

impl Machine {
	pub fn probe() -> Result<Machine> {
		ensure!(
			env::var_os("RECIPE_PROBE_GPU").is_none(),
			"probe: RECIPE_PROBE_GPU is set but this binary's main did not call recipe::machine::gpu_child_ask() and exit before Machine::probe — refusing to fork-recurse"
		);
		let host = hostname();
		let g = ogdl::text("");
		let eth = format!("measuring.{host}.ETH");
		g.add("", &eth);
		Write::block(probe, &g.section("measuring"));
		let eth_gbs = link_speed_gbs();
		let p = format!("{eth}.{}", eth_label(eth_gbs));
		g.add(format!("{eth_gbs:.3}"), &p);
		Write::block(probe, &g.section(&p));
		let disk = format!("measuring.{host}.DISK");
		g.add("", &disk);
		Write::block(probe, &g.section(&disk));
		let dd = data_dir()?;
		let disk_size = disk_total(&dd)?;
		let p = format!("{disk}.SIZE");
		g.add(disk_size.to_string(), &p);
		Write::block(probe, &g.section(&p));
		let disk_bytes = (256usize << 20).min(host_budget(1)?);
		ensure!(
			disk_bytes > 0,
			"probe: no free ram for the disk bench buffer"
		);
		let sata_gbs = bench_disk(&dd, disk_bytes)?;
		let p = format!("{disk}.SATA");
		g.add(format!("{sata_gbs:.3}"), &p);
		Write::block(probe, &g.section(&p));
		let job = gpu_core::gate::Lease::new();
		let ngpu = gpu_core::hip::device_count().unwrap_or(0).max(0) as usize;
		let mut gpus = Vec::with_capacity(ngpu);
		for d in 0..ngpu as i32 {
			let base = format!("measuring.{host}.GPU{d}");
			g.add("", &base);
			Write::block(probe, &g.section(&base));
			match measure_gpu_child(d, &g, &base) {
				Ok(dev) => gpus.push(dev),
				Err(e) => Write::line(
					probe,
					&format!(
						"{}not drivable by this binary ({e}) — storage node",
						"    ".repeat(3)
					),
				),
			}
		}
		for _present in (0..ngpu).next().into_iter() {
			gpu_core::hip::set_device(0)?;
			gpu_core::memory::release_run_backing();
			gpu_core::memory::pool_trim();
		}
		drop(job);
		let cpu = format!("measuring.{host}.CPU");
		g.add("", &cpu);
		Write::block(probe, &g.section(&cpu));
		let ram = mem_total()?;
		let p = format!("{cpu}.RAM");
		g.add(ram.to_string(), &p);
		Write::block(probe, &g.section(&p));
		let copy_bytes = bench_bytes(2)?;
		let ddr5_gbs = bench_ddr5(copy_bytes);
		let p = format!("{cpu}.DDR5");
		g.add(format!("{ddr5_gbs:.3}"), &p);
		Write::block(probe, &g.section(&p));
		let cpu_gflops = bench_cpu_flops();
		let p = format!("{cpu}.FLOPs");
		g.add(format!("{cpu_gflops:.1}"), &p);
		Write::block(probe, &g.section(&p));
		let read_bytes = bench_bytes(1)?;
		let cpu_transfer_gbs = bench_cpu_read(read_bytes);
		let p = format!("{cpu}.Transfer");
		g.add(format!("{cpu_transfer_gbs:.3}"), &p);
		Write::block(probe, &g.section(&p));
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
	fs::read_to_string("/proc/sys/kernel/hostname")
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

fn mem_total() -> Result<u64> {
	meminfo_bytes("MemTotal:")
}

fn mem_available() -> Result<u64> {
	meminfo_bytes("MemAvailable:")
}

fn meminfo_bytes(key: &str) -> Result<u64> {
	let s = fs::read_to_string("/proc/meminfo")?;
	let rest = s
		.lines()
		.find_map(|l| l.strip_prefix(key))
		.ok_or_else(|| anyhow!("probe: {key} missing from /proc/meminfo"))?;
	let kb: u64 = rest.trim().trim_end_matches("kB").trim().parse()?;
	Ok(kb * 1024)
}

fn llc_bytes() -> Result<usize> {
	let dir = "/sys/devices/system/cpu/cpu0/cache";
	let mut best = 0usize;
	for e in fs::read_dir(dir)?.flatten() {
		let Ok(txt) = fs::read_to_string(e.path().join("size")) else {
			continue;
		};
		let t = txt.trim();
		let (num, mult) = match t.strip_suffix('K') {
			Some(n) => (n, 1usize << 10),
			None => match t.strip_suffix('M') {
				Some(n) => (n, 1usize << 20),
				None => (t, 1usize),
			},
		};
		let Ok(v) = num.parse::<usize>() else {
			continue;
		};
		best = best.max(v.saturating_mul(mult));
	}
	ensure!(best > 0, "probe: no cpu cache sizes under {dir}");
	Ok(best)
}

fn host_budget(buffers: usize) -> Result<usize> {
	Ok((mem_available()? as usize).saturating_sub(USER_GB) / buffers.max(1))
}

fn bench_bytes(buffers: usize) -> Result<usize> {
	let llc = llc_bytes()?;
	let bytes = (8 * llc).min(host_budget(buffers)?);
	ensure!(
		bytes > llc,
		"probe: {} MiB free per bench buffer, llc is {} MiB — not enough ram to measure bandwidth",
		bytes >> 20,
		llc >> 20
	);
	Ok(bytes)
}

fn disk_total(path: &Path) -> Result<u64> {
	let c = CString::new(path.as_os_str().as_encoded_bytes())?;
	let mut st: libc::statvfs = unsafe { mem::zeroed() };
	ensure!(
		unsafe { libc::statvfs(c.as_ptr(), &mut st) } == 0,
		"probe: statvfs failed for {}",
		path.display()
	);
	Ok((st.f_blocks as u64).saturating_mul(st.f_frsize as u64))
}

fn link_speed_gbs() -> f64 {
	let mut best_mbps = 0i64;
	for entries in fs::read_dir("/sys/class/net").into_iter() {
		for e in entries.flatten().filter(|e| e.file_name() != "lo") {
			let sp = e.path().join("speed");
			let m = fs::read_to_string(&sp)
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

fn measure_gpu_child(dev: i32, g: &ogdl::Graph, base: &str) -> Result<GpuDev> {
	let exe = env::current_exe()?;
	let mut child = process::Command::new(exe)
		.env("RECIPE_PROBE_GPU", dev.to_string())
		.stderr(process::Stdio::piped())
		.spawn()?;
	let pipe = child
		.stderr
		.take()
		.ok_or_else(|| anyhow!("probe child stderr"))?;
	let (tx, rx) = mpsc::channel::<String>();
	let reader = thread::spawn(move || {
		use std::io::BufRead;
		for line in BufReader::new(pipe).lines() {
			let Ok(l) = line else { break };
			match tx.send(l) {
				Ok(()) => continue,
				Err(_gone) => break,
			}
		}
	});
	let mut vram = None;
	let mut pcie = None;
	let mut flops = None;
	let mut xfer = None;
	let mut take = |l: &str| {
		let mut toks = l.split_whitespace();
		match (toks.next(), toks.next(), toks.next()) {
			(Some(k @ ("VRAM" | "PCIe" | "FLOPs" | "Transfer")), Some(v), None) => {
				match k {
					"VRAM" => vram = v.parse::<u64>().ok(),
					"PCIe" => pcie = v.parse::<f64>().ok(),
					"FLOPs" => flops = v.parse::<f64>().ok(),
					_transfer => xfer = v.parse::<f64>().ok(),
				}
				let p = format!("{base}.{k}");
				g.add(v, &p);
				Write::block(probe, &g.section(&p));
			}
			_other => match l.trim().is_empty() {
				true => {}
				false => Write::line(probe, l),
			},
		}
	};
	let deadline = Instant::now() + Duration::from_secs(120);
	let status = loop {
		match rx.recv_timeout(Duration::from_millis(200)) {
			Ok(l) => {
				take(&l);
				continue;
			}
			Err(mpsc::RecvTimeoutError::Timeout) => {}
			Err(mpsc::RecvTimeoutError::Disconnected) => {
				thread::sleep(Duration::from_millis(200));
			}
		}
		match child.try_wait()? {
			Some(status) => break status,
			None => match Instant::now().cmp(&deadline) {
				Ordering::Greater => {
					child.kill().ok();
					child.wait().ok();
					reader.join().ok();
					bail!("probe child wedged (120s) — killed");
				}
				Ordering::Less | Ordering::Equal => continue,
			},
		}
	};
	reader.join()
		.map_err(|_panic| anyhow!("probe relay thread panicked"))?;
	for l in rx.try_iter() {
		take(&l);
	}
	ensure!(status.success(), "probe child exited {status}");
	Ok(GpuDev {
		vram: vram.ok_or_else(|| anyhow!("probe child sent no VRAM"))?,
		pcie_gbs: pcie.ok_or_else(|| anyhow!("probe child sent no PCIe"))?,
		flops_gflops: flops.ok_or_else(|| anyhow!("probe child sent no FLOPs"))?,
		transfer_gbs: xfer.ok_or_else(|| anyhow!("probe child sent no Transfer"))?,
	})
}

fn measure_gpu(dev: i32) -> Result<GpuDev> {
	gpu_core::hip::set_device(dev)?;
	let total = gpu_core::hip::mem_info()?.total;
	Write::line(probe, &format!("VRAM {total}"));
	let pcie_gbs = bench_pcie_h2d()?;
	Write::line(probe, &format!("PCIe {pcie_gbs:.3}"));
	let flops_gflops = bench_gemm()?;
	Write::line(probe, &format!("FLOPs {flops_gflops:.1}"));
	let transfer_gbs = bench_transfer()?;
	Write::line(probe, &format!("Transfer {transfer_gbs:.3}"));
	Ok(GpuDev {
		vram: total as u64,
		pcie_gbs,
		flops_gflops,
		transfer_gbs,
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

fn bench_ddr5(bytes: usize) -> f64 {
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

fn bench_cpu_read(bytes: usize) -> f64 {
	let mut buf = vec![0u8; bytes];
	par_touch(&mut buf);
	let threads = thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	let per = bytes.div_ceil(threads);
	let base = buf.as_ptr() as usize;
	let mut best = f64::INFINITY;
	for _rep in 0..5 {
		let t = Instant::now();
		let sum: u64 = thread::scope(|sc| {
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
									ptr::read_volatile((base + i) as *const u8)
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
		hint::black_box(sum);
		best = best.min(t.elapsed().as_secs_f64());
	}
	bytes as f64 / best / 1e9
}

pub fn data_dir() -> anyhow::Result<PathBuf> {
	let base = match env::var_os("XDG_CACHE_HOME").filter(|v| !v.is_empty()) {
		Some(x) => PathBuf::from(x),
		None => {
			let home = env::var_os("HOME")
				.filter(|v| !v.is_empty())
				.ok_or_else(|| anyhow::anyhow!("neither XDG_CACHE_HOME nor HOME is set"))?;
			PathBuf::from(home).join(".cache")
		}
	};
	let dir = base.join("recipe");
	fs::create_dir_all(&dir).map_err(|e| anyhow::anyhow!("data_dir {}: {e}", dir.display()))?;
	Ok(dir)
}

fn bench_cpu_flops() -> f64 {
	let threads = thread::available_parallelism()
		.map(|n| n.get())
		.unwrap_or(1);
	let iters: u64 = 100_000_000;
	let t = Instant::now();
	let sum: f64 = thread::scope(|sc| {
		let mut handles = Vec::new();
		for kk in 0..threads {
			handles.push(sc.spawn(move || {
				let mut lanes = [0.5_f64 + kk as f64 * 1e-9; 8];
				for i in 0..lanes.len() {
					lanes[i] += i as f64 * 1e-9;
				}
				let b = hint::black_box(0.9999999_f64);
				let c = hint::black_box(0.0000001_f64);
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
	hint::black_box(sum);
	let flops = threads as f64 * iters as f64 * 8.0 * 2.0;
	flops / t.elapsed().as_secs_f64() / 1e9
}

fn bench_disk(dir: &Path, bytes: usize) -> Result<f64> {
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
	fs::remove_file(&path).ok();
	hint::black_box(write_gbs);
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
	let mut s = String::from("machines");
	s.push(NL);
	for m in machines {
		s.push_str(&format!("\t{}{NL}", m.host));
		s.push_str("\t\tETH");
		s.push(NL);
		s.push_str(&format!(
			"\t\t\t{}\t{:.3}{NL}",
			eth_label(m.eth_gbs),
			m.eth_gbs
		));
		s.push_str("\t\tDISK");
		s.push(NL);
		s.push_str(&format!("\t\t\tSIZE\t{}{NL}", m.disk_size));
		s.push_str(&format!("\t\t\tSATA\t{:.3}{NL}", m.sata_gbs));
		for i in 0..m.gpus.len() {
			let g = &m.gpus[i];
			s.push_str(&format!("\t\tGPU{i}{NL}"));
			s.push_str(&format!("\t\t\tVRAM\t{}{NL}", g.vram));
			s.push_str(&format!("\t\t\tPCIe\t{:.3}{NL}", g.pcie_gbs));
			s.push_str(&format!("\t\t\tFLOPs\t{:.1}{NL}", g.flops_gflops));
			s.push_str(&format!("\t\t\tTransfer\t{:.3}{NL}", g.transfer_gbs));
		}
		s.push_str("\t\tCPU");
		s.push(NL);
		s.push_str(&format!("\t\t\tRAM\t{}{NL}", m.ram));
		s.push_str(&format!("\t\t\tDDR5\t{:.3}{NL}", m.ddr5_gbs));
		s.push_str(&format!("\t\t\tFLOPs\t{:.1}{NL}", m.cpu_gflops));
		s.push_str(&format!("\t\t\tTransfer\t{:.3}{NL}", m.cpu_transfer_gbs));
	}
	s.push_str("schema");
	s.push(NL);
	s.push_str("\tsizes\tbytes");
	s.push(NL);
	s.push_str("\tbandwidths\tGB/s");
	s.push(NL);
	s.push_str("\tFLOPs\tGFLOP/s");
	s.push(NL);
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
	if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
		return Ok(PathBuf::from(xdg).join("recipe"));
	}
	let home = env::var("HOME").map_err(|_e| anyhow!("probe: HOME not set"))?;
	Ok(PathBuf::from(home).join(".config/recipe"))
}

pub fn config_path() -> Result<PathBuf> {
	Ok(config_dir()?.join("config.ogdl"))
}

pub fn pool_path() -> Result<PathBuf> {
	Ok(config_dir()?.join("pool.ogdl"))
}

pub fn write_config_atomic(machines: &[Machine]) -> Result<()> {
	let path = config_path()?;
	for parent in path.parent().into_iter() {
		fs::create_dir_all(parent)?;
	}
	let tmp = path.with_extension("ogdl.tmp");
	fs::write(&tmp, write_config(machines))?;
	fs::rename(&tmp, &path)?;
	Ok(())
}

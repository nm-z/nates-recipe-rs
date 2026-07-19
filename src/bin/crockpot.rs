use anyhow::{Result, ensure};
use gpu_core::infer_ops::gpu_gemm_bt_tiles;
use gpu_core::memory::{Dtype, GpuBuffer};
use rand::rngs::StdRng;
use rand::{Rng as _, SeedableRng as _};
use rayon::prelude::*;
use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

const DTS: &[(Dtype, &str)] = &[
	(Dtype::BF16, "bf16"),
	(Dtype::F16, "f16"),
	(Dtype::F32, "f32"),
	(Dtype::F64, "f64"),
];

const LUT_F: &str = "crockpot.tsv";
const TRAIN_F: &str = "crockpot_train.csv";
const DRAW_F: &str = "crockpot_draw.csv";
const TILES_F: &str = "crockpot_tiles.txt";
const HDR: &str = "dt_bf16,dt_f16,dt_f32,dt_f64,vram,m,n,k,tile_m,tile_n,tile_k,ms";
const COLS: &str = "dt\tVRAMGB\tm\tn\tk\ttile_m\ttile_n\ttile_k\tms";
const SLICES: usize = 8;
const BUDGET_S: f64 = 4.0;
const FLOOR_GFS: f64 = 5.0;
const WIDE: usize = 8;

struct Sample {
	di: usize,
	vgb: f64,
	m: usize,
	n: usize,
	k: usize,
	tm: usize,
	tn: usize,
	tk: usize,
}


fn prop(txt: &str, key: &str) -> Option<u64> {
	for l in txt.lines() {
		let Some(v) = l.strip_prefix(key) else {
			continue;
		};
		let Ok(n) = v.trim().parse::<u64>() else {
			continue;
		};
		return Some(n);
	}
	return None;
}

fn gpu_props() -> Result<(u64, u64)> {
	let mut lds = 0u64;
	let mut cus = 0u64;
	for e in fs::read_dir("/sys/class/kfd/kfd/topology/nodes")? {
		let p = e?.path().join("properties");
		let Ok(txt) = fs::read_to_string(&p) else {
			continue;
		};
		let simd = prop(&txt, "simd_count").unwrap_or(0);
		let per = prop(&txt, "simd_per_cu").unwrap_or(1).max(1);
		let node_cus = simd / per;
		if node_cus > cus {
			cus = node_cus;
			lds = prop(&txt, "lds_size_in_kb").unwrap_or(0);
		}
	}
	ensure!(
		cus > 0 && lds > 0,
		"no kfd gpu node under /sys/class/kfd/kfd/topology/nodes (simd_count, simd_per_cu, lds_size_in_kb)"
	);
	return Ok((lds, cus));
}

fn edge_e(bytes: u64, esize: usize) -> usize {
	let e = (bytes as f64 / (3.0 * esize as f64)).sqrt() as usize;
	return e.max(1);
}

fn fill_bytes(cells: usize, dt: Dtype, seed: u64) -> Vec<u8> {
	let esz = dt.elem_size();
	let mut v = vec![0u8; cells * esz];
	let chunk_b = (1usize << 20i32) * esz;
	v.par_chunks_mut(chunk_b)
		.enumerate()
		.for_each(|(ci, chunk)| {
			let mut rng =
				StdRng::seed_from_u64(seed ^ (ci as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
			for w in chunk.chunks_exact_mut(esz) {
				let x: f64 = rng.random();
				match dt {
					Dtype::F64 => w.copy_from_slice(&x.to_le_bytes()),
					Dtype::F32 => w.copy_from_slice(&(x as f32).to_le_bytes()),
					Dtype::F16 => w.copy_from_slice(&half::f16::from_f64(x).to_le_bytes()),
					_bf16 => w.copy_from_slice(&half::bf16::from_f64(x).to_le_bytes()),
				}
			}
		});
	return v;
}

fn feat_csv(di: usize, vgb: f64, m: usize, n: usize, k: usize, tm: usize, tn: usize, tk: usize) -> String {
	let oh = |i: usize| return i32::from(i == di);
	let lg = |t: usize| return (t.max(1) as f64).log2();
	return format!(
		"{},{},{},{},{vgb:.4},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
		oh(0),
		oh(1),
		oh(2),
		oh(3),
		lg(m),
		lg(n),
		lg(k),
		lg(tm),
		lg(tn),
		lg(tk)
	);
}

fn rand_tiles(rng: &mut StdRng, m: usize, k: usize, esz: usize, lds: usize) -> (usize, usize, usize) {
	let max_tk = (lds / (2 * esz)).min(k).max(1);
	let tk = rng.random_range(1..=max_tk);
	let cap_n = ((lds / (tk * esz)) & !1).max(2);
	let tn = rng.random_range(1..=cap_n / 2) * 2;
	let tm = rng.random_range(1..=m);
	return (tm, tn, tk);
}

fn clamp_tiles(p: [f64; 3], m: usize, k: usize, esz: usize, lds: usize) -> (usize, usize, usize) {
	let max_tk = (lds / (2 * esz)).min(k).max(1);
	let tk = (p[2].exp2().round() as i64).clamp(1, max_tk as i64) as usize;
	let cap_n = ((lds / (tk * esz)) & !1).max(2);
	let tn = ((p[1].exp2().round() as i64 / 2) * 2).clamp(2, cap_n as i64) as usize;
	let tm = (p[0].exp2().round() as i64).clamp(1, m as i64) as usize;
	return (tm, tn, tk);
}

fn lut_append(lines: &[String]) -> Result<()> {
	let fresh = !Path::new(LUT_F).exists();
	let mut f = fs::OpenOptions::new().create(true).append(true).open(LUT_F)?;
	if fresh {
		writeln!(f, "{COLS}")?;
	}
	for l in lines {
		writeln!(f, "{l}")?;
	}
	return Ok(());
}

fn di_of(name: &str) -> Option<usize> {
	for (i, (_d, n)) in DTS.iter().enumerate() {
		if *n == name {
			return Some(i);
		}
	}
	return None;
}

fn read_lut() -> (Vec<Sample>, f64) {
	let mut rows: Vec<Sample> = Vec::new();
	let mut floor = FLOOR_GFS;
	let Ok(txt) = fs::read_to_string(LUT_F) else {
		return (rows, floor);
	};
	for l in txt.lines().skip(1) {
		let f: Vec<&str> = l.split('\t').collect();
		if f.len() < 9 {
			continue;
		}
		let Some(di) = di_of(f[0].trim()) else {
			continue;
		};
		let fl = |i: usize| return f[i].trim().parse::<f64>().unwrap_or(f64::NAN);
		let us = |i: usize| return f[i].trim().parse::<usize>().unwrap_or(0);
		let (vgb, ms) = (fl(1), fl(8));
		let (m, n, k, tm, tn, tk) = (us(2), us(3), us(4), us(5), us(6), us(7));
		if m == 0 || n == 0 || k == 0 || tm == 0 || tn == 0 || tk == 0 {
			continue;
		}
		if !vgb.is_finite() || !ms.is_finite() || ms <= 0.0 {
			continue;
		}
		let gf = 2.0 * m as f64 * n as f64 * k as f64 / (ms / 1e3) / 1e9;
		floor = floor.min(gf.max(0.05));
		rows.push(Sample {
			di,
			vgb,
			m,
			n,
			k,
			tm,
			tn,
			tk,
		});
	}
	return (rows, floor);
}

fn read_draw() -> Option<(usize, f64, usize, usize, usize)> {
	let txt = fs::read_to_string(DRAW_F).ok()?;
	let line = txt.lines().nth(1)?;
	let f: Vec<&str> = line.split(',').collect();
	if f.len() < 8 {
		return None;
	}
	let di = (0..DTS.len()).find(|i| return f[*i].trim() == "1")?;
	let vgb = f[4].trim().parse::<f64>().ok()?;
	let dim = |i: usize| -> Option<usize> {
		let lg = f[i].trim().parse::<f64>().ok()?;
		let v = lg.exp2().round();
		if !v.is_finite() || v < 1.0 {
			return None;
		}
		return Some(v as usize);
	};
	return Some((di, vgb, dim(5)?, dim(6)?, dim(7)?));
}

fn read_tiles() -> Option<[f64; 3]> {
	let txt = fs::read_to_string(TILES_F).ok()?;
	let v: Vec<f64> = txt
		.split_whitespace()
		.filter_map(|t| return t.parse::<f64>().ok())
		.collect();
	if v.len() < 3 {
		return None;
	}
	return Some([v[0], v[1], v[2]]);
}

fn fresh_draw(rng: &mut StdRng, claimable: u64, gfs: f64) -> (usize, f64, usize, usize, usize) {
	let esz = WIDE;
	let floor = (24 * esz) as f64;
	let budget = ((claimable as f64 - 192.0).max(floor) * rng.random::<f64>()).max(floor) as u64;
	let cv = edge_e(budget, esz);
	let (mut m, mut n, mut k) = (
		rng.random_range(1..=cv),
		rng.random_range(1..=cv),
		rng.random_range(1..=cv),
	);
	let cap = gfs * 1e9 * BUDGET_S;
	let flop = 2.0 * m as f64 * n as f64 * k as f64;
	if flop > cap {
		let s = (cap / flop).cbrt();
		m = ((m as f64 * s) as usize).max(1);
		n = ((n as f64 * s) as usize).max(1);
		k = ((k as f64 * s) as usize).max(1);
	}
	let gb = (1u64 << 30i32) as f64;
	return (rng.random_range(0..DTS.len()), budget as f64 / gb, m, n, k);
}

fn write_draw(d: (usize, f64, usize, usize, usize)) -> Result<()> {
	let mut s = String::from(HDR);
	s.push('\n');
	s.push_str(&feat_csv(d.0, d.1, d.2, d.3, d.4, 1, 1, 1));
	s.push('\n');
	fs::write(DRAW_F, s)?;
	return Ok(());
}

fn write_train(rows: &[Sample]) -> Result<()> {
	let mut s = String::from(HDR);
	s.push('\n');
	for w in rows {
		s.push_str(&feat_csv(w.di, w.vgb, w.m, w.n, w.k, w.tm, w.tn, w.tk));
		s.push('\n');
	}
	fs::write(TRAIN_F, s)?;
	return Ok(());
}

fn carve(slab: &GpuBuffer, dt: Dtype, m: usize, n: usize, k: usize) -> (GpuBuffer, GpuBuffer, GpuBuffer) {
	let esz = dt.elem_size();
	let ua = (m * k * esz).div_ceil(8);
	let ub = (n * k * esz).div_ceil(8);
	let uc = (m * n * esz).div_ceil(8);
	let a = slab.view(0, ua).as_dtype(dt);
	let b = slab.view(ua, ub).as_dtype(dt);
	let c = slab.view(ua + ub, uc).as_dtype(dt);
	return (a, b, c);
}

fn timed_gemm(
	a: &GpuBuffer,
	b: &GpuBuffer,
	c: &GpuBuffer,
	m: usize,
	n: usize,
	k: usize,
	t: (usize, usize, usize),
) -> Result<f64> {
	let rows = m.div_ceil(SLICES).max(1);
	gpu_core::hip::device_synchronize()?;
	let t0 = Instant::now();
	let mut i0 = 0usize;
	while i0 < m {
		let r = rows.min(m - i0);
		let av = a.view(i0 * k, r * k);
		let cv = c.view(i0 * n, r * n);
		gpu_gemm_bt_tiles(&av, b, r, n, k, t.0.min(r).max(1), t.1, t.2, &cv)?;
		gpu_core::hip::device_synchronize()?;
		i0 += r;
	}
	return Ok(t0.elapsed().as_secs_f64() * 1e3);
}

fn tint(row: &str) -> String {
	let mut out = String::new();
	for (i, f) in row.split('\t').enumerate() {
		if i > 0 {
			out.push('\t');
		}
		out.push_str(&recipe::tui::paint(i, f));
	}
	return out;
}

fn main() -> Result<()> {
	let (lds_kb, _cus) = gpu_props()?;
	let lds = (lds_kb as usize) << 10i32;
	let _gate = gpu_core::gate::Lease::new();
	gpu_core::hip::set_device(0)?;
	gpu_core::hip::device_synchronize()?;
	let mut rng = StdRng::from_os_rng();
	let (mut rows, gfs) = read_lut();
	let claimable = gpu_core::memory::claimable_bytes() as u64;
	let (di, vgb, m, n, k) = match read_draw() {
		Some(d) => d,
		None => fresh_draw(&mut rng, claimable, gfs),
	};
	let (dt, name) = DTS[di];
	let predicted = match read_tiles() {
		Some(p) => clamp_tiles(p, m, k, WIDE, lds),
		None => rand_tiles(&mut rng, m, k, WIDE, lds),
	};
	let explored = rand_tiles(&mut rng, m, k, WIDE, lds);
	let bytes = (m * k + n * k + m * n) * WIDE;
	let mut got = None;
	for _t in 1..=20i32 {
		match GpuBuffer::try_alloc_bytes(bytes) {
			Some(s) => {
				got = Some(s);
				break;
			}
			None => {
				gpu_core::hip::device_synchronize()?;
				std::thread::sleep(std::time::Duration::from_millis(100));
			}
		}
	}
	let Some(slab) = got else {
		eprintln!("{bytes} B unavailable");
		write_draw(fresh_draw(
			&mut rng,
			gpu_core::memory::claimable_bytes() as u64,
			gfs,
		))?;
		return Ok(());
	};
	let esz = dt.elem_size();
	let (a, b, c) = carve(&slab, dt, m, n, k);
	let host = fill_bytes(m * k + n * k, dt, 7);
	a.write_u8(&host[..m * k * esz])?;
	b.write_u8(&host[m * k * esz..])?;
	drop(host);
	println!("{COLS}");
	let mut lines = Vec::with_capacity(2);
	for (tm, tn, tk) in [predicted, explored] {
		let ms = timed_gemm(&a, &b, &c, m, n, k, (tm, tn, tk))?;
		let row = format!("{name}\t{vgb:.2}\t{m}\t{n}\t{k}\t{tm}\t{tn}\t{tk}\t{ms:.3}");
		println!("{}", tint(&row));
		lines.push(row);
		rows.push(Sample {
			di,
			vgb,
			m,
			n,
			k,
			tm,
			tn,
			tk,
		});
	}
	lut_append(&lines)?;
	write_train(&rows)?;
	write_draw(fresh_draw(
		&mut rng,
		gpu_core::memory::claimable_bytes() as u64,
		gfs,
	))?;
	return Ok(());
}

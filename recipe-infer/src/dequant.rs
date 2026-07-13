pub fn block_layout(t: u32) -> (usize, usize) {
	match t {
		0 => (4, 1),
		1 => (2, 1),
		6 => (22, 32),
		8 => (34, 32),
		12 => (144, 256),
		14 => (210, 256),
		_other => {
			drop(gpu_core::log::Write::err(format!("gguf: unsupported ggml type {t}")));
			std::process::abort()
		}
	}
}

pub fn nbytes_for(t: u32, elems: usize) -> usize {
	let (block_bytes, block_elems) = block_layout(t);
	(elems / block_elems) * block_bytes
}

fn f16(b: u16) -> f32 {
	let s = (b >> 15) & 1;
	let e = ((b >> 10) & 0x1f) as i32;
	let m = (b & 0x3ff) as u32;
	let bits = if e == 0 {
		if m == 0 {
			(s as u32) << 31
		} else {
			let mut e2 = -14i32;
			let mut m2 = m;
			while m2 & 0x400 == 0 {
				m2 <<= 1;
				e2 -= 1;
			}
			m2 &= 0x3ff;
			((s as u32) << 31) | (((e2 + 127) as u32) << 23) | (m2 << 13)
		}
	} else if e == 31 {
		((s as u32) << 31) | (0xff << 23) | (m << 13)
	} else {
		((s as u32) << 31) | (((e - 15 + 127) as u32) << 23) | (m << 13)
	};
	f32::from_bits(bits)
}

fn get_scale_min(j: usize, q: &[u8]) -> (u8, u8) {
	if j < 4 {
		(q[j] & 63, q[j + 4] & 63)
	} else {
		(
			(q[j + 4] & 0xF) | ((q[j - 4] >> 6) << 4),
			(q[j + 4] >> 4) | ((q[j] >> 6) << 4),
		)
	}
}

fn deqblock(t: u32, raw: &[u8], out: &mut Vec<f32>) {
	match t {
		0 => out.push(f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]])),
		1 => out.push(f16(u16::from_le_bytes([raw[0], raw[1]]))),
		8 => {
			let d = f16(u16::from_le_bytes([raw[0], raw[1]]));
			for i in 0..32 {
				out.push(d * (raw[2 + i] as i8 as f32));
			}
		}
		6 => {
			let d = f16(u16::from_le_bytes([raw[0], raw[1]]));
			let qh = u32::from_le_bytes([raw[2], raw[3], raw[4], raw[5]]);
			let ql = &raw[6..22];
			for i in 0..32 {
				let x = ((qh >> i) & 1) << 4;
				let q = if i < 16 {
					(ql[i] & 0xF) as u32 | x
				} else {
					(ql[i - 16] >> 4) as u32 | x
				};
				out.push(d * (q as f32 - 16.0));
			}
		}
		12 => {
			let d = f16(u16::from_le_bytes([raw[0], raw[1]]));
			let dmin = f16(u16::from_le_bytes([raw[2], raw[3]]));
			let sc = &raw[4..16];
			let qs = &raw[16..144];
			let mut is = 0;
			let mut q = 0;
			for _ in 0..4 {
				let (s1, m1) = get_scale_min(is, sc);
				let (d1, mm1) = (d * s1 as f32, dmin * m1 as f32);
				let (s2, m2) = get_scale_min(is + 1, sc);
				let (d2, mm2) = (d * s2 as f32, dmin * m2 as f32);
				for l in 0..32 {
					out.push(d1 * ((qs[q + l] & 0xF) as f32) - mm1);
				}
				for l in 0..32 {
					out.push(d2 * ((qs[q + l] >> 4) as f32) - mm2);
				}
				q += 32;
				is += 2;
			}
		}
		14 => {
			let ql = &raw[0..128];
			let qh = &raw[128..192];
			let sc: Vec<i8> = raw[192..208].iter().map(|&x| x as i8).collect();
			let d = f16(u16::from_le_bytes([raw[208], raw[209]]));
			let mut blk = [0f32; 256];
			for nn in 0..2 {
				let (qlb, qhb, scb) = (&ql[nn * 64..], &qh[nn * 32..], &sc[nn * 8..]);
				for l in 0..32 {
					let is = l / 16;
					let q1 = ((qlb[l] & 0xF) | ((qhb[l] & 3) << 4)) as i32 - 32;
					let q2 = ((qlb[l + 32] & 0xF) | (((qhb[l] >> 2) & 3) << 4)) as i32 - 32;
					let q3 = ((qlb[l] >> 4) | (((qhb[l] >> 4) & 3) << 4)) as i32 - 32;
					let q4 = ((qlb[l + 32] >> 4) | (((qhb[l] >> 6) & 3) << 4)) as i32 - 32;
					blk[nn * 128 + l] = d * scb[is] as f32 * q1 as f32;
					blk[nn * 128 + l + 32] = d * scb[is + 2] as f32 * q2 as f32;
					blk[nn * 128 + l + 64] = d * scb[is + 4] as f32 * q3 as f32;
					blk[nn * 128 + l + 96] = d * scb[is + 6] as f32 * q4 as f32;
				}
			}
			out.extend_from_slice(&blk);
		}
		_other => {
			drop(gpu_core::log::Write::err(format!("gguf: dequant unsupported ggml type {t}")));
			std::process::abort()
		}
	}
}

pub fn dequant_f32(t: u32, bytes: &[u8], out: &mut Vec<f32>) {
	let (block_bytes, block_elems) = block_layout(t);
	let nb = bytes.len() / block_bytes;
	out.reserve(nb * block_elems);
	for b in 0..nb {
		deqblock(t, &bytes[b * block_bytes..(b + 1) * block_bytes], out);
	}
}

fn f32_to_bf16(x: f32) -> u16 {
	let bits = x.to_bits();
	if x.is_nan() {
		return ((bits >> 16) as u16) | 0x0040;
	}
	let rounded = bits.wrapping_add(0x7fff + ((bits >> 16) & 1));
	(rounded >> 16) as u16
}

pub fn dequant_bf16(t: u32, bytes: &[u8]) -> Vec<u8> {
	let mut f = Vec::new();
	dequant_f32(t, bytes, &mut f);
	let mut out = Vec::with_capacity(f.len() * 2);
	for x in f {
		out.extend_from_slice(&f32_to_bf16(x).to_le_bytes());
	}
	out
}

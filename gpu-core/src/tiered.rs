use crate::hip::{self, HipError};
use crate::log::Write;
use std::cmp;
use std::ffi::{CString, c_void};
use std::fmt;
use std::fs;
use std::fs::File;
use std::mem;
use std::num::NonZeroUsize;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process;
use std::ptr;

pub const P: usize = 2 << 20;

const RESERVE_V: usize = 1 << 30;
const RESERVE_R: usize = 1 << 30;
const RESERVE_D: usize = 1 << 30;

#[derive(Clone, Copy, Debug)]
pub enum Residence {
	Vram(u32),
	Ram(u32),
	Disk(u64),
}

#[derive(Debug)]
pub struct Full {
	pub need: usize,
	pub cap: usize,
}

impl fmt::Display for Full {
	fn fmt<'f>(&self, f: &mut fmt::Formatter<'f>) -> fmt::Result {
		write!(
			f,
			"buffer {} exceeds VRAM+RAM+disk ceiling {}",
			human(self.need),
			human(self.cap)
		)
	}
}

#[derive(Clone, Copy, Debug)]
pub struct Budgets {
	pub vram_data: usize,
	pub ram_data: usize,
	pub disk_data: usize,
	pub cap: usize,
	pub n_v: usize,
	pub n_r: usize,
}

impl Budgets {
	pub fn measure(weights_bytes: usize, grad_bytes: usize, spill: &Path) -> Self {
		let total = vram_total_free().total;
		let vram_data = total
			.saturating_sub(weights_bytes)
			.saturating_sub(grad_bytes)
			.saturating_sub(RESERVE_V);
		let ram_data = meminfo_free().saturating_sub(RESERVE_R);
		let disk_data = disk_free(spill).saturating_sub(RESERVE_D);
		Budgets {
			vram_data,
			ram_data,
			disk_data,
			cap: vram_data + ram_data + disk_data,
			n_v: vram_data / P,
			n_r: ram_data / P,
		}
	}
}

pub fn admit(
	b: usize,
	weights_bytes: usize,
	grad_bytes: usize,
	spill: &Path,
) -> Result<Budgets, Full> {
	let bud = Budgets::measure(weights_bytes, grad_bytes, spill);
	match b.cmp(&bud.cap) {
		cmp::Ordering::Greater => Err(Full {
			need: b,
			cap: bud.cap,
		}),
		cmp::Ordering::Less | cmp::Ordering::Equal => Ok(bud),
	}
}

fn vram_total_free() -> hip::MemInfo {
	let mut free = 0usize;
	let mut total = 0usize;
	crate::callspy::tick(&crate::callspy::MEM_GET_INFO);
	unsafe { hip::hipMemGetInfo(&mut free, &mut total) };
	hip::MemInfo { free, total }
}

fn meminfo_free() -> usize {
	let s = fs::read_to_string("/proc/meminfo").unwrap_or_default();
	for l in s.lines() {
		let Some(r) = l.strip_prefix("MemAvailable:") else {
			continue;
		};
		let Some(kb) = r
			.split_whitespace()
			.next()
			.and_then(|v| v.parse::<usize>().ok())
		else {
			continue;
		};
		return kb.saturating_mul(1024);
	}
	0
}

fn disk_free(spill: &Path) -> usize {
	use std::os::unix::ffi::OsStrExt;
	let dir = spill
		.parent()
		.filter(|p| !p.as_os_str().is_empty())
		.unwrap_or_else(|| Path::new("."));
	let Ok(c) = CString::new(dir.as_os_str().as_bytes()) else {
		return 0;
	};
	let mut st: libc::statvfs = unsafe { mem::zeroed() };
	let rc = unsafe { libc::statvfs(c.as_ptr(), &mut st) };
	match rc.cmp(&0) {
		cmp::Ordering::Equal => (st.f_bavail as usize).saturating_mul(st.f_frsize as usize),
		cmp::Ordering::Less | cmp::Ordering::Greater => 0,
	}
}

pub struct Tiered {
	b: usize,
	n_pg: usize,
	res: Vec<Residence>,
	budgets: Budgets,

	va: *mut c_void,
	handles: Vec<*mut c_void>,
	#[allow(dead_code)]
	slot_page: Vec<Option<usize>>,
	slots: usize,

	ram: Vec<Box<[u8]>>,

	disk: Option<File>,
	spill_path: PathBuf,
}

unsafe impl Send for Tiered {}

impl Tiered {
	pub fn alloc(
		b: usize,
		weights_bytes: usize,
		grad_bytes: usize,
		spill: &Path,
	) -> Result<Self, Full> {
		let budgets = Budgets::measure(weights_bytes, grad_bytes, spill);
		match b.cmp(&budgets.cap) {
			cmp::Ordering::Greater => Err(Full {
				need: b,
				cap: budgets.cap,
			}),
			cmp::Ordering::Less | cmp::Ordering::Equal => {
				let n_pg = b.div_ceil(P);
				let n_vram = n_pg.min(budgets.n_v);
				let n_ram = (n_pg - n_vram).min(budgets.n_r);
				Ok(Self::build(b, n_pg, n_vram, n_ram, budgets, spill))
			}
		}
	}

	pub fn alloc_capped(b: usize, n_v: usize, n_r: usize, spill: &Path) -> Self {
		let budgets = Budgets::measure(0, 0, spill);
		let n_pg = b.div_ceil(P);
		let n_vram = n_pg.min(n_v);
		let n_ram = (n_pg - n_vram).min(n_r);
		Self::build(b, n_pg, n_vram, n_ram, budgets, spill)
	}

	fn build(
		b: usize,
		n_pg: usize,
		n_vram: usize,
		n_ram: usize,
		budgets: Budgets,
		spill: &Path,
	) -> Self {
		let n_disk = n_pg - n_vram - n_ram;

		let mapping = reserve_and_map(n_vram).unwrap_or_else(|e| {
			drop(Write::err(&format!("vmm reserve/map: {e}")));
			process::abort()
		});
		let va = mapping.va;
		let handles = mapping.handles;
		crate::memory::tag_note_alloc("tiered-vram", handles.len() * P);
		let slot_page: Vec<Option<usize>> = (0..n_vram).map(Some).collect();

		let ram: Vec<Box<[u8]>> = (0..n_ram)
			.map(|_k| vec![0u8; P].into_boxed_slice())
			.collect();

		let disk = match n_disk.cmp(&0) {
			cmp::Ordering::Greater => {
				let f = crate::bridge::open_spill(spill).unwrap_or_else(|e| {
					drop(Write::err(&format!("open spill file: {e}")));
					process::abort()
				});
				f.set_len((n_disk * P) as u64).unwrap_or_else(|e| {
					drop(Write::err(&format!("size spill file: {e}")));
					process::abort()
				});
				Some(f)
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => None,
		};

		let mut res = Vec::with_capacity(n_pg);
		for s in 0..n_vram {
			res.push(Residence::Vram(s as u32));
		}
		for i in 0..n_ram {
			res.push(Residence::Ram(i as u32));
		}
		for i in 0..n_disk {
			res.push(Residence::Disk((i * P) as u64));
		}

		Tiered {
			b,
			n_pg,
			res,
			budgets,
			va,
			handles,
			slot_page,
			slots: n_vram,
			ram,
			disk,
			spill_path: spill.to_path_buf(),
		}
	}

	pub fn budgets(&self) -> Budgets {
		self.budgets
	}
	pub fn len(&self) -> usize {
		self.b
	}
	pub fn is_empty(&self) -> bool {
		self.b == 0
	}
	pub fn pages(&self) -> usize {
		self.n_pg
	}

	pub fn is_contiguous_vram(&self) -> bool {
		self.slots == self.n_pg && self.disk.is_none() && self.ram.is_empty()
	}

	pub fn device_ptr(&self) -> *mut c_void {
		if !self.is_contiguous_vram() {
			drop(Write::err(
				"device_ptr on a spilled buffer — stage pages instead",
			));
			process::abort();
		}
		self.va
	}

	pub fn fill(&mut self, src: &[u8]) {
		if !(src.len() <= self.b) {
			drop(Write::err("fill src longer than buffer"));
			process::abort();
		}
		for p in 0..self.n_pg {
			let lo = p * P;
			match lo.cmp(&src.len()) {
				cmp::Ordering::Less => {
					let hi = (lo + P).min(src.len());
					self.write_page(p, &src[lo..hi]);
				}
				cmp::Ordering::Equal | cmp::Ordering::Greater => break,
			}
		}
	}

	fn write_page(&mut self, p: usize, bytes: &[u8]) {
		match self.res[p] {
			Residence::Vram(s) => {
				let dst =
					unsafe { (self.va as *mut u8).add(s as usize * P) as *mut c_void };
				unsafe {
					crate::memory::xfer(
						dst,
						bytes.as_ptr() as *const c_void,
						bytes.len(),
						hip::HIP_MEMCPY_H2D,
						ptr::null_mut(),
					)
				}
				.unwrap_or_else(|e| {
					drop(Write::err(&format!("H2D page fill: {e}")));
					process::abort()
				});
			}
			Residence::Ram(i) => {
				self.ram[i as usize][..bytes.len()].copy_from_slice(bytes);
			}
			Residence::Disk(off) => {
				self.disk
					.as_ref()
					.unwrap_or_else(|| {
						drop(Write::err("disk tier missing"));
						process::abort()
					})
					.write_all_at(bytes, off)
					.unwrap_or_else(|e| {
						drop(Write::err(&format!("spill write: {e}")));
						process::abort()
					});
			}
		}
	}

	pub fn sync(&self) -> Result<(), HipError> {
		hip::device_synchronize()
	}

	pub fn stage_bytes(&self, off: usize, len: usize, window: *mut c_void) {
		let mut scratch = vec![0u8; P];
		let mut done = 0usize;
		while done < len {
			let gpos = off + done;
			let p = gpos / P;
			let poff = gpos % P;
			let chunk = (P - poff).min(len - done);
			let dst = unsafe { (window as *mut u8).add(done) as *mut c_void };
			match self.res[p] {
				Residence::Vram(s) => unsafe {
					let src = (self.va as *mut u8).add(s as usize * P + poff)
						as *const c_void;
					crate::memory::xfer(
						dst,
						src,
						chunk,
						hip::HIP_MEMCPY_D2D,
						ptr::null_mut(),
					)
					.unwrap_or_else(|e| {
						drop(Write::err(&format!("stage_bytes D2D: {e}")));
						process::abort()
					});
				},
				Residence::Ram(i) => unsafe {
					let src = self.ram[i as usize].as_ptr().add(poff) as *const c_void;
					crate::memory::xfer(
						dst,
						src,
						chunk,
						hip::HIP_MEMCPY_H2D,
						ptr::null_mut(),
					)
					.unwrap_or_else(|e| {
						drop(Write::err(&format!("stage_bytes H2D: {e}")));
						process::abort()
					});
				},
				Residence::Disk(diskoff) => {
					self.disk
						.as_ref()
						.unwrap_or_else(|| {
							drop(Write::err("disk tier missing"));
							process::abort()
						})
						.read_exact_at(&mut scratch[..chunk], diskoff + poff as u64)
						.unwrap_or_else(|e| {
							drop(Write::err(&format!("stage_bytes read: {e}")));
							process::abort()
						});
					unsafe {
						crate::memory::xfer(
							dst,
							scratch.as_ptr() as *const c_void,
							chunk,
							hip::HIP_MEMCPY_H2D,
							ptr::null_mut(),
						)
						.unwrap_or_else(|e| {
							drop(Write::err(&format!("stage_bytes disk H2D: {e}")));
							process::abort()
						});
					}
				}
			}
			done += chunk;
		}
	}

	pub fn stage_into(&self, first_page: usize, n_pages: usize, window: *mut c_void) {
		let mut disk_scratch = vec![0u8; P];
		for k in 0..n_pages {
			let p = first_page + k;
			match p.cmp(&self.n_pg) {
				cmp::Ordering::Equal | cmp::Ordering::Greater => break,
				cmp::Ordering::Less => {
					let bytes = P.min(self.b - p * P);
					let dst = unsafe { (window as *mut u8).add(k * P) as *mut c_void };
					match self.res[p] {
						Residence::Vram(s) => unsafe {
							let src = (self.va as *mut u8).add(s as usize * P)
								as *const c_void;
							crate::memory::xfer(
								dst,
								src,
								bytes,
								hip::HIP_MEMCPY_D2D,
								ptr::null_mut(),
							)
							.unwrap_or_else(|e| {
								drop(Write::err(&format!("stage D2D: {e}")));
								process::abort()
							});
						},
						Residence::Ram(i) => unsafe {
							let src = self.ram[i as usize].as_ptr() as *const c_void;
							crate::memory::xfer(
								dst,
								src,
								bytes,
								hip::HIP_MEMCPY_H2D,
								ptr::null_mut(),
							)
							.unwrap_or_else(|e| {
								drop(Write::err(&format!("stage H2D: {e}")));
								process::abort()
							});
						},
						Residence::Disk(off) => {
							self.disk
								.as_ref()
								.unwrap_or_else(|| {
									drop(Write::err("disk tier missing"));
									process::abort()
								})
								.read_exact_at(&mut disk_scratch[..bytes], off)
								.unwrap_or_else(|e| {
									drop(Write::err(&format!("stage read: {e}")));
									process::abort()
								});
							unsafe {
								crate::memory::xfer(
									dst,
									disk_scratch.as_ptr() as *const c_void,
									bytes,
									hip::HIP_MEMCPY_H2D,
									ptr::null_mut(),
								)
								.unwrap_or_else(|e| {
									drop(Write::err(&format!(
										"stage disk H2D: {e}"
									)));
									process::abort()
								});
							}
						}
					}
				}
			}
		}
	}
}

impl Drop for Tiered {
	fn drop(&mut self) {
		crate::memory::tag_note_free("tiered-vram", self.slots * P);
		for s in 0..self.handles.len() {
			let h = self.handles[s];
			let va = unsafe { (self.va as *mut u8).add(s * P) as *mut c_void };
			unsafe {
				crate::callspy::tick(&crate::callspy::MEM_UNMAP);
				hip::vmm_unmap(va, P);
				crate::callspy::tick(&crate::callspy::MEM_RELEASE);
				hip::vmm_release(h);
			}
		}
		for _region in ptr::NonNull::new(self.va)
			.filter(|_p| self.slots > 0)
			.into_iter()
		{
			crate::callspy::tick(&crate::callspy::MEM_ADDRESS_FREE);
			unsafe { hip::vmm_addr_free(self.va, self.slots * P) };
		}
		for _f in self.disk.as_ref().into_iter() {
			fs::remove_file(&self.spill_path).ok().unwrap_or(());
		}
	}
}

struct Mapping {
	va: *mut c_void,
	handles: Vec<*mut c_void>,
}

fn reserve_and_map(slots: usize) -> Result<Mapping, HipError> {
	let Some(_count) = NonZeroUsize::new(slots) else {
		return Ok(Mapping {
			va: ptr::null_mut(),
			handles: Vec::new(),
		});
	};
	let mut va: *mut c_void = ptr::null_mut();
	crate::callspy::tick(&crate::callspy::MEM_ADDRESS_RESERVE);
	hip::check(unsafe { hip::vmm_reserve(&mut va, slots * P) })?;
	let mut handles = Vec::with_capacity(slots);
	for s in 0..slots {
		let mut h: *mut c_void = ptr::null_mut();
		crate::callspy::tick(&crate::callspy::MEM_CREATE);
		hip::check(unsafe { hip::vmm_create(&mut h, P) })?;
		let slot_va = unsafe { (va as *mut u8).add(s * P) as *mut c_void };
		crate::callspy::tick(&crate::callspy::MEM_MAP);
		crate::callspy::tick(&crate::callspy::MEM_SET_ACCESS);
		hip::check(unsafe { hip::vmm_map_at(slot_va, P, h) })?;
		handles.push(h);
	}
	Ok(Mapping { va, handles })
}

pub fn human(b: usize) -> String {
	const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
	let mut v = b as f64;
	let mut i = 0;
	while v >= 1024.0 && i < 4 {
		v /= 1024.0;
		i += 1;
	}
	format!("{v:.2} {}", U[i])
}

extern crate alloc;
use crate::HipError;
use crate::bridge::open_spill;
use crate::callspy;
use crate::hip;
use ogdl::log::Write;
use ogdl::log::data;
use crate::memory;
use crate::memory::tag_note_alloc;
use core::cmp;
use core::ffi::c_void;
use core::fmt;
use core::iter;
use core::num::NonZeroUsize;
use core::ptr;
use std::fs;
use std::fs::File;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

pub const P: usize = 2 << 20;

const RESERVE_V: usize = 1 << 30;
const RESERVE_R: usize = 1 << 30;
const RESERVE_D: usize = 1 << 30;

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
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
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return write!(
			f,
			"buffer {} exceeds VRAM+RAM+disk ceiling {}",
			human(self.need),
			human(self.cap)
		);
	}
}

pub fn data_dir() -> std::io::Result<PathBuf> {
	let base = match std::env::var_os("XDG_CACHE_HOME").filter(|v| return !v.is_empty()) {
		Some(x) => PathBuf::from(x),
		None => {
			let Some(home) = std::env::var_os("HOME").filter(|v| return !v.is_empty()) else {
				return Err(std::io::Error::new(
					std::io::ErrorKind::NotFound,
					"data_dir: neither XDG_CACHE_HOME nor HOME is set",
				));
			};
			PathBuf::from(home).join(".cache")
		}
	};
	let dir = base.join("recipe");
	fs::create_dir_all(&dir)?;
	return Ok(dir);
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
	#[must_use]
	#[inline]
	pub fn measure(weights_bytes: usize, grad_bytes: usize, spill: &Path) -> Self {
		let mut free = 0usize;
		let mut total = 0usize;
		callspy::tick(&callspy::MEM_GET_INFO);
		// SAFETY: free and total are live local usize slots that hipMemGetInfo writes.
		unsafe {
			hip::hipMemGetInfo(&raw mut free, &raw mut total);
		}
		let vram_data = total
			.saturating_sub(weights_bytes)
			.saturating_sub(grad_bytes)
			.saturating_sub(RESERVE_V);

		let meminfo = fs::read_to_string("/proc/meminfo").unwrap_or_default();
		let ram_avail = meminfo
			.lines()
			.find_map(|l| {
				let r = l.strip_prefix("MemAvailable:")?;
				let kb = r.split_whitespace().next()?.parse::<usize>().ok()?;
				return Some(kb.saturating_mul(1024));
			})
			.unwrap_or(0);
		let host_mirror = weights_bytes.saturating_add(grad_bytes);
		let ram_data = ram_avail
			.saturating_sub(RESERVE_R)
			.saturating_sub(host_mirror);
		Write::line(data, format!(
			"RAM headroom: MemAvailable {:.2} GB, reserve {:.2} GB, host-mirror {:.2} GB, RAM budget {:.2} GB",
			ram_avail as f64 / 1e9,
			RESERVE_R as f64 / 1e9,
			host_mirror as f64 / 1e9,
			ram_data as f64 / 1e9
		));

		let dir = spill
			.parent()
			.filter(|p| return !p.as_os_str().is_empty())
			.unwrap_or_else(|| return Path::new("."));
		let disk_avail = crate::sys::disk_free_bytes(dir);
		let disk_data = disk_avail.saturating_sub(RESERVE_D);
		return Self {
			vram_data,
			ram_data,
			disk_data,
			cap: vram_data + ram_data + disk_data,
			n_v: vram_data.div_euclid(P),
			n_r: ram_data.div_euclid(P),
		};
	}
}

#[inline]
pub fn admit(
	b: usize,
	weights_bytes: usize,
	grad_bytes: usize,
	spill: &Path,
) -> Result<Budgets, Full> {
	let bud = Budgets::measure(weights_bytes, grad_bytes, spill);
	match b.cmp(&bud.cap) {
		cmp::Ordering::Greater => {
			return Err(Full {
				need: b,
				cap: bud.cap,
			});
		}
		cmp::Ordering::Less | cmp::Ordering::Equal => return Ok(bud),
	}
}

pub struct Tiered {
	b: usize,
	n_pg: usize,
	res: Vec<Residence>,
	budgets: Budgets,

	va: *mut c_void,
	handles: Vec<*mut c_void>,
	#[expect(
		dead_code,
		reason = "reserved slot-to-page reverse map for future eviction"
	)]
	slot_page: Vec<Option<usize>>,
	slots: usize,

	ram: Vec<Box<[u8]>>,

	disk: Option<File>,
	spill_path: PathBuf,
}

// SAFETY: the raw VA and handle pointers are owned solely by this Tiered and never aliased, so sending it across threads is sound.
unsafe impl Send for Tiered {}

impl Tiered {
	#[inline]
	pub fn alloc(
		b: usize,
		weights_bytes: usize,
		grad_bytes: usize,
		spill: &Path,
	) -> Result<Self, Full> {
		let budgets = Budgets::measure(weights_bytes, grad_bytes, spill);
		match b.cmp(&budgets.cap) {
			cmp::Ordering::Greater => {
				return Err(Full {
					need: b,
					cap: budgets.cap,
				});
			}
			cmp::Ordering::Less | cmp::Ordering::Equal => {
				let n_pg = b.div_ceil(P);
				let gpu_pages = n_pg.min(budgets.n_v);
				let ram_pages = (n_pg - gpu_pages).min(budgets.n_r);
				return Ok(Self::build(b, n_pg, gpu_pages, ram_pages, budgets, spill));
			}
		}
	}

	#[inline]
	#[must_use]
	pub fn alloc_capped(b: usize, n_v: usize, n_r: usize, spill: &Path) -> Self {
		let budgets = Budgets::measure(0, 0, spill);
		let n_pg = b.div_ceil(P);
		let gpu_pages = n_pg.min(n_v);
		let ram_pages = (n_pg - gpu_pages).min(n_r);
		return Self::build(b, n_pg, gpu_pages, ram_pages, budgets, spill);
	}

	fn build(
		b: usize,
		n_pg: usize,
		gpu_pages: usize,
		ram_pages: usize,
		budgets: Budgets,
		spill: &Path,
	) -> Self {
		let n_disk = n_pg - gpu_pages - ram_pages;

		let (va, handles): (*mut c_void, Vec<*mut c_void>) = NonZeroUsize::new(gpu_pages)
			.and_then(|_count| return memory::vmm_map_span(gpu_pages * P))
			.unwrap_or((ptr::null_mut(), Vec::new()));
		tag_note_alloc("tiered-vram", handles.len() * P);
		let slot_page: Vec<Option<usize>> = (0..gpu_pages).map(Some).collect();

		let ram: Vec<Box<[u8]>> = iter::repeat_with(|| return vec![0u8; P].into_boxed_slice())
			.take(ram_pages)
			.collect();

		let disk = match n_disk.cmp(&0) {
			cmp::Ordering::Greater => match open_spill(spill) {
				Ok(f) => {
					let disk_bytes = u64::try_from(n_disk * P).unwrap_or_else(|e| {
						Write::error(format!(
							"spill size {} overflows u64: {e}",
							n_disk * P
						));
						0
					});
					f.set_len(disk_bytes).unwrap_or_else(|e| {
						Write::error(format!("size spill file: {e}"));
					});
					Some(f)
				}
				Err(e) => {
					Write::error(format!("open spill file: {e}"));
					None
				}
			},
			cmp::Ordering::Less | cmp::Ordering::Equal => None,
		};

		let mut res = Vec::with_capacity(n_pg);
		for s in 0..gpu_pages {
			res.push(Residence::Vram(u32::try_from(s).unwrap_or_else(|e| {
				Write::error(format!("vram slot {s} overflows u32: {e}"));
				0
			})));
		}
		for i in 0..ram_pages {
			res.push(Residence::Ram(u32::try_from(i).unwrap_or_else(|e| {
				Write::error(format!("ram slot {i} overflows u32: {e}"));
				0
			})));
		}
		for i in 0..n_disk {
			res.push(Residence::Disk(u64::try_from(i * P).unwrap_or_else(|e| {
				Write::error(format!(
					"disk offset {} overflows u64: {e}",
					i * P
				));
				0
			})));
		}

		return Self {
			b,
			n_pg,
			res,
			budgets,
			va,
			handles,
			slot_page,
			slots: gpu_pages,
			ram,
			disk,
			spill_path: spill.to_path_buf(),
		};
	}

	#[inline]
	#[must_use]
	pub const fn budgets(&self) -> Budgets {
		return self.budgets;
	}
	#[inline]
	#[must_use]
	pub const fn len(&self) -> usize {
		return self.b;
	}
	#[must_use]
	#[inline]
	pub const fn is_empty(&self) -> bool {
		return self.b == 0;
	}
	#[must_use]
	#[inline]
	pub const fn pages(&self) -> usize {
		return self.n_pg;
	}

	#[must_use]
	#[inline]
	pub const fn is_contiguous_vram(&self) -> bool {
		return self.slots == self.n_pg && self.disk.is_none() && self.ram.is_empty();
	}

	#[must_use]
	#[inline]
	pub fn device_ptr(&self) -> *mut c_void {
		if !self.is_contiguous_vram() {
			Write::error(
				"device_ptr on a spilled buffer \u{2014} stage pages instead",
			);
			return ptr::null_mut();
		}
		return self.va;
	}

	#[inline]
	pub fn fill(&mut self, src: &[u8]) {
		if src.len() > self.b {
			Write::error("fill src longer than buffer");
			return;
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
				let off = match usize::try_from(s) {
					Ok(v) => v * P,
					Err(_) => {
						Write::error(format!("vram slot index {s} overflows usize"));
						return;
					}
				};
				let dst =
					// SAFETY: off < slots*P, so va+off addresses a mapped contiguous VRAM page.
					unsafe { self.va.cast::<u8>().add(off).cast::<c_void>() };
				// SAFETY: dst is a mapped page pointer; src and len describe a valid host slice for the H2D copy.
				unsafe {
					memory::xfer(
						dst,
						bytes.as_ptr().cast::<c_void>(),
						bytes.len(),
						hip::HIP_MEMCPY_H2D,
						ptr::null_mut(),
					)
				}
				.unwrap_or_else(|e| {
					Write::error(format!("H2D page fill: {e}"));
				});
			}
			Residence::Ram(i) => {
				let ri = match usize::try_from(i) {
					Ok(v) => v,
					Err(_) => {
						Write::error(format!("ram page index {i} overflows usize"));
						return;
					}
				};
				self.ram[ri][..bytes.len()].copy_from_slice(bytes);
			}
			Residence::Disk(off) => {
				let Some(f) = self.disk.as_ref() else {
					Write::error("disk tier missing");
					return;
				};
				f.write_all_at(bytes, off).unwrap_or_else(|e| {
					Write::error(format!("spill write: {e}"));
				});
			}
		}
	}

	#[inline]
	pub fn sync(&self) -> Result<(), HipError> {
		return hip::device_synchronize();
	}

	#[inline]
	pub fn stage_bytes(&self, off: usize, len: usize, window: *mut c_void) {
		use crate::memory::xfer;
		let mut scratch = vec![0u8; P];
		let mut done = 0usize;
		while done < len {
			let gpos = off + done;
			let p = gpos.div_euclid(P);
			let poff = gpos.rem_euclid(P);
			let chunk = (P - poff).min(len - done);
			// SAFETY: done < len keeps the write offset within the staging window.
			let dst = unsafe { window.cast::<u8>().add(done).cast::<c_void>() };
			match self.res[p] {
				Residence::Vram(s) => {
					// SAFETY: s indexes a mapped VRAM slot and poff < P, so the D2D source stays in-bounds.
					let src = unsafe {
						self.va
							.cast::<u8>()
							.add(usize::try_from(s).unwrap_or(0) * P + poff)
							.cast::<c_void>()
							.cast_const()
					};
					// SAFETY: src and dst are valid for chunk bytes and both live on the device.
					unsafe {
						xfer(dst, src, chunk, hip::HIP_MEMCPY_D2D, ptr::null_mut())
					}
					.unwrap_or_else(|e| {
						Write::error(format!("stage_bytes D2D: {e}"));
					});
				}
				Residence::Ram(i) => {
					// SAFETY: poff < P keeps the read within the resident RAM page.
					let src = unsafe {
						self.ram[usize::try_from(i).unwrap_or(0)]
							.as_ptr()
							.add(poff)
							.cast::<c_void>()
					};
					// SAFETY: src and dst are valid for chunk bytes; H2D from host RAM into the window.
					unsafe {
						xfer(dst, src, chunk, hip::HIP_MEMCPY_H2D, ptr::null_mut())
					}
					.unwrap_or_else(|e| {
						Write::error(format!("stage_bytes H2D: {e}"));
					});
				}
				Residence::Disk(diskoff) => {
					let Some(f) = self.disk.as_ref() else {
						Write::error("disk tier missing");
						return;
					};
					f.read_exact_at(
						&mut scratch[..chunk],
						diskoff + u64::try_from(poff).unwrap_or(0),
					)
					.unwrap_or_else(|e| {
						Write::error(format!("stage_bytes read: {e}"));
					});
					// SAFETY: scratch holds chunk bytes just read from disk and dst is valid for chunk bytes.
					unsafe {
						xfer(
							dst,
							scratch.as_ptr().cast::<c_void>(),
							chunk,
							hip::HIP_MEMCPY_H2D,
							ptr::null_mut(),
						)
						.unwrap_or_else(|e| {
							Write::error(format!("stage_bytes disk H2D: {e}"));
						});
					}
				}
			}
			done += chunk;
		}
	}

	#[inline]
	pub fn stage_into(&self, first_page: usize, n_pages: usize, window: *mut c_void) {
		let mut disk_scratch = vec![0u8; P];
		for k in 0..n_pages {
			let p = first_page + k;
			match p.cmp(&self.n_pg) {
				cmp::Ordering::Equal | cmp::Ordering::Greater => break,
				cmp::Ordering::Less => {
					let bytes = P.min(self.b - p * P);
					// SAFETY: k*P stays within the staged window allocation for this batch
					let dst = unsafe { window.cast::<u8>().add(k * P).cast::<c_void>() };
					match self.res[p] {
						Residence::Vram(s) => {
							let slot = match usize::try_from(s) {
								Ok(v) => v,
								Err(_) => {
									Write::error(format!(
										"stage_into vram slot {s} overflows usize"
									));
									return;
								}
							};
							// SAFETY: slot indexes a mapped VRAM page within the reserved VA span
							let src = unsafe {
								self.va
									.cast::<u8>()
									.add(slot * P)
									.cast::<c_void>()
									.cast_const()
							};
							// SAFETY: src and dst are valid device pointers for the copy length
							unsafe {
								memory::xfer(
									dst,
									src,
									bytes,
									hip::HIP_MEMCPY_D2D,
									ptr::null_mut(),
								)
							}
							.unwrap_or_else(|e| {
								Write::error(format!("stage D2D: {e}"));
							});
						}
						Residence::Ram(i) => {
							let ri = match usize::try_from(i) {
								Ok(v) => v,
								Err(_) => {
									Write::error(format!(
										"stage_into ram index {i} overflows usize"
									));
									return;
								}
							};
							// SAFETY: src points into a live RAM page; dst is a valid device pointer
							unsafe {
								let src = self.ram[ri].as_ptr().cast::<c_void>();
								memory::xfer(
									dst,
									src,
									bytes,
									hip::HIP_MEMCPY_H2D,
									ptr::null_mut(),
								)
								.unwrap_or_else(|e| {
									Write::error(format!("stage H2D: {e}"));
								});
							}
						}
						Residence::Disk(off) => {
							let Some(f) = self.disk.as_ref() else {
								Write::error("disk tier missing");
								return;
							};
							f.read_exact_at(&mut disk_scratch[..bytes], off)
								.unwrap_or_else(|e| {
									Write::error(format!("stage read: {e}"));
								});
							// SAFETY: disk_scratch holds the read bytes; dst is a valid device pointer
							unsafe {
								memory::xfer(
									dst,
									disk_scratch.as_ptr().cast::<c_void>(),
									bytes,
									hip::HIP_MEMCPY_H2D,
									ptr::null_mut(),
								)
								.unwrap_or_else(|e| {
									Write::error(format!(
										"stage disk H2D: {e}"
									));
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
	#[inline]
	fn drop(&mut self) {
		memory::tag_note_free("tiered-vram", self.slots * P);
		memory::vmm_unmap_span(self.va, &self.handles);
		if self.disk.is_some() {
			fs::remove_file(&self.spill_path).ok().unwrap_or(());
		}
	}
}

#[must_use]
#[inline]
pub fn human(b: usize) -> String {
	const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
	let mut i = 0usize;
	let mut div = 1usize;
	while i < 4 && b.div_euclid(div.saturating_mul(1_024)) > 0 {
		div = div.saturating_mul(1_024);
		i += 1;
	}
	let whole = b.div_euclid(div);
	let frac = b.rem_euclid(div).saturating_mul(100).div_euclid(div);
	return format!("{whole}.{frac:02} {}", U[i]);
}

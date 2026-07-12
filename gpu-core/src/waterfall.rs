use crate::log::{Write, gpu};
use crate::memory::{GpuBuffer, tag_scope};
use std::collections::HashMap;
use std::io::{Error, Result};

pub enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk,
}

#[derive(Clone, Copy)]
enum Fill {
	Open,
	Full,
}

enum Tier {
	Use,
	Skip,
}

pub struct Waterfall {
	slab: Option<GpuBuffer>,
	homes: HashMap<String, Home>,
	vram_full: Fill,
	ram_full: Fill,
	ram_floor: usize,
	vram_bytes: usize,
	ram_bytes: usize,
	disk_bytes: usize,
}

fn mem_available() -> usize {
	std::fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(usize::MAX, |kb| kb.saturating_mul(1024))
}

impl Default for Waterfall {
	fn default() -> Self {
		Self::new()
	}
}

impl Waterfall {
	pub fn new() -> Self {
		Waterfall {
			slab: None,
			homes: HashMap::new(),
			vram_full: Fill::Full,
			ram_full: Fill::Open,
			ram_floor: mem_available() / 10,
			vram_bytes: 0,
			ram_bytes: 0,
			disk_bytes: 0,
		}
	}

	pub fn from_arena(slab: GpuBuffer) -> Self {
		let mut w = Self::new();
		w.slab = Some(slab);
		w.vram_full = Fill::Open;
		w
	}

	pub fn place(
		&mut self,
		name: &str,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<&Home> {
		let home = self.settle(len, fill)?;
		match &home {
			Home::Vram(..) => self.vram_bytes += len,
			Home::Ram(..) => self.ram_bytes += len,
			Home::Disk => self.disk_bytes += len,
		}
		Ok(self.homes.entry(name.to_string()).or_insert(home))
	}

	fn settle(&mut self, len: usize, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<Home> {
		let vram = match self.vram_full {
			Fill::Full => Tier::Skip,
			Fill::Open => match crate::memory::arena_remaining().cmp(&len) {
				std::cmp::Ordering::Less => {
					self.vram_full = Fill::Full;
					Tier::Skip
				}
				std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => Tier::Use,
			},
		};
		match vram {
			Tier::Use => {
				let _t = tag_scope("waterfall");
				let view = GpuBuffer::alloc_bytes(len)
					.map_err(|e| Error::other(format!("carve: {e}")))?;
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				view.write_u8(&host)
					.map_err(|e| Error::other(format!("waterfall H2D: {e}")))?;
				Ok(Home::Vram(view))
			}
			Tier::Skip => self.settle_host(len, fill),
		}
	}

	fn settle_host(
		&mut self,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<Home> {
		let ram = match self.ram_full {
			Fill::Full => Tier::Skip,
			Fill::Open => match mem_available().saturating_sub(len).cmp(&self.ram_floor) {
				std::cmp::Ordering::Greater => Tier::Use,
				std::cmp::Ordering::Less | std::cmp::Ordering::Equal => {
					self.ram_full = Fill::Full;
					Tier::Skip
				}
			},
		};
		match ram {
			Tier::Use => {
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				Ok(Home::Ram(host))
			}
			Tier::Skip => Ok(Home::Disk),
		}
	}

	pub fn home(&self, name: &str) -> Option<&Home> {
		self.homes.get(name)
	}

	pub fn report(&self) {
		let gb = |b: usize| b as f64 / (1u64 << 30) as f64;
		Write::line(gpu, &format!(
			"waterfall: VRAM {:.2} GB → RAM {:.2} GB → DISK {:.2} GB ({} blobs)",
			gb(self.vram_bytes),
			gb(self.ram_bytes),
			gb(self.disk_bytes),
			self.homes.len()
		));
	}
}

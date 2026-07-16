use crate::log::{Write, gpu};
use crate::memory::{GpuBuffer, arena_remaining, tag_scope};
use core::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io::{Error, Result};

#[non_exhaustive]
pub enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk,
}

/// Whether a memory tier still has room to accept a blob.
#[derive(Clone, Copy)]
enum Fill {
	/// Tier can still accept blobs.
	Open,
	/// Tier is exhausted; skip it.
	Full,
}

/// Decision for one tier: place the blob here or fall through.
enum Tier {
	/// Place the blob in this tier.
	Use,
	/// Fall through to the next tier.
	Skip,
}

pub struct Waterfall {
	/// Device arena backing VRAM carves, if this waterfall owns one.
	slab: Option<GpuBuffer>,
	/// Named blobs and the tier each landed in.
	homes: HashMap<String, Home>,
	/// Whether VRAM can still accept blobs.
	vram_full: Fill,
	/// Whether RAM can still accept blobs.
	ram_full: Fill,
	/// Minimum RAM bytes to leave free for the user.
	ram_floor: usize,
	/// Running total of bytes placed in VRAM.
	vram_bytes: usize,
	/// Running total of bytes placed in RAM.
	ram_bytes: usize,
	/// Running total of bytes spilled to disk.
	disk_bytes: usize,
}

/// Reads `MemAvailable` from `/proc/meminfo` in bytes, or `usize::MAX` if unreadable.
fn mem_available() -> usize {
	return fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			return s
				.lines()
				.find(|l| return l.starts_with("MemAvailable:"))
				.and_then(|l| return l.split_whitespace().nth(1))
				.and_then(|v| return v.parse::<usize>().ok());
		})
		.map_or(usize::MAX, |kb| return kb.saturating_mul(1024));
}

impl Default for Waterfall {
	#[inline]
	fn default() -> Self {
		return Self::new();
	}
}

impl Waterfall {
	#[inline]
	#[must_use]
	pub fn new() -> Self {
		return Self {
			slab: None,
			homes: HashMap::new(),
			vram_full: Fill::Full,
			ram_full: Fill::Open,
			ram_floor: mem_available().div_euclid(10),
			vram_bytes: 0,
			ram_bytes: 0,
			disk_bytes: 0,
		};
	}

	#[inline]
	#[must_use]
	pub fn from_arena(slab: GpuBuffer) -> Self {
		let mut w = Self::new();
		w.slab = Some(slab);
		w.vram_full = Fill::Open;
		return w;
	}

	/// Hands back the claim-mapped slab (if this store owns one) so the caller
	/// can pass it to `release_device_arena` and end the run's claim. After this
	/// the store must not place anything further.
	#[inline]
	pub fn take_slab(&mut self) -> Option<GpuBuffer> {
		return self.slab.take();
	}

	/// # Errors
	/// Returns an error if the fill closure fails or a VRAM carve/H2D transfer fails.
	#[inline]
	pub fn place(
		&mut self,
		name: &str,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<&Home> {
		let home = self.settle(len, fill)?;
		match home {
			Home::Vram(..) => self.vram_bytes += len,
			Home::Ram(..) => self.ram_bytes += len,
			Home::Disk => self.disk_bytes += len,
		}
		return Ok(self.homes.entry(name.to_owned()).or_insert(home));
	}

	/// Places `len` bytes in VRAM if the arena has room, else falls through to host tiers.
	fn settle(&mut self, len: usize, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<Home> {
		let vram = match self.vram_full {
			Fill::Full => Tier::Skip,
			Fill::Open => match arena_remaining().cmp(&len) {
				Ordering::Less => {
					self.vram_full = Fill::Full;
					Tier::Skip
				}
				Ordering::Equal | Ordering::Greater => Tier::Use,
			},
		};
		match vram {
			Tier::Use => {
				let _t = tag_scope("waterfall");
				let view = GpuBuffer::alloc_bytes(len)
					.map_err(|e| return Error::other(format!("carve: {e}")))?;
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				view.write_u8(&host)
					.map_err(|e| return Error::other(format!("waterfall H2D: {e}")))?;
				return Ok(Home::Vram(view));
			}
			Tier::Skip => return self.settle_host(len, fill),
		}
	}

	/// Places `len` bytes in RAM if it stays above the floor, else marks the blob disk-resident.
	fn settle_host(
		&mut self,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<Home> {
		let ram = match self.ram_full {
			Fill::Full => Tier::Skip,
			Fill::Open => match mem_available().saturating_sub(len).cmp(&self.ram_floor) {
				Ordering::Greater => Tier::Use,
				Ordering::Less | Ordering::Equal => {
					self.ram_full = Fill::Full;
					Tier::Skip
				}
			},
		};
		match ram {
			Tier::Use => {
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				return Ok(Home::Ram(host));
			}
			Tier::Skip => return Ok(Home::Disk),
		}
	}

	#[inline]
	#[must_use]
	pub fn home(&self, name: &str) -> Option<&Home> {
		return self.homes.get(name);
	}

	#[inline]
	pub fn report(&self) {
		let gib: usize = 0x4000_0000;
		let gb = |b: usize| -> String {
			let whole = b.div_euclid(gib);
			let frac = b.rem_euclid(gib).saturating_mul(100).div_euclid(gib);
			return format!("{whole}.{frac:02}");
		};
		Write::line(
			gpu,
			format!(
				"waterfall: VRAM {} GB → RAM {} GB → DISK {} GB ({} blobs)",
				gb(self.vram_bytes),
				gb(self.ram_bytes),
				gb(self.disk_bytes),
				self.homes.len()
			),
		);
	}
}

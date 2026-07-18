use crate::log::{Write, gpu};
use crate::memory::{GpuBuffer, USER_GB, arena_remaining, probe_ram_ceiling, tag_scope};
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
	/// Probe-verified RAM budget in bytes, computed once on first host placement.
	ram_ceiling: Option<usize>,
	/// Running total of bytes placed in VRAM.
	vram_bytes: usize,
	/// Running total of bytes placed in RAM.
	ram_bytes: usize,
	/// Running total of bytes spilled to disk.
	disk_bytes: usize,
	/// Reusable host bounce buffer for VRAM placements, grown to the largest
	/// blob seen so load-time settle stops mallocing a fresh vec per blob.
	stage: Vec<u8>,
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

/// Formats `b` bytes as a two-decimal GB magnitude (no unit suffix).
fn gb(b: usize) -> String {
	let gib: usize = 0x4000_0000;
	let whole = b.div_euclid(gib);
	let frac = b.rem_euclid(gib).saturating_mul(100).div_euclid(gib);
	return format!("{whole}.{frac:02}");
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
			ram_ceiling: None,
			vram_bytes: 0,
			ram_bytes: 0,
			disk_bytes: 0,
			stage: Vec::new(),
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
				if self.stage.len() < len {
					self.stage.resize(len, 0);
				}
				let host = &mut self.stage[..len];
				host.fill(0);
				fill(host)?;
				view.write_u8(host)
					.map_err(|e| return Error::other(format!("waterfall H2D: {e}")))?;
				return Ok(Home::Vram(view));
			}
			Tier::Skip => return self.settle_host(len, fill),
		}
	}

	/// Probe-verified RAM budget for this waterfall, computed once and cached. The
	/// guess is `MemAvailable - USER_GB` exactly as the VRAM claim bakes the 1 GB
	/// reserve into `vram_free_base - USER_GB`; a child process then commits and
	/// touches that budget to prove the kernel will actually back it.
	fn ram_ceiling(&mut self) -> usize {
		if let Some(c) = self.ram_ceiling {
			return c;
		}
		let guess = mem_available().saturating_sub(USER_GB);
		let verified = probe_ram_ceiling(guess).unwrap_or(0);
		self.ram_ceiling = Some(verified);
		return verified;
	}

	/// Places `len` bytes in RAM if the running total stays within the probe-verified
	/// ceiling, else marks the blob disk-resident.
	fn settle_host(
		&mut self,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<Home> {
		let ceiling = self.ram_ceiling();
		let ram = match self.ram_full {
			Fill::Full => Tier::Skip,
			Fill::Open => match self.ram_bytes.saturating_add(len).cmp(&ceiling) {
				Ordering::Less | Ordering::Equal => Tier::Use,
				Ordering::Greater => {
					self.ram_full = Fill::Full;
					Write::line(
						gpu,
						format!(
							"waterfall RAM full: blob {} GB + RAM {} GB exceeds probe ceiling {} GB, spilling to DISK",
							gb(len),
							gb(self.ram_bytes),
							gb(ceiling)
						),
					);
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

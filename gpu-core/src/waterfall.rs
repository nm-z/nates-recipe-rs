//! VRAM→RAM→DISK waterfall for immutable byte blobs (model weights).
//!
//! Strict fill order — the water never pools in two layers at once:
//!   1. VRAM: `claim()` takes ONE allocation of everything the driver reports
//!      free at init (memset-committed, so the water level is touched pages,
//!      not reservations) and registers it as the process device arena; every
//!      later GpuBuffer — activations, norms, staging, library workspace,
//!      weight blobs — carves from the claim until it is exhausted. The pool
//!      is never touched again; exit frees the one claim.
//!   2. RAM until the next blob would push past 90% of MemAvailable measured
//!      at fill start (the same guard law pantry applies to dataset parsing).
//!   3. DISK: once both tiers have refused, every later blob stays on disk
//!      and its bytes are never read at fill time.
//!
//! Location is this module's output, never the caller's choice. The GPU side
//! lands in the ledger under tag "waterfall"; `report()` prints all three
//! water levels.

use crate::memory::{GpuBuffer, tag_scope};
use std::collections::HashMap;
use std::io::{Error, Result};

pub enum Home {
	Vram(GpuBuffer),
	Ram(Vec<u8>),
	Disk,
}

pub struct Waterfall {
	slab: Option<GpuBuffer>, // ONE pool allocation; blobs are bump-placed views
	homes: HashMap<String, Home>,
	vram_full: bool,
	ram_full: bool,
	ram_floor: usize, // MemAvailable value that means "RAM is full"
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
	/// An empty store: no slab, every lookup misses to DISK. Placeholder only —
	/// the real store comes from `claim()` at init.
	pub fn new() -> Self {
		Waterfall {
			slab: None,
			homes: HashMap::new(),
			vram_full: true,
			ram_full: false,
			ram_floor: mem_available() / 10,
			vram_bytes: 0,
			ram_bytes: 0,
			disk_bytes: 0,
		}
	}

	/// Wrap an already-claimed process device arena (the ONE slab handed back by
	/// `memory::claim_device_arena_bytes`, which registered it as the arena and
	/// committed its pages) as the store's VRAM tier. The one-claim lifecycle
	/// itself — the sizing/probe/claim/memset/register — lives in memory.rs now;
	/// the store only bump-places blobs into the registered arena and holds the
	/// slab alive (its Drop is the run's single arena free). Blobs placed after
	/// this carve from the arena with zero pool traffic until it is exhausted.
	pub fn from_arena(slab: GpuBuffer) -> Self {
		let mut w = Self::new();
		w.slab = Some(slab);
		w.vram_full = false;
		w
	}

	/// Place one blob. `fill` is called at most once, only when the blob lands
	/// in VRAM or RAM; a DISK placement never reads the bytes.
	pub fn place(
		&mut self,
		name: &str,
		len: usize,
		fill: impl FnOnce(&mut [u8]) -> Result<()>,
	) -> Result<&Home> {
		let home = self.settle(len, fill)?;
		match &home {
			Home::Vram(_) => self.vram_bytes += len,
			Home::Ram(_) => self.ram_bytes += len,
			Home::Disk => self.disk_bytes += len,
		}
		Ok(self.homes.entry(name.to_string()).or_insert(home))
	}

	fn settle(&mut self, len: usize, fill: impl FnOnce(&mut [u8]) -> Result<()>) -> Result<Home> {
		if !self.vram_full {
			// "Full" = the next blob doesn't fit in what's left of the claim.
			// Carves are non-owning and cost zero pool traffic; checking the
			// remainder first means the pool is NEVER touched past the claim.
			if crate::memory::arena_remaining() < len {
				self.vram_full = true;
			} else {
				let _t = tag_scope("waterfall");
				let view = GpuBuffer::alloc_bytes(len).map_err(|e| Error::other(format!("carve: {e}")))?;
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				view.write_u8(&host).map_err(|e| Error::other(format!("waterfall H2D: {e}")))?;
				return Ok(Home::Vram(view));
			}
		}
		if !self.ram_full {
			if mem_available().saturating_sub(len) > self.ram_floor {
				let mut host = vec![0u8; len];
				fill(&mut host)?;
				return Ok(Home::Ram(host));
			}
			self.ram_full = true;
		}
		Ok(Home::Disk)
	}

	pub fn home(&self, name: &str) -> Option<&Home> {
		self.homes.get(name)
	}

	pub fn report(&self) {
		let gb = |b: usize| b as f64 / (1u64 << 30) as f64;
		eprintln!(
			"waterfall: VRAM {:.2} GB → RAM {:.2} GB → DISK {:.2} GB ({} blobs)",
			gb(self.vram_bytes),
			gb(self.ram_bytes),
			gb(self.disk_bytes),
			self.homes.len()
		);
	}
}

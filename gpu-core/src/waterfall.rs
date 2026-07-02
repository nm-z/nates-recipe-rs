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

	/// The one-claim lifecycle: init → ONE pool allocation of everything the
	/// driver reports free, which becomes the process device arena — every
	/// later `GpuBuffer` allocation (norms, activations, staging, blobs, the
	/// hipBLAS workspace) carves from it with zero pool traffic — and exit →
	/// the slab's single free. One growth event (this driver's allocator
	/// stochastically wedges during growth), one memset commits every page
	/// before any bytes land (fresh pool pages read back as stale zeros).
	/// The counters' best guess at claimable VRAM: min of HIP's and the kernel's
	/// free counts minus idle pool holdings, 2 MB-floored. NOT sufficient alone —
	/// both counters over-report what the pool can physically map (an ask past
	/// the real ceiling is an uncatchable VmHeap abort), so the caller must
	/// child-process-probe downward from here and claim the size that SURVIVED.
	pub fn claim_guess() -> usize {
		let hip_free = crate::hip::mem_info().map(|(f, _)| f).unwrap_or(0);
		let sys_free = crate::hip::sysfs_vram_free().unwrap_or(hip_free);
		let slack = crate::hip::pool_slack(0).unwrap_or(0);
		let want = hip_free.min(sys_free).saturating_sub(slack) & !((1 << 21) - 1);
		eprintln!(
			"claim guess: hip_free={:.2} GB sys_free={:.2} GB pool_slack={:.2} GB -> {:.2} GB",
			hip_free as f64 / (1u64 << 30) as f64,
			sys_free as f64 / (1u64 << 30) as f64,
			slack as f64 / (1u64 << 30) as f64,
			want as f64 / (1u64 << 30) as f64
		);
		want
	}

	pub fn claim() -> Self {
		Self::claim_bytes(Self::claim_guess())
	}

	/// Claim exactly `want` bytes (a size the caller has verified mappable).
	pub fn claim_bytes(mut want: usize) -> Self {
		let mut w = Self::new();
		let _t = tag_scope("unclaimed");
		while want > (1 << 20) {
			match GpuBuffer::try_alloc_bytes(want) {
				Some(slab) => {
					if slab.memset_zero(want).is_err() {
						break;
					}
					crate::memory::set_device_arena(slab.ptr_raw(), want);
					w.slab = Some(slab);
					w.vram_full = false;
					break;
				}
				None => want -= want / 16,
			}
		}
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

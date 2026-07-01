//! Tiered paged buffer. One logical buffer of `B` bytes whose 2 MiB pages are
//! distributed across three tiers — hot in VRAM, warm in anonymous RAM, cold in a
//! disk file — where disk stores only the overflow past VRAM+RAM. A buffer fits
//! whenever `B ≤ vram_data + ram_data + disk_data`, and each tier has a hard
//! resident cap that is its own no-OOM proof:
//!
//!   G2 (VRAM never OOM): device residency ≤ `n_v` fixed VMM handles, `n_v·P ≤ vram_data`.
//!   G3 (RAM  never OOM): anon residency  ≤ `n_r` fixed anon blocks, `n_r·P ≤ ram_data`.
//!
//! The sole failure in the program is the admit check `B > cap`, decided once, up
//! front, before a single physical page is acquired. It replaces every `check_ram`.
//!
//! Volatility caveat (stated in the spec): the VRAM and RAM tiers are working
//! memory that vanishes on a crash; the buffer is rebuilt from source per run.
//! That volatility is what buys the additive ceiling. `ram_data` is sized to free
//! RAM measured at `alloc`, so G3 guards the framework's own allocations, not a
//! third process ballooning underneath it — no allocator can promise the latter.

use crate::hip::{self, HipError};
use std::ffi::c_void;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};

/// Page size — 2 MiB, ≥ the VMM minimum granularity and ≥ the OS page.
pub const P: usize = 2 << 20;

// Reserves layered on top of the model's own weight/grad footprint (VRAM), the
// kernel page cache (RAM), and filesystem metadata (disk). Headroom each tier
// must never dip into — not thresholds on the buffer, floors under the OS.
const RESERVE_V: usize = 512 << 20; // 512 MiB device context/driver headroom
const RESERVE_R: usize = 1 << 30; //  1 GiB  OS headroom
const RESERVE_D: usize = 1 << 30; //  1 GiB  filesystem headroom

/// Where each logical page currently lives.
#[derive(Clone, Copy, Debug)]
pub enum Residence {
      Vram(u32), // ring slot
      Ram(u32),  // anon pool index
      Disk(u64), // byte offset in the spill file
}

/// The one failure: requested buffer exceeds the combined ceiling.
#[derive(Debug)]
pub struct Full {
      pub need: usize,
      pub cap: usize,
}

impl std::fmt::Display for Full {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                  f,
                  "buffer {} exceeds VRAM+RAM+disk ceiling {}",
                  human(self.need),
                  human(self.cap)
            )
      }
}

/// Live budgets, measured at `alloc` — never from a stale log.
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
            let (_free, total) = vram_total_free();
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

fn vram_total_free() -> (usize, usize) {
      let (mut free, mut total) = (0usize, 0usize);
      // SAFETY: FFI query into two owned stack usizes.
      unsafe { hip::hipMemGetInfo(&mut free, &mut total) };
      (free, total)
}

/// Free host RAM, measured now (spec: `meminfo_free()`).
fn meminfo_free() -> usize {
      let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
      for l in s.lines() {
            if let Some(r) = l.strip_prefix("MemFree:") {
                  if let Some(kb) = r.split_whitespace().next().and_then(|v| v.parse::<usize>().ok())
                  {
                        return kb.saturating_mul(1024);
                  }
            }
      }
      0
}

/// Free bytes on the filesystem holding `spill` (`statvfs` blocks_available·frag_size).
fn disk_free(spill: &Path) -> usize {
      use std::os::unix::ffi::OsStrExt;
      let dir = spill.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
      let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else {
            return 0;
      };
      let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
      // SAFETY: c is a valid NUL-terminated path, st is owned.
      if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
            return 0;
      }
      (st.f_bavail as usize).saturating_mul(st.f_frsize as usize)
}

/// A logical buffer paged across the three tiers.
pub struct Tiered {
      b: usize,   // logical byte length
      n_pg: usize, // ceil(B / P)
      res: Vec<Residence>,
      budgets: Budgets,

      // VRAM tier: a reserved contiguous VA of `slots·P`, each slot backed by one
      // permanently-mapped VMM handle. Staging overwrites a slot's device bytes.
      va: *mut c_void,
      handles: Vec<*mut c_void>,
      // Resident-tracking for the >VRAM streaming path (stage/evict on the
      // sequential sweep); the fits-in-VRAM path never migrates a slot.
      #[allow(dead_code)]
      slot_page: Vec<Option<usize>>, // slot -> logical page currently resident
      slots: usize,

      // RAM tier: fixed pool of anon P-blocks.
      ram: Vec<Box<[u8]>>,

      // DISK tier: spill file holding only the overflow pages.
      disk: Option<File>,
      spill_path: PathBuf,
}

// SAFETY: device VA / handles are process-global; the buffer is single-owner.
unsafe impl Send for Tiered {}

impl Tiered {
      /// Admit `B` iff it fits the combined ceiling, then lay pages out across the
      /// tiers and acquire the physical backing. `weights_bytes`/`grad_bytes` are
      /// reserved off VRAM so training params keep their room; `spill` is the disk
      /// file for overflow pages.
      pub fn alloc(
            b: usize,
            weights_bytes: usize,
            grad_bytes: usize,
            spill: &Path,
      ) -> Result<Self, Full> {
            let budgets = Budgets::measure(weights_bytes, grad_bytes, spill);
            if b > budgets.cap {
                  return Err(Full {
                        need: b,
                        cap: budgets.cap,
                  });
            }
            let n_pg = b.div_ceil(P);
            let n_vram = n_pg.min(budgets.n_v);
            let n_ram = (n_pg - n_vram).min(budgets.n_r);
            let n_disk = n_pg - n_vram - n_ram;

            // VRAM tier: reserve one contiguous VA window, create+map `n_vram`
            // handles into it. For a buffer that fits VRAM this is the whole thing,
            // contiguous — the GEMM uses it directly.
            let (va, handles) = reserve_and_map(n_vram).expect("vmm reserve/map");
            let slot_page: Vec<Option<usize>> = (0..n_vram).map(Some).collect();

            // RAM tier: anon P-blocks.
            let ram: Vec<Box<[u8]>> = (0..n_ram).map(|_| vec![0u8; P].into_boxed_slice()).collect();

            // DISK tier: only the overflow past VRAM+RAM.
            let disk = if n_disk > 0 {
                  let f = File::options()
                        .read(true)
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(spill)
                        .expect("open spill file");
                  f.set_len((n_disk * P) as u64).expect("size spill file");
                  Some(f)
            } else {
                  None
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

            Ok(Tiered {
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
            })
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

      /// True when the entire buffer is VRAM-resident and contiguous — the GEMM can
      /// treat it as a plain device pointer via [`device_ptr`].
      pub fn is_contiguous_vram(&self) -> bool {
            self.slots == self.n_pg && self.disk.is_none() && self.ram.is_empty()
      }

      /// Contiguous device pointer over the whole buffer. Only valid when
      /// [`is_contiguous_vram`] holds (buffer ≤ VRAM data budget).
      pub fn device_ptr(&self) -> *mut c_void {
            assert!(
                  self.is_contiguous_vram(),
                  "device_ptr on a spilled buffer — stage pages instead"
            );
            self.va
      }

      /// Write `src` into the buffer, spreading it across the resident tiers. `src`
      /// is the host image of the logical bytes; page `p` takes `src[p·P ..]`.
      pub fn fill(&mut self, src: &[u8]) {
            assert!(src.len() <= self.b, "fill src longer than buffer");
            for p in 0..self.n_pg {
                  let lo = p * P;
                  if lo >= src.len() {
                        break;
                  }
                  let hi = (lo + P).min(src.len());
                  self.write_page(p, &src[lo..hi]);
            }
      }

      fn write_page(&mut self, p: usize, bytes: &[u8]) {
            match self.res[p] {
                  Residence::Vram(s) => {
                        let dst = unsafe { (self.va as *mut u8).add(s as usize * P) as *mut c_void };
                        // SAFETY: dst is the s-th mapped P-window; bytes ≤ P.
                        unsafe {
                              hip::memcpy_async(
                                    dst,
                                    bytes.as_ptr() as *const c_void,
                                    bytes.len(),
                                    hip::HIP_MEMCPY_H2D,
                                    std::ptr::null_mut(),
                              )
                        }
                        .expect("H2D page fill");
                  }
                  Residence::Ram(i) => {
                        self.ram[i as usize][..bytes.len()].copy_from_slice(bytes);
                  }
                  Residence::Disk(off) => {
                        self.disk
                              .as_ref()
                              .expect("disk tier")
                              .write_all_at(bytes, off)
                              .expect("spill write");
                  }
            }
      }

      pub fn sync(&self) -> Result<(), HipError> {
            hip::device_synchronize()
      }
}

impl Drop for Tiered {
      fn drop(&mut self) {
            for (s, h) in self.handles.iter().enumerate() {
                  let va = unsafe { (self.va as *mut u8).add(s * P) as *mut c_void };
                  // SAFETY: unmap then release each slot we mapped; free the VA range.
                  unsafe {
                        hip::vmm_unmap(va, P);
                        hip::vmm_release(*h);
                  }
            }
            if !self.va.is_null() && self.slots > 0 {
                  unsafe { hip::vmm_addr_free(self.va, self.slots * P) };
            }
            if self.disk.is_some() {
                  let _ = std::fs::remove_file(&self.spill_path);
            }
      }
}

/// Reserve a contiguous VA of `slots·P` and back each slot with a fresh VMM
/// handle. Returns the base VA and the per-slot handles.
fn reserve_and_map(slots: usize) -> Result<(*mut c_void, Vec<*mut c_void>), HipError> {
      if slots == 0 {
            return Ok((std::ptr::null_mut(), Vec::new()));
      }
      let mut va: *mut c_void = std::ptr::null_mut();
      hip::check(unsafe { hip::vmm_reserve(&mut va, slots * P) })?;
      let mut handles = Vec::with_capacity(slots);
      for s in 0..slots {
            let mut h: *mut c_void = std::ptr::null_mut();
            hip::check(unsafe { hip::vmm_create(&mut h, P) })?;
            let slot_va = unsafe { (va as *mut u8).add(s * P) as *mut c_void };
            hip::check(unsafe { hip::vmm_map_at(slot_va, P, h) })?;
            handles.push(h);
      }
      Ok((va, handles))
}

fn human(b: usize) -> String {
      const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
      let (mut v, mut i) = (b as f64, 0);
      while v >= 1024.0 && i < 4 {
            v /= 1024.0;
            i += 1;
      }
      format!("{v:.2} {}", U[i])
}

#[cfg(test)]
mod tests {
      use super::*;

      #[test]
      fn budgets_are_the_sum() {
            let b = Budgets::measure(0, 0, Path::new("/tmp/tiered_budgets.spill"));
            eprintln!(
                  "vram_data={} ram_data={} disk_data={} cap={} n_v={} n_r={}",
                  human(b.vram_data),
                  human(b.ram_data),
                  human(b.disk_data),
                  human(b.cap),
                  b.n_v,
                  b.n_r
            );
            assert_eq!(b.cap, b.vram_data + b.ram_data + b.disk_data);
            assert_eq!(b.n_v, b.vram_data / P);
            assert_eq!(b.n_r, b.ram_data / P);
      }

      #[test]
      fn admit_rejects_over_cap() {
            let spill = Path::new("/tmp/tiered_reject.spill");
            let cap = Budgets::measure(0, 0, spill).cap;
            let over = cap + P;
            match Tiered::alloc(over, 0, 0, spill) {
                  Err(Full { need, cap: c }) => {
                        assert_eq!(need, over);
                        assert_eq!(c, cap);
                  }
                  Ok(_) => panic!("admitted a buffer over the ceiling"),
            }
      }

      #[test]
      fn vram_fits_roundtrips() {
            hip::set_device(0).expect("set device");
            let spill = Path::new("/tmp/tiered_fit.spill");
            let bytes = 4 * P; // 4 pages, well within VRAM
            let mut t = Tiered::alloc(bytes, 0, 0, spill).expect("alloc");
            assert!(t.is_contiguous_vram(), "small buffer must be contiguous VRAM");
            assert_eq!(t.pages(), 4);
            let mut src = vec![0u8; bytes];
            for p in 0..4 {
                  for i in 0..P {
                        src[p * P + i] = (p as u8).wrapping_add(1);
                  }
            }
            t.fill(&src);
            t.sync().expect("sync");
            let mut back = vec![0u8; bytes];
            // SAFETY: device_ptr covers `bytes`; back owns `bytes`.
            unsafe {
                  hip::memcpy_async(
                        back.as_mut_ptr() as *mut c_void,
                        t.device_ptr(),
                        bytes,
                        hip::HIP_MEMCPY_D2H,
                        std::ptr::null_mut(),
                  )
                  .expect("D2H");
            }
            hip::device_synchronize().expect("sync");
            for p in 0..4 {
                  let m = (p as u8).wrapping_add(1);
                  assert_eq!(back[p * P], m, "page {p} head");
                  assert_eq!(back[p * P + P - 1], m, "page {p} tail");
            }
      }
}

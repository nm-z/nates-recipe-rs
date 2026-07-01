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

/// The admit check that replaces every `check_ram`: the run stops for one reason
/// only — the requested buffer exceeds VRAM+RAM+disk combined — decided from live
/// budgets before anything is allocated. Returns the measured budgets on success
/// (so the caller can log the ceiling it fit under).
pub fn admit(b: usize, weights_bytes: usize, grad_bytes: usize, spill: &Path) -> Result<Budgets, Full> {
      let bud = Budgets::measure(weights_bytes, grad_bytes, spill);
      if b > bud.cap {
            Err(Full { need: b, cap: bud.cap })
      } else {
            Ok(bud)
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
            Ok(Self::build(b, n_pg, n_vram, n_ram, budgets, spill))
      }

      /// Test-only: lay a buffer out with explicit per-tier resident caps, so a
      /// small buffer can be forced to span all three tiers on a machine with
      /// gigabytes of headroom. Budgets are still measured (for the record) but the
      /// caps override the resident split.
      #[cfg(test)]
      pub(crate) fn alloc_capped(b: usize, n_v: usize, n_r: usize, spill: &Path) -> Self {
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

      /// Stage an arbitrary logical byte range `[off, off+len)` into the device
      /// `window` (contiguous, ≥ `len` bytes), gathering each overlapping page's
      /// sub-range from its home tier. Rows do not align to pages, so the row-tiled
      /// trainer stages exact row ranges (`off = r0·d·8`, `len = R·d·8`) and gets a
      /// contiguous `[R×d]` device block.
      pub fn stage_bytes(&self, off: usize, len: usize, window: *mut c_void) {
            let mut scratch = vec![0u8; P];
            let mut done = 0usize;
            while done < len {
                  let gpos = off + done;
                  let p = gpos / P;
                  let poff = gpos % P;
                  let chunk = (P - poff).min(len - done);
                  let dst = unsafe { (window as *mut u8).add(done) as *mut c_void };
                  // SAFETY: dst = window+done covers `chunk`; each tier src is a valid
                  // page sub-range at `poff`.
                  match self.res[p] {
                        Residence::Vram(s) => unsafe {
                              let src = (self.va as *mut u8).add(s as usize * P + poff) as *const c_void;
                              hip::memcpy_async(dst, src, chunk, hip::HIP_MEMCPY_D2D, std::ptr::null_mut())
                                    .expect("stage_bytes D2D");
                        },
                        Residence::Ram(i) => unsafe {
                              let src = self.ram[i as usize].as_ptr().add(poff) as *const c_void;
                              hip::memcpy_async(dst, src, chunk, hip::HIP_MEMCPY_H2D, std::ptr::null_mut())
                                    .expect("stage_bytes H2D");
                        },
                        Residence::Disk(diskoff) => {
                              self.disk
                                    .as_ref()
                                    .expect("disk tier")
                                    .read_exact_at(&mut scratch[..chunk], diskoff + poff as u64)
                                    .expect("stage_bytes read");
                              unsafe {
                                    hip::memcpy_async(
                                          dst,
                                          scratch.as_ptr() as *const c_void,
                                          chunk,
                                          hip::HIP_MEMCPY_H2D,
                                          std::ptr::null_mut(),
                                    )
                                    .expect("stage_bytes disk H2D");
                              }
                        },
                  }
                  done += chunk;
            }
      }

      /// Stage a contiguous run of pages `[first_page, first_page+n_pages)` into the
      /// device `window` (contiguous, ≥ `n_pages·P` bytes), gathering each page from
      /// its home tier — VRAM→D2D, RAM→H2D, disk→read+H2D. This is the sequential
      /// sweep's access: the row-tiled GEMM consumes `window` as one contiguous
      /// device block. `window` is the fixed staging buffer of G3; home pages never
      /// move, so there is no writeback and read-only pages evict as a drop.
      pub fn stage_into(&self, first_page: usize, n_pages: usize, window: *mut c_void) {
            let mut disk_scratch = vec![0u8; P];
            for k in 0..n_pages {
                  let p = first_page + k;
                  if p >= self.n_pg {
                        break;
                  }
                  let bytes = P.min(self.b - p * P);
                  let dst = unsafe { (window as *mut u8).add(k * P) as *mut c_void };
                  // SAFETY: dst is the k-th P-window of `window`; src per tier is a
                  // valid P-region; `bytes` ≤ P.
                  match self.res[p] {
                        Residence::Vram(s) => unsafe {
                              let src = (self.va as *mut u8).add(s as usize * P) as *const c_void;
                              hip::memcpy_async(dst, src, bytes, hip::HIP_MEMCPY_D2D, std::ptr::null_mut())
                                    .expect("stage D2D");
                        },
                        Residence::Ram(i) => unsafe {
                              let src = self.ram[i as usize].as_ptr() as *const c_void;
                              hip::memcpy_async(dst, src, bytes, hip::HIP_MEMCPY_H2D, std::ptr::null_mut())
                                    .expect("stage H2D");
                        },
                        Residence::Disk(off) => {
                              self.disk
                                    .as_ref()
                                    .expect("disk tier")
                                    .read_exact_at(&mut disk_scratch[..bytes], off)
                                    .expect("stage read");
                              // SAFETY: scratch holds `bytes` valid host bytes.
                              unsafe {
                                    hip::memcpy_async(
                                          dst,
                                          disk_scratch.as_ptr() as *const c_void,
                                          bytes,
                                          hip::HIP_MEMCPY_H2D,
                                          std::ptr::null_mut(),
                                    )
                                    .expect("stage disk H2D");
                              }
                        },
                  }
            }
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

      // G1 "and runs": a buffer forced to span VRAM+RAM+disk is trained by a
      // row-tiled full-batch linear model (stream each block through a fixed staging
      // window, accumulate dW/db across blocks, ONE SGD update per epoch) and must
      // reach the SAME weights as the whole-batch trainer on the same data. All
      // device buffers are allocated before the VMM buffer (pool-after-VMM faults).
      #[test]
      fn tiled_full_batch_runs_and_matches_whole() {
            use crate::kernels;
            use crate::memory::GpuBuffer;
            hip::set_device(0).expect("dev");
            let (n, d, o) = (60000usize, 16usize, 4usize); // 7.68 MB → 4 pages
            let epochs = 20;
            let lr = 0.05;
            let mut xh = vec![0f64; n * d];
            let mut yh = vec![0f64; n * o];
            for i in 0..n {
                  for j in 0..d {
                        xh[i * d + j] = (((i * 7 + j * 13) % 97) as f64) / 97.0 - 0.5;
                  }
            }
            for i in 0..n {
                  for k in 0..o {
                        let mut s = 0.0;
                        for j in 0..d {
                              s += xh[i * d + j] * ((((j * 3 + k * 5) % 11) as f64) - 5.0) * 0.1;
                        }
                        yh[i * o + k] = s;
                  }
            }
            let dl = |buf: &GpuBuffer, m: usize| -> Vec<f64> {
                  let mut h = vec![0f64; m];
                  unsafe {
                        hip::memcpy_async(h.as_mut_ptr() as *mut c_void, buf.ptr, m * 8, hip::HIP_MEMCPY_D2H, std::ptr::null_mut())
                              .expect("D2H");
                  }
                  hip::device_synchronize().expect("sync");
                  h
            };
            // ---- all device buffers BEFORE any VMM buffer ----
            let x_dev = GpuBuffer::upload(&xh).expect("x");
            let y_dev = GpuBuffer::upload(&yh).expect("y");
            let make = |sz: usize| GpuBuffer::alloc(sz).expect("buf");
            let (w_ref, b_ref) = (make(d * o), make(o));
            let (w_t, b_t) = (make(d * o), make(o));
            kernels::gpu_scale_inplace(&w_ref, 0.0, d * o);
            kernels::gpu_scale_inplace(&b_ref, 0.0, o);
            kernels::gpu_scale_inplace(&w_t, 0.0, d * o);
            kernels::gpu_scale_inplace(&b_t, 0.0, o);
            let yhat = make(n * o);
            let (dw, db) = (make(d * o), make(o));
            let (dw_acc, db_acc) = (make(d * o), make(o));
            let rws_bytes = kernels::gpu_reduce_sum_cols_workspace_bytes(n, o)
                  .max(kernels::gpu_reduce_sum_cols_workspace_bytes(n * o, 1))
                  .max(kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1));
            let reduce_ws = GpuBuffer::alloc_bytes(rws_bytes).expect("rws");
            let dw_partials = make(kernels::gpu_splitk_dw_partials_elems(n, d, o));
            let rows_per_block = 4096usize;
            let window = make(rows_per_block * d);
            // warm hipBLAS workspace before the VMM buffer exists
            kernels::gpu_linear_into(&x_dev, &w_ref, &b_ref, &yhat, 1, o, d);
            hip::device_synchronize().expect("warmup");

            // ---- reference: whole-batch full-batch GD ----
            let scale = lr / n as f64;
            for _ in 0..epochs {
                  kernels::gpu_linear_into(&x_dev, &w_ref, &b_ref, &yhat, n, o, d);
                  kernels::gpu_sub_inplace(&yhat, &y_dev, n * o);
                  kernels::gpu_linear_backward_weights_only_into(&yhat, &x_dev, &dw, &db, &reduce_ws, &dw_partials, n, o, d);
                  kernels::gpu_sgd_update(&w_ref, &dw, scale, d * o);
                  kernels::gpu_sgd_update(&b_ref, &db, scale, o);
            }
            let w_ref_h = dl(&w_ref, d * o);

            // ---- tiled: X in a forced-spill Tiered buffer, streamed row-blocks ----
            let bytes = n * d * 8;
            let mut t = Tiered::alloc_capped(bytes, 1, 1, Path::new("/tmp/tiled_train.spill")); // 1 VRAM,1 RAM,2 disk
            let xbytes = unsafe { std::slice::from_raw_parts(xh.as_ptr() as *const u8, bytes) };
            t.fill(xbytes);
            t.sync().expect("fill");
            assert!(!t.is_contiguous_vram(), "buffer must span >1 tier");
            for _ in 0..epochs {
                  kernels::gpu_scale_inplace(&dw_acc, 0.0, d * o);
                  kernels::gpu_scale_inplace(&db_acc, 0.0, o);
                  let mut r0 = 0;
                  while r0 < n {
                        let r = rows_per_block.min(n - r0);
                        t.stage_bytes(r0 * d * 8, r * d * 8, window.ptr);
                        kernels::gpu_linear_into(&window, &w_t, &b_t, &yhat, r, o, d);
                        let yblk = GpuBuffer::borrow(unsafe { (y_dev.ptr as *mut f64).add(r0 * o) as *mut c_void }, r * o * 8);
                        kernels::gpu_sub_inplace(&yhat, &yblk, r * o);
                        kernels::gpu_linear_backward_weights_only_into(&yhat, &window, &dw, &db, &reduce_ws, &dw_partials, r, o, d);
                        kernels::gpu_add_inplace(&dw_acc, &dw, d * o);
                        kernels::gpu_add_inplace(&db_acc, &db, o);
                        r0 += r;
                  }
                  kernels::gpu_sgd_update(&w_t, &dw_acc, scale, d * o);
                  kernels::gpu_sgd_update(&b_t, &db_acc, scale, o);
            }
            let w_t_h = dl(&w_t, d * o);
            let maxdiff = w_ref_h
                  .iter()
                  .zip(&w_t_h)
                  .map(|(a, b)| (a - b).abs())
                  .fold(0.0f64, f64::max);
            eprintln!("[tiled] pages={} spilled=true maxdiff(tiled vs whole)={maxdiff:e}", t.pages());
            assert!(maxdiff < 1e-9, "tiled full-batch must match whole-batch: maxdiff={maxdiff}");
      }

      #[test]
      fn stage_across_three_tiers() {
            hip::set_device(0).expect("set device");
            let spill = Path::new("/tmp/tiered_3tier.spill");
            let pages = 6usize;
            let bytes = pages * P;
            // Allocate + prove the window BEFORE any VMM buffer exists, to bisect
            // whether VMM setup corrupts the stream-ordered pool.
            let window = unsafe { hip::malloc_async(bytes, std::ptr::null_mut()).expect("window") };
            // Force 2 pages per tier so the sweep exercises VRAM, RAM, and disk.
            let mut t = Tiered::alloc_capped(bytes, 2, 2, spill);
            assert_eq!(t.pages(), pages);
            assert!(!t.is_contiguous_vram(), "capped buffer must span >1 tier");
            let mut src = vec![0u8; bytes];
            for p in 0..pages {
                  for i in 0..P {
                        src[p * P + i] = (p as u8).wrapping_add(1);
                  }
            }
            t.fill(&src);
            t.sync().expect("sync");
            // Stage every page into a contiguous device window, read it back, verify
            // each page survived a round trip through its home tier.
            t.stage_into(0, pages, window);
            hip::device_synchronize().expect("sync");
            let mut back = vec![0u8; bytes];
            // SAFETY: window covers `bytes`; back owns `bytes`.
            unsafe {
                  hip::memcpy_async(
                        back.as_mut_ptr() as *mut c_void,
                        window,
                        bytes,
                        hip::HIP_MEMCPY_D2H,
                        std::ptr::null_mut(),
                  )
                  .expect("D2H");
            }
            hip::device_synchronize().expect("sync");
            for p in 0..pages {
                  let m = (p as u8).wrapping_add(1);
                  assert_eq!(back[p * P], m, "page {p} head");
                  assert_eq!(back[p * P + P - 1], m, "page {p} tail");
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

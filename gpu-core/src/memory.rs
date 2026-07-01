use crate::hip::*;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

// When set, drain the device after each hipMallocAsync so the pool commits the
// new buffer's pages before anything (notably an SDMA host->device copy, which
// runs on a queue not ordered with the alloc) writes into it. Off by default —
// training uploads once and never churns; streaming inference (fresh alloc +
// immediate copy, thousands of times) needs it to avoid GPU page faults.
static ALLOC_SYNC: AtomicBool = AtomicBool::new(false);

/// Enable/disable the post-allocation device sync (see `ALLOC_SYNC`).
pub fn set_alloc_sync(on: bool) {
	ALLOC_SYNC.store(on, Ordering::Relaxed);
}

// Live device bytes per purpose-tag. On a VRAM OOM we dump this so the failure
// names what is on the card (data / weights / scratch / other), not just a size.
static TAG_BYTES: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());

thread_local! {
	static CURRENT_TAG: Cell<&'static str> = const { Cell::new("other") };
}

/// Sets the purpose-tag for every allocation made while it is alive; restores
/// the previous tag on drop. Wrap an allocation phase: `let _t = tag_scope("data");`.
pub struct TagScope(&'static str);

pub fn tag_scope(name: &'static str) -> TagScope {
	let prev = CURRENT_TAG.with(|t| t.replace(name));
	TagScope(prev)
}

impl Drop for TagScope {
	fn drop(&mut self) {
		CURRENT_TAG.with(|t| t.set(self.0));
	}
}

fn tag_add(tag: &'static str, n: usize) {
	if let Ok(mut m) = TAG_BYTES.lock() {
		*m.entry(tag).or_insert(0) += n;
	}
}

fn tag_sub(tag: &'static str, n: usize) {
	if let Ok(mut m) = TAG_BYTES.lock() {
		let e = m.entry(tag).or_insert(0);
		*e = e.saturating_sub(n);
	}
}

fn fmt_bytes(b: usize) -> String {
	const K: f64 = 1024.0;
	let f = b as f64;
	if f >= K * K * K {
		format!("{:.2} GB", f / (K * K * K))
	} else if f >= K * K {
		format!("{:.2} MB", f / (K * K))
	} else if f >= K {
		format!("{:.2} KB", f / K)
	} else {
		format!("{b} B")
	}
}

fn oom_pair(name: &str, val: &str) -> String {
	format!("\x1b[1;31m{name}:\x1b[0m \x1b[1m{val}\x1b[0m")
}

// One-line VRAM autopsy: live tags largest-first, then request/free/total/over.
fn oom_report(req: usize) {
	let (free, total) = crate::hip::mem_info().unwrap_or((0, 0));
	let mut autopsy: Vec<(&'static str, usize)> = TAG_BYTES
		.lock()
		.map(|m| m.iter().map(|(k, v)| (*k, *v)).filter(|(_, v)| *v > 0).collect())
		.unwrap_or_default();
	autopsy.sort_by(|a, b| b.1.cmp(&a.1));
	let mut line: Vec<String> = autopsy.iter().map(|(k, v)| oom_pair(k, &fmt_bytes(*v))).collect();
	line.push(oom_pair("req", &fmt_bytes(req)));
	line.push(oom_pair("free", &fmt_bytes(free)));
	line.push(oom_pair("total", &fmt_bytes(total)));
	line.push(oom_pair("over", &fmt_bytes(req.saturating_sub(free))));
	eprintln!("{}", line.join(", "));
}

pub fn mark_shutting_down() {
      SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

thread_local! {
	static ALLOC_FROZEN: Cell<bool> = const { Cell::new(false) };
}

pub fn alloc_count_reset() -> usize {
	ALLOC_COUNT.swap(0, Ordering::Relaxed)
}

pub fn alloc_freeze() {
	ALLOC_FROZEN.with(|f| f.set(true));
}

pub fn alloc_unfreeze() {
	ALLOC_FROZEN.with(|f| f.set(false));
}

pub struct AllocGuard(std::marker::PhantomData<*const ()>);

impl AllocGuard {
      pub fn freeze() -> Self {
            alloc_freeze();
            AllocGuard(std::marker::PhantomData)
      }
}

impl Drop for AllocGuard {
      fn drop(&mut self) {
            alloc_unfreeze();
      }
}

const ARENA_ALIGN: usize = 256;
static ARENA_BASE: AtomicUsize = AtomicUsize::new(0);
static ARENA_SIZE: AtomicUsize = AtomicUsize::new(0);
static ARENA_OFFSET: AtomicUsize = AtomicUsize::new(0);

pub struct GpuBuffer {
	pub(crate) ptr: *mut c_void,
	len: usize,
	owned: bool,
	tag: &'static str,
}

// SAFETY: HIP device pointers are thread-safe; the runtime serializes kernel launches per-stream.
unsafe impl Send for GpuBuffer {}
unsafe impl Sync for GpuBuffer {}

impl GpuBuffer {
	pub fn borrow(ptr: *mut c_void, len: usize) -> Self {
		Self {
			ptr,
			len,
			owned: false,
			tag: "borrow",
		}
	}

	pub fn alloc(n_floats: usize) -> Result<Self, HipError> {
		Self::alloc_bytes(n_floats * std::mem::size_of::<f64>())
	}

	pub fn alloc_bytes(n_bytes: usize) -> Result<Self, HipError> {
		ALLOC_FROZEN.with(|f| {
			assert!(
				!f.get(),
				"GPU allocation inside frozen training loop (requested {n_bytes} bytes)"
			)
		});
		ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
		let tag = CURRENT_TAG.with(|t| t.get());
		let base = ARENA_BASE.load(Ordering::Relaxed);
		if base != 0 {
			let size = ARENA_SIZE.load(Ordering::Relaxed);
			let aligned = (n_bytes + ARENA_ALIGN - 1) & !(ARENA_ALIGN - 1);
			let mut off = ARENA_OFFSET.load(Ordering::Relaxed);
			while off + aligned <= size {
				match ARENA_OFFSET.compare_exchange_weak(
					off,
					off + aligned,
					Ordering::Relaxed,
					Ordering::Relaxed,
				) {
					Ok(_) => {
						let ptr = unsafe { (base as *mut u8).add(off) as *mut c_void };
						tag_add(tag, n_bytes);
						return Ok(Self {
							ptr,
							len: n_bytes,
							owned: false,
							tag,
						});
					}
					Err(cur) => off = cur,
				}
			}
		}
		let mut ptr: *mut c_void = std::ptr::null_mut();
		let code = unsafe { hipMallocAsync(&mut ptr, n_bytes, std::ptr::null_mut()) };
		if code == 2 {
			oom_report(n_bytes);
		}
		check(code)?;
		if ALLOC_SYNC.load(Ordering::Relaxed) {
			check(unsafe { hipDeviceSynchronize() })?;
		}
		tag_add(tag, n_bytes);
		Ok(Self {
			ptr,
			len: n_bytes,
			owned: true,
			tag,
		})
	}

	pub fn upload(data: &[f64]) -> Result<Self, HipError> {
		let buf = Self::alloc(data.len())?;
		let bytes = std::mem::size_of_val(data);
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	pub fn upload_u8(data: &[u8]) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(data.len())?;
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				data.len(),
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	/// Copy host bytes into this (already-allocated) buffer — the reuse path for
	/// a persistent staging window, avoiding a fresh alloc per upload.
	pub fn write_u8(&self, data: &[u8]) -> Result<(), HipError> {
		assert!(
			data.len() <= self.len,
			"write_u8: {} bytes into a {}-byte buffer",
			data.len(),
			self.len
		);
		check(unsafe {
			hipMemcpy(self.ptr, data.as_ptr() as *const c_void, data.len(), HIP_MEMCPY_H2D)
		})
	}

	pub fn upload_f32(data: &[f32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	pub fn upload_i32(data: &[i32]) -> Result<Self, HipError> {
		let bytes = data.len() * 4;
		let buf = Self::alloc_bytes(bytes)?;
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	pub fn zeros_bytes(n_bytes: usize) -> Result<Self, HipError> {
		let buf = Self::alloc_bytes(n_bytes)?;
		check(unsafe { hipMemset(buf.ptr, 0, n_bytes) })?;
		Ok(buf)
	}

	pub fn zeros_f32(n: usize) -> Result<Self, HipError> {
		Self::zeros_bytes(n * 4)
	}

	pub fn memset_zero(&self, n_bytes: usize) -> Result<(), HipError> {
		check(unsafe { hipMemset(self.ptr, 0, n_bytes) })
	}

	pub fn download(&self, dst: &mut [f64]) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
			)
		})
	}

	pub fn download_f32(&self, dst: &mut [f32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
			)
		})
	}

	pub fn download_u8(&self, dst: &mut [u8]) -> Result<(), HipError> {
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				dst.len(),
				HIP_MEMCPY_D2H,
			)
		})
	}

	pub fn download_i32(&self, dst: &mut [i32]) -> Result<(), HipError> {
		let bytes = dst.len() * 4;
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr,
				bytes,
				HIP_MEMCPY_D2H,
			)
		})
	}

	pub fn len(&self) -> usize {
		self.len
	}
	pub fn n_floats(&self) -> usize {
		self.len / std::mem::size_of::<f64>()
	}
	pub fn ptr_addr(&self) -> usize {
		self.ptr as usize
	}
	pub fn ptr_raw(&self) -> *mut c_void {
		self.ptr
	}

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	pub fn as_ptr_offset(&self, n_floats: usize) -> *mut c_void {
		assert!(
			n_floats * 8 <= self.len,
			"as_ptr_offset: offset {} bytes exceeds buffer len {}",
			n_floats * 8,
			self.len
		);
		unsafe { (self.ptr as *mut u8).add(n_floats * 8) as *mut c_void }
	}

	pub fn view(&self, offset_floats: usize, len_floats: usize) -> GpuBuffer {
		GpuBuffer::borrow(self.as_ptr_offset(offset_floats), len_floats * 8)
	}

	pub fn copy_from(&mut self, src: &GpuBuffer, n_bytes: usize) -> Result<(), HipError> {
		check(unsafe { hipMemcpy(self.ptr, src.ptr as *const c_void, n_bytes, HIP_MEMCPY_D2D) })
	}

	pub fn fill_bytes(&self, value: u8, n_bytes: usize) -> Result<(), HipError> {
		check(unsafe { hipMemset(self.ptr, value as i32, n_bytes) })
	}

	pub unsafe fn upload_async(data: &[f64], stream: *mut c_void) -> Result<Self, HipError> {
		let bytes = std::mem::size_of_val(data);
		let buf = Self::alloc(data.len())?;
		// SAFETY: FFI call — caller must ensure pointer validity and size.
		check(unsafe {
			hipMemcpyAsync(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
				stream,
			)
		})?;
		Ok(buf)
	}

	pub unsafe fn download_async(
		&self,
		dst: &mut [f64],
		stream: *mut c_void,
	) -> Result<(), HipError> {
		let bytes = std::mem::size_of_val(dst);
		// SAFETY: FFI call — caller must ensure pointer validity and size.
		check(unsafe {
			hipMemcpyAsync(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr as *const c_void,
				bytes,
				HIP_MEMCPY_D2H,
				stream,
			)
		})
	}

	pub fn download_vec(&self) -> Result<Vec<f64>, HipError> {
		let mut v = vec![0.0f64; self.n_floats()];
		self.download(&mut v)?;
		Ok(v)
	}

	pub fn download_vec_f32(&self) -> Result<Vec<f32>, HipError> {
		let mut v = vec![0.0f32; self.len / 4];
		self.download_f32(&mut v)?;
		Ok(v)
	}

	pub fn upload_f16(data: &[half::f16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	pub fn download_f16(&self, dst: &mut [half::f16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr as *const c_void,
				bytes,
				HIP_MEMCPY_D2H,
			)
		})
	}

	pub fn upload_bf16(data: &[half::bf16]) -> Result<Self, HipError> {
		let bytes = data.len() * 2;
		let buf = Self::alloc_bytes(bytes)?;
		check(unsafe {
			hipMemcpy(
				buf.ptr,
				data.as_ptr() as *const c_void,
				bytes,
				HIP_MEMCPY_H2D,
			)
		})?;
		Ok(buf)
	}

	pub fn download_bf16(&self, dst: &mut [half::bf16]) -> Result<(), HipError> {
		let bytes = dst.len() * 2;
		check(unsafe {
			hipMemcpy(
				dst.as_mut_ptr() as *mut c_void,
				self.ptr as *const c_void,
				bytes,
				HIP_MEMCPY_D2H,
			)
		})
	}
}

impl Drop for GpuBuffer {
	fn drop(&mut self) {
		if self.owned && !self.ptr.is_null() && !SHUTTING_DOWN.load(Ordering::Relaxed) {
			tag_sub(self.tag, self.len);
			unsafe { hipFreeAsync(self.ptr, std::ptr::null_mut()) };
			self.ptr = std::ptr::null_mut();
		}
	}
}

#![allow(unsafe_code)]

use core::alloc::{GlobalAlloc, Layout};
use std::alloc::System;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

pub type Mat = ndarray::Array2<f64>;
pub type Vec1 = ndarray::Array1<f64>;

pub const TEXT_CONTEXT: usize = 256;

pub mod bpe;
pub mod data;
pub mod detect;
pub mod encode;
pub mod formats;

pub use data::*;
pub use detect::*;

#[derive(Clone)]
pub enum Kind {
	Numeric,
	Temporal,
	Categorical(Vec<String>),
	Ordinal(Vec<String>),
	Text(Vec<String>),
	Image,
}

#[derive(Clone)]
pub struct Attr {
	pub name: String,
	pub kind: Kind,
}

pub fn available_ram_bytes() -> usize {
	fs::read_to_string("/proc/meminfo")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("MemAvailable:"))
				.and_then(|l| l.split_whitespace().nth(1))
				.and_then(|v| v.parse::<usize>().ok())
		})
		.map_or(usize::MAX, |kb| kb.saturating_mul(1024))
}

// ─

static HEAP_LIVE: AtomicUsize = AtomicUsize::new(0);
static HEAP_TOTAL: AtomicUsize = AtomicUsize::new(0);

pub struct TrackedAlloc;

unsafe impl GlobalAlloc for TrackedAlloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		// SAFETY: layout is forwarded unchanged to the system allocator
		let p = unsafe { System.alloc(layout) };
		if !p.is_null() {
			HEAP_LIVE.fetch_add(layout.size(), Ordering::Relaxed);
			HEAP_TOTAL.fetch_add(layout.size(), Ordering::Relaxed);
		}
		return p;
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		// SAFETY: layout is forwarded unchanged to the system allocator
		let p = unsafe { System.alloc_zeroed(layout) };
		if !p.is_null() {
			HEAP_LIVE.fetch_add(layout.size(), Ordering::Relaxed);
			HEAP_TOTAL.fetch_add(layout.size(), Ordering::Relaxed);
		}
		return p;
	}

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		// SAFETY: ptr and layout uphold the caller's GlobalAlloc contract
		let p = unsafe { System.realloc(ptr, layout, new_size) };
		if !p.is_null() {
			HEAP_LIVE.fetch_add(new_size, Ordering::Relaxed);
			HEAP_LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
			HEAP_TOTAL.fetch_add(new_size, Ordering::Relaxed);
		}
		return p;
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		// SAFETY: ptr and layout uphold the caller's GlobalAlloc contract
		unsafe { System.dealloc(ptr, layout) };
		HEAP_LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
		return;
	}
}

#[global_allocator]
static HEAP: TrackedAlloc = TrackedAlloc;

pub fn heap_live_bytes() -> usize {
	return HEAP_LIVE.load(Ordering::Relaxed);
}

pub fn heap_total_bytes() -> usize {
	return HEAP_TOTAL.load(Ordering::Relaxed);
}

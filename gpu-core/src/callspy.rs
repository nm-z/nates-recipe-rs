//! Runtime tally of every HIP call the framework makes. Every hip API is
//! wrapped at exactly one choke point, so each wrapper ticks its counter and
//! `report()` prints the full tree at shutdown. Relaxed atomics — a tick is
//! one fetch_add, safe on the hottest paths (kernel launches).

use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! counters {
	($($name:ident),* $(,)?) => {
		$(pub(crate) static $name: AtomicU64 = AtomicU64::new(0);)*
	};
}

counters!(
	HOST_MALLOC, HOST_FREE,
	MEMCPY_ASYNC, MALLOC_ASYNC, MEMSET_ASYNC, FREE_ASYNC,
	XFER_ASYNC,
	LAUNCH,
	GET_LAST_ERROR, PEEK_AT_LAST_ERROR, GET_ERROR_STRING, GET_ERROR_NAME,
	EVENT_RECORD, EVENT_ELAPSED_TIME, EVENT_DESTROY, EVENT_CREATE,
	STREAM_SYNCHRONIZE, DEVICE_SYNCHRONIZE, EVENT_SYNCHRONIZE,
	STREAM_DESTROY, STREAM_CREATE,
	MEM_GET_INFO, SET_DEVICE, GET_DEVICE_COUNT, DEVICE_GET_ATTRIBUTE,
	DEVICE_ENABLE_PEER_ACCESS, DEVICE_CAN_ACCESS_PEER, GET_DEVICE_PROPERTIES, GET_DEVICE,
	GET_DEFAULT_MEMPOOL, MEMPOOL_GET_ATTRIBUTE, MEMPOOL_TRIM_TO, MEMPOOL_SET_ATTRIBUTE,
	MEM_UNMAP, MEM_SET_ACCESS, MEM_RELEASE, MEM_MAP,
	MEM_GET_ALLOCATION_GRANULARITY, MEM_CREATE, MEM_ADDRESS_RESERVE, MEM_ADDRESS_FREE,
	HIPBLAS,
);

#[inline]
pub(crate) fn tick(c: &AtomicU64) {
	c.fetch_add(1, Ordering::Relaxed);
}

pub const N: usize = 42;
static ALL: [&AtomicU64; N] = [
	&HOST_MALLOC, &HOST_FREE,
	&MEMCPY_ASYNC, &MALLOC_ASYNC, &MEMSET_ASYNC, &FREE_ASYNC,
	&XFER_ASYNC,
	&LAUNCH,
	&GET_LAST_ERROR, &PEEK_AT_LAST_ERROR, &GET_ERROR_STRING, &GET_ERROR_NAME,
	&EVENT_RECORD, &EVENT_ELAPSED_TIME, &EVENT_DESTROY, &EVENT_CREATE,
	&STREAM_SYNCHRONIZE, &DEVICE_SYNCHRONIZE, &EVENT_SYNCHRONIZE,
	&STREAM_DESTROY, &STREAM_CREATE,
	&MEM_GET_INFO, &SET_DEVICE, &GET_DEVICE_COUNT, &DEVICE_GET_ATTRIBUTE,
	&DEVICE_ENABLE_PEER_ACCESS, &DEVICE_CAN_ACCESS_PEER, &GET_DEVICE_PROPERTIES, &GET_DEVICE,
	&GET_DEFAULT_MEMPOOL, &MEMPOOL_GET_ATTRIBUTE, &MEMPOOL_TRIM_TO, &MEMPOOL_SET_ATTRIBUTE,
	&MEM_UNMAP, &MEM_SET_ACCESS, &MEM_RELEASE, &MEM_MAP,
	&MEM_GET_ALLOCATION_GRANULARITY, &MEM_CREATE, &MEM_ADDRESS_RESERVE, &MEM_ADDRESS_FREE,
	&HIPBLAS,
];

/// Counter values right now — pass to `report_since` for a run-scoped delta.
pub fn snapshot() -> [u64; N] {
	let mut s = [0u64; N];
	for (i, c) in ALL.iter().enumerate() {
		s[i] = c.load(Ordering::Relaxed);
	}
	s
}

pub fn report() -> String {
	report_since(&[0u64; N])
}

pub fn report_since(base: &[u64; N]) -> String {
	report_between(base, &snapshot())
}

/// Delta between two snapshots — phase-scoped counts (init/loop/exit).
pub fn report_between(base: &[u64; N], end: &[u64; N]) -> String {
	let g = |c: &AtomicU64| {
		let i = ALL
			.iter()
			.position(|x| std::ptr::eq(*x, c))
			.expect("counter registered in ALL");
		end[i].saturating_sub(base[i])
	};
	let groups: &[(&str, &[(u64, &str)])] = &[
		("sync", &[
			(g(&HOST_MALLOC), "allocations"),
			(g(&HOST_FREE), "frees"),
		]),
		("async", &[
			(g(&MEMCPY_ASYNC), "transfers"),
			(g(&MALLOC_ASYNC), "allocations"),
			(g(&MEMSET_ASYNC), "memsets"),
			(g(&FREE_ASYNC), "frees"),
		]),
		("kernel launch", &[(g(&LAUNCH), "hipLaunchKernelGGL")]),
		("reporting", &[
			(g(&GET_LAST_ERROR), "hipGetLastError"),
			(g(&PEEK_AT_LAST_ERROR), "hipPeekAtLastError"),
			(g(&GET_ERROR_STRING), "hipGetErrorString"),
			(g(&GET_ERROR_NAME), "hipGetErrorName"),
			(g(&EVENT_RECORD), "hipEventRecord"),
			(g(&EVENT_ELAPSED_TIME), "hipEventElapsedTime"),
			(g(&EVENT_DESTROY), "hipEventDestroy"),
			(g(&EVENT_CREATE), "hipEventCreate"),
		]),
		("syncs", &[
			(g(&STREAM_SYNCHRONIZE), "hipStreamSynchronize"),
			(g(&DEVICE_SYNCHRONIZE), "hipDeviceSynchronize"),
			(g(&EVENT_SYNCHRONIZE), "hipEventSynchronize"),
		]),
		("streams", &[
			(g(&STREAM_DESTROY), "hipStreamDestroy"),
			(g(&STREAM_CREATE), "hipStreamCreate"),
		]),
		("device/settings", &[
			(g(&MEM_GET_INFO), "hipMemGetInfo"),
			(g(&SET_DEVICE), "hipSetDevice"),
			(g(&GET_DEVICE_COUNT), "hipGetDeviceCount"),
			(g(&DEVICE_GET_ATTRIBUTE), "hipDeviceGetAttribute"),
			(g(&DEVICE_ENABLE_PEER_ACCESS), "hipDeviceEnablePeerAccess"),
			(g(&DEVICE_CAN_ACCESS_PEER), "hipDeviceCanAccessPeer"),
			(g(&GET_DEVICE_PROPERTIES), "hipGetDeviceProperties"),
			(g(&GET_DEVICE), "hipGetDevice"),
		]),
		("pool", &[
			(g(&GET_DEFAULT_MEMPOOL), "hipDeviceGetDefaultMemPool"),
			(g(&MEMPOOL_GET_ATTRIBUTE), "hipMemPoolGetAttribute"),
			(g(&MEMPOOL_TRIM_TO), "hipMemPoolTrimTo"),
			(g(&MEMPOOL_SET_ATTRIBUTE), "hipMemPoolSetAttribute"),
		]),
		("VMM", &[
			(g(&MEM_UNMAP), "hipMemUnmap"),
			(g(&MEM_SET_ACCESS), "hipMemSetAccess"),
			(g(&MEM_RELEASE), "hipMemRelease"),
			(g(&MEM_MAP), "hipMemMap"),
			(g(&MEM_GET_ALLOCATION_GRANULARITY), "hipMemGetAllocationGranularity"),
			(g(&MEM_CREATE), "hipMemCreate"),
			(g(&MEM_ADDRESS_RESERVE), "hipMemAddressReserve"),
			(g(&MEM_ADDRESS_FREE), "hipMemAddressFree"),
		]),
		("other", &[(g(&HIPBLAS), "hipBLAS")]),
	];
	let mut out = String::new();
	for (group, entries) in groups {
		if entries.iter().all(|(n, _)| *n == 0) {
			continue;
		}
		out.push_str(group);
		out.push('\n');
		for (n, name) in *entries {
			if *n > 0 {
				out.push_str(&format!("{n:>13} {name}\n"));
			}
		}
	}
	out
}

// ── The run state table ──────────────────────────────────────────────────────
// Every training run prints the spec's init/loop/exit tree from raw counter
// deltas at the phase boundaries (run entry → first epoch → last epoch → run
// end). Cells: in-place = kernel launches; alloc = hipHostMalloc + hipMallocAsync
// + hipMemCreate; async = standalone enqueue-only transfer calls (XFER_ASYNC —
// transfers riding a claim/park op and blocking transfers are not standalone);
// sync = every call that blocks the calling thread (hipStreamSynchronize +
// hipDeviceSynchronize + hipEventSynchronize). Vendor hipBLAS and hipFreeAsync
// have no spec cell — nonzero counts print below the tree, never silently.

static LOOP_START: std::sync::Mutex<Option<[u64; N]>> = std::sync::Mutex::new(None);
static LOOP_END: std::sync::Mutex<Option<[u64; N]>> = std::sync::Mutex::new(None);

/// First epoch is about to run — everything before this is the run's init.
pub fn mark_loop_start() {
	*LOOP_START.lock().unwrap_or_else(|p| p.into_inner()) = Some(snapshot());
}

/// Last epoch just finished — everything after this is the run's exit.
pub fn mark_loop_end() {
	*LOOP_END.lock().unwrap_or_else(|p| p.into_inner()) = Some(snapshot());
}

/// The per-run state table: raw deltas between `run_start`, the two loop marks,
/// and now. `None` when no loop ran (forward-only / skipped run). Consumes the
/// marks so a later run can never mix phases across runs.
pub fn state_report(run_start: &[u64; N]) -> Option<String> {
	let ls = LOOP_START.lock().unwrap_or_else(|p| p.into_inner()).take()?;
	let le = LOOP_END.lock().unwrap_or_else(|p| p.into_inner()).take()?;
	let end = snapshot();
	let idx = |c: &AtomicU64| {
		ALL.iter().position(|x| std::ptr::eq(*x, c)).expect("counter registered in ALL")
	};
	let cell = |a: &[u64; N], b: &[u64; N], cs: &[&AtomicU64]| -> u64 {
		cs.iter().map(|c| b[idx(c)].saturating_sub(a[idx(c)])).sum()
	};
	let phases: [(&str, &[u64; N], &[u64; N]); 3] =
		[("init", run_start, &ls), ("loop", &ls, &le), ("exit", &le, &end)];
	let mut out = String::new();
	let mut hipblas = [0u64; 3];
	let mut frees = [0u64; 3];
	for (i, (name, a, b)) in phases.iter().enumerate() {
		out.push_str(&format!("{name}\n    calcs\n"));
		out.push_str(&format!("        {:<8}{:>7}\n", "in-place", format!("{}x", cell(a, b, &[&LAUNCH]))));
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"alloc",
			format!("{}x", cell(a, b, &[&HOST_MALLOC, &MALLOC_ASYNC, &MEM_CREATE]))
		));
		out.push_str("    transfers\n");
		out.push_str(&format!("        {:<8}{:>7}\n", "async", format!("{}x", cell(a, b, &[&XFER_ASYNC]))));
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"sync",
			format!("{}x", cell(a, b, &[&STREAM_SYNCHRONIZE, &DEVICE_SYNCHRONIZE, &EVENT_SYNCHRONIZE]))
		));
		hipblas[i] = cell(a, b, &[&HIPBLAS]);
		frees[i] = cell(a, b, &[&HOST_FREE, &FREE_ASYNC, &MEM_RELEASE]);
	}
	for (what, v) in [("hipBLAS", hipblas), ("free", frees)] {
		if v.iter().sum::<u64>() > 0 {
			out.push_str(&format!(
				"{what} (no spec cell)  init {}x  loop {}x  exit {}x\n",
				v[0], v[1], v[2]
			));
		}
	}
	Some(out)
}

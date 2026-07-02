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

pub fn report() -> String {
	let g = |c: &AtomicU64| c.load(Ordering::Relaxed);
	let groups: &[(&str, &[(u64, &str)])] = &[
		("sync", &[
			(g(&HOST_MALLOC), "allocations"),
			(g(&HOST_FREE), "frees"),
		]),
		("async", &[
			(g(&MEMCPY_ASYNC), "transfers"),
			(g(&MALLOC_ASYNC) + g(&MEMSET_ASYNC), "allocations"),
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

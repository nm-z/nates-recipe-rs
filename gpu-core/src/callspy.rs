use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! counters {
	($($name:ident),* $(,)?) => {
		$(pub(crate) static $name: AtomicU64 = AtomicU64::new(0);)*
	};
}

counters!(
	HOST_MALLOC,
	HOST_FREE,
	MEMCPY_ASYNC,
	MALLOC_ASYNC,
	MEMSET_ASYNC,
	FREE_ASYNC,
	XFER_ASYNC,
	LAUNCH,
	GET_LAST_ERROR,
	PEEK_AT_LAST_ERROR,
	GET_ERROR_STRING,
	GET_ERROR_NAME,
	EVENT_RECORD,
	EVENT_ELAPSED_TIME,
	EVENT_DESTROY,
	EVENT_CREATE,
	STREAM_SYNCHRONIZE,
	DEVICE_SYNCHRONIZE,
	EVENT_SYNCHRONIZE,
	STREAM_DESTROY,
	STREAM_CREATE,
	MEM_GET_INFO,
	SET_DEVICE,
	GET_DEVICE_COUNT,
	DEVICE_GET_ATTRIBUTE,
	DEVICE_ENABLE_PEER_ACCESS,
	DEVICE_CAN_ACCESS_PEER,
	GET_DEVICE_PROPERTIES,
	GET_DEVICE,
	GET_DEFAULT_MEMPOOL,
	MEMPOOL_GET_ATTRIBUTE,
	MEMPOOL_TRIM_TO,
	MEMPOOL_SET_ATTRIBUTE,
	MEM_UNMAP,
	MEM_SET_ACCESS,
	MEM_RELEASE,
	MEM_MAP,
	MEM_GET_ALLOCATION_GRANULARITY,
	MEM_CREATE,
	MEM_ADDRESS_RESERVE,
	MEM_ADDRESS_FREE,
	HIPBLAS,
	HOST_REGISTER,
	HOST_UNREGISTER,
	MANAGED_MALLOC,
	MEM_ADVISE,
	HOST_GET_DEVICE_POINTER,
);

#[inline]
pub(crate) fn tick(c: &AtomicU64) {
	c.fetch_add(1, Ordering::Relaxed);
}

pub const N: usize = 47;
static ALL: [&AtomicU64; N] = [
	&HOST_MALLOC,
	&HOST_FREE,
	&MEMCPY_ASYNC,
	&MALLOC_ASYNC,
	&MEMSET_ASYNC,
	&FREE_ASYNC,
	&XFER_ASYNC,
	&LAUNCH,
	&GET_LAST_ERROR,
	&PEEK_AT_LAST_ERROR,
	&GET_ERROR_STRING,
	&GET_ERROR_NAME,
	&EVENT_RECORD,
	&EVENT_ELAPSED_TIME,
	&EVENT_DESTROY,
	&EVENT_CREATE,
	&STREAM_SYNCHRONIZE,
	&DEVICE_SYNCHRONIZE,
	&EVENT_SYNCHRONIZE,
	&STREAM_DESTROY,
	&STREAM_CREATE,
	&MEM_GET_INFO,
	&SET_DEVICE,
	&GET_DEVICE_COUNT,
	&DEVICE_GET_ATTRIBUTE,
	&DEVICE_ENABLE_PEER_ACCESS,
	&DEVICE_CAN_ACCESS_PEER,
	&GET_DEVICE_PROPERTIES,
	&GET_DEVICE,
	&GET_DEFAULT_MEMPOOL,
	&MEMPOOL_GET_ATTRIBUTE,
	&MEMPOOL_TRIM_TO,
	&MEMPOOL_SET_ATTRIBUTE,
	&MEM_UNMAP,
	&MEM_SET_ACCESS,
	&MEM_RELEASE,
	&MEM_MAP,
	&MEM_GET_ALLOCATION_GRANULARITY,
	&MEM_CREATE,
	&MEM_ADDRESS_RESERVE,
	&MEM_ADDRESS_FREE,
	&HIPBLAS,
	&HOST_REGISTER,
	&HOST_UNREGISTER,
	&MANAGED_MALLOC,
	&MEM_ADVISE,
	&HOST_GET_DEVICE_POINTER,
];

pub fn snapshot() -> [u64; N] {
	let mut s = [0u64; N];
	for i in 0..N {
		s[i] = ALL[i].load(Ordering::Relaxed);
	}
	s
}

pub fn report() -> String {
	report_since(&[0u64; N])
}

pub fn report_since(base: &[u64; N]) -> String {
	report_between(base, &snapshot())
}

struct CounterEntry<'a> {
	n: u64,
	name: &'a str,
}

struct CounterGroup<'a> {
	group: &'a str,
	entries: &'a [CounterEntry<'a>],
}

pub fn report_between(base: &[u64; N], end: &[u64; N]) -> String {
	let g = |c: &AtomicU64| {
		let i = ALL
			.iter()
			.position(|x| std::ptr::eq(*x, c))
			.expect("counter registered in ALL");
		end[i].saturating_sub(base[i])
	};
	let groups: &[CounterGroup] = &[
		CounterGroup {
			group: "sync",
			entries: &[
				CounterEntry {
					n: g(&HOST_MALLOC),
					name: "allocations",
				},
				CounterEntry {
					n: g(&HOST_FREE),
					name: "frees",
				},
			],
		},
		CounterGroup {
			group: "async",
			entries: &[
				CounterEntry {
					n: g(&MEMCPY_ASYNC),
					name: "transfers",
				},
				CounterEntry {
					n: g(&MALLOC_ASYNC),
					name: "allocations",
				},
				CounterEntry {
					n: g(&MEMSET_ASYNC),
					name: "memsets",
				},
				CounterEntry {
					n: g(&FREE_ASYNC),
					name: "frees",
				},
			],
		},
		CounterGroup {
			group: "kernel launch",
			entries: &[CounterEntry {
				n: g(&LAUNCH),
				name: "hipLaunchKernelGGL",
			}],
		},
		CounterGroup {
			group: "reporting",
			entries: &[
				CounterEntry {
					n: g(&GET_LAST_ERROR),
					name: "hipGetLastError",
				},
				CounterEntry {
					n: g(&PEEK_AT_LAST_ERROR),
					name: "hipPeekAtLastError",
				},
				CounterEntry {
					n: g(&GET_ERROR_STRING),
					name: "hipGetErrorString",
				},
				CounterEntry {
					n: g(&GET_ERROR_NAME),
					name: "hipGetErrorName",
				},
				CounterEntry {
					n: g(&EVENT_RECORD),
					name: "hipEventRecord",
				},
				CounterEntry {
					n: g(&EVENT_ELAPSED_TIME),
					name: "hipEventElapsedTime",
				},
				CounterEntry {
					n: g(&EVENT_DESTROY),
					name: "hipEventDestroy",
				},
				CounterEntry {
					n: g(&EVENT_CREATE),
					name: "hipEventCreate",
				},
			],
		},
		CounterGroup {
			group: "syncs",
			entries: &[
				CounterEntry {
					n: g(&STREAM_SYNCHRONIZE),
					name: "hipStreamSynchronize",
				},
				CounterEntry {
					n: g(&DEVICE_SYNCHRONIZE),
					name: "hipDeviceSynchronize",
				},
				CounterEntry {
					n: g(&EVENT_SYNCHRONIZE),
					name: "hipEventSynchronize",
				},
			],
		},
		CounterGroup {
			group: "streams",
			entries: &[
				CounterEntry {
					n: g(&STREAM_DESTROY),
					name: "hipStreamDestroy",
				},
				CounterEntry {
					n: g(&STREAM_CREATE),
					name: "hipStreamCreate",
				},
			],
		},
		CounterGroup {
			group: "device/settings",
			entries: &[
				CounterEntry {
					n: g(&MEM_GET_INFO),
					name: "hipMemGetInfo",
				},
				CounterEntry {
					n: g(&SET_DEVICE),
					name: "hipSetDevice",
				},
				CounterEntry {
					n: g(&GET_DEVICE_COUNT),
					name: "hipGetDeviceCount",
				},
				CounterEntry {
					n: g(&DEVICE_GET_ATTRIBUTE),
					name: "hipDeviceGetAttribute",
				},
				CounterEntry {
					n: g(&DEVICE_ENABLE_PEER_ACCESS),
					name: "hipDeviceEnablePeerAccess",
				},
				CounterEntry {
					n: g(&DEVICE_CAN_ACCESS_PEER),
					name: "hipDeviceCanAccessPeer",
				},
				CounterEntry {
					n: g(&GET_DEVICE_PROPERTIES),
					name: "hipGetDeviceProperties",
				},
				CounterEntry {
					n: g(&GET_DEVICE),
					name: "hipGetDevice",
				},
			],
		},
		CounterGroup {
			group: "pool",
			entries: &[
				CounterEntry {
					n: g(&GET_DEFAULT_MEMPOOL),
					name: "hipDeviceGetDefaultMemPool",
				},
				CounterEntry {
					n: g(&MEMPOOL_GET_ATTRIBUTE),
					name: "hipMemPoolGetAttribute",
				},
				CounterEntry {
					n: g(&MEMPOOL_TRIM_TO),
					name: "hipMemPoolTrimTo",
				},
				CounterEntry {
					n: g(&MEMPOOL_SET_ATTRIBUTE),
					name: "hipMemPoolSetAttribute",
				},
			],
		},
		CounterGroup {
			group: "VMM",
			entries: &[
				CounterEntry {
					n: g(&MEM_UNMAP),
					name: "hipMemUnmap",
				},
				CounterEntry {
					n: g(&MEM_SET_ACCESS),
					name: "hipMemSetAccess",
				},
				CounterEntry {
					n: g(&MEM_RELEASE),
					name: "hipMemRelease",
				},
				CounterEntry {
					n: g(&MEM_MAP),
					name: "hipMemMap",
				},
				CounterEntry {
					n: g(&MEM_GET_ALLOCATION_GRANULARITY),
					name: "hipMemGetAllocationGranularity",
				},
				CounterEntry {
					n: g(&MEM_CREATE),
					name: "hipMemCreate",
				},
				CounterEntry {
					n: g(&MEM_ADDRESS_RESERVE),
					name: "hipMemAddressReserve",
				},
				CounterEntry {
					n: g(&MEM_ADDRESS_FREE),
					name: "hipMemAddressFree",
				},
			],
		},
		CounterGroup {
			group: "other",
			entries: &[CounterEntry {
				n: g(&HIPBLAS),
				name: "hipBLAS",
			}],
		},
	];
	let mut out = String::new();
	for grp in groups {
		for _present in grp.entries.iter().find(|e| e.n != 0).into_iter() {
			out.push_str(grp.group);
			out.push('\n');
		}
		let body: String = grp
			.entries
			.iter()
			.filter(|e| e.n != 0)
			.map(|e| format!("{n:>13} {name}\n", n = e.n, name = e.name))
			.collect();
		out.push_str(&body);
	}
	out
}

static LOOP_START: std::sync::Mutex<Option<[u64; N]>> = std::sync::Mutex::new(None);
static LOOP_END: std::sync::Mutex<Option<[u64; N]>> = std::sync::Mutex::new(None);

pub fn mark_loop_start() {
	*LOOP_START.lock().unwrap_or_else(|p| p.into_inner()) = Some(snapshot());
}

pub fn mark_loop_end() {
	*LOOP_END.lock().unwrap_or_else(|p| p.into_inner()) = Some(snapshot());
}

struct Phase<'a> {
	name: &'a str,
	a: &'a [u64; N],
	b: &'a [u64; N],
}

struct Tail<'a> {
	what: &'a str,
	v: [u64; 3],
}

pub fn state_report(run_start: &[u64; N]) -> Option<(String, Vec<String>)> {
	let ls = LOOP_START
		.lock()
		.unwrap_or_else(|p| p.into_inner())
		.take()?;
	let le = LOOP_END.lock().unwrap_or_else(|p| p.into_inner()).take()?;
	let end = snapshot();
	let idx = |c: &AtomicU64| {
		ALL.iter()
			.position(|x| std::ptr::eq(*x, c))
			.expect("counter registered in ALL")
	};
	let cell = |a: &[u64; N], b: &[u64; N], cs: &[&AtomicU64]| -> u64 {
		cs.iter().map(|c| b[idx(c)].saturating_sub(a[idx(c)])).sum()
	};
	let phases: [Phase; 3] = [
		Phase {
			name: "init",
			a: run_start,
			b: &ls,
		},
		Phase {
			name: "loop",
			a: &ls,
			b: &le,
		},
		Phase {
			name: "exit",
			a: &le,
			b: &end,
		},
	];
	let mut out = String::new();
	let mut hipblas = [0u64; 3];
	let mut frees = [0u64; 3];
	for i in 0..phases.len() {
		let ph = &phases[i];
		let name = ph.name;
		let a = ph.a;
		let b = ph.b;
		out.push_str(&format!("{name}\n    calcs\n"));
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"in-place",
			format!("{}x", cell(a, b, &[&LAUNCH]))
		));
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"alloc",
			format!(
				"{}x",
				cell(
					a,
					b,
					&[
						&HOST_MALLOC,
						&MALLOC_ASYNC,
						&MEM_CREATE,
						&MANAGED_MALLOC,
						&HOST_REGISTER
					]
				)
			)
		));
		out.push_str("    transfers\n");
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"async",
			format!(
				"{}x",
				cell(a, b, &[&XFER_ASYNC, &MEM_ADVISE, &HOST_GET_DEVICE_POINTER])
			)
		));
		out.push_str(&format!(
			"        {:<8}{:>7}\n",
			"sync",
			format!(
				"{}x",
				cell(
					a,
					b,
					&[&STREAM_SYNCHRONIZE, &DEVICE_SYNCHRONIZE, &EVENT_SYNCHRONIZE]
				)
			)
		));
		hipblas[i] = cell(a, b, &[&HIPBLAS]);
		frees[i] = cell(
			a,
			b,
			&[&HOST_FREE, &FREE_ASYNC, &MEM_RELEASE, &HOST_UNREGISTER],
		);
	}
	let tails: [Tail; 2] = [
		Tail {
			what: "hipBLAS",
			v: hipblas,
		},
		Tail {
			what: "free",
			v: frees,
		},
	];
	let mut errs: Vec<String> = Vec::new();
	for t in tails {
		for _present in t.v.iter().find(|x| **x != 0).into_iter() {
			errs.push(format!(
				"{what} (no spec cell)  init {i0}x  loop {i1}x  exit {i2}x",
				what = t.what,
				i0 = t.v[0],
				i1 = t.v[1],
				i2 = t.v[2]
			));
		}
	}
	Some((out.trim_end().to_string(), errs))
}

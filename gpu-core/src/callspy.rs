use ogdl::log::Write;
use core::ptr;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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

#[inline]
pub fn snapshot() -> [u64; N] {
	let mut s = [0u64; N];
	for (slot, cell) in s.iter_mut().zip(ALL.iter().copied()) {
		*slot = cell.load(Ordering::Relaxed);
	}
	return s;
}

#[inline]
#[must_use]
pub fn report() -> ogdl::Node {
	return report_since(&[0u64; N]);
}

#[inline]
#[must_use]
pub fn report_since(base: &[u64; N]) -> ogdl::Node {
	return report_between(base, &snapshot());
}

fn kv(name: &str, val: String) -> ogdl::Node {
	return ogdl::Node::new(name.to_owned(), vec![ogdl::Node::new(val, Vec::new())]);
}

struct CounterEntry {
	counter: &'static AtomicU64,
	name: &'static str,
}

struct CounterGroup {
	group: &'static str,
	entries: &'static [CounterEntry],
}

static GROUPS: &[CounterGroup] = &[
	CounterGroup {
		group: "sync",
		entries: &[
			CounterEntry {
				counter: &HOST_MALLOC,
				name: "allocations",
			},
			CounterEntry {
				counter: &HOST_FREE,
				name: "frees",
			},
		],
	},
	CounterGroup {
		group: "async",
		entries: &[
			CounterEntry {
				counter: &MEMCPY_ASYNC,
				name: "transfers",
			},
			CounterEntry {
				counter: &MALLOC_ASYNC,
				name: "allocations",
			},
			CounterEntry {
				counter: &MEMSET_ASYNC,
				name: "memsets",
			},
			CounterEntry {
				counter: &FREE_ASYNC,
				name: "frees",
			},
		],
	},
	CounterGroup {
		group: "kernel launch",
		entries: &[CounterEntry {
			counter: &LAUNCH,
			name: "hipLaunchKernelGGL",
		}],
	},
	CounterGroup {
		group: "reporting",
		entries: &[
			CounterEntry {
				counter: &GET_LAST_ERROR,
				name: "hipGetLastError",
			},
			CounterEntry {
				counter: &PEEK_AT_LAST_ERROR,
				name: "hipPeekAtLastError",
			},
			CounterEntry {
				counter: &GET_ERROR_STRING,
				name: "hipGetErrorString",
			},
			CounterEntry {
				counter: &GET_ERROR_NAME,
				name: "hipGetErrorName",
			},
			CounterEntry {
				counter: &EVENT_RECORD,
				name: "hipEventRecord",
			},
			CounterEntry {
				counter: &EVENT_ELAPSED_TIME,
				name: "hipEventElapsedTime",
			},
			CounterEntry {
				counter: &EVENT_DESTROY,
				name: "hipEventDestroy",
			},
			CounterEntry {
				counter: &EVENT_CREATE,
				name: "hipEventCreate",
			},
		],
	},
	CounterGroup {
		group: "syncs",
		entries: &[
			CounterEntry {
				counter: &STREAM_SYNCHRONIZE,
				name: "hipStreamSynchronize",
			},
			CounterEntry {
				counter: &DEVICE_SYNCHRONIZE,
				name: "hipDeviceSynchronize",
			},
			CounterEntry {
				counter: &EVENT_SYNCHRONIZE,
				name: "hipEventSynchronize",
			},
		],
	},
	CounterGroup {
		group: "streams",
		entries: &[
			CounterEntry {
				counter: &STREAM_DESTROY,
				name: "hipStreamDestroy",
			},
			CounterEntry {
				counter: &STREAM_CREATE,
				name: "hipStreamCreate",
			},
		],
	},
	CounterGroup {
		group: "device/settings",
		entries: &[
			CounterEntry {
				counter: &MEM_GET_INFO,
				name: "hipMemGetInfo",
			},
			CounterEntry {
				counter: &SET_DEVICE,
				name: "hipSetDevice",
			},
			CounterEntry {
				counter: &GET_DEVICE_COUNT,
				name: "hipGetDeviceCount",
			},
			CounterEntry {
				counter: &DEVICE_GET_ATTRIBUTE,
				name: "hipDeviceGetAttribute",
			},
			CounterEntry {
				counter: &DEVICE_ENABLE_PEER_ACCESS,
				name: "hipDeviceEnablePeerAccess",
			},
			CounterEntry {
				counter: &DEVICE_CAN_ACCESS_PEER,
				name: "hipDeviceCanAccessPeer",
			},
			CounterEntry {
				counter: &GET_DEVICE_PROPERTIES,
				name: "hipGetDeviceProperties",
			},
			CounterEntry {
				counter: &GET_DEVICE,
				name: "hipGetDevice",
			},
		],
	},
	CounterGroup {
		group: "pool",
		entries: &[
			CounterEntry {
				counter: &GET_DEFAULT_MEMPOOL,
				name: "hipDeviceGetDefaultMemPool",
			},
			CounterEntry {
				counter: &MEMPOOL_GET_ATTRIBUTE,
				name: "hipMemPoolGetAttribute",
			},
			CounterEntry {
				counter: &MEMPOOL_TRIM_TO,
				name: "hipMemPoolTrimTo",
			},
			CounterEntry {
				counter: &MEMPOOL_SET_ATTRIBUTE,
				name: "hipMemPoolSetAttribute",
			},
		],
	},
	CounterGroup {
		group: "VMM",
		entries: &[
			CounterEntry {
				counter: &MEM_UNMAP,
				name: "hipMemUnmap",
			},
			CounterEntry {
				counter: &MEM_SET_ACCESS,
				name: "hipMemSetAccess",
			},
			CounterEntry {
				counter: &MEM_RELEASE,
				name: "hipMemRelease",
			},
			CounterEntry {
				counter: &MEM_MAP,
				name: "hipMemMap",
			},
			CounterEntry {
				counter: &MEM_GET_ALLOCATION_GRANULARITY,
				name: "hipMemGetAllocationGranularity",
			},
			CounterEntry {
				counter: &MEM_CREATE,
				name: "hipMemCreate",
			},
			CounterEntry {
				counter: &MEM_ADDRESS_RESERVE,
				name: "hipMemAddressReserve",
			},
			CounterEntry {
				counter: &MEM_ADDRESS_FREE,
				name: "hipMemAddressFree",
			},
		],
	},
	CounterGroup {
		group: "other",
		entries: &[CounterEntry {
			counter: &HIPBLAS,
			name: "hipBLAS",
		}],
	},
];

fn index_of(c: &AtomicU64) -> usize {
	return ALL
		.iter()
		.position(|x| return ptr::eq(*x, c))
		.unwrap_or_else(|| {
			Write::error("callspy: counter not registered in ALL");
				return 0;
		});
}

#[inline]
#[must_use]
pub fn report_between(base: &[u64; N], end: &[u64; N]) -> ogdl::Node {
	let g = |c: &AtomicU64| {
		let i = index_of(c);
		return end[i].saturating_sub(base[i]);
	};
	let mut root = ogdl::Node::default();
	for grp in GROUPS {
		let kids: Vec<ogdl::Node> = grp
			.entries
			.iter()
			.filter(|e| return g(e.counter) != 0)
			.map(|e| return kv(e.name, g(e.counter).to_string()))
			.collect();
		if !kids.is_empty() {
			root.children
				.push(ogdl::Node::new(grp.group.to_owned(), kids));
		}
	}
	return root;
}

static LOOP_START: Mutex<Option<[u64; N]>> = Mutex::new(None);
static LOOP_END: Mutex<Option<[u64; N]>> = Mutex::new(None);

#[inline]
pub fn mark_loop_start() {
	*LOOP_START.lock().unwrap_or_else(|p| return p.into_inner()) = Some(snapshot());
}

#[inline]
pub fn mark_loop_end() {
	*LOOP_END.lock().unwrap_or_else(|p| return p.into_inner()) = Some(snapshot());
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

struct Sub {
	label: &'static str,
	counters: &'static [&'static AtomicU64],
}

struct Cat {
	label: &'static str,
	subs: &'static [Sub],
}

static CATS: &[Cat] = &[
	Cat {
		label: "calcs",
		subs: &[
			Sub {
				label: "in-place",
				counters: &[&LAUNCH],
			},
			Sub {
				label: "alloc",
				counters: &[
					&HOST_MALLOC,
					&MALLOC_ASYNC,
					&MEM_CREATE,
					&MANAGED_MALLOC,
					&HOST_REGISTER,
				],
			},
		],
	},
	Cat {
		label: "transfers",
		subs: &[
			Sub {
				label: "async",
				counters: &[&XFER_ASYNC, &MEM_ADVISE, &HOST_GET_DEVICE_POINTER],
			},
			Sub {
				label: "sync",
				counters: &[&STREAM_SYNCHRONIZE, &DEVICE_SYNCHRONIZE, &EVENT_SYNCHRONIZE],
			},
		],
	},
];

#[inline]
pub fn state_report(run_start: &[u64; N]) -> Option<(ogdl::Node, Vec<String>)> {
	let ls = LOOP_START
		.lock()
		.unwrap_or_else(|p| return p.into_inner())
		.take()?;
	let le = LOOP_END
		.lock()
		.unwrap_or_else(|p| return p.into_inner())
		.take()?;
	let end = snapshot();
	let cell = |a: &[u64; N], b: &[u64; N], cs: &[&AtomicU64]| -> u64 {
		return cs
			.iter()
			.map(|c| return b[index_of(c)].saturating_sub(a[index_of(c)]))
			.sum();
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
	let mut tree = ogdl::Node::default();
	let mut hipblas = [0u64; 3];
	let mut frees = [0u64; 3];
	for (i, ph) in phases.iter().enumerate() {
		let a = ph.a;
		let b = ph.b;
		let cats: Vec<ogdl::Node> = CATS
			.iter()
			.map(|cat| {
				let subs: Vec<ogdl::Node> = cat
					.subs
					.iter()
					.map(|s| return kv(s.label, format!("{}x", cell(a, b, s.counters))))
					.collect();
				return ogdl::Node::new(cat.label.to_owned(), subs);
			})
			.collect();
		tree.children
			.push(ogdl::Node::new(ph.name.to_owned(), cats));
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
		if t.v.iter().any(|x| return *x != 0) {
			errs.push(format!(
				"{what} (no spec cell)  init {i0}x  loop {i1}x  exit {i2}x",
				what = t.what,
				i0 = t.v[0],
				i1 = t.v[1],
				i2 = t.v[2]
			));
		}
	}
	return Some((tree, errs));
}

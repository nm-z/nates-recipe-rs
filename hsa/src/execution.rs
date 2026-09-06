use core::{
	cell::{Cell, RefCell},
	ffi::c_void,
	fmt,
	marker::PhantomData,
	mem::MaybeUninit,
	ptr::NonNull,
};
use std::{
	ffi::CString,
	num::NonZeroU32,
	rc::{Rc, Weak},
	sync::{
		Arc,
		atomic::{AtomicU16, Ordering},
	},
	time::{Duration, Instant},
};

use write::err;

use crate::{
	Error, MemoryPoolFlags, MemorySegment, Queue, QueueKind, Result, Runtime, Session,
	abi::*,
	discovery::{AgentDescription, DiscoveredAgent, MemoryPoolDescription, RawPool},
	identity::c_bool,
	loader::Api,
	session::{QueueCore, SharedFault},
};

#[derive(Clone)]
pub(crate) struct SignalPool {
	inner: Rc<SignalPoolInner>,
}

struct SignalPoolInner {
	api: Arc<Api>,
	fault: Arc<SharedFault>,
	timestamp_frequency_hz: u64,
	state: RefCell<SignalPoolState>,
}

#[derive(Default)]
struct SignalPoolState {
	available: Vec<HsaSignal>,
	retired: Vec<RetiredSignal>,
	retirement_reservations: usize,
	fatal_signal: Option<i64>,
}

struct RetiredResource<S, K> {
	signal: S,
	keepalive: K,
}

type RetiredSignal = RetiredResource<Rc<SignalRecord>, PendingKeepalive>;

struct SignalRecord {
	pool: Weak<SignalPoolInner>,
	api: Arc<Api>,
	signal: HsaSignal,
	terminal: Cell<Option<i64>>,
	retirement_reserved: Cell<bool>,
}

#[derive(Default)]
struct PendingKeepalive {
	_queue: Option<Rc<QueueCore>>,
	_executable: Option<Rc<ExecutableInner>>,
	_allocations: Vec<Rc<AllocationInner>>,
	_dependencies: Vec<Rc<SignalRecord>>,
}

impl PendingKeepalive {
	/// Release every device resource after system-scope terminal completion
	/// while retaining the preallocated vector storage needed to rearm a loop
	/// token without allocating.
	fn release_resources(&mut self) {
		self._queue = None;
		self._executable = None;
		self._allocations.clear();
		self._dependencies.clear();
	}
}

/// Result of one explicit deferred-retirement progress pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetirementReport {
	/// Records reclaimed during this pass.
	pub reclaimed: usize,
	/// Records still awaiting a positive completion signal.
	pub pending: usize,
	/// The first negative completion value observed by this session.
	pub failed_signal: Option<i64>,
	/// Whether an asynchronous queue callback has permanently poisoned the
	/// session.
	pub poisoned: bool,
}

impl RetirementReport {
	pub const fn is_drained(self) -> bool { self.pending == 0 }
}

impl SignalRecord {
	fn raw(&self) -> HsaSignal { self.signal }

	fn mark_terminal(&self, value: i64) {
		debug_assert!(value <= 0);
		self.terminal.set(Some(value));
	}

	fn release_retirement_reservation(&self) {
		if !self.retirement_reserved.replace(false) {
			return;
		}
		if let Some(pool) = self.pool.upgrade() {
			let mut state = pool.state.borrow_mut();
			debug_assert!(state.retirement_reservations > 0);
			state.retirement_reservations -= 1;
		}
	}
}

impl Drop for SignalRecord {
	fn drop(&mut self) {
		self.release_retirement_reservation();
		let value = self.terminal.get().unwrap_or_else(|| {
			// SAFETY: this record owns the signal until this destructor returns.
			unsafe { (self.api.signal_load_scacquire)(self.signal) }
		});
		if value > 0 {
			// This is an invariant violation: unfinished records must be owned by
			// the explicit retirement set. Do not destroy a signal the device can
			// still reference, and make the terminal leak visible.
			drop(err(format!(
				"recipe-hsa: terminal resource leak: completion signal {} remained positive outside the deferred-retirement set",
				self.signal.handle
			)));
			return;
		}
		if value < 0 {
			// SAFETY: a negative completion is terminal and cannot later be
			// decremented by the submitted operation.
			unsafe {
				(self.api.signal_destroy)(self.signal);
			}
			return;
		}

		if let Some(pool) = self.pool.upgrade() {
			let mut state = pool.state.borrow_mut();
			if state.available.try_reserve(1).is_ok() {
				// SAFETY: zero was observed with acquire semantics and no owner
				// retains a device-visible reference to this signal.
				unsafe {
					(self.api.signal_store_screlease)(self.signal, 1);
				}
				state.available.push(self.signal);
				return;
			}
		}
		// SAFETY: zero is terminal. Destruction is the allocation-failure and
		// pool-shutdown fallback; it never leaks a completed signal.
		unsafe {
			(self.api.signal_destroy)(self.signal);
		}
	}
}

fn release_retired<S, K>(retired: RetiredResource<S, K>) {
	// Resources referenced by the operation are released before the completion
	// signal can be recycled or destroyed.
	drop(retired.keepalive);
	drop(retired.signal);
}

impl SignalPool {
	pub(crate) fn new(api: Arc<Api>, fault: Arc<SharedFault>, timestamp_frequency_hz: u64) -> Self {
		Self {
			inner: Rc::new(SignalPoolInner {
				api,
				fault,
				timestamp_frequency_hz,
				state: RefCell::new(SignalPoolState::default()),
			}),
		}
	}

	fn acquire(&self) -> Result<Rc<SignalRecord>> {
		let _ = self.collect_retired();
		let mut state = self.inner.state.borrow_mut();
		if let Some(value) = state.fatal_signal {
			return Err(Error::AsyncSignal { value });
		}
		let required = state
			.retired
			.len()
			.checked_add(state.retirement_reservations)
			.and_then(|count| count.checked_add(1))
			.ok_or(Error::AllocationFailed {
				field: "deferred HSA retirement set",
				requested: usize::MAX,
			})?;
		if required > state.retired.capacity() {
			let additional = required - state.retired.len();
			state.retired.try_reserve(additional).map_err(|_| {
				Error::AllocationFailed {
					field: "deferred HSA retirement set",
					requested: required,
				}
			})?;
		}
		state.retirement_reservations += 1;
		let available = state.available.pop();
		drop(state);

		let signal = if let Some(signal) = available {
			signal
		} else {
			let mut signal = HsaSignal { handle: 0 };
			// SAFETY: output storage is valid. A zero consumer count permits any
			// agent to wait and makes the null consumer pointer valid.
			let status = unsafe { (self.inner.api.signal_create)(1, 0, std::ptr::null(), &mut signal) };
			if let Err(error) = self.inner.api.check("hsa_signal_create", status) {
				self.release_reservation();
				return Err(error);
			}
			if signal.handle == 0 {
				self.release_reservation();
				return Err(Error::NullSignal);
			}
			signal
		};
		Ok(Rc::new(SignalRecord {
			pool: Rc::downgrade(&self.inner),
			api: Arc::clone(&self.inner.api),
			signal,
			terminal: Cell::new(None),
			retirement_reserved: Cell::new(true),
		}))
	}

	fn release_reservation(&self) {
		let mut state = self.inner.state.borrow_mut();
		debug_assert!(state.retirement_reservations > 0);
		state.retirement_reservations -= 1;
	}

	fn rearm(&self, signal: &SignalRecord) -> Result<()> {
		let mut state = self.inner.state.borrow_mut();
		if let Some(value) = state.fatal_signal {
			return Err(Error::AsyncSignal { value });
		}
		let required = state
			.retired
			.len()
			.checked_add(state.retirement_reservations)
			.and_then(|count| count.checked_add(1))
			.ok_or(Error::AllocationFailed {
				field: "deferred HSA retirement set",
				requested: usize::MAX,
			})?;
		if required > state.retired.capacity() {
			let additional = required - state.retired.len();
			state.retired.try_reserve(additional).map_err(|_| {
				Error::AllocationFailed {
					field: "deferred HSA retirement set",
					requested: required,
				}
			})?;
		}
		state.retirement_reservations += 1;
		debug_assert!(!signal.retirement_reserved.replace(true));
		signal.terminal.set(None);
		// SAFETY: terminal completion was observed, this record is the sole
		// submission owner, and ROCr permits restoring a reusable signal to one.
		unsafe {
			(self.inner.api.signal_store_screlease)(signal.raw(), 1);
		}
		Ok(())
	}

	fn collect_retired(&self) -> RetirementReport {
		let mut reclaimed = 0;
		loop {
			let completed = {
				let mut state = self.inner.state.borrow_mut();
				let terminal = state
					.retired
					.iter()
					.enumerate()
					.find_map(|(index, retired)| {
						// SAFETY: every retired record and signal remain owned by the
						// pool until the record is removed below.
						let value = unsafe { (self.inner.api.signal_load_scacquire)(retired.signal.raw()) };
						(value <= 0).then_some((index, value))
					});
				terminal.map(|(index, value)| {
					let retired = state.retired.remove(index);
					retired.signal.mark_terminal(value);
					if value < 0 {
						state.fatal_signal.get_or_insert(value);
					}
					retired
				})
			};
			let Some(completed) = completed else {
				break;
			};
			reclaimed += 1;
			release_retired(completed);
		}
		let state = self.inner.state.borrow();
		RetirementReport {
			reclaimed,
			pending: state.retired.len(),
			failed_signal: state.fatal_signal,
			poisoned: self.inner.fault.snapshot().is_some(),
		}
	}

	fn wait_one_retired(&self, timeout: Duration) {
		let signal = self
			.inner
			.state
			.borrow()
			.retired
			.first()
			.map(|retired| retired.signal.raw());
		let Some(signal) = signal else {
			return;
		};
		let ticks = duration_to_ticks(timeout, self.inner.timestamp_frequency_hz);
		// SAFETY: the explicit retirement set owns this signal throughout the
		// bounded wait. LT 1 observes completion or negative failure.
		unsafe {
			(self.inner.api.signal_wait_scacquire)(signal, SIGNAL_CONDITION_LT, 1, ticks, WAIT_STATE_ACTIVE);
		}
	}

	fn release_unsubmitted(&self, signal: Rc<SignalRecord>) {
		signal.mark_terminal(0);
		signal.release_retirement_reservation();
		drop(signal);
	}

	fn fail(&self, signal: &SignalRecord, value: i64) {
		signal.mark_terminal(value);
		signal.release_retirement_reservation();
		let mut state = self.inner.state.borrow_mut();
		state.fatal_signal.get_or_insert(value);
	}

	fn retire(&self, signal: Rc<SignalRecord>, keepalive: PendingKeepalive) {
		let mut state = self.inner.state.borrow_mut();
		debug_assert!(signal.retirement_reserved.replace(false));
		debug_assert!(state.retirement_reservations > 0);
		state.retirement_reservations -= 1;
		debug_assert!(state.retired.len() < state.retired.capacity());
		state.retired.push(RetiredResource { signal, keepalive });
	}
}

impl Drop for SignalPoolInner {
	fn drop(&mut self) {
		let state = self.state.get_mut();
		for signal in state.available.drain(..) {
			// SAFETY: available signals have no outstanding users.
			unsafe {
				(self.api.signal_destroy)(signal);
			}
		}
		let mut unresolved = 0;
		for retired in state.retired.drain(..) {
			// SAFETY: every retired signal remains valid while its keepalive is
			// present.
			let value = unsafe { (self.api.signal_load_scacquire)(retired.signal.raw()) };
			if value <= 0 {
				retired.signal.mark_terminal(value);
				release_retired(retired);
			} else {
				unresolved += 1;
				// Runtime shutdown may still be pending. Keep all references
				// alive instead of guessing that device work stopped.
				std::mem::forget(retired);
			}
		}
		if unresolved != 0 {
			drop(err(format!(
				"recipe-hsa: terminal resource leak: {unresolved} deferred HSA operation(s) remained unresolved; call Session::drain_retirements before dropping the session"
			)));
		}
	}
}

struct AllocationInner {
	api: Arc<Api>,
	pointer: Option<NonNull<c_void>>,
	size: usize,
	owner: HsaAgent,
	pool_index: usize,
	global_flags: Option<MemoryPoolFlags>,
	accessible_agents: RefCell<Vec<u64>>,
}

impl AllocationInner {
	fn destroy(&mut self) -> Result<()> {
		let Some(pointer) = self.pointer.take() else {
			return Ok(());
		};
		// SAFETY: this is the one matching free for the allocation pointer.
		let status = unsafe { (self.api.amd_memory_pool_free)(pointer.as_ptr()) };
		self.api.check("hsa_amd_memory_pool_free", status)
	}

	fn is_accessible_by(&self, agent: HsaAgent) -> bool { self.accessible_agents.borrow().contains(&agent.handle) }

	fn record_access_grant(&self, agents: &[HsaAgent]) { *self.accessible_agents.borrow_mut() = exact_access_handles(self.owner, agents); }
}

fn exact_access_handles(owner: HsaAgent, agents: &[HsaAgent]) -> Vec<u64> {
	let mut handles = Vec::with_capacity(agents.len().saturating_add(1));
	handles.push(owner.handle);
	for agent in agents {
		if !handles.contains(&agent.handle) {
			handles.push(agent.handle);
		}
	}
	handles
}

impl Drop for AllocationInner {
	fn drop(&mut self) { let _ = self.destroy(); }
}

/// One allocation from an exactly discovered ROCr memory pool.
pub struct Allocation<'runtime> {
	inner: Rc<AllocationInner>,
	_runtime: PhantomData<&'runtime Runtime>,
}

impl Allocation<'_> {
	fn inner(&self) -> &Rc<AllocationInner> { &self.inner }

	pub fn as_ptr(&self) -> *mut c_void {
		self.inner
			.pointer
			.expect("live allocation owns a pointer")
			.as_ptr()
	}

	pub fn len(&self) -> usize { self.inner.size }

	pub fn is_empty(&self) -> bool { self.len() == 0 }

	pub fn pool_index(&self) -> usize { self.inner.pool_index }

	/// Copy host bytes directly into an allocation known by the caller to be
	/// host-accessible and coherent.
	///
	/// # Safety
	///
	/// The selected ROCr pool must permit host writes for this allocation, and
	/// no device operation may concurrently read or write the byte range.
	pub unsafe fn copy_from_host(&self, offset: usize, source: &[u8]) -> Result<()> {
		check_range("destination", offset, source.len(), self.len())?;
		// SAFETY: the caller provides the host-accessibility/coherency guarantee;
		// bounds were checked above and the regions cannot overlap.
		unsafe {
			std::ptr::copy_nonoverlapping(
				source.as_ptr(),
				self.as_ptr().cast::<u8>().add(offset),
				source.len(),
			);
		}
		Ok(())
	}

	/// Copy bytes from an allocation known by the caller to be host-accessible.
	///
	/// # Safety
	///
	/// The selected ROCr pool must permit host reads for this allocation, and
	/// the producing device operation must have completed with system scope.
	pub unsafe fn copy_to_host(&self, offset: usize, destination: &mut [u8]) -> Result<()> {
		check_range("source", offset, destination.len(), self.len())?;
		// SAFETY: the caller provides the host-accessibility/coherency guarantee;
		// bounds were checked above and the regions cannot overlap.
		unsafe {
			std::ptr::copy_nonoverlapping(
				self.as_ptr().cast::<u8>().add(offset),
				destination.as_mut_ptr(),
				destination.len(),
			);
		}
		Ok(())
	}

	pub fn close(self) -> Result<()> {
		match Rc::try_unwrap(self.inner) {
			Ok(mut inner) => inner.destroy(),
			Err(inner) => {
				drop(inner);
				Err(Error::ResourceBusy {
					resource: "HSA memory allocation",
				})
			}
		}
	}
}

fn check_range(buffer: &'static str, offset: usize, size: usize, capacity: usize) -> Result<()> {
	if offset.checked_add(size).is_some_and(|end| end <= capacity) {
		Ok(())
	} else {
		Err(Error::CopyOutOfBounds {
			buffer,
			offset,
			size,
			capacity,
		})
	}
}

fn allocate_from<'runtime>(runtime: &'runtime Runtime, owner: HsaAgent, raw_pools: &[RawPool], descriptions: &[MemoryPoolDescription], pool_index: usize, size: usize) -> Result<Allocation<'runtime>> {
	runtime.ensure_active()?;
	let pool = raw_pools
		.get(pool_index)
		.ok_or(Error::InvalidMemoryPoolIndex {
			index: pool_index,
			pool_count: raw_pools.len(),
		})?;
	let description = descriptions
		.get(pool_index)
		.ok_or(Error::InvalidMemoryPoolIndex {
			index: pool_index,
			pool_count: descriptions.len(),
		})?;
	if description.runtime_allocation.is_none() {
		return Err(Error::MemoryPoolNotAllocatable { index: pool_index });
	}
	if size == 0
		|| description
			.maximum_aggregate_allocation_bytes
			.is_some_and(|maximum| size > maximum)
	{
		return Err(Error::InvalidAllocationSize {
			requested: size,
			maximum: description.maximum_aggregate_allocation_bytes,
		});
	}

	let mut pointer = std::ptr::null_mut();
	// SAFETY: the pool was obtained from ROCr and reported runtime allocation
	// support; flags zero is the standard allocation mode.
	let status = unsafe { (runtime.api.amd_memory_pool_allocate)(pool.handle, size, 0, &mut pointer) };
	runtime.api.check("hsa_amd_memory_pool_allocate", status)?;
	let pointer = NonNull::new(pointer).ok_or(Error::NullAllocation)?;
	Ok(Allocation {
		inner: Rc::new(AllocationInner {
			api: Arc::clone(&runtime.api),
			pointer: Some(pointer),
			size,
			owner,
			pool_index,
			global_flags: description.global_flags,
			accessible_agents: RefCell::new(vec![owner.handle]),
		}),
		_runtime: PhantomData,
	})
}

fn matching_pool(descriptions: &[MemoryPoolDescription], kind: &'static str, predicate: impl Fn(MemoryPoolFlags) -> bool) -> Result<usize> {
	descriptions
		.iter()
		.position(|pool| pool.segment == MemorySegment::Global && pool.runtime_allocation.is_some() && pool.global_flags.is_some_and(&predicate))
		.ok_or(Error::NoMatchingMemoryPool { kind })
}

fn grant_access(api: &Api, agents: &[HsaAgent], allocation: &Allocation<'_>) -> Result<()> {
	let count = u32::try_from(agents.len()).map_err(|_| {
		Error::InvalidDispatch {
			reason: "HSA access-grant agent count exceeds the public ABI",
		}
	})?;
	if count == 0 {
		return Err(Error::InvalidDispatch {
			reason: "HSA access-grant set must not be empty",
		});
	}
	// ROCr defines this as replacement, not accumulation: upon return only
	// these agents plus the pool owner retain direct access.
	// SAFETY: every agent was discovered from the active runtime; flags are
	// reserved and therefore null; the pointer came from its pool allocator.
	let status = unsafe {
		(api.amd_agents_allow_access)(
			count,
			agents.as_ptr(),
			std::ptr::null(),
			allocation.as_ptr().cast_const(),
		)
	};
	api.check("hsa_amd_agents_allow_access", status)?;
	allocation.inner.record_access_grant(agents);
	Ok(())
}

impl<'runtime> DiscoveredAgent<'runtime> {
	pub fn allocate(&self, pool_index: usize, size: usize) -> Result<Allocation<'runtime>> {
		allocate_from(
			self.runtime,
			self.raw_agent,
			&self.raw_pools,
			&self.description.memory_pools,
			pool_index,
			size,
		)
	}

	pub fn allocate_coarse(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "coarse-grained", |flags| {
			flags.contains(MemoryPoolFlags::COARSE_GRAINED)
		})?;
		self.allocate(index, size)
	}

	pub fn allocate_fine(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "fine-grained", |flags| {
			flags.contains(MemoryPoolFlags::FINE_GRAINED) || flags.contains(MemoryPoolFlags::EXTENDED_SCOPE_FINE_GRAINED)
		})?;
		self.allocate(index, size)
	}

	pub fn allocate_kernarg(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "kernarg", |flags| {
			flags.contains(MemoryPoolFlags::KERNARG_INITIALIZATION)
		})?;
		self.allocate(index, size)
	}

	pub fn grant_access(&self, allocation: &Allocation<'_>) -> Result<()> {
		self.runtime.ensure_active()?;
		grant_access(
			&self.runtime.api,
			std::slice::from_ref(&self.raw_agent),
			allocation,
		)
	}
}

impl<'runtime> Session<'runtime> {
	pub fn allocate(&self, pool_index: usize, size: usize) -> Result<Allocation<'runtime>> {
		allocate_from(
			self.runtime,
			self.raw_agent,
			&self.raw_pools,
			&self.description.memory_pools,
			pool_index,
			size,
		)
	}

	pub fn allocate_coarse(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "coarse-grained", |flags| {
			flags.contains(MemoryPoolFlags::COARSE_GRAINED)
		})?;
		self.allocate(index, size)
	}

	pub fn allocate_fine(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "fine-grained", |flags| {
			flags.contains(MemoryPoolFlags::FINE_GRAINED) || flags.contains(MemoryPoolFlags::EXTENDED_SCOPE_FINE_GRAINED)
		})?;
		self.allocate(index, size)
	}

	pub fn allocate_kernarg(&self, size: usize) -> Result<Allocation<'runtime>> {
		let index = matching_pool(&self.description.memory_pools, "kernarg", |flags| {
			flags.contains(MemoryPoolFlags::KERNARG_INITIALIZATION)
		})?;
		self.allocate(index, size)
	}

	pub fn grant_access(&self, allocation: &Allocation<'_>) -> Result<()> {
		self.runtime.ensure_active()?;
		grant_access(
			&self.runtime.api,
			std::slice::from_ref(&self.raw_agent),
			allocation,
		)
	}

	/// Replace an allocation's direct-access set with the exact discovered
	/// sessions and agents supplied here. ROCr always retains the pool owner.
	pub fn grant_access_exact_set(&self, allocation: &Allocation<'_>, sessions: &[&Session<'runtime>], agents: &[&DiscoveredAgent<'runtime>]) -> Result<()> {
		self.runtime.ensure_active()?;
		if !Arc::ptr_eq(&self.runtime.api, &allocation.inner.api)
			|| sessions
				.iter()
				.any(|session| !std::ptr::eq(self.runtime, session.runtime))
			|| agents
				.iter()
				.any(|agent| !std::ptr::eq(self.runtime, agent.runtime))
		{
			return Err(Error::InvalidDispatch {
				reason: "HSA access-grant objects belong to different runtimes",
			});
		}
		let mut exact = sessions
			.iter()
			.map(|session| session.raw_agent)
			.chain(agents.iter().map(|agent| agent.raw_agent))
			.collect::<Vec<_>>();
		exact.sort_by_key(|agent| agent.handle);
		exact.dedup_by_key(|agent| agent.handle);
		grant_access(&self.runtime.api, &exact, allocation)
	}

	/// Make one nonblocking pass over the explicit deferred-retirement set.
	pub fn poll_retirements(&self) -> RetirementReport { self.signals.collect_retired() }

	/// Drain deferred operations for at most `timeout`.
	///
	/// A poisoned session fails immediately while retaining every unresolved
	/// signal and allocation. A timeout likewise leaves the set intact so a
	/// later call can complete ordered teardown.
	pub fn drain_retirements(&self, timeout: Duration) -> Result<RetirementReport> {
		let start = Instant::now();
		loop {
			let report = self.poll_retirements();
			if report.is_drained() {
				if let Some(value) = report.failed_signal {
					return Err(Error::AsyncSignal { value });
				}
				return Ok(report);
			}
			if report.poisoned || start.elapsed() >= timeout {
				return Err(Error::DeferredRetirement {
					pending: report.pending,
					poisoned: report.poisoned,
				});
			}
			let remaining = timeout.saturating_sub(start.elapsed());
			self.signals
				.wait_one_retired(remaining.min(Duration::from_millis(1)));
		}
	}
}

struct ExecutableInner {
	api: Arc<Api>,
	hsaco: Option<Arc<[u8]>>,
	reader: Option<HsaCodeObjectReader>,
	executable: Option<HsaExecutable>,
	agent: HsaAgent,
}

impl ExecutableInner {
	fn destroy(&mut self) -> Result<()> {
		if let Some(executable) = self.executable.take() {
			// SAFETY: this is the executable handle's one matching destroy.
			let status = unsafe { (self.api.executable_destroy)(executable) };
			if let Err(error) = self.api.check("hsa_executable_destroy", status) {
				self.leak_reader_backing();
				return Err(error);
			}
		}
		if let Some(reader) = self.reader.take() {
			// SAFETY: the executable was destroyed first, satisfying the
			// reader's documented lifetime constraint.
			let status = unsafe { (self.api.code_object_reader_destroy)(reader) };
			if let Err(error) = self.api.check("hsa_code_object_reader_destroy", status) {
				if let Some(hsaco) = self.hsaco.take() {
					std::mem::forget(hsaco);
				}
				return Err(error);
			}
		}
		drop(self.hsaco.take());
		Ok(())
	}

	fn leak_reader_backing(&mut self) {
		self.reader.take();
		if let Some(hsaco) = self.hsaco.take() {
			std::mem::forget(hsaco);
		}
	}
}

impl Drop for ExecutableInner {
	fn drop(&mut self) { let _ = self.destroy(); }
}

pub struct Executable<'session, 'runtime> {
	inner: Rc<ExecutableInner>,
	_session: PhantomData<&'session Session<'runtime>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelMetadata {
	pub kernarg_segment_size: u32,
	pub kernarg_segment_alignment: u32,
	pub group_segment_size: u32,
	pub private_segment_size: u32,
	pub dynamic_callstack: bool,
}

pub struct Kernel<'session, 'runtime> {
	name: String,
	object: u64,
	metadata: KernelMetadata,
	executable: Rc<ExecutableInner>,
	_session: PhantomData<&'session Session<'runtime>>,
}

impl Kernel<'_, '_> {
	pub fn name(&self) -> &str { &self.name }

	pub fn metadata(&self) -> &KernelMetadata { &self.metadata }
}

impl<'session, 'runtime> Executable<'session, 'runtime> {
	pub fn kernel(&self, name: &str) -> Result<Kernel<'session, 'runtime>> {
		let name_c = CString::new(name).map_err(|_| {
			Error::NameContainsNul {
				field: "kernel symbol name",
			}
		})?;
		let executable = self
			.inner
			.executable
			.expect("live executable owns an HSA handle");
		let mut symbol = MaybeUninit::<HsaExecutableSymbol>::uninit();
		// SAFETY: the name is NUL terminated, executable/agent are live, and
		// output storage matches the public ABI.
		let status = unsafe {
			(self.inner.api.executable_get_symbol_by_name)(
				executable,
				name_c.as_ptr(),
				&self.inner.agent,
				symbol.as_mut_ptr(),
			)
		};
		self.inner
			.api
			.check("hsa_executable_get_symbol_by_name", status)?;
		// SAFETY: successful lookup initialized the complete symbol handle.
		let symbol = unsafe { symbol.assume_init() };
		let kind: i32 = executable_symbol_info(
			&self.inner.api,
			symbol,
			EXECUTABLE_SYMBOL_INFO_TYPE,
			"HSA executable symbol type",
		)?;
		if kind != SYMBOL_KIND_KERNEL {
			return Err(Error::SymbolNotKernel {
				name: name.to_owned(),
				kind,
			});
		}

		let object = executable_symbol_info(
			&self.inner.api,
			symbol,
			EXECUTABLE_SYMBOL_INFO_KERNEL_OBJECT,
			"HSA kernel object",
		)?;
		if object == 0 {
			return Err(Error::InvalidKernel {
				reason: "frozen executable returned a zero kernel object",
			});
		}
		let metadata = KernelMetadata {
			kernarg_segment_size: executable_symbol_info(
				&self.inner.api,
				symbol,
				EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_SIZE,
				"HSA kernel kernarg size",
			)?,
			kernarg_segment_alignment: executable_symbol_info(
				&self.inner.api,
				symbol,
				EXECUTABLE_SYMBOL_INFO_KERNEL_KERNARG_SEGMENT_ALIGNMENT,
				"HSA kernel kernarg alignment",
			)?,
			group_segment_size: executable_symbol_info(
				&self.inner.api,
				symbol,
				EXECUTABLE_SYMBOL_INFO_KERNEL_GROUP_SEGMENT_SIZE,
				"HSA kernel group segment size",
			)?,
			private_segment_size: executable_symbol_info(
				&self.inner.api,
				symbol,
				EXECUTABLE_SYMBOL_INFO_KERNEL_PRIVATE_SEGMENT_SIZE,
				"HSA kernel private segment size",
			)?,
			dynamic_callstack: c_bool(
				"HSA kernel dynamic callstack",
				executable_symbol_info(
					&self.inner.api,
					symbol,
					EXECUTABLE_SYMBOL_INFO_KERNEL_DYNAMIC_CALLSTACK,
					"HSA kernel dynamic callstack",
				)?,
			)?,
		};
		if metadata.kernarg_segment_alignment == 0 || !metadata.kernarg_segment_alignment.is_power_of_two() {
			return Err(Error::InvalidKernel {
				reason: "ROCr returned an invalid kernarg alignment",
			});
		}

		Ok(Kernel {
			name: name.to_owned(),
			object,
			metadata,
			executable: Rc::clone(&self.inner),
			_session: PhantomData,
		})
	}

	pub fn close(self) -> Result<()> {
		match Rc::try_unwrap(self.inner) {
			Ok(mut inner) => inner.destroy(),
			Err(inner) => {
				drop(inner);
				Err(Error::ResourceBusy {
					resource: "HSA executable",
				})
			}
		}
	}
}

fn executable_symbol_info<T: Copy>(api: &Api, symbol: HsaExecutableSymbol, attribute: HsaInfo, operation: &'static str) -> Result<T> {
	let mut value = MaybeUninit::<T>::uninit();
	// SAFETY: each internal call pairs a reviewed symbol attribute with its
	// public-header output type.
	let status = unsafe { (api.executable_symbol_get_info)(symbol, attribute, value.as_mut_ptr().cast::<c_void>()) };
	api.check(operation, status)?;
	// SAFETY: successful symbol queries initialize the complete value.
	Ok(unsafe { value.assume_init() })
}

impl<'runtime> Session<'runtime> {
	pub fn load_hsaco<'session>(&'session self, hsaco: &[u8]) -> Result<Executable<'session, 'runtime>> {
		self.runtime.ensure_active()?;
		self.ensure_healthy()?;
		if hsaco.is_empty() {
			return Err(Error::EmptyCodeObject);
		}
		let hsaco: Arc<[u8]> = Arc::from(hsaco);
		let mut inner = ExecutableInner {
			api: Arc::clone(&self.runtime.api),
			hsaco: Some(Arc::clone(&hsaco)),
			reader: None,
			executable: None,
			agent: self.raw_agent,
		};

		let mut reader = MaybeUninit::<HsaCodeObjectReader>::uninit();
		// SAFETY: the Arc-backed buffer is nonempty and remains alive in
		// `inner`; output storage matches the public ABI.
		let status = unsafe {
			(self.runtime.api.code_object_reader_create_from_memory)(
				hsaco.as_ptr().cast::<c_void>(),
				hsaco.len(),
				reader.as_mut_ptr(),
			)
		};
		self.runtime
			.api
			.check("hsa_code_object_reader_create_from_memory", status)?;
		// SAFETY: successful creation initialized the handle.
		inner.reader = Some(unsafe { reader.assume_init() });

		let mut executable = MaybeUninit::<HsaExecutable>::uninit();
		// SAFETY: profile and nearest-even mode are valid public enum values;
		// null options are permitted.
		let status = unsafe {
			(self.runtime.api.executable_create_alt)(
				self.description.profile.as_raw(),
				DEFAULT_FLOAT_ROUNDING_MODE_NEAR,
				std::ptr::null(),
				executable.as_mut_ptr(),
			)
		};
		if let Err(error) = self.runtime.api.check("hsa_executable_create_alt", status) {
			let _ = inner.destroy();
			return Err(error);
		}
		// SAFETY: successful creation initialized the handle.
		inner.executable = Some(unsafe { executable.assume_init() });

		let mut loaded = MaybeUninit::<HsaLoadedCodeObject>::uninit();
		// SAFETY: executable, reader, and agent are live and mutually scoped;
		// null options are permitted.
		let status = unsafe {
			(self.runtime.api.executable_load_agent_code_object)(
				inner.executable.expect("set above"),
				self.raw_agent,
				inner.reader.expect("set above"),
				std::ptr::null(),
				loaded.as_mut_ptr(),
			)
		};
		if let Err(error) = self
			.runtime
			.api
			.check("hsa_executable_load_agent_code_object", status)
		{
			let _ = inner.destroy();
			return Err(error);
		}

		// SAFETY: the executable is live and null options are permitted.
		let status = unsafe { (self.runtime.api.executable_freeze)(inner.executable.expect("set above"), std::ptr::null()) };
		if let Err(error) = self.runtime.api.check("hsa_executable_freeze", status) {
			let _ = inner.destroy();
			return Err(error);
		}
		self.ensure_healthy()?;

		Ok(Executable {
			inner: Rc::new(inner),
			_session: PhantomData,
		})
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DispatchGeometry {
	pub dimensions: u8,
	pub grid: [u32; 3],
	pub workgroup: [u16; 3],
	pub dynamic_group_bytes: u32,
	pub dynamic_private_bytes: u32,
	pub barrier: bool,
}

impl DispatchGeometry {
	pub const fn one_dimensional(grid: u32, workgroup: u16) -> Self {
		Self {
			dimensions: 1,
			grid: [grid, 1, 1],
			workgroup: [workgroup, 1, 1],
			dynamic_group_bytes: 0,
			dynamic_private_bytes: 0,
			barrier: false,
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollStatus {
	Pending,
	Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WaitOutcome {
	Complete,
	TimedOut,
}

enum SignalState {
	Pending,
	Complete,
	Failed(i64),
}

fn classify_signal(value: i64) -> SignalState {
	if value > 0 {
		SignalState::Pending
	} else if value == 0 {
		SignalState::Complete
	} else {
		SignalState::Failed(value)
	}
}

/// A clonable, session-scoped reference to a device-visible completion signal.
///
/// Clones keep the signal handle alive and non-reusable until every submitted
/// consumer that retained a clone has completed.
pub struct Dependency<'session, 'runtime> {
	pool: SignalPool,
	record: Rc<SignalRecord>,
	_session: PhantomData<&'session Session<'runtime>>,
}

impl Clone for Dependency<'_, '_> {
	fn clone(&self) -> Self {
		Self {
			pool: self.pool.clone(),
			record: Rc::clone(&self.record),
			_session: PhantomData,
		}
	}
}

impl Dependency<'_, '_> {
	/// Observe this dependency without consuming it or recycling its signal.
	pub fn poll(&self) -> Result<PollStatus> {
		let signal = self.record.raw();
		// SAFETY: this dependency owns a shared signal record.
		let value = unsafe { (self.pool.inner.api.signal_load_scacquire)(signal) };
		poll_dependency_value(value, &self.pool.inner.fault)
	}
}

fn poll_dependency_value(value: i64, fault: &SharedFault) -> Result<PollStatus> {
	match classify_signal(value) {
		SignalState::Pending => {
			fault.check()?;
			Ok(PollStatus::Pending)
		}
		SignalState::Complete => {
			fault.check()?;
			Ok(PollStatus::Complete)
		}
		SignalState::Failed(value) => Err(Error::AsyncSignal { value }),
	}
}

fn wait_decision(status: PollStatus, deadline_reached: bool) -> Option<WaitOutcome> {
	match (status, deadline_reached) {
		(PollStatus::Complete, _) => Some(WaitOutcome::Complete),
		(PollStatus::Pending, true) => Some(WaitOutcome::TimedOut),
		(PollStatus::Pending, false) => None,
	}
}

#[must_use = "dropping an incomplete token defers all referenced HSA resources"]
pub struct Pending<'session, 'runtime> {
	pool: SignalPool,
	signal: Option<Rc<SignalRecord>>,
	keepalive: Option<PendingKeepalive>,
	terminal: Option<Result<()>>,
	_session: PhantomData<&'session Session<'runtime>>,
}

impl<'session, 'runtime> Pending<'session, 'runtime> {
	/// Borrow this operation as a reusable dependency for later submissions.
	pub fn dependency(&self) -> Dependency<'session, 'runtime> {
		Dependency {
			pool: self.pool.clone(),
			record: Rc::clone(
				self.signal
					.as_ref()
					.expect("a pending token retains its completion record"),
			),
			_session: PhantomData,
		}
	}

	pub fn poll(&mut self) -> Result<PollStatus> {
		if let Some(terminal) = &self.terminal {
			return terminal.clone().map(|()| PollStatus::Complete);
		}
		let signal = self
			.signal
			.as_ref()
			.expect("nonterminal pending token has a signal");
		// SAFETY: the signal remains owned by this token.
		let value = unsafe { (self.pool.inner.api.signal_load_scacquire)(signal.raw()) };
		match classify_signal(value) {
			SignalState::Pending => {
				match self.pool.inner.fault.check() {
					Ok(()) => Ok(PollStatus::Pending),
					// A queue callback can poison the session before this operation's
					// completion signal reaches a terminal value. Keep the operation
					// nonterminal so Drop moves its signal and every referenced
					// resource into deferred retirement instead of releasing objects
					// the device may still reference.
					Err(error) => Err(error),
				}
			}
			SignalState::Complete => {
				signal.mark_terminal(0);
				signal.release_retirement_reservation();
				if let Some(keepalive) = &mut self.keepalive {
					keepalive.release_resources();
				}
				let terminal = self.pool.inner.fault.check();
				self.terminal = Some(terminal.clone());
				terminal.map(|()| PollStatus::Complete)
			}
			SignalState::Failed(value) => {
				self.pool.fail(signal, value);
				if let Some(keepalive) = &mut self.keepalive {
					keepalive.release_resources();
				}
				let error = Error::AsyncSignal { value };
				self.terminal = Some(Err(error.clone()));
				Err(error)
			}
		}
	}

	/// Wait for at most the requested host duration, checking a ROCr active-wait
	/// hint in bounded chunks. ROCr documents timeout values as hints, so a
	/// driver call may overshoot slightly; this method never requests an
	/// unbounded wait.
	pub fn wait(&mut self, timeout: Duration) -> Result<WaitOutcome> {
		let start = Instant::now();
		loop {
			let status = self.poll()?;
			let elapsed = start.elapsed();
			if let Some(outcome) = wait_decision(status, elapsed >= timeout) {
				return Ok(outcome);
			}
			let remaining = timeout.saturating_sub(elapsed);
			let chunk = remaining.min(Duration::from_millis(1));
			let ticks = duration_to_ticks(chunk, self.pool.inner.timestamp_frequency_hz);
			let signal = self
				.signal
				.as_ref()
				.expect("pending token still has a signal")
				.raw();
			// SAFETY: the signal remains live; LT 1 observes either completion
			// zero or a negative asynchronous failure.
			unsafe {
				(self.pool.inner.api.signal_wait_scacquire)(signal, SIGNAL_CONDITION_LT, 1, ticks, WAIT_STATE_ACTIVE);
			}
		}
	}
}

impl Drop for Pending<'_, '_> {
	fn drop(&mut self) {
		let signal = self.signal.take();
		let keepalive = self.keepalive.take();
		match (self.terminal.is_some(), signal, keepalive) {
			(false, Some(signal), Some(keepalive)) => self.pool.retire(signal, keepalive),
			(true, Some(signal), Some(keepalive)) => {
				drop(keepalive);
				drop(signal);
			}
			(_, remaining_signal, remaining_keepalive) => drop((remaining_keepalive, remaining_signal)),
		}
	}
}

enum PreparedPendingState<'session, 'runtime> {
	Ready {
		signal: Rc<SignalRecord>,
		keepalive: PendingKeepalive,
	},
	Active(Pending<'session, 'runtime>),
	Consumed,
}

/// A completion token whose signal and keepalive storage are fully realized
/// before run initialization.
///
/// Submission methods mutate this token in place. They never acquire a signal
/// or grow an owned collection, so the live loop cannot trigger ROCr object
/// creation or host allocation through this path.
#[must_use = "a prepared HSA token owns a completion signal until consumed"]
pub struct PreparedPending<'session, 'runtime> {
	pool: SignalPool,
	state: PreparedPendingState<'session, 'runtime>,
	_session: PhantomData<&'session Session<'runtime>>,
}

impl fmt::Debug for PreparedPending<'_, '_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let state = match &self.state {
			PreparedPendingState::Ready { .. } => "ready",
			PreparedPendingState::Active(_) => "active",
			PreparedPendingState::Consumed => "consumed",
		};
		formatter
			.debug_struct("PreparedPending")
			.field("state", &state)
			.finish_non_exhaustive()
	}
}

impl<'session, 'runtime> PreparedPending<'session, 'runtime> {
	fn begin(&mut self, required_allocations: usize, required_dependencies: usize) -> Result<(Rc<SignalRecord>, PendingKeepalive)> {
		let state = core::mem::replace(&mut self.state, PreparedPendingState::Consumed);
		let PreparedPendingState::Ready {
			signal,
			mut keepalive,
		} = state
		else {
			self.state = state;
			return Err(Error::ResourceBusy {
				resource: "preallocated HSA pending token",
			});
		};
		if keepalive._allocations.capacity() < required_allocations || keepalive._dependencies.capacity() < required_dependencies {
			self.state = PreparedPendingState::Ready { signal, keepalive };
			return Err(Error::InvalidDispatch {
				reason: "preallocated HSA pending token has insufficient keepalive capacity",
			});
		}
		keepalive._queue = None;
		keepalive._executable = None;
		keepalive._allocations.clear();
		keepalive._dependencies.clear();
		Ok((signal, keepalive))
	}

	fn restore_ready(&mut self, signal: Rc<SignalRecord>, mut keepalive: PendingKeepalive) {
		keepalive.release_resources();
		self.state = PreparedPendingState::Ready { signal, keepalive };
	}

	fn activate(&mut self, signal: Rc<SignalRecord>, keepalive: PendingKeepalive) {
		self.state = PreparedPendingState::Active(Pending {
			pool: self.pool.clone(),
			signal: Some(signal),
			keepalive: Some(keepalive),
			terminal: None,
			_session: PhantomData,
		});
	}

	fn retire_active(&mut self) {
		let state = core::mem::replace(&mut self.state, PreparedPendingState::Consumed);
		drop(state);
	}

	/// Observe the pre-realized operation without blocking.
	pub fn poll(&mut self) -> Result<PollStatus> {
		match &mut self.state {
			PreparedPendingState::Active(pending) => pending.poll(),
			PreparedPendingState::Ready { .. } => {
				Err(Error::InvalidDispatch {
					reason: "preallocated HSA pending token has not been submitted",
				})
			}
			PreparedPendingState::Consumed => {
				Err(Error::ResourceBusy {
					resource: "consumed HSA pending token",
				})
			}
		}
	}

	/// Restores a terminal prepared token for another pre-final warm pass.
	///
	/// The same ROCr signal and keepalive allocations are retained. Calling
	/// this before terminal completion fails without consuming the token.
	pub fn reset(&mut self) -> Result<()> {
		let state = core::mem::replace(&mut self.state, PreparedPendingState::Consumed);
		match state {
			PreparedPendingState::Ready { signal, keepalive } => {
				self.state = PreparedPendingState::Ready { signal, keepalive };
				Ok(())
			}
			PreparedPendingState::Active(mut pending) => {
				match pending.poll() {
					Ok(PollStatus::Pending) => {
						self.state = PreparedPendingState::Active(pending);
						Err(Error::ResourceBusy {
							resource: "nonterminal prepared HSA pending token",
						})
					}
					Ok(PollStatus::Complete) => {
						let signal = pending.signal.take().ok_or(Error::ResourceBusy {
							resource: "completed HSA pending signal",
						})?;
						let keepalive = pending.keepalive.take().ok_or(Error::ResourceBusy {
							resource: "completed HSA pending keepalive",
						})?;
						self.pool.rearm(&signal)?;
						self.restore_ready(signal, keepalive);
						Ok(())
					}
					Err(error) => {
						self.state = PreparedPendingState::Active(pending);
						Err(error)
					}
				}
			}
			PreparedPendingState::Consumed => {
				Err(Error::ResourceBusy {
					resource: "consumed HSA pending token",
				})
			}
		}
	}

	/// Borrow an active pre-realized operation as an AQL dependency.
	pub fn dependency(&self) -> Result<Dependency<'session, 'runtime>> {
		match &self.state {
			PreparedPendingState::Active(pending) => Ok(pending.dependency()),
			PreparedPendingState::Ready { .. } | PreparedPendingState::Consumed => {
				Err(Error::InvalidDispatch {
					reason: "only an active HSA pending token can be a dependency",
				})
			}
		}
	}
}

impl Drop for PreparedPending<'_, '_> {
	fn drop(&mut self) {
		let state = core::mem::replace(&mut self.state, PreparedPendingState::Consumed);
		match state {
			PreparedPendingState::Ready { signal, .. } => self.pool.release_unsubmitted(signal),
			PreparedPendingState::Active(pending) => drop(pending),
			PreparedPendingState::Consumed => {}
		}
	}
}

fn duration_to_ticks(duration: Duration, frequency_hz: u64) -> u64 {
	let ticks = duration.as_nanos().saturating_mul(u128::from(frequency_hz)) / 1_000_000_000;
	ticks.clamp(1, u128::from(u64::MAX)) as u64
}

impl<'runtime> Session<'runtime> {
	/// Realize one signal and fixed keepalive vectors before `init`.
	pub fn prepare_pending<'session>(&'session self, allocation_capacity: usize, dependency_capacity: usize) -> Result<PreparedPending<'session, 'runtime>> {
		self.runtime.ensure_active()?;
		self.ensure_healthy()?;
		let signal = self.signals.acquire()?;
		let mut allocations = Vec::new();
		if let Err(_error) = allocations.try_reserve_exact(allocation_capacity) {
			self.signals.release_unsubmitted(signal);
			return Err(Error::AllocationFailed {
				field: "preallocated HSA allocation keepalive slots",
				requested: allocation_capacity,
			});
		}
		let mut dependencies = Vec::new();
		if let Err(_error) = dependencies.try_reserve_exact(dependency_capacity) {
			self.signals.release_unsubmitted(signal);
			return Err(Error::AllocationFailed {
				field: "preallocated HSA dependency keepalive slots",
				requested: dependency_capacity,
			});
		}
		Ok(PreparedPending {
			pool: self.signals.clone(),
			state: PreparedPendingState::Ready {
				signal,
				keepalive: PendingKeepalive {
					_queue: None,
					_executable: None,
					_allocations: allocations,
					_dependencies: dependencies,
				},
			},
			_session: PhantomData,
		})
	}

	/// Submit one direct asynchronous copy through an already realized token.
	pub fn copy_async_prepared(&self, destination: &Allocation<'runtime>, destination_offset: usize, source: &Allocation<'runtime>, source_offset: usize, size: usize, pending: &mut PreparedPending<'_, 'runtime>) -> Result<()> {
		self.runtime.ensure_active()?;
		self.ensure_healthy()?;
		if !Rc::ptr_eq(&pending.pool.inner, &self.signals.inner) {
			return Err(Error::InvalidDispatch {
				reason: "preallocated completion token belongs to another HSA session",
			});
		}
		if size == 0 {
			return Err(Error::InvalidDispatch {
				reason: "zero-byte async copies have ambiguous completion semantics",
			});
		}
		check_range("destination", destination_offset, size, destination.len())?;
		check_range("source", source_offset, size, source.len())?;
		if !source.inner.is_accessible_by(destination.inner.owner) || !destination.inner.is_accessible_by(source.inner.owner) {
			return Err(Error::InvalidDispatch {
				reason: "copy endpoint agents do not both have direct access to both allocations",
			});
		}
		let destination_pointer = destination
			.as_ptr()
			.cast::<u8>()
			.wrapping_add(destination_offset)
			.cast::<c_void>();
		let source_pointer = source
			.as_ptr()
			.cast::<u8>()
			.wrapping_add(source_offset)
			.cast::<c_void>()
			.cast_const();
		let (signal, mut keepalive) = pending.begin(2, 0)?;
		keepalive._allocations.push(Rc::clone(destination.inner()));
		keepalive._allocations.push(Rc::clone(source.inner()));

		// SAFETY: validated ranges lie within live mutually accessible
		// allocations. The prepared token owns the initialized completion signal.
		let status = unsafe {
			(self.runtime.api.amd_memory_async_copy)(
				destination_pointer,
				destination.inner.owner,
				source_pointer,
				source.inner.owner,
				size,
				0,
				std::ptr::null(),
				signal.raw(),
			)
		};
		if let Err(error) = self
			.runtime
			.api
			.check_status_only("hsa_amd_memory_async_copy", status)
		{
			pending.restore_ready(signal, keepalive);
			return Err(error);
		}
		pending.activate(signal, keepalive);
		if let Err(error) = self.ensure_healthy() {
			pending.retire_active();
			return Err(error);
		}
		Ok(())
	}

	pub fn copy_async<'session>(&'session self, destination: &Allocation<'runtime>, destination_offset: usize, source: &Allocation<'runtime>, source_offset: usize, size: usize) -> Result<Pending<'session, 'runtime>> {
		self.runtime.ensure_active()?;
		self.ensure_healthy()?;
		if size == 0 {
			return Err(Error::InvalidDispatch {
				reason: "zero-byte async copies have ambiguous completion semantics",
			});
		}
		check_range("destination", destination_offset, size, destination.len())?;
		check_range("source", source_offset, size, source.len())?;
		if !source.inner.is_accessible_by(destination.inner.owner) || !destination.inner.is_accessible_by(source.inner.owner) {
			return Err(Error::InvalidDispatch {
				reason: "copy endpoint agents do not both have direct access to both allocations",
			});
		}
		let signal = self.signals.acquire()?;
		let destination_pointer = destination
			.as_ptr()
			.cast::<u8>()
			.wrapping_add(destination_offset)
			.cast::<c_void>();
		let source_pointer = source
			.as_ptr()
			.cast::<u8>()
			.wrapping_add(source_offset)
			.cast::<c_void>()
			.cast_const();
		let keepalive = PendingKeepalive {
			_queue: None,
			_executable: None,
			_allocations: vec![Rc::clone(destination.inner()), Rc::clone(source.inner())],
			_dependencies: Vec::new(),
		};

		// SAFETY: validated ranges lie within live allocations; owner agents are
		// exact discovery handles; no dependency array is supplied; completion
		// signal is initialized to one.
		let status = unsafe {
			(self.runtime.api.amd_memory_async_copy)(
				destination_pointer,
				destination.inner.owner,
				source_pointer,
				source.inner.owner,
				size,
				0,
				std::ptr::null(),
				signal.raw(),
			)
		};
		if let Err(error) = self.runtime.api.check("hsa_amd_memory_async_copy", status) {
			self.signals.release_unsubmitted(signal);
			return Err(error);
		}
		if let Err(error) = self.ensure_healthy() {
			self.signals.retire(signal, keepalive);
			return Err(error);
		}
		Ok(Pending {
			pool: self.signals.clone(),
			signal: Some(signal),
			keepalive: Some(keepalive),
			terminal: None,
			_session: PhantomData,
		})
	}
}

trait QueueIo {
	fn load_write_index_relaxed(&self) -> u64;
	fn load_read_index_scacquire(&self) -> u64;
	fn write_packet(&self, index: u64, packet: AqlPacket);
	fn publish_header_screlease(&self, index: u64, header: u16);
	fn publish_write_index_screlease(&self, next: u64);
	fn ring_doorbell_screlease(&self, index: u64);
}

#[derive(Clone, Copy)]
enum AqlPacket {
	Kernel(HsaKernelDispatchPacket),
	BarrierAnd(HsaBarrierAndPacket),
}

impl AqlPacket {
	fn invalid_header(self) -> u16 {
		match self {
			Self::Kernel(packet) => packet.header,
			Self::BarrierAnd(packet) => packet.header,
		}
	}
}

#[derive(Clone, Copy)]
struct PreparedPacket {
	body: AqlPacket,
	final_header: u16,
}

fn queue_occupancy(io: &impl QueueIo) -> (u64, u64, u64) {
	let write_index = io.load_write_index_relaxed();
	let read_index = io.load_read_index_scacquire();
	(
		write_index,
		read_index,
		write_index.wrapping_sub(read_index),
	)
}

fn enqueue_packets(io: &impl QueueIo, size: u32, packets: &[PreparedPacket]) -> Result<u64> {
	debug_assert!(!packets.is_empty());
	let (write_index, read_index, occupied) = queue_occupancy(io);
	let required = packets.len() as u64;
	if occupied > u64::from(size) || required > u64::from(size).saturating_sub(occupied) {
		return Err(Error::QueueFull {
			write_index,
			read_index,
			size,
		});
	}
	for (offset, packet) in packets.iter().enumerate() {
		debug_assert_eq!(packet.body.invalid_header(), PACKET_TYPE_INVALID);
		let index = write_index.wrapping_add(offset as u64);
		// The body remains invisible until release publication of the valid
		// header. Each packet is then made visible to the consumer in order.
		io.write_packet(index, packet.body);
		io.publish_header_screlease(index, packet.final_header);
		io.publish_write_index_screlease(index.wrapping_add(1));
		io.ring_doorbell_screlease(index);
	}
	Ok(write_index)
}

/// Result of an explicitly bounded, nonblocking queue-capacity probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueueProgress {
	Ready {
		available_packets: u32,
		probes: u32,
	},
	Backpressured {
		available_packets: u32,
		required_packets: u32,
		probes: u32,
	},
}

fn progress_capacity(io: &impl QueueIo, size: u32, required_packets: u32, probe_budget: NonZeroU32) -> Result<QueueProgress> {
	if required_packets == 0 || required_packets > size {
		return Err(Error::InvalidProgressRequest {
			required_packets,
			queue_size: size,
		});
	}
	let mut available_packets = 0;
	for probe in 1..=probe_budget.get() {
		let (_, _, occupied) = queue_occupancy(io);
		available_packets = u64::from(size)
			.saturating_sub(occupied)
			.min(u64::from(u32::MAX)) as u32;
		if available_packets >= required_packets {
			return Ok(QueueProgress::Ready {
				available_packets,
				probes: probe,
			});
		}
	}
	Ok(QueueProgress::Backpressured {
		available_packets,
		required_packets,
		probes: probe_budget.get(),
	})
}

struct HsaQueueIo<'a> {
	core: &'a QueueCore,
}

impl HsaQueueIo<'_> {
	fn slot(&self, index: u64) -> *mut c_void {
		// SAFETY: queue creation validated a non-null power-of-two ring. Every
		// AQL slot is exactly 64 bytes.
		unsafe {
			let size = (*self.core.raw).size;
			let slot = index & u64::from(size - 1);
			(*self.core.raw)
				.base_address
				.cast::<[u8; 64]>()
				.add(slot as usize)
				.cast::<c_void>()
		}
	}
}

impl QueueIo for HsaQueueIo<'_> {
	fn load_write_index_relaxed(&self) -> u64 {
		// SAFETY: the core owns a live queue.
		unsafe { (self.core.api.queue_load_write_index_relaxed)(self.core.raw) }
	}

	fn load_read_index_scacquire(&self) -> u64 {
		// SAFETY: the core owns a live queue.
		unsafe { (self.core.api.queue_load_read_index_scacquire)(self.core.raw) }
	}

	fn write_packet(&self, index: u64, packet: AqlPacket) {
		debug_assert_eq!(packet.invalid_header(), PACKET_TYPE_INVALID);
		// SAFETY: capacity was checked and the selected unconsumed ring slot is
		// exclusively owned by this single producer.
		unsafe {
			match packet {
				AqlPacket::Kernel(packet) => {
					std::ptr::write(self.slot(index).cast::<HsaKernelDispatchPacket>(), packet);
				}
				AqlPacket::BarrierAnd(packet) => {
					std::ptr::write(self.slot(index).cast::<HsaBarrierAndPacket>(), packet);
				}
			}
		}
	}

	fn publish_header_screlease(&self, index: u64, header: u16) {
		// SAFETY: header is the first aligned u16 in the exclusively owned slot.
		// Release publication makes every prior packet-body write visible.
		unsafe {
			self.slot(index)
				.cast::<AtomicU16>()
				.as_ref()
				.expect("queue slot is non-null")
				.store(header, Ordering::Release);
		}
	}

	fn publish_write_index_screlease(&self, next: u64) {
		// SAFETY: this API is exposed only for a host-confined single producer.
		// Release ordering follows release publication of the packet header.
		unsafe {
			(self.core.api.queue_store_write_index_screlease)(self.core.raw, next);
		}
	}

	fn ring_doorbell_screlease(&self, index: u64) {
		// SAFETY: the doorbell belongs to this live queue. Release ordering
		// follows release publication of the packet header.
		unsafe {
			let doorbell = (*self.core.raw).doorbell_signal;
			(self.core.api.signal_store_screlease)(doorbell, index as i64);
		}
	}
}

fn kernel_packet_header(barrier: bool) -> u16 { PACKET_TYPE_KERNEL_DISPATCH | (u16::from(barrier) << 8) | (FENCE_SCOPE_SYSTEM << PACKET_HEADER_ACQUIRE_FENCE_SCOPE) | (FENCE_SCOPE_SYSTEM << PACKET_HEADER_RELEASE_FENCE_SCOPE) }

fn barrier_and_packet_header() -> u16 { PACKET_TYPE_BARRIER_AND | (FENCE_SCOPE_SYSTEM << PACKET_HEADER_ACQUIRE_FENCE_SCOPE) | (FENCE_SCOPE_SYSTEM << PACKET_HEADER_RELEASE_FENCE_SCOPE) }

const BARRIER_AND_FAN_IN: usize = 5;

fn barrier_packet_count(mut dependencies: usize) -> usize {
	let mut packets = 0;
	while dependencies != 0 {
		dependencies = dependencies.div_ceil(BARRIER_AND_FAN_IN);
		packets += dependencies;
		if dependencies == 1 {
			break;
		}
	}
	packets
}

fn lower_barrier_packets(dependencies: &[HsaSignal], completions: &[HsaSignal]) -> Vec<HsaBarrierAndPacket> {
	debug_assert_eq!(completions.len(), barrier_packet_count(dependencies.len()));
	let mut packets = Vec::with_capacity(completions.len());
	let mut current = dependencies.to_vec();
	let mut completion_index = 0;
	while !current.is_empty() {
		let mut next = Vec::with_capacity(current.len().div_ceil(BARRIER_AND_FAN_IN));
		for chunk in current.chunks(BARRIER_AND_FAN_IN) {
			let completion_signal = completions[completion_index];
			completion_index += 1;
			let mut dep_signal = [HsaSignal { handle: 0 }; BARRIER_AND_FAN_IN];
			dep_signal[..chunk.len()].copy_from_slice(chunk);
			packets.push(HsaBarrierAndPacket {
				header: PACKET_TYPE_INVALID,
				reserved0: 0,
				reserved1: 0,
				dep_signal,
				reserved2: 0,
				completion_signal,
			});
			next.push(completion_signal);
		}
		if next.len() == 1 {
			break;
		}
		current = next;
	}
	packets
}

/// Number of AQL packets needed for a kernel dispatch after the requested
/// number of dependencies.
pub fn dependent_dispatch_packet_count(dependency_count: usize) -> Result<u32> {
	let count = barrier_packet_count(dependency_count)
		.checked_add(1)
		.and_then(|count| u32::try_from(count).ok())
		.ok_or(Error::InvalidDispatch {
			reason: "dependency barrier tree packet count overflows",
		})?;
	Ok(count)
}

fn validate_geometry(description: &AgentDescription, geometry: DispatchGeometry) -> Result<()> {
	if !(1..=3).contains(&geometry.dimensions) {
		return Err(Error::InvalidDispatch {
			reason: "dispatch dimensions must be 1, 2, or 3",
		});
	}
	for dimension in 0..3 {
		let active = dimension < usize::from(geometry.dimensions);
		if geometry.grid[dimension] == 0 || geometry.workgroup[dimension] == 0 {
			return Err(Error::InvalidDispatch {
				reason: "grid and workgroup dimensions must be nonzero",
			});
		}
		if !active && (geometry.grid[dimension] != 1 || geometry.workgroup[dimension] != 1) {
			return Err(Error::InvalidDispatch {
				reason: "unused grid and workgroup dimensions must equal one",
			});
		}
		if u32::from(geometry.workgroup[dimension]) > geometry.grid[dimension] {
			return Err(Error::InvalidDispatch {
				reason: "workgroup dimensions cannot exceed grid dimensions",
			});
		}
	}
	let workgroup_total = geometry
		.workgroup
		.iter()
		.try_fold(1_u32, |total, value| total.checked_mul(u32::from(*value)))
		.ok_or(Error::InvalidDispatch {
			reason: "workgroup cardinality overflows",
		})?;
	let grid_total = geometry
		.grid
		.iter()
		.try_fold(1_u64, |total, value| total.checked_mul(u64::from(*value)))
		.ok_or(Error::InvalidDispatch {
			reason: "grid cardinality overflows",
		})?;
	for isa in &description.isas {
		if workgroup_total > isa.maximum_workgroup_size || grid_total > isa.maximum_grid_size || (0..3).any(|dimension| geometry.workgroup[dimension] > isa.maximum_workgroup_dimensions[dimension] || geometry.grid[dimension] > isa.maximum_grid_dimensions[dimension]) {
			return Err(Error::InvalidDispatch {
				reason: "geometry exceeds an exact discovered ISA limit",
			});
		}
	}
	Ok(())
}

impl<'session, 'runtime> Queue<'session, 'runtime> {
	/// Publish one dependency-free kernel packet through a signal and
	/// keepalive token created before `init`.
	///
	/// Host-side DAG readiness is established by the executor before this call.
	/// Device-side dependency trees continue to use [`Self::dispatch_after`]
	/// until a bounded dependency set is supplied during realization.
	pub fn dispatch_prepared(&self, kernel: &Kernel<'session, 'runtime>, kernarg: Option<&Allocation<'runtime>>, geometry: DispatchGeometry, pending: &mut PreparedPending<'session, 'runtime>) -> Result<()> {
		self.session.runtime.ensure_active()?;
		self.session.ensure_healthy()?;
		if !Rc::ptr_eq(&pending.pool.inner, &self.session.signals.inner) {
			return Err(Error::InvalidDispatch {
				reason: "preallocated completion token belongs to another HSA session",
			});
		}
		let core = self.core();
		if core.kind != QueueKind::SingleProducer {
			return Err(Error::InvalidDispatch {
				reason: "safe AQL publication requires a single-producer queue",
			});
		}
		if core.agent != kernel.executable.agent {
			return Err(Error::InvalidDispatch {
				reason: "kernel executable belongs to a different agent",
			});
		}
		validate_geometry(&self.session.description, geometry)?;
		let kernarg_pointer = match (kernel.metadata.kernarg_segment_size, kernarg) {
			(0, None) => std::ptr::null_mut(),
			(_, Some(allocation)) => {
				if !allocation
					.inner
					.global_flags
					.is_some_and(|flags| flags.contains(MemoryPoolFlags::KERNARG_INITIALIZATION))
				{
					return Err(Error::InvalidDispatch {
						reason: "kernarg must use a discovered kernarg-capable pool",
					});
				}
				if !allocation.inner.is_accessible_by(core.agent) {
					return Err(Error::InvalidDispatch {
						reason: "queue agent has not been granted access to kernarg memory",
					});
				}
				if allocation.len() < kernel.metadata.kernarg_segment_size as usize {
					return Err(Error::InvalidDispatch {
						reason: "kernarg allocation is smaller than kernel metadata",
					});
				}
				if !(allocation.as_ptr() as usize).is_multiple_of(kernel.metadata.kernarg_segment_alignment as usize) {
					return Err(Error::InvalidDispatch {
						reason: "kernarg allocation does not meet kernel alignment",
					});
				}
				allocation.as_ptr()
			}
			(_, None) => {
				return Err(Error::InvalidDispatch {
					reason: "kernel requires a kernarg allocation",
				});
			}
		};
		if kernel.metadata.dynamic_callstack && geometry.dynamic_private_bytes == 0 {
			return Err(Error::InvalidDispatch {
				reason: "dynamic-callstack kernel requires an explicit private-byte allowance",
			});
		}
		let private_segment_size = kernel
			.metadata
			.private_segment_size
			.checked_add(geometry.dynamic_private_bytes)
			.ok_or(Error::InvalidDispatch {
				reason: "private segment size overflows",
			})?;
		let group_segment_size = kernel
			.metadata
			.group_segment_size
			.checked_add(geometry.dynamic_group_bytes)
			.ok_or(Error::InvalidDispatch {
				reason: "group segment size overflows",
			})?;
		let (signal, mut keepalive) = pending.begin(usize::from(kernarg.is_some()), 0)?;
		keepalive._queue = Some(Rc::clone(core));
		keepalive._executable = Some(Rc::clone(&kernel.executable));
		if let Some(allocation) = kernarg {
			keepalive._allocations.push(Rc::clone(allocation.inner()));
		}
		let packet = HsaKernelDispatchPacket {
			header: PACKET_TYPE_INVALID,
			setup: u16::from(geometry.dimensions),
			workgroup_size_x: geometry.workgroup[0],
			workgroup_size_y: geometry.workgroup[1],
			workgroup_size_z: geometry.workgroup[2],
			reserved0: 0,
			grid_size_x: geometry.grid[0],
			grid_size_y: geometry.grid[1],
			grid_size_z: geometry.grid[2],
			private_segment_size,
			group_segment_size,
			kernel_object: kernel.object,
			kernarg_address: kernarg_pointer,
			reserved2: 0,
			completion_signal: signal.raw(),
		};
		let packets = [PreparedPacket {
			body: AqlPacket::Kernel(packet),
			final_header: kernel_packet_header(geometry.barrier),
		}];
		let io = HsaQueueIo { core };
		if let Err(error) = enqueue_packets(&io, self.size_packets(), &packets) {
			pending.restore_ready(signal, keepalive);
			return Err(error);
		}
		pending.activate(signal, keepalive);
		if let Err(error) = self.session.ensure_healthy() {
			pending.retire_active();
			return Err(error);
		}
		Ok(())
	}

	pub fn dispatch(&self, kernel: &Kernel<'session, 'runtime>, kernarg: Option<&Allocation<'runtime>>, geometry: DispatchGeometry) -> Result<Pending<'session, 'runtime>> { self.dispatch_after(kernel, kernarg, geometry, &[]) }

	/// Probe queue capacity at most `probe_budget` times without sleeping,
	/// blocking, or publishing a packet.
	pub fn progress_capacity(&self, required_packets: u32, probe_budget: NonZeroU32) -> Result<QueueProgress> {
		self.session.runtime.ensure_active()?;
		self.session.ensure_healthy()?;
		progress_capacity(
			&HsaQueueIo { core: self.core() },
			self.size_packets(),
			required_packets,
			probe_budget,
		)
	}

	/// Publish a deterministic barrier-AND reduction tree followed immediately
	/// by a dependent kernel packet. The host never waits for a dependency: the
	/// terminal barrier packet prevents the queue from reaching the kernel until
	/// every input signal reaches zero.
	pub fn dispatch_after(&self, kernel: &Kernel<'session, 'runtime>, kernarg: Option<&Allocation<'runtime>>, geometry: DispatchGeometry, dependencies: &[Dependency<'session, 'runtime>]) -> Result<Pending<'session, 'runtime>> {
		self.session.runtime.ensure_active()?;
		self.session.ensure_healthy()?;
		let core = self.core();
		if core.kind != QueueKind::SingleProducer {
			return Err(Error::InvalidDispatch {
				reason: "safe AQL publication requires a single-producer queue",
			});
		}
		if core.agent != kernel.executable.agent {
			return Err(Error::InvalidDispatch {
				reason: "kernel executable belongs to a different agent",
			});
		}
		validate_geometry(&self.session.description, geometry)?;
		for dependency in dependencies {
			if !Rc::ptr_eq(&dependency.pool.inner, &self.session.signals.inner) {
				return Err(Error::InvalidDispatch {
					reason: "dependency belongs to a different HSA session",
				});
			}
			// Detect already-terminal negative signals and queue poison before
			// publishing a barrier that could never be satisfied.
			let _ = dependency.poll()?;
		}
		let required_packets = dependent_dispatch_packet_count(dependencies.len())?;
		if required_packets > self.size_packets() {
			return Err(Error::InvalidProgressRequest {
				required_packets,
				queue_size: self.size_packets(),
			});
		}

		let kernarg_pointer = match (kernel.metadata.kernarg_segment_size, kernarg) {
			(0, None) => std::ptr::null_mut(),
			(_, Some(allocation)) => {
				if !allocation
					.inner
					.global_flags
					.is_some_and(|flags| flags.contains(MemoryPoolFlags::KERNARG_INITIALIZATION))
				{
					return Err(Error::InvalidDispatch {
						reason: "kernarg must use a discovered kernarg-capable pool",
					});
				}
				if !allocation.inner.is_accessible_by(core.agent) {
					return Err(Error::InvalidDispatch {
						reason: "queue agent has not been granted access to kernarg memory",
					});
				}
				if allocation.len() < kernel.metadata.kernarg_segment_size as usize {
					return Err(Error::InvalidDispatch {
						reason: "kernarg allocation is smaller than kernel metadata",
					});
				}
				if !(allocation.as_ptr() as usize).is_multiple_of(kernel.metadata.kernarg_segment_alignment as usize) {
					return Err(Error::InvalidDispatch {
						reason: "kernarg allocation does not meet kernel alignment",
					});
				}
				allocation.as_ptr()
			}
			(_, None) => {
				return Err(Error::InvalidDispatch {
					reason: "kernel requires a kernarg allocation",
				});
			}
		};
		if kernel.metadata.dynamic_callstack && geometry.dynamic_private_bytes == 0 {
			return Err(Error::InvalidDispatch {
				reason: "dynamic-callstack kernel requires an explicit private-byte allowance",
			});
		}
		let private_segment_size = kernel
			.metadata
			.private_segment_size
			.checked_add(geometry.dynamic_private_bytes)
			.ok_or(Error::InvalidDispatch {
				reason: "private segment size overflows",
			})?;
		let group_segment_size = kernel
			.metadata
			.group_segment_size
			.checked_add(geometry.dynamic_group_bytes)
			.ok_or(Error::InvalidDispatch {
				reason: "group segment size overflows",
			})?;

		let signal = self.session.signals.acquire()?;
		let barrier_count = barrier_packet_count(dependencies.len());
		let mut barrier_signals = Vec::with_capacity(barrier_count);
		for _ in 0..barrier_count {
			match self.session.signals.acquire() {
				Ok(barrier_signal) => barrier_signals.push(barrier_signal),
				Err(error) => {
					for barrier_signal in barrier_signals {
						self.session.signals.release_unsubmitted(barrier_signal);
					}
					self.session.signals.release_unsubmitted(signal);
					return Err(error);
				}
			}
		}

		let dependency_handles: Vec<_> = dependencies
			.iter()
			.map(|dependency| dependency.record.raw())
			.collect();
		let barrier_completion_handles: Vec<_> = barrier_signals.iter().map(|record| record.raw()).collect();
		let barrier_packets = lower_barrier_packets(&dependency_handles, &barrier_completion_handles);

		// Construct every potentially allocating keepalive before reserving a
		// queue slot. No allocation or fallible construction follows enqueue.
		let keepalive = PendingKeepalive {
			_queue: Some(Rc::clone(core)),
			_executable: Some(Rc::clone(&kernel.executable)),
			_allocations: kernarg
				.into_iter()
				.map(|allocation| Rc::clone(allocation.inner()))
				.collect(),
			_dependencies: dependencies
				.iter()
				.map(|dependency| Rc::clone(&dependency.record))
				.chain(barrier_signals.iter().cloned())
				.collect(),
		};
		let packet = HsaKernelDispatchPacket {
			header: PACKET_TYPE_INVALID,
			setup: u16::from(geometry.dimensions),
			workgroup_size_x: geometry.workgroup[0],
			workgroup_size_y: geometry.workgroup[1],
			workgroup_size_z: geometry.workgroup[2],
			reserved0: 0,
			grid_size_x: geometry.grid[0],
			grid_size_y: geometry.grid[1],
			grid_size_z: geometry.grid[2],
			private_segment_size,
			group_segment_size,
			kernel_object: kernel.object,
			kernarg_address: kernarg_pointer,
			reserved2: 0,
			completion_signal: signal.raw(),
		};
		let mut packets = Vec::with_capacity(required_packets as usize);
		packets.extend(barrier_packets.into_iter().map(|packet| {
			PreparedPacket {
				body: AqlPacket::BarrierAnd(packet),
				final_header: barrier_and_packet_header(),
			}
		}));
		packets.push(PreparedPacket {
			body: AqlPacket::Kernel(packet),
			final_header: kernel_packet_header(geometry.barrier),
		});
		let io = HsaQueueIo { core };
		if let Err(error) = enqueue_packets(&io, self.size_packets(), &packets) {
			drop(keepalive);
			for barrier_signal in barrier_signals {
				self.session.signals.release_unsubmitted(barrier_signal);
			}
			self.session.signals.release_unsubmitted(signal);
			return Err(error);
		}
		for barrier_signal in &barrier_signals {
			barrier_signal.release_retirement_reservation();
		}
		drop(barrier_signals);

		if let Err(error) = self.session.ensure_healthy() {
			self.session.signals.retire(signal, keepalive);
			return Err(error);
		}
		Ok(Pending {
			pool: self.session.signals.clone(),
			signal: Some(signal),
			keepalive: Some(keepalive),
			terminal: None,
			_session: PhantomData,
		})
	}
}

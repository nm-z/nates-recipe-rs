use core::{ffi::c_void, mem::MaybeUninit}; use std::{ ptr, rc::Rc, sync::{ Arc, Condvar, Mutex,
		atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering}, }, };

use crate::{ DeviceType, Error, QueueKind, Result, Runtime, abi::{
		AGENT_FEATURE_KERNEL_DISPATCH, AMD_AGENT_INFO_MEMORY_AVAIL, HsaAgent, HsaQueue, HsaStatus,
		SYSTEM_INFO_TIMESTAMP_FREQUENCY, }, discovery::{AgentDescription, DiscoveredAgent, RawPool}, execution::SignalPool,
	loader::Api, };

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueFault { pub status: i32, pub source_queue_id: Option<u64>, pub epoch: u64, }

pub(crate) struct SharedFault { poisoned: AtomicBool, status: AtomicI32, source_queue_id: AtomicU64,
	source_known: AtomicBool, epoch: AtomicU64, _wait_lock: Mutex<()>, wake: Condvar, }

impl SharedFault { pub(crate) fn new() -> Self { Self { poisoned: AtomicBool::new(false), status: AtomicI32::new(0),
			source_queue_id: AtomicU64::new(0), source_known: AtomicBool::new(false), epoch: AtomicU64::new(0),
			_wait_lock: Mutex::new(()), wake: Condvar::new(), } }

	pub(crate) fn record(&self, status: i32, source_queue_id: Option<u64>) { self.status.store(status, Ordering::Relaxed);
		if let Some(source_queue_id) = source_queue_id { self.source_queue_id .store(source_queue_id, Ordering::Relaxed);
			self.source_known.store(true, Ordering::Relaxed); } else { self.source_known.store(false, Ordering::Relaxed); }
		self.epoch.fetch_add(1, Ordering::Relaxed); self.poisoned.store(true, Ordering::Release);
		// This callback path takes no lock and allocates no memory.
		self.wake.notify_all(); }

	pub(crate) fn snapshot(&self) -> Option<QueueFault> { if !self.poisoned.load(Ordering::Acquire) { return None; }
		Some(QueueFault { status: self.status.load(Ordering::Relaxed), source_queue_id: self .source_known
				.load(Ordering::Relaxed) .then(|| self.source_queue_id.load(Ordering::Relaxed)),
			epoch: self.epoch.load(Ordering::Relaxed), }) }

	pub(crate) fn check(&self) -> Result<()> { if let Some(fault) = self.snapshot() { Err(Error::SessionPoisoned {
				status: fault.status, source_queue_id: fault.source_queue_id, epoch: fault.epoch, }) } else { Ok(()) } } }

struct QueueCallbackContext { fault: Arc<SharedFault>, }

unsafe extern "C" fn queue_error_callback(status: HsaStatus, source: *mut HsaQueue, data: *mut c_void) {
	if data.is_null() { return; }
	// SAFETY: a live queue retains this stable boxed context until
	// `hsa_queue_destroy` has returned.
	let context = unsafe { &*data.cast::<QueueCallbackContext>() }; let source_queue_id = if source.is_null() { None
	} else {
		// SAFETY: ROCr supplies a live source queue for the callback duration.
		Some(unsafe { (*source).id }) }; context.fault.record(status, source_queue_id); }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueConfig { pub size_packets: u32, pub kind: QueueKind,
	/// `u32::MAX` asks ROCr to select its normal limit.
	pub private_segment_size: u32,
	/// `u32::MAX` asks ROCr to select its normal limit.
	pub group_segment_size: u32, }

impl QueueConfig { pub const fn new(size_packets: u32, kind: QueueKind) -> Self { Self { size_packets, kind,
			private_segment_size: u32::MAX, group_segment_size: u32::MAX, } } }

/// A realized ownership scope for one discovered AMD GPU agent.
pub struct Session<'runtime> {
	pub(crate) runtime: &'runtime Runtime,
	pub(crate) raw_agent: HsaAgent, pub(crate) raw_pools: Vec<RawPool>, pub(crate) description: AgentDescription,
	pub(crate) fault: Arc<SharedFault>, pub(crate) signals: SignalPool, }

impl<'runtime> DiscoveredAgent<'runtime> {
	/// Consume a discovery record into a GPU-only session.
	pub fn into_session(self) -> Result<Session<'runtime>> {
		self.runtime.ensure_active()?; if self.description.device_type != DeviceType::Gpu {
			return Err(Error::UnsupportedAgent {
				reason: "the agent is not a GPU",
			}); }
		if !self.description.supports_kernel_dispatch() { return Err(Error::UnsupportedAgent {
				reason: "the GPU does not report kernel-dispatch support",
			}); }
		if self.description.isas.is_empty() || self .description .isas .iter() .any(|isa| isa.amd_target.is_none())
		{ return Err(Error::UnsupportedAgent {
				reason: "the GPU lacks an exact AMD ISA identity",
			}); }

		let mut timestamp_frequency_hz = MaybeUninit::<u64>::uninit();
		// SAFETY: the reviewed attribute writes one `u64` into valid storage.
		let status = unsafe { (self.runtime.api.system_get_info)( SYSTEM_INFO_TIMESTAMP_FREQUENCY,
				timestamp_frequency_hz.as_mut_ptr().cast::<c_void>(), ) }; self.runtime .api
			.check("hsa_system_get_info(TIMESTAMP_FREQUENCY)", status)?;
		// SAFETY: the successful query initialized the complete value.
		let timestamp_frequency_hz = unsafe { timestamp_frequency_hz.assume_init() };

		let fault = Arc::new(SharedFault::new()); let signals = SignalPool::new( Arc::clone(&self.runtime.api),
			Arc::clone(&fault), timestamp_frequency_hz, ); Ok(Session { runtime: self.runtime, raw_agent: self.raw_agent,
			raw_pools: self.raw_pools, description: self.description, fault, signals, }) } }

impl Session<'_> {
	pub fn description(&self) -> &AgentDescription { &self.description }

	/// Queries ROCr's current available-memory counter for this exact agent.
	pub fn available_memory_bytes(&self) -> Result<u64> { self.runtime.ensure_active()?; self.ensure_healthy()?;
		let mut value = MaybeUninit::<u64>::uninit();
		// SAFETY: AMD_AGENT_INFO_MEMORY_AVAIL writes one `u64` to valid storage.
		let status = unsafe { (self.runtime.api.agent_get_info)( self.raw_agent, AMD_AGENT_INFO_MEMORY_AVAIL,
				value.as_mut_ptr().cast::<core::ffi::c_void>(), ) }; self.runtime .api
			.check("hsa_agent_get_info(AMD_MEMORY_AVAIL)", status)?;
		// SAFETY: the successful query initialized the complete value.
		Ok(unsafe { value.assume_init() }) }

	/// The newest asynchronous queue fault, if any. Poisoning is permanent.
	pub fn fault(&self) -> Option<QueueFault> { self.fault.snapshot() }

	pub fn ensure_healthy(&self) -> Result<()> { self.fault.check() }

	pub fn create_queue(&self, config: QueueConfig) -> Result<Queue<'_, '_>> {
		self.runtime.ensure_active()?; self.ensure_healthy()?;
		let capabilities = self.description.queue.ok_or(Error::UnsupportedAgent {
			reason: "the GPU reports no user-mode queue capability",
		})?; if config.size_packets < capabilities.minimum_packets || config.size_packets > capabilities.maximum_packets
			|| !config.size_packets.is_power_of_two()
		{ return Err(Error::InvalidQueueSize { requested: config.size_packets, minimum: capabilities.minimum_packets,
				maximum: capabilities.maximum_packets,
				reason: "requested size must be a power of two inside the discovered range",
			}); }

		let kind_allowed = match capabilities.advertised_kind {
			QueueKind::SingleProducer => config.kind == QueueKind::SingleProducer, QueueKind::MultiProducer => { matches!(
					config.kind, QueueKind::MultiProducer | QueueKind::SingleProducer ) }
			QueueKind::Cooperative => config.kind == QueueKind::Cooperative, }; if !kind_allowed {
			return Err(Error::UnsupportedQueueKind { requested: config.kind, advertised: capabilities.advertised_kind, }); }

		let callback = Box::new(QueueCallbackContext { fault: Arc::clone(&self.fault), });
		let callback_data = (&*callback as *const QueueCallbackContext) .cast_mut() .cast::<c_void>();
		let mut raw = ptr::null_mut();
		// SAFETY: the reviewed ABI and validated discovery/configuration satisfy
		// queue creation. Callback data remains stable in `QueueCore`.
		let status = unsafe { (self.runtime.api.queue_create)( self.raw_agent, config.size_packets, config.kind.as_raw(),
				Some(queue_error_callback), callback_data, config.private_segment_size, config.group_segment_size, &mut raw, ) };
		self.runtime.api.check("hsa_queue_create", status)?;
		if raw.is_null() { return Err(Error::NullQueue); }
		// SAFETY: a successful queue creation returns a readable queue object.
		let (base_is_null, actual_size, actual_kind_raw, features) = unsafe { ( (*raw).base_address.is_null(), (*raw).size,
				(*raw).kind, (*raw).features, ) }; let actual_kind = QueueKind::from_raw(actual_kind_raw);
		let invalid = base_is_null || actual_size == 0 || !actual_size.is_power_of_two()
			|| features & AGENT_FEATURE_KERNEL_DISPATCH == 0
			|| actual_kind.is_err(); if invalid {
			// SAFETY: this is the newly created queue's one matching destroy.
			let destroy_status = unsafe { (self.runtime.api.queue_destroy)(raw) };
			if destroy_status == crate::abi::STATUS_SUCCESS { drop(callback); } else {
				// Destruction failed, so ROCr may still retain callback data.
				Box::leak(callback); }
			return Err(Error::InvalidQueueReturned { kind: actual_kind_raw, features, size: actual_size, base_is_null, }); }

		Ok(Queue { session: self, core: Some(Rc::new(QueueCore { api: Arc::clone(&self.runtime.api), raw,
				callback: Some(callback),
				// ROCr can report the agent's MULTI protocol in this read-only
				// field even when SINGLE was requested. The requested producer
				// discipline is retained and enforced by this confined API.
				kind: config.kind, agent: self.raw_agent, })), }) } }

pub(crate) struct QueueCore { pub(crate) api: Arc<Api>, pub(crate) raw: *mut HsaQueue,
	callback: Option<Box<QueueCallbackContext>>, pub(crate) kind: QueueKind, pub(crate) agent: HsaAgent, }

impl QueueCore { fn destroy(&mut self) -> Result<()> { if self.raw.is_null() { return Ok(()); }
		let raw = std::mem::replace(&mut self.raw, ptr::null_mut());
		// SAFETY: `raw` is the one live queue pointer owned by this core.
		let status = unsafe { (self.api.queue_destroy)(raw) }; if status == crate::abi::STATUS_SUCCESS {
			drop(self.callback.take()); Ok(()) } else {
			// Failed destruction has ambiguous callback ownership. Retain the
			// context forever rather than permit a callback use-after-free.
			if let Some(callback) = self.callback.take() { Box::leak(callback); }
			self.api.check("hsa_queue_destroy", status)
		} } }

impl Drop for QueueCore { fn drop(&mut self) { let _ = self.destroy(); } }

/// A host-thread-confined HSA queue. Single-producer dispatch is available only
/// when `kind()` is `SingleProducer`.
pub struct Queue<'session, 'runtime> {
	pub(crate) session: &'session Session<'runtime>,
	pub(crate) core: Option<Rc<QueueCore>>, }

impl Queue<'_, '_> {
	pub(crate) fn core(&self) -> &Rc<QueueCore> { self.core.as_ref().expect("live queue owns its core") }

	pub fn id(&self) -> u64 {
		// SAFETY: the queue core owns a live read-only queue object.
		unsafe { (*self.core().raw).id } }

	pub fn size_packets(&self) -> u32 {
		// SAFETY: same invariant as `id`.
		unsafe { (*self.core().raw).size } }

	pub fn kind(&self) -> QueueKind { self.core().kind }

	pub fn session(&self) -> &Session<'_> { self.session }

	/// Explicitly destroy a queue when it has no pending-token keepalive.
	pub fn close(mut self) -> Result<()> {
		let core = self.core.take().expect("live queue owns its core");
		match Rc::try_unwrap(core) { Ok(mut core) => core.destroy(), Err(core) => { drop(core); Err(Error::ResourceBusy {
					resource: "HSA queue",
				}) } } } }

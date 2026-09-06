use core::fmt;
use std::{
	fs::File,
	os::unix::fs::FileExt,
	sync::{
		Arc, Condvar, Mutex, TryLockError,
		atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering},
	},
	thread::{self, JoinHandle},
	time::Duration,
};

use recipe_core::ByteCount;

use crate::{Arena, Error, Result, arena::Backing};

const SLOT_UNCLAIMED: u8 = 0;
const SLOT_PREPARED: u8 = 1;
const SLOT_QUEUED: u8 = 2;
const SLOT_RUNNING: u8 = 3;
const SLOT_COMPLETE: u8 = 4;
const SLOT_FAILED: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotCapacity(usize);

impl SlotCapacity {
	pub fn new(value: usize) -> Result<Self> {
		match value {
			0 => {
				Err(Error::InvalidConfiguration(
					"job slot capacity must be nonzero",
				))
			}
			value => Ok(Self(value)),
		}
	}

	#[must_use]
	pub const fn get(self) -> usize { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
	worker_threads: usize,
	slots: SlotCapacity,
	staging_bytes_per_worker: usize,
}

impl RuntimeConfig {
	pub fn new(worker_threads: usize, slots: SlotCapacity, staging_bytes_per_worker: usize) -> Result<Self> {
		if worker_threads == 0 {
			return Err(Error::InvalidConfiguration(
				"worker thread count must be nonzero",
			));
		}
		if staging_bytes_per_worker == 0 {
			return Err(Error::InvalidConfiguration(
				"per-worker disk staging capacity must be nonzero",
			));
		}
		Ok(Self {
			worker_threads,
			slots,
			staging_bytes_per_worker,
		})
	}

	#[must_use]
	pub const fn worker_threads(self) -> usize { self.worker_threads }

	#[must_use]
	pub const fn slots(self) -> SlotCapacity { self.slots }

	#[must_use]
	pub const fn staging_bytes_per_worker(self) -> usize { self.staging_bytes_per_worker }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeState {
	Ready,
	Poisoned,
	Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollStatus {
	Pending,
	Complete,
}

#[derive(Debug)]
struct Job {
	source: Arc<Backing>,
	source_offset: u64,
	destination: Arc<Backing>,
	destination_offset: u64,
	bytes: u64,
}

#[derive(Debug)]
struct JobSlot {
	state: AtomicU8,
	job: Mutex<Option<Job>>,
	failure: Mutex<Option<Error>>,
}

impl JobSlot {
	fn new() -> Self {
		Self {
			state: AtomicU8::new(SLOT_UNCLAIMED),
			job: Mutex::new(None),
			failure: Mutex::new(None),
		}
	}
}

#[derive(Debug)]
struct Shared {
	slots: Vec<Arc<JobSlot>>,
	wake_generation: AtomicU64,
	wake_lock: Mutex<()>,
	wake: Condvar,
	stopping: AtomicBool,
	poisoned: AtomicBool,
}

pub struct Runtime {
	config: RuntimeConfig,
	shared: Arc<Shared>,
	workers: Vec<JoinHandle<()>>,
	next_slot: AtomicUsize,
	state: RuntimeState,
}

impl Runtime {
	pub fn new(config: RuntimeConfig) -> Result<Self> {
		let mut slots = Vec::new();
		slots.try_reserve_exact(config.slots.get())
			.map_err(|_| Error::InvalidConfiguration("host slot table allocation failed"))?;
		for _ in 0..config.slots.get() {
			slots.push(Arc::new(JobSlot::new()));
		}
		let shared = Arc::new(Shared {
			slots,
			wake_generation: AtomicU64::new(0),
			wake_lock: Mutex::new(()),
			wake: Condvar::new(),
			stopping: AtomicBool::new(false),
			poisoned: AtomicBool::new(false),
		});
		let worker_count = config.worker_threads.min(config.slots.get());
		let mut staging = Vec::new();
		staging
			.try_reserve_exact(worker_count)
			.map_err(|_| Error::InvalidConfiguration("host staging table allocation failed"))?;
		for _ in 0..worker_count {
			staging.push(allocate_staging(config.staging_bytes_per_worker)?);
		}
		let mut workers = Vec::new();
		workers
			.try_reserve_exact(worker_count)
			.map_err(|_| Error::InvalidConfiguration("host worker table allocation failed"))?;
		for (index, mut worker_staging) in staging.into_iter().enumerate() {
			let worker_shared = Arc::clone(&shared);
			let spawned = thread::Builder::new()
				.name(format!("recipe-host-{index}"))
				.spawn(move || worker_loop(&worker_shared, &mut worker_staging));
			match spawned {
				Ok(handle) => workers.push(handle),
				Err(error) => {
					shared.stopping.store(true, Ordering::Release);
					shared.wake.notify_all();
					for worker in workers {
						let _ = worker.join();
					}
					return Err(Error::ThreadSpawn(error.kind()));
				}
			}
		}
		Ok(Self {
			config,
			shared,
			workers,
			next_slot: AtomicUsize::new(0),
			state: RuntimeState::Ready,
		})
	}

	#[must_use]
	pub const fn config(&self) -> RuntimeConfig { self.config }

	#[must_use]
	pub fn state(&self) -> RuntimeState {
		match self.state {
			RuntimeState::Ready if self.shared.poisoned.load(Ordering::Acquire) => RuntimeState::Poisoned,
			state => state,
		}
	}

	pub fn prepare_copy(&self) -> Result<PendingCopy> {
		if self.state() != RuntimeState::Ready {
			return Err(Error::Poisoned);
		}
		let index = self.next_slot.fetch_add(1, Ordering::Relaxed);
		let slot = self
			.shared
			.slots
			.get(index)
			.ok_or(Error::SlotCapacityExhausted)?;
		slot.state
			.compare_exchange(
				SLOT_UNCLAIMED,
				SLOT_PREPARED,
				Ordering::AcqRel,
				Ordering::Acquire,
			)
			.map_err(|_| Error::SlotBusy)?;
		Ok(PendingCopy {
			slot: Arc::clone(slot),
			shared: Arc::clone(&self.shared),
		})
	}

	pub fn close(mut self) -> Result<()> { self.shutdown() }

	fn shutdown(&mut self) -> Result<()> {
		if self.state == RuntimeState::Closed {
			return Ok(());
		}
		self.shared.stopping.store(true, Ordering::Release);
		self.shared.wake.notify_all();
		let mut result = Ok(());
		for worker in self.workers.drain(..) {
			if worker.join().is_err() {
				result = Err(Error::ThreadPanicked);
			}
		}
		self.state = RuntimeState::Closed;
		result
	}
}

impl fmt::Debug for Runtime {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Runtime")
			.field("config", &self.config)
			.field("state", &self.state())
			.field("prepared_slots", &self.next_slot.load(Ordering::Relaxed))
			.finish_non_exhaustive()
	}
}

impl Drop for Runtime {
	fn drop(&mut self) { let _ = self.shutdown(); }
}

pub struct PendingCopy {
	slot: Arc<JobSlot>,
	shared: Arc<Shared>,
}

impl PendingCopy {
	pub fn submit(&mut self, source: &Arena, source_offset: u64, destination: &Arena, destination_offset: u64, bytes: ByteCount) -> Result<()> {
		if self.shared.poisoned.load(Ordering::Acquire) {
			return Err(Error::Poisoned);
		}
		if self.slot.state.load(Ordering::Acquire) != SLOT_PREPARED {
			return Err(Error::InvalidPendingState);
		}
		validate_range(source, source_offset, bytes)?;
		validate_range(destination, destination_offset, bytes)?;
		let mut job = match self.slot.job.try_lock() {
			Ok(job) => job,
			Err(TryLockError::WouldBlock) => return Err(Error::SlotBusy),
			Err(TryLockError::Poisoned(_)) => return Err(Error::Poisoned),
		};
		if job.is_some() {
			return Err(Error::SlotBusy);
		}
		*job = Some(Job {
			source: Arc::clone(&source.backing),
			source_offset,
			destination: Arc::clone(&destination.backing),
			destination_offset,
			bytes: bytes.get(),
		});
		self.slot.state.store(SLOT_QUEUED, Ordering::Release);
		drop(job);
		notify_worker(&self.shared);
		Ok(())
	}

	pub fn poll(&self) -> Result<PollStatus> {
		match self.slot.state.load(Ordering::Acquire) {
			SLOT_QUEUED | SLOT_RUNNING => Ok(PollStatus::Pending),
			SLOT_COMPLETE => Ok(PollStatus::Complete),
			SLOT_FAILED => {
				let failure = self.slot.failure.lock().map_err(|_| Error::Poisoned)?;
				Err(failure.clone().unwrap_or(Error::Poisoned))
			}
			SLOT_UNCLAIMED | SLOT_PREPARED => Err(Error::InvalidPendingState),
			_ => Err(Error::Poisoned),
		}
	}

	pub(crate) fn reset(&mut self) -> Result<()> {
		if self.slot.state.load(Ordering::Acquire) != SLOT_COMPLETE {
			return Err(Error::InvalidPendingState);
		}
		let mut job = self.slot.job.lock().map_err(|_| Error::Poisoned)?;
		let mut failure = self.slot.failure.lock().map_err(|_| Error::Poisoned)?;
		*job = None;
		*failure = None;
		self.slot.state.store(SLOT_PREPARED, Ordering::Release);
		Ok(())
	}
}

impl fmt::Debug for PendingCopy {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		let state = match self.slot.state.load(Ordering::Acquire) {
			SLOT_UNCLAIMED => "unclaimed",
			SLOT_PREPARED => "prepared",
			SLOT_QUEUED => "queued",
			SLOT_RUNNING => "running",
			SLOT_COMPLETE => "complete",
			SLOT_FAILED => "failed",
			_ => "invalid",
		};
		formatter
			.debug_struct("PendingCopy")
			.field("state", &state)
			.finish_non_exhaustive()
	}
}

fn validate_range(arena: &Arena, offset: u64, bytes: ByteCount) -> Result<()> {
	if bytes == ByteCount::ZERO {
		return Err(Error::InvalidConfiguration(
			"host copy size must be nonzero",
		));
	}
	offset.checked_add(bytes.get())
		.filter(|end| *end <= arena.bytes().get())
		.ok_or(Error::OutOfBounds {
			device: arena.device(),
		})
		.map(|_| ())
}

fn notify_worker(shared: &Shared) {
	shared.wake_generation.fetch_add(1, Ordering::Release);
	shared.wake.notify_one();
}

fn allocate_staging(bytes: usize) -> Result<Box<[u8]>> {
	let mut staging = Vec::new();
	staging
		.try_reserve_exact(bytes)
		.map_err(|_| Error::InvalidConfiguration("host staging allocation failed"))?;
	staging.resize(bytes, 0);
	Ok(staging.into_boxed_slice())
}

fn worker_loop(shared: &Shared, staging: &mut [u8]) {
	loop {
		let mut progressed = false;
		for slot in &shared.slots {
			if slot
				.state
				.compare_exchange(
					SLOT_QUEUED,
					SLOT_RUNNING,
					Ordering::AcqRel,
					Ordering::Acquire,
				)
				.is_err()
			{
				continue;
			}
			progressed = true;
			let outcome = execute_slot(slot, staging);
			match outcome {
				Ok(()) => slot.state.store(SLOT_COMPLETE, Ordering::Release),
				Err(error) => {
					if let Ok(mut failure) = slot.failure.lock() {
						*failure = Some(error);
					} else {
						shared.poisoned.store(true, Ordering::Release);
					}
					slot.state.store(SLOT_FAILED, Ordering::Release);
				}
			}
		}
		if shared.stopping.load(Ordering::Acquire)
			&& !shared.slots.iter().any(|slot| {
				matches!(
					slot.state.load(Ordering::Acquire),
					SLOT_QUEUED | SLOT_RUNNING
				)
			}) {
			break;
		}
		if !progressed {
			let observed = shared.wake_generation.load(Ordering::Acquire);
			let Ok(wake_guard) = shared.wake_lock.lock() else {
				shared.poisoned.store(true, Ordering::Release);
				break;
			};
			let _guard = shared
				.wake
				.wait_timeout_while(wake_guard, Duration::from_millis(10), |_| {
					shared.wake_generation.load(Ordering::Acquire) == observed && !shared.stopping.load(Ordering::Acquire)
				});
		}
	}
}

fn execute_slot(slot: &JobSlot, staging: &mut [u8]) -> Result<()> {
	let job = slot
		.job
		.lock()
		.map_err(|_| Error::Poisoned)?
		.take()
		.ok_or(Error::InvalidPendingState)?;
	execute_copy(&job, staging)
}

fn execute_copy(job: &Job, staging: &mut [u8]) -> Result<()> {
	match (&*job.source, &*job.destination) {
		(
			Backing::Ram {
				storage: source, ..
			},
			Backing::Ram {
				storage: destination,
				..
			},
		) => copy_ram_to_ram(job, source, destination),
		(
			Backing::Ram {
				storage: source, ..
			},
			Backing::Disk { file, .. },
		) => {
			let source = source.lock().map_err(|_| Error::Poisoned)?;
			let range = host_range(job.source_offset, job.bytes)?;
			write_all_at(file, &source[range], job.destination_offset)
		}
		(
			Backing::Disk { file, .. },
			Backing::Ram {
				storage: destination,
				..
			},
		) => {
			let mut destination = destination.lock().map_err(|_| Error::Poisoned)?;
			let range = host_range(job.destination_offset, job.bytes)?;
			read_exact_at(file, &mut destination[range], job.source_offset)
		}
		(
			Backing::Disk { file: source, .. },
			Backing::Disk {
				file: destination, ..
			},
		) => copy_disk_to_disk(job, source, destination, staging),
	}
}

fn copy_ram_to_ram(job: &Job, source: &Mutex<Box<[u8]>>, destination: &Mutex<Box<[u8]>>) -> Result<()> {
	if Arc::ptr_eq(&job.source, &job.destination) {
		let mut bytes = source.lock().map_err(|_| Error::Poisoned)?;
		let source = host_range(job.source_offset, job.bytes)?;
		let destination = usize::try_from(job.destination_offset).map_err(|_| Error::RangeOverflow)?;
		bytes.copy_within(source, destination);
		return Ok(());
	}
	let source_address = Arc::as_ptr(&job.source).cast::<()>() as usize;
	let destination_address = Arc::as_ptr(&job.destination).cast::<()>() as usize;
	if source_address < destination_address {
		let source = source.lock().map_err(|_| Error::Poisoned)?;
		let mut destination = destination.lock().map_err(|_| Error::Poisoned)?;
		copy_ram_slices(job, &source, &mut destination)
	} else {
		let mut destination = destination.lock().map_err(|_| Error::Poisoned)?;
		let source = source.lock().map_err(|_| Error::Poisoned)?;
		copy_ram_slices(job, &source, &mut destination)
	}
}

fn copy_ram_slices(job: &Job, source: &[u8], destination: &mut [u8]) -> Result<()> {
	let source_range = host_range(job.source_offset, job.bytes)?;
	let destination_range = host_range(job.destination_offset, job.bytes)?;
	destination[destination_range].copy_from_slice(&source[source_range]);
	Ok(())
}

fn copy_disk_to_disk(job: &Job, source: &File, destination: &File, staging: &mut [u8]) -> Result<()> {
	let same = Arc::ptr_eq(&job.source, &job.destination);
	let overlaps_backward = same && job.destination_offset > job.source_offset && job.destination_offset < job.source_offset.saturating_add(job.bytes);
	if overlaps_backward {
		let mut remaining = job.bytes;
		while remaining != 0 {
			let chunk = usize::try_from(remaining.min(staging.len() as u64)).map_err(|_| Error::RangeOverflow)?;
			let chunk_offset = remaining
				.checked_sub(chunk as u64)
				.ok_or(Error::RangeOverflow)?;
			read_exact_at(
				source,
				&mut staging[..chunk],
				job.source_offset + chunk_offset,
			)?;
			write_all_at(
				destination,
				&staging[..chunk],
				job.destination_offset + chunk_offset,
			)?;
			remaining = chunk_offset;
		}
		return Ok(());
	}
	let mut copied = 0_u64;
	while copied < job.bytes {
		let remaining = job.bytes - copied;
		let chunk = usize::try_from(remaining.min(staging.len() as u64)).map_err(|_| Error::RangeOverflow)?;
		read_exact_at(source, &mut staging[..chunk], job.source_offset + copied)?;
		write_all_at(
			destination,
			&staging[..chunk],
			job.destination_offset + copied,
		)?;
		copied = copied
			.checked_add(chunk as u64)
			.ok_or(Error::RangeOverflow)?;
	}
	Ok(())
}

fn host_range(offset: u64, bytes: u64) -> Result<std::ops::Range<usize>> {
	let start = usize::try_from(offset).map_err(|_| Error::RangeOverflow)?;
	let length = usize::try_from(bytes).map_err(|_| Error::RangeOverflow)?;
	let end = start.checked_add(length).ok_or(Error::RangeOverflow)?;
	Ok(start..end)
}

fn read_exact_at(file: &File, mut destination: &mut [u8], mut offset: u64) -> Result<()> {
	while !destination.is_empty() {
		let count = file.read_at(destination, offset).map_err(|error| {
			Error::WorkerFailed {
				operation: "disk read",
				kind: error.kind(),
			}
		})?;
		if count == 0 {
			return Err(Error::WorkerFailed {
				operation: "disk read",
				kind: std::io::ErrorKind::UnexpectedEof,
			});
		}
		offset = offset
			.checked_add(count as u64)
			.ok_or(Error::RangeOverflow)?;
		destination = &mut destination[count..];
	}
	Ok(())
}

fn write_all_at(file: &File, mut source: &[u8], mut offset: u64) -> Result<()> {
	while !source.is_empty() {
		let count = file.write_at(source, offset).map_err(|error| {
			Error::WorkerFailed {
				operation: "disk write",
				kind: error.kind(),
			}
		})?;
		if count == 0 {
			return Err(Error::WorkerFailed {
				operation: "disk write",
				kind: std::io::ErrorKind::WriteZero,
			});
		}
		offset = offset
			.checked_add(count as u64)
			.ok_or(Error::RangeOverflow)?;
		source = &source[count..];
	}
	Ok(())
}

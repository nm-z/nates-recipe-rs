use crate::discovery::DeviceInfo;
use crate::driver::Driver;
use crate::error::{CudaError, Result};
use crate::ffi::CuContext;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr;
use std::rc::Rc;

const CU_CTX_SCHED_MASK: u32 = 0x07;
const CU_CTX_SCHED_AUTO: u32 = 0x00;
const CU_CTX_SCHED_SPIN: u32 = 0x01;
const CU_CTX_SCHED_YIELD: u32 = 0x02;
const CU_CTX_SCHED_BLOCKING_SYNC: u32 = 0x04;
const CU_CTX_MAP_HOST: u32 = 0x08;
const CU_CTX_LMEM_RESIZE_TO_MAX: u32 = 0x10;
const KNOWN_CONTEXT_FLAGS: u32 = CU_CTX_SCHED_MASK | CU_CTX_MAP_HOST | CU_CTX_LMEM_RESIZE_TO_MAX;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchedulingPolicy {
	#[default]
	Auto,
	Spin,
	Yield,
	BlockingSync,
}

impl SchedulingPolicy {
	const fn bits(self) -> u32 {
		match self {
			Self::Auto => CU_CTX_SCHED_AUTO,
			Self::Spin => CU_CTX_SCHED_SPIN,
			Self::Yield => CU_CTX_SCHED_YIELD,
			Self::BlockingSync => CU_CTX_SCHED_BLOCKING_SYNC,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContextFlags {
	pub scheduling: SchedulingPolicy,
	pub map_host: bool,
	pub local_memory_resize_to_max: bool,
}

impl ContextFlags {
	pub const fn new(scheduling: SchedulingPolicy) -> Self {
		Self {
			scheduling,
			map_host: false,
			local_memory_resize_to_max: false,
		}
	}

	pub const fn with_map_host(mut self, enabled: bool) -> Self {
		self.map_host = enabled;
		self
	}

	pub const fn with_local_memory_resize_to_max(mut self, enabled: bool) -> Self {
		self.local_memory_resize_to_max = enabled;
		self
	}

	pub const fn bits(self) -> u32 {
		self.scheduling.bits()
			| if self.map_host { CU_CTX_MAP_HOST } else { 0 }
			| if self.local_memory_resize_to_max {
				CU_CTX_LMEM_RESIZE_TO_MAX
			} else {
				0
			}
	}

	pub fn from_bits(bits: u32) -> Result<Self> {
		if bits & !KNOWN_CONTEXT_FLAGS != 0 {
			return Err(CudaError::InvalidContextFlags { bits });
		}
		let scheduling = match bits & CU_CTX_SCHED_MASK {
			CU_CTX_SCHED_AUTO => SchedulingPolicy::Auto,
			CU_CTX_SCHED_SPIN => SchedulingPolicy::Spin,
			CU_CTX_SCHED_YIELD => SchedulingPolicy::Yield,
			CU_CTX_SCHED_BLOCKING_SYNC => SchedulingPolicy::BlockingSync,
			_ => return Err(CudaError::InvalidContextFlags { bits }),
		};
		Ok(Self {
			scheduling,
			map_host: bits & CU_CTX_MAP_HOST != 0,
			local_memory_resize_to_max: bits & CU_CTX_LMEM_RESIZE_TO_MAX != 0,
		})
	}
}

pub struct Context {
	driver: Driver,
	raw: Option<CuContext>,
	device: DeviceInfo,
	_not_send_or_sync: PhantomData<Rc<()>>,
}

impl Context {
	pub fn create(driver: &Driver, device: &DeviceInfo, flags: ContextFlags) -> Result<Self> {
		let bits = flags.bits();
		let _ = ContextFlags::from_bits(bits)?;
		let mut raw = ptr::null_mut();
		driver.check("cuCtxCreate_v2", unsafe {
			(driver.inner.api.ctx_create_v2)(&raw mut raw, bits, device.handle)
		})?;
		if raw.is_null() {
			return Err(CudaError::InvalidDriverValue {
				operation: "cuCtxCreate_v2",
				detail: "success with a null context".to_owned(),
			});
		}

		let mut popped = ptr::null_mut();
		let pop_status = unsafe { (driver.inner.api.ctx_pop_current_v2)(&raw mut popped) };
		if let Err(error) = driver.check("cuCtxPopCurrent_v2(after create)", pop_status) {
			let _ = unsafe { (driver.inner.api.ctx_destroy_v2)(raw) };
			return Err(error);
		}
		if popped != raw {
			let _ = unsafe { (driver.inner.api.ctx_destroy_v2)(raw) };
			return Err(CudaError::ContextStackMismatch);
		}

		Ok(Self {
			driver: driver.clone(),
			raw: Some(raw),
			device: device.clone(),
			_not_send_or_sync: PhantomData,
		})
	}

	pub fn device(&self) -> &DeviceInfo {
		&self.device
	}

	pub fn enter(&self) -> Result<ContextGuard<'_>> {
		let raw = self.raw.ok_or(CudaError::ContextClosed)?;
		self.driver.check("cuCtxPushCurrent_v2", unsafe {
			(self.driver.inner.api.ctx_push_current_v2)(raw)
		})?;
		Ok(ContextGuard {
			context: self,
			active: true,
		})
	}

	pub fn as_raw(&self) -> Result<*mut core::ffi::c_void> {
		self.raw.ok_or(CudaError::ContextClosed)
	}

	/// Returns the driver's current free and total memory counters for this
	/// exact context device.
	pub fn memory_info(&self) -> Result<MemoryInfo> {
		let guard = self.enter()?;
		let mut free = 0;
		let mut total = 0;
		let result = self.driver.check("cuMemGetInfo_v2", unsafe {
			(self.driver.inner.api.mem_get_info_v2)(&raw mut free, &raw mut total)
		});
		guard.leave()?;
		result?;
		Ok(MemoryInfo {
			free_bytes: free,
			total_bytes: total,
		})
	}

	pub(crate) fn driver(&self) -> &Driver {
		&self.driver
	}

	pub(crate) fn same_context(&self, other: &Self) -> bool {
		core::ptr::eq(self, other)
	}

	pub fn close(mut self) -> Result<()> {
		self.destroy()
	}

	fn destroy(&mut self) -> Result<()> {
		let raw = self.raw.take().ok_or(CudaError::ContextClosed)?;
		self.driver.check("cuCtxDestroy_v2", unsafe {
			(self.driver.inner.api.ctx_destroy_v2)(raw)
		})
	}
}

/// One live CUDA memory-capacity observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryInfo {
	pub free_bytes: usize,
	pub total_bytes: usize,
}

impl Drop for Context {
	fn drop(&mut self) {
		if let Some(raw) = self.raw.take() {
			let _ = unsafe { (self.driver.inner.api.ctx_destroy_v2)(raw) };
		}
	}
}

impl core::fmt::Debug for Context {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Context")
			.field("device", &self.device)
			.field("open", &self.raw.is_some())
			.finish_non_exhaustive()
	}
}

pub struct ContextGuard<'a> {
	context: &'a Context,
	active: bool,
}

impl ContextGuard<'_> {
	pub fn leave(mut self) -> Result<()> {
		self.pop()
	}

	fn pop(&mut self) -> Result<()> {
		let expected = self.context.raw.ok_or(CudaError::ContextClosed)?;
		let mut popped = ptr::null_mut();
		self.context.driver.check("cuCtxPopCurrent_v2", unsafe {
			(self.context.driver.inner.api.ctx_pop_current_v2)(&raw mut popped)
		})?;
		self.active = false;
		if popped != expected {
			return Err(CudaError::ContextStackMismatch);
		}
		Ok(())
	}
}

impl Deref for ContextGuard<'_> {
	type Target = Context;

	fn deref(&self) -> &Self::Target {
		self.context
	}
}

impl Drop for ContextGuard<'_> {
	fn drop(&mut self) {
		if self.active {
			let _ = self.pop();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ffi::test_support;
	use crate::ffi::test_support::{
		CONTEXT_CREATES, CONTEXT_DESTROYS, CONTEXT_POPS, CONTEXT_PUSHES, reset_context_counts,
	};
	use core::sync::atomic::Ordering;
	use std::sync::Mutex;

	static CONTEXT_TEST_LOCK: Mutex<()> = Mutex::new(());

	#[test]
	fn context_is_detached_then_explicitly_scoped_and_destroyed() {
		let _lock = CONTEXT_TEST_LOCK.lock().unwrap();
		reset_context_counts();
		let driver = Driver::from_test_api(test_support::api(false));
		let discovery = driver.discover().unwrap();
		let context = Context::create(
			&driver,
			&discovery.devices[0],
			ContextFlags::new(SchedulingPolicy::BlockingSync).with_map_host(true),
		)
		.unwrap();
		assert_eq!(CONTEXT_CREATES.load(Ordering::SeqCst), 1);
		assert_eq!(CONTEXT_POPS.load(Ordering::SeqCst), 1);
		{
			let guard = context.enter().unwrap();
			assert_eq!(guard.device().ordinal.get(), 0);
			guard.leave().unwrap();
		}
		assert_eq!(CONTEXT_PUSHES.load(Ordering::SeqCst), 1);
		assert_eq!(CONTEXT_POPS.load(Ordering::SeqCst), 2);
		context.close().unwrap();
		assert_eq!(CONTEXT_DESTROYS.load(Ordering::SeqCst), 1);
	}

	#[test]
	fn invalid_combined_scheduling_flags_fail_closed() {
		assert!(matches!(
			ContextFlags::from_bits(CU_CTX_SCHED_SPIN | CU_CTX_SCHED_YIELD),
			Err(CudaError::InvalidContextFlags { .. })
		));
	}
}

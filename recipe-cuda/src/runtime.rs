use crate::context::Context;
use crate::driver::Driver;
use crate::error::{CudaError, Result};
use crate::ffi::{CUDA_ERROR_NOT_READY, CUDA_SUCCESS, CuDevicePtr, CuEvent, CuFunction, CuModule, CuStream};
use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use std::ffi::CString;
use std::time::{Duration, Instant};

const CU_STREAM_NON_BLOCKING: u32 = 1;
const CU_EVENT_DISABLE_TIMING: u32 = 2;

fn with_current<T>(context: &Context, operation: impl FnOnce(&Driver) -> Result<T>) -> Result<T> {
	let guard = context.enter()?;
	let result = operation(context.driver());
	let leave = guard.leave();
	match result {
		Err(error) => {
			let _ = leave;
			Err(error)
		}
		Ok(value) => {
			leave?;
			Ok(value)
		}
	}
}

fn same_context(operation: &'static str, left: &Context, right: &Context) -> Result<()> {
	if left.same_context(right) {
		Ok(())
	} else {
		Err(CudaError::ResourceContextMismatch { operation })
	}
}

pub struct Module<'ctx> {
	context: &'ctx Context,
	raw: Option<CuModule>,
}

impl<'ctx> Module<'ctx> {
	pub fn load_cubin(context: &'ctx Context, cubin: &[u8]) -> Result<Self> {
		if !cubin.starts_with(b"\x7fELF") {
			return Err(CudaError::InvalidInput {
				operation: "cuModuleLoadData",
				detail: "Recipe accepts an ELF cubin, not PTX or a fat binary".to_owned(),
			});
		}
		let raw = with_current(context, |driver| {
			let mut raw = ptr::null_mut();
			driver.check("cuModuleLoadData", unsafe {
				(driver.inner.api.module_load_data)(&raw mut raw, cubin.as_ptr().cast::<c_void>())
			})?;
			if raw.is_null() {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuModuleLoadData",
					detail: "success with a null module".to_owned(),
				});
			}
			Ok(raw)
		})?;
		Ok(Self {
			context,
			raw: Some(raw),
		})
	}

	pub fn function<'module>(&'module self, name: &str) -> Result<Function<'module, 'ctx>> {
		let module = self.raw.ok_or(CudaError::ContextClosed)?;
		let name_c = CString::new(name).map_err(|_| CudaError::InvalidInput {
			operation: "cuModuleGetFunction",
			detail: "kernel name contains NUL".to_owned(),
		})?;
		if name.is_empty() {
			return Err(CudaError::InvalidInput {
				operation: "cuModuleGetFunction",
				detail: "kernel name is empty".to_owned(),
			});
		}
		let raw = with_current(self.context, |driver| {
			let mut raw = ptr::null_mut();
			driver.check("cuModuleGetFunction", unsafe {
				(driver.inner.api.module_get_function)(&raw mut raw, module, name_c.as_ptr())
			})?;
			if raw.is_null() {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuModuleGetFunction",
					detail: "success with a null function".to_owned(),
				});
			}
			Ok(raw)
		})?;
		Ok(Function {
			module: self,
			raw,
			name: name.to_owned(),
		})
	}

	pub fn unload(mut self) -> Result<()> {
		self.destroy()
	}

	fn destroy(&mut self) -> Result<()> {
		let raw = self.raw.take().ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check("cuModuleUnload", unsafe {
				(driver.inner.api.module_unload)(raw)
			})
		})
	}
}

impl Drop for Module<'_> {
	fn drop(&mut self) {
		if let Some(raw) = self.raw.take() {
			let _ = with_current(self.context, |driver| {
				driver.check("cuModuleUnload", unsafe {
					(driver.inner.api.module_unload)(raw)
				})
			});
		}
	}
}

pub struct Function<'module, 'ctx> {
	module: &'module Module<'ctx>,
	raw: CuFunction,
	name: String,
}

impl Function<'_, '_> {
	pub fn name(&self) -> &str {
		&self.name
	}
}

pub struct DeviceBuffer<'ctx> {
	context: &'ctx Context,
	pointer: Option<CuDevicePtr>,
	len: usize,
}

impl<'ctx> DeviceBuffer<'ctx> {
	pub fn allocate(context: &'ctx Context, len: usize) -> Result<Self> {
		if len == 0 {
			return Err(CudaError::InvalidInput {
				operation: "cuMemAlloc_v2",
				detail: "zero-byte device allocation".to_owned(),
			});
		}
		let pointer = with_current(context, |driver| {
			let mut pointer = 0;
			driver.check("cuMemAlloc_v2", unsafe {
				(driver.inner.api.mem_alloc_v2)(&raw mut pointer, len)
			})?;
			if pointer == 0 {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuMemAlloc_v2",
					detail: "success with a null device pointer".to_owned(),
				});
			}
			Ok(pointer)
		})?;
		Ok(Self {
			context,
			pointer: Some(pointer),
			len,
		})
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn is_empty(&self) -> bool {
		false
	}

	pub fn device_ptr(&self) -> Result<u64> {
		self.pointer.ok_or(CudaError::ContextClosed)
	}

	pub fn free(mut self) -> Result<()> {
		self.destroy()
	}

	fn checked_pointer(&self, offset: usize, bytes: usize, operation: &'static str) -> Result<CuDevicePtr> {
		offset.checked_add(bytes)
			.filter(|end| *end <= self.len)
			.ok_or_else(|| CudaError::InvalidInput {
				operation,
				detail: format!(
					"range {offset}..{} exceeds {}-byte device allocation",
					offset.saturating_add(bytes),
					self.len
				),
			})?;
		let pointer = self.pointer.ok_or(CudaError::ContextClosed)?;
		pointer
			.checked_add(u64::try_from(offset).map_err(|_| CudaError::InvalidInput {
				operation,
				detail: format!("offset {offset} does not fit a CUDA device pointer"),
			})?)
			.ok_or_else(|| CudaError::InvalidInput {
				operation,
				detail: "device pointer offset overflow".to_owned(),
			})
	}

	fn destroy(&mut self) -> Result<()> {
		let pointer = self.pointer.take().ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check("cuMemFree_v2", unsafe {
				(driver.inner.api.mem_free_v2)(pointer)
			})
		})
	}
}

impl Drop for DeviceBuffer<'_> {
	fn drop(&mut self) {
		if let Some(pointer) = self.pointer.take() {
			let _ = with_current(self.context, |driver| {
				driver.check("cuMemFree_v2", unsafe {
					(driver.inner.api.mem_free_v2)(pointer)
				})
			});
		}
	}
}

pub struct PinnedHostBuffer<'ctx> {
	context: &'ctx Context,
	pointer: Option<NonNull<u8>>,
	len: usize,
}

impl<'ctx> PinnedHostBuffer<'ctx> {
	pub fn allocate(context: &'ctx Context, len: usize) -> Result<Self> {
		if len == 0 {
			return Err(CudaError::InvalidInput {
				operation: "cuMemHostAlloc",
				detail: "zero-byte pinned host allocation".to_owned(),
			});
		}
		let pointer = with_current(context, |driver| {
			let mut pointer = ptr::null_mut();
			driver.check("cuMemHostAlloc", unsafe {
				(driver.inner.api.mem_host_alloc)(&raw mut pointer, len, 0)
			})?;
			NonNull::new(pointer.cast::<u8>()).ok_or_else(|| CudaError::InvalidDriverValue {
				operation: "cuMemHostAlloc",
				detail: "success with a null host pointer".to_owned(),
			})
		})?;
		unsafe {
			pointer.as_ptr().write_bytes(0, len);
		}
		Ok(Self {
			context,
			pointer: Some(pointer),
			len,
		})
	}

	pub fn len(&self) -> usize {
		self.len
	}

	pub fn is_empty(&self) -> bool {
		false
	}

	pub fn as_slice(&self) -> &[u8] {
		let pointer = self.pointer.expect("open pinned host buffer");
		unsafe { core::slice::from_raw_parts(pointer.as_ptr(), self.len) }
	}

	pub fn as_mut_slice(&mut self) -> &mut [u8] {
		let pointer = self.pointer.expect("open pinned host buffer");
		unsafe { core::slice::from_raw_parts_mut(pointer.as_ptr(), self.len) }
	}

	pub fn free(mut self) -> Result<()> {
		self.destroy()
	}

	fn checked_pointer(&self, offset: usize, bytes: usize, operation: &'static str) -> Result<NonNull<u8>> {
		offset.checked_add(bytes)
			.filter(|end| *end <= self.len)
			.ok_or_else(|| CudaError::InvalidInput {
				operation,
				detail: format!(
					"range {offset}..{} exceeds {}-byte pinned allocation",
					offset.saturating_add(bytes),
					self.len
				),
			})?;
		let pointer = self.pointer.ok_or(CudaError::ContextClosed)?;
		Ok(unsafe { NonNull::new_unchecked(pointer.as_ptr().add(offset)) })
	}

	fn destroy(&mut self) -> Result<()> {
		let pointer = self.pointer.take().ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check("cuMemFreeHost", unsafe {
				(driver.inner.api.mem_free_host)(pointer.as_ptr().cast::<c_void>())
			})
		})
	}
}

impl Drop for PinnedHostBuffer<'_> {
	fn drop(&mut self) {
		if let Some(pointer) = self.pointer.take() {
			let _ = with_current(self.context, |driver| {
				driver.check("cuMemFreeHost", unsafe {
					(driver.inner.api.mem_free_host)(pointer.as_ptr().cast::<c_void>())
				})
			});
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dim3 {
	pub x: u32,
	pub y: u32,
	pub z: u32,
}

impl Dim3 {
	pub fn new(x: u32, y: u32, z: u32) -> Result<Self> {
		if x == 0 || y == 0 || z == 0 {
			return Err(CudaError::InvalidInput {
				operation: "cuLaunchKernel",
				detail: format!("zero launch dimension ({x}, {y}, {z})"),
			});
		}
		Ok(Self { x, y, z })
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LaunchConfig {
	pub grid: Dim3,
	pub block: Dim3,
	pub dynamic_shared_memory_bytes: u32,
}

impl LaunchConfig {
	pub const fn new(grid: Dim3, block: Dim3) -> Self {
		Self {
			grid,
			block,
			dynamic_shared_memory_bytes: 0,
		}
	}

	pub const fn with_dynamic_shared_memory(mut self, bytes: u32) -> Self {
		self.dynamic_shared_memory_bytes = bytes;
		self
	}
}

pub struct Stream<'ctx> {
	context: &'ctx Context,
	raw: Option<CuStream>,
}

impl<'ctx> Stream<'ctx> {
	pub fn create_nonblocking(context: &'ctx Context) -> Result<Self> {
		let raw = with_current(context, |driver| {
			let mut raw = ptr::null_mut();
			driver.check("cuStreamCreate", unsafe {
				(driver.inner.api.stream_create)(&raw mut raw, CU_STREAM_NON_BLOCKING)
			})?;
			if raw.is_null() {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuStreamCreate",
					detail: "success with a null stream".to_owned(),
				});
			}
			Ok(raw)
		})?;
		Ok(Self {
			context,
			raw: Some(raw),
		})
	}

	pub fn poll_idle(&self) -> Result<CompletionStatus> {
		let raw = self.raw.ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			query_status(driver, "cuStreamQuery", unsafe {
				(driver.inner.api.stream_query)(raw)
			})
		})
	}

	/// Enqueues a pinned-host to device copy and records `completion` after it.
	///
	/// # Safety
	///
	/// The returned `Pending` must be retained and driven to a terminal result.
	/// Discarding a still-pending token and then destroying either allocation
	/// violates the CUDA lifetime contract.
	pub unsafe fn copy_h2d<'op>(
		&'op self,
		destination: &'op DeviceBuffer<'ctx>,
		destination_offset: usize,
		source: &'op PinnedHostBuffer<'ctx>,
		source_offset: usize,
		bytes: usize,
		completion: Event<'ctx>,
	) -> Result<Pending<'op, 'ctx>> {
		same_context("cuMemcpyHtoDAsync_v2", self.context, destination.context)?;
		same_context("cuMemcpyHtoDAsync_v2", self.context, source.context)?;
		same_context("cuEventRecord", self.context, completion.context)?;
		let stream = self.raw.ok_or(CudaError::ContextClosed)?;
		let destination = destination.checked_pointer(destination_offset, bytes, "cuMemcpyHtoDAsync_v2")?;
		let source = source.checked_pointer(source_offset, bytes, "cuMemcpyHtoDAsync_v2")?;
		let event = completion.raw.ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check_status_only("cuMemcpyHtoDAsync_v2", unsafe {
				(driver.inner.api.memcpy_htod_async_v2)(
					destination,
					source.as_ptr().cast::<c_void>(),
					bytes,
					stream,
				)
			})?;
			driver.check_status_only("cuEventRecord", unsafe {
				(driver.inner.api.event_record)(event, stream)
			})
		})?;
		Ok(Pending::new(completion))
	}

	/// Enqueues a device to pinned-host copy and records `completion` after it.
	///
	/// # Safety
	///
	/// The returned `Pending` must be retained and driven to a terminal result.
	/// Discarding it while pending can permit the destination borrow to end
	/// before CUDA has stopped writing.
	pub unsafe fn copy_d2h<'op>(
		&'op self,
		destination: &'op mut PinnedHostBuffer<'ctx>,
		destination_offset: usize,
		source: &'op DeviceBuffer<'ctx>,
		source_offset: usize,
		bytes: usize,
		completion: Event<'ctx>,
	) -> Result<Pending<'op, 'ctx>> {
		same_context("cuMemcpyDtoHAsync_v2", self.context, destination.context)?;
		same_context("cuMemcpyDtoHAsync_v2", self.context, source.context)?;
		same_context("cuEventRecord", self.context, completion.context)?;
		let stream = self.raw.ok_or(CudaError::ContextClosed)?;
		let destination = destination.checked_pointer(destination_offset, bytes, "cuMemcpyDtoHAsync_v2")?;
		let source = source.checked_pointer(source_offset, bytes, "cuMemcpyDtoHAsync_v2")?;
		let event = completion.raw.ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check_status_only("cuMemcpyDtoHAsync_v2", unsafe {
				(driver.inner.api.memcpy_dtoh_async_v2)(
					destination.as_ptr().cast::<c_void>(),
					source,
					bytes,
					stream,
				)
			})?;
			driver.check_status_only("cuEventRecord", unsafe {
				(driver.inner.api.event_record)(event, stream)
			})
		})?;
		Ok(Pending::new(completion))
	}

	/// Enqueues a device-to-device copy and records `completion` after it.
	///
	/// # Safety
	///
	/// The returned `Pending` must be retained and driven to a terminal result.
	/// Discarding it while pending can permit either allocation borrow to end
	/// before CUDA has stopped accessing the copied range.
	pub unsafe fn copy_d2d<'op>(
		&'op self,
		destination: &'op DeviceBuffer<'ctx>,
		destination_offset: usize,
		source: &'op DeviceBuffer<'ctx>,
		source_offset: usize,
		bytes: usize,
		completion: Event<'ctx>,
	) -> Result<Pending<'op, 'ctx>> {
		same_context("cuMemcpyDtoDAsync_v2", self.context, destination.context)?;
		same_context("cuMemcpyDtoDAsync_v2", self.context, source.context)?;
		same_context("cuEventRecord", self.context, completion.context)?;
		let stream = self.raw.ok_or(CudaError::ContextClosed)?;
		let destination = destination.checked_pointer(destination_offset, bytes, "cuMemcpyDtoDAsync_v2")?;
		let source = source.checked_pointer(source_offset, bytes, "cuMemcpyDtoDAsync_v2")?;
		let event = completion.raw.ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check_status_only("cuMemcpyDtoDAsync_v2", unsafe {
				(driver.inner.api.memcpy_dtod_async_v2)(destination, source, bytes, stream)
			})?;
			driver.check_status_only("cuEventRecord", unsafe {
				(driver.inner.api.event_record)(event, stream)
			})
		})?;
		Ok(Pending::new(completion))
	}

	/// Enqueues a kernel and records `completion` after it.
	///
	/// `parameters` must contain pointers to correctly aligned host-side CUDA
	/// kernel argument values matching the cubin entry ABI. CUDA copies those
	/// values before this function returns. Every device allocation referenced
	/// by those values must also appear in `keepalive`; the returned `Pending`
	/// then keeps their Rust borrows live until terminal completion.
	///
	/// # Safety
	///
	/// The function's cubin ABI, parameter pointer types, pointee sizes, and
	/// launch geometry must match. The driver cannot validate that Rust-side
	/// argument storage has the required type. The returned `Pending` must also
	/// be retained and driven to a terminal result before any referenced
	/// resource is destroyed.
	pub unsafe fn launch<'op>(
		&'op self,
		function: &'op Function<'_, 'ctx>,
		config: LaunchConfig,
		parameters: &mut [*mut c_void],
		keepalive: &'op [&'op DeviceBuffer<'ctx>],
		completion: Event<'ctx>,
	) -> Result<Pending<'op, 'ctx>> {
		same_context("cuLaunchKernel", self.context, function.module.context)?;
		same_context("cuEventRecord", self.context, completion.context)?;
		for buffer in keepalive {
			same_context("cuLaunchKernel", self.context, buffer.context)?;
		}
		let stream = self.raw.ok_or(CudaError::ContextClosed)?;
		let event = completion.raw.ok_or(CudaError::ContextClosed)?;
		let parameter_pointer = if parameters.is_empty() {
			ptr::null_mut()
		} else {
			parameters.as_mut_ptr()
		};
		with_current(self.context, |driver| {
			driver.check_status_only("cuLaunchKernel", unsafe {
				(driver.inner.api.launch_kernel)(
					function.raw,
					config.grid.x,
					config.grid.y,
					config.grid.z,
					config.block.x,
					config.block.y,
					config.block.z,
					config.dynamic_shared_memory_bytes,
					stream,
					parameter_pointer,
					ptr::null_mut(),
				)
			})?;
			driver.check_status_only("cuEventRecord", unsafe {
				(driver.inner.api.event_record)(event, stream)
			})
		})?;
		Ok(Pending::new(completion))
	}

	pub fn destroy(mut self) -> Result<()> {
		self.close()
	}

	fn close(&mut self) -> Result<()> {
		let raw = self.raw.take().ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check("cuStreamDestroy_v2", unsafe {
				(driver.inner.api.stream_destroy_v2)(raw)
			})
		})
	}
}

impl Drop for Stream<'_> {
	fn drop(&mut self) {
		if let Some(raw) = self.raw.take() {
			let _ = with_current(self.context, |driver| {
				driver.check("cuStreamDestroy_v2", unsafe {
					(driver.inner.api.stream_destroy_v2)(raw)
				})
			});
		}
	}
}

pub struct Event<'ctx> {
	context: &'ctx Context,
	raw: Option<CuEvent>,
}

impl<'ctx> Event<'ctx> {
	pub fn create_completion(context: &'ctx Context) -> Result<Self> {
		let raw = with_current(context, |driver| {
			let mut raw = ptr::null_mut();
			driver.check("cuEventCreate", unsafe {
				(driver.inner.api.event_create)(&raw mut raw, CU_EVENT_DISABLE_TIMING)
			})?;
			if raw.is_null() {
				return Err(CudaError::InvalidDriverValue {
					operation: "cuEventCreate",
					detail: "success with a null event".to_owned(),
				});
			}
			Ok(raw)
		})?;
		Ok(Self {
			context,
			raw: Some(raw),
		})
	}

	pub fn destroy(mut self) -> Result<()> {
		self.close()
	}

	fn close(&mut self) -> Result<()> {
		let raw = self.raw.take().ok_or(CudaError::ContextClosed)?;
		with_current(self.context, |driver| {
			driver.check("cuEventDestroy_v2", unsafe {
				(driver.inner.api.event_destroy_v2)(raw)
			})
		})
	}
}

impl Drop for Event<'_> {
	fn drop(&mut self) {
		if let Some(raw) = self.raw.take() {
			let _ = with_current(self.context, |driver| {
				driver.check("cuEventDestroy_v2", unsafe {
					(driver.inner.api.event_destroy_v2)(raw)
				})
			});
		}
	}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionStatus {
	Pending,
	Complete,
}

#[must_use = "a CUDA submission must be polled or waited to terminal completion"]
pub struct Pending<'op, 'ctx> {
	event: Option<Event<'ctx>>,
	complete: bool,
	_operation_borrows: PhantomData<&'op ()>,
}

impl<'op, 'ctx> Pending<'op, 'ctx> {
	fn new(event: Event<'ctx>) -> Self {
		Self {
			event: Some(event),
			complete: false,
			_operation_borrows: PhantomData,
		}
	}

	pub fn poll(&mut self) -> Result<CompletionStatus> {
		if self.complete {
			return Ok(CompletionStatus::Complete);
		}
		let event = self.event.as_ref().ok_or(CudaError::ContextClosed)?;
		let raw = event.raw.ok_or(CudaError::ContextClosed)?;
		let status = with_current(event.context, |driver| {
			query_status(driver, "cuEventQuery", unsafe {
				(driver.inner.api.event_query)(raw)
			})
		})?;
		if status == CompletionStatus::Complete {
			self.complete = true;
		}
		Ok(status)
	}

	pub fn wait(mut self, timeout: Duration) -> Result<WaitOutcome<'op, 'ctx>> {
		let deadline = Instant::now()
			.checked_add(timeout)
			.ok_or_else(|| CudaError::InvalidInput {
				operation: "Pending::wait",
				detail: format!("timeout {timeout:?} overflows Instant"),
			})?;
		loop {
			if self.poll()? == CompletionStatus::Complete {
				let event = self.event.take().ok_or(CudaError::ContextClosed)?;
				return Ok(WaitOutcome::Complete(event));
			}
			if Instant::now() >= deadline {
				return Ok(WaitOutcome::TimedOut(self));
			}
			std::thread::yield_now();
		}
	}

	/// Recovers the same completion event after a terminal nonblocking poll.
	///
	/// This is used by pre-final warm execution to recycle an already-created
	/// event without creating a replacement driver object.
	pub fn recycle_event(mut self) -> Result<Event<'ctx>> {
		match self.poll()? {
			CompletionStatus::Pending => Err(CudaError::InvalidInput {
				operation: "Pending::recycle_event",
				detail: "completion event is still pending".to_owned(),
			}),
			CompletionStatus::Complete => self.event.take().ok_or(CudaError::ContextClosed),
		}
	}
}

#[must_use = "a timed-out CUDA submission still owns its pending token"]
pub enum WaitOutcome<'op, 'ctx> {
	Complete(Event<'ctx>),
	TimedOut(Pending<'op, 'ctx>),
}

fn query_status(driver: &Driver, operation: &'static str, status: i32) -> Result<CompletionStatus> {
	match status {
		CUDA_SUCCESS => Ok(CompletionStatus::Complete),
		CUDA_ERROR_NOT_READY => Ok(CompletionStatus::Pending),
		status => {
			driver.check_status_only(operation, status)?;
			unreachable!("non-success status passed Driver::check")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::context::ContextFlags;
	use crate::error::{DriverCallError, DriverStatus};
	use crate::ffi::test_support;
	use crate::ffi::{
		CUDA_ERROR_NOT_READY,
		test_support::{TEST_LOCK, reset_runtime_state, set_event_query_script, teardown_log},
	};

	fn fixture() -> (Driver, crate::Discovery) {
		let driver = Driver::from_test_api(test_support::api(false));
		let discovery = driver.discover().unwrap();
		(driver, discovery)
	}

	#[test]
	fn pending_is_observable_before_completion_and_copy_roundtrips() {
		let _lock = TEST_LOCK.lock().unwrap();
		reset_runtime_state();
		let (driver, discovery) = fixture();
		let context = Context::create(&driver, &discovery.devices[0], ContextFlags::default()).unwrap();
		let stream = Stream::create_nonblocking(&context).unwrap();
		let device = DeviceBuffer::allocate(&context, 4).unwrap();
		let copied_device = DeviceBuffer::allocate(&context, 4).unwrap();
		let mut source = PinnedHostBuffer::allocate(&context, 4).unwrap();
		source.as_mut_slice().copy_from_slice(&[1, 2, 3, 4]);
		let event = Event::create_completion(&context).unwrap();
		set_event_query_script([CUDA_ERROR_NOT_READY, CUDA_SUCCESS]);
		let mut pending = unsafe { stream.copy_h2d(&device, 0, &source, 0, 4, event) }.unwrap();
		assert_eq!(pending.poll().unwrap(), CompletionStatus::Pending);
		assert_eq!(pending.poll().unwrap(), CompletionStatus::Complete);
		let event = match pending.wait(Duration::ZERO).unwrap() {
			WaitOutcome::Complete(event) => event,
			WaitOutcome::TimedOut(_) => panic!("completed token timed out"),
		};
		set_event_query_script([CUDA_SUCCESS]);
		let pending = unsafe { stream.copy_d2d(&copied_device, 0, &device, 0, 4, event) }.unwrap();
		let event = match pending.wait(Duration::from_millis(1)).unwrap() {
			WaitOutcome::Complete(event) => event,
			WaitOutcome::TimedOut(_) => panic!("fake device copy timed out"),
		};

		let mut destination = PinnedHostBuffer::allocate(&context, 4).unwrap();
		set_event_query_script([CUDA_SUCCESS]);
		let pending = unsafe { stream.copy_d2h(&mut destination, 0, &copied_device, 0, 4, event) }.unwrap();
		let event = match pending.wait(Duration::from_millis(1)).unwrap() {
			WaitOutcome::Complete(event) => event,
			WaitOutcome::TimedOut(_) => panic!("fake copy timed out"),
		};
		assert_eq!(destination.as_slice(), &[1, 2, 3, 4]);
		drop(event);
	}

	#[test]
	fn asynchronous_query_failure_is_propagated() {
		let _lock = TEST_LOCK.lock().unwrap();
		reset_runtime_state();
		let (driver, discovery) = fixture();
		let context = Context::create(&driver, &discovery.devices[0], ContextFlags::default()).unwrap();
		let stream = Stream::create_nonblocking(&context).unwrap();
		let device = DeviceBuffer::allocate(&context, 4).unwrap();
		let source = PinnedHostBuffer::allocate(&context, 4).unwrap();
		let event = Event::create_completion(&context).unwrap();
		set_event_query_script([700]);
		let mut pending = unsafe { stream.copy_h2d(&device, 0, &source, 0, 4, event) }.unwrap();
		let error = pending.poll().unwrap_err();
		assert!(matches!(
			error,
			CudaError::DriverCall(DriverCallError {
				operation: "cuEventQuery",
				status: DriverStatus(700),
				..
			})
		));
	}

	#[test]
	fn resources_teardown_before_context_without_modern_optional_symbols() {
		let _lock = TEST_LOCK.lock().unwrap();
		reset_runtime_state();
		let (driver, discovery) = fixture();
		assert_eq!(driver.module_loading_mode().unwrap(), None);
		let context = Context::create(&driver, &discovery.devices[1], ContextFlags::default()).unwrap();
		let module = Module::load_cubin(&context, b"\x7fELFfake-cubin").unwrap();
		let function = module.function("recipe_probe").unwrap();
		let device = DeviceBuffer::allocate(&context, 4).unwrap();
		let host = PinnedHostBuffer::allocate(&context, 4).unwrap();
		let stream = Stream::create_nonblocking(&context).unwrap();
		let event = Event::create_completion(&context).unwrap();

		let mut pointer = device.device_ptr().unwrap();
		let mut parameters = [(&raw mut pointer).cast::<c_void>()];
		let keepalive = [&device];
		set_event_query_script([CUDA_SUCCESS]);
		let pending = unsafe {
			stream.launch(
				&function,
				LaunchConfig::new(Dim3::new(1, 1, 1).unwrap(), Dim3::new(32, 1, 1).unwrap()),
				&mut parameters,
				&keepalive,
				event,
			)
		}
		.unwrap();
		let event = match pending.wait(Duration::from_millis(1)).unwrap() {
			WaitOutcome::Complete(event) => event,
			WaitOutcome::TimedOut(_) => panic!("fake kernel timed out"),
		};

		drop(event);
		drop(stream);
		drop(host);
		drop(device);
		drop(function);
		drop(module);
		drop(context);

		assert_eq!(
			teardown_log(),
			[
				"event",
				"stream",
				"host_memory",
				"device_memory",
				"module",
				"context"
			]
		);
	}

	#[test]
	fn bounded_wait_returns_the_live_pending_token_on_timeout() {
		let _lock = TEST_LOCK.lock().unwrap();
		reset_runtime_state();
		let (driver, discovery) = fixture();
		let context = Context::create(&driver, &discovery.devices[0], ContextFlags::default()).unwrap();
		let stream = Stream::create_nonblocking(&context).unwrap();
		let device = DeviceBuffer::allocate(&context, 1).unwrap();
		let source = PinnedHostBuffer::allocate(&context, 1).unwrap();
		let event = Event::create_completion(&context).unwrap();
		set_event_query_script([CUDA_ERROR_NOT_READY, CUDA_SUCCESS]);
		let pending = unsafe { stream.copy_h2d(&device, 0, &source, 0, 1, event) }.unwrap();
		let pending = match pending.wait(Duration::ZERO).unwrap() {
			WaitOutcome::TimedOut(pending) => pending,
			WaitOutcome::Complete(_) => panic!("first query should be pending"),
		};
		assert!(matches!(
			pending.wait(Duration::from_millis(1)).unwrap(),
			WaitOutcome::Complete(_)
		));
	}
}

//! Narrow CUDA launch-parameter FFI boundary.

use recipe_cuda::{DeviceBuffer, Event, Function, LaunchConfig, Pending, Stream};

use crate::{Error, Result};

#[derive(Debug)]
pub(crate) struct ParameterBlock {
	values: Box<[u64]>,
	parameters: Box<[*mut u8]>,
	keepalive: Vec<*const DeviceBuffer<'static>>,
}

impl ParameterBlock {
	pub(crate) fn new(argument_count: usize) -> Self {
		let mut values = vec![0_u64; argument_count].into_boxed_slice();
		let parameters = values
			.iter_mut()
			.map(|value| core::ptr::from_mut(value).cast::<u8>())
			.collect::<Vec<_>>()
			.into_boxed_slice();
		Self {
			values,
			parameters,
			keepalive: Vec::with_capacity(argument_count.saturating_sub(1)),
		}
	}

	pub(crate) fn len(&self) -> usize {
		self.values.len()
	}

	pub(crate) fn reset_keepalive(&mut self) {
		self.keepalive.clear();
	}

	pub(crate) fn set_value(&mut self, index: usize, value: u64) -> Result<()> {
		let destination = self.values.get_mut(index).ok_or(Error::IntegerOverflow {
			field: "CUDA launch argument index",
		})?;
		*destination = value;
		Ok(())
	}

	pub(crate) fn retain(&mut self, buffer: &DeviceBuffer<'_>) {
		self.keepalive
			.push(core::ptr::from_ref(buffer).cast::<DeviceBuffer<'static>>());
	}

	pub(crate) unsafe fn launch<'operation, 'context>(
		&'operation mut self,
		stream: &'operation Stream<'context>,
		function: &'operation Function<'_, 'context>,
		config: LaunchConfig,
		argument_count: usize,
		event: Event<'context>,
	) -> recipe_cuda::Result<Pending<'operation, 'context>> {
		let keepalive = unsafe {
			// SAFETY: each pointer was made from a live arena buffer. The caller
			// retains those arenas until the returned pending token is terminal.
			core::slice::from_raw_parts(
				self.keepalive.as_ptr().cast::<&DeviceBuffer<'static>>(),
				self.keepalive.len(),
			)
		};
		let parameters = unsafe {
			// SAFETY: a raw pointer has the same representation regardless of
			// its pointee marker. CUDA reads only the pointer-sized entries.
			core::slice::from_raw_parts_mut(self.parameters.as_mut_ptr().cast(), argument_count)
		};
		let pending = unsafe {
			// SAFETY: the caller validated the artifact ABI, argument values,
			// launch geometry, and keepalive set before entering this wrapper.
			stream.launch(function, config, parameters, keepalive, event)
		};
		debug_assert_eq!(self.parameters.len(), self.values.len());
		pending
	}
}

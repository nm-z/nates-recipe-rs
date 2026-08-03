//! Narrow CUDA launch-parameter FFI boundary.

use recipe_cuda::{DeviceBuffer, Function, LaunchConfig, Stream};

#[derive(Debug)]
pub(crate) struct ParameterBlock { values: Box<[u64]>, parameters: Box<[*mut u8]>,
	keepalive: Vec<*const DeviceBuffer<'static>>,
}

impl ParameterBlock { pub(crate) fn new(argument_count: usize) -> Self {
		let mut values = vec![0_u64; argument_count].into_boxed_slice(); let parameters = values .iter_mut()
			.map(|value| core::ptr::from_mut(value).cast::<u8>()) .collect::<Vec<_>>() .into_boxed_slice(); Self { values,
			parameters, keepalive: Vec::with_capacity(argument_count.saturating_sub(1)), } }

	pub(crate) fn len(&self) -> usize { self.values.len() }

	pub(crate) fn reset_keepalive(&mut self) { self.keepalive.clear(); }

	pub(crate) fn set_value(&mut self, index: usize, value: u64) { self.values[index] = value; }

	pub(crate) fn retain(&mut self, buffer: &DeviceBuffer<'_>) {
		self.keepalive
			.push(core::ptr::from_ref(buffer).cast::<DeviceBuffer<'static>>());
	}

	pub(crate) unsafe fn enqueue<'context>(
		&mut self,
		stream: &Stream<'context>,
		function: &Function<'_, 'context>,
		config: LaunchConfig, argument_count: usize, ) -> recipe_cuda::Result<()> {
		// SAFETY: each pointer was made from an arena buffer retained until
		// the executor observes this stream idle.
		let keepalive = unsafe { core::slice::from_raw_parts(
				self.keepalive.as_ptr().cast::<&DeviceBuffer<'static>>(),
				self.keepalive.len(), ) };
		// SAFETY: `argument_count` was validated against the fixed parameter
		// block before this call, and the block remains borrowed for the launch.
		let parameters = unsafe { core::slice::from_raw_parts_mut(self.parameters.as_mut_ptr().cast(), argument_count) };
		// SAFETY: the parameter and buffer slices remain valid until the stream
		// has consumed the enqueued launch.
		let result = unsafe { stream.enqueue_launch(function, config, parameters, keepalive) };
		debug_assert_eq!(self.parameters.len(), self.values.len()); result } }

use std::time::{Duration, Instant};

use recipe_core::{
	AliasPermission, AliasRule, ByteCount, BytesPerSecond, DType, ElementCount, FlopCount, FlopsPerSecond,
	IndexSpace, KernelInput, KernelInputId, KernelOutput, KernelOutputId, KernelTemplate, KernelTemplateId,
	ScalarConstant, ScalarInput, ScalarInstruction, ScalarLiteral, ScalarOpcode, ScalarProgram, ScalarValueId,
	StaticBufferAccess, }; use recipe_probe::{BoundedBenchmarkPlan, ProbeError, ProbeResult};

const NANOS_PER_SECOND: u128 = 1_000_000_000; pub(crate) const MAXIMUM_COMPUTE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimedWork { pub iterations: u32, pub elapsed: Duration, }

pub(crate) fn time_bounded( plan: BoundedBenchmarkPlan,
	mut submit_and_complete: impl FnMut(Duration) -> ProbeResult<()>, ) -> ProbeResult<TimedWork> { if !plan.is_bounded() {
		return Err(ProbeError::Benchmark(
			"GPU benchmark plan is not bounded".to_owned(),
		)); }
	let start = Instant::now(); let mut iterations = 0_u32; while iterations < plan.iterations {
		let elapsed = start.elapsed(); let remaining = plan.maximum_duration.saturating_sub(elapsed); if remaining.is_zero() {
			break; }
		submit_and_complete(remaining)?; iterations += 1; }
	let elapsed = start.elapsed(); if iterations == 0 || elapsed.is_zero() { return Err(ProbeError::Benchmark(
			"bounded GPU benchmark completed no timed work".to_owned(),
		)); }
	Ok(TimedWork { iterations, elapsed, }) }

pub(crate) fn transfer_rate(bytes_per_iteration: u64, timed: TimedWork) -> ProbeResult<BytesPerSecond> {
	let total = u128::from(bytes_per_iteration) .checked_mul(u128::from(timed.iterations))
		.ok_or_else(|| ProbeError::Benchmark("transfer work counter overflowed".to_owned()))?;
	let rate = rate_per_second(total, timed.elapsed)?; BytesPerSecond::new(rate)
		.map_err(|error| ProbeError::Benchmark(format!("invalid measured transfer rate: {error}")))
}

pub(crate) fn calculation_rate(flops_per_iteration: FlopCount, timed: TimedWork) -> ProbeResult<FlopsPerSecond> {
	let total = u128::from(flops_per_iteration.get()) .checked_mul(u128::from(timed.iterations))
		.ok_or_else(|| ProbeError::Benchmark("calculation work counter overflowed".to_owned()))?;
	let rate = rate_per_second(total, timed.elapsed)?; FlopsPerSecond::new(rate)
		.map_err(|error| ProbeError::Benchmark(format!("invalid measured calculation rate: {error}")))
}

fn rate_per_second(work: u128, elapsed: Duration) -> ProbeResult<u64> { let nanos = elapsed.as_nanos(); if nanos == 0 {
		return Err(ProbeError::Benchmark(
			"benchmark timer reported zero elapsed nanoseconds".to_owned(),
		)); }
	let rate = work .checked_mul(NANOS_PER_SECOND)
		.ok_or_else(|| ProbeError::Benchmark("benchmark rate numerator overflowed".to_owned()))?
		/ nanos;
	u64::try_from(rate).map_err(|_| ProbeError::Benchmark("measured rate does not fit u64".to_owned()))
}

pub(crate) fn compute_buffer_bytes(plan: BoundedBenchmarkPlan) -> ProbeResult<usize> {
	let bytes = plan.buffer_bytes.get().min(MAXIMUM_COMPUTE_BYTES);
	let aligned = bytes - bytes % u64::from(DType::F32.byte_width()); if aligned == 0 { return Err(ProbeError::Benchmark(
			"GPU benchmark buffer cannot hold one f32".to_owned(),
		)); }
	usize::try_from(aligned).map_err(|_| ProbeError::Benchmark("GPU benchmark buffer does not fit usize".to_owned()))
}

pub(crate) fn fma_template(bytes: usize, chain_length: u16) -> ProbeResult<KernelTemplate> {
	let elements = u64::try_from(bytes / usize::from(DType::F32.byte_width()))
		.map_err(|_| ProbeError::Benchmark("element count does not fit u64".to_owned()))?;
	let index_space = IndexSpace::new(vec![ElementCount::new(elements)
		.map_err(|error| ProbeError::Benchmark(format!("invalid GPU index space: {error}")))?])
	.map_err(|error| ProbeError::Benchmark(format!("invalid GPU index space: {error}")))?;
	let access = StaticBufferAccess::contiguous(&index_space, DType::F32)
		.map_err(|error| ProbeError::Benchmark(format!("invalid GPU buffer access: {error}")))?;
	if chain_length == 0 { return Err(ProbeError::Benchmark(
			"FLOP benchmark FMA chain cannot be empty".to_owned(),
		)); }

	let input = ScalarValueId::new(1); let multiplier = ScalarValueId::new(2); let addend = ScalarValueId::new(3);
	let mut previous = input; let mut instructions = Vec::with_capacity(usize::from(chain_length));
	for index in 0..chain_length { let result = ScalarValueId::new(u64::from(index) + 4);
		instructions.push(ScalarInstruction { result, dtype: DType::F32, opcode: ScalarOpcode::Fma,
			operands: vec![previous, multiplier, addend], }); previous = result; }
	Ok(KernelTemplate { id: KernelTemplateId::new(0x0050_524f_4245), index_space, inputs: vec![KernelInput {
			id: KernelInputId::new(1), dtype: DType::F32, access: access.clone(), }], outputs: vec![KernelOutput {
			id: KernelOutputId::new(1), dtype: DType::F32, access, }], program: ScalarProgram { inputs: vec![ScalarInput {
				id: input, dtype: DType::F32, }], constants: vec![ ScalarConstant { id: multiplier,
					value: ScalarLiteral::F32Bits(1.000_976_6_f32.to_bits()), }, ScalarConstant { id: addend,
					value: ScalarLiteral::F32Bits(0.000_122_070_31_f32.to_bits()), }, ], instructions, outputs: vec![previous], },
		alias_rules: vec![AliasRule { input: KernelInputId::new(1), output: KernelOutputId::new(1),
			permission: AliasPermission::Forbidden, }], }) }

pub(crate) fn fill_input(bytes: &mut [u8]) { for (index, chunk) in bytes.chunks_exact_mut(4).enumerate() {
		let mantissa = u32::try_from(index & 0x3ff).expect("masked value fits u32");
		let value = f32::from_bits(0x3f00_0000 | mantissa); chunk.copy_from_slice(&value.to_ne_bytes()); } }

pub(crate) fn verify_compute_output(input: &[u8], output: &[u8]) -> ProbeResult<()> {
	if input.len() != output.len() || output.len() < 4 { return Err(ProbeError::Benchmark(
			"FLOP benchmark verification buffers have invalid lengths".to_owned(),
		)); }
	let input_value = f32::from_ne_bytes(input[..4].try_into().expect("four-byte slice"));
	let output_value = f32::from_ne_bytes(output[..4].try_into().expect("four-byte slice"));
	if !output_value.is_finite() || output_value.to_bits() == input_value.to_bits() { return Err(ProbeError::Benchmark(
			"Recipe-owned FLOP kernel did not produce a finite changed value".to_owned(),
		)); }
	Ok(()) }

pub(crate) fn measured<T>(value: T) -> recipe_core::Property<T> { recipe_core::Property::new(value, recipe_core::PropertyProvenance::Measured) }

pub(crate) fn plan_bytes(plan: BoundedBenchmarkPlan) -> ProbeResult<usize> { usize::try_from(plan.buffer_bytes.get())
		.map_err(|_| ProbeError::Benchmark("GPU transfer buffer does not fit usize".to_owned()))
}

pub(crate) fn capacity(bytes: u64) -> ProbeResult<ByteCount> { if bytes == 0 { Err(ProbeError::Benchmark(
			"native backend reported zero GPU capacity".to_owned(),
		)) } else { Ok(ByteCount::new(bytes)) } }

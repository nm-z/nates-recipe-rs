use std::collections::BTreeMap;

use recipe_core::{
	AliasRule, ByteCount, DType, ElementCount, FlopCount, IndexSpace, KernelInput, KernelInputId, KernelOutput,
	KernelOutputId, KernelTemplate, ScalarLiteral, StaticBufferAccess, ValueId,
};
use recipe_language::{
	Elementwise, Gather, Histogram, IndexBounds, IndexMap, PrimitiveKernel, PrimitiveKind, RandomMap, Reduce,
	ReduceOperator, ReduceResult, Scan, ScanMode, Scatter, ScatterConflict, Sort, Tensor,
};

use crate::error::{LoweringError, LoweringErrorKind, LoweringResult};
use crate::model::*;

#[derive(Debug)]
struct StageDraft {
	geometry: DispatchGeometry,
	bindings: Vec<BufferBinding>,
	synchronization: Vec<SynchronizationPoint>,
	atomics: Vec<AtomicContract>,
	fault: Option<FaultContract>,
	resources: StageResourceBounds,
	kind: StageKind,
}

macro_rules! push_stage {
	(
		$builder:expr,
		$geometry:expr,
		$bindings:expr,
		$synchronization:expr,
		$atomics:expr,
		$fault:expr,
		$resources:expr,
		$kind:expr $(,)?
	) => {
		$builder.push_stage(StageDraft {
			geometry: $geometry,
			bindings: $bindings,
			synchronization: $synchronization,
			atomics: $atomics,
			fault: $fault,
			resources: $resources,
			kind: $kind,
		})
	};
}

pub fn lower(
	kernel: &PrimitiveKernel,
	tensors: &BTreeMap<ValueId, &Tensor>,
	hardware: LoweringHardware,
) -> LoweringResult<LoweredProgram> {
	validate_hardware(kernel, hardware)?;
	kernel.validate(tensors).map_err(LoweringError::from)?;
	let mut builder = ProgramBuilder::new(kernel.id, hardware);
	for value in kernel.inputs.iter().chain(&kernel.outputs) {
		let tensor = tensor(tensors, *value, kernel.id)?;
		builder.add_tensor(tensor)?;
	}

	match &kernel.kind {
		PrimitiveKind::Elementwise(spec) => lower_elementwise(&mut builder, kernel, spec, tensors)?,
		PrimitiveKind::Reduce(spec) => lower_reduce(&mut builder, kernel, spec, tensors)?,
		PrimitiveKind::Scan(spec) => lower_scan(&mut builder, kernel, spec, tensors)?,
		PrimitiveKind::Contraction(spec) => {
			lower_contraction(&mut builder, kernel, spec, tensors)?;
		}
		PrimitiveKind::Gather(spec) => lower_gather(&mut builder, kernel, *spec, tensors)?,
		PrimitiveKind::Scatter(spec) => lower_scatter(&mut builder, kernel, *spec, tensors)?,
		PrimitiveKind::Histogram(spec) => {
			lower_histogram(&mut builder, kernel, *spec, tensors)?;
		}
		PrimitiveKind::Sort(spec) => lower_sort(&mut builder, kernel, *spec, tensors)?,
		PrimitiveKind::IndexMap(spec) => lower_index_map(&mut builder, kernel, *spec, tensors)?,
		PrimitiveKind::Random(spec) => lower_random(&mut builder, kernel, *spec, tensors)?,
	}

	builder.finish(
		kernel.alias_rules
			.iter()
			.map(|rule| SourceAliasContract {
				input: rule.input,
				output: rule.output,
				permission: rule.permission,
			})
			.collect(),
		kernel.inputs.len(),
		kernel.outputs.len(),
	)
}

fn validate_hardware(kernel: &PrimitiveKernel, hardware: LoweringHardware) -> LoweringResult<()> {
	let valid = hardware.subgroup_lanes != 0
		&& hardware.subgroup_lanes.is_power_of_two()
		&& hardware.maximum_workgroup_lanes >= hardware.subgroup_lanes
		&& hardware.maximum_shared_memory_per_workgroup.get() != 0;
	if valid {
		Ok(())
	} else {
		Err(LoweringError::new(
			LoweringErrorKind::InvalidLoweredProgram,
			format!("invalid measured lowering hardware limits: {hardware:?}"),
		)
		.for_kernel(kernel.id))
	}
}

fn tensor<'a>(
	tensors: &'a BTreeMap<ValueId, &Tensor>,
	value: ValueId,
	kernel: recipe_core::KernelTemplateId,
) -> LoweringResult<&'a Tensor> {
	tensors.get(&value).copied().ok_or_else(|| {
		LoweringError::new(
			LoweringErrorKind::MissingTensor,
			format!("tensor {value} is absent from the lowering index"),
		)
		.for_kernel(kernel)
		.for_value(value)
	})
}

#[derive(Debug)]
struct ProgramBuilder {
	kernel: recipe_core::KernelTemplateId,
	hardware: LoweringHardware,
	buffers: Vec<ProgramBuffer>,
	tensor_buffers: BTreeMap<ValueId, BufferId>,
	stages: Vec<ProgramStage>,
	scratch_ordinal: u32,
	fault_buffer: Option<BufferId>,
}

impl ProgramBuilder {
	fn new(kernel: recipe_core::KernelTemplateId, hardware: LoweringHardware) -> Self {
		Self {
			kernel,
			hardware,
			buffers: Vec::new(),
			tensor_buffers: BTreeMap::new(),
			stages: Vec::new(),
			scratch_ordinal: 0,
			fault_buffer: None,
		}
	}

	fn workgroup_lanes(&self, logical_lanes: u64) -> u32 {
		let logical = u32::try_from(logical_lanes).unwrap_or(u32::MAX).max(1);
		floor_power_of_two(logical.min(self.hardware.maximum_workgroup_lanes))
	}

	fn collective_lanes(&self, logical_width: u64, payload_words: u64, declared_maximum: u32) -> u32 {
		let bytes_per_lane = payload_words.saturating_mul(4).max(1);
		let shared_lanes = self.hardware.maximum_shared_memory_per_workgroup.get() / bytes_per_lane;
		let shared_lanes = u32::try_from(shared_lanes).unwrap_or(u32::MAX).max(1);
		let logical = u32::try_from(logical_width).unwrap_or(u32::MAX).max(1);
		floor_power_of_two(
			logical
				.min(declared_maximum)
				.min(self.hardware.maximum_workgroup_lanes)
				.min(shared_lanes),
		)
	}

	fn add_tensor(&mut self, tensor: &Tensor) -> LoweringResult<BufferId> {
		match self.tensor_buffers.get(&tensor.id).copied() {
			Some(buffer) => Ok(buffer),
			None => {
				let id = self.next_buffer_id()?;
				let access = StaticAccess {
					logical_extents: tensor.shape.extents().to_vec(),
					offset_elements: tensor.layout.offset_elements,
					strides: tensor.layout.strides.clone(),
					storage_bytes: tensor.storage_bytes,
				};
				self.buffers.push(ProgramBuffer {
					id,
					origin: BufferOrigin::Tensor(tensor.id),
					lifetime: BufferLifetime::ExternalValue,
					dtype: tensor.dtype,
					shape: tensor.shape.extents().to_vec(),
					access,
				});
				self.tensor_buffers.insert(tensor.id, id);
				Ok(id)
			}
		}
	}

	fn tensor_buffer(&self, value: ValueId) -> LoweringResult<BufferId> {
		self.tensor_buffers.get(&value).copied().ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::MissingTensor,
				format!("tensor {value} has no program buffer"),
			)
			.for_kernel(self.kernel)
			.for_value(value)
		})
	}

	fn scratch(&mut self, dtype: DType, elements: u64, purpose: ScratchPurpose) -> LoweringResult<BufferId> {
		let bytes = elements
			.checked_mul(u64::from(dtype.byte_width()))
			.ok_or_else(|| self.overflow("scratch byte size"))?;
		let id = self.next_buffer_id()?;
		let ordinal = self.scratch_ordinal;
		self.scratch_ordinal = self
			.scratch_ordinal
			.checked_add(1)
			.ok_or_else(|| self.overflow("scratch ordinal"))?;
		self.buffers.push(ProgramBuffer {
			id,
			origin: BufferOrigin::Scratch { ordinal, purpose },
			lifetime: BufferLifetime::ProgramScratch,
			dtype,
			shape: vec![elements],
			access: StaticAccess {
				logical_extents: vec![elements],
				offset_elements: 0,
				strides: vec![1],
				storage_bytes: ByteCount::new(bytes),
			},
		});
		Ok(id)
	}

	fn fault(&mut self) -> LoweringResult<BufferId> {
		match self.fault_buffer {
			Some(buffer) => Ok(buffer),
			None => {
				let id = self.next_buffer_id()?;
				self.buffers.push(ProgramBuffer {
					id,
					origin: BufferOrigin::FaultFlag,
					lifetime: BufferLifetime::ProgramFaultFlag,
					dtype: DType::I32,
					shape: vec![1],
					access: StaticAccess {
						logical_extents: vec![1],
						offset_elements: 0,
						strides: vec![1],
						storage_bytes: ByteCount::new(4),
					},
				});
				self.fault_buffer = Some(id);
				Ok(id)
			}
		}
	}

	fn buffer(&self, id: BufferId) -> LoweringResult<&ProgramBuffer> {
		let index = usize::try_from(id.get()).map_err(|error| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				format!("buffer index conversion failed: {error}"),
			)
			.for_kernel(self.kernel)
		})?;
		self.buffers.get(index).ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::InvalidLoweredProgram,
				format!("buffer {} is absent", id.get()),
			)
			.for_kernel(self.kernel)
		})
	}

	fn binding(&self, id: BufferId, mode: AccessMode) -> LoweringResult<BufferBinding> {
		let buffer = self.buffer(id)?;
		Ok(BufferBinding {
			buffer: id,
			dtype: buffer.dtype,
			mode,
			view: buffer.access.clone(),
		})
	}

	fn checked_fault(
		&mut self,
		reason: FaultReason,
		code: i32,
	) -> LoweringResult<(FaultContract, BufferBinding, AtomicContract)> {
		let flag = self.fault()?;
		let publish = AtomicContract {
			buffer: flag,
			dtype: DType::I32,
			operation: OwnedAtomicOperation::Exchange,
			ordering: OwnedAtomicOrdering::Release,
			domain: AtomicAddressDomain::SingleFaultFlag,
		};
		let contract = FaultContract {
			flag,
			reason,
			code,
			guard_before_address: true,
			publish,
		};
		Ok((
			contract,
			self.binding(flag, AccessMode::ReadWriteAtomic)?,
			publish,
		))
	}

	fn push_stage(&mut self, draft: StageDraft) -> LoweringResult<StageId> {
		let StageDraft {
			geometry,
			bindings,
			synchronization,
			atomics,
			fault,
			resources,
			kind,
		} = draft;
		let id = StageId::new(u32::try_from(self.stages.len()).map_err(|error| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				format!("stage identifier conversion failed: {error}"),
			)
			.for_kernel(self.kernel)
		})?);
		let dependencies = match self.stages.last() {
			Some(stage) => vec![stage.id],
			None => Vec::with_capacity(0),
		};
		self.stages.push(ProgramStage {
			id,
			dependencies,
			geometry,
			bindings,
			synchronization,
			atomics,
			fault,
			resources,
			kind,
		});
		Ok(id)
	}

	fn next_buffer_id(&self) -> LoweringResult<BufferId> {
		u32::try_from(self.buffers.len())
			.map(BufferId::new)
			.map_err(|error| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					format!("buffer identifier conversion failed: {error}"),
				)
				.for_kernel(self.kernel)
			})
	}

	fn overflow(&self, subject: &str) -> LoweringError {
		LoweringError::new(
			LoweringErrorKind::ArithmeticOverflow,
			format!("{subject} overflowed its static representation"),
		)
		.for_kernel(self.kernel)
	}

	fn finish(
		self,
		source_aliases: Vec<SourceAliasContract>,
		source_input_count: usize,
		source_output_count: usize,
	) -> LoweringResult<LoweredProgram> {
		let resources = aggregate_resources(&self.buffers, &self.stages, self.kernel)?;
		let mut program = LoweredProgram {
			schema_version: LOWERED_PROGRAM_SCHEMA_VERSION,
			source_kernel: self.kernel,
			source_input_count,
			source_output_count,
			source_aliases,
			buffers: self.buffers,
			stages: self.stages,
			resources,
			digest: ProgramDigest::ZERO,
		};
		program.digest = program.canonical_digest();
		program.validate().map_err(|errors| {
			LoweringError::new(
				LoweringErrorKind::InvalidLoweredProgram,
				errors.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>()
					.join("; "),
			)
			.for_kernel(program.source_kernel)
		})?;
		Ok(program)
	}
}

fn floor_power_of_two(value: u32) -> u32 {
	1_u32 << (31 - value.max(1).leading_zeros())
}

fn aggregate_resources(
	buffers: &[ProgramBuffer],
	stages: &[ProgramStage],
	kernel: recipe_core::KernelTemplateId,
) -> LoweringResult<ProgramResourceBounds> {
	let mut total_flops = 0_u64;
	let mut total_integer_operations = 0_u64;
	let mut total_atomic_operations = 0_u64;
	let mut peak_shared = 0_u64;
	let mut peak_private = 0_u64;
	let mut maximum_workgroup_lanes = 0_u32;
	for stage in stages {
		total_flops = total_flops
			.checked_add(stage.resources.flops.get())
			.ok_or_else(|| overflow(kernel, "total FLOP bound"))?;
		total_integer_operations = total_integer_operations
			.checked_add(stage.resources.integer_operations)
			.ok_or_else(|| overflow(kernel, "total integer-operation bound"))?;
		total_atomic_operations = total_atomic_operations
			.checked_add(stage.resources.atomic_operations)
			.ok_or_else(|| overflow(kernel, "total atomic-operation bound"))?;
		peak_shared = peak_shared.max(stage.resources.shared_bytes_per_workgroup.get());
		peak_private = peak_private.max(stage.resources.private_bytes_per_lane.get());
		maximum_workgroup_lanes = maximum_workgroup_lanes.max(stage.resources.maximum_workgroup_lanes);
	}
	let mut persistent_scratch_bytes = 0_u64;
	let mut fault_bytes = 0_u64;
	for buffer in buffers {
		match buffer.lifetime {
			BufferLifetime::ProgramScratch => {
				persistent_scratch_bytes = persistent_scratch_bytes
					.checked_add(buffer.access.storage_bytes.get())
					.ok_or_else(|| overflow(kernel, "persistent scratch bound"))?;
			}
			BufferLifetime::ProgramFaultFlag => {
				fault_bytes = fault_bytes
					.checked_add(buffer.access.storage_bytes.get())
					.ok_or_else(|| overflow(kernel, "fault bound"))?;
			}
			BufferLifetime::ExternalValue => continue,
		}
	}
	Ok(ProgramResourceBounds {
		total_flops: FlopCount::new(total_flops),
		total_integer_operations,
		total_atomic_operations,
		persistent_scratch_bytes: ByteCount::new(persistent_scratch_bytes),
		fault_bytes: ByteCount::new(fault_bytes),
		peak_shared_bytes_per_workgroup: ByteCount::new(peak_shared),
		peak_private_bytes_per_lane: ByteCount::new(peak_private),
		maximum_workgroup_lanes,
	})
}

fn overflow(kernel: recipe_core::KernelTemplateId, subject: &str) -> LoweringError {
	LoweringError::new(
		LoweringErrorKind::ArithmeticOverflow,
		format!("{subject} overflowed its static representation"),
	)
	.for_kernel(kernel)
}

fn geometry(logical_lanes: u64, workgroup_lanes: u32) -> DispatchGeometry {
	let lanes = u64::from(workgroup_lanes);
	DispatchGeometry {
		logical_lanes,
		workgroup_lanes,
		workgroups: logical_lanes.div_ceil(lanes),
	}
}

fn basic_resources(
	flops: u64,
	integer_operations: u64,
	atomic_operations: u64,
	shared_bytes: u64,
	private_bytes: u64,
	workgroup_lanes: u32,
) -> StageResourceBounds {
	StageResourceBounds {
		flops: FlopCount::new(flops),
		integer_operations,
		atomic_operations,
		shared_bytes_per_workgroup: ByteCount::new(shared_bytes),
		private_bytes_per_lane: ByteCount::new(private_bytes),
		maximum_workgroup_lanes: workgroup_lanes,
	}
}

fn lower_elementwise(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: &Elementwise,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let false = output.shape.is_empty() else {
		return Ok(());
	};
	let index_space = IndexSpace::new(
		output.shape
			.extents()
			.iter()
			.copied()
			.map(ElementCount::new)
			.collect::<Result<Vec<_>, _>>()
			.map_err(|error| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					format!("elementwise index extent is invalid: {error}"),
				)
				.for_kernel(kernel.id)
			})?,
	)
	.map_err(|error| {
		LoweringError::new(
			LoweringErrorKind::ArithmeticOverflow,
			format!("elementwise index space is invalid: {error}"),
		)
		.for_kernel(kernel.id)
	})?;

	let mut core_inputs = Vec::with_capacity(kernel.inputs.len());
	for (index, value) in kernel.inputs.iter().copied().enumerate() {
		let tensor = tensor(tensors, value, kernel.id)?;
		core_inputs.push(KernelInput {
			id: KernelInputId::new(u64::try_from(index + 1).map_err(|error| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					format!("kernel input identifier conversion failed: {error}"),
				)
				.for_kernel(kernel.id)
			})?),
			dtype: tensor.dtype,
			access: broadcast_access(tensor, output.shape.extents())?,
		});
	}
	let mut core_outputs = Vec::with_capacity(kernel.outputs.len());
	for (index, value) in kernel.outputs.iter().copied().enumerate() {
		let tensor = tensor(tensors, value, kernel.id)?;
		core_outputs.push(KernelOutput {
			id: KernelOutputId::new(u64::try_from(index + 1).map_err(|error| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					format!("kernel output identifier conversion failed: {error}"),
				)
				.for_kernel(kernel.id)
			})?),
			dtype: tensor.dtype,
			access: StaticBufferAccess {
				offset_elements: tensor.layout.offset_elements,
				strides: tensor.layout.strides.clone(),
				storage_bytes: tensor.storage_bytes,
			},
		});
	}
	let aliases = kernel
		.alias_rules
		.iter()
		.map(|rule| AliasRule {
			input: core_inputs[rule.input].id,
			output: core_outputs[rule.output].id,
			permission: rule.permission,
		})
		.collect();
	let template = KernelTemplate {
		id: kernel.id,
		index_space,
		inputs: core_inputs,
		outputs: core_outputs,
		program: spec.program.clone(),
		alias_rules: aliases,
	};

	let mut bindings = Vec::new();
	for value in &kernel.inputs {
		bindings.push(builder.binding(builder.tensor_buffer(*value)?, AccessMode::Read)?);
	}
	for value in &kernel.outputs {
		bindings.push(builder.binding(builder.tensor_buffer(*value)?, AccessMode::Write)?);
	}
	let mut atomics = Vec::new();
	let fault = match spec.program.requires_fault_flag() {
		true => {
			let (fault, binding, atomic) = builder.checked_fault(FaultReason::ArithmeticDomain, 2)?;
			bindings.push(binding);
			atomics.push(atomic);
			Some(fault)
		}
		false => None,
	};
	let per_lane_flops = spec
		.program
		.instructions
		.iter()
		.try_fold(0_u64, |total, instruction| {
			total.checked_add(instruction.opcode.flops())
		})
		.ok_or_else(|| builder.overflow("elementwise FLOP count"))?;
	let flops = per_lane_flops
		.checked_mul(output.shape.elements())
		.ok_or_else(|| builder.overflow("elementwise FLOP count"))?;
	let scalar_slots = spec.program.inputs.len() + spec.program.constants.len() + spec.program.instructions.len();
	let private_bytes = u64::try_from(scalar_slots)
		.map_err(|error| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				format!("elementwise scalar-slot conversion failed: {error}"),
			)
			.for_kernel(kernel.id)
		})?
		.checked_mul(4)
		.ok_or_else(|| builder.overflow("elementwise private bound"))?;
	let atomic_bound = u64::from(fault.is_some()).saturating_mul(output.shape.elements());
	let workgroup_lanes = builder.workgroup_lanes(output.shape.elements());
	push_stage!(
		builder,
		geometry(output.shape.elements(), workgroup_lanes),
		bindings,
		Vec::new(),
		atomics,
		fault,
		basic_resources(
			flops,
			output.shape.elements(),
			atomic_bound,
			0,
			private_bytes,
			workgroup_lanes,
		),
		StageKind::ScalarMap { template },
	)?;
	Ok(())
}

fn broadcast_access(tensor: &Tensor, output_extents: &[u64]) -> LoweringResult<StaticBufferAccess> {
	let leading = output_extents
		.len()
		.checked_sub(tensor.shape.rank())
		.ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::InvalidStaticAccess,
				"broadcast input rank exceeds output rank",
			)
			.for_value(tensor.id)
		})?;
	let mut strides = vec![0_u64; output_extents.len()];
	for (input_axis, extent) in tensor.shape.extents().iter().copied().enumerate() {
		let output_axis = leading + input_axis;
		strides[output_axis] = match extent == 1 && output_extents[output_axis] != 1 {
			true => 0,
			false => tensor.layout.strides[input_axis],
		};
	}
	Ok(StaticBufferAccess {
		offset_elements: tensor.layout.offset_elements,
		strides,
		storage_bytes: tensor.storage_bytes,
	})
}

fn lower_reduce(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: &Reduce,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let input = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let reduced_width = spec.axes.as_slice().iter().try_fold(1_u64, |value, axis| {
		value.checked_mul(input.shape.extents()[*axis])
	});
	let reduced_width = reduced_width.ok_or_else(|| builder.overflow("reduction width"))?;
	let output_elements = tensor(tensors, kernel.outputs[0], kernel.id)?
		.shape
		.elements();
	let false = output_elements == 0 else {
		return Ok(());
	};
	let false = reduced_width == 0 else {
		return lower_empty_reduction(builder, kernel, spec, tensors);
	};

	let track_index = spec.result != ReduceResult::Value;
	let payload_words = 1_u64 + u64::from(track_index);
	let lanes = builder.collective_lanes(reduced_width, payload_words, spec.tree_lanes);
	let tree = reduction_tree(lanes);
	let mut current_value = builder.tensor_buffer(kernel.inputs[0])?;
	let mut current_index = None;
	let pass_widths = std::iter::successors(Some(reduced_width), |current_width| {
		let output_width = current_width.div_ceil(u64::from(lanes));
		(output_width > 1).then_some(output_width)
	});
	for (pass_index, current_width) in pass_widths.enumerate() {
		let pass = u32::try_from(pass_index).map_err(|error| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				format!("reduction pass conversion failed: {error}"),
			)
			.for_kernel(kernel.id)
		})?;
		let output_width = current_width.div_ceil(u64::from(lanes));
		let final_pass = output_width == 1;
		let value_output = match (final_pass, spec.result) {
			(true, ReduceResult::Value | ReduceResult::ValueAndIndex) => {
				Some(builder.tensor_buffer(kernel.outputs[0])?)
			}
			(true, ReduceResult::Index) => None,
			(false, _) => Some(builder.scratch(
				input.dtype,
				checked_mul(
					builder,
					output_elements,
					output_width,
					"reduction value scratch",
				)?,
				ScratchPurpose::ReductionValues,
			)?),
		};
		let index_output = match (final_pass, spec.result) {
			(true, ReduceResult::Index) => Some(builder.tensor_buffer(kernel.outputs[0])?),
			(true, ReduceResult::ValueAndIndex) => Some(builder.tensor_buffer(kernel.outputs[1])?),
			(true, ReduceResult::Value) => None,
			(false, ReduceResult::Index | ReduceResult::ValueAndIndex) => Some(builder.scratch(
				DType::I32,
				checked_mul(
					builder,
					output_elements,
					output_width,
					"reduction index scratch",
				)?,
				ScratchPurpose::ReductionIndices,
			)?),
			(false, ReduceResult::Value) => None,
		};
		let mut bindings = vec![builder.binding(current_value, AccessMode::Read)?];
		let current_index_binding = current_index
			.map(|index| builder.binding(index, AccessMode::Read))
			.transpose()?;
		bindings.extend(current_index_binding);
		let value_output_binding = value_output
			.map(|value| builder.binding(value, AccessMode::Write))
			.transpose()?;
		bindings.extend(value_output_binding);
		let index_output_binding = index_output
			.map(|index| builder.binding(index, AccessMode::Write))
			.transpose()?;
		bindings.extend(index_output_binding);
		let groups = checked_mul(
			builder,
			output_elements,
			output_width,
			"reduction group count",
		)?;
		let logical_lanes = checked_mul(builder, groups, u64::from(lanes), "reduction lanes")?;
		let combines = checked_mul(
			builder,
			groups,
			u64::from(lanes - 1),
			"reduction combine count",
		)?;
		let flop_multiplier = match spec.result {
			ReduceResult::ValueAndIndex => 2,
			ReduceResult::Value | ReduceResult::Index => 1,
		};
		let flops = checked_mul(builder, combines, flop_multiplier, "reduction FLOP count")?;
		let shared = checked_mul(
			builder,
			checked_mul(
				builder,
				u64::from(lanes),
				payload_words,
				"reduction shared words",
			)?,
			4,
			"reduction shared bytes",
		)?;
		push_stage!(
			builder,
			DispatchGeometry {
				logical_lanes,
				workgroup_lanes: lanes,
				workgroups: groups,
			},
			bindings,
			tree_synchronization(&tree),
			Vec::new(),
			None,
			basic_resources(flops, logical_lanes, 0, shared, payload_words * 8, lanes),
			StageKind::FixedTreeReduce(ReductionStage {
				pass,
				operator: spec.operator,
				result: spec.result,
				dtype: input.dtype,
				sequences: output_elements,
				input_width: current_width,
				output_width,
				reduced_axes: spec.axes.as_slice().to_vec(),
				tree: tree.clone(),
				padding: ReductionPadding::OperatorIdentity,
				tie_break: ReductionTieBreak::LowestLogicalIndex,
			}),
		)?;
		let false = final_pass else {
			continue;
		};
		current_value = value_output.ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::InvalidLoweredProgram,
				"non-final reduction omitted its value scratch",
			)
			.for_kernel(kernel.id)
		})?;
		current_index = index_output;
	}
	Ok(())
}

fn lower_empty_reduction(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: &Reduce,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let dtype = tensor(tensors, kernel.inputs[0], kernel.id)?.dtype;
	let identity = match (spec.operator, dtype) {
		(ReduceOperator::Sum | ReduceOperator::Any, DType::F32) => ScalarLiteral::F32Bits(0),
		(ReduceOperator::Product | ReduceOperator::All, DType::F32) => ScalarLiteral::F32Bits(1.0_f32.to_bits()),
		(ReduceOperator::Sum | ReduceOperator::Any, DType::I32) => ScalarLiteral::I32(0),
		(ReduceOperator::Product | ReduceOperator::All, DType::I32) => ScalarLiteral::I32(1),
		(ReduceOperator::Minimum | ReduceOperator::Maximum, _) => {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidLanguage,
				"empty minimum/maximum should have been rejected by the language",
			)
			.for_kernel(kernel.id));
		}
	};
	for output in &kernel.outputs {
		let tensor = tensor(tensors, *output, kernel.id)?;
		let false = tensor.shape.elements() == 0 else {
			continue;
		};
		let literal = match tensor.dtype == DType::I32 && spec.result == ReduceResult::Index {
			true => ScalarLiteral::I32(0),
			false => identity,
		};
		let binding = builder.binding(builder.tensor_buffer(*output)?, AccessMode::Write)?;
		let workgroup_lanes = builder.workgroup_lanes(tensor.shape.elements());
		push_stage!(
			builder,
			geometry(tensor.shape.elements(), workgroup_lanes),
			vec![binding],
			Vec::new(),
			Vec::new(),
			None,
			basic_resources(0, tensor.shape.elements(), 0, 0, 4, workgroup_lanes),
			StageKind::Fill { value: literal },
		)?;
	}
	Ok(())
}

fn reduction_tree(lanes: u32) -> FixedTree {
	let steps = (0..lanes.trailing_zeros())
		.map(|level| TreeStep {
			phase: TreePhase::Reduction,
			stride: lanes >> (level + 1),
		})
		.collect();
	FixedTree { lanes, steps }
}

fn scan_tree(lanes: u32) -> FixedTree {
	let levels = lanes.trailing_zeros();
	let steps = (0..levels)
		.map(|level| TreeStep {
			phase: TreePhase::ScanUpsweep,
			stride: 1 << level,
		})
		.chain((0..levels).rev().map(|level| TreeStep {
			phase: TreePhase::ScanDownsweep,
			stride: 1 << level,
		}))
		.collect();
	FixedTree { lanes, steps }
}

fn tree_synchronization(tree: &FixedTree) -> Vec<SynchronizationPoint> {
	tree.steps
		.iter()
		.zip(0_u32..)
		.map(|(_, index)| SynchronizationPoint {
			after_step: index,
			scope: SynchronizationScope::Workgroup,
			memory: MemorySemantics::SharedAcquireRelease,
		})
		.collect()
}

#[derive(Clone, Copy, Debug)]
struct ScanLevel {
	level: u32,
	width: u64,
	output: BufferId,
}

fn lower_scan(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: &Scan,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let input = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let false = input.shape.is_empty() else {
		return Ok(());
	};
	let axis_width = input.shape.extents()[spec.axis];
	let false = axis_width == 0 else {
		return Ok(());
	};
	let sequences = input.shape.elements() / axis_width;
	let lanes = builder.collective_lanes(axis_width, 1, spec.tree_lanes);
	let tree = scan_tree(lanes);
	let mut current_input = builder.tensor_buffer(kernel.inputs[0])?;
	let mut levels = Vec::new();

	let hierarchy_widths = std::iter::successors(Some(axis_width), |current_width| {
		let blocks = current_width.div_ceil(u64::from(lanes));
		(blocks > 1).then_some(blocks)
	});
	for (level_index, current_width) in hierarchy_widths.enumerate() {
		let level = u32::try_from(level_index).map_err(|error| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				format!("scan hierarchy-level conversion failed: {error}"),
			)
			.for_kernel(kernel.id)
		})?;
		let blocks = current_width.div_ceil(u64::from(lanes));
		let output = match level == 0 {
			true => builder.tensor_buffer(kernel.outputs[0])?,
			false => builder.scratch(
				input.dtype,
				checked_mul(builder, sequences, current_width, "scan level output")?,
				ScratchPurpose::ScanValues,
			)?,
		};
		let totals = match blocks > 1 {
			true => Some(builder.scratch(
				input.dtype,
				checked_mul(builder, sequences, blocks, "scan block totals")?,
				ScratchPurpose::ScanBlockTotals,
			)?),
			false => None,
		};
		let mut bindings = vec![
			builder.binding(current_input, AccessMode::Read)?,
			builder.binding(output, AccessMode::Write)?,
		];
		let totals_binding = totals
			.map(|buffer| builder.binding(buffer, AccessMode::Write))
			.transpose()?;
		bindings.extend(totals_binding);
		let groups = checked_mul(builder, sequences, blocks, "scan workgroups")?;
		let logical_lanes = checked_mul(builder, groups, u64::from(lanes), "scan logical lanes")?;
		let tree_combines = checked_mul(
			builder,
			groups,
			u64::from(2 * (lanes - 1)),
			"scan tree combines",
		)?;
		let active = checked_mul(builder, sequences, current_width, "scan active elements")?;
		let mode = match level == 0 {
			true => match spec.mode {
				ScanMode::Inclusive => ScanStageMode::UserInclusive,
				ScanMode::Exclusive { identity } => ScanStageMode::UserExclusive { identity },
			},
			false => ScanStageMode::HierarchyExclusiveIdentity,
		};
		let inclusive_combines = u64::from(matches!(mode, ScanStageMode::UserInclusive))
			.checked_mul(active)
			.ok_or_else(|| builder.overflow("inclusive scan combine count"))?;
		let flops = tree_combines
			.checked_add(inclusive_combines)
			.ok_or_else(|| builder.overflow("scan FLOP count"))?;
		let shared = checked_mul(builder, u64::from(lanes), 4, "scan shared bytes")?;
		push_stage!(
			builder,
			DispatchGeometry {
				logical_lanes,
				workgroup_lanes: lanes,
				workgroups: groups,
			},
			bindings,
			tree_synchronization(&tree),
			Vec::new(),
			None,
			basic_resources(flops, logical_lanes, 0, shared, 12, lanes),
			StageKind::FixedTreeScanLocal(ScanLocalStage {
				level,
				operator: spec.operator,
				dtype: input.dtype,
				sequences,
				input_width: current_width,
				output_width: blocks,
				axis: spec.axis,
				reverse: spec.reverse,
				mode,
				tree: tree.clone(),
			}),
		)?;
		levels.push(ScanLevel {
			level,
			width: current_width,
			output,
		});
		current_input = match totals {
			Some(buffer) => buffer,
			None => output,
		};
	}

	for pair in levels.windows(2).rev() {
		let lower = pair[0];
		let upper = pair[1];
		let active_per_sequence = lower.width.saturating_sub(u64::from(lanes));
		let active = checked_mul(
			builder,
			sequences,
			active_per_sequence,
			"scan uniform combines",
		)?;
		let false = active == 0 else {
			continue;
		};
		let workgroup_lanes = builder.workgroup_lanes(active);
		push_stage!(
			builder,
			geometry(active, workgroup_lanes),
			vec![
				builder.binding(lower.output, AccessMode::ReadWrite)?,
				builder.binding(upper.output, AccessMode::Read)?,
			],
			Vec::new(),
			Vec::new(),
			None,
			basic_resources(active, active, 0, 0, 12, workgroup_lanes),
			StageKind::ScanUniformCombine(ScanUniformStage {
				level: lower.level,
				operator: spec.operator,
				dtype: input.dtype,
				sequences,
				width: lower.width,
				block_lanes: lanes,
				reverse: spec.reverse,
			}),
		)?;
	}
	Ok(())
}

fn lower_contraction(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: &recipe_language::Contraction,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let left = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let false = output.shape.is_empty() else {
		return Ok(());
	};
	let contracted_elements = spec
		.contract_axes
		.iter()
		.try_fold(1_u64, |value, (axis, _)| {
			value.checked_mul(left.shape.extents()[*axis])
		});
	let contracted_elements = contracted_elements.ok_or_else(|| builder.overflow("contraction reduction extent"))?;
	let output_products = checked_mul(
		builder,
		output.shape.elements(),
		contracted_elements,
		"contraction output products",
	)?;
	let flops = checked_mul(builder, output_products, 2, "contraction FLOP count")?;
	let physical = contraction_physical_plan(
		builder,
		left,
		spec,
		output.shape.elements(),
		contracted_elements,
	)?;
	let reduction_tiles = contracted_elements.div_ceil(u64::from(physical.tile.reduction));
	let synchronization_count = match physical.strategy {
		ContractionStrategy::Direct => 0,
		ContractionStrategy::Staged => checked_mul(
			builder,
			reduction_tiles,
			2,
			"contraction synchronization count",
		)?,
	};
	let synchronization_count = u32::try_from(synchronization_count).map_err(|error| {
		LoweringError::new(
			LoweringErrorKind::ArithmeticOverflow,
			format!("contraction synchronization conversion failed: {error}"),
		)
		.for_kernel(kernel.id)
	})?;
	let synchronization = (0..synchronization_count)
		.map(|after_step| SynchronizationPoint {
			after_step,
			scope: SynchronizationScope::Workgroup,
			memory: MemorySemantics::SharedAcquireRelease,
		})
		.collect();
	push_stage!(
		builder,
		geometry(output.shape.elements(), physical.workgroup_lanes),
		vec![
			builder.binding(builder.tensor_buffer(kernel.inputs[0])?, AccessMode::Read)?,
			builder.binding(builder.tensor_buffer(kernel.inputs[1])?, AccessMode::Read)?,
			builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write)?,
		],
		synchronization,
		Vec::new(),
		None,
		basic_resources(
			flops,
			output.shape.elements(),
			0,
			physical.shared_bytes,
			24,
			physical.workgroup_lanes,
		),
		StageKind::TiledContraction(ContractionStage {
			spec: spec.clone(),
			dtype: left.dtype,
			output_elements: output.shape.elements(),
			contracted_elements,
			strategy: physical.strategy,
			tile: physical.tile,
			canonical_contracted_order: true,
		}),
	)?;
	Ok(())
}

#[derive(Clone, Copy, Debug)]
struct ContractionPhysicalPlan {
	strategy: ContractionStrategy,
	tile: ContractionTile,
	workgroup_lanes: u32,
	shared_bytes: u64,
}

fn contraction_physical_plan(
	builder: &ProgramBuilder,
	left: &Tensor,
	spec: &recipe_language::Contraction,
	output_elements: u64,
	contracted_elements: u64,
) -> LoweringResult<ContractionPhysicalPlan> {
	let free_extent = left
		.shape
		.extents()
		.iter()
		.copied()
		.enumerate()
		.filter(|(axis, _)| {
			!spec.batch_axes.iter().any(|(left, _)| left == axis)
				&& !spec.contract_axes.iter().any(|(left, _)| left == axis)
		})
		.try_fold(1_u64, |product, (_, extent)| product.checked_mul(extent))
		.ok_or_else(|| builder.overflow("contraction free-axis extent"))?;
	let right_extent = output_elements
		.checked_div(free_extent.max(1))
		.unwrap_or(1)
		.max(1);
	let maximum = builder.workgroup_lanes(output_elements);
	let mut balanced = 1_u32;
	while balanced
		.checked_mul(2)
		.is_some_and(|next| next <= maximum / next)
	{
		balanced *= 2;
	}
	let left_capacity = floor_power_of_two(
		u32::try_from(free_extent)
			.unwrap_or(u32::MAX)
			.max(1)
			.min(maximum),
	);
	let right_capacity = floor_power_of_two(
		u32::try_from(right_extent)
			.unwrap_or(u32::MAX)
			.max(1)
			.min(maximum),
	);
	let mut output_x = left_capacity.min(balanced).max(1);
	let mut output_y = right_capacity.min(maximum / output_x).max(1);
	while output_x < left_capacity && output_x <= maximum / 2 / output_y {
		output_x *= 2;
	}
	while output_y < right_capacity && output_y <= maximum / 2 / output_x {
		output_y *= 2;
	}
	let workgroup_lanes = output_x
		.checked_mul(output_y)
		.ok_or_else(|| builder.overflow("contraction workgroup width"))?;
	let reduction = floor_power_of_two(
		u32::try_from(contracted_elements)
			.unwrap_or(u32::MAX)
			.max(1)
			.min(builder.hardware.subgroup_lanes),
	);
	let staged_bytes = u64::from(workgroup_lanes)
		.checked_mul(u64::from(left.dtype.byte_width()))
		.ok_or_else(|| builder.overflow("contraction staged accumulator bytes"))?;
	let strategy = if workgroup_lanes >= builder.hardware.subgroup_lanes
		&& contracted_elements >= u64::from(builder.hardware.subgroup_lanes)
		&& staged_bytes <= builder.hardware.maximum_shared_memory_per_workgroup.get()
	{
		ContractionStrategy::Staged
	} else {
		ContractionStrategy::Direct
	};
	Ok(ContractionPhysicalPlan {
		strategy,
		tile: ContractionTile {
			output_x,
			output_y,
			reduction,
		},
		workgroup_lanes,
		shared_bytes: match strategy {
			ContractionStrategy::Direct => 0,
			ContractionStrategy::Staged => staged_bytes,
		},
	})
}

fn lower_gather(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: Gather,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let false = output.shape.is_empty() else {
		return Ok(());
	};
	let mut bindings = vec![
		builder.binding(builder.tensor_buffer(kernel.inputs[0])?, AccessMode::Read)?,
		builder.binding(builder.tensor_buffer(kernel.inputs[1])?, AccessMode::Read)?,
		builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write)?,
	];
	let mut atomics = Vec::new();
	let fault = match spec.bounds == IndexBounds::Reject {
		true => {
			let (fault, binding, atomic) = builder.checked_fault(FaultReason::IndexOutOfBounds, 1)?;
			bindings.push(binding);
			atomics.push(atomic);
			Some(fault)
		}
		false => None,
	};
	let atomic_bound = u64::from(fault.is_some()).saturating_mul(output.shape.elements());
	let workgroup_lanes = builder.workgroup_lanes(output.shape.elements());
	push_stage!(
		builder,
		geometry(output.shape.elements(), workgroup_lanes),
		bindings,
		Vec::new(),
		atomics,
		fault,
		basic_resources(
			0,
			output.shape.elements(),
			atomic_bound,
			0,
			16,
			workgroup_lanes,
		),
		StageKind::Gather {
			axis: spec.axis,
			bounds: spec.bounds,
		},
	)?;
	Ok(())
}

fn lower_scatter(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: Scatter,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let base = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let updates = tensor(tensors, kernel.inputs[2], kernel.id)?;
	let copy_lanes = builder.workgroup_lanes(base.shape.elements());
	let copy_stage = (!base.shape.is_empty())
		.then(|| {
			push_stage!(
				builder,
				geometry(base.shape.elements(), copy_lanes),
				vec![
					builder.binding(builder.tensor_buffer(kernel.inputs[0])?, AccessMode::Read)?,
					builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write)?,
				],
				Vec::new(),
				Vec::new(),
				None,
				basic_resources(0, base.shape.elements(), 0, 0, 8, copy_lanes,),
				StageKind::Copy {
					reason: CopyReason::ScatterBase,
				},
			)
		})
		.transpose()?;
	debug_assert_eq!(copy_stage.is_some(), !base.shape.is_empty());
	let false = updates.shape.is_empty() else {
		return Ok(());
	};
	let output_mode = match spec.conflict {
		ScatterConflict::UniqueIndices => AccessMode::Write,
		ScatterConflict::Atomic { .. } => AccessMode::ReadWriteAtomic,
	};
	let output_buffer = builder.tensor_buffer(kernel.outputs[0])?;
	let mut bindings = vec![
		builder.binding(builder.tensor_buffer(kernel.inputs[1])?, AccessMode::Read)?,
		builder.binding(builder.tensor_buffer(kernel.inputs[2])?, AccessMode::Read)?,
		builder.binding(output_buffer, output_mode)?,
	];
	let mut atomics = Vec::new();
	let payload_atomic = match spec.conflict {
		ScatterConflict::UniqueIndices => None,
		ScatterConflict::Atomic {
			operation,
			ordering,
		} => Some(AtomicContract {
			buffer: output_buffer,
			dtype: base.dtype,
			operation: operation.into(),
			ordering: ordering.into(),
			domain: AtomicAddressDomain::TensorElements,
		}),
	};
	atomics.extend(payload_atomic);
	let fault = match spec.bounds == IndexBounds::Reject {
		true => {
			let (fault, binding, atomic) = builder.checked_fault(FaultReason::IndexOutOfBounds, 1)?;
			bindings.push(binding);
			atomics.push(atomic);
			Some(fault)
		}
		false => None,
	};
	let payload_atomic_count = u64::from(payload_atomic.is_some()).saturating_mul(updates.shape.elements());
	let fault_atomic_count = u64::from(fault.is_some()).saturating_mul(updates.shape.elements());
	let flops_per_atomic = match spec.conflict {
		ScatterConflict::Atomic {
			operation:
				recipe_language::AtomicOperation::Add
				| recipe_language::AtomicOperation::Minimum
				| recipe_language::AtomicOperation::Maximum,
			..
		} => 1,
		ScatterConflict::Atomic {
			operation: recipe_language::AtomicOperation::Exchange,
			..
		}
		| ScatterConflict::UniqueIndices => 0,
	};
	let flops = updates
		.shape
		.elements()
		.checked_mul(flops_per_atomic)
		.ok_or_else(|| builder.overflow("scatter FLOP count"))?;
	let workgroup_lanes = builder.workgroup_lanes(updates.shape.elements());
	push_stage!(
		builder,
		geometry(updates.shape.elements(), workgroup_lanes),
		bindings,
		Vec::new(),
		atomics,
		fault,
		basic_resources(
			flops,
			updates.shape.elements(),
			payload_atomic_count.saturating_add(fault_atomic_count),
			0,
			16,
			workgroup_lanes,
		),
		StageKind::Scatter {
			axis: spec.axis,
			bounds: spec.bounds,
			conflict: spec.conflict,
		},
	)?;
	Ok(())
}

fn lower_histogram(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: Histogram,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let input = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let output_buffer = builder.tensor_buffer(kernel.outputs[0])?;
	let clear_lanes = builder.workgroup_lanes(output.shape.elements());
	push_stage!(
		builder,
		geometry(output.shape.elements(), clear_lanes),
		vec![builder.binding(output_buffer, AccessMode::Write)?],
		Vec::new(),
		Vec::new(),
		None,
		basic_resources(0, output.shape.elements(), 0, 0, 4, clear_lanes,),
		StageKind::HistogramClear,
	)?;
	let false = input.shape.is_empty() else {
		return Ok(());
	};
	let mapping = match input.dtype {
		DType::I32 => HistogramBinMapping::I32Direct,
		DType::F32 => HistogramBinMapping::F32TruncateTowardZero,
	};
	let atomic = AtomicContract {
		buffer: output_buffer,
		dtype: output.dtype,
		operation: OwnedAtomicOperation::Add,
		ordering: spec.ordering.into(),
		domain: AtomicAddressDomain::HistogramBins,
	};
	let (fault, fault_binding, fault_atomic) = builder.checked_fault(FaultReason::HistogramBinOutOfBounds, 3)?;
	let mut bindings = vec![
		builder.binding(builder.tensor_buffer(kernel.inputs[0])?, AccessMode::Read)?,
		builder.binding(output_buffer, AccessMode::ReadWriteAtomic)?,
	];
	let weight_binding = spec
		.weighted
		.then(|| {
			let buffer = builder.tensor_buffer(kernel.inputs[1])?;
			builder.binding(buffer, AccessMode::Read)
		})
		.transpose()?;
	bindings.extend(weight_binding);
	bindings.push(fault_binding);
	let accumulate_lanes = builder.workgroup_lanes(input.shape.elements());
	push_stage!(
		builder,
		geometry(input.shape.elements(), accumulate_lanes),
		bindings,
		Vec::new(),
		vec![atomic, fault_atomic],
		Some(fault),
		basic_resources(
			input.shape.elements(),
			input.shape.elements(),
			input.shape.elements().saturating_mul(2),
			0,
			16,
			accumulate_lanes,
		),
		StageKind::HistogramAccumulate {
			bins: spec.bins,
			weighted: spec.weighted,
			bin_mapping: mapping,
			ordering: spec.ordering.into(),
		},
	)?;
	Ok(())
}

fn lower_sort(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: Sort,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let input = tensor(tensors, kernel.inputs[0], kernel.id)?;
	let false = input.shape.is_empty() else {
		return Ok(());
	};
	let axis_length = input.shape.extents()[spec.axis];
	let false = axis_length == 0 else {
		return Ok(());
	};
	let padded = axis_length
		.checked_next_power_of_two()
		.ok_or_else(|| builder.overflow("sort padded axis"))?;
	let slices = input.shape.elements() / axis_length;
	let scratch_elements = checked_mul(builder, slices, padded, "sort scratch elements")?;
	let values = builder.scratch(input.dtype, scratch_elements, ScratchPurpose::SortValues)?;
	let indices = builder.scratch(DType::I32, scratch_elements, ScratchPurpose::SortIndices)?;
	let network = SortNetwork {
		axis: spec.axis,
		axis_length,
		padded_axis_length: padded,
		slices,
		direction: spec.direction,
		requested_stable: spec.stable,
		tie_break: SortTieBreak::OriginalAxisIndexAscending,
		float_order: FloatSortOrder::Ieee754TotalOrder,
		padding: SortPadding::AfterAllValidElements,
	};
	let initialize_lanes = builder.workgroup_lanes(scratch_elements);
	push_stage!(
		builder,
		geometry(scratch_elements, initialize_lanes),
		vec![
			builder.binding(builder.tensor_buffer(kernel.inputs[0])?, AccessMode::Read)?,
			builder.binding(values, AccessMode::Write)?,
			builder.binding(indices, AccessMode::Write)?,
		],
		Vec::new(),
		Vec::new(),
		None,
		basic_resources(0, scratch_elements, 0, 0, 16, initialize_lanes,),
		StageKind::StableSortInitialize { network },
	)?;

	let comparisons = scratch_elements / 2;
	let comparison_lanes = builder.workgroup_lanes(comparisons);
	let network_levels = padded.trailing_zeros();
	for merge_level in 1..=network_levels {
		let merge_width = 1_u64 << merge_level;
		for distance_level in (0..merge_level).rev() {
			let distance = 1_u64 << distance_level;
			push_stage!(
				builder,
				geometry(comparisons, comparison_lanes),
				vec![
					builder.binding(values, AccessMode::ReadWrite)?,
					builder.binding(indices, AccessMode::ReadWrite)?,
				],
				Vec::new(),
				Vec::new(),
				None,
				basic_resources(comparisons, comparisons, 0, 0, 20, comparison_lanes,),
				StageKind::StableSortCompareExchange(SortCompareStage {
					network,
					merge_width,
					compare_distance: distance,
				}),
			)?;
		}
	}
	let mut bindings = vec![
		builder.binding(values, AccessMode::Read)?,
		builder.binding(indices, AccessMode::Read)?,
		builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write)?,
	];
	let index_output_binding = spec
		.emit_indices
		.then(|| {
			let buffer = builder.tensor_buffer(kernel.outputs[1])?;
			builder.binding(buffer, AccessMode::Write)
		})
		.transpose()?;
	bindings.extend(index_output_binding);
	let finalize_lanes = builder.workgroup_lanes(input.shape.elements());
	push_stage!(
		builder,
		geometry(input.shape.elements(), finalize_lanes),
		bindings,
		Vec::new(),
		Vec::new(),
		None,
		basic_resources(0, input.shape.elements(), 0, 0, 12, finalize_lanes,),
		StageKind::StableSortFinalize {
			network,
			emit_indices: spec.emit_indices,
		},
	)?;
	Ok(())
}

fn lower_random(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: RandomMap,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let false = output.shape.is_empty() else {
		return Ok(());
	};
	let round_work = checked_mul(
		builder,
		output.shape.elements(),
		u64::from(spec.philox_rounds),
		"Philox round work",
	)?;
	let flops = checked_mul(builder, round_work, 4, "Philox scheduled work")?;
	let integer_operations = output
		.shape
		.elements()
		.checked_mul(20)
		.ok_or_else(|| builder.overflow("Philox integer-operation bound"))?;
	let workgroup_lanes = builder.workgroup_lanes(output.shape.elements());
	push_stage!(
		builder,
		geometry(output.shape.elements(), workgroup_lanes),
		vec![builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write,)?],
		Vec::new(),
		Vec::new(),
		None,
		basic_resources(flops, integer_operations, 0, 0, 32, workgroup_lanes,),
		StageKind::Philox4x32_10(Philox10Contract {
			key: spec.key,
			distribution: spec.distribution,
			rounds: 10,
			multiplier_0: 0xd251_1f53,
			multiplier_1: 0xcd9e_8d57,
			weyl_0: 0x9e37_79b9,
			weyl_1: 0xbb67_ae85,
			counter: [
				PhiloxCounterWord::ElementLow,
				PhiloxCounterWord::ElementHigh,
				PhiloxCounterWord::IterationXorStreamLow,
				PhiloxCounterWord::IterationXorStreamHigh,
			],
			fold_kernel_id_into_key: true,
			fold_run_id_into_key: true,
			uniform_i32: UniformI32Mapping::UnbiasedMultiplyHighWithCounterRejection,
			normal_f32: NormalF32Mapping::OwnedBoxMullerV1,
		}),
	)?;
	Ok(())
}

fn lower_index_map(
	builder: &mut ProgramBuilder,
	kernel: &PrimitiveKernel,
	spec: IndexMap,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> LoweringResult<()> {
	let output = tensor(tensors, kernel.outputs[0], kernel.id)?;
	let false = output.shape.is_empty() else {
		return Ok(());
	};
	let (fault, fault_binding, fault_atomic) = builder.checked_fault(FaultReason::ArithmeticDomain, 4)?;
	let logical = output.shape.elements();
	let integer_operations = logical
		.checked_mul(INDEX_MAP_INTEGER_OPERATIONS_PER_LANE)
		.ok_or_else(|| builder.overflow("index-map integer-operation bound"))?;
	let workgroup_lanes = builder.workgroup_lanes(logical);
	push_stage!(
		builder,
		geometry(logical, workgroup_lanes),
		vec![
			builder.binding(builder.tensor_buffer(kernel.outputs[0])?, AccessMode::Write)?,
			fault_binding,
		],
		Vec::new(),
		vec![fault_atomic],
		Some(fault),
		basic_resources(0, integer_operations, logical, 0, 32, workgroup_lanes,),
		StageKind::IndexMap(spec),
	)?;
	Ok(())
}

fn checked_mul(builder: &ProgramBuilder, left: u64, right: u64, subject: &str) -> LoweringResult<u64> {
	left.checked_mul(right)
		.ok_or_else(|| builder.overflow(subject))
}

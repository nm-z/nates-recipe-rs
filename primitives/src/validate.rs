use std::collections::BTreeSet;

use recipe_core::{ByteCount, DType, FlopCount};
use recipe_language::{AtomicOperation, IndexBounds, ReduceResult, ScatterConflict};

use crate::{error::ProgramValidationError, model::*};

pub(crate) fn validate(program: &LoweredProgram) -> Result<(), Vec<ProgramValidationError>> {
	let mut errors = Vec::new();
	require(
		program.schema_version == LOWERED_PROGRAM_SCHEMA_VERSION,
		"schema_version",
		format!(
			"schema {} is unsupported; expected {LOWERED_PROGRAM_SCHEMA_VERSION}",
			program.schema_version
		),
		&mut errors,
	);
	validate_source_aliases(
		&program.source_aliases,
		program.source_input_count,
		program.source_output_count,
		&mut errors,
	);
	validate_buffers(&program.buffers, &mut errors);
	validate_stages(&program.buffers, &program.stages, &mut errors);
	validate_resources(program, &mut errors);
	require(
		program.digest == program.canonical_digest(),
		"digest",
		"program digest does not match its canonical contents",
		&mut errors,
	);
	errors.is_empty().then_some(()).ok_or(errors)
}

fn validate_source_aliases(aliases: &[SourceAliasContract], input_count: usize, output_count: usize, errors: &mut Vec<ProgramValidationError>) {
	let mut pairs = BTreeSet::new();
	for (index, alias) in aliases.iter().enumerate() {
		require(
			alias.input < input_count && alias.output < output_count,
			format!("source_aliases[{index}]"),
			"source alias pair exceeds the declared input/output arity",
			errors,
		);
		require(
			pairs.insert((alias.input, alias.output)),
			format!("source_aliases[{index}]"),
			"source input/output alias pair appears more than once",
			errors,
		);
	}
	require(
		aliases.len() == input_count.saturating_mul(output_count),
		"source_aliases",
		"source alias matrix is incomplete",
		errors,
	);
}

fn validate_buffers(buffers: &[ProgramBuffer], errors: &mut Vec<ProgramValidationError>) {
	let mut origins = BTreeSet::new();
	for (index, buffer) in buffers.iter().enumerate() {
		let path = format!("buffers[{index}]");
		require(
			usize::try_from(buffer.id.get()) == Ok(index),
			format!("{path}.id"),
			"buffer identifiers must be dense and ordered",
			errors,
		);
		require(
			!buffer.shape.is_empty(),
			format!("{path}.shape"),
			"buffer shape must have an explicit rank",
			errors,
		);
		require(
			buffer.shape == buffer.access.logical_extents,
			format!("{path}.access.logical_extents"),
			"canonical buffer access must describe its complete shape",
			errors,
		);
		validate_access(&buffer.access, buffer.dtype, false, &path, errors);
		let origin_key = match buffer.origin {
			BufferOrigin::Tensor(value) => (0_u8, value.get(), 0_u32),
			BufferOrigin::Scratch { ordinal, purpose } => (1, u64::from(ordinal), scratch_purpose_tag(purpose)),
			BufferOrigin::FaultFlag => (2, 0, 0),
		};
		require(
			origins.insert(origin_key),
			format!("{path}.origin"),
			"buffer origin appears more than once",
			errors,
		);
		let standard_contract = matches!(
			(&buffer.origin, buffer.lifetime, buffer.shape.as_slice()),
			(BufferOrigin::Tensor(_), BufferLifetime::ExternalValue, _)
				| (
					BufferOrigin::Scratch { .. },
					BufferLifetime::ProgramScratch,
					[_]
				)
		);
		let fault_contract = matches!(
			(
				&buffer.origin,
				buffer.lifetime,
				buffer.dtype,
				buffer.shape.as_slice()
			),
			(
				BufferOrigin::FaultFlag,
				BufferLifetime::ProgramFaultFlag,
				DType::I32,
				[1]
			)
		);
		require(
			standard_contract || fault_contract,
			format!("{path}.lifetime"),
			"buffer origin, lifetime, type, and shape are inconsistent",
			errors,
		);
		require(
			!fault_contract || buffer.access.storage_bytes == ByteCount::new(4),
			format!("{path}.access.storage_bytes"),
			"fault flag must occupy exactly four bytes",
			errors,
		);
	}
}

fn scratch_purpose_tag(value: ScratchPurpose) -> u32 {
	match value {
		ScratchPurpose::ReductionValues => 0,
		ScratchPurpose::ReductionIndices => 1,
		ScratchPurpose::ScanValues => 2,
		ScratchPurpose::ScanBlockTotals => 3,
		ScratchPurpose::SortValues => 4,
		ScratchPurpose::SortIndices => 5,
	}
}

fn validate_access(access: &StaticAccess, dtype: DType, writable: bool, path: &str, errors: &mut Vec<ProgramValidationError>) {
	require(
		access.logical_extents.len() == access.strides.len(),
		format!("{path}.strides"),
		"static access extent and stride ranks differ",
		errors,
	);
	let empty = access.logical_extents.contains(&0);
	let required = match empty {
		true => Some(0),
		false => access_required_bytes(access, dtype),
	};
	require(
		required.is_some(),
		format!("{path}.access"),
		"static access address calculation overflows u64",
		errors,
	);
	let storage_fits = required.is_some_and(|required| required <= access.storage_bytes.get());
	require(
		storage_fits,
		format!("{path}.storage_bytes"),
		format!(
			"static access exceeds its {} byte storage",
			access.storage_bytes.get()
		),
		errors,
	);
	let mut axes = access
		.logical_extents
		.iter()
		.copied()
		.zip(access.strides.iter().copied())
		.filter(|(extent, _)| *extent > 1)
		.collect::<Vec<_>>();
	axes.sort_by_key(|(_, stride)| *stride);
	let injective = axes
		.into_iter()
		.try_fold(1_u64, |occupied, (extent, stride)| {
			(stride >= occupied).then_some(())?;
			let contribution = (extent - 1).checked_mul(stride)?;
			contribution.checked_add(occupied)
		})
		.is_some();
	require(
		!writable || empty || access.logical_extents.len() != access.strides.len() || injective,
		format!("{path}.strides"),
		"non-atomic writable access is overlapping or broadcast",
		errors,
	);
}

fn access_required_bytes(access: &StaticAccess, dtype: DType) -> Option<u64> {
	let maximum = access
		.logical_extents
		.iter()
		.zip(&access.strides)
		.try_fold(access.offset_elements, |value, (extent, stride)| {
			let last = extent.checked_sub(1)?;
			let span = last.checked_mul(*stride)?;
			value.checked_add(span)
		})?;
	let elements = maximum.checked_add(1)?;
	elements.checked_mul(u64::from(dtype.byte_width()))
}

fn validate_stages(buffers: &[ProgramBuffer], stages: &[ProgramStage], errors: &mut Vec<ProgramValidationError>) {
	for (index, stage) in stages.iter().enumerate() {
		let path = format!("stages[{index}]");
		require(
			usize::try_from(stage.id.get()) == Ok(index),
			format!("{path}.id"),
			"stage identifiers must be dense and ordered",
			errors,
		);
		let expected_dependencies = stage
			.id
			.get()
			.checked_sub(1)
			.map(StageId::new)
			.into_iter()
			.collect::<Vec<_>>();
		require(
			stage.dependencies == expected_dependencies,
			format!("{path}.dependencies"),
			"canonical lowering serializes stages with the immediately previous dispatch dependency",
			errors,
		);
		require(
			stage.geometry.logical_lanes != 0,
			format!("{path}.geometry.logical_lanes"),
			"an emitted stage must have at least one logical lane",
			errors,
		);
		require(
			stage.geometry.workgroup_lanes != 0,
			format!("{path}.geometry.workgroup_lanes"),
			"workgroup lane count must be nonzero",
			errors,
		);
		let expected_workgroups = stage
			.geometry
			.logical_lanes
			.div_ceil(u64::from(stage.geometry.workgroup_lanes));
		require(
			stage.geometry.workgroups == expected_workgroups,
			format!("{path}.geometry.workgroups"),
			"workgroup count is not the ceiling of logical lanes divided by workgroup lanes",
			errors,
		);
		validate_bindings(buffers, stage, &path, errors);
		validate_atomics(stage, &path, errors);
		validate_fault(stage, &path, errors);
		validate_kind(stage, &path, errors);
		validate_stage_resources(stage, &path, errors);
	}
}

fn validate_bindings(buffers: &[ProgramBuffer], stage: &ProgramStage, path: &str, errors: &mut Vec<ProgramValidationError>) {
	for (index, binding) in stage.bindings.iter().enumerate() {
		let binding_path = format!("{path}.bindings[{index}]");
		let Ok(buffer_index) = usize::try_from(binding.buffer.get()) else {
			errors.push(ProgramValidationError::new(
				format!("{binding_path}.buffer"),
				"buffer identifier cannot index the buffer table",
			));
			continue;
		};
		let Some(buffer) = buffers.get(buffer_index) else {
			errors.push(ProgramValidationError::new(
				format!("{binding_path}.buffer"),
				"binding references an absent buffer",
			));
			continue;
		};
		require(
			buffer.id == binding.buffer,
			format!("{binding_path}.buffer"),
			"binding references the wrong dense buffer slot",
			errors,
		);
		require(
			buffer.dtype == binding.dtype,
			format!("{binding_path}.dtype"),
			"binding type differs from its buffer type",
			errors,
		);
		require(
			binding.view.storage_bytes.get() <= buffer.access.storage_bytes.get(),
			format!("{binding_path}.view.storage_bytes"),
			"binding view exceeds its buffer storage",
			errors,
		);
		validate_access(
			&binding.view,
			binding.dtype,
			matches!(binding.mode, AccessMode::Write | AccessMode::ReadWrite),
			&binding_path,
			errors,
		);
	}
}

fn validate_atomics(stage: &ProgramStage, path: &str, errors: &mut Vec<ProgramValidationError>) {
	let mut contracts = BTreeSet::new();
	for (index, atomic) in stage.atomics.iter().enumerate() {
		let atomic_path = format!("{path}.atomics[{index}]");
		require(
			contracts.insert((
				atomic.buffer,
				atomic_domain_tag(atomic.domain),
				atomic_operation_tag(atomic.operation),
				atomic_ordering_tag(atomic.ordering),
			)),
			atomic_path.clone(),
			"atomic contract appears more than once",
			errors,
		);
		let binding = stage
			.bindings
			.iter()
			.find(|binding| binding.buffer == atomic.buffer && binding.mode == AccessMode::ReadWriteAtomic);
		require(
			binding.is_some(),
			format!("{atomic_path}.buffer"),
			"atomic contract requires a ReadWriteAtomic binding",
			errors,
		);
		let Some(binding) = binding else {
			continue;
		};
		require(
			binding.dtype == atomic.dtype,
			format!("{atomic_path}.dtype"),
			"atomic contract type differs from its binding",
			errors,
		);
	}
}

fn atomic_domain_tag(value: AtomicAddressDomain) -> u8 {
	match value {
		AtomicAddressDomain::SingleFaultFlag => 0,
		AtomicAddressDomain::TensorElements => 1,
		AtomicAddressDomain::HistogramBins => 2,
	}
}

fn atomic_operation_tag(value: OwnedAtomicOperation) -> u8 {
	match value {
		OwnedAtomicOperation::Exchange => 0,
		OwnedAtomicOperation::Add => 1,
		OwnedAtomicOperation::Minimum => 2,
		OwnedAtomicOperation::Maximum => 3,
	}
}

fn atomic_ordering_tag(value: OwnedAtomicOrdering) -> u8 {
	match value {
		OwnedAtomicOrdering::Relaxed => 0,
		OwnedAtomicOrdering::Acquire => 1,
		OwnedAtomicOrdering::Release => 2,
		OwnedAtomicOrdering::AcquireRelease => 3,
		OwnedAtomicOrdering::SequentiallyConsistent => 4,
	}
}

fn validate_fault(stage: &ProgramStage, path: &str, errors: &mut Vec<ProgramValidationError>) {
	let Some(fault) = stage.fault else {
		return;
	};
	require(
		fault.guard_before_address,
		format!("{path}.fault.guard_before_address"),
		"checked rejection must guard before forming a payload address",
		errors,
	);
	require(
		fault.flag == fault.publish.buffer,
		format!("{path}.fault.publish.buffer"),
		"fault publication must target the declared flag",
		errors,
	);
	require(
		fault.publish.domain == AtomicAddressDomain::SingleFaultFlag && fault.publish.operation == OwnedAtomicOperation::Exchange && fault.publish.dtype == DType::I32,
		format!("{path}.fault.publish"),
		"fault publication must be an explicit int32 atomic exchange",
		errors,
	);
	require(
		stage.atomics.contains(&fault.publish),
		format!("{path}.atomics"),
		"fault publication is absent from the stage atomic list",
		errors,
	);
}

fn validate_kind(stage: &ProgramStage, path: &str, errors: &mut Vec<ProgramValidationError>) {
	match &stage.kind {
		StageKind::ScalarMap { template } => {
			errors.extend(
				template
					.validate()
					.err()
					.map(|core_errors| ProgramValidationError::new(format!("{path}.kind.template"), core_errors.to_string())),
			);
			require(
				template.index_space.elements().get() == stage.geometry.logical_lanes,
				format!("{path}.geometry.logical_lanes"),
				"scalar-map launch differs from its KernelTemplate index space",
				errors,
			);
			require(
				stage.bindings.len() == template.inputs.len() + template.outputs.len() + usize::from(stage.fault.is_some()),
				format!("{path}.bindings"),
				"scalar-map binding arity differs from its template and fault ABI",
				errors,
			);
			require(
				template.program.requires_fault_flag() == stage.fault.is_some(),
				format!("{path}.fault"),
				"scalar-map arithmetic fault ABI does not match its scalar program",
				errors,
			);
		}
		StageKind::FixedTreeReduce(reduction) => {
			validate_reduction_tree(&reduction.tree, path, errors);
			require(
				reduction.output_width
					== reduction
						.input_width
						.div_ceil(u64::from(reduction.tree.lanes)),
				format!("{path}.kind.output_width"),
				"reduction output width does not match the fixed tree fan-in",
				errors,
			);
			require(
				reduction.tie_break == ReductionTieBreak::LowestLogicalIndex,
				format!("{path}.kind.tie_break"),
				"value/index reductions must deterministically retain the lowest source index",
				errors,
			);
		}
		StageKind::FixedTreeScanLocal(scan) => {
			validate_scan_tree(&scan.tree, path, errors);
			require(
				scan.output_width == scan.input_width.div_ceil(u64::from(scan.tree.lanes)),
				format!("{path}.kind.output_width"),
				"scan block count does not match the fixed tree width",
				errors,
			);
		}
		StageKind::TiledContraction(contraction) => {
			require(
				contraction.canonical_contracted_order,
				format!("{path}.kind.canonical_contracted_order"),
				"contraction must fix a backend-independent accumulation order",
				errors,
			);
			require(
				contraction.tile.output_x != 0 && contraction.tile.output_y != 0 && contraction.tile.reduction != 0,
				format!("{path}.kind.tile"),
				"contraction tile dimensions must be nonzero",
				errors,
			);
			require(
				contraction
					.tile
					.output_x
					.checked_mul(contraction.tile.output_y)
					== Some(stage.geometry.workgroup_lanes),
				format!("{path}.kind.tile"),
				"contraction output tile must equal the realized workgroup width",
				errors,
			);
		}
		StageKind::Gather { bounds, .. } | StageKind::Scatter { bounds, .. } => {
			require(
				(*bounds == IndexBounds::Reject) == stage.fault.is_some(),
				format!("{path}.fault"),
				"Reject index semantics require the preallocated checked fault path",
				errors,
			);
		}
		StageKind::HistogramAccumulate { .. } => {
			require(
				matches!(
					stage.fault,
					Some(FaultContract {
						reason: FaultReason::HistogramBinOutOfBounds,
						..
					})
				),
				format!("{path}.fault"),
				"histogram accumulation must reject invalid bins through the fault flag",
				errors,
			)
		}
		StageKind::StableSortInitialize { network } | StageKind::StableSortFinalize { network, .. } => {
			validate_sort_network(*network, path, errors);
		}
		StageKind::StableSortCompareExchange(compare) => {
			validate_sort_network(compare.network, path, errors);
			require(
				compare.merge_width.is_power_of_two() && compare.merge_width <= compare.network.padded_axis_length && compare.compare_distance.is_power_of_two() && compare.compare_distance < compare.merge_width,
				format!("{path}.kind.network_phase"),
				"sort compare-exchange phase is not a valid fixed bitonic-network phase",
				errors,
			);
		}
		StageKind::Philox4x32_10(contract) => {
			require(
				contract.rounds == 10 && contract.multiplier_0 == 0xd251_1f53 && contract.multiplier_1 == 0xcd9e_8d57 && contract.weyl_0 == 0x9e37_79b9 && contract.weyl_1 == 0xbb67_ae85,
				format!("{path}.kind.philox"),
				"random stage is not the Recipe-owned Philox4x32-10 contract",
				errors,
			);
			require(
				contract.counter
					== [
						PhiloxCounterWord::ElementLow,
						PhiloxCounterWord::ElementHigh,
						PhiloxCounterWord::IterationXorStreamLow,
						PhiloxCounterWord::IterationXorStreamHigh,
					] && contract.fold_kernel_id_into_key
					&& contract.fold_run_id_into_key,
				format!("{path}.kind.counter"),
				"Philox counter/key folding contract is not canonical",
				errors,
			);
		}
		StageKind::IndexMap(spec) => {
			require(
				stage.bindings.len() == 2,
				format!("{path}.bindings"),
				"index map requires one output and one fault binding",
				errors,
			);
			require(
				spec.modulus.is_none_or(|modulus| modulus > 0),
				format!("{path}.kind.modulus"),
				"index-map modulus must be strictly positive when present",
				errors,
			);
			require(
				matches!(
					stage.fault,
					Some(FaultContract {
						reason: FaultReason::ArithmeticDomain,
						..
					})
				),
				format!("{path}.fault"),
				"index-map arithmetic requires the preallocated checked fault path",
				errors,
			);
		}
		StageKind::Fill { .. } => {
			require(
				stage.bindings.len() == 1,
				format!("{path}.bindings"),
				"fill stage requires exactly one output binding",
				errors,
			)
		}
		StageKind::Copy { .. } => {
			require(
				stage.bindings.len() == 2,
				format!("{path}.bindings"),
				"copy stage requires one input and one output binding",
				errors,
			)
		}
		StageKind::ScanUniformCombine(_) => {
			require(
				stage.bindings.len() == 2,
				format!("{path}.bindings"),
				"uniform scan combine requires target and offset bindings",
				errors,
			)
		}
		StageKind::HistogramClear => {
			require(
				stage.bindings.len() == 1,
				format!("{path}.bindings"),
				"histogram clear requires exactly one output binding",
				errors,
			)
		}
	}
}

fn validate_reduction_tree(tree: &FixedTree, path: &str, errors: &mut Vec<ProgramValidationError>) {
	require(
		tree.lanes.is_power_of_two() && (1..=1024).contains(&tree.lanes),
		format!("{path}.kind.tree.lanes"),
		"reduction tree lanes must be a power of two in 1..=1024",
		errors,
	);
	let expected = expected_reduction_steps(tree.lanes);
	require(
		tree.steps == expected,
		format!("{path}.kind.tree.steps"),
		"reduction tree steps are not the canonical descending-stride tree",
		errors,
	);
}

fn validate_scan_tree(tree: &FixedTree, path: &str, errors: &mut Vec<ProgramValidationError>) {
	require(
		tree.lanes.is_power_of_two() && (1..=1024).contains(&tree.lanes),
		format!("{path}.kind.tree.lanes"),
		"scan tree lanes must be a power of two in 1..=1024",
		errors,
	);
	let expected = expected_scan_steps(tree.lanes);
	require(
		tree.steps == expected,
		format!("{path}.kind.tree.steps"),
		"scan tree steps are not the canonical Blelloch upsweep/downsweep tree",
		errors,
	);
}

fn expected_reduction_steps(lanes: u32) -> Vec<TreeStep> {
	let count = lanes.trailing_zeros();
	(0..count)
		.map(|level| {
			TreeStep {
				phase: TreePhase::Reduction,
				stride: lanes >> (level + 1),
			}
		})
		.collect()
}

fn expected_scan_steps(lanes: u32) -> Vec<TreeStep> {
	let count = lanes.trailing_zeros();
	(0..count)
		.map(|level| {
			TreeStep {
				phase: TreePhase::ScanUpsweep,
				stride: 1 << level,
			}
		})
		.chain((0..count).rev().map(|level| {
			TreeStep {
				phase: TreePhase::ScanDownsweep,
				stride: 1 << level,
			}
		}))
		.collect()
}

fn validate_sort_network(network: SortNetwork, path: &str, errors: &mut Vec<ProgramValidationError>) {
	require(
		network.axis_length != 0 && network.padded_axis_length.is_power_of_two() && network.padded_axis_length >= network.axis_length,
		format!("{path}.kind.network.padded_axis_length"),
		"sort network does not use the least power-of-two padded domain",
		errors,
	);
	require(
		network.padded_axis_length == network.axis_length.next_power_of_two(),
		format!("{path}.kind.network.padded_axis_length"),
		"sort network padding is larger than necessary",
		errors,
	);
	require(
		network.tie_break == SortTieBreak::OriginalAxisIndexAscending && network.float_order == FloatSortOrder::Ieee754TotalOrder && network.padding == SortPadding::AfterAllValidElements,
		format!("{path}.kind.network"),
		"sort must be stable and deterministic for equal, NaN, signed-zero, and padded keys",
		errors,
	);
}

fn validate_stage_resources(stage: &ProgramStage, path: &str, errors: &mut Vec<ProgramValidationError>) {
	let expected = expected_stage_resources(stage);
	match expected {
		Some(expected) => {
			require(
				stage.resources == expected,
				format!("{path}.resources"),
				format!("stage resource bounds differ from the exact kind/geometry bounds: expected {expected:?}"),
				errors,
			)
		}
		None => {
			errors.push(ProgramValidationError::new(
				format!("{path}.resources"),
				"stage resource arithmetic overflowed",
			))
		}
	}
	let expected_sync = match &stage.kind {
		StageKind::FixedTreeReduce(reduction) => Some(tree_sync(&reduction.tree)),
		StageKind::FixedTreeScanLocal(scan) => Some(tree_sync(&scan.tree)),
		StageKind::TiledContraction(contraction) => {
			let count = match contraction.strategy {
				ContractionStrategy::Direct => 0,
				ContractionStrategy::Staged => {
					contraction
						.contracted_elements
						.div_ceil(u64::from(contraction.tile.reduction))
						.saturating_mul(2)
				}
			};
			match usize::try_from(count) {
				Ok(count) => Some(count),
				Err(error) => {
					errors.push(ProgramValidationError::new(
						format!("{path}.synchronization"),
						format!("synchronization count conversion failed: {error}"),
					));
					Some(stage.synchronization.len())
				}
			}
		}
		StageKind::ScalarMap { .. } | StageKind::Fill { .. } | StageKind::Copy { .. } | StageKind::ScanUniformCombine(_) | StageKind::Gather { .. } | StageKind::Scatter { .. } | StageKind::HistogramClear | StageKind::HistogramAccumulate { .. } | StageKind::StableSortInitialize { .. } | StageKind::StableSortCompareExchange(_) | StageKind::StableSortFinalize { .. } | StageKind::IndexMap(_) | StageKind::Philox4x32_10(_) => Some(0),
	};
	match expected_sync {
		Some(expected_sync) => {
			require(
				stage.synchronization.len() == expected_sync,
				format!("{path}.synchronization"),
				"synchronization count differs from the fixed algorithm",
				errors,
			)
		}
		None => {
			errors.push(ProgramValidationError::new(
				format!("{path}.synchronization"),
				"synchronization count cannot be represented",
			))
		}
	}
}

fn tree_sync(tree: &FixedTree) -> usize { tree.steps.len() }

fn expected_stage_resources(stage: &ProgramStage) -> Option<StageResourceBounds> {
	let lanes = stage.geometry.workgroup_lanes;
	let logical = stage.geometry.logical_lanes;
	let (flops, integer, atomic, shared, private) = match &stage.kind {
		StageKind::ScalarMap { template } => {
			let per_lane = template
				.program
				.instructions
				.iter()
				.try_fold(0_u64, |total, instruction| {
					total.checked_add(instruction.opcode.flops())
				})?;
			let slots = template.program.inputs.len() + template.program.constants.len() + template.program.instructions.len();
			let Ok(slots) = u64::try_from(slots) else {
				return None;
			};
			(
				per_lane.checked_mul(logical)?,
				logical,
				u64::from(stage.fault.is_some()).checked_mul(logical)?,
				0,
				slots.checked_mul(4)?,
			)
		}
		StageKind::Fill { .. } => (0, logical, 0, 0, 4),
		StageKind::Copy { .. } => (0, logical, 0, 0, 8),
		StageKind::FixedTreeReduce(reduction) => {
			let groups = reduction.sequences.checked_mul(reduction.output_width)?;
			let combines = groups.checked_mul(u64::from(reduction.tree.lanes - 1))?;
			let multiplier = match reduction.result {
				ReduceResult::ValueAndIndex => 2,
				ReduceResult::Value | ReduceResult::Index => 1,
			};
			let words = 1 + u64::from(reduction.result != ReduceResult::Value);
			(
				combines.checked_mul(multiplier)?,
				logical,
				0,
				u64::from(reduction.tree.lanes)
					.checked_mul(words)?
					.checked_mul(4)?,
				words.checked_mul(8)?,
			)
		}
		StageKind::FixedTreeScanLocal(scan) => {
			let groups = scan.sequences.checked_mul(scan.output_width)?;
			let tree = groups.checked_mul(u64::from(2 * (scan.tree.lanes - 1)))?;
			let inclusive = match scan.mode {
				ScanStageMode::UserInclusive => scan.sequences.checked_mul(scan.input_width)?,
				ScanStageMode::UserExclusive { .. } | ScanStageMode::HierarchyExclusiveIdentity => 0,
			};
			(
				tree.checked_add(inclusive)?,
				logical,
				0,
				u64::from(scan.tree.lanes).checked_mul(4)?,
				12,
			)
		}
		StageKind::ScanUniformCombine(_) => (logical, logical, 0, 0, 12),
		StageKind::TiledContraction(contraction) => {
			let shared = match contraction.strategy {
				ContractionStrategy::Direct => 0,
				ContractionStrategy::Staged => u64::from(lanes).checked_mul(u64::from(contraction.dtype.byte_width()))?,
			};
			(
				contraction
					.output_elements
					.checked_mul(contraction.contracted_elements)?
					.checked_mul(2)?,
				logical,
				0,
				shared,
				24,
			)
		}
		StageKind::Gather { .. } => {
			(
				0,
				logical,
				u64::from(stage.fault.is_some()).checked_mul(logical)?,
				0,
				16,
			)
		}
		StageKind::Scatter { conflict, .. } => {
			let payload_atomic = u64::from(matches!(conflict, ScatterConflict::Atomic { .. }));
			let fault_atomic = u64::from(stage.fault.is_some());
			let arithmetic = match conflict {
				ScatterConflict::Atomic {
					operation: AtomicOperation::Add | AtomicOperation::Minimum | AtomicOperation::Maximum,
					..
				} => 1,
				ScatterConflict::UniqueIndices
				| ScatterConflict::Atomic {
					operation: AtomicOperation::Exchange,
					..
				} => 0,
			};
			(
				logical.checked_mul(arithmetic)?,
				logical,
				logical.checked_mul(payload_atomic.checked_add(fault_atomic)?)?,
				0,
				16,
			)
		}
		StageKind::HistogramClear => (0, logical, 0, 0, 4),
		StageKind::HistogramAccumulate { .. } => (logical, logical, logical.checked_mul(2)?, 0, 16),
		StageKind::StableSortInitialize { .. } => (0, logical, 0, 0, 16),
		StageKind::StableSortCompareExchange(_) => (logical, logical, 0, 0, 20),
		StageKind::StableSortFinalize { .. } => (0, logical, 0, 0, 12),
		StageKind::IndexMap(_) => {
			(
				0,
				logical.checked_mul(INDEX_MAP_INTEGER_OPERATIONS_PER_LANE)?,
				logical,
				0,
				32,
			)
		}
		StageKind::Philox4x32_10(contract) => {
			(
				logical
					.checked_mul(u64::from(contract.rounds))?
					.checked_mul(4)?,
				logical.checked_mul(20)?,
				0,
				0,
				32,
			)
		}
	};
	Some(StageResourceBounds {
		flops: FlopCount::new(flops),
		integer_operations: integer,
		atomic_operations: atomic,
		shared_bytes_per_workgroup: ByteCount::new(shared),
		private_bytes_per_lane: ByteCount::new(private),
		maximum_workgroup_lanes: lanes,
	})
}

fn validate_resources(program: &LoweredProgram, errors: &mut Vec<ProgramValidationError>) {
	let totals = program.stages.iter().try_fold(
		(0_u64, 0_u64, 0_u64, 0_u64, 0_u64, 0_u32),
		|(flops, integer, atomic, shared, private, lanes), stage| {
			Some((
				flops.checked_add(stage.resources.flops.get())?,
				integer.checked_add(stage.resources.integer_operations)?,
				atomic.checked_add(stage.resources.atomic_operations)?,
				shared.max(stage.resources.shared_bytes_per_workgroup.get()),
				private.max(stage.resources.private_bytes_per_lane.get()),
				lanes.max(stage.resources.maximum_workgroup_lanes),
			))
		},
	);
	let storage = program
		.buffers
		.iter()
		.try_fold((0_u64, 0_u64), |(scratch, fault), buffer| {
			match buffer.lifetime {
				BufferLifetime::ExternalValue => Some((scratch, fault)),
				BufferLifetime::ProgramScratch => {
					Some((
						scratch.checked_add(buffer.access.storage_bytes.get())?,
						fault,
					))
				}
				BufferLifetime::ProgramFaultFlag => {
					Some((
						scratch,
						fault.checked_add(buffer.access.storage_bytes.get())?,
					))
				}
			}
		});
	let Some((flops, integer, atomic, shared, private, lanes)) = totals else {
		errors.push(ProgramValidationError::new(
			"resources",
			"program resource aggregation overflowed",
		));
		return;
	};
	let Some((scratch, fault)) = storage else {
		errors.push(ProgramValidationError::new(
			"resources",
			"program storage aggregation overflowed",
		));
		return;
	};
	let expected = ProgramResourceBounds {
		total_flops: FlopCount::new(flops),
		total_integer_operations: integer,
		total_atomic_operations: atomic,
		persistent_scratch_bytes: ByteCount::new(scratch),
		fault_bytes: ByteCount::new(fault),
		peak_shared_bytes_per_workgroup: ByteCount::new(shared),
		peak_private_bytes_per_lane: ByteCount::new(private),
		maximum_workgroup_lanes: lanes,
	};
	require(
		program.resources == expected,
		"resources",
		format!("program resource bounds differ from exact aggregate {expected:?}"),
		errors,
	);
}

fn require(condition: bool, path: impl Into<String>, detail: impl Into<String>, errors: &mut Vec<ProgramValidationError>) { errors.extend((!condition).then(|| ProgramValidationError::new(path, detail))); }

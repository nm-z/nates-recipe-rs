use recipe_core::{AliasPermission, DType, KernelTemplate, ScalarLiteral, ScalarOpcode, StaticBufferAccess};
use recipe_language::{
	AtomicOperation, AtomicOrdering, IndexBounds, RandomDistribution, ReduceOperator, ReduceResult, ScatterConflict,
	SortDirection, };

use crate::model::*;

const HASH_DOMAIN: &[u8] = b"recipe-lowered-primitive-program-v2\0";

pub(crate) fn program_digest(program: &LoweredProgram) -> ProgramDigest { let mut writer = CanonicalWriter::default();
	writer.bytes(HASH_DOMAIN); writer.u32(program.schema_version); writer.u64(program.source_kernel.get());
	writer.usize(program.source_input_count); writer.usize(program.source_output_count);
	writer.sequence(&program.source_aliases, |writer, alias| { writer.usize(alias.input); writer.usize(alias.output);
		writer.alias_permission(alias.permission); }); writer.sequence(&program.buffers, CanonicalWriter::buffer);
	writer.sequence(&program.stages, CanonicalWriter::stage); writer.program_resources(program.resources);
	ProgramDigest::new(sha256(&writer.finish())) }

#[derive(Debug, Default)]
struct CanonicalWriter { bytes: Vec<u8>, }

impl CanonicalWriter { fn finish(self) -> Vec<u8> { self.bytes }

	fn bytes(&mut self, value: &[u8]) { self.length(value.len()); self.bytes.extend_from_slice(value); }

	fn bool(&mut self, value: bool) { self.u8(u8::from(value)); }

	fn u8(&mut self, value: u8) { self.bytes.push(value); }

	fn u32(&mut self, value: u32) { self.bytes.extend_from_slice(&value.to_le_bytes()); }

	fn i32(&mut self, value: i32) { self.bytes.extend_from_slice(&value.to_le_bytes()); }

	fn u64(&mut self, value: u64) { self.bytes.extend_from_slice(&value.to_le_bytes()); }

	fn usize(&mut self, value: usize) { self.length(value); }

	fn length(&mut self, value: usize) { self.bytes.extend_from_slice(&usize_u128_le_bytes(value)); }

	fn tag(&mut self, value: u8) { self.u8(value); }

	fn sequence<T>(&mut self, values: &[T], mut encode: impl FnMut(&mut Self, &T)) { self.length(values.len());
		for value in values { encode(self, value); } }

	fn dtype(&mut self, value: DType) { self.tag(match value { DType::F32 => 0, DType::I32 => 1, }); }

	fn alias_permission(&mut self, value: AliasPermission) { self.tag(match value { AliasPermission::Forbidden => 0,
			AliasPermission::MayAliasExact => 1, AliasPermission::MustAliasExact => 2, }); }

	fn literal(&mut self, value: ScalarLiteral) { match value { ScalarLiteral::F32Bits(bits) => { self.tag(0);
				self.u32(bits); }
			ScalarLiteral::I32(value) => { self.tag(1); self.i32(value); } } }

	fn scalar_opcode(&mut self, value: ScalarOpcode) {
		// ScalarOpcode is non-exhaustive. Its stable variant spelling is part
		// of this schema's canonical encoding, so future opcodes cannot collide
		// with an existing numeric fallback tag.
		self.bytes(format!("{value:?}").as_bytes());
	}

	fn static_access(&mut self, value: &StaticAccess) {
		self.sequence(&value.logical_extents, |writer, value| writer.u64(*value)); self.u64(value.offset_elements);
		self.sequence(&value.strides, |writer, value| writer.u64(*value)); self.u64(value.storage_bytes.get()); }

	fn core_static_access(&mut self, value: &StaticBufferAccess) { self.u64(value.offset_elements);
		self.sequence(&value.strides, |writer, value| writer.u64(*value)); self.u64(value.storage_bytes.get()); }

	fn kernel_template(&mut self, value: &KernelTemplate) { self.u64(value.id.get());
		self.sequence(value.index_space.dimensions(), |writer, extent| { writer.u64(extent.get()); });
		self.sequence(&value.inputs, |writer, input| { writer.u64(input.id.get()); writer.dtype(input.dtype);
			writer.core_static_access(&input.access); }); self.sequence(&value.outputs, |writer, output| {
			writer.u64(output.id.get()); writer.dtype(output.dtype); writer.core_static_access(&output.access); });
		self.sequence(&value.program.inputs, |writer, input| { writer.u64(input.id.get()); writer.dtype(input.dtype); });
		self.sequence(&value.program.constants, |writer, constant| { writer.u64(constant.id.get());
			writer.literal(constant.value); }); self.sequence(&value.program.instructions, |writer, instruction| {
			writer.u64(instruction.result.get()); writer.dtype(instruction.dtype); writer.scalar_opcode(instruction.opcode);
			writer.sequence(&instruction.operands, |writer, operand| { writer.u64(operand.get()) }); });
		self.sequence(&value.program.outputs, |writer, output| { writer.u64(output.get()) });
		self.sequence(&value.alias_rules, |writer, rule| { writer.u64(rule.input.get()); writer.u64(rule.output.get());
			writer.alias_permission(rule.permission); }); }

	fn buffer(&mut self, value: &ProgramBuffer) { self.u32(value.id.get()); match value.origin {
			BufferOrigin::Tensor(id) => { self.tag(0); self.u64(id.get()); }
			BufferOrigin::Scratch { ordinal, purpose } => { self.tag(1); self.u32(ordinal); self.scratch_purpose(purpose); }
			BufferOrigin::FaultFlag => self.tag(2), }
		self.tag(match value.lifetime { BufferLifetime::ExternalValue => 0, BufferLifetime::ProgramScratch => 1,
			BufferLifetime::ProgramFaultFlag => 2, }); self.dtype(value.dtype);
		self.sequence(&value.shape, |writer, extent| writer.u64(*extent)); self.static_access(&value.access); }

	fn scratch_purpose(&mut self, value: ScratchPurpose) { self.tag(match value { ScratchPurpose::ReductionValues => 0,
			ScratchPurpose::ReductionIndices => 1, ScratchPurpose::ScanValues => 2, ScratchPurpose::ScanBlockTotals => 3,
			ScratchPurpose::SortValues => 4, ScratchPurpose::SortIndices => 5, }); }

	fn stage(&mut self, value: &ProgramStage) { self.u32(value.id.get());
		self.sequence(&value.dependencies, |writer, dependency| { writer.u32(dependency.get()); });
		self.u64(value.geometry.logical_lanes); self.u32(value.geometry.workgroup_lanes); self.u64(value.geometry.workgroups);
		self.sequence(&value.bindings, CanonicalWriter::binding); self.sequence(&value.synchronization, |writer, point| {
			writer.u32(point.after_step); writer.tag(match point.scope { SynchronizationScope::Workgroup => 0,
				SynchronizationScope::DispatchBoundary => 1, }); writer.tag(match point.memory {
				MemorySemantics::SharedAcquireRelease => 0, MemorySemantics::GlobalAcquireRelease => 1, }); });
		self.sequence(&value.atomics, |writer, atomic| writer.atomic(*atomic)); match value.fault { Some(fault) => {
				self.bool(true); self.fault(fault); }
			None => self.bool(false), }
		self.stage_resources(value.resources); self.stage_kind(&value.kind); }

	fn binding(&mut self, value: &BufferBinding) { self.u32(value.buffer.get()); self.dtype(value.dtype);
		self.tag(match value.mode { AccessMode::Read => 0, AccessMode::Write => 1, AccessMode::ReadWrite => 2,
			AccessMode::ReadWriteAtomic => 3, }); self.static_access(&value.view); }

	fn atomic(&mut self, value: AtomicContract) { self.u32(value.buffer.get()); self.dtype(value.dtype);
		self.tag(match value.operation { OwnedAtomicOperation::Exchange => 0, OwnedAtomicOperation::Add => 1,
			OwnedAtomicOperation::Minimum => 2, OwnedAtomicOperation::Maximum => 3, }); self.atomic_ordering(value.ordering);
		self.tag(match value.domain { AtomicAddressDomain::SingleFaultFlag => 0, AtomicAddressDomain::TensorElements => 1,
			AtomicAddressDomain::HistogramBins => 2, }); }

	fn atomic_ordering(&mut self, value: OwnedAtomicOrdering) { self.tag(match value { OwnedAtomicOrdering::Relaxed => 0,
			OwnedAtomicOrdering::Acquire => 1, OwnedAtomicOrdering::Release => 2, OwnedAtomicOrdering::AcquireRelease => 3,
			OwnedAtomicOrdering::SequentiallyConsistent => 4, }); }

	fn fault(&mut self, value: FaultContract) { self.u32(value.flag.get()); self.tag(match value.reason {
			FaultReason::ArithmeticDomain => 0, FaultReason::IndexOutOfBounds => 1, FaultReason::HistogramBinOutOfBounds => 2,
		}); self.i32(value.code); self.bool(value.guard_before_address); self.atomic(value.publish); }

	fn fixed_tree(&mut self, value: &FixedTree) { self.u32(value.lanes); self.sequence(&value.steps, |writer, step| {
			writer.tag(match step.phase { TreePhase::Reduction => 0, TreePhase::ScanUpsweep => 1, TreePhase::ScanDownsweep => 2,
			}); writer.u32(step.stride); }); }

	fn reduce_operator(&mut self, value: ReduceOperator) { self.tag(match value { ReduceOperator::Sum => 0,
			ReduceOperator::Product => 1, ReduceOperator::Minimum => 2, ReduceOperator::Maximum => 3, ReduceOperator::Any => 4,
			ReduceOperator::All => 5, }); }

	fn reduce_result(&mut self, value: ReduceResult) { self.tag(match value { ReduceResult::Value => 0,
			ReduceResult::Index => 1, ReduceResult::ValueAndIndex => 2, }); }

	fn index_bounds(&mut self, value: IndexBounds) { self.tag(match value { IndexBounds::Reject => 0,
			IndexBounds::Clamp => 1, IndexBounds::Wrap => 2, }); }

	fn atomic_operation(&mut self, value: AtomicOperation) { self.tag(match value { AtomicOperation::Exchange => 0,
			AtomicOperation::Add => 1, AtomicOperation::Minimum => 2, AtomicOperation::Maximum => 3, }); }

	fn language_atomic_ordering(&mut self, value: AtomicOrdering) { self.tag(match value { AtomicOrdering::Relaxed => 0,
			AtomicOrdering::Acquire => 1, AtomicOrdering::Release => 2, AtomicOrdering::AcquireRelease => 3,
			AtomicOrdering::SequentiallyConsistent => 4, }); }

	fn scatter_conflict(&mut self, value: ScatterConflict) { match value { ScatterConflict::UniqueIndices => self.tag(0),
			ScatterConflict::Atomic { operation, ordering, } => { self.tag(1); self.atomic_operation(operation);
				self.language_atomic_ordering(ordering); } } }

	fn sort_direction(&mut self, value: SortDirection) { self.tag(match value { SortDirection::Ascending => 0,
			SortDirection::Descending => 1, }); }

	fn scan_mode(&mut self, value: ScanStageMode) { match value { ScanStageMode::UserInclusive => self.tag(0),
			ScanStageMode::UserExclusive { identity } => { self.tag(1); self.literal(identity); }
			ScanStageMode::HierarchyExclusiveIdentity => self.tag(2), } }

	fn sort_network(&mut self, value: SortNetwork) { self.usize(value.axis); self.u64(value.axis_length);
		self.u64(value.padded_axis_length); self.u64(value.slices); self.sort_direction(value.direction);
		self.bool(value.requested_stable); self.tag(match value.tie_break { SortTieBreak::OriginalAxisIndexAscending => 0, });
		self.tag(match value.float_order { FloatSortOrder::Ieee754TotalOrder => 0, }); self.tag(match value.padding {
			SortPadding::AfterAllValidElements => 0, }); }

	fn random_distribution(&mut self, value: RandomDistribution) { match value {
			RandomDistribution::UniformF32 => self.tag(0), RandomDistribution::NormalF32 => self.tag(1),
			RandomDistribution::BernoulliI32 { probability_bits } => { self.tag(2); self.u32(probability_bits); }
			RandomDistribution::UniformI32 { low, high_exclusive, } => { self.tag(3); self.i32(low); self.i32(high_exclusive); }
		} }

	fn contraction(&mut self, value: &ContractionStage) { self.sequence(&value.spec.batch_axes, |writer, pair| {
			writer.usize(pair.0); writer.usize(pair.1); }); self.sequence(&value.spec.contract_axes, |writer, pair| {
			writer.usize(pair.0); writer.usize(pair.1); }); self.dtype(value.dtype); self.u64(value.output_elements);
		self.u64(value.contracted_elements); self.tag(match value.strategy { ContractionStrategy::Direct => 0,
			ContractionStrategy::Staged => 1, }); self.u32(value.tile.output_x); self.u32(value.tile.output_y);
		self.u32(value.tile.reduction); self.bool(value.canonical_contracted_order); }

	fn stage_kind(&mut self, value: &StageKind) { match value { StageKind::ScalarMap { template } => { self.tag(0);
				self.kernel_template(template); }
			StageKind::Fill { value } => { self.tag(1); self.literal(*value); }
			StageKind::Copy { reason } => { self.tag(2); self.tag(match reason { CopyReason::ScatterBase => 0, }); }
			StageKind::FixedTreeReduce(stage) => { self.tag(3); self.u32(stage.pass); self.reduce_operator(stage.operator);
				self.reduce_result(stage.result); self.dtype(stage.dtype); self.u64(stage.sequences); self.u64(stage.input_width);
				self.u64(stage.output_width); self.sequence(&stage.reduced_axes, |writer, axis| writer.usize(*axis));
				self.fixed_tree(&stage.tree); self.tag(match stage.padding { ReductionPadding::OperatorIdentity => 0, });
				self.tag(match stage.tie_break { ReductionTieBreak::LowestLogicalIndex => 0, }); }
			StageKind::FixedTreeScanLocal(stage) => { self.tag(4); self.u32(stage.level); self.reduce_operator(stage.operator);
				self.dtype(stage.dtype); self.u64(stage.sequences); self.u64(stage.input_width); self.u64(stage.output_width);
				self.usize(stage.axis); self.bool(stage.reverse); self.scan_mode(stage.mode); self.fixed_tree(&stage.tree); }
			StageKind::ScanUniformCombine(stage) => { self.tag(5); self.u32(stage.level); self.reduce_operator(stage.operator);
				self.dtype(stage.dtype); self.u64(stage.sequences); self.u64(stage.width); self.u32(stage.block_lanes);
				self.bool(stage.reverse); }
			StageKind::TiledContraction(stage) => { self.tag(6); self.contraction(stage); }
			StageKind::Gather { axis, bounds } => { self.tag(7); self.usize(*axis); self.index_bounds(*bounds); }
			StageKind::Scatter { axis, bounds, conflict, } => { self.tag(8); self.usize(*axis); self.index_bounds(*bounds);
				self.scatter_conflict(*conflict); }
			StageKind::HistogramClear => self.tag(9), StageKind::HistogramAccumulate { bins, weighted, bin_mapping, ordering,
			} => { self.tag(10); self.u32(*bins); self.bool(*weighted); self.tag(match bin_mapping {
					HistogramBinMapping::I32Direct => 0, HistogramBinMapping::F32TruncateTowardZero => 1, });
				self.atomic_ordering(*ordering); }
			StageKind::StableSortInitialize { network } => { self.tag(11); self.sort_network(*network); }
			StageKind::StableSortCompareExchange(stage) => { self.tag(12); self.sort_network(stage.network);
				self.u64(stage.merge_width); self.u64(stage.compare_distance); }
			StageKind::StableSortFinalize { network, emit_indices, } => { self.tag(13); self.sort_network(*network);
				self.bool(*emit_indices); }
			StageKind::Philox4x32_10(contract) => { self.tag(14); self.u64(contract.key.seed_low);
				self.u64(contract.key.seed_high); self.u64(contract.key.stream); self.random_distribution(contract.distribution);
				self.u8(contract.rounds); self.u32(contract.multiplier_0); self.u32(contract.multiplier_1);
				self.u32(contract.weyl_0); self.u32(contract.weyl_1); for word in contract.counter { self.tag(match word {
						PhiloxCounterWord::ElementLow => 0, PhiloxCounterWord::ElementHigh => 1,
						PhiloxCounterWord::IterationXorStreamLow => 2, PhiloxCounterWord::IterationXorStreamHigh => 3, }); }
				self.bool(contract.fold_kernel_id_into_key); self.bool(contract.fold_run_id_into_key);
				self.tag(match contract.uniform_i32 { UniformI32Mapping::UnbiasedMultiplyHighWithCounterRejection => 0, });
				self.tag(match contract.normal_f32 { NormalF32Mapping::OwnedBoxMullerV1 => 0, }); }
			StageKind::IndexMap(spec) => { self.tag(15); self.i32(spec.start); self.i32(spec.element_step);
				self.i32(spec.iteration_step); match spec.modulus { Some(modulus) => { self.bool(true); self.i32(modulus); }
					None => self.bool(false), } } } }

	fn stage_resources(&mut self, value: StageResourceBounds) { self.u64(value.flops.get());
		self.u64(value.integer_operations); self.u64(value.atomic_operations);
		self.u64(value.shared_bytes_per_workgroup.get()); self.u64(value.private_bytes_per_lane.get());
		self.u32(value.maximum_workgroup_lanes); }

	fn program_resources(&mut self, value: ProgramResourceBounds) { self.u64(value.total_flops.get());
		self.u64(value.total_integer_operations); self.u64(value.total_atomic_operations);
		self.u64(value.persistent_scratch_bytes.get()); self.u64(value.fault_bytes.get());
		self.u64(value.peak_shared_bytes_per_workgroup.get()); self.u64(value.peak_private_bytes_per_lane.get());
		self.u32(value.maximum_workgroup_lanes); } }

fn usize_u128_le_bytes(value: usize) -> [u8; 16] { let source = value.to_le_bytes(); let mut result = [0_u8; 16];
	result[..source.len()].copy_from_slice(&source); result }

fn sha256(input: &[u8]) -> [u8; 32] { const INITIAL: [u32; 8] = [ 0x6a09_e667, 0xbb67_ae85, 0x3c6e_f372, 0xa54f_f53a,
		0x510e_527f, 0x9b05_688c, 0x1f83_d9ab, 0x5be0_cd19, ]; const ROUND: [u32; 64] = [ 0x428a_2f98, 0x7137_4491,
		0xb5c0_fbcf, 0xe9b5_dba5, 0x3956_c25b, 0x59f1_11f1, 0x923f_82a4, 0xab1c_5ed5, 0xd807_aa98, 0x1283_5b01, 0x2431_85be,
		0x550c_7dc3, 0x72be_5d74, 0x80de_b1fe, 0x9bdc_06a7, 0xc19b_f174, 0xe49b_69c1, 0xefbe_4786, 0x0fc1_9dc6, 0x240c_a1cc,
		0x2de9_2c6f, 0x4a74_84aa, 0x5cb0_a9dc, 0x76f9_88da, 0x983e_5152, 0xa831_c66d, 0xb003_27c8, 0xbf59_7fc7, 0xc6e0_0bf3,
		0xd5a7_9147, 0x06ca_6351, 0x1429_2967, 0x27b7_0a85, 0x2e1b_2138, 0x4d2c_6dfc, 0x5338_0d13, 0x650a_7354, 0x766a_0abb,
		0x81c2_c92e, 0x9272_2c85, 0xa2bf_e8a1, 0xa81a_664b, 0xc24b_8b70, 0xc76c_51a3, 0xd192_e819, 0xd699_0624, 0xf40e_3585,
		0x106a_a070, 0x19a4_c116, 0x1e37_6c08, 0x2748_774c, 0x34b0_bcb5, 0x391c_0cb3, 0x4ed8_aa4a, 0x5b9c_ca4f, 0x682e_6ff3,
		0x748f_82ee, 0x78a5_636f, 0x84c8_7814, 0x8cc7_0208, 0x90be_fffa, 0xa450_6ceb, 0xbef9_a3f7, 0xc671_78f2, ];

	let wide_bit_length = u128::from_le_bytes(usize_u128_le_bytes(input.len())).wrapping_mul(8);
	let bit_length_bytes = wide_bit_length.to_be_bytes(); let mut padded = input.to_vec(); padded.push(0x80);
	let zero_padding = (56 + 64 - (padded.len() % 64)) % 64; padded.extend(std::iter::repeat_n(0, zero_padding));
	padded.extend_from_slice(&bit_length_bytes[8..]);

	let mut state = INITIAL; for block in padded.chunks_exact(64) { let mut schedule = [0_u32; 64];
		for (index, word) in block.chunks_exact(4).enumerate() {
			schedule[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]); }
		for index in 16..64 { let s0 = schedule[index - 15].rotate_right(7) ^ schedule[index - 15].rotate_right(18)
				^ (schedule[index - 15] >> 3); let s1 = schedule[index - 2].rotate_right(17)
				^ schedule[index - 2].rotate_right(19)
				^ (schedule[index - 2] >> 10); schedule[index] = schedule[index - 16] .wrapping_add(s0)
				.wrapping_add(schedule[index - 7]) .wrapping_add(s1); }
		let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
		for (index, schedule_word) in schedule.iter().copied().enumerate() {
			let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25); let choice = (e & f) ^ (!e & g);
			let temp1 = h .wrapping_add(sum1) .wrapping_add(choice) .wrapping_add(ROUND[index]) .wrapping_add(schedule_word);
			let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22); let majority = (a & b) ^ (a & c) ^ (b & c);
			let temp2 = sum0.wrapping_add(majority); h = g; g = f; f = e; e = d.wrapping_add(temp1); d = c; c = b; b = a;
			a = temp1.wrapping_add(temp2); }
		for (slot, value) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) { *slot = slot.wrapping_add(value); } }

	let mut output = [0_u8; 32]; for (chunk, value) in output.chunks_exact_mut(4).zip(state) {
		chunk.copy_from_slice(&value.to_be_bytes()); }
	output }

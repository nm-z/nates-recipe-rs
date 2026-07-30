use std::collections::BTreeMap;

use recipe_core::{
	ArtifactBuildAccess, ArtifactBuildRecipe, ByteCount, DType, Digest, ElementCount, KernelResourceBounds,
	ScalarLiteral,
};
use recipe_language::{
	IndexBounds, IndexMap, RandomDistribution, ReduceOperator, ReduceResult, ScatterConflict, SortDirection,
};
use recipe_primitives::{
	AccessMode, AtomicAddressDomain, ContractionStage, ContractionStrategy, FaultContract, FixedTree, FloatSortOrder,
	HistogramBinMapping, LoweredProgram, MemorySemantics, NormalF32Mapping, OwnedAtomicOperation,
	OwnedAtomicOrdering, Philox10Contract, PhiloxCounterWord, ProgramStage, ReductionPadding, ReductionStage,
	ReductionTieBreak, ScanLocalStage, ScanStageMode, ScanUniformStage, SortCompareStage, SortNetwork, SortPadding,
	SortTieBreak, StageKind, SynchronizationScope, TreePhase, UniformI32Mapping,
};
use sha2::{Digest as _, Sha256};

use crate::{
	BufferAccess, KernelAbi, KernelArgument, KernelTarget, LoweredKernel, LoweringError, LoweringErrorKind,
	LoweringOptions, audit_llvm_ir,
};

/// Recompute the planner's canonical deferred-artifact contract digest.
///
/// Realize uses this independently implemented verifier before trusting any
/// stage recipe. A stale digest therefore cannot bless mutated bindings,
/// geometry, work, fault storage, or resource bounds.
#[must_use]
pub(crate) fn artifact_build_contract_digest(build: &ArtifactBuildRecipe) -> Digest {
	let mut digest = ContractDigest::new("recipe-planner-artifact-build-v1");
	digest.u64(build.artifact.get());
	digest.u64(build.kernel_template.get());
	digest.u64(build.source_kernel.get());
	digest.digest(build.provenance.program_digest);
	digest.u64(u64::from(build.provenance.stage_ordinal));
	digest.length(build.bindings.len());
	for binding in &build.bindings {
		digest.u64(binding.value.get());
		digest.u8(match binding.dtype {
			DType::F32 => 0,
			DType::I32 => 1,
		});
		digest.u8(match binding.access {
			ArtifactBuildAccess::Read => 0,
			ArtifactBuildAccess::Write => 1,
			ArtifactBuildAccess::ReadWrite => 2,
			ArtifactBuildAccess::ReadWriteAtomic => 3,
		});
		digest.length(binding.view.logical_extents.len());
		for extent in &binding.view.logical_extents {
			digest.u64(*extent);
		}
		digest.u64(binding.view.offset_elements);
		digest.length(binding.view.strides.len());
		for stride in &binding.view.strides {
			digest.u64(*stride);
		}
		digest.u64(binding.view.storage_bytes.get());
	}
	digest.u64(build.dispatch.logical_lanes);
	digest.u64(u64::from(build.dispatch.workgroup_lanes));
	digest.u64(build.dispatch.workgroups);
	digest.u64(build.work.flops.get());
	digest.u64(build.work.integer_operations);
	digest.u64(build.work.atomic_operations);
	match build.fault_flag {
		Some(fault) => {
			digest.u8(1);
			digest.u64(fault.get());
		}
		None => digest.u8(0),
	}
	digest.u64(build.resources.private_bytes_per_lane.get());
	digest.u64(build.resources.shared_bytes_per_workgroup.get());
	digest.u64(build.resources.scratch_bytes_per_dispatch.get());
	digest.u64(u64::from(build.resources.maximum_workgroup_lanes));
	digest.finish()
}

/// Realize one exact primitive stage into target LLVM IR.
///
/// The complete lowered program is required so stage ordinal, program digest,
/// scan-axis context, and the immutable stage contract can be checked without
/// trusting caller-selected fragments.
pub(crate) fn lower_stage(
	program: &LoweredProgram,
	build: &ArtifactBuildRecipe,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<LoweredKernel, LoweringError> {
	let stage = validate_contract(program, build, target, options)?;
	match &stage.kind {
		StageKind::ScalarMap { template } => {
			let mut lowered = crate::lower_elementwise(template, target, options)?;
			rewrite_scalar_fault(&mut lowered, stage.fault)?;
			validate_realized_kernel(&lowered, stage, build)?;
			Ok(lowered)
		}
		kind => lower_owned_stage(program, stage, kind, build, target, options),
	}
}

fn rewrite_scalar_fault(lowered: &mut LoweredKernel, fault: Option<FaultContract>) -> Result<(), LoweringError> {
	let fault = match fault {
		Some(fault) => fault,
		None => return Ok(()),
	};
	let old = "atomicrmw or ptr addrspace(1) %fault_flag, i32 1 monotonic, align 4";
	ensure_stage(
		lowered.llvm_ir.contains(old),
		"scalar stage requires a fault publication that the scalar emitter did not produce",
	)?;
	let replacement = format!(
		"atomicrmw xchg ptr addrspace(1) %fault_flag, i32 {} release, align 4",
		fault.code
	);
	lowered.llvm_ir = lowered.llvm_ir.replacen(old, &replacement, 1);
	Ok(())
}

fn validate_contract<'a>(
	program: &'a LoweredProgram,
	build: &ArtifactBuildRecipe,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<&'a ProgramStage, LoweringError> {
	ensure_stage(
		program.validate().is_ok(),
		"lowered program failed canonical validation",
	)?;
	ensure_stage(
		build.validate().is_ok(),
		"artifact build recipe failed canonical validation",
	)?;
	target.validate()?;
	validate_options(options)?;
	ensure_stage(
		build.provenance.program_digest == Digest::new(program.digest.bytes()),
		"artifact build program digest differs from the canonical lowered program",
	)?;
	ensure_stage(
		build.source_kernel == program.source_kernel,
		"artifact build source kernel differs from the lowered program",
	)?;
	let stage = program
		.stages
		.iter()
		.find(|stage| stage.id.get() == build.provenance.stage_ordinal)
		.ok_or_else(|| stage_error("artifact build stage ordinal is absent from the lowered program"))?;
	let expected_template = stage_template_identity(program, stage)?;
	ensure_stage(
		build.kernel_template == expected_template,
		"artifact build stage-scoped kernel identity is not canonical",
	)?;
	ensure_stage(
		build.artifact.get() == build.kernel_template.get(),
		"artifact ID differs from the reserved stage-scoped identity",
	)?;
	ensure_stage(
		build.provenance.contract_digest == artifact_build_contract_digest(build),
		"artifact build contract digest does not match its contents",
	)?;
	ensure_stage(
		build.dispatch.logical_lanes == stage.geometry.logical_lanes
			&& build.dispatch.workgroup_lanes == stage.geometry.workgroup_lanes
			&& build.dispatch.workgroups == stage.geometry.workgroups,
		"artifact dispatch geometry differs from the primitive stage",
	)?;
	ensure_stage(
		options.workgroup_lanes == stage.geometry.workgroup_lanes,
		"lowering options workgroup size differs from the immutable stage geometry",
	)?;
	ensure_stage(
		build.work.flops == stage.resources.flops
			&& build.work.integer_operations == stage.resources.integer_operations
			&& build.work.atomic_operations == stage.resources.atomic_operations,
		"artifact work bounds differ from the primitive stage",
	)?;
	let expected_resources = KernelResourceBounds {
		private_bytes_per_lane: stage.resources.private_bytes_per_lane,
		shared_bytes_per_workgroup: stage.resources.shared_bytes_per_workgroup,
		scratch_bytes_per_dispatch: ByteCount::ZERO,
		maximum_workgroup_lanes: stage.resources.maximum_workgroup_lanes,
	};
	ensure_stage(
		build.resources == expected_resources,
		"artifact resource envelope differs from the primitive stage",
	)?;
	ensure_stage(
		build.bindings.len() == stage.bindings.len(),
		"artifact binding count differs from the primitive stage",
	)?;
	for (actual, expected) in build.bindings.iter().zip(&stage.bindings) {
		let expected_access = access(expected.mode);
		ensure_stage(
			actual.dtype == expected.dtype
				&& actual.access == expected_access
				&& actual.view.logical_extents == expected.view.logical_extents
				&& actual.view.offset_elements == expected.view.offset_elements
				&& actual.view.strides == expected.view.strides
				&& actual.view.storage_bytes == expected.view.storage_bytes,
			"artifact binding differs from the primitive stage",
		)?;
	}
	validate_fault_binding(stage, build)?;
	Ok(stage)
}

fn validate_fault_binding(stage: &ProgramStage, build: &ArtifactBuildRecipe) -> Result<(), LoweringError> {
	let fault = match stage.fault {
		Some(fault) => fault,
		None => {
			return ensure_stage(
				build.fault_flag.is_none(),
				"artifact supplies a fault value for an unchecked stage",
			);
		}
	};
	let position = stage
		.bindings
		.iter()
		.position(|binding| binding.buffer == fault.flag)
		.ok_or_else(|| stage_error("stage fault flag has no ordered binding"))?;
	ensure_stage(
		build.fault_flag == Some(build.bindings[position].value),
		"artifact fault value differs from the stage fault binding",
	)
}

fn validate_options(options: &LoweringOptions) -> Result<(), LoweringError> {
	let valid_symbol = !options.entry_symbol.is_empty()
		&& options
			.entry_symbol
			.bytes()
			.next()
			.is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
		&& options
			.entry_symbol
			.bytes()
			.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
	ensure(
		valid_symbol,
		LoweringErrorKind::InvalidEntrySymbol,
		"entry symbol must be an ASCII LLVM identifier without punctuation",
	)?;
	ensure(
		(1..=1024).contains(&options.workgroup_lanes),
		LoweringErrorKind::InvalidWorkgroupSize,
		"workgroup size must be between 1 and 1024 lanes",
	)?;
	Ok(())
}

fn stage_template_identity(
	program: &LoweredProgram,
	stage: &ProgramStage,
) -> Result<recipe_core::KernelTemplateId, LoweringError> {
	let mut digest = ContractDigest::new("recipe-planner-stage-template-v1");
	digest.bytes(&program.digest.bytes());
	digest.u64(program.source_kernel.get());
	digest.u64(u64::from(stage.id.get()));
	let bytes = digest.finish().bytes();
	let mut prefix = [0_u8; 8];
	prefix.copy_from_slice(&bytes[..8]);
	let identity = recipe_core::KernelTemplateId::new(u64::from_le_bytes(prefix));
	ensure_stage(
		identity.get() != 0,
		"canonical stage-scoped kernel identity is the reserved zero value",
	)?;
	Ok(identity)
}

const fn access(mode: AccessMode) -> ArtifactBuildAccess {
	match mode {
		AccessMode::Read => ArtifactBuildAccess::Read,
		AccessMode::Write => ArtifactBuildAccess::Write,
		AccessMode::ReadWrite => ArtifactBuildAccess::ReadWrite,
		AccessMode::ReadWriteAtomic => ArtifactBuildAccess::ReadWriteAtomic,
	}
}

fn stage_error(message: impl Into<String>) -> LoweringError {
	LoweringError::new(LoweringErrorKind::InvalidStageContract, message)
}

fn ensure(condition: bool, kind: LoweringErrorKind, message: &'static str) -> Result<(), LoweringError> {
	match condition {
		true => Ok(()),
		false => Err(LoweringError::new(kind, message)),
	}
}

fn ensure_stage(condition: bool, message: &'static str) -> Result<(), LoweringError> {
	ensure(condition, LoweringErrorKind::InvalidStageContract, message)
}

#[derive(Debug)]
struct ContractDigest {
	hasher: Sha256,
}

impl ContractDigest {
	fn new(domain: &str) -> Self {
		let mut result = Self {
			hasher: Sha256::new(),
		};
		result.bytes(domain.as_bytes());
		result
	}

	fn bytes(&mut self, value: &[u8]) {
		self.hasher.update(value.len().to_string().as_bytes());
		self.hasher.update(b":");
		self.hasher.update(value);
	}

	fn digest(&mut self, value: Digest) {
		self.hasher.update(value.bytes());
	}

	fn length(&mut self, value: usize) {
		self.bytes(value.to_string().as_bytes());
	}

	fn u8(&mut self, value: u8) {
		self.hasher.update([value]);
	}

	fn u64(&mut self, value: u64) {
		self.hasher.update(value.to_le_bytes());
	}

	fn finish(self) -> Digest {
		let finalized = self.hasher.finalize();
		let mut bytes = [0_u8; 32];
		bytes.copy_from_slice(&finalized);
		Digest::new(bytes)
	}
}

#[derive(Clone, Debug)]
struct BoundPointer {
	dtype: DType,
	view: recipe_core::ArtifactBuildView,
	read: Option<String>,
	write: Option<String>,
	fault: bool,
}

#[derive(Debug)]
struct StageSignature {
	pointers: Vec<BoundPointer>,
	arguments: Vec<KernelArgument>,
	parameters: Vec<String>,
	has_run_id: bool,
	has_loop_iteration: bool,
}

impl StageSignature {
	fn new(
		stage: &ProgramStage,
		build: &ArtifactBuildRecipe,
		has_run_id: bool,
		has_loop_iteration: bool,
	) -> Result<Self, LoweringError> {
		let fault_buffer = stage.fault.map(|fault| fault.flag);
		let mut pointers = build
			.bindings
			.iter()
			.zip(&stage.bindings)
			.map(|(build, stage)| BoundPointer {
				dtype: build.dtype,
				view: build.view.clone(),
				read: None,
				write: None,
				fault: fault_buffer == Some(stage.buffer),
			})
			.collect::<Vec<_>>();
		let mut arguments = Vec::new();
		let mut parameters = Vec::new();
		for (input, (pointer, binding)) in pointers
			.iter_mut()
			.zip(&build.bindings)
			.filter(|(pointer, binding)| !pointer.fault && binding.access.reads())
			.enumerate()
		{
			let name = format!("%input_{input}");
			pointer.read = Some(name.clone());
			parameters.push(format!("ptr addrspace(1) {name}"));
			arguments.push(KernelArgument::Buffer {
				access: BufferAccess::Read,
				dtype: binding.dtype,
				alignment: 4,
			});
		}
		for (output, (pointer, binding)) in pointers
			.iter_mut()
			.zip(&build.bindings)
			.filter(|(pointer, binding)| !pointer.fault && binding.access.writes())
			.enumerate()
		{
			let name = format!("%output_{output}");
			pointer.write = Some(name.clone());
			parameters.push(format!("ptr addrspace(1) {name}"));
			arguments.push(KernelArgument::Buffer {
				access: BufferAccess::Write,
				dtype: binding.dtype,
				alignment: 4,
			});
		}
		let fault_arguments = usize::from(stage.fault.is_some());
		parameters.extend(std::iter::repeat_n(
			"ptr addrspace(1) %fault_flag".to_owned(),
			fault_arguments,
		));
		arguments.extend(std::iter::repeat_n(
			KernelArgument::FaultFlag,
			fault_arguments,
		));
		parameters.extend(std::iter::repeat_n(
			"i64 %run_id".to_owned(),
			usize::from(has_run_id),
		));
		arguments.extend(std::iter::repeat_n(
			KernelArgument::RunId,
			usize::from(has_run_id),
		));
		parameters.extend(std::iter::repeat_n(
			"i64 %loop_iteration".to_owned(),
			usize::from(has_loop_iteration),
		));
		arguments.extend(std::iter::repeat_n(
			KernelArgument::LoopIteration,
			usize::from(has_loop_iteration),
		));
		parameters.push("i64 %element_count".to_owned());
		arguments.push(KernelArgument::ElementCount);
		Ok(Self {
			pointers,
			arguments,
			parameters,
			has_run_id,
			has_loop_iteration,
		})
	}

	fn pointer(&self, index: usize) -> Result<&BoundPointer, LoweringError> {
		self.pointers
			.get(index)
			.ok_or_else(|| stage_error(format!("stage binding {index} is absent")))
	}

	fn abi(&self, entry_symbol: &str, logical_lanes: u64, workgroup_lanes: u32) -> Result<KernelAbi, LoweringError> {
		let argument_count = match u32::try_from(self.arguments.len()) {
			Ok(count) => count,
			Err(error) => {
				return Err(LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					error.to_string(),
				));
			}
		};
		let argument_bytes = argument_count.checked_mul(8).ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				"stage kernel ABI size overflowed",
			)
		})?;
		let elements = ElementCount::new(logical_lanes).map_err(|error| {
			stage_error(format!(
				"stage logical lane count cannot form a kernel ABI: {error}"
			))
		})?;
		Ok(KernelAbi {
			entry_symbol: entry_symbol.to_owned(),
			arguments: self.arguments.clone(),
			argument_bytes,
			argument_alignment: 8,
			elements,
			workgroup_lanes,
		})
	}
}

#[derive(Debug)]
struct Ir {
	body: String,
	next: u64,
	declarations: BTreeMap<&'static str, &'static str>,
	globals: Vec<String>,
	helpers: BTreeMap<String, String>,
}

impl Ir {
	fn new() -> Self {
		Self {
			body: String::default(),
			next: 0,
			declarations: BTreeMap::new(),
			globals: Vec::new(),
			helpers: BTreeMap::new(),
		}
	}

	fn temporary(&mut self, purpose: &str) -> String {
		let result = format!("%{purpose}_{}", self.next);
		self.next += 1;
		result
	}

	fn label(&mut self, purpose: &str) -> String {
		let result = format!("{purpose}_{}", self.next);
		self.next += 1;
		result
	}

	fn line(&mut self, arguments: std::fmt::Arguments<'_>) {
		self.body.push_str(&arguments.to_string());
		self.body.push('\n');
	}

	fn declare(&mut self, name: &'static str, declaration: &'static str) {
		self.declarations.insert(name, declaration);
	}

	fn emit_global_index(&mut self, target: &KernelTarget, workgroup_lanes: u32) {
		match target {
			KernelTarget::Amd(_) => {
				self.declare(
					"llvm.amdgcn.workgroup.id.x",
					"declare i32 @llvm.amdgcn.workgroup.id.x()",
				);
				self.declare(
					"llvm.amdgcn.workitem.id.x",
					"declare i32 @llvm.amdgcn.workitem.id.x()",
				);
				self.line(format_args!(
					"  %local_id_32 = call i32 @llvm.amdgcn.workitem.id.x()"
				));
				self.line(format_args!(
					"  %group_id_32 = call i32 @llvm.amdgcn.workgroup.id.x()"
				));
			}
			KernelTarget::Nvidia(_) => {
				self.declare(
					"llvm.nvvm.read.ptx.sreg.ctaid.x",
					"declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()",
				);
				self.declare(
					"llvm.nvvm.read.ptx.sreg.tid.x",
					"declare i32 @llvm.nvvm.read.ptx.sreg.tid.x()",
				);
				self.line(format_args!(
					"  %local_id_32 = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()"
				));
				self.line(format_args!(
					"  %group_id_32 = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()"
				));
			}
		}
		self.line(format_args!("  %local_id = zext i32 %local_id_32 to i64"));
		self.line(format_args!("  %group_id = zext i32 %group_id_32 to i64"));
		self.line(format_args!(
			"  %group_base = mul i64 %group_id, {workgroup_lanes}"
		));
		self.line(format_args!(
			"  %global_id = add i64 %group_base, %local_id"
		));
	}

	fn barrier(&mut self, target: &KernelTarget) {
		match target {
			KernelTarget::Amd(_) => {
				self.declare(
					"llvm.amdgcn.s.barrier",
					"declare void @llvm.amdgcn.s.barrier()",
				);
				self.line(format_args!("  call void @llvm.amdgcn.s.barrier()"));
			}
			KernelTarget::Nvidia(_) => {
				self.declare("llvm.nvvm.barrier0", "declare void @llvm.nvvm.barrier0()");
				self.line(format_args!("  call void @llvm.nvvm.barrier0()"));
			}
		}
	}

	fn affine_index(&mut self, pointer: &BoundPointer, linear: &str, purpose: &str) -> String {
		let coordinates = self.coordinates(linear, &pointer.view.logical_extents, purpose);
		self.index_from_coordinates(pointer, &coordinates, purpose)
	}

	fn coordinates(&mut self, linear: &str, extents: &[u64], purpose: &str) -> Vec<String> {
		let mut divisor = extents.iter().copied().product::<u64>();
		extents
			.iter()
			.copied()
			.enumerate()
			.map(|(axis, extent)| {
				divisor /= extent;
				let quotient = match divisor {
					0 => unreachable!("tensor extents are nonzero"),
					1 => linear.to_owned(),
					2..=u64::MAX => {
						let value = self.temporary(&format!("{purpose}_axis_{axis}_quotient"));
						self.line(format_args!("  {value} = udiv i64 {linear}, {divisor}"));
						value
					}
				};
				match extent {
					0 => unreachable!("tensor extents are nonzero"),
					1 => "0".to_owned(),
					2..=u64::MAX => {
						let value = self.temporary(&format!("{purpose}_axis_{axis}"));
						self.line(format_args!("  {value} = urem i64 {quotient}, {extent}"));
						value
					}
				}
			})
			.collect()
	}

	fn index_from_coordinates(&mut self, pointer: &BoundPointer, coordinates: &[String], purpose: &str) -> String {
		let mut accumulated = match pointer.view.offset_elements {
			0 => None,
			offset => Some(offset.to_string()),
		};
		for (axis, (coordinate, stride)) in coordinates
			.iter()
			.zip(&pointer.view.strides)
			.enumerate()
			.filter(|(_, (coordinate, stride))| **stride != 0 && coordinate.as_str() != "0")
		{
			let contribution = match *stride {
				0 => unreachable!("zero strides were filtered"),
				1 => coordinate.clone(),
				2..=u64::MAX => {
					let value = self.temporary(&format!("{purpose}_axis_{axis}_offset"));
					self.line(format_args!("  {value} = mul i64 {coordinate}, {stride}"));
					value
				}
			};
			accumulated = Some(match accumulated {
				Some(previous) => {
					let value = self.temporary(&format!("{purpose}_index"));
					self.line(format_args!(
						"  {value} = add i64 {previous}, {contribution}"
					));
					value
				}
				None => contribution,
			});
		}
		accumulated.unwrap_or_else(|| "0".to_owned())
	}

	fn load(&mut self, pointer: &BoundPointer, index: &str, purpose: &str) -> Result<String, LoweringError> {
		let base = pointer
			.read
			.as_deref()
			.ok_or_else(|| stage_error(format!("{purpose} binding is not readable")))?;
		let address = self.temporary(&format!("{purpose}_address"));
		self.line(format_args!(
			"  {address} = getelementptr inbounds {}, ptr addrspace(1) {base}, i64 {index}",
			llvm_type(pointer.dtype)
		));
		let value = self.temporary(purpose);
		self.line(format_args!(
			"  {value} = load {}, ptr addrspace(1) {address}, align 4",
			llvm_type(pointer.dtype)
		));
		Ok(value)
	}

	fn store(
		&mut self,
		pointer: &BoundPointer,
		index: &str,
		value: &str,
		purpose: &str,
	) -> Result<(), LoweringError> {
		let base = pointer
			.write
			.as_deref()
			.ok_or_else(|| stage_error(format!("{purpose} binding is not writable")))?;
		let address = self.temporary(&format!("{purpose}_address"));
		self.line(format_args!(
			"  {address} = getelementptr inbounds {}, ptr addrspace(1) {base}, i64 {index}",
			llvm_type(pointer.dtype)
		));
		self.line(format_args!(
			"  store {} {value}, ptr addrspace(1) {address}, align 4",
			llvm_type(pointer.dtype)
		));
		Ok(())
	}

	fn write_address(&mut self, pointer: &BoundPointer, index: &str, purpose: &str) -> Result<String, LoweringError> {
		let base = pointer
			.write
			.as_deref()
			.ok_or_else(|| stage_error(format!("{purpose} binding is not writable")))?;
		let address = self.temporary(&format!("{purpose}_address"));
		self.line(format_args!(
			"  {address} = getelementptr inbounds {}, ptr addrspace(1) {base}, i64 {index}",
			llvm_type(pointer.dtype)
		));
		Ok(address)
	}
}

fn llvm_type(dtype: DType) -> &'static str {
	match dtype {
		DType::F32 => "float",
		DType::I32 => "i32",
	}
}

const fn private_address_space(target: &KernelTarget) -> u32 {
	match target {
		KernelTarget::Amd(_) | KernelTarget::Nvidia(_) => 5,
	}
}

fn lower_owned_stage(
	program: &LoweredProgram,
	stage: &ProgramStage,
	kind: &StageKind,
	build: &ArtifactBuildRecipe,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<LoweredKernel, LoweringError> {
	let has_run_id = matches!(kind, StageKind::Philox4x32_10(_));
	let has_loop_iteration = matches!(kind, StageKind::Philox4x32_10(_))
		|| matches!(kind, StageKind::IndexMap(spec) if spec.iteration_step != 0);
	let signature = StageSignature::new(stage, build, has_run_id, has_loop_iteration)?;
	let mut ir = Ir::new();
	ir.emit_global_index(target, stage.geometry.workgroup_lanes);
	match kind {
		StageKind::Fill { value } => emit_fill(&mut ir, &signature, *value)?,
		StageKind::Copy { .. } => emit_copy(&mut ir, &signature)?,
		StageKind::FixedTreeReduce(reduction) => {
			emit_reduction(&mut ir, &signature, stage, reduction, target, options)?
		}
		StageKind::FixedTreeScanLocal(scan) => emit_scan_local(&mut ir, &signature, stage, scan, target, options)?,
		StageKind::ScanUniformCombine(scan) => {
			let axis = scan_axis(program, stage, scan.level)?;
			emit_scan_uniform(&mut ir, &signature, scan, axis)?
		}
		StageKind::TiledContraction(contraction) => {
			emit_contraction(&mut ir, &signature, contraction, target, options)?
		}
		StageKind::Gather { axis, bounds } => emit_gather(&mut ir, &signature, stage.fault, *axis, *bounds)?,
		StageKind::Scatter {
			axis,
			bounds,
			conflict,
		} => emit_scatter(
			&mut ir,
			&signature,
			stage.fault,
			stage,
			*axis,
			*bounds,
			*conflict,
		)?,
		StageKind::HistogramClear => emit_zero(&mut ir, &signature)?,
		StageKind::HistogramAccumulate {
			bins,
			weighted,
			bin_mapping,
			ordering,
		} => emit_histogram(
			&mut ir,
			&signature,
			stage,
			HistogramContract {
				fault: stage
					.fault
					.ok_or_else(|| stage_error("histogram stage omitted its checked fault contract"))?,
				bins: *bins,
				weighted: *weighted,
				mapping: *bin_mapping,
				ordering: *ordering,
			},
		)?,
		StageKind::StableSortInitialize { network } => emit_sort_initialize(&mut ir, &signature, *network)?,
		StageKind::StableSortCompareExchange(compare) => emit_sort_compare(&mut ir, &signature, *compare)?,
		StageKind::StableSortFinalize {
			network,
			emit_indices,
		} => emit_sort_finalize(&mut ir, &signature, *network, *emit_indices)?,
		StageKind::IndexMap(spec) => emit_index_map(&mut ir, &signature, stage, *spec)?,
		StageKind::Philox4x32_10(contract) => emit_philox(&mut ir, &signature, build, contract, target)?,
		StageKind::ScalarMap { .. } => {
			return Err(stage_error(
				"scalar-map stage reached the owned-stage emitter",
			));
		}
	}
	let abi = signature.abi(
		&options.entry_symbol,
		stage.geometry.logical_lanes,
		stage.geometry.workgroup_lanes,
	)?;
	let llvm_ir = assemble_stage_module(target, options, &signature, &ir);
	let findings = audit_llvm_ir(&llvm_ir);
	match findings.first() {
		Some(finding) => Err(LoweringError::new(
			LoweringErrorKind::ProhibitedInterface,
			format!(
				"generated stage LLVM IR failed audit at line {}: {:?} `{}`",
				finding.line, finding.kind, finding.token
			),
		)),
		None => Ok(()),
	}?;
	let lowered = LoweredKernel {
		llvm_ir,
		abi,
		work: build.work.flops,
		target: target.clone(),
	};
	validate_realized_kernel(&lowered, stage, build)?;
	Ok(lowered)
}

fn validate_realized_kernel(
	lowered: &LoweredKernel,
	stage: &ProgramStage,
	build: &ArtifactBuildRecipe,
) -> Result<(), LoweringError> {
	ensure_stage(
		lowered.abi.elements.get() == stage.geometry.logical_lanes
			&& lowered.abi.workgroup_lanes == stage.geometry.workgroup_lanes
			&& lowered.work == build.work.flops,
		"realized kernel ABI or work differs from the immutable build recipe",
	)?;
	let expected_faults = usize::from(stage.fault.is_some());
	ensure_stage(
		lowered
			.abi
			.arguments
			.iter()
			.filter(|argument| matches!(argument, KernelArgument::FaultFlag))
			.count() == expected_faults,
		"realized kernel fault ABI differs from the immutable stage",
	)?;
	Ok(())
}

fn assemble_stage_module(
	target: &KernelTarget,
	options: &LoweringOptions,
	signature: &StageSignature,
	ir: &Ir,
) -> String {
	let mut module = String::default();
	match target {
		KernelTarget::Amd(_) => module.push_str("target triple = \"amdgcn-amd-amdhsa\"\n\n"),
		KernelTarget::Nvidia(_) => module.push_str("target triple = \"nvptx64-nvidia-cuda\"\n\n"),
	}
	for declaration in ir.declarations.values() {
		module.push_str(declaration);
		module.push('\n');
	}
	push_section_gap(&mut module, !ir.declarations.is_empty());
	for global in &ir.globals {
		module.push_str(global);
		module.push('\n');
	}
	push_section_gap(&mut module, !ir.globals.is_empty());
	for helper in ir.helpers.values() {
		module.push_str(helper.trim_end_matches('\n'));
		module.push_str("\n\n");
	}
	let convention = match target {
		KernelTarget::Amd(_) => "amdgpu_kernel",
		KernelTarget::Nvidia(_) => "ptx_kernel",
	};
	module.push_str(&format!(
		"define {convention} void @{}({}) #0 {{",
		options.entry_symbol,
		signature.parameters.join(", ")
	));
	module.push('\n');
	module.push_str("entry:\n");
	module.push_str(&ir.body);
	module.push_str("}\n\n");
	module.push_str(
		"attributes #0 = { nounwind strictfp \"denormal-fp-math-f32\"=\"ieee\" \"no-trapping-math\"=\"false\" }\n",
	);
	module
}

fn push_section_gap(module: &mut String, present: bool) {
	module.extend(std::iter::repeat_n('\n', usize::from(present)));
}

fn begin_lane(ir: &mut Ir) {
	ir.line(format_args!(
		"  %in_bounds = icmp ult i64 %global_id, %element_count"
	));
	ir.line(format_args!("  br i1 %in_bounds, label %body, label %exit"));
	ir.line(format_args!("body:"));
}

fn finish_lane(ir: &mut Ir) {
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("exit:"));
	ir.line(format_args!("  ret void"));
}

fn emit_fill(ir: &mut Ir, signature: &StageSignature, value: ScalarLiteral) -> Result<(), LoweringError> {
	let output = signature.pointer(0)?;
	ensure_stage(
		output.dtype == value.dtype(),
		"fill literal type differs from its output",
	)?;
	begin_lane(ir);
	let index = ir.affine_index(output, "%global_id", "fill");
	let value = literal_operand(ir, value, "fill_value");
	ir.store(output, &index, &value, "fill")?;
	finish_lane(ir);
	Ok(())
}

fn emit_zero(ir: &mut Ir, signature: &StageSignature) -> Result<(), LoweringError> {
	let output = signature.pointer(0)?;
	begin_lane(ir);
	let index = ir.affine_index(output, "%global_id", "zero");
	let zero = match output.dtype {
		DType::F32 => "0.000000e+00",
		DType::I32 => "0",
	};
	ir.store(output, &index, zero, "zero")?;
	finish_lane(ir);
	Ok(())
}

fn emit_copy(ir: &mut Ir, signature: &StageSignature) -> Result<(), LoweringError> {
	let input = signature.pointer(0)?;
	let output = signature.pointer(1)?;
	ensure_stage(
		input.dtype == output.dtype,
		"copy input and output types differ",
	)?;
	begin_lane(ir);
	let input_index = ir.affine_index(input, "%global_id", "copy_input");
	let output_index = ir.affine_index(output, "%global_id", "copy_output");
	let value = ir.load(input, &input_index, "copy_value")?;
	ir.store(output, &output_index, &value, "copy_value")?;
	finish_lane(ir);
	Ok(())
}

fn emit_index_map(
	ir: &mut Ir,
	signature: &StageSignature,
	stage: &ProgramStage,
	spec: IndexMap,
) -> Result<(), LoweringError> {
	let output = signature.pointer(0)?;
	let fault = stage
		.fault
		.ok_or_else(|| stage_error("index-map stage omitted its checked fault contract"))?;
	ensure_stage(
		output.dtype == DType::I32
			&& !signature.has_run_id
			&& signature.has_loop_iteration == (spec.iteration_step != 0)
			&& fault.reason == recipe_primitives::FaultReason::ArithmeticDomain
			&& fault.code == 4,
		"index-map type, dynamic ABI, or arithmetic-fault contract is invalid",
	)?;
	ensure_stage(
		spec.modulus.is_none_or(|modulus| modulus > 0),
		"index-map modulus is not strictly positive",
	)?;

	begin_lane(ir);
	let mut rejected = Vec::new();
	let element = checked_unsigned_scale(
		ir,
		"%global_id",
		spec.element_step,
		"index_map_element",
		&mut rejected,
	);
	let iteration = match spec.iteration_step {
		0 => "0".to_owned(),
		step => checked_unsigned_scale(
			ir,
			"%loop_iteration",
			step,
			"index_map_iteration",
			&mut rejected,
		),
	};
	let with_element = checked_signed_add(
		ir,
		&spec.start.to_string(),
		&element,
		"index_map_start_element",
		&mut rejected,
	);
	let affine = checked_signed_add(
		ir,
		&with_element,
		&iteration,
		"index_map_affine",
		&mut rejected,
	);
	let result = match spec.modulus {
		Some(modulus) => {
			let remainder = ir.temporary("index_map_remainder");
			ir.line(format_args!("  {remainder} = srem i64 {affine}, {modulus}"));
			let negative = ir.temporary("index_map_remainder_negative");
			ir.line(format_args!("  {negative} = icmp slt i64 {remainder}, 0"));
			let corrected = ir.temporary("index_map_remainder_corrected");
			ir.line(format_args!(
				"  {corrected} = add i64 {remainder}, {modulus}"
			));
			let euclidean = ir.temporary("index_map_euclidean");
			ir.line(format_args!(
				"  {euclidean} = select i1 {negative}, i64 {corrected}, i64 {remainder}"
			));
			let narrowed = ir.temporary("index_map_result");
			ir.line(format_args!("  {narrowed} = trunc i64 {euclidean} to i32"));
			narrowed
		}
		None => {
			let below = ir.temporary("index_map_below_i32");
			ir.line(format_args!(
				"  {below} = icmp slt i64 {affine}, {}",
				i32::MIN
			));
			rejected.push(below);
			let above = ir.temporary("index_map_above_i32");
			ir.line(format_args!(
				"  {above} = icmp sgt i64 {affine}, {}",
				i32::MAX
			));
			rejected.push(above);
			let narrowed = ir.temporary("index_map_result");
			ir.line(format_args!("  {narrowed} = trunc i64 {affine} to i32"));
			narrowed
		}
	};

	let rejected = rejected
		.into_iter()
		.fold("false".to_owned(), |left, right| {
			let combined = ir.temporary("index_map_rejected");
			ir.line(format_args!("  {combined} = or i1 {left}, {right}"));
			combined
		});
	let publish = ir.label("index_map_publish_fault");
	let write = ir.label("index_map_write");
	ir.line(format_args!(
		"  br i1 {rejected}, label %{publish}, label %{write}"
	));
	ir.line(format_args!("{publish}:"));
	let previous = ir.temporary("index_map_fault_previous");
	ir.line(format_args!(
		"  {previous} = atomicrmw xchg ptr addrspace(1) %fault_flag, i32 {} release, align 4",
		fault.code
	));
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("{write}:"));
	let output_index = ir.affine_index(output, "%global_id", "index_map_output");
	ir.store(output, &output_index, &result, "index_map_output")?;
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("exit:"));
	ir.line(format_args!("  ret void"));
	Ok(())
}

fn checked_unsigned_scale(
	ir: &mut Ir,
	unsigned: &str,
	scale: i32,
	purpose: &str,
	rejected: &mut Vec<String>,
) -> String {
	if scale == 0 {
		return "0".to_owned();
	}
	ir.declare(
		"llvm.smul.with.overflow.i64",
		"declare { i64, i1 } @llvm.smul.with.overflow.i64(i64, i64)",
	);
	let outside_signed = ir.temporary(&format!("{purpose}_outside_signed"));
	ir.line(format_args!(
		"  {outside_signed} = icmp ugt i64 {unsigned}, {}",
		i64::MAX
	));
	rejected.push(outside_signed);
	let pair = ir.temporary(&format!("{purpose}_pair"));
	ir.line(format_args!(
		"  {pair} = call {{ i64, i1 }} @llvm.smul.with.overflow.i64(i64 {unsigned}, i64 {scale})"
	));
	let value = ir.temporary(&format!("{purpose}_value"));
	ir.line(format_args!(
		"  {value} = extractvalue {{ i64, i1 }} {pair}, 0"
	));
	let overflow = ir.temporary(&format!("{purpose}_overflow"));
	ir.line(format_args!(
		"  {overflow} = extractvalue {{ i64, i1 }} {pair}, 1"
	));
	rejected.push(overflow);
	value
}

fn checked_signed_add(ir: &mut Ir, left: &str, right: &str, purpose: &str, rejected: &mut Vec<String>) -> String {
	ir.declare(
		"llvm.sadd.with.overflow.i64",
		"declare { i64, i1 } @llvm.sadd.with.overflow.i64(i64, i64)",
	);
	let pair = ir.temporary(&format!("{purpose}_pair"));
	ir.line(format_args!(
		"  {pair} = call {{ i64, i1 }} @llvm.sadd.with.overflow.i64(i64 {left}, i64 {right})"
	));
	let value = ir.temporary(&format!("{purpose}_value"));
	ir.line(format_args!(
		"  {value} = extractvalue {{ i64, i1 }} {pair}, 0"
	));
	let overflow = ir.temporary(&format!("{purpose}_overflow"));
	ir.line(format_args!(
		"  {overflow} = extractvalue {{ i64, i1 }} {pair}, 1"
	));
	rejected.push(overflow);
	value
}

fn literal_operand(ir: &mut Ir, literal: ScalarLiteral, purpose: &str) -> String {
	match literal {
		ScalarLiteral::I32(value) => value.to_string(),
		ScalarLiteral::F32Bits(bits) => {
			let value = ir.temporary(purpose);
			ir.line(format_args!("  {value} = bitcast i32 {bits} to float"));
			value
		}
	}
}

fn scan_axis(program: &LoweredProgram, stage: &ProgramStage, level: u32) -> Result<usize, LoweringError> {
	let mut axes = program
		.stages
		.iter()
		.filter(|candidate| candidate.id != stage.id)
		.filter_map(|candidate| scan_local_axis(&candidate.kind, level))
		.collect::<Vec<_>>();
	axes.sort_unstable();
	axes.dedup();
	match axes.as_slice() {
		[axis] => Ok(*axis),
		[] => Err(stage_error(
			"uniform scan stage has no same-level local stage carrying its axis",
		)),
		[_, _, ..] => Err(stage_error(
			"uniform scan stage has ambiguous same-level scan axes",
		)),
	}
}

fn scan_local_axis(kind: &StageKind, level: u32) -> Option<usize> {
	match kind {
		StageKind::FixedTreeScanLocal(local) => (local.level == level).then_some(local.axis),
		StageKind::ScalarMap { .. }
		| StageKind::Fill { .. }
		| StageKind::Copy { .. }
		| StageKind::FixedTreeReduce(_)
		| StageKind::ScanUniformCombine(_)
		| StageKind::TiledContraction(_)
		| StageKind::Gather { .. }
		| StageKind::Scatter { .. }
		| StageKind::HistogramClear
		| StageKind::HistogramAccumulate { .. }
		| StageKind::StableSortInitialize { .. }
		| StageKind::StableSortCompareExchange(_)
		| StageKind::StableSortFinalize { .. }
		| StageKind::IndexMap(_)
		| StageKind::Philox4x32_10(_) => None,
	}
}

fn emit_reduction(
	ir: &mut Ir,
	signature: &StageSignature,
	stage: &ProgramStage,
	reduction: &ReductionStage,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<(), LoweringError> {
	ensure_stage(
		reduction.result == ReduceResult::Value || reduction.input_width <= 2_147_483_647_u64,
		"reduction index domain exceeds the int32 result contract",
	)?;
	validate_tree_contract(&reduction.tree, stage, TreePhase::Reduction)?;
	ensure_stage(
		reduction.padding == ReductionPadding::OperatorIdentity
			&& reduction.tie_break == ReductionTieBreak::LowestLogicalIndex,
		"reduction padding or tie-break contract is not Recipe canonical",
	)?;
	let reads = signature
		.pointers
		.iter()
		.enumerate()
		.filter(|(_, pointer)| !pointer.fault && pointer.read.is_some())
		.map(|(index, _)| index)
		.collect::<Vec<_>>();
	let writes = signature
		.pointers
		.iter()
		.enumerate()
		.filter(|(_, pointer)| !pointer.fault && pointer.write.is_some())
		.map(|(index, _)| index)
		.collect::<Vec<_>>();
	let value_input = signature.pointer(
		*reads.first()
			.ok_or_else(|| stage_error("reduction stage has no readable value binding"))?,
	)?;
	ensure_stage(
		value_input.dtype == reduction.dtype,
		"reduction value input type differs from its stage contract",
	)?;
	let has_index = reduction.result != ReduceResult::Value;
	let index_input =
		match has_index && reduction.pass != 0 {
			true => Some(signature
				.pointer(*reads.get(1).ok_or_else(|| {
					stage_error("later reduction pass omitted its readable index binding")
				})?)?),
			false => None,
		};
	let final_pass = reduction.output_width == 1;
	let (value_output, index_output) = match (final_pass, reduction.result) {
		(_, ReduceResult::Value) => (
			Some(signature.pointer(
				*writes
					.first()
					.ok_or_else(|| stage_error("value reduction omitted its writable result"))?,
			)?),
			None,
		),
		(true, ReduceResult::Index) => (
			None,
			Some(signature.pointer(
				*writes
					.first()
					.ok_or_else(|| stage_error("index reduction omitted its writable result"))?,
			)?),
		),
		(false, ReduceResult::Index) | (_, ReduceResult::ValueAndIndex) => (
			Some(signature.pointer(
				*writes
					.first()
					.ok_or_else(|| stage_error("indexed reduction omitted its writable value"))?,
			)?),
			Some(signature.pointer(
				*writes
					.get(1)
					.ok_or_else(|| stage_error("indexed reduction omitted its writable index"))?,
			)?),
		),
	};
	ensure_stage(
		index_input
			.into_iter()
			.chain(index_output)
			.all(|pointer| pointer.dtype == DType::I32),
		"reduction index binding is not int32",
	)?;

	let value_global = format!("{}_reduction_values", options.entry_symbol);
	ir.globals.push(format!(
		"@{value_global} = internal addrspace(3) global [{} x i32] undef, align 4",
		reduction.tree.lanes
	));
	let index_global = has_index.then(|| {
		let name = format!("{}_reduction_indices", options.entry_symbol);
		ir.globals.push(format!(
			"@{name} = internal addrspace(3) global [{} x i32] undef, align 4",
			reduction.tree.lanes
		));
		name
	});

	begin_lane(ir);
	let block = ir.temporary("reduction_block");
	ir.line(format_args!(
		"  {block} = urem i64 %group_id, {}",
		reduction.output_width
	));
	let sequence = ir.temporary("reduction_sequence");
	ir.line(format_args!(
		"  {sequence} = udiv i64 %group_id, {}",
		reduction.output_width
	));
	let block_base = ir.temporary("reduction_block_base");
	ir.line(format_args!(
		"  {block_base} = mul i64 {block}, {}",
		reduction.tree.lanes
	));
	let reduced_linear = ir.temporary("reduction_linear");
	ir.line(format_args!(
		"  {reduced_linear} = add i64 {block_base}, %local_id"
	));
	let active = ir.temporary("reduction_active");
	ir.line(format_args!(
		"  {active} = icmp ult i64 {reduced_linear}, {}",
		reduction.input_width
	));
	let safe_reduced = ir.temporary("reduction_safe_linear");
	ir.line(format_args!(
		"  {safe_reduced} = select i1 {active}, i64 {reduced_linear}, i64 0"
	));
	let input_index = reduction_input_index(ir, value_input, reduction, &sequence, &safe_reduced)?;
	let loaded = ir.load(value_input, &input_index, "reduction_input")?;
	let identity = reduction_identity(
		ir,
		reduction.operator,
		reduction.dtype,
		"reduction_identity",
	)?;
	let initial = ir.temporary("reduction_initial");
	ir.line(format_args!(
		"  {initial} = select i1 {active}, {} {loaded}, {} {identity}",
		llvm_type(reduction.dtype),
		llvm_type(reduction.dtype)
	));
	shared_store(
		ir,
		&value_global,
		reduction.tree.lanes,
		"%local_id",
		reduction.dtype,
		&initial,
		"reduction_shared_value",
	);

	initialize_reduction_index(
		ir,
		index_global.as_deref(),
		index_input,
		reduction,
		&sequence,
		&safe_reduced,
		&active,
	)?;
	ir.barrier(target);

	for (step_index, step) in reduction.tree.steps.iter().enumerate() {
		let combine = ir.label("reduction_combine");
		let joined = ir.label("reduction_join");
		let participates = ir.temporary("reduction_participates");
		ir.line(format_args!(
			"  {participates} = icmp ult i64 %local_id, {}",
			step.stride
		));
		ir.line(format_args!(
			"  br i1 {participates}, label %{combine}, label %{joined}"
		));
		ir.line(format_args!("{combine}:"));
		let right_lane = ir.temporary("reduction_right_lane");
		ir.line(format_args!(
			"  {right_lane} = add i64 %local_id, {}",
			step.stride
		));
		let left_value = shared_load(
			ir,
			&value_global,
			reduction.tree.lanes,
			"%local_id",
			reduction.dtype,
			"reduction_left",
		);
		let right_value = shared_load(
			ir,
			&value_global,
			reduction.tree.lanes,
			&right_lane,
			reduction.dtype,
			"reduction_right",
		);
		let left_index = index_global.as_ref().map(|global| {
			shared_load(
				ir,
				global,
				reduction.tree.lanes,
				"%local_id",
				DType::I32,
				"reduction_left_index",
			)
		});
		let right_index = index_global.as_ref().map(|global| {
			shared_load(
				ir,
				global,
				reduction.tree.lanes,
				&right_lane,
				DType::I32,
				"reduction_right_index",
			)
		});
		let (combined, combined_index) = emit_reduce_combine(
			ir,
			reduction.operator,
			reduction.dtype,
			&left_value,
			&right_value,
			left_index.as_deref(),
			right_index.as_deref(),
		)?;
		shared_store(
			ir,
			&value_global,
			reduction.tree.lanes,
			"%local_id",
			reduction.dtype,
			&combined,
			"reduction_combined",
		);
		store_reduction_combined_index(
			ir,
			index_global.as_deref(),
			combined_index.as_deref(),
			reduction.tree.lanes,
		)?;
		ir.line(format_args!("  br label %{joined}"));
		ir.line(format_args!("{joined}:"));
		ir.barrier(target);
		let expected_step = match u32::try_from(step_index) {
			Ok(step) => step,
			Err(error) => return Err(stage_error(error.to_string())),
		};
		ensure_stage(
			stage.synchronization[step_index].after_step == expected_step,
			"reduction synchronization ordinal differs from its tree step",
		)?;
	}

	let write = ir.label("reduction_write");
	let done = ir.label("reduction_done");
	let leader = ir.temporary("reduction_leader");
	ir.line(format_args!("  {leader} = icmp eq i64 %local_id, 0"));
	ir.line(format_args!(
		"  br i1 {leader}, label %{write}, label %{done}"
	));
	ir.line(format_args!("{write}:"));
	let output_linear_base = ir.temporary("reduction_output_base");
	ir.line(format_args!(
		"  {output_linear_base} = mul i64 {sequence}, {}",
		reduction.output_width
	));
	let output_linear = ir.temporary("reduction_output_linear");
	ir.line(format_args!(
		"  {output_linear} = add i64 {output_linear_base}, {block}"
	));
	store_reduction_outputs(
		ir,
		ReductionOutputs {
			value: value_output,
			index: index_output,
			value_global: &value_global,
			index_global: index_global.as_deref(),
			linear: &output_linear,
			lanes: reduction.tree.lanes,
			dtype: reduction.dtype,
		},
	)?;
	ir.line(format_args!("  br label %{done}"));
	ir.line(format_args!("{done}:"));
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("exit:"));
	ir.line(format_args!("  ret void"));
	Ok(())
}

fn initialize_reduction_index(
	ir: &mut Ir,
	index_global: Option<&str>,
	index_input: Option<&BoundPointer>,
	reduction: &ReductionStage,
	sequence: &str,
	safe_reduced: &str,
	active: &str,
) -> Result<(), LoweringError> {
	let index_global = match index_global {
		Some(global) => global,
		None => return Ok(()),
	};
	let raw_index = match index_input {
		Some(pointer) => {
			let flat = ir.temporary("reduction_index_input_linear");
			ir.line(format_args!(
				"  {flat} = mul i64 {sequence}, {}",
				reduction.input_width
			));
			let flat_index = ir.temporary("reduction_index_input_index");
			ir.line(format_args!(
				"  {flat_index} = add i64 {flat}, {safe_reduced}"
			));
			let physical = ir.affine_index(pointer, &flat_index, "reduction_index_input");
			ir.load(pointer, &physical, "reduction_source_index")?
		}
		None => {
			let converted = ir.temporary("reduction_source_index");
			ir.line(format_args!(
				"  {converted} = trunc i64 {safe_reduced} to i32"
			));
			converted
		}
	};
	let initial_index = ir.temporary("reduction_initial_index");
	ir.line(format_args!(
		"  {initial_index} = select i1 {active}, i32 {raw_index}, i32 2147483647"
	));
	shared_store(
		ir,
		index_global,
		reduction.tree.lanes,
		"%local_id",
		DType::I32,
		&initial_index,
		"reduction_shared_index",
	);
	Ok(())
}

fn store_reduction_combined_index(
	ir: &mut Ir,
	global: Option<&str>,
	combined_index: Option<&str>,
	lanes: u32,
) -> Result<(), LoweringError> {
	match (global, combined_index) {
		(Some(global), Some(combined_index)) => {
			shared_store(
				ir,
				global,
				lanes,
				"%local_id",
				DType::I32,
				combined_index,
				"reduction_combined_index",
			);
			Ok(())
		}
		(None, None) => Ok(()),
		(Some(_), None) | (None, Some(_)) => Err(stage_error(
			"reduction index combine contract is incomplete",
		)),
	}
}

#[derive(Clone, Copy, Debug)]
struct ReductionOutputs<'a> {
	value: Option<&'a BoundPointer>,
	index: Option<&'a BoundPointer>,
	value_global: &'a str,
	index_global: Option<&'a str>,
	linear: &'a str,
	lanes: u32,
	dtype: DType,
}

fn store_reduction_outputs(ir: &mut Ir, outputs: ReductionOutputs<'_>) -> Result<(), LoweringError> {
	match outputs.value {
		Some(output) => {
			let index = ir.affine_index(output, outputs.linear, "reduction_value_output");
			let value = shared_load(
				ir,
				outputs.value_global,
				outputs.lanes,
				"0",
				outputs.dtype,
				"reduction_result",
			);
			ir.store(output, &index, &value, "reduction_result")
		}
		None => Ok(()),
	}?;
	match (outputs.index, outputs.index_global) {
		(Some(output), Some(global)) => {
			let index = ir.affine_index(output, outputs.linear, "reduction_index_output");
			let value = shared_load(
				ir,
				global,
				outputs.lanes,
				"0",
				DType::I32,
				"reduction_index_result",
			);
			ir.store(output, &index, &value, "reduction_index_result")
		}
		(None, None) => Ok(()),
		(Some(_), None) | (None, Some(_)) => Err(stage_error("reduction index output contract is incomplete")),
	}
}

fn validate_tree_contract(tree: &FixedTree, stage: &ProgramStage, phase: TreePhase) -> Result<(), LoweringError> {
	ensure_stage(
		tree.lanes == stage.geometry.workgroup_lanes && tree.steps.len() == stage.synchronization.len(),
		"fixed tree differs from launch geometry or synchronization count",
	)?;
	for (step, synchronization) in tree.steps.iter().zip(&stage.synchronization) {
		ensure_stage(
			(phase != TreePhase::Reduction || step.phase == TreePhase::Reduction)
				&& synchronization.scope == SynchronizationScope::Workgroup
				&& synchronization.memory == MemorySemantics::SharedAcquireRelease,
			"fixed tree synchronization semantics are not canonical",
		)?;
	}
	Ok(())
}

fn reduction_input_index(
	ir: &mut Ir,
	input: &BoundPointer,
	reduction: &ReductionStage,
	sequence: &str,
	reduced: &str,
) -> Result<String, LoweringError> {
	match reduction.pass {
		0 => reduction_first_pass_index(ir, input, reduction, sequence, reduced),
		1..=u32::MAX => {
			let base = ir.temporary("reduction_flat_input_base");
			ir.line(format_args!(
				"  {base} = mul i64 {sequence}, {}",
				reduction.input_width
			));
			let linear = ir.temporary("reduction_flat_input");
			ir.line(format_args!("  {linear} = add i64 {base}, {reduced}"));
			Ok(ir.affine_index(input, &linear, "reduction_input"))
		}
	}
}

fn reduction_first_pass_index(
	ir: &mut Ir,
	input: &BoundPointer,
	reduction: &ReductionStage,
	sequence: &str,
	reduced: &str,
) -> Result<String, LoweringError> {
	let rank = input.view.logical_extents.len();
	ensure_stage(
		reduction.reduced_axes.iter().all(|axis| *axis < rank),
		"reduction axis exceeds its first-pass input rank",
	)?;
	let free_extents = input
		.view
		.logical_extents
		.iter()
		.copied()
		.enumerate()
		.filter(|(axis, _)| !reduction.reduced_axes.contains(axis))
		.map(|(_, extent)| extent)
		.collect::<Vec<_>>();
	let reduced_extents = reduction
		.reduced_axes
		.iter()
		.map(|axis| input.view.logical_extents[*axis])
		.collect::<Vec<_>>();
	let free_coordinates = ir.coordinates(sequence, &free_extents, "reduction_sequence");
	let reduced_coordinates = ir.coordinates(reduced, &reduced_extents, "reduction_axis");
	let mut free = free_coordinates.into_iter();
	let mut reduced = reduced_coordinates.into_iter();
	let mut coordinates = Vec::with_capacity(rank);
	for axis in 0..rank {
		let coordinate = match reduction
			.reduced_axes
			.iter()
			.position(|candidate| *candidate == axis)
		{
			Some(_) => reduced
				.next()
				.ok_or_else(|| stage_error("reduction coordinate underflow"))?,
			None => free
				.next()
				.ok_or_else(|| stage_error("free reduction coordinate underflow"))?,
		};
		coordinates.push(coordinate);
	}
	Ok(ir.index_from_coordinates(input, &coordinates, "reduction_input"))
}

fn reduction_identity(
	ir: &mut Ir,
	operator: ReduceOperator,
	dtype: DType,
	purpose: &str,
) -> Result<String, LoweringError> {
	let literal = match (operator, dtype) {
		(ReduceOperator::Sum, DType::F32) => ScalarLiteral::F32Bits(0),
		(ReduceOperator::Product, DType::F32) => ScalarLiteral::F32Bits(1.0_f32.to_bits()),
		(ReduceOperator::Minimum, DType::F32) => ScalarLiteral::F32Bits(f32::INFINITY.to_bits()),
		(ReduceOperator::Maximum, DType::F32) => ScalarLiteral::F32Bits(f32::NEG_INFINITY.to_bits()),
		(ReduceOperator::Sum, DType::I32) => ScalarLiteral::I32(0),
		(ReduceOperator::Product, DType::I32) => ScalarLiteral::I32(1),
		(ReduceOperator::Minimum, DType::I32) => ScalarLiteral::I32(i32::MAX),
		(ReduceOperator::Maximum, DType::I32) => ScalarLiteral::I32(i32::MIN),
		(ReduceOperator::Any, DType::I32) => ScalarLiteral::I32(0),
		(ReduceOperator::All, DType::I32) => ScalarLiteral::I32(1),
		(ReduceOperator::Any | ReduceOperator::All, DType::F32) => {
			return Err(stage_error("Any and All fixed trees require int32"));
		}
	};
	Ok(literal_operand(ir, literal, purpose))
}

fn shared_store(ir: &mut Ir, global: &str, lanes: u32, index: &str, dtype: DType, value: &str, purpose: &str) {
	let bits = match dtype {
		DType::I32 => value.to_owned(),
		DType::F32 => {
			let bits = ir.temporary(&format!("{purpose}_bits"));
			ir.line(format_args!("  {bits} = bitcast float {value} to i32"));
			bits
		}
	};
	let address = ir.temporary(&format!("{purpose}_address"));
	ir.line(format_args!(
		"  {address} = getelementptr inbounds [{lanes} x i32], ptr addrspace(3) @{global}, i64 0, i64 {index}"
	));
	ir.line(format_args!(
		"  store i32 {bits}, ptr addrspace(3) {address}, align 4"
	));
}

fn shared_load(ir: &mut Ir, global: &str, lanes: u32, index: &str, dtype: DType, purpose: &str) -> String {
	let address = ir.temporary(&format!("{purpose}_address"));
	ir.line(format_args!(
		"  {address} = getelementptr inbounds [{lanes} x i32], ptr addrspace(3) @{global}, i64 0, i64 {index}"
	));
	let bits = ir.temporary(&format!("{purpose}_bits"));
	ir.line(format_args!(
		"  {bits} = load i32, ptr addrspace(3) {address}, align 4"
	));
	match dtype {
		DType::I32 => bits,
		DType::F32 => {
			let value = ir.temporary(purpose);
			ir.line(format_args!("  {value} = bitcast i32 {bits} to float"));
			value
		}
	}
}

fn emit_reduce_combine(
	ir: &mut Ir,
	operator: ReduceOperator,
	dtype: DType,
	left: &str,
	right: &str,
	left_index: Option<&str>,
	right_index: Option<&str>,
) -> Result<(String, Option<String>), LoweringError> {
	match operator {
		ReduceOperator::Minimum | ReduceOperator::Maximum => {
			let choose_right = ordered_choose_right(
				ir,
				OrderedComparison {
					dtype,
					maximum: operator == ReduceOperator::Maximum,
					left,
					right,
					indices: (left_index, right_index),
					purpose: "reduce_order",
				},
			)?;
			let value = ir.temporary("reduce_selected");
			ir.line(format_args!(
				"  {value} = select i1 {choose_right}, {} {right}, {} {left}",
				llvm_type(dtype),
				llvm_type(dtype)
			));
			let value = match dtype {
				DType::F32 => canonicalize_nan(ir, &value, "reduce_canonical"),
				DType::I32 => value,
			};
			let index = match (left_index, right_index) {
				(Some(left), Some(right)) => {
					let value = ir.temporary("reduce_selected_index");
					ir.line(format_args!(
						"  {value} = select i1 {choose_right}, i32 {right}, i32 {left}"
					));
					Some(value)
				}
				(None, None) => None,
				(Some(_), None) | (None, Some(_)) => {
					return Err(stage_error("reduction combine has only one index operand"));
				}
			};
			Ok((value, index))
		}
		ReduceOperator::Sum | ReduceOperator::Product => {
			let value = match dtype {
				DType::I32 => {
					let value = ir.temporary("reduce_integer");
					let operation = match operator {
						ReduceOperator::Sum => "add",
						ReduceOperator::Product => "mul",
						ReduceOperator::Minimum
						| ReduceOperator::Maximum
						| ReduceOperator::Any
						| ReduceOperator::All => unreachable!("arithmetic reduction operator"),
					};
					ir.line(format_args!("  {value} = {operation} i32 {left}, {right}"));
					value
				}
				DType::F32 => constrained_binary(
					ir,
					match operator {
						ReduceOperator::Sum => "fadd",
						ReduceOperator::Product => "fmul",
						ReduceOperator::Minimum
						| ReduceOperator::Maximum
						| ReduceOperator::Any
						| ReduceOperator::All => unreachable!("arithmetic reduction operator"),
					},
					left,
					right,
					"reduce_float",
				),
			};
			Ok((value, None))
		}
		ReduceOperator::Any | ReduceOperator::All => {
			ensure_stage(dtype == DType::I32, "Any and All combine require int32")?;
			let left_truth = ir.temporary("reduce_left_truth");
			let right_truth = ir.temporary("reduce_right_truth");
			ir.line(format_args!("  {left_truth} = icmp ne i32 {left}, 0"));
			ir.line(format_args!("  {right_truth} = icmp ne i32 {right}, 0"));
			let truth = ir.temporary("reduce_truth");
			let operation = match operator {
				ReduceOperator::Any => "or",
				ReduceOperator::All => "and",
				ReduceOperator::Sum
				| ReduceOperator::Product
				| ReduceOperator::Minimum
				| ReduceOperator::Maximum => unreachable!("logical reduction operator"),
			};
			ir.line(format_args!(
				"  {truth} = {operation} i1 {left_truth}, {right_truth}"
			));
			let value = ir.temporary("reduce_boolean");
			ir.line(format_args!("  {value} = zext i1 {truth} to i32"));
			Ok((value, None))
		}
	}
}

fn constrained_binary(ir: &mut Ir, operation: &'static str, left: &str, right: &str, purpose: &str) -> String {
	match operation {
		"fadd" => ir.declare(
			"llvm.experimental.constrained.fadd.f32",
			"declare float @llvm.experimental.constrained.fadd.f32(float, float, metadata, metadata)",
		),
		"fmul" => ir.declare(
			"llvm.experimental.constrained.fmul.f32",
			"declare float @llvm.experimental.constrained.fmul.f32(float, float, metadata, metadata)",
		),
		"fsub" => ir.declare(
			"llvm.experimental.constrained.fsub.f32",
			"declare float @llvm.experimental.constrained.fsub.f32(float, float, metadata, metadata)",
		),
		operation => unreachable!("Recipe constrained binary opcode {operation}"),
	}
	let raw = ir.temporary(purpose);
	ir.line(format_args!(
		"  {raw} = call float @llvm.experimental.constrained.{operation}.f32(float {left}, float {right}, metadata !\"round.tonearest\", metadata !\"fpexcept.ignore\")"
	));
	canonicalize_nan(ir, &raw, &format!("{purpose}_canonical"))
}

fn plain_fdiv(ir: &mut Ir, left: &str, right: &str, purpose: &str) -> String {
	let raw = ir.temporary(purpose);
	ir.line(format_args!("  {raw} = fdiv float {left}, {right}"));
	canonicalize_nan(ir, &raw, &format!("{purpose}_canonical"))
}

fn square_root(ir: &mut Ir, value: &str, purpose: &str) -> String {
	ir.declare("llvm.sqrt.f32", "declare float @llvm.sqrt.f32(float)");
	let raw = ir.temporary(purpose);
	ir.line(format_args!(
		"  {raw} = call float @llvm.sqrt.f32(float {value})"
	));
	canonicalize_nan(ir, &raw, &format!("{purpose}_canonical"))
}

fn canonicalize_nan(ir: &mut Ir, value: &str, purpose: &str) -> String {
	let is_nan = ir.temporary(&format!("{purpose}_is_nan"));
	ir.line(format_args!("  {is_nan} = fcmp uno float {value}, {value}"));
	let canonical = ir.temporary(purpose);
	ir.line(format_args!(
		"  {canonical} = select i1 {is_nan}, float 0x7FF8000000000000, float {value}"
	));
	canonical
}

#[derive(Clone, Copy, Debug)]
struct OrderedComparison<'a> {
	dtype: DType,
	maximum: bool,
	left: &'a str,
	right: &'a str,
	indices: (Option<&'a str>, Option<&'a str>),
	purpose: &'a str,
}

fn ordered_choose_right(ir: &mut Ir, comparison: OrderedComparison<'_>) -> Result<String, LoweringError> {
	let OrderedComparison {
		dtype,
		maximum,
		left,
		right,
		indices: (left_index, right_index),
		purpose,
	} = comparison;
	let right_better = match dtype {
		DType::I32 => {
			let value = ir.temporary(&format!("{purpose}_right_better"));
			let predicate = match maximum {
				true => "sgt",
				false => "slt",
			};
			ir.line(format_args!(
				"  {value} = icmp {predicate} i32 {right}, {left}"
			));
			value
		}
		DType::F32 => {
			let left_key = total_order_key(ir, left, &format!("{purpose}_left"));
			let right_key = total_order_key(ir, right, &format!("{purpose}_right"));
			let value = ir.temporary(&format!("{purpose}_right_better"));
			let predicate = match maximum {
				true => "ugt",
				false => "ult",
			};
			ir.line(format_args!(
				"  {value} = icmp {predicate} i32 {right_key}, {left_key}"
			));
			value
		}
	};
	match (left_index, right_index) {
		(Some(left_index), Some(right_index)) => {
			let equal = match dtype {
				DType::I32 => {
					let value = ir.temporary(&format!("{purpose}_equal"));
					ir.line(format_args!("  {value} = icmp eq i32 {right}, {left}"));
					value
				}
				DType::F32 => {
					let left_bits = ir.temporary(&format!("{purpose}_left_bits_equal"));
					let right_bits = ir.temporary(&format!("{purpose}_right_bits_equal"));
					ir.line(format_args!("  {left_bits} = bitcast float {left} to i32"));
					ir.line(format_args!(
						"  {right_bits} = bitcast float {right} to i32"
					));
					let value = ir.temporary(&format!("{purpose}_equal"));
					ir.line(format_args!(
						"  {value} = icmp eq i32 {right_bits}, {left_bits}"
					));
					value
				}
			};
			let lower_index = ir.temporary(&format!("{purpose}_lower_index"));
			ir.line(format_args!(
				"  {lower_index} = icmp ult i32 {right_index}, {left_index}"
			));
			let equal_and_lower = ir.temporary(&format!("{purpose}_equal_lower"));
			ir.line(format_args!(
				"  {equal_and_lower} = and i1 {equal}, {lower_index}"
			));
			let result = ir.temporary(&format!("{purpose}_choose_right"));
			ir.line(format_args!(
				"  {result} = or i1 {right_better}, {equal_and_lower}"
			));
			Ok(result)
		}
		(None, None) => Ok(right_better),
		(Some(_), None) | (None, Some(_)) => Err(stage_error("ordered comparison has only one index operand")),
	}
}

fn total_order_key(ir: &mut Ir, value: &str, purpose: &str) -> String {
	let bits = ir.temporary(&format!("{purpose}_bits"));
	ir.line(format_args!("  {bits} = bitcast float {value} to i32"));
	let negative = ir.temporary(&format!("{purpose}_negative"));
	ir.line(format_args!("  {negative} = icmp slt i32 {bits}, 0"));
	let inverted = ir.temporary(&format!("{purpose}_inverted"));
	ir.line(format_args!("  {inverted} = xor i32 {bits}, -1"));
	let positive = ir.temporary(&format!("{purpose}_positive"));
	ir.line(format_args!("  {positive} = xor i32 {bits}, -2147483648"));
	let key = ir.temporary(&format!("{purpose}_total_key"));
	ir.line(format_args!(
		"  {key} = select i1 {negative}, i32 {inverted}, i32 {positive}"
	));
	key
}

fn emit_scan_local(
	ir: &mut Ir,
	signature: &StageSignature,
	stage: &ProgramStage,
	scan: &ScanLocalStage,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<(), LoweringError> {
	validate_tree_contract(&scan.tree, stage, TreePhase::ScanUpsweep)?;
	let input = signature.pointer(0)?;
	let output = signature.pointer(1)?;
	let totals = signature.pointers.get(2);
	ensure_stage(
		input.dtype == scan.dtype
			&& output.dtype == scan.dtype
			&& totals.is_none_or(|pointer| pointer.dtype == scan.dtype),
		"scan binding type differs from its contract",
	)?;
	let shared = format!("{}_scan_values", options.entry_symbol);
	ir.globals.push(format!(
		"@{shared} = internal addrspace(3) global [{} x i32] undef, align 4",
		scan.tree.lanes
	));

	begin_lane(ir);
	let block = ir.temporary("scan_block");
	ir.line(format_args!(
		"  {block} = urem i64 %group_id, {}",
		scan.output_width
	));
	let sequence = ir.temporary("scan_sequence");
	ir.line(format_args!(
		"  {sequence} = udiv i64 %group_id, {}",
		scan.output_width
	));
	let block_base = ir.temporary("scan_block_base");
	ir.line(format_args!(
		"  {block_base} = mul i64 {block}, {}",
		scan.tree.lanes
	));
	let forward_position = ir.temporary("scan_forward_position");
	ir.line(format_args!(
		"  {forward_position} = add i64 {block_base}, %local_id"
	));
	let active = ir.temporary("scan_active");
	ir.line(format_args!(
		"  {active} = icmp ult i64 {forward_position}, {}",
		scan.input_width
	));
	let safe_forward = ir.temporary("scan_safe_forward");
	ir.line(format_args!(
		"  {safe_forward} = select i1 {active}, i64 {forward_position}, i64 0"
	));
	let position = match scan.reverse {
		true => {
			let last = scan.input_width - 1;
			let value = ir.temporary("scan_position");
			ir.line(format_args!("  {value} = sub i64 {last}, {safe_forward}"));
			value
		}
		false => safe_forward.clone(),
	};
	let input_index = scan_value_index(ir, input, scan, &sequence, &position, "scan_input")?;
	let loaded = ir.load(input, &input_index, "scan_input_value")?;
	let padding = reduction_identity(ir, scan.operator, scan.dtype, "scan_padding")?;
	let initial = ir.temporary("scan_initial");
	ir.line(format_args!(
		"  {initial} = select i1 {active}, {} {loaded}, {} {padding}",
		llvm_type(scan.dtype),
		llvm_type(scan.dtype)
	));
	shared_store(
		ir,
		&shared,
		scan.tree.lanes,
		"%local_id",
		scan.dtype,
		&initial,
		"scan_shared_initial",
	);
	ir.barrier(target);

	let mut root_cleared = false;
	for (step_index, step) in scan.tree.steps.iter().enumerate() {
		root_cleared = match (step.phase, root_cleared) {
			(TreePhase::ScanDownsweep, false) => {
				emit_scan_root_reset(ir, &shared, scan, &sequence, &block, totals)?;
				true
			}
			(TreePhase::ScanDownsweep, true) | (TreePhase::ScanUpsweep, true) | (TreePhase::Reduction, true) => {
				true
			}
			(TreePhase::ScanUpsweep, false) | (TreePhase::Reduction, false) => false,
		};
		let span = u64::from(step.stride) * 2;
		let one_based = ir.temporary("scan_one_based_lane");
		ir.line(format_args!("  {one_based} = add i64 %local_id, 1"));
		let remainder = ir.temporary("scan_step_remainder");
		ir.line(format_args!("  {remainder} = urem i64 {one_based}, {span}"));
		let participates = ir.temporary("scan_step_participates");
		ir.line(format_args!(
			"  {participates} = icmp eq i64 {remainder}, 0"
		));
		let action = ir.label("scan_step");
		let joined = ir.label("scan_join");
		ir.line(format_args!(
			"  br i1 {participates}, label %{action}, label %{joined}"
		));
		ir.line(format_args!("{action}:"));
		let left_lane = ir.temporary("scan_left_lane");
		ir.line(format_args!(
			"  {left_lane} = sub i64 %local_id, {}",
			step.stride
		));
		let left = shared_load(
			ir,
			&shared,
			scan.tree.lanes,
			&left_lane,
			scan.dtype,
			"scan_left",
		);
		let right = shared_load(
			ir,
			&shared,
			scan.tree.lanes,
			"%local_id",
			scan.dtype,
			"scan_right",
		);
		match step.phase {
			TreePhase::ScanUpsweep => {
				let (combined, _) =
					emit_reduce_combine(ir, scan.operator, scan.dtype, &left, &right, None, None)?;
				shared_store(
					ir,
					&shared,
					scan.tree.lanes,
					"%local_id",
					scan.dtype,
					&combined,
					"scan_upsweep",
				);
			}
			TreePhase::ScanDownsweep => {
				shared_store(
					ir,
					&shared,
					scan.tree.lanes,
					&left_lane,
					scan.dtype,
					&right,
					"scan_downsweep_left",
				);
				let (combined, _) =
					emit_reduce_combine(ir, scan.operator, scan.dtype, &right, &left, None, None)?;
				shared_store(
					ir,
					&shared,
					scan.tree.lanes,
					"%local_id",
					scan.dtype,
					&combined,
					"scan_downsweep_right",
				);
			}
			TreePhase::Reduction => {
				return Err(stage_error("scan tree contains a reduction step"));
			}
		}
		ir.line(format_args!("  br label %{joined}"));
		ir.line(format_args!("{joined}:"));
		ir.barrier(target);
		let expected_ordinal = match u32::try_from(step_index) {
			Ok(step) => step,
			Err(error) => return Err(stage_error(error.to_string())),
		};
		ensure_stage(
			stage.synchronization[step_index].after_step == expected_ordinal,
			"scan synchronization ordinal differs from its tree step",
		)?;
	}
	match root_cleared {
		true => Ok(()),
		false => emit_scan_root_reset(ir, &shared, scan, &sequence, &block, totals),
	}?;

	let write = ir.label("scan_write");
	let done = ir.label("scan_done");
	ir.line(format_args!(
		"  br i1 {active}, label %{write}, label %{done}"
	));
	ir.line(format_args!("{write}:"));
	let exclusive = shared_load(
		ir,
		&shared,
		scan.tree.lanes,
		"%local_id",
		scan.dtype,
		"scan_exclusive",
	);
	let result = match scan.mode {
		ScanStageMode::UserInclusive => {
			emit_reduce_combine(
				ir,
				scan.operator,
				scan.dtype,
				&exclusive,
				&loaded,
				None,
				None,
			)?
			.0
		}
		ScanStageMode::UserExclusive { .. } | ScanStageMode::HierarchyExclusiveIdentity => exclusive,
	};
	let output_index = scan_value_index(ir, output, scan, &sequence, &position, "scan_output")?;
	ir.store(output, &output_index, &result, "scan_output")?;
	ir.line(format_args!("  br label %{done}"));
	ir.line(format_args!("{done}:"));
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("exit:"));
	ir.line(format_args!("  ret void"));
	Ok(())
}

fn emit_scan_root_reset(
	ir: &mut Ir,
	shared: &str,
	scan: &ScanLocalStage,
	sequence: &str,
	block: &str,
	totals: Option<&BoundPointer>,
) -> Result<(), LoweringError> {
	let root = u64::from(scan.tree.lanes - 1);
	let leader = ir.temporary("scan_root_lane");
	ir.line(format_args!("  {leader} = icmp eq i64 %local_id, {root}"));
	let reset = ir.label("scan_root_reset");
	let joined = ir.label("scan_root_join");
	ir.line(format_args!(
		"  br i1 {leader}, label %{reset}, label %{joined}"
	));
	ir.line(format_args!("{reset}:"));
	let total = shared_load(
		ir,
		shared,
		scan.tree.lanes,
		&root.to_string(),
		scan.dtype,
		"scan_block_total",
	);
	match totals {
		Some(totals) => {
			let base = ir.temporary("scan_total_base");
			ir.line(format_args!(
				"  {base} = mul i64 {sequence}, {}",
				scan.output_width
			));
			let linear = ir.temporary("scan_total_linear");
			ir.line(format_args!("  {linear} = add i64 {base}, {block}"));
			let index = ir.affine_index(totals, &linear, "scan_total");
			ir.store(totals, &index, &total, "scan_total")
		}
		None => Ok(()),
	}?;
	let identity = match scan.mode {
		ScanStageMode::UserExclusive { identity } => literal_operand(ir, identity, "scan_user_identity"),
		ScanStageMode::UserInclusive | ScanStageMode::HierarchyExclusiveIdentity => {
			reduction_identity(ir, scan.operator, scan.dtype, "scan_identity")?
		}
	};
	shared_store(
		ir,
		shared,
		scan.tree.lanes,
		&root.to_string(),
		scan.dtype,
		&identity,
		"scan_root_identity",
	);
	ir.line(format_args!("  br label %{joined}"));
	ir.line(format_args!("{joined}:"));
	Ok(())
}

fn scan_value_index(
	ir: &mut Ir,
	pointer: &BoundPointer,
	scan: &ScanLocalStage,
	sequence: &str,
	position: &str,
	purpose: &str,
) -> Result<String, LoweringError> {
	match pointer.view.logical_extents.len() {
		1 => {
			let base = ir.temporary(&format!("{purpose}_base"));
			ir.line(format_args!(
				"  {base} = mul i64 {sequence}, {}",
				scan.input_width
			));
			let linear = ir.temporary(&format!("{purpose}_linear"));
			ir.line(format_args!("  {linear} = add i64 {base}, {position}"));
			return Ok(ir.affine_index(pointer, &linear, purpose));
		}
		0 | 2.. => ensure_stage(
			scan.axis < pointer.view.logical_extents.len()
				&& pointer.view.logical_extents[scan.axis] == scan.input_width,
			"scan axis or width differs from its tensor binding",
		)?,
	}
	let free_extents = pointer
		.view
		.logical_extents
		.iter()
		.copied()
		.enumerate()
		.filter(|(axis, _)| *axis != scan.axis)
		.map(|(_, extent)| extent)
		.collect::<Vec<_>>();
	let mut free = ir.coordinates(sequence, &free_extents, purpose).into_iter();
	let mut coordinates = Vec::with_capacity(pointer.view.logical_extents.len());
	for axis in 0..pointer.view.logical_extents.len() {
		coordinates.push(match axis == scan.axis {
			true => position.to_owned(),
			false => free
				.next()
				.ok_or_else(|| stage_error("scan coordinate underflow"))?,
		});
	}
	Ok(ir.index_from_coordinates(pointer, &coordinates, purpose))
}

fn emit_scan_uniform(
	ir: &mut Ir,
	signature: &StageSignature,
	scan: &ScanUniformStage,
	axis: usize,
) -> Result<(), LoweringError> {
	let target = signature.pointer(0)?;
	let offsets = signature.pointer(1)?;
	ensure_stage(
		target.dtype == scan.dtype && offsets.dtype == scan.dtype,
		"uniform scan binding type differs from its contract",
	)?;
	let active_per_sequence = scan.width.saturating_sub(u64::from(scan.block_lanes));
	ensure_stage(
		active_per_sequence != 0,
		"uniform scan stage has no post-first-block elements",
	)?;
	begin_lane(ir);
	let sequence = ir.temporary("uniform_scan_sequence");
	ir.line(format_args!(
		"  {sequence} = udiv i64 %global_id, {active_per_sequence}"
	));
	let tail = ir.temporary("uniform_scan_tail");
	ir.line(format_args!(
		"  {tail} = urem i64 %global_id, {active_per_sequence}"
	));
	let forward = ir.temporary("uniform_scan_forward");
	ir.line(format_args!(
		"  {forward} = add i64 {tail}, {}",
		scan.block_lanes
	));
	let position = match scan.reverse {
		true => {
			let value = ir.temporary("uniform_scan_position");
			ir.line(format_args!(
				"  {value} = sub i64 {}, {forward}",
				scan.width - 1
			));
			value
		}
		false => forward.clone(),
	};
	let target_index = scan_uniform_target_index(ir, target, scan, axis, &sequence, &position)?;
	let current = ir.load(target, &target_index, "uniform_scan_current")?;
	let block = ir.temporary("uniform_scan_block");
	ir.line(format_args!(
		"  {block} = udiv i64 {forward}, {}",
		scan.block_lanes
	));
	let blocks = scan.width.div_ceil(u64::from(scan.block_lanes));
	let offset_base = ir.temporary("uniform_scan_offset_base");
	ir.line(format_args!(
		"  {offset_base} = mul i64 {sequence}, {blocks}"
	));
	let offset_linear = ir.temporary("uniform_scan_offset_linear");
	ir.line(format_args!(
		"  {offset_linear} = add i64 {offset_base}, {block}"
	));
	let offset_index = ir.affine_index(offsets, &offset_linear, "uniform_scan_offset");
	let offset = ir.load(offsets, &offset_index, "uniform_scan_offset")?;
	let result = emit_reduce_combine(ir, scan.operator, scan.dtype, &offset, &current, None, None)?.0;
	ir.store(target, &target_index, &result, "uniform_scan_result")?;
	finish_lane(ir);
	Ok(())
}

fn scan_uniform_target_index(
	ir: &mut Ir,
	pointer: &BoundPointer,
	scan: &ScanUniformStage,
	axis: usize,
	sequence: &str,
	position: &str,
) -> Result<String, LoweringError> {
	match pointer.view.logical_extents.len() {
		1 => {
			let base = ir.temporary("uniform_scan_target_base");
			ir.line(format_args!(
				"  {base} = mul i64 {sequence}, {}",
				scan.width
			));
			let linear = ir.temporary("uniform_scan_target_linear");
			ir.line(format_args!("  {linear} = add i64 {base}, {position}"));
			return Ok(ir.affine_index(pointer, &linear, "uniform_scan_target"));
		}
		0 | 2.. => ensure_stage(
			axis < pointer.view.logical_extents.len() && pointer.view.logical_extents[axis] == scan.width,
			"uniform scan axis or width differs from its target binding",
		)?,
	}
	let free_extents = pointer
		.view
		.logical_extents
		.iter()
		.copied()
		.enumerate()
		.filter(|(candidate, _)| *candidate != axis)
		.map(|(_, extent)| extent)
		.collect::<Vec<_>>();
	let mut free = ir
		.coordinates(sequence, &free_extents, "uniform_scan_sequence")
		.into_iter();
	let mut coordinates = Vec::with_capacity(pointer.view.logical_extents.len());
	for candidate in 0..pointer.view.logical_extents.len() {
		coordinates.push(match candidate == axis {
			true => position.to_owned(),
			false => free
				.next()
				.ok_or_else(|| stage_error("uniform scan coordinate underflow"))?,
		});
	}
	Ok(ir.index_from_coordinates(pointer, &coordinates, "uniform_scan_target"))
}

fn emit_contraction(
	ir: &mut Ir,
	signature: &StageSignature,
	contraction: &ContractionStage,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<(), LoweringError> {
	let private_address_space = private_address_space(target);
	let left = signature.pointer(0)?;
	let right = signature.pointer(1)?;
	let output = signature.pointer(2)?;
	ensure_stage(
		left.dtype == contraction.dtype && right.dtype == contraction.dtype && output.dtype == contraction.dtype,
		"contraction binding type differs from its contract",
	)?;
	ensure_stage(
		contraction.canonical_contracted_order,
		"contraction does not declare canonical contracted order",
	)?;
	let left_rank = left.view.logical_extents.len();
	let right_rank = right.view.logical_extents.len();
	let mut used_left = vec![false; left_rank];
	let mut used_right = vec![false; right_rank];
	for (left_axis, right_axis) in contraction
		.spec
		.batch_axes
		.iter()
		.chain(&contraction.spec.contract_axes)
	{
		ensure_stage(
			*left_axis < left_rank
				&& *right_axis < right_rank
				&& !used_left[*left_axis]
				&& !used_right[*right_axis]
				&& left.view.logical_extents[*left_axis] == right.view.logical_extents[*right_axis],
			"contraction axis pairs do not match operand bindings",
		)?;
		used_left[*left_axis] = true;
		used_right[*right_axis] = true;
	}
	let left_free = (0..left_rank)
		.filter(|axis| !used_left[*axis])
		.collect::<Vec<_>>();
	let right_free = (0..right_rank)
		.filter(|axis| !used_right[*axis])
		.collect::<Vec<_>>();
	let expected_output_rank = contraction.spec.batch_axes.len() + left_free.len() + right_free.len();
	ensure_stage(
		output.view.logical_extents.len() == expected_output_rank.max(1),
		"contraction output rank differs from its canonical axis mapping",
	)?;
	let workgroup_lanes = contraction
		.tile
		.output_x
		.checked_mul(contraction.tile.output_y)
		.ok_or_else(|| stage_error("contraction workgroup width overflowed"))?;
	let staged_global = match contraction.strategy {
		ContractionStrategy::Direct => None,
		ContractionStrategy::Staged => {
			let name = format!("{}_contraction_accumulators", options.entry_symbol);
			ir.globals.push(format!(
				"@{name} = internal addrspace(3) global [{workgroup_lanes} x i32] undef, align 4"
			));
			Some(name)
		}
	};

	// Every launched lane, including the final partial workgroup's inactive
	// lanes, executes both barriers for every fixed reduction tile. Inactive
	// lanes use element zero for a safe dummy and never publish a result.
	ir.line(format_args!(
		"  %in_bounds = icmp ult i64 %global_id, %element_count"
	));
	let safe_output = ir.temporary("contraction_safe_output");
	ir.line(format_args!(
		"  {safe_output} = select i1 %in_bounds, i64 %global_id, i64 0"
	));
	let output_coordinates = ir.coordinates(
		&safe_output,
		&output.view.logical_extents,
		"contraction_output",
	);
	let mut left_coordinates = vec!["0".to_owned(); left_rank];
	let mut right_coordinates = vec!["0".to_owned(); right_rank];
	let mut output_axis = 0_usize;
	for (left_axis, right_axis) in &contraction.spec.batch_axes {
		left_coordinates[*left_axis] = output_coordinates[output_axis].clone();
		right_coordinates[*right_axis] = output_coordinates[output_axis].clone();
		output_axis += 1;
	}
	for axis in &left_free {
		left_coordinates[*axis] = output_coordinates[output_axis].clone();
		output_axis += 1;
	}
	for axis in &right_free {
		right_coordinates[*axis] = output_coordinates[output_axis].clone();
		output_axis += 1;
	}

	let accumulator = ir.temporary("contraction_accumulator_slot");
	ir.line(format_args!(
		"  {accumulator} = alloca {}, align 4, addrspace({private_address_space})",
		llvm_type(contraction.dtype),
	));
	let zero = match contraction.dtype {
		DType::F32 => "0.000000e+00",
		DType::I32 => "0",
	};
	ir.line(format_args!(
		"  store {} {zero}, ptr addrspace({private_address_space}) {accumulator}, align 4",
		llvm_type(contraction.dtype)
	));
	let tile_slot = ir.temporary("contraction_tile_slot");
	let inner_slot = ir.temporary("contraction_inner_slot");
	ir.line(format_args!(
		"  {tile_slot} = alloca i64, align 8, addrspace({private_address_space})"
	));
	ir.line(format_args!(
		"  {inner_slot} = alloca i64, align 8, addrspace({private_address_space})"
	));
	ir.line(format_args!(
		"  store i64 0, ptr addrspace({private_address_space}) {tile_slot}, align 8"
	));
	let tile_check = ir.label("contraction_tile_check");
	let tile_body = ir.label("contraction_tile_body");
	let inner_check = ir.label("contraction_inner_check");
	let inner_body = ir.label("contraction_inner_body");
	let inner_done = ir.label("contraction_inner_done");
	let tile_done = ir.label("contraction_tile_done");
	let publish = ir.label("contraction_publish");
	let exit = ir.label("contraction_exit");
	ir.line(format_args!("  br label %{tile_check}"));
	ir.line(format_args!("{tile_check}:"));
	let tile_base = ir.temporary("contraction_tile_base");
	ir.line(format_args!(
		"  {tile_base} = load i64, ptr addrspace({private_address_space}) {tile_slot}, align 8"
	));
	let has_tile = ir.temporary("contraction_has_tile");
	ir.line(format_args!(
		"  {has_tile} = icmp ult i64 {tile_base}, {}",
		contraction.contracted_elements
	));
	ir.line(format_args!(
		"  br i1 {has_tile}, label %{tile_body}, label %{tile_done}"
	));
	ir.line(format_args!("{tile_body}:"));
	ir.line(format_args!(
		"  store i64 0, ptr addrspace({private_address_space}) {inner_slot}, align 8"
	));
	ir.line(format_args!("  br label %{inner_check}"));
	ir.line(format_args!("{inner_check}:"));
	let inner = ir.temporary("contraction_inner");
	ir.line(format_args!(
		"  {inner} = load i64, ptr addrspace({private_address_space}) {inner_slot}, align 8"
	));
	let inside_tile = ir.temporary("contraction_inside_tile");
	ir.line(format_args!(
		"  {inside_tile} = icmp ult i64 {inner}, {}",
		contraction.tile.reduction
	));
	let contracted = ir.temporary("contraction_contracted");
	ir.line(format_args!(
		"  {contracted} = add i64 {tile_base}, {inner}"
	));
	let inside_domain = ir.temporary("contraction_inside_domain");
	ir.line(format_args!(
		"  {inside_domain} = icmp ult i64 {contracted}, {}",
		contraction.contracted_elements
	));
	let compute = ir.temporary("contraction_compute");
	ir.line(format_args!(
		"  {compute} = and i1 {inside_tile}, {inside_domain}"
	));
	ir.line(format_args!(
		"  br i1 {compute}, label %{inner_body}, label %{inner_done}"
	));
	ir.line(format_args!("{inner_body}:"));
	let contracted_extents = contraction
		.spec
		.contract_axes
		.iter()
		.map(|(axis, _)| left.view.logical_extents[*axis])
		.collect::<Vec<_>>();
	let contracted_coordinates = ir.coordinates(&contracted, &contracted_extents, "contraction_k");
	let mut left_iteration = left_coordinates.clone();
	let mut right_iteration = right_coordinates.clone();
	for ((left_axis, right_axis), coordinate) in contraction
		.spec
		.contract_axes
		.iter()
		.zip(contracted_coordinates)
	{
		left_iteration[*left_axis] = coordinate.clone();
		right_iteration[*right_axis] = coordinate;
	}
	let left_index = ir.index_from_coordinates(left, &left_iteration, "contraction_left");
	let right_index = ir.index_from_coordinates(right, &right_iteration, "contraction_right");
	let left_value = ir.load(left, &left_index, "contraction_left")?;
	let right_value = ir.load(right, &right_index, "contraction_right")?;
	let product = match contraction.dtype {
		DType::F32 => constrained_binary(ir, "fmul", &left_value, &right_value, "contraction_product"),
		DType::I32 => {
			let value = ir.temporary("contraction_product");
			ir.line(format_args!(
				"  {value} = mul i32 {left_value}, {right_value}"
			));
			value
		}
	};
	let accumulated = ir.temporary("contraction_accumulated");
	ir.line(format_args!(
		"  {accumulated} = load {}, ptr addrspace({private_address_space}) {accumulator}, align 4",
		llvm_type(contraction.dtype)
	));
	let sum = match contraction.dtype {
		DType::F32 => constrained_binary(ir, "fadd", &accumulated, &product, "contraction_sum"),
		DType::I32 => {
			let value = ir.temporary("contraction_sum");
			ir.line(format_args!("  {value} = add i32 {accumulated}, {product}"));
			value
		}
	};
	ir.line(format_args!(
		"  store {} {sum}, ptr addrspace({private_address_space}) {accumulator}, align 4",
		llvm_type(contraction.dtype)
	));
	let next_inner = ir.temporary("contraction_next_inner");
	ir.line(format_args!("  {next_inner} = add i64 {inner}, 1"));
	ir.line(format_args!(
		"  store i64 {next_inner}, ptr addrspace({private_address_space}) {inner_slot}, align 8"
	));
	ir.line(format_args!("  br label %{inner_check}"));
	ir.line(format_args!("{inner_done}:"));
	if let Some(staged_global) = &staged_global {
		let checkpoint = ir.temporary("contraction_staged_checkpoint");
		ir.line(format_args!(
			"  {checkpoint} = load {}, ptr addrspace({private_address_space}) {accumulator}, align 4",
			llvm_type(contraction.dtype)
		));
		shared_store(
			ir,
			staged_global,
			workgroup_lanes,
			"%local_id",
			contraction.dtype,
			&checkpoint,
			"contraction_staged_checkpoint",
		);
		ir.barrier(target);
		let restored = shared_load(
			ir,
			staged_global,
			workgroup_lanes,
			"%local_id",
			contraction.dtype,
			"contraction_staged_restore",
		);
		ir.line(format_args!(
			"  store {} {restored}, ptr addrspace({private_address_space}) {accumulator}, align 4",
			llvm_type(contraction.dtype)
		));
		ir.barrier(target);
	}
	let next_tile = ir.temporary("contraction_next_tile");
	ir.line(format_args!(
		"  {next_tile} = add i64 {tile_base}, {}",
		contraction.tile.reduction
	));
	ir.line(format_args!(
		"  store i64 {next_tile}, ptr addrspace({private_address_space}) {tile_slot}, align 8"
	));
	ir.line(format_args!("  br label %{tile_check}"));
	ir.line(format_args!("{tile_done}:"));
	ir.line(format_args!(
		"  br i1 %in_bounds, label %{publish}, label %{exit}"
	));
	ir.line(format_args!("{publish}:"));
	let result = ir.temporary("contraction_result");
	ir.line(format_args!(
		"  {result} = load {}, ptr addrspace({private_address_space}) {accumulator}, align 4",
		llvm_type(contraction.dtype)
	));
	let output_index = ir.index_from_coordinates(output, &output_coordinates, "contraction_output");
	ir.store(output, &output_index, &result, "contraction_output")?;
	ir.line(format_args!("  br label %{exit}"));
	ir.line(format_args!("{exit}:"));
	ir.line(format_args!("  ret void"));
	Ok(())
}

fn emit_gather(
	ir: &mut Ir,
	signature: &StageSignature,
	fault: Option<FaultContract>,
	axis: usize,
	bounds: IndexBounds,
) -> Result<(), LoweringError> {
	let input = signature.pointer(0)?;
	let indices = signature.pointer(1)?;
	let output = signature.pointer(2)?;
	ensure_stage(
		indices.dtype == DType::I32 && input.dtype == output.dtype,
		"gather binding types are invalid",
	)?;
	begin_lane(ir);
	let output_coordinates = ir.coordinates("%global_id", &output.view.logical_extents, "gather_output");
	let mapping = emit_gather_mapping(
		ir,
		input,
		indices,
		&output_coordinates,
		axis,
		bounds,
		"gather",
	)?;
	let output_index = ir.index_from_coordinates(output, &output_coordinates, "gather_output");
	match (bounds, mapping.valid.as_deref(), fault) {
		(IndexBounds::Reject, Some(valid), Some(fault)) => {
			let payload = ir.label("gather_payload");
			let rejected = ir.label("gather_rejected");
			ir.line(format_args!(
				"  br i1 {valid}, label %{payload}, label %{rejected}"
			));
			ir.line(format_args!("{rejected}:"));
			emit_fault_publish(ir, fault)?;
			ir.line(format_args!("  br label %exit"));
			ir.line(format_args!("{payload}:"));
			Ok(())
		}
		(IndexBounds::Reject, None, None)
		| (IndexBounds::Reject, Some(_), None)
		| (IndexBounds::Reject, None, Some(_)) => Err(stage_error("rejecting gather lacks its exact fault guard")),
		(IndexBounds::Clamp | IndexBounds::Wrap, None, None) => Ok(()),
		(IndexBounds::Clamp | IndexBounds::Wrap, Some(_), None)
		| (IndexBounds::Clamp | IndexBounds::Wrap, None, Some(_))
		| (IndexBounds::Clamp | IndexBounds::Wrap, Some(_), Some(_)) => Err(stage_error(
			"gather bounds policy and fault contract disagree",
		)),
	}?;
	let input_index = ir.index_from_coordinates(input, &mapping.payload_coordinates, "gather_input");
	let value = ir.load(input, &input_index, "gather_value")?;
	ir.store(output, &output_index, &value, "gather_value")?;
	finish_lane(ir);
	Ok(())
}

fn emit_scatter(
	ir: &mut Ir,
	signature: &StageSignature,
	fault: Option<FaultContract>,
	stage: &ProgramStage,
	axis: usize,
	bounds: IndexBounds,
	conflict: ScatterConflict,
) -> Result<(), LoweringError> {
	let indices = signature.pointer(0)?;
	let updates = signature.pointer(1)?;
	let output = signature.pointer(2)?;
	ensure_stage(
		indices.dtype == DType::I32 && updates.dtype == output.dtype,
		"scatter binding types are invalid",
	)?;
	begin_lane(ir);
	let update_coordinates = ir.coordinates(
		"%global_id",
		&updates.view.logical_extents,
		"scatter_update",
	);
	let mapping = emit_gather_mapping(
		ir,
		output,
		indices,
		&update_coordinates,
		axis,
		bounds,
		"scatter",
	)?;
	let update_index = ir.index_from_coordinates(updates, &update_coordinates, "scatter_update");
	let update = ir.load(updates, &update_index, "scatter_update")?;
	match (bounds, mapping.valid.as_deref(), fault) {
		(IndexBounds::Reject, Some(valid), Some(fault)) => {
			let payload = ir.label("scatter_payload");
			let rejected = ir.label("scatter_rejected");
			ir.line(format_args!(
				"  br i1 {valid}, label %{payload}, label %{rejected}"
			));
			ir.line(format_args!("{rejected}:"));
			emit_fault_publish(ir, fault)?;
			ir.line(format_args!("  br label %exit"));
			ir.line(format_args!("{payload}:"));
			Ok(())
		}
		(IndexBounds::Reject, None, None)
		| (IndexBounds::Reject, Some(_), None)
		| (IndexBounds::Reject, None, Some(_)) => Err(stage_error("rejecting scatter lacks its exact fault guard")),
		(IndexBounds::Clamp | IndexBounds::Wrap, None, None) => Ok(()),
		(IndexBounds::Clamp | IndexBounds::Wrap, Some(_), None)
		| (IndexBounds::Clamp | IndexBounds::Wrap, None, Some(_))
		| (IndexBounds::Clamp | IndexBounds::Wrap, Some(_), Some(_)) => Err(stage_error(
			"scatter bounds policy and fault contract disagree",
		)),
	}?;
	let output_index = ir.index_from_coordinates(output, &mapping.payload_coordinates, "scatter_output");
	match conflict {
		ScatterConflict::UniqueIndices => {
			ir.store(output, &output_index, &update, "scatter_output")?;
		}
		ScatterConflict::Atomic {
			operation,
			ordering,
		} => {
			let atomic = stage
				.atomics
				.iter()
				.find(|atomic| atomic.domain == AtomicAddressDomain::TensorElements)
				.ok_or_else(|| stage_error("atomic scatter omitted its payload atomic contract"))?;
			ensure_stage(
				atomic.operation == OwnedAtomicOperation::from(operation)
					&& atomic.ordering == OwnedAtomicOrdering::from(ordering)
					&& atomic.dtype == output.dtype,
				"scatter payload atomic differs from its stage contract",
			)?;
			emit_atomic_update(
				ir,
				output,
				&output_index,
				atomic.operation,
				atomic.ordering,
				&update,
				"scatter",
			)?;
		}
	}
	finish_lane(ir);
	Ok(())
}

#[derive(Debug)]
struct GatherMapping {
	payload_coordinates: Vec<String>,
	valid: Option<String>,
}

fn emit_gather_mapping(
	ir: &mut Ir,
	payload: &BoundPointer,
	indices: &BoundPointer,
	result_coordinates: &[String],
	axis: usize,
	bounds: IndexBounds,
	purpose: &str,
) -> Result<GatherMapping, LoweringError> {
	let payload_rank = payload.view.logical_extents.len();
	ensure_stage(
		axis < payload_rank && result_coordinates.len() + 1 >= payload_rank,
		"indexed operation axis or result rank is invalid",
	)?;
	let index_rank = result_coordinates.len() + 1 - payload_rank;
	ensure_stage(
		index_rank == indices.view.logical_extents.len(),
		"indexed operation index rank differs from the result/payload relation",
	)?;
	let index_coordinates = result_coordinates[axis..axis + index_rank].to_vec();
	let index_physical = ir.index_from_coordinates(indices, &index_coordinates, &format!("{purpose}_index"));
	let raw = ir.load(indices, &index_physical, &format!("{purpose}_index"))?;
	let extent = payload.view.logical_extents[axis];
	let (normalized, valid) = normalize_index(ir, &raw, extent, bounds, purpose)?;
	let mut payload_coordinates = Vec::with_capacity(payload_rank);
	for payload_axis in 0..payload_rank {
		let coordinate = match payload_axis.cmp(&axis) {
			std::cmp::Ordering::Less => result_coordinates[payload_axis].clone(),
			std::cmp::Ordering::Equal => normalized.clone(),
			std::cmp::Ordering::Greater => result_coordinates[payload_axis + index_rank - 1].clone(),
		};
		payload_coordinates.push(coordinate);
	}
	Ok(GatherMapping {
		payload_coordinates,
		valid,
	})
}

fn normalize_index(
	ir: &mut Ir,
	raw: &str,
	extent: u64,
	bounds: IndexBounds,
	purpose: &str,
) -> Result<(String, Option<String>), LoweringError> {
	ensure_stage(
		extent != 0 && extent <= 9_223_372_036_854_775_807_u64,
		"indexed extent is outside the signed index domain",
	)?;
	let signed = ir.temporary(&format!("{purpose}_signed_index"));
	ir.line(format_args!("  {signed} = sext i32 {raw} to i64"));
	match bounds {
		IndexBounds::Reject => {
			let nonnegative = ir.temporary(&format!("{purpose}_nonnegative"));
			ir.line(format_args!("  {nonnegative} = icmp sge i64 {signed}, 0"));
			let below = ir.temporary(&format!("{purpose}_below_extent"));
			ir.line(format_args!("  {below} = icmp slt i64 {signed}, {extent}"));
			let valid = ir.temporary(&format!("{purpose}_valid_index"));
			ir.line(format_args!("  {valid} = and i1 {nonnegative}, {below}"));
			Ok((signed, Some(valid)))
		}
		IndexBounds::Clamp => {
			let negative = ir.temporary(&format!("{purpose}_negative"));
			ir.line(format_args!("  {negative} = icmp slt i64 {signed}, 0"));
			let lower = ir.temporary(&format!("{purpose}_lower_clamped"));
			ir.line(format_args!(
				"  {lower} = select i1 {negative}, i64 0, i64 {signed}"
			));
			let high = ir.temporary(&format!("{purpose}_above_extent"));
			ir.line(format_args!("  {high} = icmp sge i64 {lower}, {extent}"));
			let normalized = ir.temporary(&format!("{purpose}_clamped"));
			ir.line(format_args!(
				"  {normalized} = select i1 {high}, i64 {}, i64 {lower}",
				extent - 1
			));
			Ok((normalized, None))
		}
		IndexBounds::Wrap => {
			let remainder = ir.temporary(&format!("{purpose}_remainder"));
			ir.line(format_args!("  {remainder} = srem i64 {signed}, {extent}"));
			let negative = ir.temporary(&format!("{purpose}_negative_remainder"));
			ir.line(format_args!("  {negative} = icmp slt i64 {remainder}, 0"));
			let wrapped = ir.temporary(&format!("{purpose}_wrapped"));
			ir.line(format_args!("  {wrapped} = add i64 {remainder}, {extent}"));
			let normalized = ir.temporary(&format!("{purpose}_normalized"));
			ir.line(format_args!(
				"  {normalized} = select i1 {negative}, i64 {wrapped}, i64 {remainder}"
			));
			Ok((normalized, None))
		}
	}
}

fn emit_fault_publish(ir: &mut Ir, fault: FaultContract) -> Result<(), LoweringError> {
	ensure_stage(
		fault.guard_before_address
			&& fault.publish.operation == OwnedAtomicOperation::Exchange
			&& fault.publish.ordering == OwnedAtomicOrdering::Release
			&& fault.publish.domain == AtomicAddressDomain::SingleFaultFlag
			&& fault.publish.dtype == DType::I32,
		"fault publication contract is not the Recipe checked-path ABI",
	)?;
	let previous = ir.temporary("fault_previous");
	ir.line(format_args!(
		"  {previous} = atomicrmw xchg ptr addrspace(1) %fault_flag, i32 {} release, align 4",
		fault.code
	));
	Ok(())
}

fn emit_atomic_update(
	ir: &mut Ir,
	pointer: &BoundPointer,
	index: &str,
	operation: OwnedAtomicOperation,
	ordering: OwnedAtomicOrdering,
	operand: &str,
	purpose: &str,
) -> Result<(), LoweringError> {
	let address = ir.write_address(pointer, index, purpose)?;
	match pointer.dtype {
		DType::I32 => {
			let instruction = match operation {
				OwnedAtomicOperation::Exchange => "xchg",
				OwnedAtomicOperation::Add => "add",
				OwnedAtomicOperation::Minimum => "min",
				OwnedAtomicOperation::Maximum => "max",
			};
			let previous = ir.temporary(&format!("{purpose}_atomic_previous"));
			ir.line(format_args!(
				"  {previous} = atomicrmw {instruction} ptr addrspace(1) {address}, i32 {operand} {}, align 4",
				atomic_ordering(ordering)
			));
		}
		DType::F32 => match operation {
			OwnedAtomicOperation::Exchange => {
				let previous = ir.temporary(&format!("{purpose}_atomic_previous"));
				ir.line(format_args!(
					"  {previous} = atomicrmw xchg ptr addrspace(1) {address}, float {operand} {}, align 4",
					atomic_ordering(ordering)
				));
			}
			OwnedAtomicOperation::Add | OwnedAtomicOperation::Minimum | OwnedAtomicOperation::Maximum => {
				let helper = ensure_f32_atomic_helper(ir, operation, ordering);
				ir.line(format_args!(
					"  call void @{helper}(ptr addrspace(1) {address}, float {operand})"
				));
			}
		},
	}
	Ok(())
}

fn atomic_ordering(ordering: OwnedAtomicOrdering) -> &'static str {
	match ordering {
		OwnedAtomicOrdering::Relaxed => "monotonic",
		OwnedAtomicOrdering::Acquire => "acquire",
		OwnedAtomicOrdering::Release => "release",
		OwnedAtomicOrdering::AcquireRelease => "acq_rel",
		OwnedAtomicOrdering::SequentiallyConsistent => "seq_cst",
	}
}

fn atomic_failure_ordering(ordering: OwnedAtomicOrdering) -> &'static str {
	match ordering {
		OwnedAtomicOrdering::Relaxed | OwnedAtomicOrdering::Release => "monotonic",
		OwnedAtomicOrdering::Acquire | OwnedAtomicOrdering::AcquireRelease => "acquire",
		OwnedAtomicOrdering::SequentiallyConsistent => "seq_cst",
	}
}

fn ensure_f32_atomic_helper(ir: &mut Ir, operation: OwnedAtomicOperation, ordering: OwnedAtomicOrdering) -> String {
	let operation_name = match operation {
		OwnedAtomicOperation::Exchange => "exchange",
		OwnedAtomicOperation::Add => "add",
		OwnedAtomicOperation::Minimum => "minimum",
		OwnedAtomicOperation::Maximum => "maximum",
	};
	let ordering_name = match ordering {
		OwnedAtomicOrdering::Relaxed => "relaxed",
		OwnedAtomicOrdering::Acquire => "acquire",
		OwnedAtomicOrdering::Release => "release",
		OwnedAtomicOrdering::AcquireRelease => "acq_rel",
		OwnedAtomicOrdering::SequentiallyConsistent => "seq_cst",
	};
	let name = format!("recipe_atomic_f32_{operation_name}_{ordering_name}");
	declare_atomic_intrinsic(ir, operation);
	let desired = match operation {
		OwnedAtomicOperation::Add => String::from(
			"  %raw = call float @llvm.experimental.constrained.fadd.f32(float %old, float %operand, metadata !\"round.tonearest\", metadata !\"fpexcept.ignore\")\n\
  %raw_nan = fcmp uno float %raw, %raw\n\
  %desired = select i1 %raw_nan, float 0x7FF8000000000000, float %raw\n",
		),
		OwnedAtomicOperation::Minimum | OwnedAtomicOperation::Maximum => {
			let predicate = match operation {
				OwnedAtomicOperation::Minimum => "ult",
				OwnedAtomicOperation::Maximum => "ugt",
				OwnedAtomicOperation::Exchange | OwnedAtomicOperation::Add => unreachable!("min/max helper"),
			};
			format!(
				"  %old_nan = fcmp uno float %old, %old\n\
  %operand_nan = fcmp uno float %operand, %operand\n\
  %any_nan = or i1 %old_nan, %operand_nan\n\
  %old_negative = icmp slt i32 %expected, 0\n\
  %old_inverted = xor i32 %expected, -1\n\
  %old_positive = xor i32 %expected, -2147483648\n\
  %old_key = select i1 %old_negative, i32 %old_inverted, i32 %old_positive\n\
  %operand_bits = bitcast float %operand to i32\n\
  %operand_negative = icmp slt i32 %operand_bits, 0\n\
  %operand_inverted = xor i32 %operand_bits, -1\n\
  %operand_positive = xor i32 %operand_bits, -2147483648\n\
  %operand_key = select i1 %operand_negative, i32 %operand_inverted, i32 %operand_positive\n\
  %operand_better = icmp {predicate} i32 %operand_key, %old_key\n\
  %selected = select i1 %operand_better, float %operand, float %old\n\
  %desired = select i1 %any_nan, float 0x7FF8000000000000, float %selected\n"
			)
		}
		OwnedAtomicOperation::Exchange => unreachable!("exchange is emitted directly"),
	};
	let helper = format!(
		"define internal void @{name}(ptr addrspace(1) %address, float %operand) #0 {{\n\
entry:\n\
  %initial = load atomic i32, ptr addrspace(1) %address monotonic, align 4\n\
  br label %loop\n\
loop:\n\
  %expected = phi i32 [ %initial, %entry ], [ %observed, %retry ]\n\
  %old = bitcast i32 %expected to float\n\
{desired}\
  %desired_bits = bitcast float %desired to i32\n\
  %pair = cmpxchg ptr addrspace(1) %address, i32 %expected, i32 %desired_bits {} {}, align 4\n\
  %observed = extractvalue {{ i32, i1 }} %pair, 0\n\
  %success = extractvalue {{ i32, i1 }} %pair, 1\n\
  br i1 %success, label %exit, label %retry\n\
retry:\n\
  br label %loop\n\
exit:\n\
  ret void\n\
}}\n",
		atomic_ordering(ordering),
		atomic_failure_ordering(ordering)
	);
	ir.helpers.entry(name.clone()).or_insert(helper);
	name
}

fn declare_atomic_intrinsic(ir: &mut Ir, operation: OwnedAtomicOperation) {
	let declaration = match operation {
		OwnedAtomicOperation::Add => Some((
			"llvm.experimental.constrained.fadd.f32",
			"declare float @llvm.experimental.constrained.fadd.f32(float, float, metadata, metadata)",
		)),
		OwnedAtomicOperation::Exchange | OwnedAtomicOperation::Minimum | OwnedAtomicOperation::Maximum => None,
	};
	declaration
		.into_iter()
		.for_each(|(name, signature)| ir.declare(name, signature));
}

#[derive(Clone, Copy, Debug)]
struct HistogramContract {
	fault: FaultContract,
	bins: u32,
	weighted: bool,
	mapping: HistogramBinMapping,
	ordering: OwnedAtomicOrdering,
}

fn emit_histogram(
	ir: &mut Ir,
	signature: &StageSignature,
	stage: &ProgramStage,
	contract: HistogramContract,
) -> Result<(), LoweringError> {
	let HistogramContract {
		fault,
		bins,
		weighted,
		mapping,
		ordering,
	} = contract;
	let input = signature.pointer(0)?;
	let output = signature.pointer(1)?;
	let weight = match weighted {
		true => Some(signature.pointer(2)?),
		false => None,
	};
	let expected_output = match weighted {
		true => DType::F32,
		false => DType::I32,
	};
	ensure_stage(
		bins != 0
			&& output.view.logical_extents.as_slice() == [u64::from(bins)]
			&& output.dtype == expected_output
			&& weight.is_none_or(|pointer| pointer.dtype == DType::F32),
		"histogram binding shape or type differs from its contract",
	)?;
	let atomic = stage
		.atomics
		.iter()
		.find(|atomic| atomic.domain == AtomicAddressDomain::HistogramBins)
		.ok_or_else(|| stage_error("histogram omitted its bin atomic contract"))?;
	ensure_stage(
		atomic.operation == OwnedAtomicOperation::Add
			&& atomic.ordering == ordering
			&& atomic.dtype == output.dtype,
		"histogram bin atomic differs from its stage contract",
	)?;
	begin_lane(ir);
	let input_index = ir.affine_index(input, "%global_id", "histogram_input");
	let value = ir.load(input, &input_index, "histogram_input")?;
	let (bin, valid) = match (mapping, input.dtype) {
		(HistogramBinMapping::I32Direct, DType::I32) => {
			let signed = ir.temporary("histogram_signed_bin");
			ir.line(format_args!("  {signed} = sext i32 {value} to i64"));
			let nonnegative = ir.temporary("histogram_nonnegative");
			let below = ir.temporary("histogram_below_bins");
			let valid = ir.temporary("histogram_valid");
			ir.line(format_args!("  {nonnegative} = icmp sge i64 {signed}, 0"));
			ir.line(format_args!("  {below} = icmp slt i64 {signed}, {bins}"));
			ir.line(format_args!("  {valid} = and i1 {nonnegative}, {below}"));
			(signed, valid)
		}
		(HistogramBinMapping::F32TruncateTowardZero, DType::F32) => {
			ir.declare(
				"llvm.fptosi.sat.i32.f32",
				"declare i32 @llvm.fptosi.sat.i32.f32(float)",
			);
			let converted = ir.temporary("histogram_converted_bin");
			ir.line(format_args!(
				"  {converted} = call i32 @llvm.fptosi.sat.i32.f32(float {value})"
			));
			let ordered = ir.temporary("histogram_ordered");
			ir.line(format_args!(
				"  {ordered} = fcmp ord float {value}, {value}"
			));
			let nonnegative = ir.temporary("histogram_nonnegative");
			let below = ir.temporary("histogram_below_bins");
			ir.line(format_args!(
				"  {nonnegative} = icmp sge i32 {converted}, 0"
			));
			ir.line(format_args!("  {below} = icmp ult i32 {converted}, {bins}"));
			let in_range = ir.temporary("histogram_in_range");
			ir.line(format_args!("  {in_range} = and i1 {nonnegative}, {below}"));
			let valid = ir.temporary("histogram_valid");
			ir.line(format_args!("  {valid} = and i1 {ordered}, {in_range}"));
			let bin = ir.temporary("histogram_bin");
			ir.line(format_args!("  {bin} = zext i32 {converted} to i64"));
			(bin, valid)
		}
		(HistogramBinMapping::I32Direct, DType::F32) | (HistogramBinMapping::F32TruncateTowardZero, DType::I32) => {
			return Err(stage_error(
				"histogram bin mapping differs from its input dtype",
			));
		}
	};
	let accumulate = ir.label("histogram_accumulate");
	let rejected = ir.label("histogram_rejected");
	ir.line(format_args!(
		"  br i1 {valid}, label %{accumulate}, label %{rejected}"
	));
	ir.line(format_args!("{rejected}:"));
	emit_fault_publish(ir, fault)?;
	ir.line(format_args!("  br label %exit"));
	ir.line(format_args!("{accumulate}:"));
	let operand = match weight {
		Some(weight) => {
			let index = ir.affine_index(weight, "%global_id", "histogram_weight");
			ir.load(weight, &index, "histogram_weight")?
		}
		None => "1".to_owned(),
	};
	let output_index = ir.index_from_coordinates(output, std::slice::from_ref(&bin), "histogram_output");
	emit_atomic_update(
		ir,
		output,
		&output_index,
		OwnedAtomicOperation::Add,
		ordering,
		&operand,
		"histogram",
	)?;
	finish_lane(ir);
	Ok(())
}

fn validate_sort_network(network: SortNetwork) -> Result<(), LoweringError> {
	ensure_stage(
		network.axis_length != 0
			&& network.padded_axis_length.is_power_of_two()
			&& network.padded_axis_length == network.axis_length.next_power_of_two()
			&& network.tie_break == SortTieBreak::OriginalAxisIndexAscending
			&& network.float_order == FloatSortOrder::Ieee754TotalOrder
			&& network.padding == SortPadding::AfterAllValidElements,
		"sort network is not the Recipe stable total-order contract",
	)
}

fn emit_sort_initialize(ir: &mut Ir, signature: &StageSignature, network: SortNetwork) -> Result<(), LoweringError> {
	validate_sort_network(network)?;
	let input = signature.pointer(0)?;
	let values = signature.pointer(1)?;
	let indices = signature.pointer(2)?;
	ensure_stage(
		values.dtype == input.dtype
			&& indices.dtype == DType::I32
			&& values.view.logical_extents.as_slice() == [network.slices * network.padded_axis_length]
			&& indices.view.logical_extents == values.view.logical_extents,
		"sort initialization bindings differ from its scratch network",
	)?;
	begin_lane(ir);
	let slice = ir.temporary("sort_init_slice");
	let position = ir.temporary("sort_init_position");
	ir.line(format_args!(
		"  {slice} = udiv i64 %global_id, {}",
		network.padded_axis_length
	));
	ir.line(format_args!(
		"  {position} = urem i64 %global_id, {}",
		network.padded_axis_length
	));
	let valid = ir.temporary("sort_init_valid");
	ir.line(format_args!(
		"  {valid} = icmp ult i64 {position}, {}",
		network.axis_length
	));
	let load = ir.label("sort_init_load");
	let padding = ir.label("sort_init_padding");
	let joined = ir.label("sort_init_join");
	ir.line(format_args!(
		"  br i1 {valid}, label %{load}, label %{padding}"
	));
	ir.line(format_args!("{load}:"));
	let input_index = sort_tensor_index(ir, input, network, &slice, &position, "sort_init_input")?;
	let loaded = ir.load(input, &input_index, "sort_init_value")?;
	let original = ir.temporary("sort_init_original_index");
	ir.line(format_args!("  {original} = trunc i64 {position} to i32"));
	ir.line(format_args!("  br label %{joined}"));
	ir.line(format_args!("{padding}:"));
	let padding_value = match input.dtype {
		DType::F32 => "0.000000e+00",
		DType::I32 => "0",
	};
	ir.line(format_args!("  br label %{joined}"));
	ir.line(format_args!("{joined}:"));
	let value = ir.temporary("sort_init_selected_value");
	ir.line(format_args!(
		"  {value} = phi {} [ {loaded}, %{load} ], [ {padding_value}, %{padding} ]",
		llvm_type(input.dtype)
	));
	let original_index = ir.temporary("sort_init_selected_index");
	ir.line(format_args!(
		"  {original_index} = phi i32 [ {original}, %{load} ], [ -1, %{padding} ]"
	));
	let value_index = ir.affine_index(values, "%global_id", "sort_init_values");
	let index_index = ir.affine_index(indices, "%global_id", "sort_init_indices");
	ir.store(values, &value_index, &value, "sort_init_values")?;
	ir.store(indices, &index_index, &original_index, "sort_init_indices")?;
	finish_lane(ir);
	Ok(())
}

fn emit_sort_compare(ir: &mut Ir, signature: &StageSignature, compare: SortCompareStage) -> Result<(), LoweringError> {
	validate_sort_network(compare.network)?;
	let values = signature.pointer(0)?;
	let indices = signature.pointer(1)?;
	ensure_stage(
		indices.dtype == DType::I32
			&& values.view.logical_extents.as_slice()
				== [compare.network.slices * compare.network.padded_axis_length]
			&& indices.view.logical_extents == values.view.logical_extents
			&& compare.merge_width.is_power_of_two()
			&& compare.compare_distance.is_power_of_two()
			&& compare.compare_distance < compare.merge_width,
		"sort compare bindings or network phase are invalid",
	)?;
	let comparisons_per_slice = compare.network.padded_axis_length / 2;
	begin_lane(ir);
	let slice = ir.temporary("sort_compare_slice");
	let pair = ir.temporary("sort_compare_pair");
	ir.line(format_args!(
		"  {slice} = udiv i64 %global_id, {comparisons_per_slice}"
	));
	ir.line(format_args!(
		"  {pair} = urem i64 %global_id, {comparisons_per_slice}"
	));
	let group = ir.temporary("sort_compare_group");
	let inside = ir.temporary("sort_compare_inside");
	ir.line(format_args!(
		"  {group} = udiv i64 {pair}, {}",
		compare.compare_distance
	));
	ir.line(format_args!(
		"  {inside} = urem i64 {pair}, {}",
		compare.compare_distance
	));
	let group_base = ir.temporary("sort_compare_group_base");
	ir.line(format_args!(
		"  {group_base} = mul i64 {group}, {}",
		compare.compare_distance * 2
	));
	let left_position = ir.temporary("sort_compare_left_position");
	let right_position = ir.temporary("sort_compare_right_position");
	ir.line(format_args!(
		"  {left_position} = add i64 {group_base}, {inside}"
	));
	ir.line(format_args!(
		"  {right_position} = add i64 {left_position}, {}",
		compare.compare_distance
	));
	let slice_base = ir.temporary("sort_compare_slice_base");
	ir.line(format_args!(
		"  {slice_base} = mul i64 {slice}, {}",
		compare.network.padded_axis_length
	));
	let left_linear = ir.temporary("sort_compare_left_linear");
	let right_linear = ir.temporary("sort_compare_right_linear");
	ir.line(format_args!(
		"  {left_linear} = add i64 {slice_base}, {left_position}"
	));
	ir.line(format_args!(
		"  {right_linear} = add i64 {slice_base}, {right_position}"
	));
	let left_value_index = ir.affine_index(values, &left_linear, "sort_left_value");
	let right_value_index = ir.affine_index(values, &right_linear, "sort_right_value");
	let left_index_index = ir.affine_index(indices, &left_linear, "sort_left_index");
	let right_index_index = ir.affine_index(indices, &right_linear, "sort_right_index");
	let left_value = ir.load(values, &left_value_index, "sort_left_value")?;
	let right_value = ir.load(values, &right_value_index, "sort_right_value")?;
	let left_index = ir.load(indices, &left_index_index, "sort_left_index")?;
	let right_index = ir.load(indices, &right_index_index, "sort_right_index")?;
	let right_before_left = sort_before(
		ir,
		SortComparison {
			dtype: values.dtype,
			direction: compare.network.direction,
			left: (&right_value, &right_index),
			right: (&left_value, &left_index),
			purpose: "sort_right_before",
		},
	);
	let left_before_right = sort_before(
		ir,
		SortComparison {
			dtype: values.dtype,
			direction: compare.network.direction,
			left: (&left_value, &left_index),
			right: (&right_value, &right_index),
			purpose: "sort_left_before",
		},
	);
	let merge_mask = compare.merge_width;
	let masked = ir.temporary("sort_merge_masked");
	ir.line(format_args!(
		"  {masked} = and i64 {left_position}, {merge_mask}"
	));
	let ascending_phase = ir.temporary("sort_ascending_phase");
	ir.line(format_args!(
		"  {ascending_phase} = icmp eq i64 {masked}, 0"
	));
	let should_swap = ir.temporary("sort_should_swap");
	ir.line(format_args!(
		"  {should_swap} = select i1 {ascending_phase}, i1 {right_before_left}, i1 {left_before_right}"
	));
	let low_value = ir.temporary("sort_low_value");
	let high_value = ir.temporary("sort_high_value");
	let low_index = ir.temporary("sort_low_index");
	let high_index = ir.temporary("sort_high_index");
	ir.line(format_args!(
		"  {low_value} = select i1 {should_swap}, {} {right_value}, {} {left_value}",
		llvm_type(values.dtype),
		llvm_type(values.dtype)
	));
	ir.line(format_args!(
		"  {high_value} = select i1 {should_swap}, {} {left_value}, {} {right_value}",
		llvm_type(values.dtype),
		llvm_type(values.dtype)
	));
	ir.line(format_args!(
		"  {low_index} = select i1 {should_swap}, i32 {right_index}, i32 {left_index}"
	));
	ir.line(format_args!(
		"  {high_index} = select i1 {should_swap}, i32 {left_index}, i32 {right_index}"
	));
	ir.store(values, &left_value_index, &low_value, "sort_left_value")?;
	ir.store(values, &right_value_index, &high_value, "sort_right_value")?;
	ir.store(indices, &left_index_index, &low_index, "sort_left_index")?;
	ir.store(indices, &right_index_index, &high_index, "sort_right_index")?;
	finish_lane(ir);
	Ok(())
}

fn emit_sort_finalize(
	ir: &mut Ir,
	signature: &StageSignature,
	network: SortNetwork,
	emit_indices: bool,
) -> Result<(), LoweringError> {
	validate_sort_network(network)?;
	let values = signature.pointer(0)?;
	let indices = signature.pointer(1)?;
	let output = signature.pointer(2)?;
	let index_output = match emit_indices {
		true => Some(signature.pointer(3)?),
		false => None,
	};
	ensure_stage(
		values.dtype == output.dtype
			&& indices.dtype == DType::I32
			&& index_output.is_none_or(|pointer| pointer.dtype == DType::I32),
		"sort final binding types are invalid",
	)?;
	begin_lane(ir);
	let output_coordinates = ir.coordinates("%global_id", &output.view.logical_extents, "sort_output");
	ensure_stage(
		network.axis < output_coordinates.len() && output.view.logical_extents[network.axis] == network.axis_length,
		"sort final output axis differs from its network",
	)?;
	let position = output_coordinates[network.axis].clone();
	let free_extents = output
		.view
		.logical_extents
		.iter()
		.copied()
		.enumerate()
		.filter(|(axis, _)| *axis != network.axis)
		.map(|(_, extent)| extent)
		.collect::<Vec<_>>();
	let free_coordinates = output_coordinates
		.iter()
		.enumerate()
		.filter(|(axis, _)| *axis != network.axis)
		.map(|(_, coordinate)| coordinate.clone())
		.collect::<Vec<_>>();
	let slice = emit_linear_from_coordinates(ir, &free_coordinates, &free_extents, "sort_output_slice");
	let scratch_base = ir.temporary("sort_output_scratch_base");
	ir.line(format_args!(
		"  {scratch_base} = mul i64 {slice}, {}",
		network.padded_axis_length
	));
	let scratch_linear = ir.temporary("sort_output_scratch_linear");
	ir.line(format_args!(
		"  {scratch_linear} = add i64 {scratch_base}, {position}"
	));
	let value_index = ir.affine_index(values, &scratch_linear, "sort_output_value");
	let original_index = ir.affine_index(indices, &scratch_linear, "sort_output_index");
	let value = ir.load(values, &value_index, "sort_output_value")?;
	let index = ir.load(indices, &original_index, "sort_output_index")?;
	let output_index = ir.index_from_coordinates(output, &output_coordinates, "sort_output");
	ir.store(output, &output_index, &value, "sort_output")?;
	match index_output {
		Some(index_output) => {
			let physical = ir.index_from_coordinates(index_output, &output_coordinates, "sort_index_output");
			ir.store(index_output, &physical, &index, "sort_index_output")
		}
		None => Ok(()),
	}?;
	finish_lane(ir);
	Ok(())
}

fn sort_tensor_index(
	ir: &mut Ir,
	pointer: &BoundPointer,
	network: SortNetwork,
	slice: &str,
	position: &str,
	purpose: &str,
) -> Result<String, LoweringError> {
	ensure_stage(
		network.axis < pointer.view.logical_extents.len()
			&& pointer.view.logical_extents[network.axis] == network.axis_length,
		"sort input axis differs from its network",
	)?;
	let free_extents = pointer
		.view
		.logical_extents
		.iter()
		.copied()
		.enumerate()
		.filter(|(axis, _)| *axis != network.axis)
		.map(|(_, extent)| extent)
		.collect::<Vec<_>>();
	let mut free = ir.coordinates(slice, &free_extents, purpose).into_iter();
	let mut coordinates = Vec::with_capacity(pointer.view.logical_extents.len());
	for axis in 0..pointer.view.logical_extents.len() {
		coordinates.push(match axis == network.axis {
			true => position.to_owned(),
			false => free
				.next()
				.ok_or_else(|| stage_error("sort input coordinate underflow"))?,
		});
	}
	Ok(ir.index_from_coordinates(pointer, &coordinates, purpose))
}

#[derive(Clone, Copy, Debug)]
struct SortComparison<'a> {
	dtype: DType,
	direction: SortDirection,
	left: (&'a str, &'a str),
	right: (&'a str, &'a str),
	purpose: &'a str,
}

fn sort_before(ir: &mut Ir, comparison: SortComparison<'_>) -> String {
	let SortComparison {
		dtype,
		direction,
		left: (left_value, left_index),
		right: (right_value, right_index),
		purpose,
	} = comparison;
	let left_padding = ir.temporary(&format!("{purpose}_left_padding"));
	let right_padding = ir.temporary(&format!("{purpose}_right_padding"));
	ir.line(format_args!(
		"  {left_padding} = icmp eq i32 {left_index}, -1"
	));
	ir.line(format_args!(
		"  {right_padding} = icmp eq i32 {right_index}, -1"
	));
	let left_valid_right_padding = ir.temporary(&format!("{purpose}_valid_padding"));
	let left_valid = ir.temporary(&format!("{purpose}_left_valid"));
	ir.line(format_args!("  {left_valid} = xor i1 {left_padding}, true"));
	ir.line(format_args!(
		"  {left_valid_right_padding} = and i1 {left_valid}, {right_padding}"
	));
	let values_before = match dtype {
		DType::I32 => {
			let value = ir.temporary(&format!("{purpose}_values"));
			let predicate = match direction {
				SortDirection::Ascending => "slt",
				SortDirection::Descending => "sgt",
			};
			ir.line(format_args!(
				"  {value} = icmp {predicate} i32 {left_value}, {right_value}"
			));
			value
		}
		DType::F32 => {
			let left_key = total_order_key(ir, left_value, &format!("{purpose}_left"));
			let right_key = total_order_key(ir, right_value, &format!("{purpose}_right"));
			let value = ir.temporary(&format!("{purpose}_values"));
			let predicate = match direction {
				SortDirection::Ascending => "ult",
				SortDirection::Descending => "ugt",
			};
			ir.line(format_args!(
				"  {value} = icmp {predicate} i32 {left_key}, {right_key}"
			));
			value
		}
	};
	let values_equal = match dtype {
		DType::I32 => {
			let value = ir.temporary(&format!("{purpose}_equal"));
			ir.line(format_args!(
				"  {value} = icmp eq i32 {left_value}, {right_value}"
			));
			value
		}
		DType::F32 => {
			let left_bits = ir.temporary(&format!("{purpose}_left_equal_bits"));
			let right_bits = ir.temporary(&format!("{purpose}_right_equal_bits"));
			ir.line(format_args!(
				"  {left_bits} = bitcast float {left_value} to i32"
			));
			ir.line(format_args!(
				"  {right_bits} = bitcast float {right_value} to i32"
			));
			let value = ir.temporary(&format!("{purpose}_equal"));
			ir.line(format_args!(
				"  {value} = icmp eq i32 {left_bits}, {right_bits}"
			));
			value
		}
	};
	let lower_index = ir.temporary(&format!("{purpose}_lower_index"));
	ir.line(format_args!(
		"  {lower_index} = icmp ult i32 {left_index}, {right_index}"
	));
	let stable_tie = ir.temporary(&format!("{purpose}_stable_tie"));
	ir.line(format_args!(
		"  {stable_tie} = and i1 {values_equal}, {lower_index}"
	));
	let ordered_values = ir.temporary(&format!("{purpose}_ordered_values"));
	ir.line(format_args!(
		"  {ordered_values} = or i1 {values_before}, {stable_tie}"
	));
	let both_valid = ir.temporary(&format!("{purpose}_both_valid"));
	let right_valid = ir.temporary(&format!("{purpose}_right_valid"));
	ir.line(format_args!(
		"  {right_valid} = xor i1 {right_padding}, true"
	));
	ir.line(format_args!(
		"  {both_valid} = and i1 {left_valid}, {right_valid}"
	));
	let valid_order = ir.temporary(&format!("{purpose}_valid_order"));
	ir.line(format_args!(
		"  {valid_order} = and i1 {both_valid}, {ordered_values}"
	));
	let result = ir.temporary(&format!("{purpose}_result"));
	ir.line(format_args!(
		"  {result} = or i1 {left_valid_right_padding}, {valid_order}"
	));
	result
}

fn emit_linear_from_coordinates(ir: &mut Ir, coordinates: &[String], extents: &[u64], purpose: &str) -> String {
	let mut result = None;
	for (axis, (coordinate, extent)) in coordinates.iter().zip(extents).enumerate() {
		result = Some(match result {
			Some(previous) => {
				let value = ir.temporary(&format!("{purpose}_axis_{axis}_scaled"));
				ir.line(format_args!("  {value} = mul i64 {previous}, {extent}"));
				let linear = ir.temporary(&format!("{purpose}_axis_{axis}_linear"));
				ir.line(format_args!("  {linear} = add i64 {value}, {coordinate}"));
				linear
			}
			None => coordinate.clone(),
		});
	}
	result.unwrap_or_else(|| "0".to_owned())
}

fn emit_philox(
	ir: &mut Ir,
	signature: &StageSignature,
	build: &ArtifactBuildRecipe,
	contract: &Philox10Contract,
	target: &KernelTarget,
) -> Result<(), LoweringError> {
	ensure_stage(
		signature.has_run_id
			&& signature.has_loop_iteration
			&& contract.rounds == 10
			&& contract.multiplier_0 == 0xd251_1f53
			&& contract.multiplier_1 == 0xcd9e_8d57
			&& contract.weyl_0 == 0x9e37_79b9
			&& contract.weyl_1 == 0xbb67_ae85
			&& contract.counter
				== [
					PhiloxCounterWord::ElementLow,
					PhiloxCounterWord::ElementHigh,
					PhiloxCounterWord::IterationXorStreamLow,
					PhiloxCounterWord::IterationXorStreamHigh,
				] && contract.fold_kernel_id_into_key
			&& contract.fold_run_id_into_key
			&& contract.uniform_i32 == UniformI32Mapping::UnbiasedMultiplyHighWithCounterRejection
			&& contract.normal_f32 == NormalF32Mapping::OwnedBoxMullerV1,
		"Philox stage is not the Recipe Philox4x32-10 v1 contract",
	)?;
	let output = signature.pointer(0)?;
	let expected_dtype = match contract.distribution {
		RandomDistribution::UniformF32 | RandomDistribution::NormalF32 => DType::F32,
		RandomDistribution::BernoulliI32 { .. } | RandomDistribution::UniformI32 { .. } => DType::I32,
	};
	ensure_stage(
		output.dtype == expected_dtype,
		"Philox output dtype differs from its distribution",
	)?;
	let helper = ensure_philox_helper(ir, contract);
	begin_lane(ir);
	let element_low = ir.temporary("philox_element_low");
	let element_high = ir.temporary("philox_element_high");
	let element_high_wide = ir.temporary("philox_element_high_wide");
	ir.line(format_args!(
		"  {element_low} = trunc i64 %global_id to i32"
	));
	ir.line(format_args!(
		"  {element_high_wide} = lshr i64 %global_id, 32"
	));
	ir.line(format_args!(
		"  {element_high} = trunc i64 {element_high_wide} to i32"
	));
	let stream_xor = ir.temporary("philox_iteration_stream");
	ir.line(format_args!(
		"  {stream_xor} = xor i64 %loop_iteration, {}",
		contract.key.stream
	));
	let stream_low = ir.temporary("philox_stream_low");
	let stream_high = ir.temporary("philox_stream_high");
	let stream_high_wide = ir.temporary("philox_stream_high_wide");
	ir.line(format_args!(
		"  {stream_low} = trunc i64 {stream_xor} to i32"
	));
	ir.line(format_args!(
		"  {stream_high_wide} = lshr i64 {stream_xor}, 32"
	));
	ir.line(format_args!(
		"  {stream_high} = trunc i64 {stream_high_wide} to i32"
	));
	let kernel = build.source_kernel.get();
	let seed_low = split_u64(contract.key.seed_low);
	let seed_high = split_u64(contract.key.seed_high);
	let kernel_words = split_u64(kernel);
	let key_base_0 = seed_low[0] ^ seed_high[1] ^ kernel_words[0];
	let key_base_1 = seed_low[1] ^ seed_high[0] ^ kernel_words[1];
	let run_low = ir.temporary("philox_run_low");
	let run_high_wide = ir.temporary("philox_run_high_wide");
	let run_high = ir.temporary("philox_run_high");
	ir.line(format_args!("  {run_low} = trunc i64 %run_id to i32"));
	ir.line(format_args!("  {run_high_wide} = lshr i64 %run_id, 32"));
	ir.line(format_args!(
		"  {run_high} = trunc i64 {run_high_wide} to i32"
	));
	let key_0 = ir.temporary("philox_key_0");
	let key_1 = ir.temporary("philox_key_1");
	ir.line(format_args!("  {key_0} = xor i32 {run_low}, {key_base_0}"));
	ir.line(format_args!("  {key_1} = xor i32 {run_high}, {key_base_1}"));

	let result = match contract.distribution {
		RandomDistribution::UniformF32 => {
			let words = emit_philox_call(
				ir,
				PhiloxCall {
					helper: &helper,
					counter: [&element_low, &element_high, &stream_low, &stream_high],
					key: [&key_0, &key_1],
					purpose: "philox_uniform",
				},
			);
			uniform_f32(ir, &words[0], "philox_uniform")
		}
		RandomDistribution::BernoulliI32 { probability_bits } => {
			let words = emit_philox_call(
				ir,
				PhiloxCall {
					helper: &helper,
					counter: [&element_low, &element_high, &stream_low, &stream_high],
					key: [&key_0, &key_1],
					purpose: "philox_bernoulli",
				},
			);
			let probability = f32::from_bits(probability_bits);
			let threshold = bernoulli_threshold(probability)?;
			match threshold {
				4_294_967_296 => "1".to_owned(),
				0..=4_294_967_295 => {
					let accepted = ir.temporary("philox_bernoulli_accepted");
					ir.line(format_args!(
						"  {accepted} = icmp ult i32 {}, {}",
						words[0], threshold
					));
					let value = ir.temporary("philox_bernoulli");
					ir.line(format_args!("  {value} = zext i1 {accepted} to i32"));
					value
				}
				4_294_967_297..=u64::MAX => unreachable!("validated probability threshold"),
			}
		}
		RandomDistribution::UniformI32 {
			low,
			high_exclusive,
		} => emit_uniform_i32(
			ir,
			PhiloxInputs {
				helper: &helper,
				element_low: &element_low,
				element_high: &element_high,
				stream_low: &stream_low,
				stream_high: &stream_high,
				key: [&key_0, &key_1],
			},
			low,
			high_exclusive,
			target,
		)?,
		RandomDistribution::NormalF32 => {
			let words = emit_philox_call(
				ir,
				PhiloxCall {
					helper: &helper,
					counter: [&element_low, &element_high, &stream_low, &stream_high],
					key: [&key_0, &key_1],
					purpose: "philox_normal",
				},
			);
			owned_box_muller_v1(ir, &words[0], &words[1])
		}
	};
	let output_index = ir.affine_index(output, "%global_id", "philox_output");
	ir.store(output, &output_index, &result, "philox_output")?;
	finish_lane(ir);
	Ok(())
}

fn split_u64(value: u64) -> [u32; 2] {
	let bytes = value.to_le_bytes();
	[
		u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
		u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
	]
}

fn bernoulli_threshold(probability: f32) -> Result<u64, LoweringError> {
	ensure_stage(
		probability.is_finite() && (0.0..=1.0).contains(&probability),
		"Bernoulli probability is outside the closed unit interval",
	)?;
	match probability.to_bits() {
		0x3f80_0000 => Ok(4_294_967_296),
		bits => {
			let exponent_raw = (bits >> 23).to_le_bytes()[0];
			match exponent_raw {
				0 => Ok(0),
				1..=126 => {
					let significand = u64::from((bits & 0x007f_ffff) | 0x0080_0000);
					let shift = i32::from(exponent_raw) - 118;
					match shift {
						i32::MIN..=-1 => Ok(significand >> shift.unsigned_abs().min(63)),
						0..=8 => Ok(significand << shift.unsigned_abs()),
						9..=i32::MAX => Err(stage_error("Bernoulli probability exponent is invalid")),
					}
				}
				127..=u8::MAX => Err(stage_error("Bernoulli probability exponent is invalid")),
			}
		}
	}
}

fn ensure_philox_helper(ir: &mut Ir, contract: &Philox10Contract) -> String {
	let name = "recipe_philox4x32_10_v1".to_owned();
	let mut body = String::from(
		"define internal { i32, i32, i32, i32 } @recipe_philox4x32_10_v1(i32 %counter_0, i32 %counter_1, i32 %counter_2, i32 %counter_3, i32 %key_0, i32 %key_1) #0 {\nentry:\n",
	);
	let mut counters = [
		"%counter_0".to_owned(),
		"%counter_1".to_owned(),
		"%counter_2".to_owned(),
		"%counter_3".to_owned(),
	];
	let mut keys = ["%key_0".to_owned(), "%key_1".to_owned()];
	for round in [0_u8, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
		let wide_0 = format!("%round_{round}_wide_0");
		let wide_1 = format!("%round_{round}_wide_1");
		let input_0 = format!("%round_{round}_input_0");
		let input_2 = format!("%round_{round}_input_2");
		push_text_line(
			&mut body,
			format_args!("  {input_0} = zext i32 {} to i64", counters[0]),
		);
		push_text_line(
			&mut body,
			format_args!("  {input_2} = zext i32 {} to i64", counters[2]),
		);
		push_text_line(
			&mut body,
			format_args!("  {wide_0} = mul i64 {input_0}, {}", contract.multiplier_0),
		);
		push_text_line(
			&mut body,
			format_args!("  {wide_1} = mul i64 {input_2}, {}", contract.multiplier_1),
		);
		let high_0_64 = format!("%round_{round}_high_0_64");
		let high_1_64 = format!("%round_{round}_high_1_64");
		let high_0 = format!("%round_{round}_high_0");
		let high_1 = format!("%round_{round}_high_1");
		let low_0 = format!("%round_{round}_low_0");
		let low_1 = format!("%round_{round}_low_1");
		push_text_line(
			&mut body,
			format_args!("  {high_0_64} = lshr i64 {wide_0}, 32"),
		);
		push_text_line(
			&mut body,
			format_args!("  {high_1_64} = lshr i64 {wide_1}, 32"),
		);
		push_text_line(
			&mut body,
			format_args!("  {high_0} = trunc i64 {high_0_64} to i32"),
		);
		push_text_line(
			&mut body,
			format_args!("  {high_1} = trunc i64 {high_1_64} to i32"),
		);
		push_text_line(
			&mut body,
			format_args!("  {low_0} = trunc i64 {wide_0} to i32"),
		);
		push_text_line(
			&mut body,
			format_args!("  {low_1} = trunc i64 {wide_1} to i32"),
		);
		let xor_0 = format!("%round_{round}_xor_0");
		let next_0 = format!("%round_{round}_counter_0");
		let xor_2 = format!("%round_{round}_xor_2");
		let next_2 = format!("%round_{round}_counter_2");
		push_text_line(
			&mut body,
			format_args!("  {xor_0} = xor i32 {high_1}, {}", counters[1]),
		);
		push_text_line(
			&mut body,
			format_args!("  {next_0} = xor i32 {xor_0}, {}", keys[0]),
		);
		push_text_line(
			&mut body,
			format_args!("  {xor_2} = xor i32 {high_0}, {}", counters[3]),
		);
		push_text_line(
			&mut body,
			format_args!("  {next_2} = xor i32 {xor_2}, {}", keys[1]),
		);
		counters = [next_0, low_1, next_2, low_0];
		keys = match round {
			0..=8 => {
				let next_key_0 = format!("%round_{round}_key_0");
				let next_key_1 = format!("%round_{round}_key_1");
				push_text_line(
					&mut body,
					format_args!("  {next_key_0} = add i32 {}, {}", keys[0], contract.weyl_0),
				);
				push_text_line(
					&mut body,
					format_args!("  {next_key_1} = add i32 {}, {}", keys[1], contract.weyl_1),
				);
				[next_key_0, next_key_1]
			}
			9 => keys,
			10..=u8::MAX => unreachable!("fixed Philox round list"),
		};
	}
	body.push_str("  %result_0 = insertvalue { i32, i32, i32, i32 } poison, i32 ");
	body.push_str(&counters[0]);
	body.push_str(", 0\n  %result_1 = insertvalue { i32, i32, i32, i32 } %result_0, i32 ");
	body.push_str(&counters[1]);
	body.push_str(", 1\n  %result_2 = insertvalue { i32, i32, i32, i32 } %result_1, i32 ");
	body.push_str(&counters[2]);
	body.push_str(", 2\n  %result_3 = insertvalue { i32, i32, i32, i32 } %result_2, i32 ");
	body.push_str(&counters[3]);
	body.push_str(", 3\n  ret { i32, i32, i32, i32 } %result_3\n}\n");
	ir.helpers.entry(name.clone()).or_insert(body);
	name
}

fn push_text_line(text: &mut String, arguments: std::fmt::Arguments<'_>) {
	text.push_str(&arguments.to_string());
	text.push('\n');
}

#[derive(Clone, Copy, Debug)]
struct PhiloxCall<'a> {
	helper: &'a str,
	counter: [&'a str; 4],
	key: [&'a str; 2],
	purpose: &'a str,
}

fn emit_philox_call(ir: &mut Ir, call: PhiloxCall<'_>) -> [String; 4] {
	let tuple = ir.temporary(&format!("{}_words", call.purpose));
	ir.line(format_args!(
		"  {tuple} = call {{ i32, i32, i32, i32 }} @{}(i32 {}, i32 {}, i32 {}, i32 {}, i32 {}, i32 {})",
		call.helper, call.counter[0], call.counter[1], call.counter[2], call.counter[3], call.key[0], call.key[1]
	));
	std::array::from_fn(|index| {
		let word = ir.temporary(&format!("{}_word_{index}", call.purpose));
		ir.line(format_args!(
			"  {word} = extractvalue {{ i32, i32, i32, i32 }} {tuple}, {index}"
		));
		word
	})
}

fn uniform_f32(ir: &mut Ir, word: &str, purpose: &str) -> String {
	let fraction = ir.temporary(&format!("{purpose}_fraction"));
	let bits = ir.temporary(&format!("{purpose}_bits"));
	let value = ir.temporary(&format!("{purpose}_value"));
	ir.line(format_args!("  {fraction} = lshr i32 {word}, 9"));
	ir.line(format_args!("  {bits} = or i32 {fraction}, 1065353216"));
	ir.line(format_args!("  {value} = bitcast i32 {bits} to float"));
	let one = literal_operand(
		ir,
		ScalarLiteral::F32Bits(1.0_f32.to_bits()),
		&format!("{purpose}_one"),
	);
	constrained_binary(ir, "fsub", &value, &one, purpose)
}

#[derive(Clone, Copy, Debug)]
struct PhiloxInputs<'a> {
	helper: &'a str,
	element_low: &'a str,
	element_high: &'a str,
	stream_low: &'a str,
	stream_high: &'a str,
	key: [&'a str; 2],
}

fn emit_uniform_i32(
	ir: &mut Ir,
	inputs: PhiloxInputs<'_>,
	low: i32,
	high_exclusive: i32,
	target: &KernelTarget,
) -> Result<String, LoweringError> {
	let private_address_space = private_address_space(target);
	let range_i64 = i64::from(high_exclusive) - i64::from(low);
	let range = match u32::try_from(range_i64) {
		Ok(range) => range,
		Err(error) => return Err(stage_error(error.to_string())),
	};
	let threshold = range.wrapping_neg() % range;
	let attempt_slot = ir.temporary("uniform_i32_attempt_slot");
	let result_slot = ir.temporary("uniform_i32_result_slot");
	ir.line(format_args!(
		"  {attempt_slot} = alloca i32, align 4, addrspace({private_address_space})"
	));
	ir.line(format_args!(
		"  {result_slot} = alloca i32, align 4, addrspace({private_address_space})"
	));
	ir.line(format_args!(
		"  store i32 0, ptr addrspace({private_address_space}) {attempt_slot}, align 4"
	));
	let retry = ir.label("uniform_i32_retry");
	let accepted = ir.label("uniform_i32_accepted");
	let rejected = ir.label("uniform_i32_rejected");
	let joined = ir.label("uniform_i32_joined");
	ir.line(format_args!("  br label %{retry}"));
	ir.line(format_args!("{retry}:"));
	let attempt = ir.temporary("uniform_i32_attempt");
	ir.line(format_args!(
		"  {attempt} = load i32, ptr addrspace({private_address_space}) {attempt_slot}, align 4"
	));
	let retry_counter = ir.temporary("uniform_i32_retry_counter");
	ir.line(format_args!(
		"  {retry_counter} = xor i32 {}, {attempt}",
		inputs.stream_high
	));
	let words = emit_philox_call(
		ir,
		PhiloxCall {
			helper: inputs.helper,
			counter: [
				inputs.element_low,
				inputs.element_high,
				inputs.stream_low,
				&retry_counter,
			],
			key: inputs.key,
			purpose: "uniform_i32",
		},
	);
	let random = ir.temporary("uniform_i32_random");
	ir.line(format_args!("  {random} = zext i32 {} to i64", words[0]));
	let product = ir.temporary("uniform_i32_product");
	ir.line(format_args!("  {product} = mul i64 {random}, {range}"));
	let low_product = ir.temporary("uniform_i32_low_product");
	ir.line(format_args!("  {low_product} = trunc i64 {product} to i32"));
	let is_accepted = ir.temporary("uniform_i32_is_accepted");
	ir.line(format_args!(
		"  {is_accepted} = icmp uge i32 {low_product}, {threshold}"
	));
	ir.line(format_args!(
		"  br i1 {is_accepted}, label %{accepted}, label %{rejected}"
	));
	ir.line(format_args!("{rejected}:"));
	let next_attempt = ir.temporary("uniform_i32_next_attempt");
	ir.line(format_args!("  {next_attempt} = add i32 {attempt}, 1"));
	ir.line(format_args!(
		"  store i32 {next_attempt}, ptr addrspace({private_address_space}) {attempt_slot}, align 4"
	));
	ir.line(format_args!("  br label %{retry}"));
	ir.line(format_args!("{accepted}:"));
	let high_product = ir.temporary("uniform_i32_high_product");
	let mapped = ir.temporary("uniform_i32_mapped");
	let result = ir.temporary("uniform_i32_result");
	ir.line(format_args!("  {high_product} = lshr i64 {product}, 32"));
	ir.line(format_args!("  {mapped} = trunc i64 {high_product} to i32"));
	ir.line(format_args!("  {result} = add i32 {mapped}, {low}"));
	ir.line(format_args!(
		"  store i32 {result}, ptr addrspace({private_address_space}) {result_slot}, align 4"
	));
	ir.line(format_args!("  br label %{joined}"));
	ir.line(format_args!("{joined}:"));
	let loaded = ir.temporary("uniform_i32_output");
	ir.line(format_args!(
		"  {loaded} = load i32, ptr addrspace({private_address_space}) {result_slot}, align 4"
	));
	Ok(loaded)
}

fn owned_box_muller_v1(ir: &mut Ir, first: &str, second: &str) -> String {
	let u1 = positive_uniform_f32(ir, first, "normal_u1");
	let u2 = positive_uniform_f32(ir, second, "normal_u2");
	let log_u1 = owned_log_v1(ir, &u1);
	let negative_two = literal_operand(
		ir,
		ScalarLiteral::F32Bits((-2.0_f32).to_bits()),
		"normal_negative_two",
	);
	let radial_squared = constrained_binary(ir, "fmul", &negative_two, &log_u1, "normal_radial_squared");
	let radial = square_root(ir, &radial_squared, "normal_radial");
	let two_pi = literal_operand(
		ir,
		ScalarLiteral::F32Bits(std::f32::consts::TAU.to_bits()),
		"normal_two_pi",
	);
	let angle = constrained_binary(ir, "fmul", &two_pi, &u2, "normal_angle");
	let cosine = owned_cos_v1(ir, &angle);
	constrained_binary(ir, "fmul", &radial, &cosine, "normal_result")
}

fn positive_uniform_f32(ir: &mut Ir, word: &str, purpose: &str) -> String {
	let upper = ir.temporary(&format!("{purpose}_upper"));
	let incremented = ir.temporary(&format!("{purpose}_incremented"));
	let converted = ir.temporary(&format!("{purpose}_converted"));
	ir.line(format_args!("  {upper} = lshr i32 {word}, 8"));
	ir.line(format_args!("  {incremented} = add i32 {upper}, 1"));
	ir.line(format_args!(
		"  {converted} = uitofp i32 {incremented} to float"
	));
	let scale = literal_operand(
		ir,
		ScalarLiteral::F32Bits((1.0_f32 / 16_777_216.0_f32).to_bits()),
		&format!("{purpose}_scale"),
	);
	constrained_binary(ir, "fmul", &converted, &scale, purpose)
}

fn owned_log_v1(ir: &mut Ir, value: &str) -> String {
	let bits = ir.temporary("owned_log_bits");
	let exponent_bits = ir.temporary("owned_log_exponent_bits");
	let exponent_i32 = ir.temporary("owned_log_exponent_i32");
	let exponent = ir.temporary("owned_log_exponent");
	let mantissa_bits = ir.temporary("owned_log_mantissa_bits");
	let mantissa = ir.temporary("owned_log_mantissa");
	ir.line(format_args!("  {bits} = bitcast float {value} to i32"));
	ir.line(format_args!("  {exponent_bits} = lshr i32 {bits}, 23"));
	ir.line(format_args!(
		"  {exponent_i32} = add i32 {exponent_bits}, -127"
	));
	ir.line(format_args!(
		"  {exponent} = sitofp i32 {exponent_i32} to float"
	));
	let fraction = ir.temporary("owned_log_fraction");
	ir.line(format_args!("  {fraction} = and i32 {bits}, 8388607"));
	ir.line(format_args!(
		"  {mantissa_bits} = or i32 {fraction}, 1065353216"
	));
	ir.line(format_args!(
		"  {mantissa} = bitcast i32 {mantissa_bits} to float"
	));
	let one = literal_operand(
		ir,
		ScalarLiteral::F32Bits(1.0_f32.to_bits()),
		"owned_log_one",
	);
	let numerator = constrained_binary(ir, "fsub", &mantissa, &one, "owned_log_numerator");
	let denominator = constrained_binary(ir, "fadd", &mantissa, &one, "owned_log_denominator");
	let y = plain_fdiv(ir, &numerator, &denominator, "owned_log_y");
	let y2 = constrained_binary(ir, "fmul", &y, &y, "owned_log_y2");
	let mut power = y.clone();
	let mut sum = y;
	for divisor in [3_u16, 5, 7, 9, 11, 13, 15] {
		power = constrained_binary(ir, "fmul", &power, &y2, "owned_log_power");
		let reciprocal = literal_operand(
			ir,
			ScalarLiteral::F32Bits((1.0_f32 / f32::from(divisor)).to_bits()),
			"owned_log_reciprocal",
		);
		let term = constrained_binary(ir, "fmul", &power, &reciprocal, "owned_log_term");
		sum = constrained_binary(ir, "fadd", &sum, &term, "owned_log_sum");
	}
	let two = literal_operand(
		ir,
		ScalarLiteral::F32Bits(2.0_f32.to_bits()),
		"owned_log_two",
	);
	let log_mantissa = constrained_binary(ir, "fmul", &sum, &two, "owned_log_mantissa");
	let ln2 = literal_operand(
		ir,
		ScalarLiteral::F32Bits(std::f32::consts::LN_2.to_bits()),
		"owned_log_ln2",
	);
	let exponent_term = constrained_binary(ir, "fmul", &exponent, &ln2, "owned_log_exponent_term");
	constrained_binary(
		ir,
		"fadd",
		&log_mantissa,
		&exponent_term,
		"owned_log_result",
	)
}

fn owned_cos_v1(ir: &mut Ir, angle: &str) -> String {
	let pi = literal_operand(
		ir,
		ScalarLiteral::F32Bits(std::f32::consts::PI.to_bits()),
		"owned_cos_pi",
	);
	let tau = literal_operand(
		ir,
		ScalarLiteral::F32Bits(std::f32::consts::TAU.to_bits()),
		"owned_cos_tau",
	);
	let above_pi = ir.temporary("owned_cos_above_pi");
	ir.line(format_args!("  {above_pi} = fcmp ogt float {angle}, {pi}"));
	let reduced_candidate = constrained_binary(ir, "fsub", angle, &tau, "owned_cos_reduced_candidate");
	let reduced = ir.temporary("owned_cos_reduced");
	ir.line(format_args!(
		"  {reduced} = select i1 {above_pi}, float {reduced_candidate}, float {angle}"
	));
	let reduced_bits = ir.temporary("owned_cos_reduced_bits");
	let absolute_bits = ir.temporary("owned_cos_absolute_bits");
	let absolute = ir.temporary("owned_cos_absolute");
	ir.line(format_args!(
		"  {reduced_bits} = bitcast float {reduced} to i32"
	));
	ir.line(format_args!(
		"  {absolute_bits} = and i32 {reduced_bits}, 2147483647"
	));
	ir.line(format_args!(
		"  {absolute} = bitcast i32 {absolute_bits} to float"
	));
	let half_pi = literal_operand(
		ir,
		ScalarLiteral::F32Bits(std::f32::consts::FRAC_PI_2.to_bits()),
		"owned_cos_half_pi",
	);
	let reflect = ir.temporary("owned_cos_reflect");
	ir.line(format_args!(
		"  {reflect} = fcmp ogt float {absolute}, {half_pi}"
	));
	let reflected = constrained_binary(ir, "fsub", &pi, &absolute, "owned_cos_reflected");
	let x = ir.temporary("owned_cos_x");
	ir.line(format_args!(
		"  {x} = select i1 {reflect}, float {reflected}, float {absolute}"
	));
	let x2 = constrained_binary(ir, "fmul", &x, &x, "owned_cos_x2");
	let mut polynomial = literal_operand(
		ir,
		ScalarLiteral::F32Bits((-1.0_f32 / 3_628_800.0_f32).to_bits()),
		"owned_cos_c10",
	);
	for coefficient in [
		1.0_f32 / 40_320.0_f32,
		-1.0_f32 / 720.0_f32,
		1.0_f32 / 24.0_f32,
		-0.5_f32,
	] {
		polynomial = constrained_binary(ir, "fmul", &polynomial, &x2, "owned_cos_horner_mul");
		let coefficient = literal_operand(
			ir,
			ScalarLiteral::F32Bits(coefficient.to_bits()),
			"owned_cos_coefficient",
		);
		polynomial = constrained_binary(
			ir,
			"fadd",
			&polynomial,
			&coefficient,
			"owned_cos_horner_add",
		);
	}
	polynomial = constrained_binary(ir, "fmul", &polynomial, &x2, "owned_cos_final_mul");
	let one = literal_operand(
		ir,
		ScalarLiteral::F32Bits(1.0_f32.to_bits()),
		"owned_cos_one",
	);
	let positive = constrained_binary(ir, "fadd", &polynomial, &one, "owned_cos_positive");
	let positive_bits = ir.temporary("owned_cos_positive_bits");
	let negative_bits = ir.temporary("owned_cos_negative_bits");
	let negative = ir.temporary("owned_cos_negative");
	ir.line(format_args!(
		"  {positive_bits} = bitcast float {positive} to i32"
	));
	ir.line(format_args!(
		"  {negative_bits} = xor i32 {positive_bits}, -2147483648"
	));
	ir.line(format_args!(
		"  {negative} = bitcast i32 {negative_bits} to float"
	));
	let result = ir.temporary("owned_cos_result");
	ir.line(format_args!(
		"  {result} = select i1 {reflect}, float {negative}, float {positive}"
	));
	result
}

#[cfg(test)]
mod tests {
	use recipe_core::{
		AliasPermission, ArtifactBuildBinding, ArtifactBuildProvenance, ArtifactBuildView,
		ArtifactDispatchGeometry, ArtifactId, ArtifactWorkBounds, KernelTemplateId, ScalarInput, ScalarInstruction,
		ScalarOpcode, ScalarProgram, ScalarValueId, ValueId,
	};
	use recipe_language::{
		AtomicOperation, AtomicOrdering, AxisSet, Contraction, Elementwise, Gather, Histogram, IndexMap,
		PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, RandomKey, RandomMap, Reduce, Scan, ScanMode, Scatter,
		Shape, Sort, Tensor,
	};
	use std::collections::{BTreeMap, HashSet};
	use std::io::Write as _;
	use std::process::{Command, Stdio};

	use super::*;
	use crate::{AmdTarget, NvidiaTarget};

	fn must<T, E: std::fmt::Debug>(result: Result<T, E>, context: &str) -> T {
		match result {
			Ok(value) => value,
			Err(error) => panic!("{context}: {error:?}"),
		}
	}

	fn must_some<T>(value: Option<T>, context: &str) -> T {
		match value {
			Some(value) => value,
			None => panic!("{context}"),
		}
	}

	fn tensor(id: u64, dtype: DType, shape: &[u64]) -> Tensor {
		let shape = must(Shape::new(shape.to_vec()), "test shape");
		must(
			Tensor::contiguous(ValueId::new(id), dtype, shape, true, true),
			"test tensor",
		)
	}

	fn aliases(inputs: usize, outputs: usize) -> Vec<PrimitiveAliasRule> {
		(0..inputs)
			.flat_map(|input| {
				(0..outputs).map(move |output| PrimitiveAliasRule {
					input,
					output,
					permission: AliasPermission::Forbidden,
				})
			})
			.collect()
	}

	fn primitive(id: u64, inputs: &[u64], outputs: &[u64], kind: PrimitiveKind) -> PrimitiveKernel {
		PrimitiveKernel {
			id: KernelTemplateId::new(id),
			inputs: inputs.iter().copied().map(ValueId::new).collect(),
			outputs: outputs.iter().copied().map(ValueId::new).collect(),
			alias_rules: aliases(inputs.len(), outputs.len()),
			kind,
		}
	}

	fn lower_program(kernel: PrimitiveKernel, tensors: &[Tensor]) -> LoweredProgram {
		let index = tensors
			.iter()
			.map(|tensor| (tensor.id, tensor))
			.collect::<BTreeMap<_, _>>();
		must(
			recipe_primitives::lower(
				&kernel,
				&index,
				recipe_primitives::LoweringHardware {
					subgroup_lanes: 32,
					maximum_workgroup_lanes: 256,
					maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
				},
			),
			"primitive lowering",
		)
	}

	fn build(program: &LoweredProgram, stage: &ProgramStage) -> ArtifactBuildRecipe {
		let template = must(stage_template_identity(program, stage), "stage identity");
		let bindings = stage
			.bindings
			.iter()
			.enumerate()
			.map(|(index, binding)| ArtifactBuildBinding {
				value: ValueId::new(must(u64::try_from(index + 100), "binding ID")),
				dtype: binding.dtype,
				access: access(binding.mode),
				view: ArtifactBuildView {
					logical_extents: binding.view.logical_extents.clone(),
					offset_elements: binding.view.offset_elements,
					strides: binding.view.strides.clone(),
					storage_bytes: binding.view.storage_bytes,
				},
			})
			.collect::<Vec<_>>();
		let fault_flag = stage.fault.map(|fault| {
			let position = must_some(
				stage.bindings
					.iter()
					.position(|binding| binding.buffer == fault.flag),
				"fault binding",
			);
			bindings[position].value
		});
		let mut build = ArtifactBuildRecipe {
			artifact: ArtifactId::new(template.get()),
			kernel_template: template,
			source_kernel: program.source_kernel,
			provenance: ArtifactBuildProvenance {
				program_digest: Digest::new(program.digest.bytes()),
				stage_ordinal: stage.id.get(),
				contract_digest: Digest::ZERO,
			},
			bindings,
			dispatch: ArtifactDispatchGeometry {
				logical_lanes: stage.geometry.logical_lanes,
				workgroup_lanes: stage.geometry.workgroup_lanes,
				workgroups: stage.geometry.workgroups,
			},
			work: ArtifactWorkBounds {
				flops: stage.resources.flops,
				integer_operations: stage.resources.integer_operations,
				atomic_operations: stage.resources.atomic_operations,
			},
			fault_flag,
			resources: KernelResourceBounds {
				private_bytes_per_lane: stage.resources.private_bytes_per_lane,
				shared_bytes_per_workgroup: stage.resources.shared_bytes_per_workgroup,
				scratch_bytes_per_dispatch: ByteCount::ZERO,
				maximum_workgroup_lanes: stage.resources.maximum_workgroup_lanes,
			},
		};
		build.provenance.contract_digest = artifact_build_contract_digest(&build);
		build
	}

	fn targets() -> [KernelTarget; 2] {
		[
			KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			KernelTarget::Nvidia(NvidiaTarget {
				sm_major: 8,
				sm_minor: 6,
				ptx_isa: 75,
			}),
		]
	}

	fn programs() -> Vec<LoweredProgram> {
		let scalar_tensors = [
			tensor(1, DType::F32, &[16]),
			tensor(2, DType::F32, &[16]),
			tensor(3, DType::F32, &[16]),
		];
		let left = ScalarValueId::new(1);
		let right = ScalarValueId::new(2);
		let sum = ScalarValueId::new(3);
		let scalar = lower_program(
			primitive(
				1,
				&[1, 2],
				&[3],
				PrimitiveKind::Elementwise(Elementwise {
					program: ScalarProgram {
						inputs: vec![
							ScalarInput {
								id: left,
								dtype: DType::F32,
							},
							ScalarInput {
								id: right,
								dtype: DType::F32,
							},
						],
						constants: Vec::new(),
						instructions: vec![ScalarInstruction {
							result: sum,
							dtype: DType::F32,
							opcode: ScalarOpcode::Add,
							operands: vec![left, right],
						}],
						outputs: vec![sum],
					},
				}),
			),
			&scalar_tensors,
		);
		let reduction_tensors = [
			tensor(10, DType::F32, &[2, 9]),
			tensor(11, DType::F32, &[2]),
			tensor(12, DType::I32, &[2]),
		];
		let reduction = lower_program(
			primitive(
				2,
				&[10],
				&[11, 12],
				PrimitiveKind::Reduce(Reduce {
					operator: ReduceOperator::Maximum,
					axes: must(AxisSet::new(vec![1]), "axes"),
					keep_dimensions: false,
					result: ReduceResult::ValueAndIndex,
					tree_lanes: 4,
				}),
			),
			&reduction_tensors,
		);
		let empty_tensors = [
			tensor(20, DType::I32, &[2, 0]),
			tensor(21, DType::I32, &[2]),
		];
		let fill = lower_program(
			primitive(
				3,
				&[20],
				&[21],
				PrimitiveKind::Reduce(Reduce {
					operator: ReduceOperator::Sum,
					axes: must(AxisSet::new(vec![1]), "axes"),
					keep_dimensions: false,
					result: ReduceResult::Value,
					tree_lanes: 4,
				}),
			),
			&empty_tensors,
		);
		let scan_tensors = [
			tensor(30, DType::F32, &[2, 35]),
			tensor(31, DType::F32, &[2, 35]),
		];
		let scan = lower_program(
			primitive(
				4,
				&[30],
				&[31],
				PrimitiveKind::Scan(Scan {
					operator: ReduceOperator::Sum,
					axis: 1,
					mode: ScanMode::Inclusive,
					reverse: false,
					tree_lanes: 4,
				}),
			),
			&scan_tensors,
		);
		let contraction_tensors = [
			tensor(40, DType::F32, &[2, 3, 5]),
			tensor(41, DType::F32, &[2, 5, 7]),
			tensor(42, DType::F32, &[2, 3, 7]),
		];
		let contraction = lower_program(
			primitive(
				5,
				&[40, 41],
				&[42],
				PrimitiveKind::Contraction(Contraction {
					batch_axes: vec![(0, 0)],
					contract_axes: vec![(2, 1)],
				}),
			),
			&contraction_tensors,
		);
		let gather_tensors = [
			tensor(50, DType::F32, &[4, 9]),
			tensor(51, DType::I32, &[3]),
			tensor(52, DType::F32, &[4, 3]),
		];
		let gather = lower_program(
			primitive(
				6,
				&[50, 51],
				&[52],
				PrimitiveKind::Gather(Gather {
					axis: 1,
					bounds: IndexBounds::Reject,
				}),
			),
			&gather_tensors,
		);
		let scatter_tensors = [
			tensor(60, DType::F32, &[4, 9]),
			tensor(61, DType::I32, &[3]),
			tensor(62, DType::F32, &[4, 3]),
			tensor(63, DType::F32, &[4, 9]),
		];
		let scatter = lower_program(
			primitive(
				7,
				&[60, 61, 62],
				&[63],
				PrimitiveKind::Scatter(Scatter {
					axis: 1,
					bounds: IndexBounds::Reject,
					conflict: ScatterConflict::Atomic {
						operation: AtomicOperation::Add,
						ordering: AtomicOrdering::SequentiallyConsistent,
					},
				}),
			),
			&scatter_tensors,
		);
		let histogram_tensors = [
			tensor(70, DType::F32, &[33]),
			tensor(71, DType::F32, &[33]),
			tensor(72, DType::F32, &[7]),
		];
		let histogram = lower_program(
			primitive(
				8,
				&[70, 71],
				&[72],
				PrimitiveKind::Histogram(Histogram {
					bins: 7,
					weighted: true,
					ordering: AtomicOrdering::SequentiallyConsistent,
				}),
			),
			&histogram_tensors,
		);
		let sort_tensors = [
			tensor(80, DType::F32, &[2, 7]),
			tensor(81, DType::F32, &[2, 7]),
			tensor(82, DType::I32, &[2, 7]),
		];
		let sort = lower_program(
			primitive(
				9,
				&[80],
				&[81, 82],
				PrimitiveKind::Sort(Sort {
					axis: 1,
					direction: SortDirection::Descending,
					stable: true,
					emit_indices: true,
				}),
			),
			&sort_tensors,
		);
		let index_map_tensors = [tensor(89, DType::I32, &[64])];
		let index_map = lower_program(
			primitive(
				89,
				&[],
				&[89],
				PrimitiveKind::IndexMap(IndexMap {
					start: -3,
					element_step: 1,
					iteration_step: 64,
					modulus: Some(101),
				}),
			),
			&index_map_tensors,
		);
		let mut result = vec![
			scalar,
			reduction,
			fill,
			scan,
			contraction,
			gather,
			scatter,
			histogram,
			sort,
			index_map,
		];
		for (ordinal, (distribution, dtype)) in [
			(RandomDistribution::UniformF32, DType::F32),
			(RandomDistribution::NormalF32, DType::F32),
			(
				RandomDistribution::BernoulliI32 {
					probability_bits: 0.25_f32.to_bits(),
				},
				DType::I32,
			),
			(
				RandomDistribution::UniformI32 {
					low: -7,
					high_exclusive: 19,
				},
				DType::I32,
			),
		]
		.into_iter()
		.enumerate()
		{
			let id = 90 + must(u64::try_from(ordinal), "random ID");
			let tensors = [tensor(id, dtype, &[64])];
			result.push(lower_program(
				primitive(
					id,
					&[],
					&[id],
					PrimitiveKind::Random(RandomMap {
						distribution,
						key: RandomKey {
							seed_low: 1,
							seed_high: 2,
							stream: 3,
						},
						philox_rounds: 10,
					}),
				),
				&tensors,
			));
		}
		result
	}

	#[test]
	fn every_owned_stage_lowers_deterministically_for_amd_and_nvidia() {
		let mut kinds = HashSet::new();
		for program in programs() {
			for stage in &program.stages {
				let kind = std::mem::discriminant(&stage.kind);
				kinds.insert(kind);
				let build = build(&program, stage);
				for target in targets() {
					let options = LoweringOptions {
						entry_symbol: format!(
							"recipe_stage_{}_{}",
							program.source_kernel.get(),
							stage.id.get()
						),
						workgroup_lanes: stage.geometry.workgroup_lanes,
					};
					let first = must(
						lower_stage(&program, &build, &target, &options),
						"stage lowering",
					);
					let second = must(
						lower_stage(&program, &build, &target, &options),
						"stage lowering",
					);
					assert_eq!(first, second);
					assert!(audit_llvm_ir(&first.llvm_ir).is_empty());
					assert!(first.llvm_ir.contains("ret void"));
				}
			}
		}
		assert_eq!(kinds.len(), 16);
	}

	#[test]
	fn stack_slots_use_the_explicit_private_address_space_on_both_backends() {
		let programs = programs();
		let stages = [
			must_some(
				programs
					.iter()
					.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
					.find(|(_, stage)| matches!(stage.kind, StageKind::TiledContraction(_))),
				"contraction stage",
			),
			must_some(
				programs
					.iter()
					.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
					.find(|(_, stage)| {
						matches!(
							stage.kind,
							StageKind::Philox4x32_10(contract)
								if matches!(contract.distribution, RandomDistribution::UniformI32 { .. })
						)
					}),
				"uniform int32 stage",
			),
		];
		for (program, stage) in stages {
			for target in targets() {
				let lowered = must(
					lower_stage(
						program,
						&build(program, stage),
						&target,
						&LoweringOptions {
							entry_symbol: "recipe_private_stack".to_owned(),
							workgroup_lanes: stage.geometry.workgroup_lanes,
						},
					),
					"private-stack lowering",
				);
				let allocas = lowered
					.llvm_ir
					.lines()
					.filter(|line| line.contains(" = alloca "))
					.collect::<Vec<_>>();
				assert!(!allocas.is_empty());
				assert!(
					allocas.iter().all(|line| line.contains("addrspace(5)")),
					"target {target:?} emitted a non-private stack slot:\n{}",
					allocas.join("\n")
				);
				assert!(!lowered.llvm_ir.contains(", ptr %contraction_"));
				assert!(!lowered.llvm_ir.contains(", ptr %uniform_i32_"));
			}
		}
	}

	#[test]
	#[ignore = "requires the host LLVM AMDGPU backend"]
	fn host_llc_accepts_amd_contraction_private_stack_slots() {
		let programs = programs();
		let contraction = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| matches!(stage.kind, StageKind::TiledContraction(_))),
			"contraction stage",
		);
		let lowered = must(
			lower_stage(
				contraction.0,
				&build(contraction.0, contraction.1),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_contraction_private_stack".to_owned(),
					workgroup_lanes: contraction.1.geometry.workgroup_lanes,
				},
			),
			"contraction lowering",
		);
		let mut child = Command::new("/usr/bin/llc")
			.args([
				"-filetype=null",
				"-march=amdgcn",
				"-mcpu=gfx1101",
				"--amdhsa-code-object-version=6",
				"-",
			])
			.stdin(Stdio::piped())
			.stdout(Stdio::null())
			.stderr(Stdio::piped())
			.spawn()
			.expect("spawn host llc");
		{
			let mut stdin = child.stdin.take().expect("host llc stdin");
			stdin.write_all(lowered.llvm_ir.as_bytes())
				.expect("write LLVM IR");
		}
		let output = child.wait_with_output().expect("wait for host llc");
		assert!(
			output.status.success(),
			"host llc rejected AMD contraction:\n{}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	#[test]
	fn mismatched_build_recipe_fails_closed() {
		let program = programs().remove(1);
		let stage = &program.stages[0];
		let mut build = build(&program, stage);
		build.dispatch.logical_lanes += 1;
		let error = match lower_stage(
			&program,
			&build,
			&targets()[0],
			&LoweringOptions {
				entry_symbol: "recipe_mismatch".to_owned(),
				workgroup_lanes: stage.geometry.workgroup_lanes,
			},
		) {
			Ok(_) => panic!("tampered build must fail"),
			Err(error) => error,
		};
		assert_eq!(error.kind, LoweringErrorKind::InvalidStageContract);
	}

	#[test]
	fn checked_paths_guard_payload_addresses_and_random_abi_is_explicit() {
		let programs = programs();
		let gather = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| matches!(stage.kind, StageKind::Gather { .. })),
			"gather stage",
		);
		let lowered = must(
			lower_stage(
				gather.0,
				&build(gather.0, gather.1),
				&targets()[1],
				&LoweringOptions {
					entry_symbol: "recipe_checked_gather".to_owned(),
					workgroup_lanes: gather.1.geometry.workgroup_lanes,
				},
			),
			"gather lowering",
		);
		let guard = must_some(lowered.llvm_ir.find("gather_valid_index"), "index guard");
		let payload = must_some(
			lowered.llvm_ir.find("gather_value_address"),
			"payload address",
		);
		assert!(guard < payload);
		assert!(lowered.llvm_ir.contains("atomicrmw xchg"));

		let random = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| matches!(stage.kind, StageKind::Philox4x32_10(_))),
			"random stage",
		);
		let lowered = must(
			lower_stage(
				random.0,
				&build(random.0, random.1),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_random".to_owned(),
					workgroup_lanes: random.1.geometry.workgroup_lanes,
				},
			),
			"random lowering",
		);
		assert!(lowered.abi.arguments.contains(&KernelArgument::RunId));
		assert!(
			lowered
				.abi
				.arguments
				.contains(&KernelArgument::LoopIteration)
		);
		assert!(lowered.llvm_ir.contains("recipe_philox4x32_10_v1"));
		assert!(lowered.llvm_ir.contains("xor i64 %loop_iteration"));
		assert!(lowered.llvm_ir.contains("xor i32 %philox_run_low"));
		assert!(!lowered.llvm_ir.contains("curand"));
	}

	#[test]
	fn emits_backend_safe_plain_fdiv_for_amd_normal_initialization() {
		let programs = programs();
		let normal = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| {
					matches!(
						stage.kind,
						StageKind::Philox4x32_10(contract)
							if contract.distribution == RandomDistribution::NormalF32
					)
				}),
			"normal random stage",
		);
		let lowered = must(
			lower_stage(
				normal.0,
				&build(normal.0, normal.1),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_normal_f32".to_owned(),
					workgroup_lanes: normal.1.geometry.workgroup_lanes,
				},
			),
			"normal random lowering",
		);
		assert!(lowered.llvm_ir.contains(" = fdiv float "));
		assert!(!lowered.llvm_ir.contains("constrained.fdiv"));
		assert!(lowered.llvm_ir.contains("owned_log_y_canonical"));
		assert!(lowered.llvm_ir.contains("constrained.fmul"));
		assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
	}

	#[test]
	fn emits_backend_safe_sqrt_intrinsic_for_amd_normal_initialization() {
		let programs = programs();
		let normal = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| {
					matches!(
						stage.kind,
						StageKind::Philox4x32_10(contract)
							if contract.distribution == RandomDistribution::NormalF32
					)
				}),
			"normal random stage",
		);
		let lowered = must(
			lower_stage(
				normal.0,
				&build(normal.0, normal.1),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_normal_f32".to_owned(),
					workgroup_lanes: normal.1.geometry.workgroup_lanes,
				},
			),
			"normal random lowering",
		);
		assert!(
			lowered
				.llvm_ir
				.contains("declare float @llvm.sqrt.f32(float)")
		);
		assert!(lowered.llvm_ir.contains("call float @llvm.sqrt.f32(float "));
		assert!(!lowered.llvm_ir.contains("constrained.sqrt"));
		assert!(lowered.llvm_ir.contains("normal_radial_canonical"));
		assert!(lowered.llvm_ir.contains("constrained.fmul"));
		assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
	}

	#[test]
	#[ignore = "requires the host LLVM AMDGPU backend"]
	fn host_llc_compiles_amd_normal_initialization_with_backend_safe_divide_and_sqrt() {
		let programs = programs();
		let normal = must_some(
			programs
				.iter()
				.flat_map(|program| program.stages.iter().map(move |stage| (program, stage)))
				.find(|(_, stage)| {
					matches!(
						stage.kind,
						StageKind::Philox4x32_10(contract)
							if contract.distribution == RandomDistribution::NormalF32
					)
				}),
			"normal random stage",
		);
		let lowered = must(
			lower_stage(
				normal.0,
				&build(normal.0, normal.1),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_normal_f32".to_owned(),
					workgroup_lanes: normal.1.geometry.workgroup_lanes,
				},
			),
			"normal random lowering",
		);
		let mut child = Command::new("/usr/bin/llc")
			.args([
				"-filetype=null",
				"-march=amdgcn",
				"-mcpu=gfx1101",
				"--amdhsa-code-object-version=6",
				"-",
			])
			.stdin(Stdio::piped())
			.stdout(Stdio::null())
			.stderr(Stdio::piped())
			.spawn()
			.expect("spawn host llc");
		{
			let mut stdin = child.stdin.take().expect("host llc stdin");
			stdin.write_all(lowered.llvm_ir.as_bytes())
				.expect("write LLVM IR");
		}
		let output = child.wait_with_output().expect("wait for host llc");
		assert!(
			output.status.success(),
			"host llc rejected AMD normal initialization:\n{}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	#[test]
	fn index_map_checks_affine_arithmetic_before_forming_output_addresses() {
		let program = lower_program(
			primitive(
				200,
				&[],
				&[200],
				PrimitiveKind::IndexMap(IndexMap {
					start: i32::MIN,
					element_step: -1,
					iteration_step: i32::MAX,
					modulus: Some(97),
				}),
			),
			&[tensor(200, DType::I32, &[32])],
		);
		let stage = &program.stages[0];
		let lowered = must(
			lower_stage(
				&program,
				&build(&program, stage),
				&targets()[1],
				&LoweringOptions {
					entry_symbol: "recipe_index_map".to_owned(),
					workgroup_lanes: stage.geometry.workgroup_lanes,
				},
			),
			"index-map lowering",
		);
		assert!(
			lowered
				.abi
				.arguments
				.contains(&KernelArgument::LoopIteration)
		);
		assert!(lowered.llvm_ir.contains("@llvm.smul.with.overflow.i64"));
		assert!(lowered.llvm_ir.contains("@llvm.sadd.with.overflow.i64"));
		assert!(lowered.llvm_ir.contains("srem i64"));
		let fault = must_some(
			lowered.llvm_ir.find("atomicrmw xchg"),
			"index-map fault publication",
		);
		let address = must_some(
			lowered.llvm_ir.find("index_map_output_address"),
			"index-map output address",
		);
		assert!(fault < address);

		let invariant = lower_program(
			primitive(
				201,
				&[],
				&[201],
				PrimitiveKind::IndexMap(IndexMap {
					start: 7,
					element_step: 1,
					iteration_step: 0,
					modulus: None,
				}),
			),
			&[tensor(201, DType::I32, &[32])],
		);
		let invariant_stage = &invariant.stages[0];
		let invariant_lowered = must(
			lower_stage(
				&invariant,
				&build(&invariant, invariant_stage),
				&targets()[0],
				&LoweringOptions {
					entry_symbol: "recipe_index_map_invariant".to_owned(),
					workgroup_lanes: invariant_stage.geometry.workgroup_lanes,
				},
			),
			"iteration-invariant index-map lowering",
		);
		assert!(
			!invariant_lowered
				.abi
				.arguments
				.contains(&KernelArgument::LoopIteration)
		);
		assert!(!invariant_lowered.llvm_ir.contains("%loop_iteration"));
	}
}

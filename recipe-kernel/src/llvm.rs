use std::collections::BTreeMap;
use std::fmt::Write;

use recipe_core::{
	DType, ElementCount, FlopCount, IndexSpace, KernelTemplate, ScalarInstruction, ScalarLiteral, ScalarOpcode,
	ScalarValueId, StaticBufferAccess,
};

use crate::{KernelTarget, LoweringError, LoweringErrorKind, audit_llvm_ir};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferAccess {
	Read,
	Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelArgument {
	Buffer {
		access: BufferAccess,
		dtype: DType,
		alignment: u32,
	},
	/// Preallocated global int32 flag atomically ORed with one when a checked
	/// scalar operation rejects an input.
	FaultFlag,
	/// Dynamic per-execution identity used by Recipe-owned counter-based
	/// kernels. This is an explicit 64-bit by-value launch argument; it may not
	/// be folded into an AOT artifact.
	RunId,
	/// Zero-based static-loop iteration selected by the executor. This is an
	/// explicit unsigned 64-bit by-value launch argument and is present only
	/// for iteration-dependent kernels.
	LoopIteration,
	ElementCount,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelAbi {
	pub entry_symbol: String,
	pub arguments: Vec<KernelArgument>,
	pub argument_bytes: u32,
	pub argument_alignment: u32,
	pub elements: ElementCount,
	pub workgroup_lanes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweringOptions {
	pub entry_symbol: String,
	pub workgroup_lanes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoweredKernel {
	pub llvm_ir: String,
	pub abi: KernelAbi,
	pub work: FlopCount,
	/// Exact backend target for which this module was emitted.
	///
	/// Address spaces, intrinsics, calling convention, and floating-point
	/// attributes make lowered LLVM IR target-specific.
	pub target: KernelTarget,
}

#[derive(Clone, Debug)]
struct ValueRef {
	dtype: DType,
	operand: String,
}

struct Emitter {
	body: String,
	next_temporary: u64,
	values: BTreeMap<ScalarValueId, ValueRef>,
	uses_f32_arithmetic: bool,
	uses_fma: bool,
	uses_minimum: bool,
	uses_maximum: bool,
	uses_sat_conversion: bool,
	uses_square_root: bool,
	uses_floor: bool,
	uses_ceiling: bool,
	uses_round_nearest_even: bool,
	fault_conditions: Vec<String>,
}

impl Emitter {
	fn new() -> Self {
		Self {
			body: String::new(),
			next_temporary: 0,
			values: BTreeMap::new(),
			uses_f32_arithmetic: false,
			uses_fma: false,
			uses_minimum: false,
			uses_maximum: false,
			uses_sat_conversion: false,
			uses_square_root: false,
			uses_floor: false,
			uses_ceiling: false,
			uses_round_nearest_even: false,
			fault_conditions: Vec::new(),
		}
	}

	fn temporary(&mut self, purpose: &str) -> String {
		let id = self.next_temporary;
		self.next_temporary += 1;
		format!("%{purpose}_{id}")
	}

	fn line(&mut self, arguments: std::fmt::Arguments<'_>) {
		self.body.write_fmt(arguments).expect("writing to String");
		self.body.push('\n');
	}

	fn value(&self, id: ScalarValueId) -> Result<&ValueRef, LoweringError> {
		self.values.get(&id).ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::UnknownScalarValue,
				format!("scalar value {id} is unavailable during lowering"),
			)
			.for_scalar(id)
		})
	}

	fn canonicalize_nan(&mut self, raw: String) -> String {
		let unordered = self.temporary("is_nan");
		self.line(format_args!("  {unordered} = fcmp uno float {raw}, {raw}"));
		let canonical = self.temporary("canonicalized");
		self.line(format_args!(
			"  {canonical} = select i1 {unordered}, float %canonical_nan, float {raw}"
		));
		canonical
	}

	fn reject_when(&mut self, condition: String) {
		self.fault_conditions.push(condition);
	}

	fn emit_fault_flag(&mut self) {
		let Some(first) = self.fault_conditions.first().cloned() else {
			return;
		};
		let mut rejected = first;
		for condition in self
			.fault_conditions
			.iter()
			.skip(1)
			.cloned()
			.collect::<Vec<_>>()
		{
			let combined = self.temporary("rejected");
			self.line(format_args!("  {combined} = or i1 {rejected}, {condition}"));
			rejected = combined;
		}
		let label_id = self.next_temporary;
		self.next_temporary += 1;
		let rejected_label = format!("fault_rejected_{label_id}");
		let continue_label = format!("fault_continue_{label_id}");
		self.line(format_args!(
			"  br i1 {rejected}, label %{rejected_label}, label %{continue_label}"
		));
		self.line(format_args!("{rejected_label}:"));
		let previous = self.temporary("fault_previous");
		self.line(format_args!(
			"  {previous} = atomicrmw or ptr addrspace(1) %fault_flag, i32 1 monotonic, align 4"
		));
		self.line(format_args!("  br label %{continue_label}"));
		self.line(format_args!("{continue_label}:"));
	}
}

/// Lower an elementwise kernel template to direct AMDGPU or NVPTX LLVM IR.
///
/// The emitted entry point has one global pointer per input and output followed
/// by a 64-bit element count. Every lane processes at most one linear element.
pub fn lower_elementwise(
	template: &KernelTemplate,
	target: &KernelTarget,
	options: &LoweringOptions,
) -> Result<LoweredKernel, LoweringError> {
	template
		.validate()
		.map_err(|errors| LoweringError::new(LoweringErrorKind::InvalidKernel, errors.to_string()))?;
	target.validate()?;
	validate_options(options)?;

	let mut emitter = Emitter::new();
	emit_global_index(&mut emitter, target, options.workgroup_lanes);
	emitter.line(format_args!(
		"  %in_bounds = icmp ult i64 %global_id, %element_count"
	));
	emitter.line(format_args!("  br i1 %in_bounds, label %body, label %exit"));
	emitter.line(format_args!("body:"));
	emitter.line(format_args!(
		"  %canonical_nan = bitcast i32 2143289344 to float"
	));

	for (index, (kernel_input, scalar_input)) in template
		.inputs
		.iter()
		.zip(&template.program.inputs)
		.enumerate()
	{
		let element_index = emit_buffer_index(
			&mut emitter,
			&template.index_space,
			&kernel_input.access,
			&format!("input_{index}"),
		);
		let pointer = emitter.temporary("input_element");
		emitter.line(format_args!(
			"  {pointer} = getelementptr inbounds {}, ptr addrspace(1) %input_{index}, i64 {element_index}",
			llvm_type(kernel_input.dtype),
		));
		let loaded = emitter.temporary("loaded");
		emitter.line(format_args!(
			"  {loaded} = load {}, ptr addrspace(1) {pointer}, align 4",
			llvm_type(kernel_input.dtype)
		));
		emitter.values.insert(
			scalar_input.id,
			ValueRef {
				dtype: scalar_input.dtype,
				operand: loaded,
			},
		);
	}
	for constant in &template.program.constants {
		let operand = match constant.value {
			ScalarLiteral::I32(value) => value.to_string(),
			ScalarLiteral::F32Bits(bits) => {
				let value = emitter.temporary("constant");
				emitter.line(format_args!("  {value} = bitcast i32 {bits} to float"));
				value
			}
		};
		emitter.values.insert(
			constant.id,
			ValueRef {
				dtype: constant.value.dtype(),
				operand,
			},
		);
	}

	let mut flops_per_element = 0u64;
	for instruction in &template.program.instructions {
		let result = lower_instruction(&mut emitter, instruction)?;
		flops_per_element = flops_per_element
			.checked_add(instruction_flops(instruction.opcode))
			.ok_or_else(|| {
				LoweringError::new(
					LoweringErrorKind::ArithmeticOverflow,
					"per-element FLOP count overflowed",
				)
			})?;
		emitter.values.insert(
			instruction.result,
			ValueRef {
				dtype: instruction.dtype,
				operand: result,
			},
		);
	}
	emitter.emit_fault_flag();

	for (index, (kernel_output, scalar_output)) in template
		.outputs
		.iter()
		.zip(&template.program.outputs)
		.enumerate()
	{
		let value = emitter.value(*scalar_output)?.clone();
		if value.dtype != kernel_output.dtype {
			return Err(LoweringError::new(
				LoweringErrorKind::InvalidKernel,
				"validated output type changed during lowering",
			)
			.for_scalar(*scalar_output));
		}
		let element_index = emit_buffer_index(
			&mut emitter,
			&template.index_space,
			&kernel_output.access,
			&format!("output_{index}"),
		);
		let pointer = emitter.temporary("output_element");
		emitter.line(format_args!(
			"  {pointer} = getelementptr inbounds {}, ptr addrspace(1) %output_{index}, i64 {element_index}",
			llvm_type(kernel_output.dtype),
		));
		emitter.line(format_args!(
			"  store {} {}, ptr addrspace(1) {pointer}, align 4",
			llvm_type(kernel_output.dtype),
			value.operand
		));
	}
	emitter.line(format_args!("  br label %exit"));
	emitter.line(format_args!("exit:"));
	emitter.line(format_args!("  ret void"));

	let llvm_ir = assemble_module(target, options, template, &emitter);
	let findings = audit_llvm_ir(&llvm_ir);
	if let Some(finding) = findings.first() {
		return Err(LoweringError::new(
			LoweringErrorKind::ProhibitedInterface,
			format!(
				"generated LLVM IR failed audit at line {}: {:?} `{}`",
				finding.line, finding.kind, finding.token
			),
		));
	}
	let mut pointer_arguments = template
		.inputs
		.len()
		.checked_add(template.outputs.len())
		.ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				"kernel argument count overflowed",
			)
		})?;
	if !emitter.fault_conditions.is_empty() {
		pointer_arguments = pointer_arguments.checked_add(1).ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				"kernel fault argument count overflowed",
			)
		})?;
	}
	let argument_bytes = u32::try_from(pointer_arguments)
		.ok()
		.and_then(|count| count.checked_add(1))
		.and_then(|count| count.checked_mul(8))
		.ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				"kernel ABI size overflowed",
			)
		})?;
	let mut arguments = template
		.inputs
		.iter()
		.map(|input| KernelArgument::Buffer {
			access: BufferAccess::Read,
			dtype: input.dtype,
			alignment: 4,
		})
		.collect::<Vec<_>>();
	arguments.extend(
		template
			.outputs
			.iter()
			.map(|output| KernelArgument::Buffer {
				access: BufferAccess::Write,
				dtype: output.dtype,
				alignment: 4,
			}),
	);
	if !emitter.fault_conditions.is_empty() {
		arguments.push(KernelArgument::FaultFlag);
	}
	arguments.push(KernelArgument::ElementCount);
	let work = FlopCount::new(flops_per_element)
		.checked_mul(template.index_space.elements().get())
		.ok_or_else(|| {
			LoweringError::new(
				LoweringErrorKind::ArithmeticOverflow,
				"kernel FLOP count overflowed",
			)
		})?;
	Ok(LoweredKernel {
		llvm_ir,
		abi: KernelAbi {
			entry_symbol: options.entry_symbol.clone(),
			arguments,
			argument_bytes,
			argument_alignment: 8,
			elements: template.index_space.elements(),
			workgroup_lanes: options.workgroup_lanes,
		},
		work,
		target: target.clone(),
	})
}

fn emit_buffer_index(
	emitter: &mut Emitter,
	index_space: &IndexSpace,
	access: &StaticBufferAccess,
	label: &str,
) -> String {
	let mut logical_stride = index_space.elements().get();
	let mut accumulated = if access.offset_elements == 0 {
		None
	} else {
		Some(access.offset_elements.to_string())
	};

	for (axis, (extent, physical_stride)) in index_space
		.dimensions()
		.iter()
		.zip(&access.strides)
		.enumerate()
	{
		logical_stride /= extent.get();
		if extent.get() == 1 || *physical_stride == 0 {
			continue;
		}
		let quotient = if logical_stride == 1 {
			"%global_id".to_owned()
		} else {
			let quotient = emitter.temporary(&format!("{label}_axis_{axis}_quotient"));
			emitter.line(format_args!(
				"  {quotient} = udiv i64 %global_id, {logical_stride}"
			));
			quotient
		};
		let coordinate = if logical_stride
			.checked_mul(extent.get())
			.is_some_and(|leading_span| leading_span == index_space.elements().get())
		{
			quotient
		} else {
			let coordinate = emitter.temporary(&format!("{label}_axis_{axis}_coordinate"));
			emitter.line(format_args!(
				"  {coordinate} = urem i64 {quotient}, {}",
				extent.get()
			));
			coordinate
		};
		let contribution = if *physical_stride == 1 {
			coordinate
		} else {
			let contribution = emitter.temporary(&format!("{label}_axis_{axis}_offset"));
			emitter.line(format_args!(
				"  {contribution} = mul i64 {coordinate}, {physical_stride}"
			));
			contribution
		};
		accumulated = Some(if let Some(previous) = accumulated {
			let combined = emitter.temporary(&format!("{label}_element_index"));
			emitter.line(format_args!(
				"  {combined} = add i64 {previous}, {contribution}"
			));
			combined
		} else {
			contribution
		});
	}

	accumulated.unwrap_or_else(|| "0".to_owned())
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
	if !valid_symbol {
		return Err(LoweringError::new(
			LoweringErrorKind::InvalidEntrySymbol,
			"entry symbol must be an ASCII LLVM identifier without punctuation",
		));
	}
	if !(1..=1024).contains(&options.workgroup_lanes) {
		return Err(LoweringError::new(
			LoweringErrorKind::InvalidWorkgroupSize,
			"workgroup size must be between 1 and 1024 lanes",
		));
	}
	Ok(())
}

fn emit_global_index(emitter: &mut Emitter, target: &KernelTarget, workgroup_lanes: u32) {
	match target {
		KernelTarget::Amd(_) => {
			emitter.line(format_args!(
				"  %local_id_32 = call i32 @llvm.amdgcn.workitem.id.x()"
			));
			emitter.line(format_args!(
				"  %group_id_32 = call i32 @llvm.amdgcn.workgroup.id.x()"
			));
		}
		KernelTarget::Nvidia(_) => {
			emitter.line(format_args!(
				"  %local_id_32 = call i32 @llvm.nvvm.read.ptx.sreg.tid.x()"
			));
			emitter.line(format_args!(
				"  %group_id_32 = call i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()"
			));
		}
	}
	emitter.line(format_args!("  %local_id = zext i32 %local_id_32 to i64"));
	emitter.line(format_args!("  %group_id = zext i32 %group_id_32 to i64"));
	emitter.line(format_args!(
		"  %group_base = mul i64 %group_id, {workgroup_lanes}"
	));
	emitter.line(format_args!(
		"  %global_id = add i64 %group_base, %local_id"
	));
}

fn lower_instruction(emitter: &mut Emitter, instruction: &ScalarInstruction) -> Result<String, LoweringError> {
	let operands = instruction
		.operands
		.iter()
		.map(|id| emitter.value(*id).cloned())
		.collect::<Result<Vec<_>, _>>()?;
	let result = match (instruction.opcode, instruction.dtype) {
		(ScalarOpcode::Add, DType::F32) => constrained_binary(emitter, "fadd", &operands),
		(ScalarOpcode::Subtract, DType::F32) => constrained_binary(emitter, "fsub", &operands),
		(ScalarOpcode::Multiply, DType::F32) => constrained_binary(emitter, "fmul", &operands),
		(ScalarOpcode::Divide, DType::F32) => plain_fdiv(emitter, &operands),
		(ScalarOpcode::Remainder, DType::F32) => constrained_binary(emitter, "frem", &operands),
		(ScalarOpcode::Add, DType::I32) => integer_binary(emitter, "add", &operands),
		(ScalarOpcode::Subtract, DType::I32) => integer_binary(emitter, "sub", &operands),
		(ScalarOpcode::Multiply, DType::I32) => integer_binary(emitter, "mul", &operands),
		(ScalarOpcode::Divide, DType::I32) => checked_integer_divrem(emitter, "sdiv", &operands),
		(ScalarOpcode::Remainder, DType::I32) => checked_integer_divrem(emitter, "srem", &operands),
		(ScalarOpcode::Negate, DType::F32) => float_bit_unary(emitter, "xor", 0x8000_0000, &operands),
		(ScalarOpcode::Absolute, DType::F32) => float_bit_unary(emitter, "and", 0x7fff_ffff, &operands),
		(ScalarOpcode::Negate, DType::I32) => checked_integer_negate(emitter, &operands),
		(ScalarOpcode::Absolute, DType::I32) => checked_integer_absolute(emitter, &operands),
		(ScalarOpcode::Minimum, DType::F32) => float_intrinsic(emitter, "minimum", &operands),
		(ScalarOpcode::Maximum, DType::F32) => float_intrinsic(emitter, "maximum", &operands),
		(ScalarOpcode::Minimum, DType::I32) => integer_minmax(emitter, "slt", &operands),
		(ScalarOpcode::Maximum, DType::I32) => integer_minmax(emitter, "sgt", &operands),
		(ScalarOpcode::Fma, DType::F32) => constrained_fma(emitter, &operands),
		(ScalarOpcode::Fma, DType::I32) => {
			return Err(LoweringError::new(
				LoweringErrorKind::UnsupportedOperation,
				"Fma is defined only for f32",
			)
			.for_scalar(instruction.result));
		}
		(
			ScalarOpcode::Equal
			| ScalarOpcode::NotEqual
			| ScalarOpcode::LessThan
			| ScalarOpcode::LessThanOrEqual
			| ScalarOpcode::GreaterThan
			| ScalarOpcode::GreaterThanOrEqual,
			DType::I32,
		) => comparison(emitter, instruction.opcode, &operands),
		(ScalarOpcode::Select, dtype) => select(emitter, dtype, &operands),
		(ScalarOpcode::BitAnd, DType::I32) => integer_binary(emitter, "and", &operands),
		(ScalarOpcode::BitOr, DType::I32) => integer_binary(emitter, "or", &operands),
		(ScalarOpcode::BitXor, DType::I32) => integer_binary(emitter, "xor", &operands),
		(ScalarOpcode::BitNot, DType::I32) => integer_bit_not(emitter, &operands),
		(ScalarOpcode::BitcastF32ToI32, DType::I32) => bitcast(emitter, DType::F32, DType::I32, &operands),
		(ScalarOpcode::BitcastI32ToF32, DType::F32) => bitcast(emitter, DType::I32, DType::F32, &operands),
		(ScalarOpcode::ShiftLeft, DType::I32) => shift(emitter, "shl", &operands),
		(ScalarOpcode::ShiftRightLogical, DType::I32) => shift(emitter, "lshr", &operands),
		(ScalarOpcode::ShiftRightArithmetic, DType::I32) => shift(emitter, "ashr", &operands),
		(
			ScalarOpcode::BitAnd
			| ScalarOpcode::BitOr
			| ScalarOpcode::BitXor
			| ScalarOpcode::BitNot
			| ScalarOpcode::ShiftLeft
			| ScalarOpcode::ShiftRightLogical
			| ScalarOpcode::ShiftRightArithmetic,
			DType::F32,
		) => {
			return Err(LoweringError::new(
				LoweringErrorKind::UnsupportedOperation,
				"bitwise and shift operations require int32",
			)
			.for_scalar(instruction.result));
		}
		(ScalarOpcode::Require, DType::I32) => require(emitter, &operands),
		(ScalarOpcode::IsFinite, DType::I32) => classify_float(emitter, false, &operands),
		(ScalarOpcode::IsNan, DType::I32) => classify_float(emitter, true, &operands),
		(ScalarOpcode::SquareRoot, DType::F32) => square_root(emitter, &operands),
		(ScalarOpcode::Floor, DType::F32) => unary_float_intrinsic(emitter, "floor", &operands),
		(ScalarOpcode::Ceiling, DType::F32) => unary_float_intrinsic(emitter, "ceil", &operands),
		(ScalarOpcode::RoundNearestEven, DType::F32) => unary_float_intrinsic(emitter, "roundeven", &operands),
		(ScalarOpcode::ConvertF32ToI32, DType::I32) => {
			emitter.uses_sat_conversion = true;
			let converted = emitter.temporary("converted");
			emitter.line(format_args!(
				"  {converted} = call i32 @llvm.fptosi.sat.i32.f32(float {})",
				operands[0].operand
			));
			converted
		}
		(ScalarOpcode::ConvertI32ToF32, DType::F32) => {
			let converted = emitter.temporary("converted");
			emitter.line(format_args!(
				"  {converted} = sitofp i32 {} to float",
				operands[0].operand
			));
			converted
		}
		(ScalarOpcode::ConvertF32ToI32 | ScalarOpcode::ConvertI32ToF32, _) => {
			return Err(LoweringError::new(
				LoweringErrorKind::UnsupportedOperation,
				"conversion result type does not match the opcode",
			)
			.for_scalar(instruction.result));
		}
		_ => {
			return Err(LoweringError::new(
				LoweringErrorKind::UnsupportedOperation,
				"scalar opcode is newer than this lowering implementation",
			)
			.for_scalar(instruction.result));
		}
	};
	Ok(result)
}

fn constrained_binary(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	emitter.uses_f32_arithmetic = true;
	let raw = emitter.temporary("fp");
	emitter.line(format_args!(
		"  {raw} = call float @llvm.experimental.constrained.{operation}.f32(float {}, float {}, metadata !\"round.tonearest\", metadata !\"fpexcept.ignore\")",
		operands[0].operand, operands[1].operand
	));
	emitter.canonicalize_nan(raw)
}

fn plain_fdiv(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	let raw = emitter.temporary("fp");
	emitter.line(format_args!(
		"  {raw} = fdiv float {}, {}",
		operands[0].operand, operands[1].operand
	));
	emitter.canonicalize_nan(raw)
}

fn constrained_fma(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	emitter.uses_fma = true;
	let raw = emitter.temporary("fma");
	emitter.line(format_args!(
		"  {raw} = call float @llvm.experimental.constrained.fma.f32(float {}, float {}, float {}, metadata !\"round.tonearest\", metadata !\"fpexcept.ignore\")",
		operands[0].operand, operands[1].operand, operands[2].operand
	));
	emitter.canonicalize_nan(raw)
}

fn integer_binary(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	let result = emitter.temporary("integer");
	emitter.line(format_args!(
		"  {result} = {operation} i32 {}, {}",
		operands[0].operand, operands[1].operand
	));
	result
}

fn checked_integer_divrem(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	let zero = emitter.temporary("divisor_zero");
	emitter.line(format_args!(
		"  {zero} = icmp eq i32 {}, 0",
		operands[1].operand
	));
	let minimum = emitter.temporary("dividend_minimum");
	emitter.line(format_args!(
		"  {minimum} = icmp eq i32 {}, -2147483648",
		operands[0].operand
	));
	let negative_one = emitter.temporary("divisor_negative_one");
	emitter.line(format_args!(
		"  {negative_one} = icmp eq i32 {}, -1",
		operands[1].operand
	));
	let overflow = emitter.temporary("division_overflow");
	emitter.line(format_args!(
		"  {overflow} = and i1 {minimum}, {negative_one}"
	));
	let rejected = emitter.temporary("division_rejected");
	emitter.line(format_args!("  {rejected} = or i1 {zero}, {overflow}"));
	let divisor = emitter.temporary("safe_divisor");
	emitter.line(format_args!(
		"  {divisor} = select i1 {rejected}, i32 1, i32 {}",
		operands[1].operand
	));
	let raw = emitter.temporary("division");
	emitter.line(format_args!(
		"  {raw} = {operation} i32 {}, {divisor}",
		operands[0].operand
	));
	let result = emitter.temporary("checked_division");
	emitter.line(format_args!(
		"  {result} = select i1 {rejected}, i32 0, i32 {raw}"
	));
	emitter.reject_when(rejected);
	result
}

fn checked_integer_negate(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	let rejected = emitter.temporary("negate_rejected");
	emitter.line(format_args!(
		"  {rejected} = icmp eq i32 {}, -2147483648",
		operands[0].operand
	));
	let operand = emitter.temporary("safe_negate_operand");
	emitter.line(format_args!(
		"  {operand} = select i1 {rejected}, i32 0, i32 {}",
		operands[0].operand
	));
	let result = emitter.temporary("negated");
	emitter.line(format_args!("  {result} = sub i32 0, {operand}"));
	emitter.reject_when(rejected);
	result
}

fn checked_integer_absolute(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	let rejected = emitter.temporary("absolute_rejected");
	emitter.line(format_args!(
		"  {rejected} = icmp eq i32 {}, -2147483648",
		operands[0].operand
	));
	let operand = emitter.temporary("safe_absolute_operand");
	emitter.line(format_args!(
		"  {operand} = select i1 {rejected}, i32 0, i32 {}",
		operands[0].operand
	));
	let negative = emitter.temporary("absolute_negative");
	emitter.line(format_args!("  {negative} = icmp slt i32 {operand}, 0"));
	let negated = emitter.temporary("absolute_negated");
	emitter.line(format_args!("  {negated} = sub i32 0, {operand}"));
	let result = emitter.temporary("absolute");
	emitter.line(format_args!(
		"  {result} = select i1 {negative}, i32 {negated}, i32 {operand}"
	));
	emitter.reject_when(rejected);
	result
}

fn float_bit_unary(emitter: &mut Emitter, operation: &str, mask: u32, operands: &[ValueRef]) -> String {
	let bits = emitter.temporary("float_bits");
	emitter.line(format_args!(
		"  {bits} = bitcast float {} to i32",
		operands[0].operand
	));
	let changed = emitter.temporary("changed_bits");
	emitter.line(format_args!("  {changed} = {operation} i32 {bits}, {mask}"));
	let result = emitter.temporary("float");
	emitter.line(format_args!("  {result} = bitcast i32 {changed} to float"));
	result
}

fn float_intrinsic(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	match operation {
		"minimum" => emitter.uses_minimum = true,
		"maximum" => emitter.uses_maximum = true,
		_ => unreachable!("known float intrinsic"),
	}
	let raw = emitter.temporary(operation);
	emitter.line(format_args!(
		"  {raw} = call float @llvm.{operation}.f32(float {}, float {})",
		operands[0].operand, operands[1].operand
	));
	emitter.canonicalize_nan(raw)
}

fn comparison(emitter: &mut Emitter, opcode: ScalarOpcode, operands: &[ValueRef]) -> String {
	let predicate = match (opcode, operands[0].dtype) {
		(ScalarOpcode::Equal, DType::I32) => "eq",
		(ScalarOpcode::NotEqual, DType::I32) => "ne",
		(ScalarOpcode::LessThan, DType::I32) => "slt",
		(ScalarOpcode::LessThanOrEqual, DType::I32) => "sle",
		(ScalarOpcode::GreaterThan, DType::I32) => "sgt",
		(ScalarOpcode::GreaterThanOrEqual, DType::I32) => "sge",
		(ScalarOpcode::Equal, DType::F32) => "oeq",
		(ScalarOpcode::NotEqual, DType::F32) => "une",
		(ScalarOpcode::LessThan, DType::F32) => "olt",
		(ScalarOpcode::LessThanOrEqual, DType::F32) => "ole",
		(ScalarOpcode::GreaterThan, DType::F32) => "ogt",
		(ScalarOpcode::GreaterThanOrEqual, DType::F32) => "oge",
		_ => unreachable!("validated comparison opcode"),
	};
	let condition = emitter.temporary("comparison");
	match operands[0].dtype {
		DType::I32 => emitter.line(format_args!(
			"  {condition} = icmp {predicate} i32 {}, {}",
			operands[0].operand, operands[1].operand
		)),
		DType::F32 => emitter.line(format_args!(
			"  {condition} = fcmp {predicate} float {}, {}",
			operands[0].operand, operands[1].operand
		)),
	}
	let result = emitter.temporary("comparison_value");
	emitter.line(format_args!("  {result} = zext i1 {condition} to i32"));
	result
}

fn select(emitter: &mut Emitter, dtype: DType, operands: &[ValueRef]) -> String {
	let condition = emitter.temporary("select_condition");
	emitter.line(format_args!(
		"  {condition} = icmp ne i32 {}, 0",
		operands[0].operand
	));
	let result = emitter.temporary("selected");
	emitter.line(format_args!(
		"  {result} = select i1 {condition}, {} {}, {} {}",
		llvm_type(dtype),
		operands[1].operand,
		llvm_type(dtype),
		operands[2].operand
	));
	result
}

fn integer_bit_not(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	let result = emitter.temporary("bit_not");
	emitter.line(format_args!(
		"  {result} = xor i32 {}, -1",
		operands[0].operand
	));
	result
}

fn bitcast(emitter: &mut Emitter, from: DType, to: DType, operands: &[ValueRef]) -> String {
	let result = emitter.temporary("bitcast");
	emitter.line(format_args!(
		"  {result} = bitcast {} {} to {}",
		llvm_type(from),
		operands[0].operand,
		llvm_type(to)
	));
	result
}

fn require(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	let accepted = emitter.temporary("require_accepted");
	emitter.line(format_args!(
		"  {accepted} = icmp ne i32 {}, 0",
		operands[0].operand
	));
	let rejected = emitter.temporary("require_rejected");
	emitter.line(format_args!("  {rejected} = xor i1 {accepted}, true"));
	let result = emitter.temporary("require_value");
	emitter.line(format_args!("  {result} = zext i1 {accepted} to i32"));
	emitter.reject_when(rejected);
	result
}

fn classify_float(emitter: &mut Emitter, nan: bool, operands: &[ValueRef]) -> String {
	let bits = emitter.temporary("classification_bits");
	emitter.line(format_args!(
		"  {bits} = bitcast float {} to i32",
		operands[0].operand
	));
	let exponent = emitter.temporary("classification_exponent");
	emitter.line(format_args!("  {exponent} = and i32 {bits}, 2139095040"));
	let exponent_all_ones = emitter.temporary("classification_special");
	emitter.line(format_args!(
		"  {exponent_all_ones} = icmp eq i32 {exponent}, 2139095040"
	));
	let condition = if nan {
		let mantissa = emitter.temporary("classification_mantissa");
		emitter.line(format_args!("  {mantissa} = and i32 {bits}, 8388607"));
		let mantissa_nonzero = emitter.temporary("classification_mantissa_nonzero");
		emitter.line(format_args!(
			"  {mantissa_nonzero} = icmp ne i32 {mantissa}, 0"
		));
		let condition = emitter.temporary("classification_nan");
		emitter.line(format_args!(
			"  {condition} = and i1 {exponent_all_ones}, {mantissa_nonzero}"
		));
		condition
	} else {
		let condition = emitter.temporary("classification_finite");
		emitter.line(format_args!(
			"  {condition} = xor i1 {exponent_all_ones}, true"
		));
		condition
	};
	let result = emitter.temporary("classification_value");
	emitter.line(format_args!("  {result} = zext i1 {condition} to i32"));
	result
}

fn square_root(emitter: &mut Emitter, operands: &[ValueRef]) -> String {
	emitter.uses_square_root = true;
	let raw = emitter.temporary("sqrt");
	emitter.line(format_args!(
		"  {raw} = call float @llvm.sqrt.f32(float {})",
		operands[0].operand
	));
	emitter.canonicalize_nan(raw)
}

fn unary_float_intrinsic(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	match operation {
		"floor" => emitter.uses_floor = true,
		"ceil" => emitter.uses_ceiling = true,
		"roundeven" => emitter.uses_round_nearest_even = true,
		_ => unreachable!("known unary float intrinsic"),
	}
	let raw = emitter.temporary(operation);
	emitter.line(format_args!(
		"  {raw} = call float @llvm.{operation}.f32(float {})",
		operands[0].operand
	));
	emitter.canonicalize_nan(raw)
}

fn integer_minmax(emitter: &mut Emitter, predicate: &str, operands: &[ValueRef]) -> String {
	let condition = emitter.temporary("comparison");
	emitter.line(format_args!(
		"  {condition} = icmp {predicate} i32 {}, {}",
		operands[0].operand, operands[1].operand
	));
	let result = emitter.temporary("selected");
	emitter.line(format_args!(
		"  {result} = select i1 {condition}, i32 {}, i32 {}",
		operands[0].operand, operands[1].operand
	));
	result
}

fn shift(emitter: &mut Emitter, operation: &str, operands: &[ValueRef]) -> String {
	let count = emitter.temporary("shift_count");
	emitter.line(format_args!(
		"  {count} = and i32 {}, 31",
		operands[1].operand
	));
	let result = emitter.temporary("shifted");
	emitter.line(format_args!(
		"  {result} = {operation} i32 {}, {count}",
		operands[0].operand
	));
	result
}

fn assemble_module(
	target: &KernelTarget,
	options: &LoweringOptions,
	template: &KernelTemplate,
	emitter: &Emitter,
) -> String {
	let mut module = String::new();
	match target {
		KernelTarget::Amd(_) => {
			module.push_str("target triple = \"amdgcn-amd-amdhsa\"\n\n");
			module.push_str("declare i32 @llvm.amdgcn.workitem.id.x()\n");
			module.push_str("declare i32 @llvm.amdgcn.workgroup.id.x()\n");
		}
		KernelTarget::Nvidia(_) => {
			module.push_str("target triple = \"nvptx64-nvidia-cuda\"\n\n");
			module.push_str("declare i32 @llvm.nvvm.read.ptx.sreg.tid.x()\n");
			module.push_str("declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.x()\n");
		}
	}
	if emitter.uses_f32_arithmetic {
		for operation in ["fadd", "fsub", "fmul", "frem"] {
			if emitter
				.body
				.contains(&format!("constrained.{operation}.f32"))
			{
				writeln!(
					module,
					"declare float @llvm.experimental.constrained.{operation}.f32(float, float, metadata, metadata)"
				)
				.expect("writing to String");
			}
		}
	}
	if emitter.uses_fma {
		module.push_str(
			"declare float @llvm.experimental.constrained.fma.f32(float, float, float, metadata, metadata)\n",
		);
	}
	if emitter.uses_minimum {
		module.push_str("declare float @llvm.minimum.f32(float, float)\n");
	}
	if emitter.uses_maximum {
		module.push_str("declare float @llvm.maximum.f32(float, float)\n");
	}
	if emitter.uses_sat_conversion {
		module.push_str("declare i32 @llvm.fptosi.sat.i32.f32(float)\n");
	}
	if emitter.uses_square_root {
		module.push_str("declare float @llvm.sqrt.f32(float)\n");
	}
	if emitter.uses_floor {
		module.push_str("declare float @llvm.floor.f32(float)\n");
	}
	if emitter.uses_ceiling {
		module.push_str("declare float @llvm.ceil.f32(float)\n");
	}
	if emitter.uses_round_nearest_even {
		module.push_str("declare float @llvm.roundeven.f32(float)\n");
	}
	module.push('\n');

	let convention = match target {
		KernelTarget::Amd(_) => "amdgpu_kernel",
		KernelTarget::Nvidia(_) => "ptx_kernel",
	};
	write!(
		module,
		"define {convention} void @{}(",
		options.entry_symbol
	)
	.expect("writing to String");
	let mut arguments = template
		.inputs
		.iter()
		.enumerate()
		.map(|(index, _)| format!("ptr addrspace(1) %input_{index}"))
		.chain(
			template
				.outputs
				.iter()
				.enumerate()
				.map(|(index, _)| format!("ptr addrspace(1) %output_{index}")),
		)
		.collect::<Vec<_>>();
	if !emitter.fault_conditions.is_empty() {
		arguments.push("ptr addrspace(1) %fault_flag".to_owned());
	}
	arguments.push("i64 %element_count".to_owned());
	module.push_str(&arguments.join(", "));
	module.push_str(") #0 {\nentry:\n");
	module.push_str(&emitter.body);
	module.push_str("}\n\n");
	writeln!(
		module,
		"attributes #0 = {{ nounwind strictfp \"denormal-fp-math-f32\"=\"ieee\" \"no-trapping-math\"=\"false\" }}"
	)
	.expect("writing to String");
	module
}

const fn llvm_type(dtype: DType) -> &'static str {
	match dtype {
		DType::F32 => "float",
		DType::I32 => "i32",
	}
}

const fn instruction_flops(opcode: ScalarOpcode) -> u64 {
	opcode.flops()
}

#[cfg(test)]
mod tests {
	use std::io::Write as _;
	use std::process::{Command, Stdio};

	use recipe_core::{
		AliasPermission, AliasRule, ElementCount, IndexSpace, KernelInput, KernelInputId, KernelOutput,
		KernelOutputId, KernelTemplateId, ScalarInput, ScalarProgram,
	};

	use super::*;
	use crate::{AmdTarget, NvidiaTarget};

	fn add_template(dtype: DType) -> KernelTemplate {
		let left = ScalarValueId::new(1);
		let right = ScalarValueId::new(2);
		let result = ScalarValueId::new(3);
		let index_space = IndexSpace::new(vec![ElementCount::new(1024).unwrap()]).unwrap();
		let access = StaticBufferAccess::contiguous(&index_space, dtype).unwrap();
		KernelTemplate {
			id: KernelTemplateId::new(1),
			index_space,
			inputs: vec![
				KernelInput {
					id: KernelInputId::new(1),
					dtype,
					access: access.clone(),
				},
				KernelInput {
					id: KernelInputId::new(2),
					dtype,
					access: access.clone(),
				},
			],
			outputs: vec![KernelOutput {
				id: KernelOutputId::new(1),
				dtype,
				access,
			}],
			program: ScalarProgram {
				inputs: vec![
					ScalarInput { id: left, dtype },
					ScalarInput { id: right, dtype },
				],
				constants: Vec::new(),
				instructions: vec![ScalarInstruction {
					result,
					dtype,
					opcode: ScalarOpcode::Add,
					operands: vec![left, right],
				}],
				outputs: vec![result],
			},
			alias_rules: vec![
				AliasRule {
					input: KernelInputId::new(1),
					output: KernelOutputId::new(1),
					permission: AliasPermission::MayAliasExact,
				},
				AliasRule {
					input: KernelInputId::new(2),
					output: KernelOutputId::new(1),
					permission: AliasPermission::Forbidden,
				},
			],
		}
	}

	fn options() -> LoweringOptions {
		LoweringOptions {
			entry_symbol: "recipe_add".to_owned(),
			workgroup_lanes: 256,
		}
	}

	fn operation_template(opcode: ScalarOpcode, result_dtype: DType, input_dtypes: &[DType]) -> KernelTemplate {
		let index_space = IndexSpace::new(vec![ElementCount::new(32).unwrap()]).unwrap();
		let inputs = input_dtypes
			.iter()
			.copied()
			.enumerate()
			.map(|(index, dtype)| KernelInput {
				id: KernelInputId::new(u64::try_from(index + 1).unwrap()),
				dtype,
				access: StaticBufferAccess::contiguous(&index_space, dtype).unwrap(),
			})
			.collect::<Vec<_>>();
		let scalar_inputs = input_dtypes
			.iter()
			.copied()
			.enumerate()
			.map(|(index, dtype)| ScalarInput {
				id: ScalarValueId::new(u64::try_from(index + 1).unwrap()),
				dtype,
			})
			.collect::<Vec<_>>();
		let output = KernelOutputId::new(1);
		let result = ScalarValueId::new(u64::try_from(input_dtypes.len() + 1).unwrap());
		KernelTemplate {
			id: KernelTemplateId::new(2),
			index_space: index_space.clone(),
			alias_rules: inputs
				.iter()
				.map(|input| AliasRule {
					input: input.id,
					output,
					permission: AliasPermission::Forbidden,
				})
				.collect(),
			inputs,
			outputs: vec![KernelOutput {
				id: output,
				dtype: result_dtype,
				access: StaticBufferAccess::contiguous(&index_space, result_dtype).unwrap(),
			}],
			program: ScalarProgram {
				inputs: scalar_inputs.clone(),
				constants: Vec::new(),
				instructions: vec![ScalarInstruction {
					result,
					dtype: result_dtype,
					opcode,
					operands: scalar_inputs.iter().map(|input| input.id).collect(),
				}],
				outputs: vec![result],
			},
		}
	}

	#[test]
	fn emits_direct_amd_kernel_with_strict_f32_semantics() {
		let lowered = lower_elementwise(
			&add_template(DType::F32),
			&KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			&options(),
		)
		.unwrap();
		assert!(
			lowered
				.llvm_ir
				.contains("define amdgpu_kernel void @recipe_add")
		);
		assert!(lowered.llvm_ir.contains("constrained.fadd.f32"));
		assert!(lowered.llvm_ir.contains("canonical_nan"));
		assert_eq!(lowered.work, FlopCount::new(1024));
		assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
	}

	#[test]
	fn emits_backend_safe_plain_fdiv_for_amd_scalar_division() {
		let lowered = lower_elementwise(
			&operation_template(ScalarOpcode::Divide, DType::F32, &[DType::F32, DType::F32]),
			&KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			&options(),
		)
		.unwrap();
		assert!(lowered.llvm_ir.contains(" = fdiv float "));
		assert!(!lowered.llvm_ir.contains("constrained.fdiv"));
		assert!(lowered.llvm_ir.contains("canonical_nan"));
		assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
	}

	#[test]
	fn emits_backend_safe_sqrt_intrinsic_for_amd_scalar_square_root() {
		let lowered = lower_elementwise(
			&operation_template(ScalarOpcode::SquareRoot, DType::F32, &[DType::F32]),
			&KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			&options(),
		)
		.unwrap();
		assert!(
			lowered
				.llvm_ir
				.contains("declare float @llvm.sqrt.f32(float)")
		);
		assert!(lowered.llvm_ir.contains("call float @llvm.sqrt.f32(float "));
		assert!(!lowered.llvm_ir.contains("constrained.sqrt"));
		assert!(lowered.llvm_ir.contains("canonical_nan"));
		assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
	}

	#[test]
	#[ignore = "requires the host LLVM AMDGPU backend"]
	fn host_llc_compiles_amd_scalar_division_with_plain_fdiv() {
		let lowered = lower_elementwise(
			&operation_template(ScalarOpcode::Divide, DType::F32, &[DType::F32, DType::F32]),
			&KernelTarget::Amd(AmdTarget {
				target_id: "gfx1101".to_owned(),
				code_object_version: 6,
			}),
			&options(),
		)
		.unwrap();
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
			"host llc rejected AMD scalar division:\n{}",
			String::from_utf8_lossy(&output.stderr)
		);
	}

	#[test]
	fn emits_direct_nvptx_kernel_without_an_export_alias() {
		let lowered = lower_elementwise(
			&add_template(DType::I32),
			&KernelTarget::Nvidia(NvidiaTarget {
				sm_major: 5,
				sm_minor: 2,
				ptx_isa: 75,
			}),
			&options(),
		)
		.unwrap();
		assert!(
			lowered
				.llvm_ir
				.contains("define ptx_kernel void @recipe_add")
		);
		assert!(!lowered.llvm_ir.contains(" alias "));
		assert!(lowered.llvm_ir.contains("add i32"));
	}

	#[test]
	fn checked_integer_division_uses_a_deterministic_fault_abi() {
		let mut template = add_template(DType::I32);
		template.program.instructions[0].opcode = ScalarOpcode::Divide;
		let lowered = lower_elementwise(
			&template,
			&KernelTarget::Nvidia(NvidiaTarget {
				sm_major: 8,
				sm_minor: 6,
				ptx_isa: 75,
			}),
			&options(),
		)
		.unwrap();
		assert_eq!(
			lowered.abi.arguments,
			vec![
				KernelArgument::Buffer {
					access: BufferAccess::Read,
					dtype: DType::I32,
					alignment: 4,
				},
				KernelArgument::Buffer {
					access: BufferAccess::Read,
					dtype: DType::I32,
					alignment: 4,
				},
				KernelArgument::Buffer {
					access: BufferAccess::Write,
					dtype: DType::I32,
					alignment: 4,
				},
				KernelArgument::FaultFlag,
				KernelArgument::ElementCount,
			]
		);
		assert_eq!(lowered.abi.argument_bytes, 40);
		assert!(lowered.llvm_ir.contains("select i1 %division_rejected"));
		assert!(
			lowered
				.llvm_ir
				.contains("atomicrmw or ptr addrspace(1) %fault_flag, i32 1")
		);
		assert!(!lowered.llvm_ir.contains("sdiv i32 %loaded_1, %loaded_3"));
	}

	#[test]
	fn lowers_the_complete_scalar_opcode_families() {
		let target = KernelTarget::Amd(AmdTarget {
			target_id: "gfx1101".to_owned(),
			code_object_version: 6,
		});
		let cases = [
			(
				ScalarOpcode::Remainder,
				DType::F32,
				vec![DType::F32, DType::F32],
			),
			(
				ScalarOpcode::Remainder,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(ScalarOpcode::Negate, DType::I32, vec![DType::I32]),
			(ScalarOpcode::Absolute, DType::I32, vec![DType::I32]),
			(
				ScalarOpcode::Equal,
				DType::I32,
				vec![DType::F32, DType::F32],
			),
			(
				ScalarOpcode::NotEqual,
				DType::I32,
				vec![DType::F32, DType::F32],
			),
			(
				ScalarOpcode::LessThan,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(
				ScalarOpcode::LessThanOrEqual,
				DType::I32,
				vec![DType::F32, DType::F32],
			),
			(
				ScalarOpcode::GreaterThan,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(
				ScalarOpcode::GreaterThanOrEqual,
				DType::I32,
				vec![DType::F32, DType::F32],
			),
			(
				ScalarOpcode::Select,
				DType::F32,
				vec![DType::I32, DType::F32, DType::F32],
			),
			(
				ScalarOpcode::BitAnd,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(
				ScalarOpcode::BitOr,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(
				ScalarOpcode::BitXor,
				DType::I32,
				vec![DType::I32, DType::I32],
			),
			(ScalarOpcode::BitNot, DType::I32, vec![DType::I32]),
			(ScalarOpcode::BitcastF32ToI32, DType::I32, vec![DType::F32]),
			(ScalarOpcode::BitcastI32ToF32, DType::F32, vec![DType::I32]),
			(ScalarOpcode::Require, DType::I32, vec![DType::I32]),
			(ScalarOpcode::IsFinite, DType::I32, vec![DType::F32]),
			(ScalarOpcode::IsNan, DType::I32, vec![DType::F32]),
			(ScalarOpcode::SquareRoot, DType::F32, vec![DType::F32]),
			(ScalarOpcode::Floor, DType::F32, vec![DType::F32]),
			(ScalarOpcode::Ceiling, DType::F32, vec![DType::F32]),
			(ScalarOpcode::RoundNearestEven, DType::F32, vec![DType::F32]),
		];
		for (opcode, result_dtype, input_dtypes) in cases {
			let lowered = lower_elementwise(
				&operation_template(opcode, result_dtype, &input_dtypes),
				&target,
				&options(),
			)
			.unwrap_or_else(|error| panic!("{opcode:?} failed to lower: {error}"));
			assert!(audit_llvm_ir(&lowered.llvm_ir).is_empty());
		}
	}

	#[test]
	fn lowers_static_offsets_strides_and_broadcast_reads() {
		let mut template = add_template(DType::F32);
		template.index_space = IndexSpace::new(vec![
			ElementCount::new(2).unwrap(),
			ElementCount::new(3).unwrap(),
		])
		.unwrap();
		template.inputs[0].access = StaticBufferAccess {
			offset_elements: 1,
			strides: vec![0, 1],
			storage_bytes: recipe_core::ByteCount::new(16),
		};
		template.inputs[1].access = StaticBufferAccess::contiguous(&template.index_space, DType::F32).unwrap();
		template.outputs[0].access = StaticBufferAccess::contiguous(&template.index_space, DType::F32).unwrap();

		let lowered = lower_elementwise(
			&template,
			&KernelTarget::Nvidia(NvidiaTarget {
				sm_major: 8,
				sm_minor: 6,
				ptx_isa: 75,
			}),
			&options(),
		)
		.unwrap();
		assert!(lowered.llvm_ir.contains("urem i64 %global_id, 3"));
		assert!(lowered.llvm_ir.contains("input_0_element_index"));
		assert!(lowered.llvm_ir.contains("ptr addrspace(1) %input_0"));
		assert_eq!(lowered.abi.elements, ElementCount::new(6).unwrap());
	}
}

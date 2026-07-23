use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ValidationCode, ValidationResult, Validator};
use crate::ids::{KernelInputId, KernelOutputId, KernelTemplateId, ScalarValueId};
use crate::units::{ByteCount, ElementCount, UnitError};

/// The complete calculation-payload type domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DType {
	F32,
	I32,
}

impl DType {
	#[must_use]
	pub const fn byte_width(self) -> u8 {
		match self {
			Self::F32 | Self::I32 => 4,
		}
	}
}

/// Bit-preserving scalar literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarLiteral {
	F32Bits(u32),
	I32(i32),
}

impl ScalarLiteral {
	#[must_use]
	pub const fn dtype(self) -> DType {
		match self {
			Self::F32Bits(_) => DType::F32,
			Self::I32(_) => DType::I32,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ScalarOpcode {
	Add,
	Subtract,
	Multiply,
	/// IEEE division for f32; checked truncating division for int32.
	Divide,
	/// IEEE remainder for f32; checked truncating remainder for int32.
	Remainder,
	Negate,
	Absolute,
	Minimum,
	Maximum,
	/// Explicit one-rounding floating fused multiply-add.
	Fma,
	/// Produces int32 one or zero using IEEE ordered equality for f32.
	Equal,
	/// Produces int32 one or zero; an f32 NaN compares not-equal.
	NotEqual,
	LessThan,
	LessThanOrEqual,
	GreaterThan,
	GreaterThanOrEqual,
	/// int32 zero selects the third operand; nonzero selects the second.
	Select,
	BitAnd,
	BitOr,
	BitXor,
	BitNot,
	/// Preserve the exact binary32 representation as an int32 payload.
	BitcastF32ToI32,
	/// Interpret an int32 payload as an exact binary32 representation.
	BitcastI32ToF32,
	ShiftLeft,
	ShiftRightLogical,
	ShiftRightArithmetic,
	/// Normalize an int32 truth value and reject the calculation through its
	/// preallocated fault channel when the value is zero.
	Require,
	/// Produces int32 one for a finite f32 and zero otherwise.
	IsFinite,
	/// Produces int32 one for an f32 NaN and zero otherwise.
	IsNan,
	SquareRoot,
	Floor,
	Ceiling,
	RoundNearestEven,
	ConvertF32ToI32,
	ConvertI32ToF32,
}

impl ScalarOpcode {
	#[must_use]
	pub const fn arity(self) -> usize {
		match self {
			Self::Negate
			| Self::Absolute
			| Self::BitNot
			| Self::BitcastF32ToI32
			| Self::BitcastI32ToF32
			| Self::Require
			| Self::IsFinite
			| Self::IsNan
			| Self::SquareRoot
			| Self::Floor
			| Self::Ceiling
			| Self::RoundNearestEven
			| Self::ConvertF32ToI32
			| Self::ConvertI32ToF32 => 1,
			Self::Fma | Self::Select => 3,
			_ => 2,
		}
	}

	/// Resolve the result type for a valid operand signature.
	#[must_use]
	pub fn result_dtype(self, operands: &[DType]) -> Option<DType> {
		if operands.len() != self.arity() {
			return None;
		}
		match self {
			Self::Add
			| Self::Subtract
			| Self::Multiply
			| Self::Divide
			| Self::Remainder
			| Self::Negate
			| Self::Absolute
			| Self::Minimum
			| Self::Maximum => operands
				.iter()
				.all(|dtype| *dtype == operands[0])
				.then_some(operands[0]),
			Self::Fma => operands
				.iter()
				.all(|dtype| *dtype == DType::F32)
				.then_some(DType::F32),
			Self::Equal
			| Self::NotEqual
			| Self::LessThan
			| Self::LessThanOrEqual
			| Self::GreaterThan
			| Self::GreaterThanOrEqual => (operands[0] == operands[1]).then_some(DType::I32),
			Self::Select => (operands[0] == DType::I32 && operands[1] == operands[2]).then_some(operands[1]),
			Self::BitAnd
			| Self::BitOr
			| Self::BitXor
			| Self::BitNot
			| Self::ShiftLeft
			| Self::ShiftRightLogical
			| Self::ShiftRightArithmetic
			| Self::Require => operands
				.iter()
				.all(|dtype| *dtype == DType::I32)
				.then_some(DType::I32),
			Self::BitcastF32ToI32 | Self::IsFinite | Self::IsNan | Self::ConvertF32ToI32 => {
				(operands == [DType::F32]).then_some(DType::I32)
			}
			Self::BitcastI32ToF32 | Self::ConvertI32ToF32 => (operands == [DType::I32]).then_some(DType::F32),
			Self::SquareRoot | Self::Floor | Self::Ceiling | Self::RoundNearestEven => {
				(operands == [DType::F32]).then_some(DType::F32)
			}
		}
	}

	/// Deterministic arithmetic work used by the base scheduling equation.
	///
	/// Addressing, representation changes, validation predicates, and bit
	/// manipulation are deliberately not counted. Comparisons that produce a
	/// payload result count as one operation. FMA is two operations by the
	/// conventional FLOP definition.
	#[must_use]
	pub const fn flops(self) -> u64 {
		match self {
			Self::Add
			| Self::Subtract
			| Self::Multiply
			| Self::Divide
			| Self::Remainder
			| Self::Negate
			| Self::Absolute
			| Self::Minimum
			| Self::Maximum
			| Self::Equal
			| Self::NotEqual
			| Self::LessThan
			| Self::LessThanOrEqual
			| Self::GreaterThan
			| Self::GreaterThanOrEqual
			| Self::SquareRoot
			| Self::Floor
			| Self::Ceiling
			| Self::RoundNearestEven => 1,
			Self::Fma => 2,
			Self::Select
			| Self::BitAnd
			| Self::BitOr
			| Self::BitXor
			| Self::BitNot
			| Self::BitcastF32ToI32
			| Self::BitcastI32ToF32
			| Self::ShiftLeft
			| Self::ShiftRightLogical
			| Self::ShiftRightArithmetic
			| Self::Require
			| Self::IsFinite
			| Self::IsNan
			| Self::ConvertF32ToI32
			| Self::ConvertI32ToF32 => 0,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarInput {
	pub id: ScalarValueId,
	pub dtype: DType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarConstant {
	pub id: ScalarValueId,
	pub value: ScalarLiteral,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarInstruction {
	pub result: ScalarValueId,
	pub dtype: DType,
	pub opcode: ScalarOpcode,
	pub operands: Vec<ScalarValueId>,
}

/// Typed, acyclic, intra-kernel scalar program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarProgram {
	pub inputs: Vec<ScalarInput>,
	pub constants: Vec<ScalarConstant>,
	pub instructions: Vec<ScalarInstruction>,
	pub outputs: Vec<ScalarValueId>,
}

impl ScalarProgram {
	pub fn validate(&self) -> ValidationResult {
		let mut validator = Validator::default();
		let mut types = BTreeMap::<ScalarValueId, DType>::new();

		for (index, input) in self.inputs.iter().enumerate() {
			validator.require(
				types.insert(input.id, input.dtype).is_none(),
				ValidationCode::DuplicateScalarValue,
				format!("inputs[{index}].id"),
				format!("scalar value {} is defined more than once", input.id),
			);
		}
		for (index, constant) in self.constants.iter().enumerate() {
			validator.require(
				types.insert(constant.id, constant.value.dtype()).is_none(),
				ValidationCode::DuplicateScalarValue,
				format!("constants[{index}].id"),
				format!("scalar value {} is defined more than once", constant.id),
			);
		}

		for (index, instruction) in self.instructions.iter().enumerate() {
			let path = format!("instructions[{index}]");
			let operand_types = instruction
				.operands
				.iter()
				.map(|operand| types.get(operand).copied())
				.collect::<Vec<_>>();
			for (operand_index, (operand, dtype)) in instruction.operands.iter().zip(&operand_types).enumerate() {
				validator.require(
					dtype.is_some(),
					ValidationCode::ScalarUseBeforeDefinition,
					format!("{path}.operands[{operand_index}]"),
					format!("scalar value {operand} has not been defined"),
				);
			}
			validate_signature(
				&mut validator,
				&path,
				instruction.opcode,
				instruction.dtype,
				&operand_types,
			);
			validator.require(
				types.insert(instruction.result, instruction.dtype)
					.is_none(),
				ValidationCode::DuplicateScalarValue,
				format!("{path}.result"),
				format!(
					"scalar value {} is defined more than once",
					instruction.result
				),
			);
		}

		validator.require(
			!self.outputs.is_empty(),
			ValidationCode::MissingScalarOutput,
			"outputs",
			"scalar program must expose at least one output",
		);
		for (index, output) in self.outputs.iter().enumerate() {
			validator.require(
				types.contains_key(output),
				ValidationCode::UnknownScalarValue,
				format!("outputs[{index}]"),
				format!("unknown scalar output {output}"),
			);
		}
		validator.finish()
	}

	#[must_use]
	pub fn dtype_of(&self, value: ScalarValueId) -> Option<DType> {
		self.inputs
			.iter()
			.find_map(|input| (input.id == value).then_some(input.dtype))
			.or_else(|| {
				self.constants
					.iter()
					.find_map(|constant| (constant.id == value).then_some(constant.value.dtype()))
			})
			.or_else(|| {
				self.instructions
					.iter()
					.find_map(|instruction| (instruction.result == value).then_some(instruction.dtype))
			})
	}

	/// Whether lowering this program requires the preallocated device fault
	/// flag argument.
	#[must_use]
	pub fn requires_fault_flag(&self) -> bool {
		self.instructions.iter().any(|instruction| {
			matches!(
				(instruction.opcode, instruction.dtype),
				(ScalarOpcode::Require, DType::I32)
					| (ScalarOpcode::Divide, DType::I32)
					| (ScalarOpcode::Remainder, DType::I32)
					| (ScalarOpcode::Negate, DType::I32)
					| (ScalarOpcode::Absolute, DType::I32)
			)
		})
	}
}

fn validate_signature(
	validator: &mut Validator,
	path: &str,
	opcode: ScalarOpcode,
	result: DType,
	operands: &[Option<DType>],
) {
	let expected_arity = opcode.arity();
	validator.require(
		operands.len() == expected_arity,
		ValidationCode::ScalarArity,
		format!("{path}.operands"),
		format!(
			"{opcode:?} expects {expected_arity} operands, got {}",
			operands.len()
		),
	);
	if operands.len() != expected_arity || operands.iter().any(Option::is_none) {
		return;
	}
	let operands = operands
		.iter()
		.map(|dtype| dtype.expect("checked above"))
		.collect::<Vec<_>>();
	let valid = opcode.result_dtype(&operands) == Some(result);
	validator.require(
		valid,
		ValidationCode::ScalarTypeMismatch,
		format!("{path}.dtype"),
		format!("{opcode:?} cannot produce {result:?} from {operands:?}"),
	);
}

/// Parallel index space to which a scalar program is applied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexSpace {
	dimensions: Vec<ElementCount>,
	elements: ElementCount,
}

impl IndexSpace {
	pub fn new(dimensions: Vec<ElementCount>) -> Result<Self, UnitError> {
		if dimensions.is_empty() {
			return Err(UnitError::ZeroElementCount);
		}
		let mut elements = 1u64;
		for dimension in &dimensions {
			elements = elements
				.checked_mul(dimension.get())
				.ok_or(UnitError::Overflow)?;
		}
		Ok(Self {
			dimensions,
			elements: ElementCount::new(elements)?,
		})
	}

	#[must_use]
	pub fn dimensions(&self) -> &[ElementCount] {
		&self.dimensions
	}

	#[must_use]
	pub const fn elements(&self) -> ElementCount {
		self.elements
	}

	#[must_use]
	pub fn is_structural_singleton(&self) -> bool {
		self.elements.get() == 1
	}
}

/// Immutable mapping from the kernel's logical index space into one backing
/// buffer. Strides and the offset are expressed in elements; `storage_bytes`
/// bounds the complete backing value rather than only its logical payload.
///
/// A zero stride is valid for a read-only broadcast dimension. Writable
/// mappings are separately required to be injective.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticBufferAccess {
	pub offset_elements: u64,
	pub strides: Vec<u64>,
	pub storage_bytes: ByteCount,
}

impl StaticBufferAccess {
	pub fn linear(elements: ElementCount, dtype: DType) -> Result<Self, UnitError> {
		let storage_bytes = elements
			.get()
			.checked_mul(u64::from(dtype.byte_width()))
			.map(ByteCount::new)
			.ok_or(UnitError::Overflow)?;
		Ok(Self {
			offset_elements: 0,
			strides: vec![1],
			storage_bytes,
		})
	}

	pub fn contiguous(index_space: &IndexSpace, dtype: DType) -> Result<Self, UnitError> {
		let mut strides = vec![0_u64; index_space.dimensions().len()];
		let mut stride = 1_u64;
		for (axis, extent) in index_space.dimensions().iter().enumerate().rev() {
			strides[axis] = stride;
			stride = stride
				.checked_mul(extent.get())
				.ok_or(UnitError::Overflow)?;
		}
		let storage_bytes = stride
			.checked_mul(u64::from(dtype.byte_width()))
			.map(ByteCount::new)
			.ok_or(UnitError::Overflow)?;
		Ok(Self {
			offset_elements: 0,
			strides,
			storage_bytes,
		})
	}

	fn validate(
		&self,
		index_space: &IndexSpace,
		dtype: DType,
		writable: bool,
		path: &str,
		validator: &mut Validator,
	) {
		validator.require(
			self.strides.len() == index_space.dimensions().len(),
			ValidationCode::InvalidMemoryAccess,
			format!("{path}.strides"),
			format!(
				"buffer mapping rank {} differs from index-space rank {}",
				self.strides.len(),
				index_space.dimensions().len()
			),
		);

		let mut maximum = Some(self.offset_elements);
		for (extent, stride) in index_space.dimensions().iter().zip(&self.strides) {
			maximum = maximum.and_then(|value| {
				extent.get()
					.checked_sub(1)
					.and_then(|last| last.checked_mul(*stride))
					.and_then(|span| value.checked_add(span))
			});
		}
		let required_bytes = maximum
			.and_then(|last| last.checked_add(1))
			.and_then(|elements| elements.checked_mul(u64::from(dtype.byte_width())));
		validator.require(
			required_bytes.is_some(),
			ValidationCode::AddressOverflow,
			path,
			"buffer mapping address calculation overflows u64",
		);
		if let Some(required) = required_bytes {
			validator.require(
				required <= self.storage_bytes.get(),
				ValidationCode::InvalidMemoryAccess,
				format!("{path}.storage_bytes"),
				format!(
					"buffer mapping requires {required} bytes but backing storage has {}",
					self.storage_bytes.get()
				),
			);
		}

		if writable && self.strides.len() == index_space.dimensions().len() {
			let mut axes = index_space
				.dimensions()
				.iter()
				.zip(&self.strides)
				.filter(|(extent, _)| extent.get() > 1)
				.map(|(extent, stride)| (extent.get(), *stride))
				.collect::<Vec<_>>();
			axes.sort_by_key(|(_, stride)| *stride);
			let mut occupied_span = 1_u64;
			let mut injective = true;
			for (extent, stride) in axes {
				if stride < occupied_span {
					injective = false;
					break;
				}
				let Some(contribution) = (extent - 1).checked_mul(stride) else {
					injective = false;
					break;
				};
				let Some(next_span) = contribution.checked_add(occupied_span) else {
					injective = false;
					break;
				};
				occupied_span = next_span;
			}
			validator.require(
				injective,
				ValidationCode::InvalidMemoryAccess,
				format!("{path}.strides"),
				"writable buffer mapping is overlapping or broadcast",
			);
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelInput {
	pub id: KernelInputId,
	pub dtype: DType,
	pub access: StaticBufferAccess,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelOutput {
	pub id: KernelOutputId,
	pub dtype: DType,
	pub access: StaticBufferAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasPermission {
	Forbidden,
	MayAliasExact,
	MustAliasExact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AliasRule {
	pub input: KernelInputId,
	pub output: KernelOutputId,
	pub permission: AliasPermission,
}

/// A parallel kernel template with a complete input/output alias matrix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelTemplate {
	pub id: KernelTemplateId,
	pub index_space: IndexSpace,
	pub inputs: Vec<KernelInput>,
	pub outputs: Vec<KernelOutput>,
	pub program: ScalarProgram,
	pub alias_rules: Vec<AliasRule>,
}

impl KernelTemplate {
	pub fn validate(&self) -> ValidationResult {
		let mut validator = Validator::default();
		if let Err(errors) = self.program.validate() {
			validator.append("program", errors);
		}

		let mut input_ids = BTreeSet::new();
		for (index, input) in self.inputs.iter().enumerate() {
			validator.require(
				input_ids.insert(input.id),
				ValidationCode::DuplicateId,
				format!("inputs[{index}].id"),
				format!("kernel input {} appears more than once", input.id),
			);
			input.access.validate(
				&self.index_space,
				input.dtype,
				false,
				&format!("inputs[{index}].access"),
				&mut validator,
			);
		}
		let mut output_ids = BTreeSet::new();
		for (index, output) in self.outputs.iter().enumerate() {
			validator.require(
				output_ids.insert(output.id),
				ValidationCode::DuplicateId,
				format!("outputs[{index}].id"),
				format!("kernel output {} appears more than once", output.id),
			);
			output.access.validate(
				&self.index_space,
				output.dtype,
				true,
				&format!("outputs[{index}].access"),
				&mut validator,
			);
		}

		validator.require(
			self.inputs.len() == self.program.inputs.len(),
			ValidationCode::ScalarArity,
			"program.inputs",
			"kernel and scalar-program input counts differ",
		);
		for (index, (kernel, scalar)) in self.inputs.iter().zip(&self.program.inputs).enumerate() {
			validator.require(
				kernel.dtype == scalar.dtype,
				ValidationCode::ScalarTypeMismatch,
				format!("inputs[{index}].dtype"),
				"kernel and scalar-program input types differ",
			);
		}
		validator.require(
			self.outputs.len() == self.program.outputs.len(),
			ValidationCode::ScalarArity,
			"program.outputs",
			"kernel and scalar-program output counts differ",
		);
		for (index, (kernel, scalar_id)) in self.outputs.iter().zip(&self.program.outputs).enumerate() {
			validator.require(
				self.program.dtype_of(*scalar_id) == Some(kernel.dtype),
				ValidationCode::ScalarTypeMismatch,
				format!("outputs[{index}].dtype"),
				"kernel and scalar-program output types differ",
			);
		}

		let mut pairs = BTreeSet::new();
		for (index, rule) in self.alias_rules.iter().enumerate() {
			validator.require(
				input_ids.contains(&rule.input),
				ValidationCode::UnknownReference,
				format!("alias_rules[{index}].input"),
				format!("unknown kernel input {}", rule.input),
			);
			validator.require(
				output_ids.contains(&rule.output),
				ValidationCode::UnknownReference,
				format!("alias_rules[{index}].output"),
				format!("unknown kernel output {}", rule.output),
			);
			validator.require(
				pairs.insert((rule.input, rule.output)),
				ValidationCode::DuplicateAliasRule,
				format!("alias_rules[{index}]"),
				format!(
					"alias relationship ({}, {}) is declared more than once",
					rule.input, rule.output
				),
			);
		}
		for input in &self.inputs {
			for output in &self.outputs {
				validator.require(
					pairs.contains(&(input.id, output.id)),
					ValidationCode::MissingAliasRule,
					"alias_rules",
					format!(
						"alias relationship ({}, {}) is not declared",
						input.id, output.id
					),
				);
			}
		}

		validator.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn inplace_add_template() -> KernelTemplate {
		let lhs = ScalarValueId::new(1);
		let rhs = ScalarValueId::new(2);
		let result = ScalarValueId::new(3);
		KernelTemplate {
			id: KernelTemplateId::new(1),
			index_space: IndexSpace::new(vec![ElementCount::new(1024).unwrap()]).unwrap(),
			inputs: vec![
				KernelInput {
					id: KernelInputId::new(1),
					dtype: DType::F32,
					access: StaticBufferAccess::linear(ElementCount::new(1024).unwrap(), DType::F32).unwrap(),
				},
				KernelInput {
					id: KernelInputId::new(2),
					dtype: DType::F32,
					access: StaticBufferAccess::linear(ElementCount::new(1024).unwrap(), DType::F32).unwrap(),
				},
			],
			outputs: vec![KernelOutput {
				id: KernelOutputId::new(1),
				dtype: DType::F32,
				access: StaticBufferAccess::linear(ElementCount::new(1024).unwrap(), DType::F32).unwrap(),
			}],
			program: ScalarProgram {
				inputs: vec![
					ScalarInput {
						id: lhs,
						dtype: DType::F32,
					},
					ScalarInput {
						id: rhs,
						dtype: DType::F32,
					},
				],
				constants: vec![],
				instructions: vec![ScalarInstruction {
					result,
					dtype: DType::F32,
					opcode: ScalarOpcode::Add,
					operands: vec![lhs, rhs],
				}],
				outputs: vec![result],
			},
			alias_rules: vec![
				AliasRule {
					input: KernelInputId::new(1),
					output: KernelOutputId::new(1),
					permission: AliasPermission::MustAliasExact,
				},
				AliasRule {
					input: KernelInputId::new(2),
					output: KernelOutputId::new(1),
					permission: AliasPermission::Forbidden,
				},
			],
		}
	}

	#[test]
	fn validates_typed_ssa_and_complete_in_place_policy() {
		inplace_add_template().validate().unwrap();
	}

	#[test]
	fn validates_static_buffer_bounds_and_rejects_overlapping_writes() {
		let mut overlapping = inplace_add_template();
		overlapping.outputs[0].access.strides[0] = 0;
		let errors = overlapping.validate().unwrap_err();
		assert!(errors.contains(ValidationCode::InvalidMemoryAccess));

		let mut out_of_bounds = inplace_add_template();
		out_of_bounds.inputs[0].access.offset_elements = 1024;
		let errors = out_of_bounds.validate().unwrap_err();
		assert!(errors.contains(ValidationCode::InvalidMemoryAccess));
	}

	#[test]
	fn rejects_use_before_definition_and_type_mismatch() {
		let mut template = inplace_add_template();
		template.program.instructions[0].operands[1] = ScalarValueId::new(99);
		template.program.instructions[0].dtype = DType::I32;
		let errors = template.validate().unwrap_err();
		assert!(errors.contains(ValidationCode::ScalarUseBeforeDefinition));
		assert!(errors.contains(ValidationCode::ScalarTypeMismatch));
	}

	#[test]
	fn rejects_incomplete_alias_matrix() {
		let mut template = inplace_add_template();
		template.alias_rules.pop();
		let errors = template.validate().unwrap_err();
		assert!(errors.contains(ValidationCode::MissingAliasRule));
	}

	#[test]
	fn opcode_signatures_and_work_are_canonical() {
		assert_eq!(ScalarOpcode::Select.arity(), 3);
		assert_eq!(
			ScalarOpcode::Select.result_dtype(&[DType::I32, DType::F32, DType::F32]),
			Some(DType::F32)
		);
		assert_eq!(
			ScalarOpcode::Select.result_dtype(&[DType::F32, DType::F32, DType::F32]),
			None
		);
		assert_eq!(
			ScalarOpcode::BitcastF32ToI32.result_dtype(&[DType::F32]),
			Some(DType::I32)
		);
		assert_eq!(ScalarOpcode::Fma.flops(), 2);
		assert_eq!(ScalarOpcode::Require.flops(), 0);
	}
}

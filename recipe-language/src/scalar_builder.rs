use std::sync::atomic::{AtomicU64, Ordering};

use recipe_core::{
	DType, ScalarConstant, ScalarInput, ScalarInstruction, ScalarLiteral, ScalarOpcode, ScalarProgram, ScalarValueId,
};

use crate::{LanguageError, LanguageErrorKind, LanguageResult};

static NEXT_BUILDER: AtomicU64 = AtomicU64::new(1);

/// Opaque, typed reference to a value owned by one scalar-program builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarExpression {
	owner: u64,
	id: ScalarValueId,
	dtype: DType,
}

impl ScalarExpression {
	#[must_use]
	pub const fn id(self) -> ScalarValueId {
		self.id
	}

	#[must_use]
	pub const fn dtype(self) -> DType {
		self.dtype
	}
}

/// Deterministic constructor for typed scalar SSA programs.
///
/// It centralizes opcode signatures and prevents values from one program from
/// being accidentally spliced into another. Instruction order is exactly call
/// order and therefore becomes part of the artifact identity.
#[derive(Debug)]
pub struct ScalarProgramBuilder {
	owner: u64,
	next_value: u64,
	inputs: Vec<ScalarInput>,
	constants: Vec<ScalarConstant>,
	instructions: Vec<ScalarInstruction>,
}

impl ScalarProgramBuilder {
	pub fn new() -> LanguageResult<Self> {
		let owner = NEXT_BUILDER
			.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
				current.checked_add(1)
			})
			.map_err(|_| {
				LanguageError::new(
					LanguageErrorKind::InvalidScalarProgram,
					"scalar builder identity space exhausted",
				)
			})?;
		Ok(Self {
			owner,
			next_value: 1,
			inputs: Vec::new(),
			constants: Vec::new(),
			instructions: Vec::new(),
		})
	}

	pub fn input(&mut self, dtype: DType) -> LanguageResult<ScalarExpression> {
		let expression = self.next_expression(dtype)?;
		self.inputs.push(ScalarInput {
			id: expression.id,
			dtype,
		});
		Ok(expression)
	}

	pub fn constant(&mut self, value: ScalarLiteral) -> LanguageResult<ScalarExpression> {
		let expression = self.next_expression(value.dtype())?;
		self.constants.push(ScalarConstant {
			id: expression.id,
			value,
		});
		Ok(expression)
	}

	pub fn f32(&mut self, value: f32) -> LanguageResult<ScalarExpression> {
		self.constant(ScalarLiteral::F32Bits(value.to_bits()))
	}

	pub fn i32(&mut self, value: i32) -> LanguageResult<ScalarExpression> {
		self.constant(ScalarLiteral::I32(value))
	}

	pub fn apply(&mut self, opcode: ScalarOpcode, operands: &[ScalarExpression]) -> LanguageResult<ScalarExpression> {
		if let Some(foreign) = operands.iter().find(|operand| operand.owner != self.owner) {
			return Err(LanguageError::new(
				LanguageErrorKind::InvalidScalarProgram,
				format!(
					"scalar value {} belongs to another program builder",
					foreign.id
				),
			));
		}
		let operand_types = operands
			.iter()
			.map(|operand| operand.dtype)
			.collect::<Vec<_>>();
		let dtype = opcode.result_dtype(&operand_types).ok_or_else(|| {
			LanguageError::new(
				LanguageErrorKind::InvalidScalarProgram,
				format!("{opcode:?} does not accept operands {operand_types:?}"),
			)
		})?;
		let result = self.next_expression(dtype)?;
		self.instructions.push(ScalarInstruction {
			result: result.id,
			dtype,
			opcode,
			operands: operands.iter().map(|operand| operand.id).collect(),
		});
		Ok(result)
	}

	pub fn unary(&mut self, opcode: ScalarOpcode, operand: ScalarExpression) -> LanguageResult<ScalarExpression> {
		self.apply(opcode, &[operand])
	}

	pub fn binary(
		&mut self,
		opcode: ScalarOpcode,
		left: ScalarExpression,
		right: ScalarExpression,
	) -> LanguageResult<ScalarExpression> {
		self.apply(opcode, &[left, right])
	}

	pub fn ternary(
		&mut self,
		opcode: ScalarOpcode,
		first: ScalarExpression,
		second: ScalarExpression,
		third: ScalarExpression,
	) -> LanguageResult<ScalarExpression> {
		self.apply(opcode, &[first, second, third])
	}

	pub fn finish(self, outputs: &[ScalarExpression]) -> LanguageResult<ScalarProgram> {
		if let Some(foreign) = outputs.iter().find(|output| output.owner != self.owner) {
			return Err(LanguageError::new(
				LanguageErrorKind::InvalidScalarProgram,
				format!(
					"scalar output {} belongs to another program builder",
					foreign.id
				),
			));
		}
		let program = ScalarProgram {
			inputs: self.inputs,
			constants: self.constants,
			instructions: self.instructions,
			outputs: outputs.iter().map(|output| output.id).collect(),
		};
		program
			.validate()
			.map_err(|errors| LanguageError::new(LanguageErrorKind::InvalidScalarProgram, errors.to_string()))?;
		Ok(program)
	}

	fn next_expression(&mut self, dtype: DType) -> LanguageResult<ScalarExpression> {
		let id = self.next_value;
		self.next_value = self.next_value.checked_add(1).ok_or_else(|| {
			LanguageError::new(
				LanguageErrorKind::InvalidScalarProgram,
				"scalar value identity space exhausted",
			)
		})?;
		Ok(ScalarExpression {
			owner: self.owner,
			id: ScalarValueId::new(id),
			dtype,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn builds_a_typed_program_in_call_order() {
		let mut builder = ScalarProgramBuilder::new().unwrap();
		let left = builder.input(DType::F32).unwrap();
		let right = builder.input(DType::F32).unwrap();
		let product = builder.binary(ScalarOpcode::Multiply, left, right).unwrap();
		let one = builder.f32(1.0).unwrap();
		let result = builder.binary(ScalarOpcode::Add, product, one).unwrap();
		let program = builder.finish(&[result]).unwrap();
		assert_eq!(program.inputs.len(), 2);
		assert_eq!(program.constants.len(), 1);
		assert_eq!(program.instructions.len(), 2);
		assert_eq!(program.outputs, [result.id()]);
	}

	#[test]
	fn rejects_bad_signatures_and_foreign_values() {
		let mut first = ScalarProgramBuilder::new().unwrap();
		let float = first.input(DType::F32).unwrap();
		assert!(
			first.unary(ScalarOpcode::BitNot, float)
				.unwrap_err()
				.detail
				.contains("does not accept")
		);

		let mut second = ScalarProgramBuilder::new().unwrap();
		let other = second.input(DType::F32).unwrap();
		assert!(
			first.binary(ScalarOpcode::Add, float, other)
				.unwrap_err()
				.detail
				.contains("another program")
		);
	}
}

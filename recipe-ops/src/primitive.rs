use std::collections::BTreeMap;

use recipe_core::{AliasPermission, DType, ScalarLiteral, ValueId};
use recipe_language::{
	AtomicOperation, AtomicOrdering, IndexBounds, PrimitiveKernel, PrimitiveKind, ReduceOperator, ReduceResult,
	ScanMode, ScatterConflict, SortDirection, Tensor,
};
use recipe_primitives::{LoweredProgram, LoweringHardware};

use crate::{
	CanonicalDTypeContract, LoweringAvailability, OperationDescriptor, OperationError, OperationErrorKind,
	OperationFamily, OperationResult,
};

const F32_INPUT: &[DType] = &[DType::F32];
const F32_PAIR: &[DType] = &[DType::F32, DType::F32];
const F32_I32: &[DType] = &[DType::F32, DType::I32];
const F32_I32_F32: &[DType] = &[DType::F32, DType::I32, DType::F32];
const NO_INPUTS: &[DType] = &[];
const F32_OUTPUT: &[DType] = &[DType::F32];
const I32_OUTPUT: &[DType] = &[DType::I32];

/// Backend-neutral primitive categories exposed by the operation registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveFamily {
	Elementwise,
	Reduce,
	Scan,
	Contraction,
	Gather,
	Scatter,
	Histogram,
	Sort,
	IndexMap,
	Random,
}

/// Static axis relationship retained from a legacy operation name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisRequirement {
	Any,
	First,
	Last,
	All,
}

/// Shape class required by a contraction entrypoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractionClass {
	Vector,
	Matrix,
	LeftTransposedMatrix,
	RightTransposedMatrix,
	Batched,
}

/// Random distribution family whose concrete parameters are fixed in the
/// supplied primitive kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomRecipe {
	UniformF32,
	NormalF32,
}

/// An executable primitive recipe. Shape-dependent parameters such as axes,
/// extents, tree widths, random keys, and bounds policies remain explicit in
/// the caller-supplied `PrimitiveKernel`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveRecipe {
	Reduce {
		operator: ReduceOperator,
		result: ReduceResult,
		axes: AxisRequirement,
	},
	Scan {
		operator: ReduceOperator,
		exclusive: bool,
		axis: AxisRequirement,
	},
	Contraction(ContractionClass),
	Gather {
		axis: AxisRequirement,
	},
	ScatterAdd {
		axis: AxisRequirement,
	},
	Sort {
		direction: SortDirection,
		stable: bool,
		axis: AxisRequirement,
	},
	IndexMap,
	Random(RandomRecipe),
}

/// Borrowed, allocation-free input to primitive lowering.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveRequest<'a> {
	pub kernel: &'a PrimitiveKernel,
	pub tensors: &'a BTreeMap<ValueId, &'a Tensor>,
}

impl PrimitiveRecipe {
	pub(crate) fn for_symbol(symbol: &str) -> Option<Self> {
		SYMBOL_RECIPES
			.iter()
			.find_map(|(candidate, recipe)| (*candidate == symbol).then_some(*recipe))
	}

	#[must_use]
	pub const fn family(self) -> PrimitiveFamily {
		match self {
			Self::Reduce { .. } => PrimitiveFamily::Reduce,
			Self::Scan { .. } => PrimitiveFamily::Scan,
			Self::Contraction(_) => PrimitiveFamily::Contraction,
			Self::Gather { .. } => PrimitiveFamily::Gather,
			Self::ScatterAdd { .. } => PrimitiveFamily::Scatter,
			Self::Sort { .. } => PrimitiveFamily::Sort,
			Self::IndexMap => PrimitiveFamily::IndexMap,
			Self::Random(_) => PrimitiveFamily::Random,
		}
	}

	#[must_use]
	pub const fn operation_family(self) -> OperationFamily {
		match self.family() {
			PrimitiveFamily::Elementwise => OperationFamily::Elementwise,
			PrimitiveFamily::Reduce => OperationFamily::Reduction,
			PrimitiveFamily::Scan => OperationFamily::Scan,
			PrimitiveFamily::Contraction => OperationFamily::Contraction,
			PrimitiveFamily::Gather
			| PrimitiveFamily::Scatter
			| PrimitiveFamily::Sort
			| PrimitiveFamily::IndexMap => OperationFamily::ShapeAndIndexing,
			PrimitiveFamily::Histogram => OperationFamily::Histogram,
			PrimitiveFamily::Random => OperationFamily::Random,
		}
	}

	#[must_use]
	pub const fn dtype_contract(self) -> CanonicalDTypeContract {
		match self {
			Self::Reduce {
				result: ReduceResult::Value,
				..
			}
			| Self::Scan { .. } => CanonicalDTypeContract::Exact {
				inputs: F32_INPUT,
				outputs: F32_OUTPUT,
			},
			Self::Reduce {
				result: ReduceResult::Index,
				..
			} => CanonicalDTypeContract::Exact {
				inputs: F32_INPUT,
				outputs: I32_OUTPUT,
			},
			Self::Reduce {
				result: ReduceResult::ValueAndIndex,
				..
			} => CanonicalDTypeContract::Exact {
				inputs: F32_INPUT,
				outputs: &[DType::F32, DType::I32],
			},
			Self::Contraction(_) => CanonicalDTypeContract::Exact {
				inputs: F32_PAIR,
				outputs: F32_OUTPUT,
			},
			Self::Gather { .. } => CanonicalDTypeContract::Exact {
				inputs: F32_I32,
				outputs: F32_OUTPUT,
			},
			Self::ScatterAdd { .. } => CanonicalDTypeContract::Exact {
				inputs: F32_I32_F32,
				outputs: F32_OUTPUT,
			},
			Self::Sort { .. } => CanonicalDTypeContract::Exact {
				inputs: F32_INPUT,
				outputs: F32_OUTPUT,
			},
			Self::IndexMap => CanonicalDTypeContract::Exact {
				inputs: NO_INPUTS,
				outputs: I32_OUTPUT,
			},
			Self::Random(RandomRecipe::UniformF32 | RandomRecipe::NormalF32) => CanonicalDTypeContract::Exact {
				inputs: NO_INPUTS,
				outputs: F32_OUTPUT,
			},
		}
	}

	#[must_use]
	pub const fn definition(self) -> &'static str {
		match self {
			Self::Reduce {
				operator: ReduceOperator::Sum,
				result: ReduceResult::Value,
				..
			} => "fixed-tree binary32 sum reduction",
			Self::Reduce {
				operator: ReduceOperator::Minimum,
				result: ReduceResult::Value,
				..
			} => "fixed-tree binary32 minimum reduction",
			Self::Reduce {
				operator: ReduceOperator::Maximum,
				result: ReduceResult::Value,
				..
			} => "fixed-tree binary32 maximum reduction",
			Self::Reduce {
				operator: ReduceOperator::Minimum,
				result: ReduceResult::Index,
				..
			} => "fixed-tree binary32 argmin reduction yielding int32",
			Self::Reduce {
				operator: ReduceOperator::Maximum,
				result: ReduceResult::Index,
				..
			} => "fixed-tree binary32 argmax reduction yielding int32",
			Self::Reduce { .. } => "typed fixed-tree reduction",
			Self::Scan {
				operator: ReduceOperator::Sum,
				exclusive: false,
				..
			} => "fixed-tree inclusive binary32 prefix sum",
			Self::Scan {
				operator: ReduceOperator::Sum,
				exclusive: true,
				..
			} => "fixed-tree exclusive binary32 prefix sum",
			Self::Scan {
				operator: ReduceOperator::Product,
				..
			} => "fixed-tree inclusive binary32 cumulative product",
			Self::Scan {
				operator: ReduceOperator::Maximum,
				..
			} => "fixed-tree inclusive binary32 cumulative maximum",
			Self::Scan { .. } => "typed fixed-tree scan",
			Self::Contraction(ContractionClass::Vector) => "binary32 vector contraction",
			Self::Contraction(ContractionClass::Matrix) => "binary32 matrix contraction",
			Self::Contraction(ContractionClass::LeftTransposedMatrix) => {
				"binary32 left-transposed matrix contraction"
			}
			Self::Contraction(ContractionClass::RightTransposedMatrix) => {
				"binary32 right-transposed matrix contraction"
			}
			Self::Contraction(ContractionClass::Batched) => "binary32 batched matrix contraction",
			Self::Gather { .. } => "checked int32-indexed binary32 gather",
			Self::ScatterAdd { .. } => "checked int32-indexed binary32 atomic-add scatter",
			Self::Sort {
				direction: SortDirection::Ascending,
				stable: true,
				..
			} => "stable ascending binary32 sort",
			Self::Sort {
				direction: SortDirection::Ascending,
				stable: false,
				..
			} => "ascending binary32 sort",
			Self::Sort {
				direction: SortDirection::Descending,
				stable: true,
				..
			} => "stable descending binary32 sort",
			Self::Sort {
				direction: SortDirection::Descending,
				stable: false,
				..
			} => "descending binary32 sort",
			Self::IndexMap => {
				"checked iteration-aware affine int32 index map with optional positive Euclidean modulus"
			}
			Self::Random(RandomRecipe::UniformF32) => "counter-based Philox4x32-10 uniform binary32 map",
			Self::Random(RandomRecipe::NormalF32) => "counter-based Philox4x32-10 normal binary32 map",
		}
	}

	fn matches(self, kernel: &PrimitiveKernel, tensors: &BTreeMap<ValueId, &Tensor>) -> bool {
		match (self, &kernel.kind) {
			(
				Self::Reduce {
					operator,
					result,
					axes,
				},
				PrimitiveKind::Reduce(spec),
			) => {
				operator == spec.operator
					&& result == spec.result && axes_match(axes, spec.axes.as_slice(), kernel, tensors)
			}
			(
				Self::Scan {
					operator,
					exclusive,
					axis,
				},
				PrimitiveKind::Scan(spec),
			) => {
				operator == spec.operator
					&& exclusive == matches!(spec.mode, ScanMode::Exclusive { .. })
					&& axis_matches(axis, spec.axis, kernel, tensors)
					&& !spec.reverse && scan_identity_matches(spec)
			}
			(Self::Contraction(class), PrimitiveKind::Contraction(spec)) => {
				contraction_matches(class, spec, kernel, tensors)
			}
			(Self::Gather { axis }, PrimitiveKind::Gather(spec)) => {
				axis_matches(axis, spec.axis, kernel, tensors) && spec.bounds == IndexBounds::Reject
			}
			(Self::ScatterAdd { axis }, PrimitiveKind::Scatter(spec)) => {
				axis_matches(axis, spec.axis, kernel, tensors)
					&& spec.bounds == IndexBounds::Reject
					&& matches!(
						spec.conflict,
						ScatterConflict::Atomic {
							operation: AtomicOperation::Add,
							ordering: AtomicOrdering::SequentiallyConsistent,
						}
					) && kernel.alias_rules.iter().any(|rule| {
					rule.input == 0 && rule.output == 0 && rule.permission == AliasPermission::MustAliasExact
				})
			}
			(
				Self::Sort {
					direction,
					stable,
					axis,
				},
				PrimitiveKind::Sort(spec),
			) => {
				direction == spec.direction
					&& stable == spec.stable && axis_matches(axis, spec.axis, kernel, tensors)
					&& !spec.emit_indices
			}
			(Self::Random(RandomRecipe::UniformF32), PrimitiveKind::Random(spec)) => matches!(
				spec.distribution,
				recipe_language::RandomDistribution::UniformF32
			),
			(Self::Random(RandomRecipe::NormalF32), PrimitiveKind::Random(spec)) => matches!(
				spec.distribution,
				recipe_language::RandomDistribution::NormalF32
			),
			(Self::IndexMap, PrimitiveKind::IndexMap(_)) => true,
			(
				Self::Reduce { .. }
				| Self::Scan { .. }
				| Self::Contraction(_)
				| Self::Gather { .. }
				| Self::ScatterAdd { .. }
				| Self::Sort { .. }
				| Self::IndexMap
				| Self::Random(_),
				PrimitiveKind::Elementwise(_)
				| PrimitiveKind::Reduce(_)
				| PrimitiveKind::Scan(_)
				| PrimitiveKind::Contraction(_)
				| PrimitiveKind::Gather(_)
				| PrimitiveKind::Scatter(_)
				| PrimitiveKind::Histogram(_)
				| PrimitiveKind::Sort(_)
				| PrimitiveKind::IndexMap(_)
				| PrimitiveKind::Random(_),
			) => false,
		}
	}
}

/// Verify an operation-specific primitive recipe and lower it into the
/// deterministic backend-neutral primitive program.
pub fn lower_primitive(
	descriptor: OperationDescriptor,
	request: PrimitiveRequest<'_>,
	hardware: LoweringHardware,
) -> OperationResult<LoweredProgram> {
	let recipe = match descriptor.lowering {
		LoweringAvailability::Primitive(recipe) => recipe,
		LoweringAvailability::Scalar(_)
		| LoweringAvailability::Composition(_)
		| LoweringAvailability::Workspace(_)
		| LoweringAvailability::NonCalculation(_) => {
			return Err(OperationError::new(
				OperationErrorKind::WrongLoweringKind,
				"operation does not have a direct primitive recipe",
			)
			.for_operation(descriptor.id));
		}
		LoweringAvailability::Unsupported(reason) => {
			return Err(OperationError::new(
				OperationErrorKind::UnsupportedLowering,
				format!("{}: {reason:?}", descriptor.definition),
			)
			.for_operation(descriptor.id));
		}
	};
	lower_recipe(recipe, request, hardware).map_err(|error| error.for_operation(descriptor.id))
}

/// Lower Recipe's iteration-aware affine int32 source without inventing a
/// legacy operation-registry symbol.
///
/// The supplied kernel must have no inputs, exactly one int32 output, and no
/// aliases. Its output shape fixes the number of generated indexes; `start`,
/// `element_step`, `iteration_step`, and the optional positive modulus remain
/// explicit in the immutable lowered program.
pub fn lower_index_map(request: PrimitiveRequest<'_>, hardware: LoweringHardware) -> OperationResult<LoweredProgram> {
	lower_recipe(PrimitiveRecipe::IndexMap, request, hardware)
}

fn lower_recipe(
	recipe: PrimitiveRecipe,
	request: PrimitiveRequest<'_>,
	hardware: LoweringHardware,
) -> OperationResult<LoweredProgram> {
	match recipe.matches(request.kernel, request.tensors) {
		true => recipe_primitives::lower(request.kernel, request.tensors, hardware).map_err(|error| {
			OperationError::new(
				OperationErrorKind::PrimitiveLoweringFailed,
				error.to_string(),
			)
		}),
		false => Err(OperationError::new(
			OperationErrorKind::PrimitiveRecipeMismatch,
			format!("kernel kind does not satisfy {}", recipe.definition()),
		)),
	}
}

fn axes_match(
	requirement: AxisRequirement,
	axes: &[usize],
	kernel: &PrimitiveKernel,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> bool {
	let rank = input_rank(kernel, tensors);
	match requirement {
		AxisRequirement::Any => !axes.is_empty(),
		AxisRequirement::First => axes == [0],
		AxisRequirement::Last => rank.is_some_and(|rank| axes == [rank.saturating_sub(1)]),
		AxisRequirement::All => rank.is_some_and(|rank| axes.len() == rank && axes.iter().copied().eq(0..rank)),
	}
}

fn axis_matches(
	requirement: AxisRequirement,
	axis: usize,
	kernel: &PrimitiveKernel,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> bool {
	match requirement {
		AxisRequirement::Any => true,
		AxisRequirement::First => axis == 0,
		AxisRequirement::Last => input_rank(kernel, tensors).is_some_and(|rank| axis == rank.saturating_sub(1)),
		AxisRequirement::All => input_rank(kernel, tensors) == Some(1) && axis == 0,
	}
}

fn input_rank(kernel: &PrimitiveKernel, tensors: &BTreeMap<ValueId, &Tensor>) -> Option<usize> {
	kernel.inputs
		.first()
		.and_then(|value| tensors.get(value))
		.map(|tensor| tensor.shape.rank())
}

fn scan_identity_matches(spec: &recipe_language::Scan) -> bool {
	match (spec.operator, spec.mode) {
		(_, ScanMode::Inclusive) => true,
		(
			ReduceOperator::Sum,
			ScanMode::Exclusive {
				identity: ScalarLiteral::F32Bits(bits),
			},
		) => bits == 0.0_f32.to_bits(),
		(
			ReduceOperator::Product,
			ScanMode::Exclusive {
				identity: ScalarLiteral::F32Bits(bits),
			},
		) => bits == 1.0_f32.to_bits(),
		(
			ReduceOperator::Minimum | ReduceOperator::Maximum | ReduceOperator::Any | ReduceOperator::All,
			ScanMode::Exclusive { .. },
		)
		| (
			ReduceOperator::Sum | ReduceOperator::Product,
			ScanMode::Exclusive {
				identity: ScalarLiteral::I32(_),
			},
		) => false,
	}
}

fn contraction_matches(
	class: ContractionClass,
	spec: &recipe_language::Contraction,
	kernel: &PrimitiveKernel,
	tensors: &BTreeMap<ValueId, &Tensor>,
) -> bool {
	let ranks = kernel
		.inputs
		.first()
		.zip(kernel.inputs.get(1))
		.and_then(|(left, right)| {
			Some((
				tensors.get(left)?.shape.rank(),
				tensors.get(right)?.shape.rank(),
			))
		});
	match class {
		ContractionClass::Vector => {
			ranks == Some((1, 1)) && spec.batch_axes.is_empty() && spec.contract_axes == [(0, 0)]
		}
		ContractionClass::Matrix => {
			ranks == Some((2, 2)) && spec.batch_axes.is_empty() && spec.contract_axes == [(1, 0)]
		}
		ContractionClass::LeftTransposedMatrix => {
			ranks == Some((2, 2)) && spec.batch_axes.is_empty() && spec.contract_axes == [(0, 0)]
		}
		ContractionClass::RightTransposedMatrix => {
			ranks == Some((2, 2)) && spec.batch_axes.is_empty() && spec.contract_axes == [(1, 1)]
		}
		ContractionClass::Batched => {
			ranks.is_some_and(|(left, right)| left >= 3 && right >= 3)
				&& !spec.batch_axes.is_empty()
				&& spec.contract_axes.len() == 1
		}
	}
}

const SYMBOL_RECIPES: &[(&str, PrimitiveRecipe)] = &[
	(
		"gpu_argmax_f32",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Maximum,
			result: ReduceResult::Index,
			axes: AxisRequirement::All,
		},
	),
	(
		"gpu_argmax_rows",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Maximum,
			result: ReduceResult::Index,
			axes: AxisRequirement::Last,
		},
	),
	(
		"gpu_argmin_rows",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Minimum,
			result: ReduceResult::Index,
			axes: AxisRequirement::Last,
		},
	),
	(
		"gpu_bmm_into",
		PrimitiveRecipe::Contraction(ContractionClass::Batched),
	),
	(
		"gpu_cummax",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Maximum,
			exclusive: false,
			axis: AxisRequirement::Any,
		},
	),
	(
		"gpu_cumprod",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Product,
			exclusive: false,
			axis: AxisRequirement::Any,
		},
	),
	(
		"gpu_cumsum_cols",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Sum,
			exclusive: false,
			axis: AxisRequirement::First,
		},
	),
	(
		"gpu_cumsum_rows",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Sum,
			exclusive: false,
			axis: AxisRequirement::Last,
		},
	),
	(
		"gpu_dot",
		PrimitiveRecipe::Contraction(ContractionClass::Vector),
	),
	(
		"gpu_gather_rows_into",
		PrimitiveRecipe::Gather {
			axis: AxisRequirement::First,
		},
	),
	(
		"gpu_gemm",
		PrimitiveRecipe::Contraction(ContractionClass::Matrix),
	),
	(
		"gpu_gemm_at",
		PrimitiveRecipe::Contraction(ContractionClass::LeftTransposedMatrix),
	),
	(
		"gpu_gemm_bt",
		PrimitiveRecipe::Contraction(ContractionClass::RightTransposedMatrix),
	),
	(
		"gpu_gemm_bt_into",
		PrimitiveRecipe::Contraction(ContractionClass::RightTransposedMatrix),
	),
	(
		"gpu_max_all",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Maximum,
			result: ReduceResult::Value,
			axes: AxisRequirement::All,
		},
	),
	(
		"gpu_min_all",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Minimum,
			result: ReduceResult::Value,
			axes: AxisRequirement::All,
		},
	),
	(
		"gpu_prefix_sum_exclusive",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Sum,
			exclusive: true,
			axis: AxisRequirement::Any,
		},
	),
	(
		"gpu_prefix_sum_inclusive",
		PrimitiveRecipe::Scan {
			operator: ReduceOperator::Sum,
			exclusive: false,
			axis: AxisRequirement::Any,
		},
	),
	(
		"gpu_rand_uniform_into",
		PrimitiveRecipe::Random(RandomRecipe::UniformF32),
	),
	(
		"gpu_randn",
		PrimitiveRecipe::Random(RandomRecipe::NormalF32),
	),
	(
		"gpu_reduce_max_cols",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Maximum,
			result: ReduceResult::Value,
			axes: AxisRequirement::First,
		},
	),
	(
		"gpu_reduce_max_rows",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Maximum,
			result: ReduceResult::Value,
			axes: AxisRequirement::Last,
		},
	),
	(
		"gpu_reduce_min_cols",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Minimum,
			result: ReduceResult::Value,
			axes: AxisRequirement::First,
		},
	),
	(
		"gpu_reduce_min_rows",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Minimum,
			result: ReduceResult::Value,
			axes: AxisRequirement::Last,
		},
	),
	(
		"gpu_reduce_sum_cols_into",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Sum,
			result: ReduceResult::Value,
			axes: AxisRequirement::First,
		},
	),
	(
		"gpu_reduce_sum_rows",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Sum,
			result: ReduceResult::Value,
			axes: AxisRequirement::Last,
		},
	),
	(
		"gpu_scatter_add",
		PrimitiveRecipe::ScatterAdd {
			axis: AxisRequirement::First,
		},
	),
	(
		"gpu_sort",
		PrimitiveRecipe::Sort {
			direction: SortDirection::Ascending,
			stable: false,
			axis: AxisRequirement::All,
		},
	),
	(
		"gpu_sum_all",
		PrimitiveRecipe::Reduce {
			operator: ReduceOperator::Sum,
			result: ReduceResult::Value,
			axes: AxisRequirement::All,
		},
	),
];

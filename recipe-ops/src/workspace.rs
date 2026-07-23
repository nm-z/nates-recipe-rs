use recipe_core::ByteCount;

use crate::{LoweringAvailability, OperationDescriptor, OperationError, OperationErrorKind, OperationResult};

const TREE_LANES: u64 = 64;
const SOLVER_PANEL: u64 = 32;
const SPLIT_K_ROWS: u64 = 256;
const MAX_SPLIT_K_PARTS: u64 = 8;
const WORD_BYTES: u64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceUnit {
	Bytes,
	F32Elements,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceFormula {
	NoPersistentScratch {
		dimensions: usize,
	},
	FixedTreeReduction {
		dimensions: usize,
		sequences_dimension: Option<usize>,
	},
	FixedTreeScan {
		dimensions: usize,
	},
	StableSort {
		dimensions: usize,
	},
	SortRunEncoding {
		dimensions: usize,
	},
	MapThenReduction {
		dimensions: usize,
	},
	RandomKeySort {
		dimensions: usize,
	},
	CholeskyFactor,
	CholeskySolve,
	CholeskyInverse,
	LuFactor,
	LuSolve,
	QrFactor,
	SymmetricEigensolver,
	SingularValueDecomposition,
	SplitKPartials,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkspaceValue {
	pub amount: u64,
	pub unit: WorkspaceUnit,
}

impl WorkspaceValue {
	#[must_use]
	pub const fn bytes(self) -> Option<ByteCount> {
		match self.unit {
			WorkspaceUnit::Bytes => Some(ByteCount::new(self.amount)),
			WorkspaceUnit::F32Elements => None,
		}
	}
}

impl WorkspaceFormula {
	pub(crate) fn for_symbol(symbol: &str) -> Option<Self> {
		let formula = match symbol {
			"gpu_sum_all_workspace_bytes"
			| "gpu_max_all_workspace_bytes"
			| "gpu_min_all_workspace_bytes"
			| "gpu_mean_all_workspace_bytes" => Self::FixedTreeReduction {
				dimensions: 1,
				sequences_dimension: None,
			},
			"gpu_l2_norm_workspace_bytes" => Self::MapThenReduction { dimensions: 1 },
			"gpu_dot_workspace_bytes" => Self::NoPersistentScratch { dimensions: 1 },
			"gpu_reduce_sum_cols_workspace_bytes" => Self::FixedTreeReduction {
				dimensions: 2,
				sequences_dimension: Some(1),
			},
			"gpu_cumprod_workspace_bytes" | "gpu_cummax_workspace_bytes" => Self::FixedTreeScan { dimensions: 1 },
			"gpu_random_permutation_workspace_bytes" => Self::RandomKeySort { dimensions: 1 },
			"gpu_count_distinct_workspace_bytes" | "gpu_run_length_workspace_bytes" => {
				Self::SortRunEncoding { dimensions: 1 }
			}
			"gpu_cholesky_workspace_bytes" => Self::CholeskyFactor,
			"gpu_cholesky_solve_workspace_bytes" | "gpu_potrs_workspace_bytes" => Self::CholeskySolve,
			"gpu_cholesky_inv_workspace_bytes" => Self::CholeskyInverse,
			"gpu_lu_factor_workspace_bytes" | "gpu_solve_getrf_workspace_bytes" => Self::LuFactor,
			"gpu_lu_solve_workspace_bytes" | "gpu_solve_getrs_workspace_bytes" => Self::LuSolve,
			"gpu_qr_workspace_bytes" => Self::QrFactor,
			"gpu_eigh_sym_workspace_bytes" => Self::SymmetricEigensolver,
			"gpu_svd_workspace_bytes" => Self::SingularValueDecomposition,
			"gpu_splitk_dw_partials_elems" => Self::SplitKPartials,
			_ => return None,
		};
		Some(formula)
	}

	#[must_use]
	pub const fn definition(self) -> &'static str {
		match self {
			Self::NoPersistentScratch { .. } => {
				"zero persistent bytes; the canonical vector contraction uses only fixed shared-memory tiles"
			}
			Self::FixedTreeReduction { .. } => "sum of every non-final 64-lane fixed-tree f32 reduction level",
			Self::FixedTreeScan { .. } => {
				"f32 block totals and hierarchy outputs for every 64-lane fixed-tree scan level"
			}
			Self::StableSort { .. } => {
				"power-of-two-padded f32 values plus int32 original indexes for the stable sorting network"
			}
			Self::SortRunEncoding { .. } => {
				"stable-sort scratch plus int32 transition flags and their fixed-tree prefix scan"
			}
			Self::MapThenReduction { .. } => {
				"one f32 scalar-map image plus every non-final 64-lane fixed-tree reduction level"
			}
			Self::RandomKeySort { .. } => {
				"one f32 Philox key image plus padded f32 values and int32 indexes for the stable sorting network"
			}
			Self::CholeskyFactor => {
				"one 32-column f32 Cholesky panel, clipped to matrix order, plus one int32 fault word"
			}
			Self::CholeskySolve => "one f32 right-hand-side solve image plus one int32 fault word",
			Self::CholeskyInverse => "one dense f32 identity/solve image plus one int32 fault word",
			Self::LuFactor => "one 32-column f32 LU panel, int32 pivots, and one int32 fault word",
			Self::LuSolve => "one f32 permuted right-hand-side image plus one int32 fault word",
			Self::QrFactor => "one f32 Householder panel image plus one f32 tau value per reflector",
			Self::SymmetricEigensolver => {
				"one dense f32 Jacobi work matrix, one f32 diagonal vector, and one int32 fault word"
			}
			Self::SingularValueDecomposition => {
				"one f32 bidiagonalization image, two reflector panels, singular values, and one fault word"
			}
			Self::SplitKPartials => "f32 partial elements for at most eight deterministic 256-row split-K slices",
		}
	}

	#[must_use]
	pub const fn unit(self) -> WorkspaceUnit {
		match self {
			Self::SplitKPartials => WorkspaceUnit::F32Elements,
			_ => WorkspaceUnit::Bytes,
		}
	}

	pub fn evaluate(self, dimensions: &[u64]) -> OperationResult<WorkspaceValue> {
		let expected = self.dimension_count();
		if dimensions.len() != expected {
			return Err(OperationError::new(
				OperationErrorKind::WorkspaceFormulaMismatch,
				format!(
					"{} expects {expected} dimensions, received {}",
					self.definition(),
					dimensions.len()
				),
			));
		}
		let amount = match self {
			Self::NoPersistentScratch { .. } => 0,
			Self::FixedTreeReduction {
				sequences_dimension,
				..
			} => {
				let sequences = sequences_dimension.map_or(1, |index| dimensions[index]);
				reduction_scratch(dimensions[0], sequences)?
			}
			Self::FixedTreeScan { .. } => scan_scratch(dimensions[0])?,
			Self::StableSort { .. } => sort_scratch(dimensions[0])?,
			Self::SortRunEncoding { .. } => {
				let elements = dimensions[0];
				checked_add(
					checked_add(sort_scratch(elements)?, checked_mul(elements, WORD_BYTES)?)?,
					scan_scratch(elements)?,
				)?
			}
			Self::MapThenReduction { .. } => checked_add(
				checked_mul(dimensions[0], WORD_BYTES)?,
				reduction_scratch(dimensions[0], 1)?,
			)?,
			Self::RandomKeySort { .. } => checked_add(
				checked_mul(dimensions[0], WORD_BYTES)?,
				sort_scratch(dimensions[0])?,
			)?,
			Self::CholeskyFactor => {
				let n = dimensions[0];
				checked_add(
					checked_mul(checked_mul(n, n.min(SOLVER_PANEL))?, WORD_BYTES)?,
					WORD_BYTES,
				)?
			}
			Self::CholeskySolve => dense_image_with_fault(dimensions[0], dimensions[1])?,
			Self::CholeskyInverse => dense_image_with_fault(dimensions[0], dimensions[0])?,
			Self::LuFactor => {
				let n = dimensions[0];
				let panel = checked_mul(checked_mul(n, n.min(SOLVER_PANEL))?, WORD_BYTES)?;
				let pivots = checked_mul(n, WORD_BYTES)?;
				checked_add(checked_add(panel, pivots)?, WORD_BYTES)?
			}
			Self::LuSolve => dense_image_with_fault(dimensions[0], dimensions[1])?,
			Self::QrFactor => {
				let (m, n) = (dimensions[0], dimensions[1]);
				let reflectors = m.min(n);
				checked_mul(
					checked_add(checked_mul(m, reflectors)?, reflectors)?,
					WORD_BYTES,
				)?
			}
			Self::SymmetricEigensolver => {
				let n = dimensions[0];
				checked_add(
					checked_mul(checked_add(checked_mul(n, n)?, n)?, WORD_BYTES)?,
					WORD_BYTES,
				)?
			}
			Self::SingularValueDecomposition => {
				let (m, n) = (dimensions[0], dimensions[1]);
				let k = m.min(n);
				let image = checked_mul(m, n)?;
				let panels = checked_mul(checked_add(m, n)?, k)?;
				checked_add(
					checked_mul(checked_add(checked_add(image, panels)?, k)?, WORD_BYTES)?,
					WORD_BYTES,
				)?
			}
			Self::SplitKPartials => {
				let (m, k, n) = (dimensions[0], dimensions[1], dimensions[2]);
				let parts = if m == 0 {
					0
				} else {
					m.div_ceil(SPLIT_K_ROWS).clamp(1, MAX_SPLIT_K_PARTS)
				};
				checked_mul(checked_mul(parts, k)?, n)?
			}
		};
		Ok(WorkspaceValue {
			amount,
			unit: self.unit(),
		})
	}

	const fn dimension_count(self) -> usize {
		match self {
			Self::NoPersistentScratch { dimensions }
			| Self::FixedTreeReduction { dimensions, .. }
			| Self::FixedTreeScan { dimensions }
			| Self::StableSort { dimensions }
			| Self::SortRunEncoding { dimensions }
			| Self::MapThenReduction { dimensions }
			| Self::RandomKeySort { dimensions } => dimensions,
			Self::CholeskyFactor | Self::CholeskyInverse | Self::LuFactor | Self::SymmetricEigensolver => 1,
			Self::CholeskySolve | Self::LuSolve | Self::QrFactor | Self::SingularValueDecomposition => 2,
			Self::SplitKPartials => 3,
		}
	}
}

pub fn evaluate_workspace(descriptor: OperationDescriptor, dimensions: &[u64]) -> OperationResult<WorkspaceValue> {
	match descriptor.lowering {
		LoweringAvailability::Workspace(formula) => formula
			.evaluate(dimensions)
			.map_err(|error| error.for_operation(descriptor.id)),
		_ => Err(OperationError::new(
			OperationErrorKind::WrongLoweringKind,
			"operation does not own a static workspace formula",
		)
		.for_operation(descriptor.id)),
	}
}

fn reduction_scratch(mut width: u64, sequences: u64) -> OperationResult<u64> {
	let mut bytes = 0_u64;
	loop {
		let next = width.div_ceil(TREE_LANES);
		if next <= 1 {
			return Ok(bytes);
		}
		bytes = checked_add(
			bytes,
			checked_mul(checked_mul(sequences, next)?, WORD_BYTES)?,
		)?;
		width = next;
	}
}

fn scan_scratch(mut width: u64) -> OperationResult<u64> {
	let mut bytes = 0_u64;
	let mut level = 0_u32;
	loop {
		let blocks = width.div_ceil(TREE_LANES);
		if level > 0 {
			bytes = checked_add(bytes, checked_mul(width, WORD_BYTES)?)?;
		}
		if blocks <= 1 {
			return Ok(bytes);
		}
		bytes = checked_add(bytes, checked_mul(blocks, WORD_BYTES)?)?;
		width = blocks;
		level = level.checked_add(1).ok_or_else(workspace_overflow)?;
	}
}

fn sort_scratch(elements: u64) -> OperationResult<u64> {
	let padded = match elements {
		0 => 0,
		value => value
			.checked_next_power_of_two()
			.ok_or_else(workspace_overflow)?,
	};
	checked_mul(padded, WORD_BYTES * 2)
}

fn dense_image_with_fault(rows: u64, columns: u64) -> OperationResult<u64> {
	checked_add(
		checked_mul(checked_mul(rows, columns)?, WORD_BYTES)?,
		WORD_BYTES,
	)
}

fn checked_add(left: u64, right: u64) -> OperationResult<u64> {
	left.checked_add(right).ok_or_else(workspace_overflow)
}

fn checked_mul(left: u64, right: u64) -> OperationResult<u64> {
	left.checked_mul(right).ok_or_else(workspace_overflow)
}

fn workspace_overflow() -> OperationError {
	OperationError::new(
		OperationErrorKind::WorkspaceArithmeticOverflow,
		"workspace formula overflowed u64",
	)
}

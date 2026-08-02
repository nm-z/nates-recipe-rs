use crate::{
	CanonicalDTypeContract, LoweringAvailability, OperationDescriptor, OperationError, OperationErrorKind,
	OperationFamily, OperationResult, PrimitiveFamily,
};

/// The payload domains consumed or produced by a structured operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionPayload {
	F32,
	I32,
	F32AndI32,
	F32OrI32,
}

impl CompositionPayload {
	pub(crate) const fn dtype_contract(self) -> CanonicalDTypeContract {
		match self {
			Self::F32 => CanonicalDTypeContract::F32Payload,
			Self::I32 => CanonicalDTypeContract::I32Payload,
			Self::F32AndI32 => CanonicalDTypeContract::F32AndI32Payloads,
			Self::F32OrI32 => CanonicalDTypeContract::F32OrI32Payload,
		}
	}
}

/// A bound resolved during `prepare` from immutable tensor shapes or an
/// explicitly recorded operation parameter. No composition introduces a
/// data-dependent host loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationBound {
	Fixed(u32),
	ShapeExtent { axis: usize },
	MinimumShapeExtent,
	CeilingLog2ShapeExtent { axis: usize },
	PreparedParameter { name: &'static str },
}

/// One calculation step in an owned multi-stage operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionStep {
	Primitive {
		family: PrimitiveFamily,
		role: &'static str,
	},
	Repeat {
		bound: IterationBound,
		role: &'static str,
		body: &'static [CompositionStep],
	},
}

impl CompositionStep {
	#[must_use]
	pub const fn primitive(family: PrimitiveFamily, role: &'static str) -> Self { Self::Primitive { family, role } }

	#[must_use]
	pub const fn repeat(bound: IterationBound, role: &'static str, body: &'static [CompositionStep]) -> Self {
		Self::Repeat { bound, role, body }
	}

	#[must_use]
	pub const fn role(self) -> &'static str {
		match self {
			Self::Primitive { role, .. } | Self::Repeat { role, .. } => role,
		}
	}
}

/// A finite, backend-neutral algorithm assembled solely from Recipe-owned
/// scalar maps and the primitive families accepted by `recipe-language`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompositionRecipe {
	name: &'static str,
	definition: &'static str,
	steps: &'static [CompositionStep],
	payload: CompositionPayload,
	family: OperationFamily,
}

impl CompositionRecipe {
	const fn new(
		name: &'static str,
		definition: &'static str,
		steps: &'static [CompositionStep],
		payload: CompositionPayload,
		family: OperationFamily,
	) -> Self {
		Self {
			name,
			definition,
			steps,
			payload,
			family,
		}
	}

	#[must_use]
	pub const fn name(self) -> &'static str { self.name }

	#[must_use]
	pub const fn definition(self) -> &'static str { self.definition }

	#[must_use]
	pub const fn steps(self) -> &'static [CompositionStep] { self.steps }

	#[must_use]
	pub const fn payload(self) -> CompositionPayload { self.payload }

	#[must_use]
	pub const fn operation_family(self) -> OperationFamily { self.family }

	pub(crate) fn for_entry(symbol: &str, source: &str) -> Option<Self> { composition_for_entry(symbol, source) }

	/// Validate that the recipe is a finite, statically bounded composition of
	/// existing Recipe primitive families.
	pub fn validate(self) -> OperationResult<()> {
		if self.name.is_empty() || self.definition.is_empty() || self.steps.is_empty() {
			return Err(invalid_composition(
				"composition name, definition, and top-level step list must be nonempty",
			));
		}
		validate_steps(self.steps, 0)
	}
}

/// Validate a descriptor that owns a multi-stage composition.
pub fn validate_composition(descriptor: OperationDescriptor) -> OperationResult<()> {
	match descriptor.lowering {
		LoweringAvailability::Composition(recipe) => {
			recipe.validate()
				.map_err(|error| error.for_operation(descriptor.id))
		}
		_ => {
			Err(OperationError::new(
				OperationErrorKind::WrongLoweringKind,
				"operation does not own a multi-stage composition",
			)
			.for_operation(descriptor.id))
		}
	}
}

fn validate_steps(steps: &[CompositionStep], depth: usize) -> OperationResult<()> {
	if depth > 8 {
		return Err(invalid_composition(
			"composition nesting exceeds eight levels",
		));
	}
	for step in steps {
		if step.role().is_empty() {
			return Err(invalid_composition(
				"every composition step requires a role",
			));
		}
		match step {
			CompositionStep::Primitive { .. } => {}
			CompositionStep::Repeat { bound, body, .. } => {
				if body.is_empty() {
					return Err(invalid_composition("repeat body must be nonempty"));
				}
				match bound {
					IterationBound::Fixed(0) => {
						return Err(invalid_composition("fixed repeat bound must be nonzero"));
					}
					IterationBound::PreparedParameter { name: "" } => {
						return Err(invalid_composition(
							"prepared repeat parameter name must be nonempty",
						));
					}
					IterationBound::Fixed(_)
					| IterationBound::ShapeExtent { .. }
					| IterationBound::MinimumShapeExtent
					| IterationBound::CeilingLog2ShapeExtent { .. }
					| IterationBound::PreparedParameter { .. } => {}
				}
				validate_steps(body, depth + 1)?;
			}
		}
	}
	Ok(())
}

fn invalid_composition(detail: &'static str) -> OperationError {
	OperationError::new(OperationErrorKind::InvalidCompositionRecipe, detail)
}

const MAP: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Elementwise,
	"apply the operation-specific typed scalar SSA formula",
);
const REDUCE: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Reduce,
	"combine values with a fixed, recorded reduction tree",
);
const SCAN: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Scan,
	"apply a fixed-tree prefix recurrence",
);
const CONTRACT: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Contraction,
	"visit contracted coordinates in canonical order",
);
const GATHER: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Gather,
	"perform checked int32-indexed reads",
);
const SCATTER: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Scatter,
	"perform checked writes with an explicit conflict policy",
);
const HISTOGRAM: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Histogram,
	"accumulate into statically bounded bins",
);
const SORT: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Sort,
	"apply the stable total-order sorting network",
);
const RANDOM: CompositionStep = CompositionStep::primitive(
	PrimitiveFamily::Random,
	"generate counter-keyed Philox4x32-10 values",
);

const MAP_ONLY: &[CompositionStep] = &[MAP];
const MAP_REDUCE: &[CompositionStep] = &[MAP, REDUCE];
const MAP_REDUCE_MAP: &[CompositionStep] = &[MAP, REDUCE, MAP];
const PAIRWISE_L2_STEPS: &[CompositionStep] = &[MAP, REDUCE, CONTRACT, MAP];
const MAP_SCAN: &[CompositionStep] = &[MAP, SCAN];
const MAP_SORT: &[CompositionStep] = &[MAP, SORT];
const MAP_SORT_GATHER: &[CompositionStep] = &[MAP, SORT, GATHER];
const MAP_HISTOGRAM: &[CompositionStep] = &[MAP, HISTOGRAM];
const MAP_SCATTER: &[CompositionStep] = &[MAP, SCATTER];
const GATHER_MAP: &[CompositionStep] = &[GATHER, MAP];
const GATHER_MAP_REDUCE: &[CompositionStep] = &[GATHER, MAP, REDUCE];
const GATHER_MAP_SCATTER: &[CompositionStep] = &[GATHER, MAP, SCATTER];
const SORT_GATHER: &[CompositionStep] = &[SORT, GATHER];
const RANDOM_MAP: &[CompositionStep] = &[RANDOM, MAP];
const RANDOM_GATHER: &[CompositionStep] = &[RANDOM, GATHER];
const RANDOM_SORT_GATHER: &[CompositionStep] = &[RANDOM, SORT, GATHER];
const SORT_RANDOM_GATHER: &[CompositionStep] = &[SORT, RANDOM, GATHER];
const CONTRACT_MAP: &[CompositionStep] = &[CONTRACT, MAP];
const CONTRACT_MAP_REDUCE: &[CompositionStep] = &[CONTRACT, MAP, REDUCE];
const REDUCE_MAP: &[CompositionStep] = &[REDUCE, MAP];
const REDUCE_MAP_REDUCE: &[CompositionStep] = &[REDUCE, MAP, REDUCE];
const SCAN_MAP_SCATTER: &[CompositionStep] = &[SCAN, MAP, SCATTER];
const HISTOGRAM_REDUCE_MAP: &[CompositionStep] = &[HISTOGRAM, REDUCE, MAP];
const SORT_SCAN_MAP: &[CompositionStep] = &[SORT, SCAN, MAP];
const SORT_SCAN_SCATTER: &[CompositionStep] = &[SORT, SCAN, SCATTER];

const LINEAR_BACKWARD_FULL_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract the output gradient with the transposed weight to form the input gradient",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract the transposed input with the output gradient to form the weight gradient",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum the output gradient over rows with a fixed tree to form the bias gradient",
	),
];

const LINEAR_BACKWARD_WEIGHTS_ONLY_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract the transposed input with the output gradient to form the weight gradient",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum the output gradient over rows with a fixed tree to form the bias gradient",
	),
];

const SOFTMAX_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row maximum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"subtract the row maximum and apply the owned binary32 exponential",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row exponential sum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"divide each exponential by its checked nonzero row sum",
	),
];
const NORMALIZE_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form the operation-specific f32 statistic terms",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute the first fixed-order row statistic",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute the second fixed-order row statistic when required",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"normalize with the operation epsilon and apply affine parameters",
	),
];
const NORMALIZE_BACKWARD_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form the operation-specific f32 gradient statistic terms",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute the first fixed-order gradient statistic",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute the second fixed-order gradient statistic",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the analytic normalization gradient",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"reduce affine-parameter gradients in fixed order",
	),
];
const POOL_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"enumerate the checked pooling window",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"reduce each window in a fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the average divisor or maximum tie policy",
	),
];
const POOL_BACKWARD_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"reconstruct checked source-window coordinates",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute each source contribution",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"accumulate overlapping contributions with explicit atomic addition",
	),
];
const CONV_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"lower checked receptive fields to a static logical view",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract receptive fields and filters in canonical order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the optional bias or activation",
	),
];
const CONV_BACKWARD_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"lower the required checked gradient receptive fields",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract gradients in canonical order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"accumulate overlapping input-gradient contributions with explicit atomics",
	),
];
const ATTENTION_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"form scaled query-key scores in canonical order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the causal or supplied int32 mask",
	),
	CompositionStep::primitive(PrimitiveFamily::Reduce, "compute fixed-tree score maxima"),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"subtract maxima and apply the owned binary32 exponential",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute fixed-tree exponential sums",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"divide by checked nonzero exponential sums",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"contract normalized scores with values",
	),
];
const ATTENTION_BACKWARD_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"form value and probability gradients",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute fixed-tree softmax gradient statistics",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the softmax Jacobian and mask",
	),
	CompositionStep::primitive(PrimitiveFamily::Contraction, "form query gradients"),
	CompositionStep::primitive(PrimitiveFamily::Contraction, "form key gradients"),
	CompositionStep::primitive(PrimitiveFamily::Contraction, "form value gradients"),
];
const RNN_CELL_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"form input and recurrent affine projections",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply gates in the documented scalar order",
	),
];
const OPTIMIZER_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"update optimizer state with the named recurrence",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"update parameters after checked scalar normalization",
	),
];
const TREE_HISTOGRAM_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Histogram,
		"accumulate fixed-layout gradient statistics by bin",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scan,
		"form prefix statistics in fixed tree order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"score every legal split with the named objective",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select the maximum gain with lowest-index tie breaking",
	),
];
const TREE_ROUTE_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"read each row's checked feature and threshold",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute the deterministic int32 branch decision",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write the row or leaf assignment without conflicting writes",
	),
];
const SEGMENT_REDUCE_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Sort,
		"stably group values by int32 segment identifier",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scan,
		"mark deterministic segment boundaries",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"reduce each bounded segment with a fixed tree",
	),
];
const COUNT_DISTINCT_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Sort,
		"stably sort values by IEEE total order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"mark transitions between adjacent sorted values",
	),
	CompositionStep::primitive(PrimitiveFamily::Scan, "prefix-sum transition flags"),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write unique values and run counts to bounded outputs",
	),
];
const FFT_BUTTERFLY_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"read bit-reversed butterfly partners",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply owned sine/cosine twiddles and complex pair arithmetic",
	),
	CompositionStep::primitive(PrimitiveFamily::Scatter, "write disjoint butterfly outputs"),
];
const FFT_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::CeilingLog2ShapeExtent { axis: 0 },
	"execute every radix-2 Stockham stage",
	FFT_BUTTERFLY_BODY,
)];
const CHOLESKY_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"form the fixed-order diagonal or column dot product",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"check positivity and compute the pivot",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write the disjoint lower-triangular panel",
	),
];
const CHOLESKY_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::ShapeExtent { axis: 0 },
	"factor one matrix column per statically counted step",
	CHOLESKY_BODY,
)];
const TRIANGULAR_SOLVE_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"form the known-prefix dot product in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"check the diagonal and solve the next component",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write the disjoint solved component",
	),
];
const TRIANGULAR_SOLVE_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::ShapeExtent { axis: 0 },
	"perform statically bounded forward or backward substitution",
	TRIANGULAR_SOLVE_BODY,
)];
const LU_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select the absolute pivot with lowest-index tie breaking",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"apply the deterministic row permutation",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"check and scale the pivot column",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"update the trailing matrix in canonical order",
	),
];
const LU_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::MinimumShapeExtent,
	"factor one pivoted panel per statically counted step",
	LU_BODY,
)];
const QR_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute the Householder norm in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"construct the signed Householder reflector",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"apply the reflector in canonical order",
	),
];
const QR_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::MinimumShapeExtent,
	"apply one Householder reflector per statically counted step",
	QR_BODY,
)];
const JACOBI_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute the stable Jacobi rotation",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"apply the two-sided rotation in canonical order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"measure off-diagonal residual with a fixed tree",
	),
];
const EIGH_STEPS: &[CompositionStep] = &[
	CompositionStep::repeat(
		IterationBound::PreparedParameter {
			name: "jacobi_sweeps",
		},
		"execute the exact prepared Jacobi sweep count",
		JACOBI_BODY,
	),
	CompositionStep::primitive(
		PrimitiveFamily::Sort,
		"stable-sort eigenvalues and carry eigenvector columns with lowest-index ties",
	),
];
const SVD_STEPS: &[CompositionStep] = &[
	CompositionStep::repeat(
		IterationBound::MinimumShapeExtent,
		"bidiagonalize with statically counted Householder reflectors",
		QR_BODY,
	),
	CompositionStep::repeat(
		IterationBound::PreparedParameter {
			name: "bidiagonal_qr_sweeps",
		},
		"diagonalize the bidiagonal matrix with the prepared sweep count",
		JACOBI_BODY,
	),
];
const BORUVKA_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(PrimitiveFamily::Gather, "read checked component endpoints"),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select each component's minimum edge deterministically",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"union components with ordered atomic updates",
	),
];
const BORUVKA_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::CeilingLog2ShapeExtent { axis: 0 },
	"execute the statically bounded Boruvka contraction rounds",
	BORUVKA_BODY,
)];
const UNION_FIND_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(PrimitiveFamily::Gather, "follow checked parent indexes"),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"select the canonical minimum representative",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"publish parent compression with ordered atomics",
	),
];
const UNION_FIND_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::CeilingLog2ShapeExtent { axis: 0 },
	"execute the bounded pointer-jumping rounds",
	UNION_FIND_BODY,
)];
const DYNAMIC_PROGRAM_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(PrimitiveFamily::Gather, "read legal predecessor states"),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form the operation-specific transition score",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select or combine predecessors with fixed tie breaking",
	),
	CompositionStep::primitive(PrimitiveFamily::Scatter, "write the disjoint next state"),
];
const DYNAMIC_PROGRAM_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::PreparedParameter {
		name: "dynamic_program_steps",
	},
	"advance one statically counted dynamic-programming position",
	DYNAMIC_PROGRAM_BODY,
)];
const GENERATION_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"read checked model, token, and KV-state slices",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Contraction,
		"run model contractions in canonical order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply owned activation and normalization formulas",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select or score the next token with fixed tie breaking",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write the disjoint next token and KV-state slots within prepared bounds",
	),
];
const GENERATION_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::PreparedParameter {
		name: "maximum_generated_tokens",
	},
	"execute one prepared autoregressive decoding position",
	GENERATION_BODY,
)];
const BOOST_TRAIN_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute the source objective's f32 gradients and Hessians",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Histogram,
		"accumulate checked feature-bin statistics",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scan,
		"form deterministic prefix statistics",
	),
	CompositionStep::primitive(PrimitiveFamily::Elementwise, "score every legal split"),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select the lowest-index maximum-gain split",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"route rows through the accepted split",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Scatter,
		"write checked leaf assignments and updates",
	),
];
const BOOST_TRAIN_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::PreparedParameter {
		name: "boosting_rounds",
	},
	"execute one statically prepared boosting round",
	BOOST_TRAIN_BODY,
)];
const SMO_TRAIN_BODY: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute legal KKT violation scores",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select the deterministic working set",
	),
	CompositionStep::primitive(PrimitiveFamily::Gather, "read the selected kernel rows"),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"update pair coefficients and every gradient",
	),
];
const SMO_TRAIN_STEPS: &[CompositionStep] = &[CompositionStep::repeat(
	IterationBound::PreparedParameter {
		name: "smo_iterations",
	},
	"execute one statically prepared SMO iteration",
	SMO_TRAIN_BODY,
)];
const BITONIC_COMPARE_STEPS: &[CompositionStep] = &[CompositionStep::primitive(
	PrimitiveFamily::Sort,
	"execute exactly one prepared bitonic compare-exchange distance and merge-width level",
)];
const CROSS_ENTROPY_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row maximum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"subtract row maxima and apply the owned binary32 exponential",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row exponential sum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Gather,
		"read the checked int32 target logit",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form row log-sum-exp minus the selected target logit",
	),
];
const DISTANCE_LOSS_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form each squared feature difference",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each example's squared feature differences in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the operation-specific distance and margin formula per example",
	),
];
const TRIPLET_LOSS_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form anchor-positive and anchor-negative squared feature differences",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each anchor-positive distance in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each anchor-negative distance in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the prepared triplet margin branch per example",
	),
];
const COSINE_EMBEDDING_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form per-feature dot-product and squared-norm terms",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each example's dot-product terms in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each left input squared norm in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum each right input squared norm in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"apply the checked cosine and labeled margin formula per example",
	),
];
const DENSE_ARGMAX_ACCURACY_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select each prediction row argmax with lowest-index ties",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select each dense target row argmax with lowest-index ties",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compare the two canonical int32 class indexes",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum the per-row match flags in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"multiply the match count by the checked reciprocal row count",
	),
];
const LOG_SOFTMAX_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row maximum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"subtract row maxima and apply the owned binary32 exponential",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"compute each row exponential sum with a fixed tree",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"subtract max plus the owned logarithm of the checked row sum",
	),
];
const REPORT_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"select each row argmax with lowest-index ties",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Histogram,
		"accumulate checked class totals and correct-prediction counts",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"compute each defined class recall",
	),
	CompositionStep::primitive(PrimitiveFamily::Reduce, "sum class recalls in fixed order"),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"divide by the prepared class count",
	),
];
const LAMB_TRUST_STEPS: &[CompositionStep] = &[
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form squared parameter and direction values",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum squared parameter values in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Reduce,
		"sum squared direction values in fixed order",
	),
	CompositionStep::primitive(
		PrimitiveFamily::Elementwise,
		"form checked norms and trust ratio and update parameters",
	),
];

const fn recipe(
	name: &'static str,
	definition: &'static str,
	steps: &'static [CompositionStep],
	payload: CompositionPayload,
	family: OperationFamily,
) -> CompositionRecipe {
	CompositionRecipe::new(name, definition, steps, payload, family)
}

fn composition_for_entry(symbol: &str, source: &str) -> Option<CompositionRecipe> {
	let value = match symbol {
		"convert" | "dequant_f32" | "gpu_convert" => {
			recipe(
				"checked_dequantization",
				"decode the prepare-selected quantized representation with checked int32 fields and produce canonical f32 payload values",
				MAP_ONLY,
				CompositionPayload::F32OrI32,
				OperationFamily::Quantization,
			)
		}
		"generate" => {
			recipe(
				"bounded_autoregressive_generation",
				"run the prepared inference graph and counter-keyed or greedy token selection for the exact configured token bound",
				GENERATION_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Inference,
			)
		}
		"gpu_accuracy_into" => {
			recipe(
				"binary_accuracy",
				"threshold predictions and targets at canonical f32 0.5, sum int32 matches in a fixed tree, and multiply by the checked reciprocal element count",
				MAP_REDUCE_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_accuracy" => {
			recipe(
				"multiclass_correct_count",
				"select each prediction row argmax with lowest-index ties, compare with the checked int32 target, and return the fixed-order sum of matches",
				REDUCE_MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_argmax_accuracy_into" => {
			recipe(
				"dense_multiclass_accuracy",
				"select prediction and dense-target row argmaxes with lowest-index ties, compare them, sum matches in fixed order, and normalize by row count",
				DENSE_ARGMAX_ACCURACY_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_adagrad_update" => {
			recipe(
				"adagrad_update",
				"accumulate squared gradients and update parameters with the checked square-root denominator",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_adam_update" => {
			recipe(
				"adam_update",
				"update first and second moments, apply prepared bias corrections, and update parameters in documented scalar order",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_adamw_update" => {
			recipe(
				"adamw_update",
				"apply decoupled weight decay, update Adam moments, and update parameters in documented scalar order",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_add_col" => {
			recipe(
				"column_add",
				"gather the selected checked column, add the supplied f32 column elementwise, and scatter it into a copied matrix image",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_add_col_scaled_inplace" => {
			recipe(
				"scaled_column_accumulate",
				"gather the selected checked column, form matrix plus scale times column with explicit f32 operations, and scatter back in place",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_add_diag" => {
			recipe(
				"diagonal_add",
				"gather checked diagonal coordinates, add the supplied f32 diagonal value, and scatter to disjoint diagonal locations",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_argmax_write_split" => {
			recipe(
				"argmax_split_write",
				"reduce gain values to the lowest-index maximum and decode and write its feature and bin coordinates",
				REDUCE_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_argsort" => {
			recipe(
				"argsort",
				"stable-sort f32 values by IEEE total order while carrying original int32 indexes and expose the index result",
				SORT_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_avg_pool_1d" | "gpu_avg_pool_2d" | "gpu_avg_pool_2d_f32" => {
			recipe(
				"average_pool",
				"gather each checked pooling window, reduce it with a fixed sum tree, and divide by the documented window divisor",
				POOL_STEPS,
				CompositionPayload::F32,
				OperationFamily::Pooling,
			)
		}
		"gpu_avg_pool_2d_backward" | "gpu_avg_pool_2d_backward_f32" => {
			recipe(
				"average_pool_backward",
				"expand output gradients by the documented window divisor and atomically accumulate overlapping source contributions",
				POOL_BACKWARD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Pooling,
			)
		}
		"gpu_batchnorm_forward" => {
			recipe(
				"batch_normalization_training",
				"compute fixed-order channel mean and variance, normalize, and apply f32 scale and bias",
				NORMALIZE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_batchnorm_inference" => {
			recipe(
				"batch_normalization_inference",
				"normalize with prepared running statistics and apply f32 scale and bias",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_batchnorm_backward" => {
			recipe(
				"batch_normalization_backward",
				"compute fixed-order channel gradient statistics and the analytic input, scale, and bias gradients",
				NORMALIZE_BACKWARD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_bce_with_logits" => {
			recipe(
				"binary_cross_entropy_with_logits",
				"write max(logit,0)-logit*target+log1p(exp(-abs(logit))) and sigmoid(logit)-target per element",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_bernoulli_into" | "gpu_bernoulli_u8" => {
			recipe(
				"bernoulli_i32",
				"generate counter-keyed uniform f32 values and compare with the checked probability to produce canonical int32 masks",
				RANDOM_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Random,
			)
		}
		"gpu_bernoulli_nb_logprob" => {
			recipe(
				"bernoulli_naive_bayes_log_probability",
				"select each binary feature log probability, add its complement term, and reduce features in fixed order",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Bayesian,
			)
		}
		"gpu_bin_edges_quantile" => {
			recipe(
				"quantile_bin_edges",
				"stable-sort each feature by IEEE total order and gather the deterministic prepared quantile indexes",
				SORT_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::Encoding,
			)
		}
		"gpu_bin_edges_uniform" => {
			recipe(
				"uniform_bin_edges",
				"form every edge from the fixed-order feature minimum, maximum, bin index, and prepared bin count",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Encoding,
			)
		}
		"gpu_bitonic_step" | "gpu_bitonic_step_dd" | "gpu_bitonic_step_idx" => {
			recipe(
				"sorting_network_compare_exchange",
				"execute the named prepared bitonic compare-exchange level with IEEE total ordering and original-index tie breaking",
				BITONIC_COMPARE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_bn_update_running" => {
			recipe(
				"batch_normalization_running_update",
				"update running mean and variance as momentum*running+(1-momentum)*observed in explicit scalar order",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_bootstrap_sample" => {
			recipe(
				"bootstrap_sample",
				"generate counter-keyed uniform int32 row indexes and gather the corresponding rows with checked bounds",
				RANDOM_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_boruvka_mst" => {
			recipe(
				"boruvka_minimum_spanning_tree",
				"run the statically bounded Boruvka rounds with deterministic minimum-edge ties and ordered component unions",
				BORUVKA_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Clustering,
			)
		}
		"gpu_candidate_generate" => {
			recipe(
				"frequent_item_candidate_generation",
				"stably group the sorted itemsets, mark legal joins, prefix-sum the marks, and scatter candidates to bounded output slots",
				SORT_SCAN_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Other,
			)
		}
		"gpu_categorical_logprob" => {
			recipe(
				"categorical_log_probability",
				"gather the checked selected action probability and apply the owned binary32 logarithm",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ReinforcementLearning,
			)
		}
		"gpu_causal_softmax_rows" => {
			recipe(
				"causal_row_softmax",
				"apply the causal int32 mask then compute max-subtracted row softmax with fixed reduction trees",
				SOFTMAX_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_centroid_update" => {
			recipe(
				"centroid_update",
				"scatter-add rows into assigned centroid sums and counts, then divide nonempty sums by deterministic counts",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Clustering,
			)
		}
		"gpu_cholesky" => {
			recipe(
				"cholesky_factorization",
				"factor a positive-definite f32 matrix column by column with fixed-order dots and device-fault checks",
				CHOLESKY_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_cholesky_solve" | "gpu_potrs" => {
			recipe(
				"cholesky_solve",
				"solve against the Cholesky factors with statically bounded forward and backward substitution",
				TRIANGULAR_SOLVE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_cholesky_inv" => {
			recipe(
				"cholesky_inverse",
				"solve the Cholesky factors against the canonical identity columns and write the symmetric inverse",
				TRIANGULAR_SOLVE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_col2im_1d" | "gpu_col2im_2d" | "gpu_col2im_2d_ext" => {
			recipe(
				"column_to_image",
				"map checked lowered-column coordinates back to image coordinates and atomically add overlapping values",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Convolution,
			)
		}
		"gpu_concat_into" | "gpu_vconcat" => {
			recipe(
				"tensor_concatenation",
				"gather from the selected source view at each checked output coordinate and write disjoint output elements",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_contrastive_loss" => {
			recipe(
				"contrastive_loss",
				"compute each pair's fixed-order squared feature distance and write its documented positive or margin branch",
				DISTANCE_LOSS_STEPS,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_conv1d_into" => {
			recipe(
				"convolution_1d",
				"lower checked one-dimensional receptive fields, contract with filters, and apply the optional bias",
				CONV_STEPS,
				CompositionPayload::F32,
				OperationFamily::Convolution,
			)
		}
		"gpu_conv1d_backward_data_into" | "gpu_conv1d_backward_filter_into" | "gpu_conv1d_backward_bias_into" => {
			recipe(
				"convolution_1d_backward",
				"form the requested input, filter, or bias gradient with checked receptive fields and canonical contractions",
				CONV_BACKWARD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Convolution,
			)
		}
		"gpu_core_distance" => {
			recipe(
				"core_distance",
				"form checked pairwise distances, stable-select the prepared neighbor rank, and return its distance",
				MAP_SORT_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::Clustering,
			)
		}
		"gpu_cosine_embedding_loss" => {
			recipe(
				"cosine_embedding_loss",
				"compute each example's fixed-order dot product and norms and write its labeled cosine margin branch",
				COSINE_EMBEDDING_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Loss,
			)
		}
		"gpu_count_distinct" | "gpu_run_length" => {
			recipe(
				"stable_run_encoding",
				"stable-sort values, mark total-order transitions, prefix-sum run IDs, and scatter bounded run outputs",
				COUNT_DISTINCT_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Encoding,
			)
		}
		"gpu_cross_entropy" => {
			recipe(
				"cross_entropy",
				"compute max-subtracted row log-sum-exp, gather each checked int32 target logit, and write one negative log likelihood per row",
				CROSS_ENTROPY_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Loss,
			)
		}
		"gpu_csr_spmv" | "gpu_csr_spmm" => {
			recipe(
				"csr_sparse_contraction",
				"gather checked CSR column operands, multiply payload values, and reduce each statically bounded row in fixed order",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Graph,
			)
		}
		"gpu_data_partition" => {
			recipe(
				"stable_data_partition",
				"compute int32 partition predicates, prefix-sum each side, and scatter rows stably to disjoint destinations",
				SCAN_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_degree" => {
			recipe(
				"graph_degree",
				"histogram checked int32 edge endpoints into deterministic degree counts",
				MAP_HISTOGRAM,
				CompositionPayload::I32,
				OperationFamily::Graph,
			)
		}
		"gpu_diffusion_commit" => {
			recipe(
				"diffusion_commit",
				"select the accepted proposal and update chain state with the documented int32 gate",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Diffusion,
			)
		}
		"gpu_diffusion_sample" => {
			recipe(
				"diffusion_sample",
				"generate counter-keyed normal noise and apply the prepared reverse-diffusion mean and variance update",
				RANDOM_MAP,
				CompositionPayload::F32,
				OperationFamily::Diffusion,
			)
		}
		"gpu_discounted_returns" => {
			recipe(
				"discounted_returns",
				"apply the reverse affine recurrence return[t]=reward[t]+discount*return[t+1] with a fixed scan tree",
				MAP_SCAN,
				CompositionPayload::F32,
				OperationFamily::ReinforcementLearning,
			)
		}
		"gpu_dropout_u8_into" => {
			recipe(
				"canonical_i32_mask_dropout",
				"replace the legacy u8 mask with a canonical int32 mask and select zero or x*scale per element",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Random,
			)
		}
		"gpu_dasum" => {
			recipe(
				"canonical_f32_absolute_sum",
				"replace the prohibited legacy f64 path with f32 absolute values and a fixed-tree f32 sum",
				MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Reduction,
			)
		}
		"gpu_dgemv_into" => {
			recipe(
				"canonical_f32_matrix_vector_product",
				"replace the prohibited legacy f64 path with a canonical-order f32 matrix-vector contraction",
				CONTRACT_MAP,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_dger_into" => {
			recipe(
				"canonical_f32_rank_one_update",
				"replace the prohibited legacy f64 path with an outer-product contraction and explicit f32 matrix update",
				CONTRACT_MAP,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_dsyrk" => {
			recipe(
				"canonical_f32_symmetric_rank_k_update",
				"replace the prohibited legacy f64 path with a canonical f32 contraction and checked triangular scatter",
				CONTRACT_MAP,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_dtw" => {
			recipe(
				"dynamic_time_warping",
				"advance the bounded cost lattice by antidiagonal, combining the three legal predecessors with fixed ties",
				DYNAMIC_PROGRAM_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Distance,
			)
		}
		"gpu_eigh_sym" => {
			recipe(
				"symmetric_eigendecomposition",
				"execute the exact prepared Jacobi sweep count and sort eigenpairs deterministically",
				EIGH_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_embed_blend" => {
			recipe(
				"embedding_blend",
				"combine checked token embedding rows with the documented f32 blend weights",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Embedding,
			)
		}
		"gpu_embedding_backward" => {
			recipe(
				"embedding_backward",
				"scatter-add f32 row gradients to checked int32 token indexes with explicit atomic ordering",
				MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Embedding,
			)
		}
		"gpu_entropy_gated_step" => {
			recipe(
				"entropy_gated_diffusion_step",
				"compute entropy from prepared probabilities and select the documented diffusion state update with an int32 gate",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Diffusion,
			)
		}
		"gpu_eye" => {
			recipe(
				"identity_matrix",
				"map static row and column int32 coordinates to f32 one on the diagonal and positive zero elsewhere",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Creation,
			)
		}
		"gpu_feature_subset" => {
			recipe(
				"feature_subset",
				"gather the prepared checked int32 feature indexes from every row",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_fft_c2c_1d" | "gpu_rfft_1d" => {
			recipe(
				"stockham_fft_1d",
				"execute radix-2 Stockham stages over f32 real-imaginary pairs using owned twiddles and a fixed butterfly order",
				FFT_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Fft,
			)
		}
		"gpu_fill_sentinel" => {
			recipe(
				"sort_padding_fill",
				"write the prepared IEEE-total-order sentinel only to the statically padded tail",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Creation,
			)
		}
		"gpu_fixed_radius_neighbors" => {
			recipe(
				"fixed_radius_neighbors",
				"compute pairwise distances, mark radius-qualified pairs, prefix-sum bounded neighbor slots, and scatter checked indexes",
				SCAN_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Clustering,
			)
		}
		"gpu_flash_attention_into" | "gpu_flash_gqa" | "gpu_flash_mla" => {
			recipe(
				"tiled_online_attention",
				"compose canonical contractions with fixed-tree online softmax statistics and checked mask and head indexing",
				ATTENTION_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_flash_attention_train_into" => {
			recipe(
				"tiled_online_attention_training",
				"run tiled attention while retaining the fixed-order online-softmax statistics required by the prepared backward graph",
				ATTENTION_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_flash_attention_backward_into" => {
			recipe(
				"tiled_online_attention_backward",
				"form query, key, and value gradients through fixed-order online softmax and canonical contractions",
				ATTENTION_BACKWARD_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_focal_into" => {
			recipe(
				"focal_loss_and_gradient",
				"write the clamped binary focal loss and analytic gradient per element with owned log and pow functions",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_focal_grad_into" => {
			recipe(
				"focal_loss_gradient",
				"compute the clamped analytic focal-loss gradient and prepared inverse-count scaling per element",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_forward_backward" => {
			recipe(
				"forward_backward_sequence",
				"run fixed-order forward and reverse log-semiring scans over the statically bounded sequence lattice",
				DYNAMIC_PROGRAM_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Sequence,
			)
		}
		"gpu_gae" => {
			recipe(
				"generalized_advantage_estimation",
				"form temporal-difference residuals and apply the reverse affine GAE recurrence with a fixed scan tree",
				MAP_SCAN,
				CompositionPayload::F32,
				OperationFamily::ReinforcementLearning,
			)
		}
		"gpu_gated_delta_scan" => {
			recipe(
				"gated_delta_scan",
				"apply the documented associative gated-delta state transform with a fixed scan hierarchy",
				MAP_SCAN,
				CompositionPayload::F32,
				OperationFamily::StateSpace,
			)
		}
		"gpu_gaussian_ll" | "gpu_gaussian_logprob" => {
			recipe(
				"gaussian_log_probability",
				"evaluate the checked normal log-density from mean, log variance, and sample and reduce requested dimensions in fixed order",
				MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Bayesian,
			)
		}
		"gpu_gcn_norm" => {
			recipe(
				"gcn_edge_normalization",
				"gather checked endpoint degrees and compute reciprocal-square-root degree products for every edge",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Graph,
			)
		}
		"gpu_gemm_bt_tiles" => {
			recipe(
				"tiled_transposed_gemm",
				"execute the prepared tiled f32 contraction with canonical contracted-coordinate order",
				CONTRACT_MAP,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_goss_sample" => {
			recipe(
				"gradient_one_side_sampling",
				"stable-sort examples by absolute gradient, retain the prepared top fraction, and counter-sample the remainder",
				SORT_RANDOM_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_grad_clip_norm" => {
			recipe(
				"global_gradient_norm_clip",
				"sum squared gradients with a fixed tree, compute the checked clip scale, and scale every gradient",
				MAP_REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_grad_hess_into" | "gpu_logloss_grad_f32" | "gpu_logloss_grad_mc" => {
			recipe(
				"logistic_gradient_hessian",
				"compute canonical f32 logistic gradients and Hessians with checked int32 target and mask indexing",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_gru_cell" | "gpu_gru_cell_f32" => {
			recipe(
				"gru_cell",
				"form input and recurrent affine projections and apply reset, update, and candidate gates in documented order",
				RNN_CELL_STEPS,
				CompositionPayload::F32,
				OperationFamily::Recurrent,
			)
		}
		"gpu_has_nan" => {
			recipe(
				"any_nan",
				"map each f32 value through IsNan and combine int32 flags with a fixed-tree Any reduction",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_isfinite_all" => {
			recipe(
				"all_finite",
				"map each f32 value through IsFinite and combine int32 flags with a fixed-tree All reduction",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_hinge_loss" => {
			recipe(
				"hinge_loss",
				"write max(0,1-target*score) and its documented subgradient per element",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_histogram_build" => {
			recipe(
				"gradient_hessian_histogram",
				"accumulate masked f32 gradient, Hessian, and int32 count statistics into checked feature bins",
				MAP_HISTOGRAM,
				CompositionPayload::F32AndI32,
				OperationFamily::Histogram,
			)
		}
		"gpu_im2col_1d" | "gpu_im2col_2d" | "gpu_im2col_2d_ext" => {
			recipe(
				"image_to_column",
				"gather every checked receptive-field coordinate into the statically shaped lowered image",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Convolution,
			)
		}
		"gpu_idamax" => {
			recipe(
				"canonical_f32_absolute_argmax",
				"replace the prohibited legacy f64 path by mapping f32 absolute values and selecting the lowest-index fixed-tree maximum",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Reduction,
			)
		}
		"gpu_init_idx" | "gpu_iota" => {
			recipe(
				"int32_iota",
				"convert each statically generated linear lane index to checked canonical int32",
				MAP_ONLY,
				CompositionPayload::I32,
				OperationFamily::Creation,
			)
		}
		"gpu_itemset_support" => {
			recipe(
				"itemset_support",
				"mark transaction matches for each checked candidate and reduce support counts with fixed trees",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Other,
			)
		}
		"gpu_kernel_matrix" | "gpu_smo_kernel_row" => {
			recipe(
				"svm_kernel_matrix",
				"compute the configured linear, polynomial, or RBF kernel from fixed-order dot products and owned scalar math",
				CONTRACT_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::SupportVectorMachine,
			)
		}
		"gpu_kl_div_loss" => {
			recipe(
				"kl_divergence_loss",
				"write target*(log(target)-log_probability) per positive-target element and zero otherwise",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_l2_norm" => {
			recipe(
				"l2_norm",
				"square f32 values, sum with a fixed tree, and apply the owned square root",
				MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Reduction,
			)
		}
		"gpu_l2norm_rows" => {
			recipe(
				"row_l2_normalization",
				"compute each fixed-tree row sum of squares and multiply by its checked reciprocal square root",
				MAP_REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_lamb_phase1" => {
			recipe(
				"lamb_moment_update",
				"update Adam moments and form the prepared bias-corrected, decay-adjusted LAMB direction",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_lamb_phase2" => {
			recipe(
				"lamb_trust_ratio_update",
				"compute fixed-tree parameter and direction norms, form the checked trust ratio, and update parameters",
				LAMB_TRUST_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_layernorm_f32" | "gpu_layernorm_into" | "gpu_layernorm_opt_into" => {
			recipe(
				"layer_normalization",
				"compute fixed-order row mean and variance, normalize with epsilon, and apply f32 scale and bias",
				NORMALIZE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_layernorm_backward_f32" | "gpu_layernorm_backward_full_into" => {
			recipe(
				"layer_normalization_backward",
				"compute fixed-order gradient statistics and analytic input, scale, and bias gradients",
				NORMALIZE_BACKWARD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_leaf_finalize" | "gpu_lgbm_leaf_reduce" => {
			recipe(
				"regularized_leaf_value",
				"reduce each leaf's gradient and Hessian in fixed order and compute the checked regularized leaf value",
				HISTOGRAM_REDUCE_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_leaf_reduce" | "gpu_scatter_add_by_leaf" | "gpu_scatter_add_by_leaf_col" => {
			recipe(
				"leaf_statistic_accumulation",
				"scatter-add f32 statistics into checked int32 leaf slots with explicit atomic ordering",
				MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_leaf_split_apply" => {
			recipe(
				"leaf_split_apply",
				"gather the selected feature, evaluate the split predicate, and write the next checked leaf assignment",
				TREE_ROUTE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_lgbm_best_split" | "gpu_oblivious_split_eval" | "gpu_split_eval" | "gpu_tb_split_eval" => {
			recipe(
				"best_histogram_split",
				"prefix accumulated gradient statistics, score every legal split, and select the lowest-index maximum gain",
				TREE_HISTOGRAM_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_lgbm_hist_subtract" => {
			recipe(
				"histogram_subtraction",
				"subtract a child histogram from its parent elementwise in canonical f32 and int32 domains",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_lgbm_histogram" | "gpu_oblivious_histogram" | "gpu_tb_histogram" => {
			recipe(
				"tree_gradient_histogram",
				"accumulate checked gradient, Hessian, and count statistics by leaf, feature, and bin",
				MAP_HISTOGRAM,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_linear_into" | "gpu_linear_f32" | "gpu_matvec_bias_into" => {
			recipe(
				"linear_projection",
				"perform the canonical f32 contraction and add the prepared bias with explicit elementwise order",
				CONTRACT_MAP,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_linear_backward_full_into" => {
			recipe(
				"linear_projection_backward",
				"form input and weight gradients with canonical-order f32 contractions and the bias gradient with a fixed-tree row sum",
				LINEAR_BACKWARD_FULL_STEPS,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_linear_backward_weights_only_into" => {
			recipe(
				"linear_projection_backward_weights_only",
				"form the weight gradient with a canonical-order f32 contraction and the bias gradient with a fixed-tree row sum",
				LINEAR_BACKWARD_WEIGHTS_ONLY_STEPS,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_lion_update" => {
			recipe(
				"lion_update",
				"update momentum, apply the sign direction and decoupled weight decay, then update the stored momentum",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_log_det_cholesky" => {
			recipe(
				"cholesky_log_determinant",
				"gather checked diagonal values, apply owned logarithms, sum in fixed order, and multiply by two",
				GATHER_MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Solver,
			)
		}
		"gpu_log_softmax_rows" => {
			recipe(
				"row_log_softmax",
				"compute max-subtracted row log-sum-exp with fixed trees and subtract it from each row value",
				LOG_SOFTMAX_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_log_sum_exp_rows" => {
			recipe(
				"row_log_sum_exp",
				"compute each row maximum and exponential sum with fixed trees, then add the owned logarithm",
				REDUCE_MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Reduction,
			)
		}
		"gpu_lstm_cell" | "gpu_lstm_cell_f32" => {
			recipe(
				"lstm_cell",
				"form input and recurrent affine projections and apply input, forget, output, and cell gates in documented order",
				RNN_CELL_STEPS,
				CompositionPayload::F32,
				OperationFamily::Recurrent,
			)
		}
		"gpu_lu_factor" => {
			recipe(
				"lu_factorization",
				"perform deterministic partial pivoting and canonical trailing-matrix updates for every prepared panel",
				LU_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Solver,
			)
		}
		"gpu_lu_solve" | "gpu_solve" => {
			recipe(
				"pivoted_lu_solve",
				"gather the recorded pivot permutation and perform bounded forward and backward substitution",
				TRIANGULAR_SOLVE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Solver,
			)
		}
		"gpu_max_pool_1d" | "gpu_max_pool_2d" | "gpu_max_pool_2d_f32" => {
			recipe(
				"maximum_pool",
				"gather each checked pooling window and select its fixed-order maximum with lowest-coordinate ties",
				POOL_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"recipe_max_pool_1d" => {
			recipe(
				"channelwise_maximum_pool_1d",
				"gather non-overlapping channelwise windows, retain the final short window, and select the lowest global coordinate on ties",
				POOL_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"recipe_max_pool_1d_backward" => {
			recipe(
				"channelwise_maximum_pool_1d_backward",
				"route each output gradient to its recorded global winner through a checked unique scatter",
				POOL_BACKWARD_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"gpu_max_pool_1d_backward" | "gpu_max_pool_2d_backward" | "gpu_max_pool_2d_backward_f32" => {
			recipe(
				"maximum_pool_backward",
				"recompute the deterministic winning source coordinate and atomically accumulate overlapping gradients",
				POOL_BACKWARD_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"gpu_mean_all" => {
			recipe(
				"global_mean",
				"sum every f32 value with a fixed tree and divide by the checked prepared element count",
				REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Statistics,
			)
		}
		"gpu_mha_split" => {
			recipe(
				"multi_head_attention_split",
				"gather the prepared query, key, and value head views with checked int32 coordinates",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_mha_merge" => {
			recipe(
				"multi_head_attention_merge",
				"gather prepared head views in canonical head order and write the merged tensor",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_moe_route" => {
			recipe(
				"mixture_of_experts_route",
				"stable-select the prepared top experts per token, normalize their gates, and emit checked int32 routes",
				SORT_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::MixtureOfExperts,
			)
		}
		"gpu_moe_weighted_accumulate" => {
			recipe(
				"mixture_of_experts_accumulate",
				"gather checked expert outputs, multiply by normalized gates, and sum experts in fixed order",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::MixtureOfExperts,
			)
		}
		"gpu_moe_weighted_accumulate_backward" | "gpu_moe_backward" => {
			recipe(
				"mixture_of_experts_backward",
				"gather checked routes and form expert, gate, and token gradients with explicit atomic accumulation",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::MixtureOfExperts,
			)
		}
		"gpu_momentum_update" => {
			recipe(
				"momentum_update",
				"update the velocity from momentum and gradient, then update parameters in documented scalar order",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_mse_into" => {
			recipe(
				"mean_squared_error",
				"square prediction minus target per element, sum with a fixed reduction tree, and multiply by the checked reciprocal element count",
				MAP_REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_ss_res_into" => {
			recipe(
				"sum_squared_residuals",
				"square target minus prediction per element and sum with a fixed reduction tree",
				MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_multinomial_nb_logprob" => {
			recipe(
				"multinomial_naive_bayes_log_probability",
				"multiply feature counts by class log probabilities and reduce features in fixed order",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Bayesian,
			)
		}
		"gpu_nadam_update" => {
			recipe(
				"nadam_update",
				"update Adam moments, form the Nesterov-corrected first moment, and update parameters",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_nb_count_table" => {
			recipe(
				"naive_bayes_count_table",
				"accumulate checked class and feature int32 indexes into f32 class-feature counts with explicit atomics",
				MAP_HISTOGRAM,
				CompositionPayload::F32AndI32,
				OperationFamily::Bayesian,
			)
		}
		"gpu_nb_feature_log_prob" => {
			recipe(
				"naive_bayes_feature_log_probability",
				"apply prepared smoothing, reduce class totals in fixed order, and subtract owned logarithms",
				REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Bayesian,
			)
		}
		"gpu_neighbor_aggregate" => {
			recipe(
				"graph_neighbor_aggregation",
				"gather checked neighbor features and reduce each destination neighborhood in fixed order",
				GATHER_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Graph,
			)
		}
		"gpu_oblivious_route_full"
		| "gpu_oblivious_route_step"
		| "gpu_oblivious_route_step_dev"
		| "gpu_tree_build_into" => {
			recipe(
				"oblivious_tree_route",
				"evaluate the prepared feature-threshold predicates in level order and write checked int32 leaf indexes",
				TREE_ROUTE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_one_hot" => {
			recipe(
				"one_hot_encoding",
				"clear the statically shaped output and scatter f32 one to each checked int32 category coordinate",
				MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Encoding,
			)
		}
		"gpu_oob_mask" => {
			recipe(
				"out_of_bag_mask",
				"mark rows absent from the bounded bootstrap index image using checked int32 histogram counts",
				MAP_HISTOGRAM,
				CompositionPayload::I32,
				OperationFamily::Tree,
			)
		}
		"gpu_ordered_target_stats" => {
			recipe(
				"ordered_target_statistics",
				"stable-sort by permutation and category, run fixed-tree prefix sums and counts, and gather leave-current-out statistics",
				SORT_SCAN_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_pack_upper_tri" => {
			recipe(
				"pack_upper_triangle",
				"gather checked upper-triangular matrix coordinates in canonical row-major packed order",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_pairwise_cosine" => {
			recipe(
				"pairwise_cosine_distance",
				"compute fixed-order pairwise dot products and norms and form one minus the checked cosine similarity",
				CONTRACT_MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Distance,
			)
		}
		"gpu_pairwise_hamming" => {
			recipe(
				"pairwise_hamming_distance",
				"compare checked pair coordinates, convert mismatches to int32, and reduce feature counts in fixed order",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Distance,
			)
		}
		"gpu_pairwise_l1" => {
			recipe(
				"pairwise_l1_distance",
				"form absolute pairwise differences and reduce features with a fixed tree",
				MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Distance,
			)
		}
		"gpu_pairwise_l2" => {
			recipe(
				"pairwise_l2_distance",
				"square pairwise differences, sum features with fixed trees, and apply the owned square root",
				PAIRWISE_L2_STEPS,
				CompositionPayload::F32,
				OperationFamily::Distance,
			)
		}
		"gpu_partial_argsort" | "gpu_topk_per_row" => {
			recipe(
				"bounded_topk_indexes",
				"stable-sort by IEEE total order with original-index ties and gather the prepared bounded prefix per row",
				SORT_GATHER,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_pool_grad_expand" => {
			recipe(
				"pool_gradient_expand",
				"gather output gradients for each checked source window and atomically accumulate overlapping contributions",
				POOL_BACKWARD_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"gpu_positional_encoding" => {
			recipe(
				"sinusoidal_positional_encoding",
				"compute prepared inverse-frequency angles and apply owned sine and cosine by static position and channel indexes",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::Embedding,
			)
		}
		"gpu_qr" => {
			recipe(
				"householder_qr",
				"apply one fixed-order Householder reflector per statically counted matrix column",
				QR_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_quantize_features" => {
			recipe(
				"feature_bin_quantization",
				"binary-search prepared f32 edges with fixed iterations and emit checked int32 bin indexes",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Quantization,
			)
		}
		"gpu_random_permutation" => {
			recipe(
				"counter_keyed_permutation",
				"generate Philox keys, stable-sort key-index pairs, and expose the resulting unbiased int32 permutation",
				RANDOM_SORT_GATHER,
				CompositionPayload::I32,
				OperationFamily::Random,
			)
		}
		"gpu_random_threshold_split" => {
			recipe(
				"random_threshold_split",
				"generate counter-keyed feature and threshold choices from prepared bounds and emit checked split metadata",
				RANDOM_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_reduce_mean_cols" => {
			recipe(
				"column_mean",
				"sum each column with a fixed tree and divide by the checked prepared row count",
				REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Statistics,
			)
		}
		"gpu_reduce_var_cols" => {
			recipe(
				"column_variance",
				"compute fixed-tree column means then fixed-tree squared-deviation sums with the prepared divisor",
				REDUCE_MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Statistics,
			)
		}
		"gpu_repeat_rows" => {
			recipe(
				"repeat_rows",
				"gather each output row from its checked modulo source-row coordinate",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_report" => {
			recipe(
				"balanced_recall_report_metric",
				"select row argmax classes, histogram checked targets and correct predictions, compute class recalls, and reduce them",
				REPORT_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Metric,
			)
		}
		"gpu_rmsnorm" | "gpu_rmsnorm_f64" | "gpu_rmsnorm_f64_nogamma" => {
			recipe(
				"canonical_f32_rms_normalization",
				"replace any legacy f64 path with a fixed-tree f32 mean square, checked reciprocal square root, and optional scale",
				NORMALIZE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_rmsnorm_backward" => {
			recipe(
				"rms_normalization_backward",
				"compute the fixed-order RMS gradient statistic and analytic input and optional scale gradients",
				NORMALIZE_BACKWARD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_rmsprop_update" => {
			recipe(
				"rmsprop_update",
				"update the squared-gradient average and apply the checked reciprocal-root parameter update",
				OPTIMIZER_STEPS,
				CompositionPayload::F32,
				OperationFamily::Optimizer,
			)
		}
		"gpu_rope"
		| "gpu_rope_partial"
		| "gpu_rope_partial_factors"
		| "gpu_rope_partial_factors_pos"
		| "gpu_rope_partial_pos"
		| "gpu_rope_qk"
		| "gpu_rope_qk_heads_inplace" => {
			recipe(
				"rotary_position_embedding",
				"gather each checked f32 coordinate pair and apply the prepared owned sine-cosine rotation",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Embedding,
			)
		}
		"gpu_scaled_dot_product_attn" => {
			recipe(
				"scaled_dot_product_attention",
				"form scaled query-key scores, apply the checked mask, compute fixed-tree softmax, and contract with values",
				ATTENTION_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Attention,
			)
		}
		"gpu_scan_linear_recurrence" => {
			recipe(
				"affine_linear_recurrence_scan",
				"compose affine recurrence transforms with a fixed scan hierarchy and apply them to the initial state",
				MAP_SCAN,
				CompositionPayload::F32,
				OperationFamily::Scan,
			)
		}
		"gpu_segment_max" | "gpu_segment_sum" => {
			recipe(
				"segmented_reduction",
				"stable-group checked int32 segment IDs and reduce each bounded segment with a fixed tree",
				SEGMENT_REDUCE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Reduction,
			)
		}
		"gpu_segment_sort" => {
			recipe(
				"segmented_sort",
				"stable-sort within each prepared bounded segment by IEEE total order and original-index ties",
				MAP_SORT,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_slice_cols" | "gpu_slice_rows" | "gpu_slice_lead_into" => {
			recipe(
				"checked_tensor_slice",
				"gather the statically shaped slice with checked int32 start and extent coordinates",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_smo_argmax" => {
			recipe(
				"smo_working_set_argmax",
				"select the lowest-index maximum legal SMO KKT score with a fixed value-index reduction",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::SupportVectorMachine,
			)
		}
		"gpu_smo_kkt_score" => {
			recipe(
				"smo_kkt_score",
				"compute the checked box-constraint eligibility and KKT violation score per training row",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::SupportVectorMachine,
			)
		}
		"gpu_smo_update_gradient_rows" => {
			recipe(
				"smo_gradient_update",
				"gather the two selected kernel rows and update every gradient in explicit f32 order",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::SupportVectorMachine,
			)
		}
		"gpu_smo_train" => {
			recipe(
				"bounded_smo_training",
				"execute the exact prepared SMO iteration bound using deterministic working-set selection and gradient updates",
				SMO_TRAIN_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::SupportVectorMachine,
			)
		}
		"gpu_softmax_rows_into" | "gpu_softmax_inplace" => {
			recipe(
				"row_softmax",
				"compute max-subtracted row exponentials and fixed-tree sums, then divide by checked row sums",
				SOFTMAX_STEPS,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_softmax_backward_into" => {
			recipe(
				"softmax_backward",
				"compute the fixed-tree dot of output and upstream gradient and apply the row softmax Jacobian",
				REDUCE_MAP,
				CompositionPayload::F32,
				OperationFamily::Normalization,
			)
		}
		"gpu_softmax_ce_class_grad_f32" | "gpu_softmax_ce_grad_into" => {
			recipe(
				"softmax_cross_entropy_gradient",
				"compute fixed-tree row softmax and subtract the checked int32 target indicator with prepared scaling",
				SOFTMAX_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Loss,
			)
		}
		"gpu_sort_by_key" => {
			recipe(
				"stable_key_value_sort",
				"stable-sort f32 keys by IEEE total order while carrying values and original indexes through every compare exchange",
				MAP_SORT,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_splitk_dw_into" => {
			recipe(
				"split_k_weight_gradient",
				"form at most eight deterministic row-slice contractions and reduce their f32 partials in slice-index order",
				CONTRACT_MAP_REDUCE,
				CompositionPayload::F32,
				OperationFamily::Contraction,
			)
		}
		"gpu_ssm_conv_causal" | "gpu_ssm_conv_causal_silu" => {
			recipe(
				"state_space_causal_convolution",
				"gather the checked causal window, contract in canonical order, and apply the optional owned SiLU",
				CONV_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::StateSpace,
			)
		}
		"gpu_ssm_group_rmsnorm" => {
			recipe(
				"state_space_group_rms_normalization",
				"compute fixed-tree group mean squares, apply checked reciprocal roots, and scale each group",
				NORMALIZE_STEPS,
				CompositionPayload::F32,
				OperationFamily::StateSpace,
			)
		}
		"gpu_ssm_scan_mamba1" | "gpu_ssm_scan_mamba2" => {
			recipe(
				"mamba_state_space_scan",
				"compose the documented diagonal state-transition transforms with a fixed scan hierarchy",
				MAP_SCAN,
				CompositionPayload::F32,
				OperationFamily::StateSpace,
			)
		}
		"gpu_svd" => {
			recipe(
				"singular_value_decomposition",
				"bidiagonalize with fixed-order Householder reflectors and run the exact prepared diagonalization sweep count",
				SVD_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_tb_apply_tree" | "gpu_tree_ensemble_predict" => {
			recipe(
				"tree_ensemble_inference",
				"traverse prepared trees with checked feature gathers and deterministic predicates, then reduce tree outputs",
				TREE_ROUTE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_tb_leaf_sum" | "gpu_tb_leaf_val" => {
			recipe(
				"tree_builder_leaf_statistic",
				"accumulate checked per-leaf statistics and compute the documented regularized f32 leaf value",
				HISTOGRAM_REDUCE_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_tb_repartition" | "gpu_tb_scatter" => {
			recipe(
				"tree_builder_stable_partition",
				"evaluate the selected split, prefix-sum branch flags, and scatter rows stably to disjoint partitions",
				SCAN_MAP_SCATTER,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"gpu_td_targets" => {
			recipe(
				"temporal_difference_targets",
				"gather the checked next-state values and compute reward+discount*value under the terminal int32 mask",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ReinforcementLearning,
			)
		}
		"gpu_transpose" => {
			recipe(
				"tensor_transpose",
				"gather every output value from its checked statically permuted source coordinate",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_tri_solve" => {
			recipe(
				"triangular_solve",
				"perform statically bounded forward or backward substitution with fixed-order dot products and diagonal checks",
				TRIANGULAR_SOLVE_STEPS,
				CompositionPayload::F32,
				OperationFamily::Solver,
			)
		}
		"gpu_tril_mask" => {
			recipe(
				"lower_triangular_mask",
				"compare static row and column indexes and select the supplied f32 fill value outside the retained triangle",
				MAP_ONLY,
				CompositionPayload::F32AndI32,
				OperationFamily::ShapeAndIndexing,
			)
		}
		"gpu_triplet_loss" => {
			recipe(
				"triplet_margin_loss",
				"compute each example's fixed-order anchor-positive and anchor-negative squared distances and write its margin branch",
				TRIPLET_LOSS_STEPS,
				CompositionPayload::F32,
				OperationFamily::Loss,
			)
		}
		"gpu_union_find_cc" => {
			recipe(
				"union_find_connected_components",
				"execute bounded ordered unions and pointer-jumping rounds with the minimum int32 representative",
				UNION_FIND_STEPS,
				CompositionPayload::I32,
				OperationFamily::Clustering,
			)
		}
		"gpu_upsample_nearest_2d" => {
			recipe(
				"nearest_neighbor_upsample",
				"map each output coordinate to its checked integer source coordinate and gather the source value",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Pooling,
			)
		}
		"gpu_vae_backward_latent" => {
			recipe(
				"vae_latent_backward",
				"apply the analytic reparameterization and KL gradients to mean and log variance in explicit scalar order",
				MAP_ONLY,
				CompositionPayload::F32,
				OperationFamily::Diffusion,
			)
		}
		"gpu_viterbi" => {
			recipe(
				"viterbi_decode",
				"advance the bounded max-sum lattice with lowest-index predecessor ties and backtrack checked int32 states",
				DYNAMIC_PROGRAM_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Sequence,
			)
		}
		"gpu_write_split" => {
			recipe(
				"write_split_metadata",
				"write the prepared checked int32 feature and bin identifiers to their disjoint tree-depth slot",
				MAP_ONLY,
				CompositionPayload::I32,
				OperationFamily::Tree,
			)
		}
		_ => return source_specific_composition(symbol, source),
	};
	Some(value)
}

fn source_specific_composition(symbol: &str, source: &str) -> Option<CompositionRecipe> {
	let value = match symbol {
		"greedy" | "greedy_windowed" => {
			recipe(
				"greedy_token_selection",
				"select the lowest-index maximum f32 logit over the prepared full or windowed int32 token domain",
				MAP_REDUCE,
				CompositionPayload::F32AndI32,
				OperationFamily::Inference,
			)
		}
		"last_logits" => {
			recipe(
				"last_token_logits",
				"gather the statically checked final sequence position without host payload slicing",
				GATHER_MAP,
				CompositionPayload::F32AndI32,
				OperationFamily::Inference,
			)
		}
		"predict" | "predict_proba" => {
			recipe(
				match source {
					value if value.contains("catboost") => "catboost_inference",
					value if value.contains("lightgbm") => "lightgbm_inference",
					value if value.contains("xgboost") => "xgboost_inference",
					_ => return None,
				},
				"traverse the source model's prepared tree representation with checked feature indexes and fixed-order ensemble accumulation",
				TREE_ROUTE_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		"train" | "train_multiclass" => {
			recipe(
				match source {
					value if value.contains("catboost") => "catboost_training",
					value if value.contains("lightgbm") => "lightgbm_training",
					value if value.contains("xgboost") => "xgboost_training",
					_ => return None,
				},
				"execute the exact prepared boosting-round count using owned gradients, histograms, split selection, routing, and leaf updates",
				BOOST_TRAIN_STEPS,
				CompositionPayload::F32AndI32,
				OperationFamily::Tree,
			)
		}
		_ => return None,
	};
	Some(value)
}

# Scalar payloads, programs, and execution

This document is the contract for the scalar layer rooted at
[`core/src/scalar.rs`](../../src/scalar.rs). A scalar program is the complete
per-element calculation payload of one kernel. It is an acyclic, typed SSA
program. It does not describe tensor placement, transfers, queues, device
selection, scheduling, or lifecycle state. Those concerns consume the scalar
program after language validation and primitive lowering.

The public type domain is deliberately small:

| Type | Representation | Width |
| --- | --- | --- |
| `DType::F32` | IEEE binary32 payload | 4 bytes |
| `DType::I32` | signed 32-bit integer payload | 4 bytes |

`DType::byte_width` is four for both variants. There is no implicit promotion,
half precision type, boolean payload type, pointer type, or host scalar type in
this contract. Boolean results are represented by `DType::I32`, with the
runtime convention that predicates produce exactly zero or one. A nonzero I32
is accepted as true by `Select` and `Require`.

## The scalar data model

`ScalarValueId` is an opaque `u64` identity from `core/src/ids.rs`. It is local
to a `ScalarProgram`; it is not a tensor `ValueId`, a kernel input ID, or a
kernel output ID. The distinction matters when a scalar program is embedded in
a `KernelTemplate` and later mapped to resident arena values.

The four program records are intentionally plain and serializable:

| Record | Fields | Meaning |
| --- | --- | --- |
| `ScalarInput` | `id`, `dtype` | A value supplied once per logical lane by an input buffer. |
| `ScalarConstant` | `id`, `value` | A lane-invariant literal. `F32Bits(u32)` preserves every binary32 bit pattern; `I32(i32)` stores an integer directly. |
| `ScalarInstruction` | `result`, `dtype`, `opcode`, `operands` | One instruction in program order. Its result is defined after all earlier definitions. |
| `ScalarProgram` | `inputs`, `constants`, `instructions`, `outputs` | The complete ordered program and one or more values to store as kernel outputs. |

The program has one value namespace across inputs, constants, and instruction
results. A value cannot be redefined. Instruction order is semantic, not an
optimization hint: it is retained by OGDL encoding, included in planner
digests, and determines the resulting native artifact identity. The core type
does not require IDs to be nonzero, and it does not reject duplicate entries
in `outputs`; the language builders start at one and generate unique IDs, and
all normal callers finish through validation.

`ScalarLiteral::F32Bits` is used instead of an `f32` field so NaN payloads,
signed zero, infinities, and subnormal bit patterns survive construction,
serialization, and lowering. The language convenience method `f32` converts
with `to_bits`; `i32` creates an `I32` literal.

## Opcode set

`ScalarOpcode` is `#[non_exhaustive]`. Its current complete set is listed
below. Arity is enforced by `ScalarOpcode::arity`, and the result type is
resolved by `result_dtype` before an instruction is appended.

### Typed arithmetic

| Opcode | Arity and type rule | Payload operation |
| --- | --- | --- |
| `Add`, `Subtract`, `Multiply` | 2, both operands have the same type, result has that type | Typed binary arithmetic. The LLVM lowering emits plain I32 arithmetic for I32 and constrained round-to-nearest f32 arithmetic for F32. |
| `Divide` | 2, same type, result has that type | F32 uses IEEE division. I32 uses checked truncating signed division. A zero divisor or `i32::MIN / -1` is a rejected calculation and produces a safe zero result before publishing a fault. |
| `Remainder` | 2, same type, result has that type | F32 uses IEEE remainder. I32 uses checked truncating signed remainder with the same zero-divisor and `i32::MIN / -1` rejection rules. |
| `Negate` | 1, F32 or I32 | F32 flips the sign bit. I32 is checked for `i32::MIN`; the rejected result is zero. |
| `Absolute` | 1, F32 or I32 | F32 clears the sign bit. I32 is checked for `i32::MIN`; the rejected result is zero. |
| `Minimum`, `Maximum` | 2, same type, result has that type | F32 uses the LLVM minimum or maximum intrinsic and canonicalizes a NaN result. I32 uses signed comparison and selection. |
| `Fma` | 3, all operands F32, result F32 | One-rounding fused multiply-add. The scheduler prices it as two FLOPs. |

The core type does not attach a fault flag to F32 divide, remainder, negate, or
absolute. F32 domain checking is explicit in a program, normally through
`IsFinite` and `Require`. I32 checked operations request a flag automatically
through `ScalarProgram::requires_fault_flag`.

### Predicates and selection

| Opcode | Arity and type rule | Result |
| --- | --- | --- |
| `Equal`, `NotEqual`, `LessThan`, `LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual` | 2, operands have the same type | I32 zero or one. F32 comparisons use ordered predicates for equality and ordering. `NotEqual` uses unordered-not-equal, so an F32 NaN is not equal and yields one. The other F32 predicates are false for NaN. I32 comparisons are signed. |
| `Select` | 3, first operand I32, second and third operands have the same type | The third operand is selected when the condition is zero; the second is selected when it is nonzero. It is not a short-circuit operation: all instruction operands have already been materialized. |

`Equal` on F32 is ordered equality, so a NaN yields zero. These rules are
observable in the LLVM predicates (`oeq`, `une`, `olt`, `ole`, `ogt`, `oge`)
and are also stated by the opcode comments.

### Bit operations, casts, shifts, and checks

| Opcode | Arity and type rule | Payload operation |
| --- | --- | --- |
| `BitAnd`, `BitOr`, `BitXor` | 2, both I32, result I32 | Integer bit operation. |
| `BitNot` | 1, I32 to I32 | XOR with all-one bits. |
| `BitcastF32ToI32` | 1, F32 to I32 | Reinterpret the exact 32 bits, without numeric conversion. |
| `BitcastI32ToF32` | 1, I32 to F32 | Reinterpret the exact 32 bits, without numeric conversion. |
| `ShiftLeft`, `ShiftRightLogical`, `ShiftRightArithmetic` | 2, both I32, result I32 | Integer shift. The native emitter masks the shift count with `31`, giving the defined five-bit shift-count behavior. Logical right shift is zero-filling; arithmetic right shift preserves the sign. |
| `Require` | 1, I32 to I32 | Normalize a truth value to zero or one. Zero records a rejection in the preallocated fault channel. |
| `IsFinite` | 1, F32 to I32 | One for a finite binary32 value, zero for either infinity or NaN. It does not itself reject the lane. |
| `IsNan` | 1, F32 to I32 | One for an F32 NaN, zero otherwise. It does not itself reject the lane. |
| `ConvertF32ToI32` | 1, F32 to I32 | Numeric conversion. The current LLVM emitter uses the saturating `llvm.fptosi.sat.i32.f32` intrinsic. |
| `ConvertI32ToF32` | 1, I32 to F32 | Signed numeric conversion. |

Bitcasts and integer shifts are representation operations, not arithmetic
work. They are used heavily by Recipe math for exponent extraction,
sign-copying, and construction of exact powers of two.

### F32 unary functions

`SquareRoot`, `Floor`, `Ceiling`, and `RoundNearestEven` each take one F32 and
return F32. The LLVM emitter calls the corresponding target-independent
intrinsic and canonicalizes a NaN result. `RoundNearestEven` is an explicit
rounding operation, separate from the rounding mode on F32 arithmetic.

## Type resolution and work pricing

`result_dtype` is the one source of truth for instruction signatures:

* Arithmetic and min/max preserve one common operand type.
* FMA is F32-only.
* Comparisons accept either two F32 values or two I32 values, and always return
  I32.
* Select requires an I32 condition and equal branch types.
* Bitwise operations, shifts, and Require are I32-only.
* Bitcasts and conversions have the explicitly named source and destination
  types.
* Square root and rounding operations are F32-only.

An incorrect arity returns `None` before any result is created. A mismatched
type returns `None` as well. `ScalarProgram::validate` turns these failures
into `ValidationCode::ScalarArity` or `ValidationCode::ScalarTypeMismatch`.

`ScalarOpcode::flops` is the deterministic arithmetic work used by primitive
work accounting and the static scheduler. One FLOP is assigned to Add,
Subtract, Multiply, Divide, Remainder, Negate, Absolute, Minimum, Maximum,
all six comparisons, SquareRoot, Floor, Ceiling, and RoundNearestEven. FMA
is two FLOPs. Select, all bit operations, bitcasts, shifts, Require,
IsFinite, IsNan, and both conversions are zero FLOPs. Address formation,
loads and stores, representation changes, validation predicates, and fault
publication are intentionally not counted in this field. Primitive work sums
instruction prices per output element, then multiplies by the output element
count with checked `u64` arithmetic.

The native probe exercises the same core contract directly. Its bounded FMA
benchmark builds a `KernelTemplate` with one F32 input, two bit-preserving F32
constants, an ordered FMA chain, contiguous access, and a forbidden input to
output alias. The measured work is the chain length times two FLOPs times the
index-space element count, so the hardware rate is tied to the same scalar
pricing used later by scheduling.

## Core validation

`ScalarProgram::validate` performs one accumulating validation pass:

1. It inserts every input ID and type into a definition map, then every
   constant ID and literal-derived type. Reuse across either collection is a
   `DuplicateScalarValue` error.
2. It walks instructions in vector order. Every operand must already be in the
   map, which enforces acyclic SSA order and reports
   `ScalarUseBeforeDefinition` otherwise. Known operand types are passed to
   `validate_signature`, which checks arity and `result_dtype` against the
   instruction's declared result type.
3. The result ID is inserted after the signature check. Reusing any previous
   input, constant, or instruction ID reports `DuplicateScalarValue`.
4. At least one output is required. Every output ID must exist in the
   definition map, otherwise `UnknownScalarValue` is reported.

The validator accumulates all independent errors and returns one
`ValidationErrors` value. If an operand is unknown, signature checking for that
instruction stops after the use-before-definition error, avoiding a misleading
second type error. `dtype_of` is a lookup helper used after validation by
kernel-template checks. It searches inputs, constants, then instruction
results.

`requires_fault_flag` is deliberately narrower than “the program can reject”.
It returns true when an instruction is `Require(I32)`, `Divide(I32)`,
`Remainder(I32)`, `Negate(I32)`, or `Absolute(I32)`. F32 math programs contain
their own `Require` instructions when a finite-domain or other numerical
condition must be enforced.

### Index space and access metadata

`IndexSpace` describes the parallel logical coordinates, independently of
storage. `IndexSpace::new` requires a nonempty dimension vector, requires every
`ElementCount` to be nonzero, and checks the product for `u64` overflow. Its
`elements` value is therefore nonzero. `is_structural_singleton` is true only
when the product is one. `StaticBufferAccess::linear` creates a one-dimensional
unit-stride view; `contiguous` computes row-major strides in the index-space
rank and allocates exactly the product times four bytes.

`StaticBufferAccess::validate` enforces the memory contract used by native
address generation:

* The stride rank must equal the index-space rank.
* The maximum element address is `offset_elements + sum((extent - 1) *
  stride)`, with checked arithmetic. One more element, multiplied by the
  dtype width, must fit in `storage_bytes`.
* A writable mapping must be injective. Axes with extent greater than one are
  sorted by stride and checked as non-overlapping occupied spans. This rejects
  overlapping and broadcast writes.
* A zero stride is valid for a read-only broadcast dimension. It is rejected
  for a writable dimension whose extent is greater than one. A singleton axis
  may retain a zero stride because it names only one logical element.

The access metadata is in elements, while `storage_bytes` bounds the complete
backing allocation. It is not merely the byte size of a densely packed logical
payload.

### Kernel template validation

`KernelTemplate` combines an `IndexSpace`, ordered `KernelInput` and
`KernelOutput` records, one `ScalarProgram`, and a complete input/output alias
matrix. Validation first appends all scalar-program errors, then validates
each access mapping and input/output ID uniqueness. The ordered input count and
dtype must match `program.inputs`; the ordered output count must match
`program.outputs`, and every scalar output's dtype must match its kernel output.

Every pair of kernel input and kernel output IDs must have exactly one
`AliasRule`. The rule names storage permission, not scalar computation:

* `Forbidden` means the finalized ranges must not overlap.
* `MayAliasExact` allows no overlap or an exact same object, offset, and byte
  range.
* `MustAliasExact` requires the exact same object, offset, and byte range.

The later plan validator applies the same permissions to resolved arena
bindings. Scalar IDs never participate in this matrix directly.

## Construction in the language layer

`language/src/scalar_builder.rs` provides `ScalarProgramBuilder` and the
opaque `ScalarExpression` handle. Each builder receives a process-local owner
token from an atomic counter. Expressions carry both owner and dtype, so an
operand or output from another builder is rejected before it can enter the
program. IDs start at one and advance with checked arithmetic through the
single input, constant, and instruction namespace.

`input`, `constant`, `f32`, and `i32` allocate definitions. `unary`, `binary`,
and `ternary` are convenience wrappers over `apply`. `apply` first checks
ownership, resolves the opcode signature with `result_dtype`, allocates the
next ID, and appends one instruction in call order. `finish` checks output
ownership, assembles the four `ScalarProgram` vectors, and calls
`ScalarProgram::validate` before returning. Consequently, normal language
callers cannot construct a program with an unknown operand, foreign value, or
wrong result dtype.

The builder's call order is part of the program identity. Reordering two
equivalent calculations creates a different instruction sequence and therefore
a different OGDL representation, lowered-program digest, stage identity, and
artifact contract.

### OGDL persistence

`language/src/ogdl.rs` encodes an elementwise program under the kernel's
`kind.Elementwise.program` node with four explicit children: `inputs`,
`constants`, `instructions`, and `outputs`. Each input carries `id` and
`dtype`. Each constant carries `id` and a single literal variant, either
`F32Bits` or `I32`. Each instruction carries `result`, `dtype`, `opcode`, and
an ordered `operands.value` list. Outputs are ordered scalar value IDs.

The canonical dtype spellings in this format are exactly `F32` and `I32`, and
the literal variants are exactly `F32Bits` and `I32`. Numeric fields are parsed
as unsigned or signed values according to their destination type; no textual
float round-trip is used for a scalar literal.

Encoding validates the complete calculation graph before writing canonical
OGDL. Decoding is strict: the schema and version must match, unknown,
duplicate, or missing record fields are rejected, enum spellings and numbers
are parsed exactly, and the reconstructed graph is passed through
`CalculationGraph::validate`, which includes scalar-program validation. The
opcode encoder is intentionally explicit. If a newer non-exhaustive opcode is
not in the schema mapping, encoding fails with an unsupported-value error
rather than silently emitting a legacy spelling.

## Recipe-owned math programs

`math/src/program.rs` builds all shipped scalar math as ordinary
`ScalarProgram` SSA. `math/src/contract.rs` is the normative catalog of 22
functions, their arity, finite domain, signed-zero behavior, algorithm
identity, and error claim. Every generated function calls
`require_finite_inputs` first. Non-finite inputs therefore set the device
fault through `IsFinite` followed by `Require`, rather than producing a host
exception or silently propagating an infinity.

The current math surface is:

| Function | Arity and accepted finite domain | Signed-zero and algorithm contract |
| --- | --- | --- |
| `Reciprocal` | 1, nonzero finite `x` | Both zero signs are rejected. Exact `1 / x`. |
| `ReciprocalSquareRoot` | 1, finite `x > 0` | Both zero signs are rejected. `sqrt(x)` followed by `1 / root`. |
| `Sin` | 1, `-8192 <= x <= 8192` | Input signed zero is preserved. Cody-Waite reduction with an odd polynomial. |
| `Cos` | 1, `-8192 <= x <= 8192` | Either signed zero maps to positive one. Cody-Waite reduction with an even polynomial. |
| `Tan` | 1, `-1.4 <= x <= 1.4` | Input signed zero is preserved. Sine divided by cosine away from poles. |
| `Atan2` | 2, finite `y` and `x`, not both zero | Signed `y` zero selects the signed x-axis; `(0, 0)` is rejected. Octant reduction with an odd atan polynomial. |
| `Exp` | 1, `-80 <= x <= 80` | Either signed zero maps to positive one. Normal power-of-two reconstruction. |
| `ExpWithGradualUnderflow` | 1, every finite `x <= 0` | Positive and non-finite values are rejected. Split-scale reconstruction preserves subnormals and rounds below half the minimum subnormal to positive zero. |
| `ExpMinusOne` | 1, `-80 <= x <= 80` | Input signed zero is preserved. Local series near zero, range-reduced exponential elsewhere. |
| `Log` | 1, finite `x > 0`, including subnormals | Both zero signs are rejected. Bit decomposition and an atanh series. |
| `LogOnePlus` | 1, finite `x > -1` | Input signed zero is preserved. Local series near zero, bit-decomposed log elsewhere. |
| `Floor` | 1, any finite binary32 | Input signed zero is preserved. Scalar floor primitive. |
| `Ceil` | 1, any finite binary32 | Input signed zero is preserved. Scalar ceil primitive. |
| `RoundNearestEven` | 1, any finite binary32 | Input signed zero is preserved. Scalar round-to-nearest-even primitive. |
| `Trunc` | 1, any finite binary32 | Input signed zero is preserved. Selects ceiling for negative values and floor otherwise. |
| `Fmod` | 2, finite dividend and nonzero finite divisor | A zero dividend preserves its sign. IEEE remainder opcode with an explicit nonzero check. |
| `Pow` | 2, finite base `> 0`, finite exponent, exponent times `log(base) <= 80` | Either signed base zero is outside the domain. Positive-base log followed by split-scale exponential reconstruction. |
| `Sign` | 1, any finite binary32 | Input signed zero is preserved. Ordered negative, positive, and zero selects. |
| `Sigmoid` | 1, `-80 <= x <= 80` | Either signed zero maps to positive `0.5`. Sign-stable exponential quotient. |
| `Tanh` | 1, `-40 <= x <= 40` | Input signed zero is preserved. Sign-stable `expm1` quotient. |
| `Softplus` | 1, `-80 <= x <= 80` | Either signed zero maps to positive `ln(2)`. `max(x, 0) + log1p(exp(-abs(x)))`. |
| `Erf` | 1, `-6 <= x <= 6` | Input signed zero is preserved. Abramowitz-Stegun 7.1.26 using Recipe exponential. |

The declared absolute and relative error bounds are part of the math contract,
not additional runtime branches. Exact-composition functions such as
Reciprocal, ReciprocalSquareRoot, Floor, Ceil, RoundNearestEven, Trunc, Fmod,
and Sign have zero claimed error. Approximation bounds are recorded for the
remaining functions, with the current maxima ranging from `2e-6` for Erf to
`5e-4` for Tan. Algorithm names and versions are included in operation and
artifact identity, so changing a coefficient set is an identity change.

The builder uses only scalar opcodes. For example, `Sin` and `Cos` perform
range reduction with FMA, evaluate fixed Horner polynomials, convert the
quadrant to I32, and select signs. `ExpWithGradualUnderflow` clamps before the
F32-to-I32 exponent conversion, constructs normal powers by bitcast, and
applies a `2^-64` tail scale for lower exponents. `Log` normalizes subnormals,
extracts exponent and mantissa bits, reduces the mantissa, and evaluates an
atanh series. These are calculations in one SSA namespace, not calls to a
vendor math library.

`Require` is the domain boundary for all explicit checks. Its result is still
an I32 value and a rejected lane continues with a safe value until the final
fault branch. The executor later turns the fault readback into a failed run.

## Operation registry and composite recipes

`ops/src/registry.rs` classifies each source-qualified operation as one of
`LoweringAvailability::Scalar`, `Primitive`, `Composition`, `Workspace`,
`NonCalculation`, or `Unsupported`. The registry checks scalar recipes first,
then primitive and other categories. Unsupported entries fail closed. No
legacy source string or host fallback is substituted for a missing scalar
recipe.

`ops/src/scalar.rs` has three scalar recipe families:

* `ScalarRecipe::Opcode` maps a public symbol directly to one core opcode and
  an exact input dtype slice. Comparisons return I32; the other canonical
  elementwise opcode recipes return F32.
* `ScalarRecipe::Math` maps to one of the 22 Recipe math programs. Its input
  and output contract is F32.
* `ScalarRecipe::Composite` emits several core instructions for one operation.
  It is still one elementwise `ScalarProgram`, not a second execution model.

The registry's `ScalarRecipe` dtype contract describes one F32 output for a
canonical symbol. A few public helper constructors intentionally expose more
than one output, such as PReLU backward and the logits loss programs. They use
the same builder and validation rules; their multiple outputs are represented
by the ordinary ordered `ScalarProgram.outputs` list rather than by a special
multi-result instruction.

The current public symbol map includes direct arithmetic and predicates such as
`gpu_add_into`, `gpu_sub`, `gpu_mul`, `gpu_div_into`, `gpu_eq`, `gpu_lt`,
`gpu_gt`, `gpu_min`, `gpu_max`, `gpu_neg`, `gpu_fma`, `gpu_sqrt`, and
`gpu_where_mask`; math symbols such as `gpu_sin`, `gpu_cos`, `gpu_tan`,
`gpu_exp`, `gpu_expm1`, `gpu_log_into`, `gpu_log1p`, `gpu_pow`,
`gpu_reciprocal`, `gpu_rsqrt`, `gpu_sigmoid_into`, `gpu_tanh_into`,
`gpu_softplus`, `gpu_sign_into`, `gpu_floor`, `gpu_ceil`, `gpu_round`, and
`gpu_trunc`; and composite activation, objective, and optimizer symbols. Names
that mention legacy F16 are still admitted only through the canonical F32
recipe when the registry maps them to one of those recipes. The dtype contract
remains F32 or I32, never F16.

The composite input arities are fixed by `CompositeScalar::inputs`:

| Arity | Variants |
| --- | --- |
| 1 F32 | `Identity`, `Relu`, `Silu`, `GeluTanh` |
| 2 F32 | `ReverseAdd`, `ReverseMultiply`, `ReverseSubtract`, `ReverseDivide`, `ReluBackward`, `LeakyRelu`, `Elu`, `SigmoidBackward`, `TanhBackward`, `SiluBackward`, `GeluTanhBackward`, `GeluTanhMultiply`, `GluGeluTanh`, `GluSilu`, `ScaledExp`, `KullbackLeiblerElement`, `MeanAbsoluteErrorGradient` |
| 3 F32 | `DifferenceScaled`, `Clamp`, `ClampLegacyOrder`, `BinaryCrossEntropyGradient`, `LeakyReluBackward`, `EluBackward`, `Selu`, `Reparameterize`, `HuberGradient`, `SgdAdd`, `SgdSubtract` |
| 4 F32 | `Dropout`, `SeluBackward` |

Their definitions are direct scalar formulas:

* `Identity` copies its value. ReverseAdd and ReverseMultiply preserve the
  source ABI's historical operand order. ReverseSubtract and ReverseDivide
  emit scalar minus element and scalar divided by element.
* `DifferenceScaled` is `(left - right) * scale`. `Clamp` is
  `min(max(value, lower), upper)`. `ClampLegacyOrder` preserves the legacy
  `(lower, upper, value)` input order.
* `BinaryCrossEntropyGradient` clamps prediction to `[1e-7, 1 - 1e-7]`, forms
  `(bounded - target) / (bounded * (1 - bounded))`, then multiplies by the
  inverse count. The logits form validates a finite target in `[0, 1]`, then
  inlines stable Softplus and Sigmoid programs and returns both loss and
  gradient. The canonical focal-with-logits program similarly validates exact
  binary targets, uses the fixed `alpha = 0.25` and `gamma = 2.0`, and returns
  loss and gradient without taking logarithms of a rounded probability.
* `Dropout` selects zero when `mask < probability`; otherwise it multiplies
  value by the supplied scale.
* `Relu` is `max(value, 0)`. Its backward form selects gradient only when the
  activation is greater than zero. Leaky ReLU selects value or `alpha * value`,
  and its backward form selects gradient or `alpha * gradient`.
* ELU selects value for positive input and `alpha * (exp(value) - 1)`
  otherwise. Backward ELU selects one or `alpha * exp(value)`. SELU scales ELU
  by lambda, and its backward form scales the derivative and gradient.
* Sigmoid backward is `gradient * activation * (1 - activation)`. Tanh
  backward is `gradient * (1 - activation * activation)`. SiLU is
  `value * sigmoid(value)` and its backward form emits the analytic derivative.
* GELU uses the tanh approximation with coefficient `0.044715`; backward
  emits its analytic derivative. GELU multiply and GLU GELU multiply an
  activated value by the second input. GLU SiLU uses SiLU instead.
* ScaledExp computes `exp(value * scale)`. Reparameterize computes
  `mean + exp(0.5 * log_variance) * epsilon`. The KL element is
  `0.5 * (mean^2 + exp(log_variance) - log_variance - 1)`.
* Mean absolute error gradient is `sign(prediction - target)`. Huber gradient
  clamps `prediction - target` to `[-delta, delta]`. SgdAdd and SgdSubtract
  multiply the learning-rate input by the gradient and add or subtract it
  from weights.

`lower_scalar` accepts only a descriptor whose lowering is `Scalar`. It builds
  an opcode program with `Composer`, converts a `MathFunction` through
  `ScalarProgram::try_from`, or lowers a composite. It validates the resulting
  program again and reports `WrongLoweringKind`, `UnsupportedLowering`, or
  `InvalidScalarProgram` without changing the operation's category.

`Composer` is the operation-layer equivalent of `ScalarProgramBuilder`. It
starts IDs at one, uses a local typed `Value`, checks `result_dtype` on every
instruction, and validates on `finish`. `inline_math` is important: it takes
  a complete math program, remaps each input to an existing composer value,
  allocates fresh constants, replays instructions in order, checks every
  replacement dtype, and returns the mapped first output. Thus an inlined math
  function cannot introduce a second scalar identity space or an opaque call.

Structured materializers in `ops/src/materialize.rs` use the public language
builder in the same way. Helpers convert builder errors into operation errors,
and `scalar_finish` validates before returning a graph fragment. Parameters,
axes, shapes, and aliases remain explicit in the surrounding primitive or
composition; scalar code receives only typed values and constants.

Training and inference do not introduce another scalar representation. The
forward graph code builds recurrent equations, normalization maps, masks, and
integer attention indices with `ScalarProgramBuilder`, or asks the operation
registry for an owned symbol and calls `lower_scalar`. The graph compiler then
wraps the returned program in `PrimitiveKind::Elementwise`, supplies tensor
`ValueId`s and alias rules, and sends it through the same primitive validator
and lowering path described below. This includes I32 index programs such as
causal masks and head-major source indices, F32 recurrent-cell equations, and
inference helpers that explicitly validate conversions and index ranges with
`Require`. A training or inference declaration therefore cannot bypass scalar
typing or create a host-side evaluator.

## From `ScalarProgram` to a primitive stage

An elementwise `PrimitiveKernel` owns a scalar program in its
`PrimitiveKind::Elementwise` variant. Language validation checks the scalar
program, requires tensor input arity and dtype to match scalar inputs, computes
the broadcast result shape, and requires each scalar output's dtype and shape
to match the corresponding tensor output. A constant-only elementwise map is
not accepted; the language directs callers to Random or an explicit filled
input for that case. The primitive's input/output alias matrix must contain
every pair exactly once.

That restriction belongs to the language primitive contract, not the core
scalar type. A manually assembled core `ScalarProgram` and `KernelTemplate`
may have zero buffer inputs and expose a constant as an output, provided all
core validation rules hold. The primitive layer deliberately rejects that form
so a normal elementwise graph always has a tensor input boundary.

`recipe_primitives::lower` validates the primitive and creates a
`LoweredProgram`. For `Elementwise`, `lower_elementwise` performs these steps:

1. A nonempty output shape becomes a core `IndexSpace`. Empty tensor payloads
   produce no dispatch stage, because a zero-element shape is a valid language
   tensor but cannot form a core nonzero `IndexSpace`.
2. Each primitive input becomes a one-based `KernelInputId`. Its access view is
   produced by `broadcast_access`: leading rank is padded, singleton input
   dimensions use zero stride when expanded, and the original tensor offset,
   non-broadcast strides, storage bytes, and dtype are retained.
3. Each primitive output becomes a one-based `KernelOutputId` with the original
   tensor layout. The primitive alias matrix is translated from positional
   input/output indices to these kernel IDs.
4. A `KernelTemplate` is assembled with the cloned scalar program and is
   validated again. This is the point where scalar values are paired with
   concrete tensor access, but the scalar program itself remains placement
   free.
5. Program buffers are bound in input order and output order. If
   `program.requires_fault_flag()` is true, the builder adds one I32,
   four-byte, read/write-atomic fault binding with `FaultReason::ArithmeticDomain`
   and code `2`.
6. Per-lane FLOPs are the checked sum of instruction `flops`; total FLOPs are
   that sum times output elements. The scalar private bound is
   `(input_count + constant_count + instruction_count) * 4` bytes per lane.
   The stage records an integer-operation bound equal to the output element
   count and an atomic-operation bound equal to the output element count only
   when a fault flag exists.
7. Measured hardware chooses a bounded power-of-two workgroup width. The
   dispatch has one logical lane per output element and a ceiling-divided
   workgroup count. The `ProgramBuilder` serializes each emitted stage behind
   the immediately preceding stage and includes all bindings, resources, and
   the canonical program digest in `LoweredProgram` validation.

The primitive lowering representation therefore contains both the original
`KernelTemplate` for a scalar map and the surrounding `StageKind::ScalarMap`
contract. Reductions, scans, contractions, indexing, sorting, and random
stages use other stage kinds, although they may use scalar literals for
identities or fills. They do not create a second scalar program ontology.

`LoweredProgram::validate` rechecks the scalar stage without runtime state. It
revalidates the embedded template, requires stage logical lanes to equal the
template index-space product, requires the binding count to equal scalar input
count plus scalar output count plus one optional fault binding, and requires
`program.requires_fault_flag()` to agree with `stage.fault.is_some()`. Scalar
maps have no synchronization points. Their exact resource equation is
recomputed from the instruction FLOPs, logical lanes, fault presence, and
four-byte scalar slot count; a caller-supplied bound that differs is invalid.
The fault validator additionally requires a checked stage to guard before a
payload address is formed and to publish an explicit I32 atomic exchange
through the declared fault binding.

## Planner and static scheduling

`planner/src/planner.rs` lowers every graph node once using common measured
hardware limits. Each lowered stage receives a collision-checked
stage-scoped `KernelTemplateId` derived from the lowered program digest,
source kernel ID, and stage ordinal. Scalar-map templates are cloned with that
identity and validated before they enter the candidate.

For every stage invocation, the planner:

* materializes tensor, scratch, and fault buffers as resident `ValueSpec`
  values on the selected GPU device;
* resolves stage bindings into ordered `ArtifactBuildBinding` records with
  dtype, access mode, affine view, and exact backing bytes;
* derives `CalculationTask.inputs` from read bindings and
  `CalculationTask.outputs` from write bindings, excluding the fault value;
* creates an `ArtifactBuildRecipe` containing stage identity, source program
  digest, stage ordinal, dispatch geometry, FLOP and resource bounds, and an
  independently hashed contract digest; and
* either selects an exact catalog artifact or leaves the build deferred for
  native realization. A catalog artifact must match stage identity,
  provenance, resource bounds, and discovered target exactly.

The resulting `CalculationTask` is a Loop-phase task with the stage's
`FlopCount`, dependencies from prior stage barriers and buffer producers, and
one iteration domain. A scalar fault value is not an ordinary calculation
input or output. It is a separate resident I32 value, initialized with the
device's data image and bound at the ABI fault position.

The planner's graph identity includes each lowered program digest, and its
kernel-template hash includes index dimensions, access views, scalar input and
constant IDs, literal bits, instruction result IDs, dtypes, opcode spellings,
operand order, output IDs, and alias permissions. Two programs that differ
only in instruction order or an F32 literal's bit pattern therefore cannot
share a candidate identity or silently reuse a native artifact.

The planner groups checked calculations by `(device, iteration domain)` and
requires one shared flag per cohort. It emits exactly one
`MetricTask { purpose: FaultReadback }` for the cohort, with direct dependencies
on all checked calculations. The readback has the same iteration domain as its
calculations, owns an exclusive metric slot, and must precede every user metric
and exit publication task. Core plan validation rejects a missing, duplicate,
unrelated, slot-shared, or domain-mismatched readback. User metrics are
separate and do not replace the fault readback.

`DraftPlan::validate` checks that every calculation is Loop phase and placed
on a GPU storage device, that its artifact resolves to exactly one realized
identity or deferred build, and that the artifact stage identity and work
match. Deferred bindings must preserve ordered inputs and outputs, resident
device, dtype, and backing bytes. A fault value must be one aligned four-byte
I32 resident on the calculation GPU, and its presence must agree with the
kernel or build contract.

The static scheduler receives these `CalculationTask` values, not raw scalar
instructions. `prepare_calculation` converts `calculation.work` with the
measured device FLOP rate using checked ceiling arithmetic, enforces a minimum
one-nanosecond duration, claims compute lanes, and accounts for transfer
overlap policy. The scalar `flops` definition therefore influences schedule
windows only through the immutable stage work bound. Addressing, integer
operation, and atomic bounds remain resource metadata rather than a second
model-work kind.

## LLVM lowering of a scalar map

`kernel/src/llvm.rs::lower_elementwise` is the direct native lowering boundary
for a validated `KernelTemplate`. It validates the template, target, and
launch options, then emits one-dimensional AMDGPU or NVPTX LLVM IR:

1. Target intrinsics produce local and workgroup IDs. The emitter computes
   `global_id = group_id * workgroup_lanes + local_id`, compares it with the
   ABI `element_count`, and exits lanes outside the logical range.
2. For each kernel input, `emit_buffer_index` decomposes the linear logical ID
   into index-space coordinates, applies the static offset and strides, emits
   an in-bounds element pointer, and loads one typed element. The corresponding
   scalar input ID is inserted in the emitter's value map.
3. Constants are emitted as I32 immediates or as an exact `bitcast i32 bits to
   float`, preserving `ScalarLiteral` representation.
4. Instructions are visited in program order. Every operand must already be in
   the value map; a missing ID returns `UnknownScalarValue`. The native matcher
   rejects an opcode/type combination not covered by the current implementation
   rather than substituting another operation.
5. The accumulated fault conditions branch once after scalar instructions. A
   rejected lane atomically publishes the flag and then rejoins the common
   path, so its safe substituted values can still be stored. The direct
   `lower_elementwise` API emits an atomic OR of one; deferred primitive-stage
   realization rewrites that publication to the stage's exact fault code.
6. Each scalar output is looked up, checked against the kernel output dtype,
   addressed through its output view, and stored. The generated entry point
   has input pointers, output pointers, an optional fault pointer, and a final
   I64 element count. Each ABI slot is eight bytes, and the reported work is
   per-instruction FLOPs multiplied by `IndexSpace::elements`.

F32 arithmetic uses constrained LLVM operations with round-to-nearest metadata
and IEEE denormal settings. Add, Subtract, Multiply, and Remainder use
constrained intrinsics; F32 Divide uses `fdiv`; FMA uses constrained one-rounding
FMA. Arithmetic, min/max, square root, and rounding results pass through
`canonicalize_nan`, which replaces any NaN with the canonical binary32 bits
`0x7fc00000`. Bitwise F32 negate and absolute preserve the original payload
bits because they operate directly on the sign bit. Integer add, subtract, and
multiply are emitted without a checked-fault path. Checked I32 divide,
remainder, negate, absolute, and Require select safe values, record rejection
conditions, and are guarded before the final fault publication.

The module is emitted with strict floating-point attributes, IEEE denormal
behavior, and no vendor runtime or math-library call. The target-specific
address spaces and entry convention are the only backend-specific pieces of
this lowering boundary.

### Deferred stage realization

When a planner build is deferred, `kernel/src/stage.rs::lower_stage` validates
the complete `LoweredProgram` and `ArtifactBuildRecipe` before emitting an
artifact. It independently checks the program digest, source kernel, stage
ordinal, stage-scoped template identity, geometry, work bounds, resources,
ordered binding views, and fault binding. For `StageKind::ScalarMap`, it calls
the same `lower_elementwise` emitter, then rewrites the direct atomic OR into
an `atomicrmw xchg` with the stage's release ordering and fault code `2`.
This makes the artifact's device-fault code part of the immutable stage
contract while reusing the scalar emitter rather than adding a second scalar
implementation.

`ArtifactBuilder` is the next boundary. It verifies the emitted LLVM module
with the pinned offline toolchain, compiles AMD modules to inspected HSACO or
NVIDIA modules to inspected cubin, and preserves entry-symbol, argument, and
target identity in the artifact provenance. Artifact compilation is available
only in the explicit offline or realization build phases; it is not loop work
and it does not reinterpret scalar instructions.

## Native execution and fault observation

Before submission, `native-executor/src/plan.rs` validates the realized
artifact ABI against the finalized bundle. For a deferred build it derives
the expected logical element count and ordered operand list directly from the
build bindings. It then checks:

* ABI element count and workgroup width equal the immutable dispatch geometry.
* Buffer arguments are exactly the calculation inputs followed by outputs,
  with the expected dtype, read/write access, storage bytes, and alignment.
* The optional fault argument exists exactly when `CalculationTask.fault_flag`
  exists, names one resolved four-byte I32 on the calculation device, and is
  in the canonical suffix position.
* Dynamic `RunId` and `LoopIteration` arguments occur at most once and, when
  present, follow the buffers and fault pointer. `ElementCount` is always the
  final argument.

The CUDA backend's `fill_invocation` maps each resolved value location to an
arena device pointer plus its immutable arena offset, retains the backing
allocation until the driver submission completes, and passes the fault pointer
only once. It passes run ID, loop iteration, and element count by value when
the ABI declares them. The HSA backend's `fill_kernarg` performs the same
mapping into an eight-byte-per-argument kernarg block and copies the block to
the host-visible HSA allocation before publication. Neither backend evaluates
scalar instructions; both submit the already-realized native image.

The generic executor keeps the immutable `Init -> Loop -> Exit` lifecycle. It
prepares a loop calculation with resolved input, output, and optional fault
locations, submits it through the selected native backend, and schedules the
cohort's four-byte metric readback after the checked calculations. A
`FaultReadback` result of I32 zero records a successful fault check. A nonzero
I32 code becomes `DeviceFault` and fails the run. An F32 result, missing metric,
duplicate fault argument, wrong dtype, or incompatible ABI is a protocol
error. Fault bytes are zero-initialized in the device init image. A successful
iteration leaves them zero, while a nonzero code terminates the run before a
later iteration can begin, so a separate loop-time fault reset is not a scalar
operation. This is the fail-closed boundary for every `Require` and checked I32
operation in a scalar program.

The RAM and disk host adapter is not a scalar fallback. It accepts admissions,
transfers, and four-byte metric readbacks, but rejects `BackendWork::Calculation`
with an unsupported-work error. Payload calculations therefore remain on the
CUDA or HSA native adapters even when a run also contains host-resident values.

## Invariants to preserve

The following invariants are cross-crate and should be treated as one contract:

* Scalar programs contain only F32 and I32 payloads. Mixed types require an
  explicit conversion or bitcast, and predicates produce I32 masks.
* Scalar IDs are local SSA identities. Tensor `ValueId`, kernel argument IDs,
  stage-scoped template IDs, artifact IDs, task IDs, and arena locations are
  separate namespaces connected by explicit ordered mappings.
* A program is valid only when every operand is defined earlier, every result
  has the declared type, every output exists, and every kernel input/output
  pair has an alias permission.
* Tensor broadcasting is an access-view property. It is represented by
  read-only zero strides and never by implicit scalar instructions.
* Writable views are injective, backing storage is large enough for the full
  affine span, and every native pointer is resolved from finalized arena
  metadata rather than recreated in the GUI or executor.
* Checked scalar operations never issue an invalid integer operation or form a
  rejected payload address. They select a safe value, publish a preallocated
  I32 fault flag, and let the mandatory readback decide run success.
* Math domain checks are explicit scalar instructions. F32 arithmetic does not
  acquire a hidden exception mechanism, and nonfinite values are rejected only
  where the owning math program inserts `Require`.
* FLOP work excludes addressing, copies, bit manipulation, conversions,
  predicates used only for validation, and fault publication. FMA is priced as
  two operations everywhere work is derived from the scalar instruction list.
* The operation registry, OGDL codec, primitive lowering, stage realization,
  and LLVM emitter all fail closed for an opcode or operation category they do
  not own. There is no legacy source-string fallback or alternate scalar
  evaluator.
* Native execution receives one immutable artifact ABI. Any mismatch between
  program, stage, build recipe, finalized value location, or backend argument
  block is an error, not an opportunity to infer a replacement mapping.

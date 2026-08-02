# Scalar operation recipes and lowering

This document describes the scalar boundary owned by `recipe-ops`. The paired
implementation is [`ops/src/scalar.rs`](../../src/scalar.rs). It maps the
source-qualified operation inventory to typed, backend-neutral
`recipe_core::ScalarProgram` values and exposes the canonical model programs
used by training. It also records the callers and the concrete path from one
scalar value per logical lane to a resident native kernel.

A scalar program is one per-element calculation. It is not a tensor view, a
broadcast rule, a transfer, a queue, a device choice, a schedule, or a
lifecycle phase. Those contracts are owned by the enclosing language
primitive, primitive lowering, planner, and executor. The scalar program is
the complete calculation payload that those layers consume.

## Position in the operation pipeline

The operation and execution path is:

```text
operation-surface.txt
        |
        | ops/build.rs preserves each (symbol, source) row
        v
RAW_OPERATION_SURFACE in OUT_DIR/operation_surface.rs
        |
        v
OperationRegistry::iter / resolve_unique / resolve_exact
        |
        +-- LoweringAvailability::Scalar(ScalarRecipe)
        |       |
        |       +-- lower_scalar -> ScalarProgram
        |       |
        |       +-- training/inference emit Elementwise
        |
        +-- LoweringAvailability::Composition
        |       |
        |       +-- materialize_composition
        |               |
        |               +-- materializers build ScalarProgram values with
        |                   ScalarProgramBuilder and emit Elementwise stages
        |
        +-- Primitive, Workspace, NonCalculation, or Unsupported
                (other owned path or explicit fail-closed error)

ScalarProgram
        -> PrimitiveKind::Elementwise
        -> recipe-primitives StageKind::ScalarMap and KernelTemplate
        -> planner stage identity, resources, and artifact contract
        -> kernel LLVM scalar emitter
        -> native executor launch, authoritative output state, and fault readback
```

`operation-surface.txt` currently has 421 accepted rows. The registry keeps
those rows in order and appends the two Recipe-owned pooling extensions. Scalar
classification is performed while a descriptor is built, not by the build
script. A missing scalar definition never falls through to a legacy call or a
host evaluator.

## Owned Rust surfaces

The scalar implementation is deliberately split by responsibility:

| Source | Responsibility |
| --- | --- |
| [`ops/src/scalar.rs`](../../src/scalar.rs) | `ScalarRecipe`, `CompositeScalar`, the symbol table, canonical model programs, `Composer`, and `lower_scalar`. |
| [`ops/src/registry.rs`](../../src/registry.rs) | Source-qualified descriptors, lowering classification, dtype/family/alias/determinism contracts, and resolution errors. |
| [`core/src/scalar.rs`](../../../core/src/scalar.rs) | `DType`, scalar literals/opcodes/records, SSA validation, index-space and access metadata, and `KernelTemplate` validation. |
| [`language/src/scalar_builder.rs`](../../../language/src/scalar_builder.rs) | Owner-checked construction for scalar programs used by graph and materialization callers. |
| [`math/src/program.rs`](../../../math/src/program.rs) and [`math/src/contract.rs`](../../../math/src/contract.rs) | Recipe-owned f32 math programs, finite-domain `Require` checks, algorithm identities, and error contracts. |
| [`ops/src/materialize.rs`](../../src/materialize.rs) | Shared scalar-builder adapters and graph emission for structured compositions. |
| [`primitives/src/lower.rs`](../../../primitives/src/lower.rs) | Converts an elementwise primitive to a `StageKind::ScalarMap` with accesses, bindings, fault state, geometry, and resource bounds. |
| [`planner/src/planner.rs`](../../../planner/src/planner.rs) | Assigns stage-scoped identities and turns scalar stages into immutable calculation and artifact build contracts. |
| [`kernel/src/llvm.rs`](../../../kernel/src/llvm.rs) and [`kernel/src/stage.rs`](../../../kernel/src/stage.rs) | Emits target LLVM for a validated `KernelTemplate` and rewrites deferred scalar fault publication to the stage fault code. |

`ops/src/scalar.rs` uses its private `Composer` for registry recipes. The
structured materializers use the public `ScalarProgramBuilder` instead. Both
produce the same core representation and pass through the same validation and
native lowering boundary. There is no second scalar execution model.

## Scalar data model and state

The complete scalar value domain is two four-byte payload types:

| Type | Representation | Width |
| --- | --- | --- |
| `DType::F32` | IEEE binary32 payload | 4 bytes |
| `DType::I32` | signed 32-bit integer payload | 4 bytes |

There is no implicit promotion, boolean payload, pointer, F16, F64, U8, or
host scalar in this contract. Predicate results are canonical I32 zero or one.
`Select` and `Require` treat any nonzero I32 as true at the native boundary.

`ScalarLiteral` stores either `F32Bits(u32)` or `I32(i32)`. The bit-preserving
f32 form is intentional: signed zero, infinities, subnormals, and NaN payloads
must survive construction, OGDL persistence, hashing, and native lowering.
`ScalarLiteral::dtype` is the only type decision for a literal. The f32
convenience methods call `to_bits`; they do not reject a non-finite value.

`ScalarValueId` is an opaque local identity from `core/src/ids.rs`. It is not a
tensor `ValueId`, a `KernelInputId`, or a `KernelOutputId`. One scalar program
has one namespace shared by all of these ordered records:

| Record | Fields | Invariant |
| --- | --- | --- |
| `ScalarInput` | `id`, `dtype` | A value loaded from one kernel input for the current logical lane. |
| `ScalarConstant` | `id`, `value` | A lane-invariant literal with type from the literal variant. |
| `ScalarInstruction` | `result`, `dtype`, `opcode`, `operands` | Defines one new value after all operand definitions. |
| `ScalarProgram` | `inputs`, `constants`, `instructions`, `outputs` | Complete ordered SSA program and one or more values to store. |

The core representation does not require IDs to be nonzero and does not reject
duplicate output entries. Normal callers use either `Composer` or
`ScalarProgramBuilder`, both of which start IDs at one and finish with core
validation. Instruction order is semantic. It is retained in OGDL, included in
program and stage digests, and therefore changes planner identity and native
artifact identity even when two formulas are algebraically equivalent.

`ScalarRecipe` is the operation-layer choice of one of three forms:

```text
Opcode { opcode, inputs: &'static [DType] }
Math(MathFunction)
Composite(CompositeScalar)
```

`Opcode` is one core instruction. `Math` delegates to a complete owned math
program. `Composite` expands a named multi-instruction formula into the same
SSA namespace. Tensor broadcasting and view selection remain outside this
enum, in the enclosing elementwise primitive.

## Core opcode contract

`recipe_core::ScalarOpcode` is `#[non_exhaustive]`. `arity` resolves the current
arity before a result exists, and `result_dtype` is the single source of truth
for accepted operand types and the result type. No caller may insert an
implicit conversion.

| Opcode group | Signature | Result and native meaning |
| --- | --- | --- |
| `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`, `Minimum`, `Maximum` | Two operands of one common type, F32 or I32 | The common type. F32 arithmetic is IEEE or constrained round-to-nearest as documented by the emitter. I32 divide and remainder are checked truncating operations. |
| `Negate`, `Absolute` | One F32 or one I32 | The operand type. I32 `MIN` is rejected and replaced with zero. |
| `Fma` | Three F32 operands | F32 one-rounding fused multiply-add. |
| `Equal`, `NotEqual`, `LessThan`, `LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual` | Two operands with the same F32 or I32 type | I32 zero or one. F32 uses ordered predicates except `NotEqual`, which is unordered-not-equal. |
| `Select` | I32 condition, then two equal-typed arms | Arm type. Zero selects the third operand; nonzero selects the second. |
| `BitAnd`, `BitOr`, `BitXor` | Two I32 operands | I32 bit operation. |
| `BitNot` | One I32 operand | I32 XOR with all-one bits. |
| `BitcastF32ToI32`, `BitcastI32ToF32` | One operand of the named source type | Reinterpret the exact 32-bit payload. |
| `ShiftLeft`, `ShiftRightLogical`, `ShiftRightArithmetic` | Two I32 operands | I32 shift; the native emitter masks the count to five bits. |
| `Require` | One I32 operand | I32 zero or one. Zero records a lane rejection through the fault channel. |
| `IsFinite`, `IsNan` | One F32 operand | I32 classification flag; classification alone does not reject. |
| `SquareRoot`, `Floor`, `Ceiling`, `RoundNearestEven` | One F32 operand | F32 intrinsic result. |
| `ConvertF32ToI32` | One F32 operand | Saturating numeric conversion in the current LLVM emitter. |
| `ConvertI32ToF32` | One I32 operand | Signed numeric conversion. |

Comparisons use signed I32 predicates for I32 and ordered `oeq`, `olt`, `ole`,
`ogt`, and `oge` predicates for F32. An F32 NaN is false for those ordered
predicates. F32 `NotEqual` uses `une`, so NaN is not equal and returns one.
F32 arithmetic, minimum, maximum, square root, and rounding results are
canonicalized to one NaN bit pattern by the native emitter. Sign-bit negate and
absolute operate on the original representation and therefore preserve the
non-sign bits.

`ScalarOpcode::flops` is the static arithmetic price. Add, subtract, multiply,
divide, remainder, negate, absolute, min, max, all comparisons, square root,
floor, ceiling, and round-even cost one. FMA costs two. Select, bit operations,
bitcasts, shifts, `Require`, classification, and conversions cost zero.
Addressing, loads, stores, representation changes, validation predicates, and
fault publication are resource metadata, not scalar FLOPs.

## Core validation and memory invariants

`ScalarProgram::validate` performs one accumulating pass:

1. Insert every input and constant ID with its type into a definition map.
   Reuse is `DuplicateScalarValue`.
2. Walk instructions in vector order. An operand absent from the map is
   `ScalarUseBeforeDefinition`. Known operand types are checked for exact
   arity and `result_dtype`, producing `ScalarArity` or `ScalarTypeMismatch`.
3. Insert each result only after signature checking. Reuse of any prior value
   is another `DuplicateScalarValue`.
4. Require a nonempty output list (`MissingScalarOutput`) and require every
   output ID to be defined (`UnknownScalarValue`).

Unknown operands stop signature checking for that instruction, so one invalid
reference does not create a misleading type error. The validator reports all
independent errors through the core validation result. `dtype_of` looks up an
ID in input, constant, then instruction order and is used by kernel-template
validation.

`ScalarProgram::requires_fault_flag` is intentionally narrower than “the
program contains a possible domain failure.” It returns true when an
instruction is `Require(I32)`, `Divide(I32)`, `Remainder(I32)`, `Negate(I32)`,
or `Absolute(I32)`. F32 math programs include explicit `IsFinite` and
`Require` instructions when a domain violation must reject a lane.

The scalar program is embedded in `KernelTemplate` with an `IndexSpace`, typed
`KernelInput` and `KernelOutput` access views, and a complete input/output
`AliasRule` matrix. `IndexSpace::new` rejects an empty dimension list, rejects
zero `ElementCount`, and checks the product for `u64` overflow. Static buffer
access validation requires rank-matched strides, checked maximum addresses
within `storage_bytes`, and injective writable mappings. A zero stride is valid
only for a read-only broadcast dimension, except on singleton axes. Kernel
template validation then requires scalar input/output counts and dtypes to
match the ordered kernel records, and requires exactly one alias rule for every
input/output pair.

The core layer owns these placement and memory types, but it does not choose
the tensor views. The language and primitive layers supply them after scalar
construction.

## Language builder used by graph and materialization callers

[`language/src/scalar_builder.rs`](../../../language/src/scalar_builder.rs)
provides `ScalarProgramBuilder` and the opaque `ScalarExpression` handle. A
handle contains a private process-local owner token, a `ScalarValueId`, and a
`DType`. Only `id()` and `dtype()` are exposed, so callers cannot forge a
foreign owner or a mismatched type.

`ScalarProgramBuilder::new` allocates an owner from `NEXT_BUILDER`, an atomic
counter with checked increment. Each builder starts `next_value` at one and
uses checked increment for all inputs, constants, and instruction results.
Owner or value identity exhaustion returns
`LanguageErrorKind::InvalidScalarProgram` and leaves the builder unchanged.

The state transitions are:

```text
new
  -> input / constant / f32 / i32
  -> apply / unary / binary / ternary
  -> finish(outputs)
  -> validated recipe_core::ScalarProgram
```

`apply` rejects handles from another builder before resolving the opcode
signature. A bad arity or type returns `InvalidScalarProgram` before an ID or
instruction is allocated. `finish` rejects foreign outputs, assembles the four
core vectors, and calls `ScalarProgram::validate`. The builder owns no tensor,
shape, alias, device, or execution state. Its call order is part of the
program identity.

## Registry classification and exact symbol inventory

`ops/build.rs` parses each non-comment inventory row into
`RawSurfaceEntry { ordinal, line, symbol, source, occurrence, occurrences }`.
`registry.rs` includes the generated source and `describe` computes one
`OperationDescriptor` for each row. `OperationRegistry::resolve_unique` fails
with `UnknownOperation` for a missing symbol and `AmbiguousSymbol` for multiple
source rows. `resolve_exact` requires one exact symbol and source pair.

`registry::lowering` uses this precedence:

1. `ScalarRecipe::for_symbol`.
2. `PrimitiveRecipe::for_symbol`.
3. `WorkspaceFormula::for_symbol`.
4. `NonCalculationRecipe::for_entry(symbol, source)`.
5. `CompositionRecipe::for_entry(symbol, source)`.
6. Explicit legacy dtype exclusion.
7. Dynamic `convert` or `gpu_convert` exclusion.
8. Non-`gpu_` host behavior.
9. `DedicatedPrimitiveCompositionPending` for an otherwise unowned GPU row.

The scalar lookup is symbol based. The original source remains in the
descriptor for identity, exact resolution, source-qualified compositions, and
error reporting. The current scalar table contains 94 symbols. The table below
is copied from `ScalarRecipe::for_symbol`; every alias in a cell maps to the
one recipe named in the first column.

### One-op `Opcode` recipes

| Recipe | Exact symbols | Inputs and output |
| --- | --- | --- |
| `Absolute` | `gpu_abs_into` | F32 -> F32 |
| `Add` | `gpu_add_into`, `gpu_add_scalar`, `gpu_bias_add`, `gpu_bias_add_f32`, `gpu_add_f16` | F32, F32 -> F32 |
| `Divide` | `gpu_broadcast_div`, `gpu_div_into`, `gpu_div_scalar` | F32, F32 -> F32 |
| `Multiply` | `gpu_broadcast_mul`, `gpu_mul`, `gpu_mul_f16`, `gpu_row_scale` | F32, F32 -> F32 |
| `Subtract` | `gpu_broadcast_sub_into`, `gpu_sub`, `gpu_sub_scalar`, `gpu_mse_grad_into` | F32, F32 -> F32 |
| `Equal` | `gpu_eq` | F32, F32 -> I32 mask |
| `Fma` | `gpu_fma` | F32, F32, F32 -> F32 |
| `GreaterThan` | `gpu_gt`, `gpu_gt_scalar` | F32, F32 -> I32 mask |
| `LessThan` | `gpu_lt`, `gpu_lt_scalar` | F32, F32 -> I32 mask |
| `Maximum` | `gpu_max` | F32, F32 -> F32 |
| `Minimum` | `gpu_min` | F32, F32 -> F32 |
| `Negate` | `gpu_neg` | F32 -> F32 |
| `SquareRoot` | `gpu_sqrt` | F32 -> F32 |
| `Select` | `gpu_where_mask` | I32, F32, F32 -> F32 |

The static slices `F32_1`, `F32_2`, `F32_3`, and `I32_F32_F32` are the exact
input signatures. `dtype_contract` gives I32 output only to comparison
opcodes; all other mapped opcode recipes give F32 output.

### Owned `Math` recipes

`ScalarRecipe::Math` converts the function through
`ScalarProgram::try_from`. The math crate builds a complete SSA program with
finite input checks and the algorithm identity from `MathContract`; it does
not emit a vendor library call. The 20 registry aliases are:

| Function | Symbol | Domain check in the owned program |
| --- | --- | --- |
| `Atan2` | `gpu_atan2` | Both finite and not both zero. |
| `Ceil` | `gpu_ceil` | Any finite binary32 input. |
| `Cos` | `gpu_cos` | Finite input in `[-8192, 8192]`. |
| `Exp` | `gpu_exp` | Finite input in `[-80, 80]`. |
| `ExpMinusOne` | `gpu_expm1` | Finite input in `[-80, 80]`. |
| `Floor` | `gpu_floor` | Any finite binary32 input. |
| `Fmod` | `gpu_fmod` | Finite dividend and nonzero finite divisor. |
| `Log` | `gpu_log_into` | Finite positive input. |
| `LogOnePlus` | `gpu_log1p` | Finite input greater than `-1`. |
| `Pow` | `gpu_pow` | Positive finite base, finite exponent, and bounded exponent times log(base). |
| `Reciprocal` | `gpu_reciprocal` | Finite nonzero input. |
| `RoundNearestEven` | `gpu_round` | Any finite binary32 input. |
| `ReciprocalSquareRoot` | `gpu_rsqrt` | Finite positive input. |
| `Sigmoid` | `gpu_sigmoid_into` | Finite input in `[-80, 80]`. |
| `Sign` | `gpu_sign_into` | Any finite binary32 input, preserving signed zero. |
| `Sin` | `gpu_sin` | Finite input in `[-8192, 8192]`. |
| `Softplus` | `gpu_softplus` | Finite input in `[-80, 80]`. |
| `Tan` | `gpu_tan` | Finite input in `[-1.4, 1.4]`. |
| `Tanh` | `gpu_tanh_into` | Finite input in `[-40, 40]`. |
| `Trunc` | `gpu_trunc` | Any finite binary32 input. |

`MathFunction::ALL` also contains `ExpWithGradualUnderflow` and `Erf`; no
legacy scalar symbol currently selects either function. They remain available
to Recipe-owned math callers through the math crate, not through a missing
registry alias. Every mapped math program uses `IsFinite` followed by
`Require`, so a domain failure is represented in the scalar program and not by
a host branch.

### Multi-op `Composite` recipes

`CompositeScalar::inputs` fixes the exact F32 input signature. Every composite
recipe returns one F32 output through the registry. Input order below is the
order supplied by the legacy operation ABI and is therefore semantic.

| Composite | Exact symbols | Inputs and formula |
| --- | --- | --- |
| `ReverseAdd` | `gpu_add_inplace`, `gpu_add_scalar_inplace` | 2; `x1 + x0` |
| `BinaryCrossEntropyGradient` | `gpu_bce_grad_into` | 3; clamp prediction to `[1e-7, 1 - 1e-7]`, then `((bounded - target) / (bounded * (1 - bounded))) * inverse_count` |
| `ReverseMultiply` | `gpu_mul_inplace`, `gpu_scale_inplace`, `gpu_scale_f64_inplace` | 2; `x1 * x0` |
| `ReverseSubtract` | `gpu_sub_inplace`, `gpu_rsub_scalar` | 2; `x1 - x0` |
| `ReverseDivide` | `gpu_rdiv_scalar` | 2; `x1 / x0` |
| `DifferenceScaled` | `gpu_sub_scale_into` | 3; `(x0 - x1) * x2` |
| `Clamp` | `gpu_clamp_into` | 3; `min(max(value, lower), upper)` |
| `ClampLegacyOrder` | `gpu_clip_value` | 3; `min(max(x2, x0), x1)`, preserving lower, upper, value order |
| `Identity` | `gpu_copy_into`, `gpu_fill`, `gpu_fill_f32` | 1; `x0` |
| `Dropout` | `gpu_dropout_into` | 4; mask less than probability selects zero, otherwise `value * scale` |
| `Elu` | `gpu_elu` | 2; positive value or `alpha * (exp(value) - 1)` |
| `EluBackward` | `gpu_elu_backward` | 3; `gradient * (1 if value > 0 else alpha * exp(value))` |
| `GeluTanh` | `gpu_gelu_f16`, `gpu_gelu_f32`, `gpu_gelu_into` | 1; `0.5 * value * (1 + tanh(0.7978846 * (value + 0.044715 * value^3)))` |
| `GeluTanhBackward` | `gpu_gelu_backward_f32`, `gpu_gelu_backward_into` | 2; gradient times the analytic derivative of the tanh GELU approximation |
| `GeluTanhMultiply` | `gpu_gelu_mul` | 2; tanh GELU of `x0`, multiplied by `x1` |
| `GluGeluTanh` | `gpu_glu_gelu` | 2; tanh GELU of the first split value, multiplied by the second |
| `GluSilu` | `gpu_glu_silu` | 2; SiLU of the first split value, multiplied by the second |
| `HuberGradient` | `gpu_huber_grad` | 3; clamp `x0 - x1` to `[-delta, delta]` |
| `KullbackLeiblerElement` | `gpu_kl_div` | 2; `0.5 * (mean^2 + exp(log_variance) - log_variance - 1)` |
| `LeakyRelu` | `gpu_leaky_relu_into` | 2; value if positive, otherwise `alpha * value` |
| `LeakyReluBackward` | `gpu_leaky_relu_backward_into` | 3; gradient if activation is positive, otherwise `alpha * gradient` |
| `MeanAbsoluteErrorGradient` | `gpu_mae_grad` | 2; `sign(prediction - target)` |
| `Relu` | `gpu_relu_f16`, `gpu_relu_f32`, `gpu_relu_into` | 1; `max(value, 0)` |
| `ReluBackward` | `gpu_relu_backward_f32`, `gpu_relu_backward_into` | 2; gradient if activation is greater than zero, otherwise zero |
| `Reparameterize` | `gpu_reparameterize` | 3; `mean + exp(0.5 * log_variance) * epsilon` |
| `ScaledExp` | `gpu_scaled_exp` | 2; `exp(value * scale)` |
| `SgdAdd` | `gpu_sgd_update` | 3; `weights + learning_rate * gradient` using the legacy value order |
| `SgdSubtract` | `gpu_sgd_update_f32` | 3; `weights - learning_rate * gradient` using the legacy value order |
| `Selu` | `gpu_selu` | 3; `lambda * ELU(value, alpha)` |
| `SeluBackward` | `gpu_selu_backward` | 4; `gradient * lambda * (1 if value > 0 else alpha * exp(value))` |
| `SigmoidBackward` | `gpu_sigmoid_backward_into` | 2; `gradient * activation * (1 - activation)` |
| `Silu` | `gpu_silu_into` | 1; `value * sigmoid(value)` |
| `SiluBackward` | `gpu_silu_backward_into` | 2; `gradient * sigmoid(value) * (1 + value * (1 - sigmoid(value)))` |
| `TanhBackward` | `gpu_tanh_backward_into` | 2; `gradient * (1 - activation^2)` |

The aliases with `_f16`, `_f64`, or similar historical spelling are still
canonical F32 recipes. `registry::explicit_legacy_dtype` records the excluded
legacy payload in `OperationDescriptor::legacy_dtype`, but because scalar
lookup has precedence, the descriptor's active dtype contract is the F32 or
I32 recipe contract. No legacy payload is authorized by the spelling.

The exact source rows represented by the table are preserved in the inventory:

| Source family | Scalar symbols present in that source |
| --- | --- |
| `gpu-core/src/kernels.rs` | `gpu_abs_into`, `gpu_add_inplace`, `gpu_add_into`, `gpu_add_scalar`, `gpu_add_scalar_inplace`, `gpu_bce_grad_into`, `gpu_bias_add`, `gpu_broadcast_div`, `gpu_broadcast_mul`, `gpu_broadcast_sub_into`, `gpu_clamp_into`, `gpu_copy_into`, `gpu_div_into`, `gpu_dropout_into`, `gpu_eq`, `gpu_exp`, `gpu_fill`, `gpu_fill_f32`, `gpu_fma`, `gpu_gelu_backward_into`, `gpu_gelu_into`, `gpu_gt`, `gpu_gt_scalar`, `gpu_kl_div`, `gpu_leaky_relu_backward_into`, `gpu_leaky_relu_into`, `gpu_log_into`, `gpu_lt`, `gpu_lt_scalar`, `gpu_mse_grad_into`, `gpu_mul`, `gpu_mul_inplace`, `gpu_neg`, `gpu_pow`, `gpu_relu_backward_into`, `gpu_relu_into`, `gpu_reparameterize`, `gpu_row_scale`, `gpu_scale_inplace`, `gpu_scaled_exp`, `gpu_sgd_update`, `gpu_sigmoid_backward_into`, `gpu_sigmoid_into`, `gpu_sign_into`, `gpu_silu_backward_into`, `gpu_silu_into`, `gpu_sqrt`, `gpu_sub`, `gpu_sub_inplace`, `gpu_sub_scale_into`, `gpu_tanh_backward_into`, `gpu_tanh_into`, `gpu_where_mask` |
| `gpu-core/src/math_ops.rs` | `gpu_atan2`, `gpu_ceil`, `gpu_cos`, `gpu_div_scalar`, `gpu_expm1`, `gpu_floor`, `gpu_fmod`, `gpu_log1p`, `gpu_max`, `gpu_min`, `gpu_rdiv_scalar`, `gpu_reciprocal`, `gpu_round`, `gpu_rsqrt`, `gpu_rsub_scalar`, `gpu_sin`, `gpu_sub_scalar`, `gpu_tan`, `gpu_trunc` |
| `gpu-core/src/nn_f32.rs` | `gpu_add_f16`, `gpu_bias_add_f32`, `gpu_gelu_backward_f32`, `gpu_gelu_f16`, `gpu_gelu_f32`, `gpu_mul_f16`, `gpu_relu_backward_f32`, `gpu_relu_f16`, `gpu_relu_f32`, `gpu_sgd_update_f32` |
| `gpu-core/src/k_gapact.rs` | `gpu_elu`, `gpu_elu_backward`, `gpu_selu`, `gpu_selu_backward`, `gpu_softplus` |
| `gpu-core/src/infer_ops.rs` | `gpu_gelu_mul`, `gpu_glu_gelu`, `gpu_glu_silu`, `gpu_scale_f64_inplace` |
| `gpu-core/src/losses.rs` | `gpu_huber_grad`, `gpu_mae_grad` |
| `gpu-core/src/optimizers.rs` | `gpu_clip_value` |

## `lower_scalar` and `Composer`

`lower_scalar(descriptor)` accepts only
`LoweringAvailability::Scalar(recipe)`:

* `Primitive`, `Composition`, `Workspace`, and `NonCalculation` return
  `OperationErrorKind::WrongLoweringKind` with the operation ID.
* `Unsupported(reason)` returns `UnsupportedLowering` with the selected
  definition and reason. It does not invent a scalar path.
* `Opcode` allocates typed inputs with `Composer::inputs`, applies the one
  opcode, and finishes one output.
* `Math` calls `ScalarProgram::try_from(function)`. A math construction error
  is reported as `InvalidScalarProgram` for the descriptor.
* `Composite` calls `lower_composite` and attaches the descriptor ID to any
  resulting operation error.
* The completed program is validated again. Any validation display becomes
  `InvalidScalarProgram` for the descriptor.

`Composer` is private to `ops/src/scalar.rs` and has only
`next_value`, `inputs`, `constants`, and `instructions` state. It starts IDs at
one. `input`, `constant`, and `f32` allocate definitions. `unary`, `binary`,
and `ternary` call `apply`, which resolves `ScalarOpcode::result_dtype` before
allocating a result. Checked ID increment reports
`InvalidScalarProgram: scalar value identity space exhausted`.

`inline_math` is the important composition callee. It takes a complete math
program, checks its input arity and the dtype of each argument, maps math input
IDs to existing composer values, allocates fresh constants, and replays every
instruction in order through `apply`. It checks each replacement result type,
requires a first output, and rejects an unknown output or operand. Inlining
keeps every scalar value in one acyclic identity space and prevents opaque
math calls or a nested artifact boundary.

`finish` builds `ScalarProgram` and validates it. The implementation therefore
has two validation points: each `apply` checks the local signature, and the
finished graph checks global uniqueness, definition order, and outputs.

## Canonical model programs and their callers

The public constructors in `ops/src/lib.rs` are the stable model-facing scalar
programs. They are not alternate registry symbols.

| Constructor | Program and caller role |
| --- | --- |
| `canonical_leaky_relu_program` | One F32 input, a fixed `alpha = 0.01` constant, and the leaky-ReLU forward formula. |
| `canonical_leaky_relu_backward_program` | Gradient and activation inputs, fixed alpha, and the selected input-gradient formula. |
| `canonical_prelu_program` | Activation and one learned F32 alpha input. The enclosing elementwise node broadcasts alpha. |
| `canonical_prelu_backward_program` | Gradient, activation, and alpha inputs. Output zero is input gradient; output one is the per-element alpha-gradient contribution. The training compiler reduces output one over the complete logical training partition before clipping and optimization. |
| `canonical_elu_program` and `canonical_elu_backward_program` | ELU with fixed `alpha = 1.0`, forward and derivative forms. |
| `canonical_selu_program` and `canonical_selu_backward_program` | Self-normalizing ELU with `alpha = 1.6732632` and `lambda = 1.050701`. |
| `canonical_focal_with_logits_program` | Two F32 inputs and two outputs, stable binary focal loss and its gradient with fixed `FOCAL_ALPHA = 0.25` and `FOCAL_GAMMA = 2.0`. |

`binary_cross_entropy_with_logits_program` is crate-private because it is the
scalar payload of the source-qualified composition `gpu_bce_with_logits`.
`ops/src/materialize/training.rs` validates that composition's tensor ABI and
then calls this constructor. It emits an elementwise primitive with two F32
outputs, loss and gradient.

The logits BCE program validates a finite target in `[0, 1]` with three
`Require` checks, inlines stable Softplus and Sigmoid, and returns
`softplus(logit) - logit * target` plus `sigmoid(logit) - target`. The focal
program validates a finite target that is exactly zero or one, forms a signed
logit, and uses `sigmoid(-signed_logit)` and `softplus(-signed_logit)` rather
than taking a logarithm of a rounded probability. Both constructors return
ordinary ordered `ScalarProgram.outputs`; no multi-result instruction exists.

`training/src/forward.rs::lower_activation` uses owned registry symbols for
Cos, Exp, Log, Tan, ReLU, Sigmoid, Tanh, GELU, and SiLU. It uses the canonical
constructors for Leaky ReLU, SELU, ELU, and PReLU. `training/src/compile.rs`
uses the corresponding backward constructors, the focal program, and the
PReLU two-output program. The PReLU backward path explicitly reduces the
second output and returns the learned scalar gradient. A PReLU backward call
without its learned scalar fails with the training compiler's invalid-network
error; it does not substitute a default alpha.

## Other scalar-builder callers in `recipe-ops`

Structured graph builders do not call `lower_scalar` for every internal map.
They use `ScalarProgramBuilder` directly and emit the resulting program in a
`PrimitiveKind::Elementwise`. The shared adapters in `ops/src/materialize.rs`
are:

* `scalar_builder` maps builder creation errors to
  `GraphMaterializationFailed` and attaches `OperationId`.
* `scalar_input`, `scalar_f32`, `scalar_i32`, `scalar_unary`,
  `scalar_binary`, and `scalar_ternary` map each builder call through the same
  operation-aware error path.
* `scalar_finish` consumes the builder and maps final validation errors to
  `GraphMaterializationFailed`.

The current direct/helper caller inventory is:

| Module | Scalar-builder work |
| --- | --- |
| `ops/src/bayes.rs` | Configuration contribution, joint/query bin indexes, posterior probability, and checked I32 bounds. |
| `ops/src/binary_metrics.rs` | Input checks, Brier/ranking terms, recall and calibration contributions, positive-count normalization, and fixed sums. |
| `ops/src/kmeans.rs` | Square and pairwise distance, membership conversion, one constants, and centroid updates. |
| `ops/src/knn_outputs.rs` | Square and pairwise distances, known-count and masked distance, finite divide/identity, and categorical code/bin maps. |
| `ops/src/tree.rs` | Tree indexing, node routing, leaf indexing, scaling, finite checks, and I32 interval checks. |
| `ops/src/materialize.rs` | FFT and triangular-solve maps, identity and normalization maps, optimizer state and weight updates, clipping, tree branches, and shared scalar validation helpers. |
| `ops/src/materialize/training.rs` | Bias add and the private BCE-with-logits elementwise map. |
| `ops/src/materialize/convolution_pooling.rs` | Divisor, typed pair identity, channel/global indexing, and pool-backward maps. |
| `ops/src/materialize/attention_sequence_embedding.rs` | Positional/head selection, causal masks, safe shifts/divides, mask-to-zero, and rotation maps. |
| `ops/src/materialize/indexing_sort_encoding.rs` | Segment boundaries and destinations, scaled adds, select maps, clears, checked one values, and triangular masks. |
| `ops/src/materialize/graph_cluster_rl.rs` | Temporal difference, GCN normalization, advantage/Gaussian maps, checked actions, categorical/masked maps, and distance/representative maps. |
| `ops/src/materialize/loss_metrics.rs` | Accuracy/count normalization, class checks, finite flags, hinge, log/KL, squared, contrastive, cosine, and triplet maps. |
| `ops/src/materialize/tree_boosting.rs` | Split indexing, gradient/Hessian, histogram, gain, leaf, threshold, prediction, and split-write maps. |

These callers keep all policy values and shapes in their materialization
request. The scalar program receives only typed F32/I32 values and constants.
No builder caller performs a host payload loop or owns device state.

## Registry consumers outside materialization

The root facade in [`src/facade.rs`](../../../src/facade.rs) reexports
`ScalarProgram`, exposes `operations::registry`, `all`, `resolve`, and
`resolve_exact`, and forwards `operations::lower_scalar` to
`recipe_ops::lower_scalar`. It is the public declaration-facing boundary; it
does not execute the program.

Training and inference have parallel graph-emitter paths:

```text
emit_owned_scalar(symbol, tensor inputs, tensor outputs)
    -> operation_registry().resolve_unique(symbol)
    -> lower_scalar(descriptor)
    -> emit_elementwise(..., PrimitiveKind::Elementwise { program })
```

`training/src/compile.rs::emit_scalar_operation` chooses between an owned
symbol and an already constructed canonical program. Its elementwise emitter
adds forbidden alias rules for ordinary maps and records the requested
iteration domain. `training/src/inference.rs::emit_owned_scalar` performs the
same symbol resolution and elementwise insertion for inference. A unique
resolution failure or scalar lowering error is converted to the enclosing
training or inference compile error without a fallback operation.

Structured composition symbols use a different path:

```text
resolve_unique(symbol)
    -> materialize_composition(MaterializationRequest)
    -> request and prepared-parameter validation
    -> concrete family dispatch
    -> ScalarProgramBuilder maps plus other primitive stages
    -> validated CalculationGraph
```

`materialize_composition` rejects a scalar descriptor with
`WrongLoweringKind`; `lower_scalar` rejects a composition descriptor with the
same kind. The separation prevents a source-qualified composition from being
silently treated as one elementwise map. `remaining_composition_manifest`
filters only descriptors whose lowering is `Composition`, so scalar rows are
never reported as missing materializers.

## Elementwise primitive and materialization boundary

`recipe-language::Elementwise` contains one `ScalarProgram`. Primitive kernel
validation first resolves input and output tensor IDs, checks the complete
alias matrix, validates the scalar program, requires tensor input arity and
dtypes to equal scalar inputs, computes the broadcast result shape, and
requires each scalar output's dtype and shape to equal the corresponding tensor
output. Constant-only maps are rejected at this language boundary; callers
must use a random primitive or an explicit filled input. The core scalar type
itself does not impose that tensor policy.

`recipe_primitives::lower_elementwise` then:

1. Converts the nonempty output shape to a core `IndexSpace`. An empty tensor
   emits no dispatch stage because core index spaces are nonzero.
2. Creates one-based `KernelInputId` values with broadcast access views. Input
   singleton dimensions use zero strides only for read broadcasts; offsets,
   non-broadcast strides, storage bytes, and dtypes are retained.
3. Creates one-based `KernelOutputId` values with the tensor output views and
   translates positional primitive aliases to core `AliasRule` values.
4. Builds and validates `KernelTemplate { index_space, inputs, outputs,
   program, alias_rules }`.
5. Binds input buffers in order, output buffers in order, and an optional
   four-byte atomic I32 fault buffer when `program.requires_fault_flag()` is
   true. The fault reason is `ArithmeticDomain`, with the stage code supplied
   by the primitive builder.
6. Computes per-lane FLOPs as the checked sum of instruction `flops`, total
   FLOPs as that sum times output elements, private bytes as
   `(inputs + constants + instructions) * 4`, integer-operation bound as output
   elements, and atomic-operation bound as output elements only when a fault
   flag exists.
7. Chooses the measured power-of-two workgroup width and emits one logical
   lane per output element with ceiling-divided workgroups.

The resulting `StageKind::ScalarMap` has no synchronization points. Program
validation, access validation, alias validation, binding count, geometry,
fault presence, and the exact resource equation are checked again by
`recipe_primitives::validate`.

## Planner, native lowering, and fault state

`planner/src/planner.rs::lower_programs` lowers each graph node with the common
measured hardware limits. For each scalar stage it derives a stage-scoped
`KernelTemplateId` from the lowered program digest, source kernel, and stage
ordinal, clones the template with that identity, and validates it before
creating an artifact candidate. The scalar program digest includes input and
constant IDs and literal bits, instruction result IDs, dtypes, opcode names,
operand order, output IDs, accesses, and aliases. Reordering instructions or
changing an F32 bit pattern therefore cannot reuse a different native artifact.

The planner converts a scalar stage to a Loop-phase `CalculationTask` on a GPU
device. Read and write bindings become tensor inputs and outputs. A fault value
is a separate resident I32 value, not a user calculation input or output. The
planner groups checked calculations by device and iteration domain, emits one
fault readback metric for the cohort, and requires that readback before user
metrics and exit publication. The scheduler sees the immutable stage FLOP
bound, not the individual scalar instructions.

`kernel/src/llvm.rs::lower_elementwise` is the final scalar-codegen boundary:

1. Target intrinsics compute `global_id` and exit lanes outside the element
   count.
2. Input views are decomposed into logical coordinates and static byte-bounded
   addresses. One typed input is loaded per lane.
3. Constants are emitted as I32 immediates or exact `bitcast i32 bits to float`
   values.
4. Instructions are visited in program order. An unavailable operand is an
   `UnknownScalarValue` lowering error, and an opcode/type combination not
   implemented by the emitter is an `UnsupportedOperation` error.
5. Checked integer operations and `Require` select safe values and collect
   rejection conditions. One branch publishes the fault after scalar
   calculation, then rejoins so safe values can still be stored.
6. Outputs are looked up, dtype-checked, addressed through output views, and
   stored. The ABI is input pointers, output pointers, an optional fault
   pointer, and the final I64 element count.

The direct `lower_elementwise` API emits an atomic OR of one for a rejected
lane. Deferred primitive-stage realization in `kernel/src/stage.rs` verifies
the exact lowered program and rewrites that publication to
`atomicrmw xchg` with the stage's `FaultContract` code. If the expected atomic
instruction is absent, realization fails rather than accepting a different
fault ABI. The executor reads the planned fault value through the dedicated
metric task; a fault is a failed run, not an alternate result.

## Error and validation matrix

Scalar construction and lowering fail closed at each boundary:

| Boundary | Validation or error | Meaning |
| --- | --- | --- |
| Registry resolution | `UnknownOperation`, `AmbiguousSymbol` | No exact descriptor or more than one source-qualified descriptor. |
| `lower_scalar` category | `WrongLoweringKind` | Descriptor is primitive, composition, workspace, or non-calculation rather than scalar. |
| Unsupported descriptor | `UnsupportedLowering` | Registry has no owned scalar path and reports its explicit reason. |
| Core scalar program | `DuplicateScalarValue`, `ScalarUseBeforeDefinition`, `ScalarArity`, `ScalarTypeMismatch`, `MissingScalarOutput`, `UnknownScalarValue` | Invalid SSA identity, order, signature, or output. |
| Composer | `InvalidScalarProgram` | Opcode signature, inline math mapping, ID overflow, or final validation failure. |
| Language builder | `InvalidScalarProgram` | Foreign builder handle, bad signature, owner/value counter exhaustion, or final validation failure. |
| Materializer scalar adapter | `GraphMaterializationFailed` | Builder or shape/axis failure attached to the source operation ID. |
| Primitive language boundary | `InvalidScalarProgram`, `ArityMismatch`, dtype/shape/alias errors | Scalar and tensor ABI do not agree. |
| Primitive lowering | `PrimitiveLoweringFailed` or lowering validation error | Access, index-space, binding, geometry, resource, or fault contract cannot be formed. |
| Kernel LLVM lowering | `InvalidKernel`, `UnknownScalarValue`, `UnsupportedOperation`, `ArithmeticOverflow`, `ProhibitedInterface` | Native emitter cannot realize the validated scalar contract. |

No error path substitutes a legacy symbol, retries another implementation, or
silently changes a dtype. Operation errors carry the `OperationId` whenever the
failure originates after descriptor resolution. Their display is
`<kind>: <detail> [operation <ordinal>]`, preserving the source identity for
callers and diagnostics.

## End-to-end role and non-goals

The scalar layer owns exactly one concern: deterministic typed calculation for
one logical element. It owns opcode signatures, Recipe math algorithms,
composite formula expansion, constants, value-definition order, explicit
domain rejection, and static work pricing. It does not own tensor shape or
broadcast policy, memory allocation, alias resolution, hardware probing,
queue or synchronization policy, transfer tasks, scheduling, native image
loading, or lifecycle state.

The authoritative end state is therefore established only after the complete
path succeeds:

```text
public symbol or model declaration
  -> source-qualified descriptor
  -> typed ScalarProgram
  -> Elementwise tensor graph
  -> validated ScalarMap stage and resident buffers
  -> planner CalculationTask and artifact contract
  -> target LLVM/native image
  -> executor launch
  -> published output tensors and fault metric
```

Every stage retains the same scalar program identity and type contract. This is
what lets Recipe use one backend-neutral calculation definition for both AMDGPU
and NVPTX while keeping operation semantics in Recipe and leaving placement,
resource measurement, and asynchronous execution to the downstream layers.

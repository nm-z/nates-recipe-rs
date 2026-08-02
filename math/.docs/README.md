# `recipe-math`

`recipe-math` is Recipe's backend-neutral scalar mathematics crate. It turns a
closed `MathFunction` value into a deterministic, typed `recipe_core::ScalarProgram`
made only from Recipe's `F32` and `I32` scalar instruction set. The crate also
publishes the finite-domain, special-value, error, and algorithm identity
contract for each function. It does not execute a value on the host, call a
vendor math library, choose a device, schedule work, or emit target code.

The intended path is:

```text
MathFunction
    |
    +--> MathFunction::contract()       (declarative domain and error claim)
    |
    +--> ScalarProgram::try_from(...)   (typed backend-neutral SSA)
              |
              +--> language graph / OGDL Elementwise program
              +--> recipe-ops scalar lowering and materialization
              +--> recipe-primitives ScalarMap stage
              +--> recipe-kernel LLVM lowering for AMDGPU or NVPTX
              +--> planner, prepare, and native executor
```

The program is the executable part of the contract. Domain checks are emitted
as `ScalarOpcode::Require` instructions, so an invalid element raises the
preallocated device fault channel instead of relying on host-side validation.

## Manifest and module graph

[`Cargo.toml`](../Cargo.toml) declares package `recipe-math` version `0.1.0`,
Rust edition 2024, MIT licensing, and the description
`Recipe-owned deterministic scalar math programs`. It has exactly two path
dependencies:

| Dependency | Boundary role |
| --- | --- |
| `recipe-core` | Owns `DType`, `ScalarOpcode`, `ScalarProgram`, scalar literals, and validation. |
| `recipe-language` | Owns `ScalarProgramBuilder`, typed `ScalarExpression`, and `LanguageError`. |

The manifest forbids unsafe Rust. There are no feature flags, build scripts,
native libraries, generated files, examples, or tests in this crate. The root
facade re-exports it as `recipe::engine::math`, and the root package also
declares the path dependency as `recipe-math`.

The implementation graph is intentionally two modules behind a small facade:

```text
src/lib.rs
├── src/contract.rs
│   └── public contract records, MathFunction, and const contract methods
└── src/program.rs
    └── private ScalarProgram construction and approximation emitters

contract.rs  --> (no crate dependency; only core language primitives in types)
program.rs   --> recipe-core + recipe-language + contract::MathFunction
lib.rs        --> re-exports contract records and TryFrom facade
```

`contract.rs` is the single source for the shipped function set, arities,
finite domains, signed-zero statements, numerical error bounds, and algorithm
identities. `program.rs` is the single source for the corresponding SSA graph
construction. `lib.rs` keeps construction private, re-exports contract values,
and provides the one specialized helper for gradual exponential underflow.

### Source map

| Region | Responsibility |
| --- | --- |
| [`src/lib.rs`](../src/lib.rs#L1-L43) | Crate policy, public re-exports, gradual-underflow helper, and `TryFrom` implementation. |
| [`src/contract.rs`](../src/contract.rs#L1-L63) | Contract record types and field meanings. |
| [`src/contract.rs`](../src/contract.rs#L65-L193) | `MathFunction`, static domain slices, and the 22-variant surface. |
| [`src/contract.rs`](../src/contract.rs#L195-L294) | `ALL`, `arity`, `contract`, and versioned algorithm identities. |
| [`src/contract.rs`](../src/contract.rs#L296-L409) | Finite-domain and signed-zero dispatch. |
| [`src/contract.rs`](../src/contract.rs#L411-L520) | Absolute/relative error bounds and approximation notes. |
| [`src/program.rs`](../src/program.rs#L1-L136) | Builder entry point, function dispatch, and per-function domain checks. |
| [`src/program.rs`](../src/program.rs#L138-L212) | Shared `Require`, zero-preservation, and Horner helpers. |
| [`src/program.rs`](../src/program.rs#L213-L400) | Trigonometric, atan2, exponential, and underflow emitters. |
| [`src/program.rs`](../src/program.rs#L403-L515) | `expm1`, `log`, and `log1p` emitters. |
| [`src/program.rs`](../src/program.rs#L518-L607) | Rounding, sign, sigmoid, tanh, softplus, erf, and sign-copy helpers. |

## Public surface and intent vocabulary

| Public item | Intent | Result |
| --- | --- | --- |
| `AlgorithmIdentity { name, version }` | Identify one implementation and coefficient set. | Stable static name plus integer version. `Pow` is version 3; every other shipped function is version 1. |
| `FiniteBound` | Describe one interval endpoint. | `Unbounded`, `Inclusive(f32)`, or `Exclusive(f32)`. |
| `FiniteInputDomain` | Describe one named scalar argument. | Argument name, lower and upper bounds, and a `nonzero` marker. |
| `FiniteDomain` | Describe all arguments and their relationship. | Static input slice plus a human-readable relation. |
| `NonFiniteBehavior` | Describe non-finite handling. | The current sole variant, `RejectWithRequire`, means NaN and either infinity set the device fault flag. |
| `SpecialValueBehavior` | Describe behavior outside ordinary finite intervals. | Non-finite policy, signed-zero statement, and domain-violation statement. |
| `ErrorBound` | State the approximation claim. | Maximum absolute and relative error plus the reason for the bound. A pair of zero bounds means exact composition of shipped scalar primitives. |
| `MathContract` | Bundle the complete declaration. | Domain, special values, error claim, and algorithm identity. |
| `MathFunction` | Closed registry of the 22 shipped functions. | `ALL`, `arity`, `contract`, and `algorithm` are all `const` and deterministic. |
| `exp_with_gradual_underflow_program()` | Name the one specialized public constructor. | `Result<ScalarProgram, LanguageError>` for `MathFunction::ExpWithGradualUnderflow`. |
| `TryFrom<MathFunction> for ScalarProgram` | General constructor boundary. | `ScalarProgram::try_from(function)` calls the private builder and returns a validated program or `LanguageError`. |

No public function evaluates an `f32`. A caller receives an SSA description and
must pass it through the normal language, primitive, planning, and native
execution boundaries.

## Function contract inventory

The table below is the normative content of `MathFunction::contract()`. Every
row has one or two `F32` inputs and one `F32` output. `finite` is implicit for
every input because `build` emits `IsFinite` followed by `Require` before the
function-specific graph. Bounds shown in the domain column are inclusive unless
the table says otherwise.

| Function | Arity | Finite domain and relation | Signed zero | Error bound `(abs, rel)` | Algorithm identity |
| --- | ---: | --- | --- | --- | --- |
| `Reciprocal` | 1 | `x != 0` | Both zeros rejected | `(0, 0)` | `recipe.math.reciprocal.ieee-divide-f32@1` |
| `ReciprocalSquareRoot` | 1 | `x > 0` | Both zeros rejected | `(0, 0)` | `recipe.math.rsqrt.sqrt-divide-f32@1` |
| `Sin` | 1 | `-8192 <= x <= 8192` | Input signed zero preserved exactly | `(2e-4, 2e-4)` | `recipe.math.sin.cody-waite-s11-f32@1` |
| `Cos` | 1 | `-8192 <= x <= 8192` | Either zero maps to `+1` | `(2e-4, 2e-4)` | `recipe.math.cos.cody-waite-c12-f32@1` |
| `Tan` | 1 | `-1.4 <= x <= 1.4`, excluding poles | Input signed zero preserved exactly | `(5e-4, 5e-4)` | `recipe.math.tan.cody-waite-ratio-f32@1` |
| `Atan2` | 2 | finite `y`, finite `x`, not both zero | Signed `y` selects the signed x-axis result | `(3e-5, 3e-5)` | `recipe.math.atan2.octant-atan15-f32@1` |
| `Exp` | 1 | `-80 <= x <= 80` | Either zero maps to `+1` | `(2e-6, 5e-6)` | `recipe.math.exp.bitpow2-taylor7-f32@1` |
| `ExpWithGradualUnderflow` | 1 | every finite `x <= 0`; round-to-nearest-even underflow | Either zero maps to `+1` | `(2e-6, 5e-6)` | `recipe.math.exp.split-scale-subnormal-f32@1` |
| `ExpMinusOne` | 1 | `-80 <= x <= 80` | Input signed zero preserved exactly | `(3e-6, 7e-6)` | `recipe.math.expm1.hybrid-taylor8-f32@1` |
| `Log` | 1 | `x > 0`, including positive normals and subnormals | Both zeros rejected | `(6e-6, 6e-6)` | `recipe.math.log.bitdecompose-atanh13-f32@1` |
| `LogOnePlus` | 1 | `x > -1` | Input signed zero preserved exactly | `(8e-6, 8e-6)` | `recipe.math.log1p.hybrid-series12-f32@1` |
| `Floor` | 1 | every finite binary32 value | Input signed zero preserved exactly | `(0, 0)` | `recipe.math.floor.scalar-primitive-f32@1` |
| `Ceil` | 1 | every finite binary32 value | Input signed zero preserved exactly | `(0, 0)` | `recipe.math.ceil.scalar-primitive-f32@1` |
| `RoundNearestEven` | 1 | every finite binary32 value | Input signed zero preserved exactly | `(0, 0)` | `recipe.math.rne.scalar-primitive-f32@1` |
| `Trunc` | 1 | every finite binary32 value | Input signed zero preserved exactly | `(0, 0)` | `recipe.math.trunc.floor-ceil-select-f32@1` |
| `Fmod` | 2 | finite dividend, finite nonzero divisor | Zero dividend preserves its sign | `(0, 0)` | `recipe.math.fmod.ieee-remainder-f32@1` |
| `Pow` | 2 | `base > 0`; `exponent * log(base) <= 80`; negative results may underflow to `+0` | Either base zero is out of domain; an in-domain zero exponent maps to `+1` | `(3e-5, 2e-4)` | `recipe.math.pow.log-exp-subnormal-v3-f32@3` |
| `Sign` | 1 | every finite binary32 value | Zero is returned with its input sign | `(0, 0)` | `recipe.math.sign.ordered-select-f32@1` |
| `Sigmoid` | 1 | `-80 <= x <= 80` | Either zero maps to `+0.5` | `(4e-6, 6e-6)` | `recipe.math.sigmoid.stable-exp-f32@1` |
| `Tanh` | 1 | `-40 <= x <= 40`; `|2*x|` stays in exp range | Input signed zero preserved exactly | `(8e-6, 1e-5)` | `recipe.math.tanh.stable-expm1-f32@1` |
| `Softplus` | 1 | `-80 <= x <= 80` | Either zero maps to positive `ln(2)` | `(1e-5, 1e-5)` | `recipe.math.softplus.stable-log1p-f32@1` |
| `Erf` | 1 | `-6 <= x <= 6` | Input signed zero preserved exactly | `(2e-6, 2e-6)` | `recipe.math.erf.as-7.1.26-f32@1` |

The `FiniteInputDomain::nonzero` field is metadata for the named argument; the
program still emits an explicit `NotEqual` check for each function that needs
it. The `relation` string supplies cross-argument conditions that cannot be
represented by one interval, such as the `Atan2` pair and the `Pow` product
bound. The contract data are static and do not read configuration or hardware.

## Program construction

`program::build` is the only constructor used by `TryFrom<MathFunction>`:

1. Create one `ScalarProgramBuilder`.
2. Allocate `function.arity()` `DType::F32` inputs in argument order.
3. For every input, emit `IsFinite` and `Require`.
4. Dispatch the function to `emit_function`, which emits additional domain
   checks and the approximation graph.
5. Finish with one output expression. `ScalarProgramBuilder::finish` runs
   `ScalarProgram::validate` before returning.

The builder gives values a local owner token, starts IDs at one, and assigns
IDs in call order. It rejects an expression borrowed from another builder,
rejects an invalid opcode signature immediately, and rejects a foreign output
at `finish`. Consequently, the generated instruction order, constants, and
value IDs are deterministic for one function implementation.

### Domain checks and fault semantics

The private helpers have one job each:

| Helper | Emitted condition |
| --- | --- |
| `require_finite_inputs` | `IsFinite(input)` for every argument, then `Require`. |
| `require_nonzero` | `value != 0`, then `Require`. |
| `require_positive` | `value > 0`, then `Require`. |
| `require_closed_interval` | `value >= lower` and `value <= upper`, each required. |
| `require_atan2_nonzero_pair` | `(y != 0) OR (x != 0)`, then `Require`. |
| `preserve_zero` | Ordered equality with `0`; select the original input when zero, preserving its sign bit. |
| `copy_sign_from` | Bitcast sign source, shift the sign bit, and select the positive or negated magnitude. |

`Require` normalizes an integer predicate to zero or one and records a
rejection. In generated LLVM, all rejection conditions are combined into one
atomic OR to a preallocated device `i32` fault flag after scalar instructions
have been emitted. Invalid inputs therefore report a fault through the normal
execution contract. The scalar math layer does not throw, branch to host code,
or substitute a fallback value. Later layers may make an invalid arithmetic
result safe enough to store, but the fault flag remains the authoritative
failure signal.

Because `require_finite_inputs` runs for every variant, every program produced
by this crate contains at least one `Require` and therefore
`ScalarProgram::requires_fault_flag()` is true. The primitive lowerer allocates
one `ProgramFaultFlag` for each such scalar map, and core, scheduler, executor,
and native launch validation require the corresponding binding and readback.

### Approximation emitters

The approximation code is deliberately expressed only in `ScalarOpcode`:

* `horner` builds a coefficient chain with `Fma`, preserving one explicit
  rounding per polynomial step.
* `emit_sine_cosine_core` uses split `pi/2` constants, round-to-nearest-even
  Cody-Waite reduction, odd/even polynomials, and a quadrant select table.
* `emit_atan2_core` compares absolute arguments, swaps into the first octant,
  applies the transform `(r - 1) / (r + 1)` above `0.41421357`, evaluates an
  odd degree-15 polynomial, then restores quadrant and the sign of `y`.
* `emit_exp_reconstruction` rounds `x * log2(e)`, subtracts split `ln(2)`
  with two FMAs, evaluates a degree-7 residual polynomial, and constructs
  `2^k` by integer bit operations. `emit_exp_core` uses the normal range only.
* `emit_exp_with_underflow` clamps before float-to-int conversion, splits
  exponents below `-126` into `2^(k+64) * 2^-64`, and selects positive zero
  below `ln(2^-150)`. The strict comparison makes the half-minimum-subnormal
  tie round to zero under round-to-nearest-even.
* `emit_expm1_core` uses a degree-8 local series for `|x| <= 0.25` and
  `exp(x)-1` outside it.
* `emit_log_core` scales positive subnormals by `2^23`, extracts exponent and
  mantissa bits, reduces the mantissa around `sqrt(2)`, and evaluates the
  atanh series through `y^13` with split `ln(2)`.
* `emit_log1p_core` uses a degree-12 local series for `|x| <= 0.25` and
  `log(1+x)` outside it.
* `emit_trunc` computes both floor and ceiling and selects by the sign of the
  input. `emit_sign` selects `-1`, `+1`, or the original zero.
* `emit_sigmoid_core` computes a sign-stable quotient, using `exp(-x)` on the
  nonnegative branch and `exp(x)` on the negative branch.
* `emit_tanh_core` computes `expm1(2*abs(x)) / (expm1(2*abs(x)) + 2)` and
  restores the sign. `emit_softplus_core` computes
  `max(x, 0) + log1p(exp(-abs(x)))`.
* `emit_erf_core` is Abramowitz-Stegun 7.1.26 evaluated with Recipe's own
  exponential and then sign-restored.

The coefficients and split constants are intrinsic to these named algorithms;
they are not duplicated in callers and are not configuration values. The only
integer bit constructions are representation operations demanded by binary32
range reduction and do not introduce another calculation ontology.

## Consumer graph

### Operation registry and scalar composition

`recipe-ops` is the first consumer. `ops/src/scalar.rs` maps the following
legacy operation symbols to `ScalarRecipe::Math`:

| Symbol | Function | Legacy source in `operation-surface.txt` |
| --- | --- | --- |
| `gpu_atan2` | `Atan2` | `gpu-core/src/math_ops.rs:238` |
| `gpu_ceil` | `Ceil` | `gpu-core/src/math_ops.rs:303` |
| `gpu_cos` | `Cos` | `gpu-core/src/math_ops.rs:206` |
| `gpu_exp` | `Exp` | `gpu-core/src/kernels.rs:5450` |
| `gpu_expm1` | `ExpMinusOne` | `gpu-core/src/math_ops.rs:271` |
| `gpu_floor` | `Floor` | `gpu-core/src/math_ops.rs:287` |
| `gpu_fmod` | `Fmod` | `gpu-core/src/math_ops.rs:351` |
| `gpu_log_into` | `Log` | `gpu-core/src/kernels.rs:3008` |
| `gpu_log1p` | `LogOnePlus` | `gpu-core/src/math_ops.rs:255` |
| `gpu_pow` | `Pow` | `gpu-core/src/kernels.rs:5498` |
| `gpu_reciprocal` | `Reciprocal` | `gpu-core/src/math_ops.rs:140` |
| `gpu_round` | `RoundNearestEven` | `gpu-core/src/math_ops.rs:319` |
| `gpu_rsqrt` | `ReciprocalSquareRoot` | `gpu-core/src/math_ops.rs:124` |
| `gpu_sigmoid_into` | `Sigmoid` | `gpu-core/src/kernels.rs:2420` |
| `gpu_sign_into` | `Sign` | `gpu-core/src/kernels.rs:5482` |
| `gpu_sin` | `Sin` | `gpu-core/src/math_ops.rs:190` |
| `gpu_softplus` | `Softplus` | `gpu-core/src/k_gapact.rs:250` |
| `gpu_tan` | `Tan` | `gpu-core/src/math_ops.rs:222` |
| `gpu_tanh_into` | `Tanh` | `gpu-core/src/kernels.rs:2457` |
| `gpu_trunc` | `Trunc` | `gpu-core/src/math_ops.rs:335` |

`ExpWithGradualUnderflow` and `Erf` are not legacy operation symbols. The
former has a dedicated public constructor for inference; the latter is a
public Recipe math function available through `MathFunction` but has no
operation-registry mapping in this workspace.

`OperationRegistry::describe` first asks `ScalarRecipe::for_symbol`. A scalar
math descriptor advertises an exact `F32` input/output contract, a
`PerElementExactOrder` determinism contract, and a definition equal to the
algorithm identity name. `recipe_ops::lower_scalar` then calls
`ScalarProgram::try_from(function)`, validates the returned program again, and
wraps a construction failure as `OperationErrorKind::InvalidScalarProgram`.

Composite scalar recipes use the same math programs inline. The `Composer`
path calls `inline_math`, checks arity and input dtypes, remaps every generated
constant into the composer's value namespace, replays each instruction through
`ScalarOpcode::result_dtype`, and checks the declared output. This is used by
binary cross-entropy, focal loss, ELU, SiLU, GELU, scaled exponential,
reparameterization, KL divergence, and the mean-absolute-error sign gradient.
Inlining is the only composition mechanism: it does not create a nested
program call or a second backend path.

### Materialized operations

The operation materializers instantiate `ScalarProgram` values directly and
wrap them in `PrimitiveKind::Elementwise` stages. Current direct uses are:

* `attention_sequence_embedding`: `Sin` and `Cos` for verified positional
  angles, and `Exp` in causal softmax after a safe shift.
* `loss_metrics`: `Log` after targets are mapped to a positive-or-one value in
  KL-divergence loss.
* `graph_cluster_rl`: `Exp` for Gaussian standard deviations and categorical
  softmax exponentials, and `Log` for categorical log sums.

Each materializer maps a `LanguageError` to the operation's
`GraphMaterializationFailed` or language error with the descriptor ID. Shape,
dtype, finite-parameter, and verification predicates are checked at the
materialization boundary; the math program still owns its elementwise domain
checks at runtime.

### Inference and the gradual-underflow program

`training/src/inference.rs` uses
`exp_with_gradual_underflow_program()` in two target-free dense paths:

1. Stable sigmoid maps a logit to `-abs(logit)`, computes the gradual
   exponential, and selects `1/(1+e)` or `e/(1+e)` by the original logit's
   sign.
2. Stable softmax reduces each row to its maximum, subtracts it, requires the
   shifted value to be nonpositive, replaces non-finite shifted values with
   `-104`, computes the gradual exponential, and divides by the row sum.

The surrounding programs prove the precondition that the math function needs.
The gradual constructor still emits its own finite and nonpositive checks, so
an invalid row sets the same device fault flag instead of being silently
accepted. The compiled inference graph is validated, serialized to OGDL, read
back, and converted to a `StaticCalculationProgram`; the math program is thus
carried through the normal graph round trip rather than evaluated during
compilation.

### Language, OGDL, primitives, and kernel lowering

`recipe-language` treats a math result as an ordinary `Elementwise { program }`.
`Elementwise` validation checks the scalar program, input and output arity,
input dtypes, broadcast shape, output dtypes, and output shape. The OGDL codec
serializes inputs, bit-preserving constants, ordered instructions, opcode names,
operands, and outputs. It does not serialize `MathFunction`, `MathContract`, or
the algorithm name. A decoded program must be validated by the graph boundary;
the textual representation is the generic scalar IR and the function identity
is compile-time context.

`recipe-primitives::lower_elementwise` maps graph tensors to a
`recipe_core::KernelTemplate`, preserving the program, affine accesses, and
alias matrix. It allocates a checked arithmetic fault flag whenever
`program.requires_fault_flag()` is true, counts opcode FLOPs, and emits a
`StageKind::ScalarMap`. Primitive validation requires the stage fault presence
to equal the scalar program's requirement.

Training and inference compilers use the same boundary rather than constructing
an alternate math kernel. `training/src/compile.rs::emit_owned_scalar` and
`training/src/inference.rs::emit_owned_scalar` resolve an operation symbol with
`operation_registry().resolve_unique`, call `recipe_ops::lower_scalar`, and
insert the resulting program into an `Elementwise` node. A descriptor that is
unknown, ambiguous, unsupported, or not scalar therefore fails compilation at
the operation/program boundary. `training/src/compile.rs::emit_scalar_operation`
also accepts an already-built program for explicit composite declarations, but
both cases converge on `emit_elementwise` and the same primitive lowerer.

`recipe-kernel::lower_elementwise` validates the template, target, and launch
options, loads one value per input lane, materializes constants by exact bitcast,
lowers instructions in order, aggregates fault conditions, stores outputs, and
returns target-specific LLVM plus an explicit `KernelAbi`. It uses constrained
round-to-nearest LLVM arithmetic and IEEE denormal settings. `Require` becomes
an integer predicate and a recorded rejection. `IsFinite` and `IsNan` inspect
binary32 exponent and mantissa bits. Arithmetic and intrinsic paths that call
the emitter's `canonicalize_nan` helper map NaN to one fixed quiet NaN; exact
bitwise sign and bitcast paths preserve representation. Neither behavior
replaces the math domain fault.

The ABI order is input buffers, output buffers, optional `FaultFlag`, then the
64-bit `ElementCount`. The direct lowerer supports both AMDGPU and NVPTX and
does not call CUDA Runtime, HIP, or vendor math libraries. Stage lowering,
artifact inspection, planner binding, native argument packing, and executor
fault readback retain the same optional flag contract.

### Planner, preparation, and executor fault path

The scalar flag remains a first-class value after LLVM lowering. The planner
copies `stage.fault` into `CalculationTask::fault_flag`, excludes that value from
ordinary input/output lists, and records one fault cohort per device and loop
iteration domain. It adds exactly one `MetricPurpose::FaultReadback` task per
cohort, with direct dependencies on every checked calculation. Core plan
validation requires the flag to be one four-byte resident `I32`, requires one
exclusive metric slot, and requires every user metric and exit publication to
depend on the readback. A missing, duplicate, unused, or misordered readback is
`ValidationCode::InvalidFaultReadback`.

`kernel/src/stage.rs` verifies the deferred artifact contract before lowering a
stage and rewrites the scalar emitter's relaxed atomic OR into the stage's
canonical release `xchg` fault publication with its `ArithmeticDomain` code.
`prepare` and native argument packing preserve the ordered `FaultFlag` ABI and
the executor resets its four bytes in the init image before a run. On metric
completion, an I32 value of zero records `FaultChecked`; any nonzero code is an
`ExecutorError::DeviceFault`. There is no host-side attempt to infer which
element failed and no fallback result after a device fault.

## Invariants

The following statements are required for every generated math program:

1. `MathFunction::ALL` contains exactly the 22 enum variants, in the order
   declared in `contract.rs`; `arity()` agrees with the inputs allocated by
   `program::build`.
2. Every input and output payload is `DType::F32`. Predicates and bit
   operations are temporary `DType::I32` values only.
3. Every input is finite-checked before a function-specific operation. Domain
   checks are explicit `Require` instructions, not comments or host policy.
4. Every instruction is typed, acyclic, and uses a previously defined value.
   Constants preserve exact f32 bits, including signed zero and subnormals.
5. Every generated program has at least one output and passes
   `ScalarProgram::validate` before it leaves the crate.
6. Signed-zero behavior is implemented by bit-preserving selects or bitwise
   sign operations, not by host conversion.
7. Approximation algorithms use only the Recipe scalar opcode set. No host
   `libm`, CUDA Runtime, HIP, cuBLAS, cuDNN, NCCL, or vendor math call appears
   in the construction path.
8. All generated math programs require a preallocated fault flag. The flag is
   a device-resident four-byte I32 value and is read through the normal
   calculation-stage metric path.
9. The contract's domain, special-value text, error bound, and algorithm
   identity must remain synchronized with the emitted graph. Changing a
   coefficient or reduction method requires changing the corresponding
   `AlgorithmIdentity` version and error statement.
10. There is no fallback implementation. A language, graph, primitive,
    lowering, ABI, toolchain, or runtime mismatch is reported at its owning
    boundary.

## Failure map

| Boundary | Failure | Observable result |
| --- | --- | --- |
| `ScalarProgramBuilder::new` or ID allocation | Builder identity or scalar ID space exhausted | `LanguageErrorKind::InvalidScalarProgram`. |
| Builder `apply` | Wrong arity, mixed dtypes, foreign expression | `LanguageErrorKind::InvalidScalarProgram`. |
| Builder `finish` | Foreign output or invalid SSA graph | `LanguageErrorKind::InvalidScalarProgram`, with core validation text. |
| `ScalarProgram::validate` | Duplicate value, use before definition, bad signature, missing or unknown output | Aggregated core `ValidationErrors` with `DuplicateScalarValue`, `ScalarUseBeforeDefinition`, `ScalarArity`, `ScalarTypeMismatch`, `MissingScalarOutput`, or `UnknownScalarValue`. |
| `recipe_ops::lower_scalar` | Descriptor is not scalar, unsupported surface row, or generated program invalid | `WrongLoweringKind`, `UnsupportedLowering`, or `InvalidScalarProgram`, tied to the operation ID. |
| `Composer::inline_math` | Arity/dtype mismatch, unknown generated value, no output, or changed result dtype | `OperationErrorKind::InvalidScalarProgram`. |
| Language elementwise validation | Tensor/program arity, dtype, broadcast, or output-shape mismatch | `LanguageErrorKind::ArityMismatch`, `DTypeMismatch`, `ShapeMismatch`, or `InvalidScalarProgram`. |
| OGDL encode/decode | Unknown opcode, missing/duplicate field, malformed number, or unsupported schema value | `OgdlCodecError` with a path under `Elementwise.program`. |
| Primitive lowering/validation | Fault ABI, scalar FLOP, access, alias, or stage geometry mismatch | `LoweringError` or `ProgramValidationError`; no alternate stage is substituted. |
| Kernel lowering | Invalid template/target/options, unknown scalar value, unsupported opcode, arithmetic overflow, or LLVM audit finding | `LoweringError` before a native artifact is accepted. |
| Runtime domain violation | Non-finite input, interval violation, zero divisor, invalid positive argument, or invalid cross-argument relation | Device `FaultFlag` is atomically set and later read back by the normal schedule. |

The program may still produce a deterministic bit pattern after a rejected
element because the backend keeps unsafe arithmetic operands safe enough for
the lane to complete. That pattern is not a successful mathematical result;
the fault flag is the authoritative outcome.

## Scope boundaries

`recipe-math` owns scalar approximation formulas, coefficients, finite-domain
metadata, special-value declarations, and versioned algorithm identities. It
does not own tensor shapes, broadcasting, graph nodes, operation symbols,
materializer ABI proofs, hardware profiles, workgroup choices, native images,
launches, transfers, scheduling, or fault readbacks. Those consumers must use
the generated `ScalarProgram` through their existing validation boundaries.

The empty `.docs/src/contract.md`, `.docs/src/lib.md`, and
`.docs/src/program.md` files are not implementation inputs. This README is the
single maintained intent and structure document for the current math crate.

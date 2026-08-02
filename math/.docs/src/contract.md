---
file: math/src/contract.rs
crate: recipe-math
role: deterministic-scalar-contract
intent: >
  Publish the finite-domain, special-value, error, algorithm-identity, and
  arity contract for every Recipe-owned scalar math program.
metadata_boundary: MathFunction::contract -> MathContract
execution_boundary: MathFunction::try_from -> ScalarProgram -> Elementwise primitive
nonfinite_rule: every input is checked with IsFinite and Require
fault_rule: domain and nonfinite violations record the preallocated device fault flag
cost_rule: derive ScalarOpcode::flops from the generated program, then multiply by output element count
authority:
  - math/src/contract.rs
  - math/src/program.rs
  - math/src/lib.rs
  - core/src/scalar.rs
  - language/src/scalar_builder.rs
  - language/src/primitive.rs
  - ops/src/scalar.rs
  - kernel/src/llvm.rs
---

# `recipe_math::contract`

`math/src/contract.rs` is the public contract table for Recipe's deterministic
scalar f32 math. It does not execute an operation, inspect hardware, or perform
host-side validation. `MathFunction` is the shared key: its `contract()` method
selects the finite domain, special-value behavior, error bound, and versioned
algorithm identity, while `math/src/program.rs` uses the same key to build the
backend-neutral `ScalarProgram` that enforces the domain on the device.

The two representations are intentionally separate:

| Representation | Authority | Runtime role |
| --- | --- | --- |
| `MathContract` | Static metadata from `MathFunction::contract()` | Describes the accepted finite inputs, fault policy, numerical claim, and algorithm identity. There is no mutable contract registry. |
| `ScalarProgram` | `MathFunction` lowered by `program::build` | Performs all checks and calculations as typed SSA instructions. `Require` records a device fault when a check is false. |

`MathContract` currently has no call sites outside this crate. The generated
program, not a host-side interpretation of the metadata, is therefore the
runtime enforcement boundary.

## Public records

The records are `Copy`, immutable value descriptions. Their fields are public
so callers can inspect the complete contract without a registry lookup.

| Type | Fields | Meaning |
| --- | --- | --- |
| `AlgorithmIdentity` | `name: &'static str`, `version: u32` | Stable identity for one implementation and coefficient set. A changed algorithm must use a new version. |
| `FiniteBound` | `Unbounded`, `Inclusive(f32)`, `Exclusive(f32)` | One endpoint of a finite interval. `Unbounded` does not admit infinity because every generated program first checks `IsFinite`. |
| `FiniteInputDomain` | `name`, `lower`, `upper`, `nonzero` | Per-argument interval and an independent nonzero requirement. |
| `FiniteDomain` | `inputs: &'static [FiniteInputDomain]`, `relation` | Complete finite argument description, including cross-argument or derived-value constraints not expressible as one interval. |
| `NonFiniteBehavior` | `RejectWithRequire` | NaN and either infinity set the kernel fault flag through `Require`. This is the only current variant. |
| `SpecialValueBehavior` | `non_finite`, `signed_zero`, `domain_violation` | Behavior outside the ordinary finite interval. The current domain-violation text is `sets the preallocated fault flag through Require`. |
| `ErrorBound` | `maximum_absolute`, `maximum_relative`, `note` | Claim over the declared finite domain. A result conforms when either the absolute or relative bound holds. Both bounds equal to zero mean an exact composition of the shipped scalar primitives, not an exact real-number oracle. |
| `MathContract` | `domain`, `special_values`, `error`, `algorithm` | Complete contract for one `MathFunction`. |

## Function inventory and contract matrix

`MathFunction::ALL` is the complete ordered list of 22 shipped functions. The
arity is one for every function except `Atan2`, `Fmod`, and `Pow`, which are
binary. All arguments and the single output of a generated program are
`DType::F32`.

The `finite inputs` column describes the entries in
`FiniteDomain::inputs`. Every listed value is additionally required to be
finite. `instructions` is the number of generated SSA instructions, and
`FLOPs/element` is the sum of `ScalarOpcode::flops()` for that program. Those
two cost columns are derived facts from the current builder output, not fields
stored in `MathContract`.

| Function | Arity | Finite inputs | Relation | Signed-zero behavior | Max absolute / relative error | Algorithm identity (version) | Instructions / FLOPs per element |
| --- | ---: | --- | --- | --- | ---: | --- | ---: |
| `Reciprocal` | 1 | `x`, nonzero | `x` is finite and nonzero | Both signed zeros are rejected | `0 / 0`, exact primitive composition | `recipe.math.reciprocal.ieee-divide-f32` (1) | `5 / 2` |
| `ReciprocalSquareRoot` | 1 | `x > 0`, nonzero | `x` is finite and strictly positive | Both signed zeros are rejected | `0 / 0`, exact primitive composition | `recipe.math.rsqrt.sqrt-divide-f32` (1) | `6 / 3` |
| `Sin` | 1 | `x in [-8192, 8192]` | Cody-Waite reduction is bounded to `|x| <= 8192` | Input signed zero is preserved exactly | `2e-4 / 2e-4`, Cody-Waite plus odd/even polynomial | `recipe.math.sin.cody-waite-s11-f32` (1) | `40 / 42` |
| `Cos` | 1 | `x in [-8192, 8192]` | Cody-Waite reduction is bounded to `|x| <= 8192` | Either signed zero maps to positive one in-domain | `2e-4 / 2e-4`, Cody-Waite plus odd/even polynomial | `recipe.math.cos.cody-waite-c12-f32` (1) | `38 / 41` |
| `Tan` | 1 | `x in [-1.4, 1.4]` | The interval excludes every tangent pole | Input signed zero is preserved exactly | `5e-4 / 5e-4`, sine/cosine ratio away from poles | `recipe.math.tan.cody-waite-ratio-f32` (1) | `41 / 43` |
| `Atan2` | 2 | `y`, `x`, unbounded intervals | `y` and `x` are finite and not both zero | Signed `y` zero selects the signed `x`-axis result; `(0, 0)` is rejected | `3e-5 / 3e-5`, octant reduction with degree-15 odd atan polynomial | `recipe.math.atan2.octant-atan15-f32` (1) | `47 / 47` |
| `Exp` | 1 | `x in [-80, 80]` | Range-reduction exponent remains a normal binary32 value | Either signed zero maps to positive one in-domain | `2e-6 / 5e-6`, normal power-of-two reconstruction with degree-7 residual polynomial | `recipe.math.exp.bitpow2-taylor7-f32` (1) | `22 / 24` |
| `ExpWithGradualUnderflow` | 1 | `x <= 0`, lower unbounded | Every finite nonpositive `x` is accepted; underflow uses round-to-nearest-ties-to-even | Either signed zero maps to positive one in-domain | `2e-6 / 5e-6`, split-scale reconstruction through binary32 subnormals | `recipe.math.exp.split-scale-subnormal-f32` (1) | `28 / 28` |
| `ExpMinusOne` | 1 | `x in [-80, 80]` | Range-reduction exponent remains a normal binary32 value | Input signed zero is preserved exactly | `3e-6 / 7e-6`, local degree-8 series and range-reduced exp | `recipe.math.expm1.hybrid-taylor8-f32` (1) | `36 / 43` |
| `Log` | 1 | `x > 0`, nonzero | All positive finite normals and subnormals are accepted | Both signed zeros are rejected | `6e-6 / 6e-6`, bit decomposition with atanh series through `y^13` | `recipe.math.log.bitdecompose-atanh13-f32` (1) | `36 / 30` |
| `LogOnePlus` | 1 | `x > -1` | `1 + x` is strictly positive | Input signed zero is preserved exactly | `8e-6 / 8e-6`, local degree-12 series and bit-decomposed log | `recipe.math.log1p.hybrid-series12-f32` (1) | `54 / 57` |
| `Floor` | 1 | any finite `x` | Every finite binary32 value is accepted | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.floor.scalar-primitive-f32` (1) | `3 / 1` |
| `Ceil` | 1 | any finite `x` | Every finite binary32 value is accepted | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.ceil.scalar-primitive-f32` (1) | `3 / 1` |
| `RoundNearestEven` | 1 | any finite `x` | Every finite binary32 value is accepted | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.rne.scalar-primitive-f32` (1) | `3 / 1` |
| `Trunc` | 1 | any finite `x` | Every finite binary32 value is accepted | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.trunc.floor-ceil-select-f32` (1) | `6 / 3` |
| `Fmod` | 2 | `dividend` any finite; `divisor`, nonzero | The finite divisor is nonzero | A zero dividend preserves its sign | `0 / 0`, exact primitive composition | `recipe.math.fmod.ieee-remainder-f32` (1) | `9 / 3` |
| `Pow` | 2 | `base > 0`, nonzero; `exponent` any finite | `base > 0` and `exponent * log(base) <= 80`; binary32 underflow rounds to positive zero | A signed-zero base is rejected; a signed-zero exponent yields positive one | `3e-5 / 2e-4`, positive-base log plus split-scale exp through subnormals | `recipe.math.pow.log-exp-subnormal-v3-f32` (3) | `65 / 59` |
| `Sign` | 1 | any finite `x` | Every finite binary32 value is accepted | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.sign.ordered-select-f32` (1) | `6 / 2` |
| `Sigmoid` | 1 | `x in [-80, 80]` | Range-reduction exponent remains a normal binary32 value | Either signed zero maps to positive `0.5` | `4e-6 / 6e-6`, sign-stable exp quotient | `recipe.math.sigmoid.stable-exp-f32` (1) | `45 / 52` |
| `Tanh` | 1 | `x in [-40, 40]` | `|2*x|` remains inside the exp range-reduction domain | Input signed zero is preserved exactly | `8e-6 / 1e-5`, sign-stable expm1 quotient | `recipe.math.tanh.stable-expm1-f32` (1) | `44 / 48` |
| `Softplus` | 1 | `x in [-80, 80]` | Range-reduction exponent remains a normal binary32 value | Either signed zero maps to positive `ln(2)` | `1e-5 / 1e-5`, `max(x,0) + log1p(exp(-abs(x)))` | `recipe.math.softplus.stable-log1p-f32` (1) | `75 / 83` |
| `Erf` | 1 | `x in [-6, 6]` | The shipped approximation is contracted over `|x| <= 6` | Input signed zero is preserved exactly | `2e-6 / 2e-6`, Abramowitz-Stegun 7.1.26 with Recipe exp | `recipe.math.erf.as-7.1.26-f32` (1) | `41 / 43` |

All non-finite inputs are rejected with `NonFiniteBehavior::RejectWithRequire`,
regardless of the interval shown above. `FiniteBound::Unbounded` therefore
means all finite values, not NaN or infinity. A domain violation has the same
device-side action for every function: `Require` contributes a rejection to the
preallocated fault flag. The output lane can still be physically written after
the flag is set, but it is outside the numerical contract and must not be used
as a successful result.

## Program construction and domain enforcement

`MathFunction` implements `TryFrom<MathFunction> for ScalarProgram` in
`math/src/lib.rs`. The conversion calls `program::build` and follows one path
for every variant:

1. Create a fresh `ScalarProgramBuilder`.
2. Allocate exactly `arity()` inputs, all with `DType::F32`.
3. Emit `IsFinite` followed by `Require` for every input before the function
   body.
4. Emit the function-specific domain checks and calculation.
5. Finish with exactly one output. `ScalarProgramBuilder::finish` validates the
   complete typed SSA graph before returning it.

The helper operations in `program.rs` are the executable form of the contract:

| Helper or pattern | Instructions | Used for |
| --- | --- | --- |
| `require_finite_inputs` | `IsFinite`, then `Require` | Every input of every function. |
| `require_nonzero` | `NotEqual(value, 0)`, then `Require` | Reciprocal and the Fmod divisor. |
| `require_positive` | `GreaterThan(value, 0)`, then `Require` | ReciprocalSquareRoot, Log, and Pow base. |
| `require_closed_interval` | Inclusive lower and upper comparisons, each followed by `Require` | Sin, Cos, Tan, Exp, ExpMinusOne, Sigmoid, Tanh, and Softplus. |
| `require_atan2_nonzero_pair` | `NotEqual` for `y` and `x`, `BitOr`, then `Require` | Rejects only `(0, 0)` after finite checks. |
| `preserve_zero` | Ordered `Equal(value, 0)` and `Select(condition, value, result)` | Retains either sign bit of an input zero. `Select` is not a short-circuit operation. |
| `copy_sign_from` | f32-to-i32 bitcast, logical sign-bit shift, negate, and select | Restores the source sign for Atan2, Tanh, and Erf. |

The function-specific checks not represented by a single interval are:

- `ExpWithGradualUnderflow` requires `x <= 0` and accepts no finite positive
  input.
- `Pow` requires a positive base, computes `power = exponent * log(base)`,
  then requires `power <= 80` before the gradual-underflow exp reconstruction.
- `LogOnePlus` requires `x > -1` before forming `1 + x`.
- `Atan2` requires `y != 0 || x != 0`.

`Require` normalizes an int32 predicate and records a fault atomically in the
preallocated device flag when the predicate is zero. It does not allocate an
exception, call the host, or create an alternate result. Because the checks are
SSA instructions, invalid inputs do not create a second implementation path.

## Algorithm cores

The following are the implementation choices behind the algorithm names in the
matrix. Constants are inserted as bit-preserving f32 literals by the builder.

| Functions | Core emitted by `program.rs` |
| --- | --- |
| `Sin`, `Cos`, `Tan` | Cody-Waite reduction uses `2/pi`, split high and low `pi/2`, and round-to-nearest-even. Odd and even Horner polynomials produce sine and cosine, then quadrant selects apply signs. `Tan` divides the two results. |
| `Atan2` | Absolute values choose the smaller-to-larger ratio. Ratios above `0.41421357` use `(r - 1)/(r + 1)` plus `pi/4`; a degree-15 odd polynomial is reflected through octants and receives the sign of `y`. |
| `Exp` | Round `x * log2(e)` to an integer, split the residual with high and low `ln(2)`, evaluate a degree-7 Taylor polynomial, and construct `2^k` by biased exponent bits. |
| `ExpWithGradualUnderflow` | Detect values below `-103.97208`, clamp before f32-to-i32 conversion, split exponents below `-126` by `+64` and a `2^-64` tail scale, and select positive zero below the half-minimum-subnormal cutoff. |
| `ExpMinusOne` | For `|x| <= 0.25`, evaluate a degree-8 local series. Otherwise use `Exp(x) - 1`. |
| `Log` | Scale positive subnormals by `2^23`, extract exponent and mantissa bits, reduce the mantissa around `sqrt(2)`, and evaluate `2*y*(1 + y^2/3 + ... + y^12/13)` with split `ln(2)`. |
| `LogOnePlus` | For `|x| <= 0.25`, use the local degree-12 alternating series. Otherwise form `1 + x` and call the same log core. |
| `Trunc` | Compute floor and ceiling, then choose ceiling for negative inputs and floor otherwise. |
| `Sign` | Ordered comparisons select `-1`, the original zero, or `+1`. |
| `Sigmoid` | Compute both `1/(1 + exp(-x))` and `exp(x)/(1 + exp(x))`, then select by `x >= 0` to avoid overflow and preserve the zero contract. |
| `Tanh` | Compute `expm1(2*abs(x))/(expm1(2*abs(x)) + 2)` and copy the source sign. |
| `Softplus` | Compute `max(x, 0) + log1p(exp(-abs(x)))`; this intentionally maps both zeros to positive `ln(2)`. |
| `Erf` | Use Abramowitz-Stegun 7.1.26 with `p = 0.3275911`, a degree-5 polynomial in `t = 1/(1 + p*abs(x))`, Recipe `Exp(-x^2)`, and copied sign. |
| `Reciprocal`, `ReciprocalSquareRoot`, `Fmod`, `Floor`, `Ceil`, `RoundNearestEven` | Compose the corresponding backend-neutral scalar primitive after the required domain checks. |
| `Pow` | Use the positive-base `Log` core, multiply by the exponent, and use gradual-underflow exp reconstruction. |

## Cost and scheduling contract

`MathContract` contains no cost field. Cost is derived from the resulting
`ScalarProgram` by the common language and kernel paths:

```text
elementwise_work = output_element_count *
                   sum(instruction.opcode.flops() for instruction in program)
```

`ScalarOpcode::flops()` prices ordinary arithmetic, remainder, min/max,
negation, absolute value, comparisons, square root, floor, ceiling, and
round-to-nearest-even as one operation; `Fma` as two operations; and `Select`,
bit operations, bitcasts, shifts, conversions, `Require`, `IsFinite`, and
`IsNan` as zero operations.
This is the base scheduler's conventional FLOP work, not an instruction count
or a latency claim. The matrix's `FLOPs per element` values are the current
generated-program result under that rule.

`language::PrimitiveKind::Elementwise::work` validates the primitive, sums the
same opcode costs, multiplies by the output element count, and reports
`LanguageErrorKind::WorkOverflow` if the checked `u64` arithmetic fails.
`kernel::llvm::lower_elementwise` repeats the per-element sum while lowering
and stores the checked total in `FlopCount`. The two paths must agree before a
stage can be scheduled. Device rates and transfer times are supplied later by
the measured planner and scheduler; Recipe math does not select a hardware
implementation or add a separate latency model.

## Operation-registry surface

`ops/src/scalar.rs` maps the following public operation symbols to
`ScalarRecipe::Math`. `ScalarRecipe::dtype_contract()` gives every entry an
exact f32 input/output contract, and `OperationDescriptor::lowering` records
`LoweringAvailability::Scalar`. Registry descriptors are deterministic with
`PerElementExactOrder`.

| Symbol | `MathFunction` | Arity | Registry family |
| --- | --- | ---: | --- |
| `gpu_atan2` | `Atan2` | 2 | Elementwise |
| `gpu_ceil` | `Ceil` | 1 | Elementwise |
| `gpu_cos` | `Cos` | 1 | Elementwise |
| `gpu_exp` | `Exp` | 1 | Elementwise |
| `gpu_expm1` | `ExpMinusOne` | 1 | Elementwise |
| `gpu_floor` | `Floor` | 1 | Elementwise |
| `gpu_fmod` | `Fmod` | 2 | Elementwise |
| `gpu_log_into` | `Log` | 1 | Elementwise |
| `gpu_log1p` | `LogOnePlus` | 1 | Elementwise |
| `gpu_pow` | `Pow` | 2 | Elementwise |
| `gpu_reciprocal` | `Reciprocal` | 1 | Elementwise |
| `gpu_round` | `RoundNearestEven` | 1 | Elementwise |
| `gpu_rsqrt` | `ReciprocalSquareRoot` | 1 | Elementwise |
| `gpu_sigmoid_into` | `Sigmoid` | 1 | Activation |
| `gpu_sign_into` | `Sign` | 1 | Elementwise |
| `gpu_sin` | `Sin` | 1 | Elementwise |
| `gpu_softplus` | `Softplus` | 1 | Activation |
| `gpu_tan` | `Tan` | 1 | Elementwise |
| `gpu_tanh_into` | `Tanh` | 1 | Activation |
| `gpu_trunc` | `Trunc` | 1 | Elementwise |

The registry's alias policy is independent of the math program: names ending
in `_into` receive `AliasContract::NoAlias`, while other `gpu_` names receive
the registry's `OperationSpecific` policy. The math builder itself never
creates tensor aliases or owns storage.

`ExpWithGradualUnderflow` and `Erf` are public `MathFunction` variants but have
no direct `operation-surface.txt` symbol in the current registry. They remain
available through `ScalarProgram::try_from`; the gradual-underflow program is
also exposed by `exp_with_gradual_underflow_program()`.

## Producers and consumers

### Producers

- `MathFunction::contract`, `arity`, `algorithm`, and the private domain,
  signed-zero, and error match arms in `math/src/contract.rs` produce the
  metadata.
- `program::build` in `math/src/program.rs` produces the executable
  `ScalarProgram`; `TryFrom<MathFunction>` is the public construction boundary.
- `ops::ScalarRecipe::for_symbol` produces registry recipes for the 20 symbols
  listed above. `ops::lower_scalar` then turns a scalar descriptor into an
  owned, validated program.
- The operation materializers directly construct programs for specialized
  graph stages. `attention_sequence_embedding::emit_positional_encoding`
  uses `Sin` and `Cos`, `attention_sequence_embedding::emit_causal_softmax`
  uses `Exp`, `loss_metrics::emit_kl_divergence_loss` uses `Log`, and
  `graph_cluster_rl::emit_gaussian_log_probability` and
  `graph_cluster_rl::emit_categorical_log_probability` use `Exp` and `Log`.
- `training/src/inference.rs` directly calls
  `exp_with_gradual_underflow_program()` for stable sigmoid and stable softmax
  exponentials. Stable sigmoid forms `-abs(logit)`; stable softmax requires a
  nonpositive shifted logit and substitutes a safe `-104` under a recorded
  non-finite fault.
- `src/facade.rs` re-exports the crate as `recipe::engine::math`; it adds no alternate
  implementation.

### Consumers

- `ops::lower_scalar` validates the returned program and places it in an
  elementwise `PrimitiveKind`. `Composer::inline_math` uses the same
  `ScalarProgram::try_from` path for composite losses, activations, and
  optimizer calculations, then checks arity, input dtypes, value references,
  and output type while splicing the instructions into one SSA namespace.

The current `Composer::inline_math` call sites are:

| Consumer in `ops/src/scalar.rs` | Inlined functions | Role |
| --- | --- | --- |
| `binary_cross_entropy_with_logits_program` | `Softplus`, `Sigmoid` | Stable loss and logit gradient. |
| `canonical_focal_with_logits_program` | `Sigmoid` twice, `Softplus` | Parameterless focal loss and gradient. |
| `CompositeScalar::ScaledExp` | `Exp` | Scaled exponential. |
| `CompositeScalar::Reparameterize` | `Exp` | Standard-deviation reconstruction. |
| `CompositeScalar::MeanAbsoluteErrorGradient` | `Sign` | Signed prediction error. |
| `elu` and `elu_derivative` | `Exp` | ELU forward and derivative. |
| `silu` and `silu_backward` | `Sigmoid` | SiLU forward and derivative. |
| `gelu_tanh` and `gelu_tanh_backward` | `Tanh` | Tanh-approximated GELU forward and derivative. |
| `kl_divergence` | `Exp` | Variance from log variance. |

`CompositeScalar::EluBackward`, `Selu`, `SeluBackward`, `GeluTanhMultiply`,
`GluGeluTanh`, and `GluSilu` reach these helpers transitively. The inliner
copies constants and instructions into the caller's builder, so it does not
introduce a nested program or a second runtime boundary.
- `language::PrimitiveKernel::validate` checks that kernel input/output dtypes
  and counts match the scalar program. Its `work` method supplies the cost used
  by planning.
- `kernel::llvm::lower_elementwise` lowers the validated program to strict IEEE
  f32 LLVM for AMDGPU or NVPTX. A program containing `Require` receives the
  preallocated fault-flag argument, and all recorded rejection predicates are
  atomically ORed into that flag.
- Planner, scheduler, and native execution consume the resulting elementwise
  stage and its `FlopCount`; they do not reinterpret `MathContract` or replace
  a math function with a vendor library.

## Invariants and errors

The following invariants are required by the source and by the consumers above:

1. `MathFunction::ALL` contains every enum variant exactly once. Any new
   variant must be added to `ALL`, `arity`, `contract`, `algorithm`, the
   domain, signed-zero, and error matches, and `program::emit_function`.
2. Every generated input and output is f32, every function has one output, and
   the input count equals `arity()`.
3. Every input is checked for finiteness before the function body. All finite
   domain restrictions are emitted as `Require`; there is no host-side
   fallback, retry, or alternate implementation.
4. Constants are embedded in the scalar program as f32 or i32 literals. The
   builder preserves f32 bit patterns, including signed zero and subnormals.
5. The algorithm name and version are part of the public identity. `Pow` is
   currently version 3; every other shipped function is version 1.
6. `Select` evaluates already-materialized SSA operands and is not a branch.
   Domain checks therefore communicate rejection through the fault channel,
   not by promising that invalid arithmetic is never formed.
7. All three construction layers use checked validation: the math builder,
   `ScalarProgram::validate`, and the enclosing kernel template validation.

Construction and lowering errors are ordinary `Result` errors:

| Boundary | Error behavior |
| --- | --- |
| `ScalarProgramBuilder::new`, `input`, constants, or opcode application | Returns `LanguageError` with `LanguageErrorKind::InvalidScalarProgram` if builder identity or scalar-value identity space is exhausted, a value belongs to another builder, or an opcode signature is invalid. |
| `ScalarProgramBuilder::finish` | Returns `LanguageErrorKind::InvalidScalarProgram` after `ScalarProgram::validate` reports duplicate values, use before definition, wrong arity or type, missing output, or an unknown output. |
| `TryFrom<MathFunction> for ScalarProgram` and `exp_with_gradual_underflow_program` | Propagate the language error. They do not return a host numerical-domain error because domain checks belong in the program. |
| `ops::lower_scalar` | Wraps construction or validation failures as `OperationErrorKind::InvalidScalarProgram`; a descriptor with a non-scalar lowering returns `WrongLoweringKind`, and an explicitly unsupported descriptor returns `UnsupportedLowering`. |
| Runtime non-finite or domain-invalid lane | `Require` sets the device fault flag. It does not produce a Rust `Result` or a host exception. A run must treat the flagged result as rejected. |
| Elementwise work calculation | Checked multiplication and addition return `LanguageErrorKind::WorkOverflow` if the per-element cost times the output element count cannot fit in `u64`. |

These errors are distinct from numerical error bounds. A bound describes an
accepted finite input; it does not authorize an out-of-domain value or clear a
device fault. Likewise, a zero error bound means the selected primitive
composition is exact for Recipe's scalar semantics, not that a transcendental
oracle is being called.

---
file: math/src/program.rs
crate: recipe-math
role: deterministic-scalar-program-construction
intent: >
  Build every shipped MathFunction as one typed, backend-neutral ScalarProgram
  whose ordered F32 SSA instructions contain the numerical algorithm, finite
  domain guards, signed-zero handling, and one output value.
construction_boundary: MathFunction::try_from -> program::build -> ScalarProgramBuilder::finish
evaluation_boundary: ScalarProgram -> PrimitiveKind::Elementwise -> KernelTemplate -> LLVM -> native-image
interpreter_rule: no_host_or_vendor_math_interpreter_exists; evaluation_is_native_lane_execution
domain_rule: every_input_is_checked_with_IsFinite_and_Require_before_function_specific_guards
fault_rule: Require_records_a_device_fault_but_lane_continues_with_the_emitted_safe_value_until_fault_readback
cost_rule: sum_ScalarOpcode_flops_per_instruction_then_multiply_by_output_element_count_with_checked_u64_arithmetic
authority:
  - math/src/program.rs
  - math/src/lib.rs
  - math/src/contract.rs
  - core/src/scalar.rs
  - language/src/scalar_builder.rs
  - ops/src/scalar.rs
  - primitives/src/lower.rs
  - kernel/src/llvm.rs
---

# Recipe math program construction

This document is the implementation contract for
[`math/src/program.rs`](../../src/program.rs). The file owns the deterministic
instruction construction for the 22 [`MathFunction`](../../src/contract.rs)
variants. It does not own a tensor, a buffer view, a device, a launch, a
compiler invocation, or a host-side numeric evaluator. It emits ordinary
[`recipe_core::ScalarProgram`](../../../core/src/scalar.rs) records and leaves
all placement and execution to the language, primitive, kernel, and executor
layers.

The normative catalogue is split deliberately:

* [`math/src/contract.rs`](../../src/contract.rs) states arity, finite domain,
  special-value behavior, algorithm identity, and the absolute or relative
  error claim.
* `program.rs` is the executable construction of that contract. Its guards and
  algorithms are scalar instructions, not metadata that a later layer may
  ignore.
* [`core/src/scalar.rs`](../../../core/src/scalar.rs) defines the SSA records,
  opcode signatures, validation, and FLOP prices.
* [`language/src/scalar_builder.rs`](../../../language/src/scalar_builder.rs)
  allocates owned typed expressions and validates the finished program.

## Parseable structure

```yaml
structure:
  constants: [range_reduction_splits, underflow_cutoff, polynomial_constants]
  entry:
    function: build
    input: MathFunction
    output: ScalarProgram
    steps:
      - allocate ScalarProgramBuilder
      - allocate arity(function) F32 inputs
      - append IsFinite then Require for every input
      - dispatch emit_function(function)
      - finish one output and validate
  shared_helpers:
    - require_finite_inputs
    - require_nonzero
    - require_positive
    - require_closed_interval
    - require_condition
    - require_atan2_nonzero_pair
    - preserve_zero
    - horner
    - copy_sign_from
  algorithm_helpers:
    - emit_sine_cosine_core
    - emit_atan_polynomial
    - emit_atan2_core
    - emit_exp_core
    - emit_exp_with_underflow
    - emit_exp_reconstruction
    - emit_expm1_core
    - emit_log_core
    - emit_log1p_core
    - emit_trunc
    - emit_sign
    - emit_sigmoid_core
    - emit_tanh_core
    - emit_softplus_core
    - emit_erf_core
  output_rule: exactly_one_scalar_output_for_each_MathFunction_program
  value_rule: one_builder_owned_ScalarValueId_namespace_in_call_order
  execution_rule: no_short_circuit_or_host_evaluation; all_selected_operands_are_materialized
```

## Source map and ownership

| Source range | Item | Responsibility |
| --- | --- | --- |
| `program.rs:6-15` | Numeric constants | Bit-preserving split constants for `ln(2)`, `2/pi`, `pi/2`, the subnormal cutoff, and the 64-bit exponent split. |
| `program.rs:17-28` | `build` | Creates the builder, F32 inputs, universal finite checks, function body, and one validated output. |
| `program.rs:30-136` | `emit_function` | Exhaustive `MathFunction` dispatch. Each branch adds its domain-specific `Require` predicates and invokes a shared body helper. |
| `program.rs:138-197` | Guard and special-value helpers | Turn finite, nonzero, positive, interval, pair, and signed-zero rules into scalar instructions. |
| `program.rs:199-211` | `horner` | Constructs coefficient constants and an ordered FMA chain. |
| `program.rs:213-271` | `emit_sine_cosine_core` | Cody-Waite reduction, sine/cosine polynomials, quadrant signs, and paired result selection. |
| `program.rs:273-325` | `emit_atan_polynomial`, `emit_atan2_core` | Degree-15 odd atan approximation, octant reduction, reflection, and sign copy. |
| `program.rs:327-401` | Exponential family core | Normal range reduction and optional subnormal reconstruction with safe underflow clamping. |
| `program.rs:403-516` | `expm1`, `log`, `log1p` | Hybrid local series and range-reduced or bit-decomposed paths. |
| `program.rs:518-538` | `trunc`, `sign` | Exact scalar primitive compositions with signed-zero preservation. |
| `program.rs:541-595` | Stable activations and `erf` | Sign-stable sigmoid, tanh, softplus, and Abramowitz-Stegun erf approximation. |
| `program.rs:598-608` | `copy_sign_from` | Bitcasts the source sign, shifts it to a one-bit I32 predicate, and selects a negated or positive magnitude. |

`program.rs` is private to the math crate. The public construction boundary is
`impl TryFrom<MathFunction> for ScalarProgram` in `math/src/lib.rs:39-43`.
The specialized `exp_with_gradual_underflow_program` function at
`math/src/lib.rs:23-37` is only a named convenience wrapper around the same
`TryFrom` path.

## Constant and coefficient identities

All numeric values below are compiled as F32 constants by the builder unless
the source explicitly constructs an I32 literal. Their bit patterns become
part of the ordered `ScalarProgram` and therefore part of the algorithm and
native-artifact identity.

| Source constant | Value or expression | Used by |
| --- | --- | --- |
| `LOG_TWO_HIGH` | `0.69314575` | High split term in exponential and logarithm reconstruction. |
| `LOG_TWO_LOW` | `1.4286068e-6` | Low split term in exponential and logarithm reconstruction. |
| `LOG_TWO_RECIPROCAL` | `core::f32::consts::LOG2_E` | Base-two exponential range reduction. |
| `EXP_HALF_MIN_SUBNORMAL_LOG` | `-103.97208` | Strict underflow selector for `exp_with_underflow`. |
| `EXP_SPLIT_SCALE_BITS` | I32 `64` | Lower-exponent normal-plus-`2^-64` split. |
| `PI_OVER_TWO_HIGH` | `f32::from_bits(0x3fc9_0fda)` | High split term in Cody-Waite reduction. |
| `PI_OVER_TWO_LOW` | `7.5497894e-8` | Low split term in Cody-Waite reduction. |
| `TWO_OVER_PI` | `core::f32::consts::FRAC_2_PI` | Cody-Waite quadrant reduction. |

The Horner chains are fixed in source order. `horner(first, remaining)` emits
one constant for `first`, then one `Fma(accumulator, x, coefficient)` for each
remaining item:

```text
sine:    first 1.589691e-10; remaining [-2.505076e-8, 2.7557314e-6,
         -1.984127e-4, 8.333334e-3, -1.6666667e-1]
cosine:  first -1.135965e-11; remaining [2.0875723e-9, -2.7557314e-7,
         2.4801588e-5, -1.3888889e-3, 4.1666668e-2]
atan:    first -1.0/15.0; remaining [1.0/13.0, -1.0/11.0, 1.0/9.0,
         -1.0/7.0, 1.0/5.0, -1.0/3.0]
exp:     first 1.0/5040.0; remaining [1.0/720.0, 1.0/120.0,
         1.0/24.0, 1.0/6.0, 0.5, 1.0, 1.0]
expm1:   first 1.0/40320.0; remaining [1.0/5040.0, 1.0/720.0,
         1.0/120.0, 1.0/24.0, 1.0/6.0, 0.5, 1.0]
log:     first 1.0/13.0; remaining [1.0/11.0, 1.0/9.0, 1.0/7.0,
         1.0/5.0, 1.0/3.0, 1.0]
log1p:   first -1.0/12.0; remaining [1.0/11.0, -1.0/10.0, 1.0/9.0,
         -1.0/8.0, 1.0/7.0, -1.0/6.0, 1.0/5.0, -0.25, 1.0/3.0,
         -0.5, 1.0]
erf:     first 1.0614054; remaining [-1.4531521, 1.4214138,
         -0.28449672, 0.2548296]
```

The source uses Rust `f32` literals and compile-time arithmetic for the
fractional coefficients. A documentation decimal is descriptive; the exact
`ScalarLiteral::F32Bits` produced by `to_bits` is authoritative.

The non-polynomial literals are also intentional identity material:

| Helper | Integer or scalar literals | Meaning |
| --- | --- | --- |
| `emit_sine_cosine_core` | I32 `3`, `0`, `2` | Mask `nearest` to four quadrants and select quadrant signs. |
| `emit_atan2_core` | `0.41421357`, `1.0`, `pi/4`, `pi/2`, `0.0`, `pi` | Ratio transform threshold and quadrant reflection. |
| `emit_exp_reconstruction` | I32 bias `127`, shift `23` | Construct the normal binary32 exponent field. |
| `emit_exp_reconstruction(..., true)` | I32 `-126`, split `64`; bit pattern `(127-64)<<23`; F32 `1.0` | Detect lower normal exponents, build `2^(k+64)`, then apply `2^-64`. |
| `emit_log_core` | F32 `f32::MIN_POSITIVE`, scale `8_388_608.0`, `sqrt(2)`, `0.5`; I32 shift `23`, mask `0xff`, bias `127`, adjustment `-23`, masks `0x007f_ffff` and `0x3f80_0000`, integers `1` and `0` | Normalize subnormals and decompose exponent and mantissa bits. |
| `emit_expm1_core`, `emit_log1p_core` | F32 cutoff `0.25`; F32 `1.0` for the regular path | Select a cancellation-safe local series around zero. |
| `emit_trunc` | F32 `0.0` | Ordered negative test chooses ceiling or floor. |
| `emit_sign` | F32 `0.0`, `1.0`, `-1.0` | Preserve zero or produce an ordered sign value. |
| `emit_tanh_core` | F32 `2.0` | Convert `abs(x)` to the `expm1(2*abs(x))` form. |
| `emit_softplus_core`, `emit_erf_core` | F32 `0.0`, `1.0`; erf `p=0.3275911` | Stable maximum/correction and Abramowitz-Stegun parameter. |
| `copy_sign_from` | I32 shift `31` | Extract the source sign bit as an I32 predicate. |

These literals are not tunable runtime policy. Changing one changes the
program sequence and must be treated as an algorithm identity change.

## Construction protocol

`build(function)` has one fixed sequence:

1. `ScalarProgramBuilder::new()` obtains a process-local owner token and starts
   a fresh value namespace. Failure is a `LanguageError` if the owner counter
   cannot advance.
2. `function.arity()` determines the input count. `Atan2`, `Fmod`, and `Pow`
   allocate two F32 inputs; every other variant allocates one. There is no
   implicit broadcast, conversion, or type promotion.
3. `require_finite_inputs` appends `IsFinite(input)` followed by
   `Require(predicate)` for every input, in input order. This makes NaN and
   either infinity a runtime domain fault for every function, including the
   exact primitive wrappers.
4. `emit_function` appends the branch-specific checks and body. The input
   expressions are passed by value, so helper calls cannot switch builders or
   create a second scalar identity space.
5. `builder.finish(&[output])` verifies output ownership, assembles
   `ScalarProgram { inputs, constants, instructions, outputs }`, and calls
   `ScalarProgram::validate`. A successful math program always has one output.

`ScalarProgramBuilder` assigns IDs from one shared monotonically increasing
namespace across inputs, constants, and instruction results. Calls are emitted
in source order, so changing helper order, coefficient order, or even an
equivalent constant representation changes the program's ordered artifact
identity. `ScalarLiteral::F32Bits` retains the exact bits of every `f32`
constant, including signed zero and subnormal values.

There is no branch in `program.rs` that evaluates a float. Rust `if` is used
only for construction-time choices such as `support_subnormals` in
`emit_exp_reconstruction`. Runtime choices are represented by `Select`, and
all operands of a `Select` have already been constructed and will be lowered.

### Opcode typing used by the math builder

The math code relies on the core opcode signatures rather than performing any
manual casts:

| Value family | Current math uses | Result type |
| --- | --- | --- |
| F32 arithmetic | `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`, `Negate`, `Absolute`, `Maximum` | F32, with both operands or the unary input F32. |
| F32 approximation | `Fma`, `SquareRoot`, `Floor`, `Ceiling`, `RoundNearestEven` | F32. `Fma` is the only three-operand arithmetic opcode and performs one-rounding F32 work. |
| F32 predicates | `Equal`, `NotEqual`, `LessThan`, `LessThanOrEqual`, `GreaterThan`, `GreaterThanOrEqual`, `IsFinite` | I32 zero-or-one predicates. |
| Selection | `Select(condition, when_nonzero, when_zero)` | The two branch values retain one common dtype, F32 for math outputs or I32 for intermediate masks. |
| Representation and integer reduction | `BitAnd`, `BitOr`, `ShiftLeft`, `ShiftRightLogical`, `BitcastF32ToI32`, `BitcastI32ToF32`, `ConvertF32ToI32`, `ConvertI32ToF32` | I32 masks, I32 exponent fields, or explicitly named F32 conversions. |
| Fault boundary | `Require(I32)` | I32 normalized condition plus a recorded device rejection. |

The builder rejects a mismatched signature before appending an instruction. For
example, `Select` cannot use an F32 condition, `Fma` cannot accept the I32
quadrant values, and bitwise operations cannot consume F32 values. The few
places where an F32 value must become an I32 payload are explicit conversions
or bitcasts in the source, so an apparently integer operation in an
approximation is never an implicit host cast.

## Runtime guards and special values

The guard helpers have one instruction-level meaning:

| Helper | Emitted predicate | Runtime effect |
| --- | --- | --- |
| `require_finite_inputs` | `IsFinite(value)` then `Require` | Rejects NaN and either infinity through the shared fault channel. |
| `require_nonzero` | `NotEqual(value, +0)` then `Require` | Rejects both signed zeros because both compare equal to zero. The LLVM `une` predicate would classify NaN as not-equal, but the earlier finite guard has already rejected it. |
| `require_positive` | `GreaterThan(value, +0)` then `Require` | Requires a strictly positive finite value, rejecting both zeros and negatives. |
| `require_closed_interval` | `value >= lower`, then `value <= upper`, each followed by `Require` | Inclusive endpoints are accepted. Both checks are emitted even when the first one rejects. |
| `require_condition` | `Require(condition)` | Adds a fault-producing validation predicate and keeps the normalized I32 result unused. |
| `require_atan2_nonzero_pair` | `y != 0`, `x != 0`, `BitOr`, `Require` | Rejects only the `(zero, zero)` pair while admitting an axis value with one nonzero coordinate. |
| `preserve_zero` | `Equal(input, +0)`, `Select(is_zero, input, result)` | Returns the original input bits for either signed zero; otherwise returns the computed result. |

`Require` does not abort a lane. The LLVM lowering normalizes a rejected
predicate to zero, records its condition, and emits one atomic fault
publication after all scalar instructions. The lane may therefore produce a
safe or numerically meaningless value for an out-of-domain input, but the
mandatory fault readback makes the run fail. Domain violations are not Rust
`Result` errors and are not silently converted into host exceptions.

The generated programs preserve signed zero only where the contract says so:
`preserve_zero` is used by `Sin`, `Tan`, `ExpMinusOne`, `LogOnePlus`, `Fmod`,
`Tanh`, and `Erf`; `emit_sign` returns the original zero directly. `Cos`,
`Exp`, and `ExpWithGradualUnderflow` intentionally return positive one for an
in-domain zero. `Pow` does the same when its exponent is zero; its base zero
is outside the positive-base domain. `Sigmoid` and `Softplus` use their stable formulas,
which produce positive `0.5` and positive `ln(2)` respectively. `Atan2`
handles a single signed zero through `copy_sign_from` and rejects only the
zero pair.

## Function dispatch and static cost

The following table describes the exact branch selected by `emit_function`.
`FLOPs/element` is the static work for the current instruction sequence,
computed with `ScalarOpcode::flops()`. It includes all scalar arithmetic,
including the integer `Add` or `Subtract` used by exponent reconstruction, and
all arithmetic comparisons. It counts each FMA as two, and excludes constants,
`Select`, bit operations, bitcasts, shifts, conversions, `IsFinite`, and
`Require`. It is not a new contract field and must be recomputed if the source
sequence changes.

| Function | Inputs | Guards after finite checks | Body and finalization | FLOPs/element |
| --- | ---: | --- | --- | ---: |
| `Reciprocal` | 1 | `require_nonzero(x)` | `1 / x` | 2 |
| `ReciprocalSquareRoot` | 1 | `require_positive(x)` | `1 / sqrt(x)` | 3 |
| `Sin` | 1 | `-8192 <= x <= 8192` | Paired sine/cosine core, then `preserve_zero` | 42 |
| `Cos` | 1 | `-8192 <= x <= 8192` | Paired sine/cosine core, return cosine | 41 |
| `Tan` | 1 | `-1.4 <= x <= 1.4` | Paired core, `sine / cosine`, then `preserve_zero` | 43 |
| `Atan2` | 2 (`y`, `x`) | pair not both zero | Octant reduction and `copy_sign_from` | 47 |
| `Exp` | 1 | `-80 <= x <= 80` | Normal power-of-two reconstruction | 24 |
| `ExpWithGradualUnderflow` | 1 | `x <= 0` | Subnormal-aware reconstruction and underflow select | 28 |
| `ExpMinusOne` | 1 | `-80 <= x <= 80` | Series for `abs(x) <= 0.25`, otherwise `exp(x)-1`, then `preserve_zero` | 43 |
| `Log` | 1 | `x > 0` | Subnormal normalization, bit decomposition, atanh series | 30 |
| `LogOnePlus` | 1 | `x > -1` | Series for `abs(x) <= 0.25`, otherwise `log(1+x)`, then `preserve_zero` | 57 |
| `Floor` | 1 | none beyond finite | `ScalarOpcode::Floor` | 1 |
| `Ceil` | 1 | none beyond finite | `ScalarOpcode::Ceiling` | 1 |
| `RoundNearestEven` | 1 | none beyond finite | `ScalarOpcode::RoundNearestEven` | 1 |
| `Trunc` | 1 | none beyond finite | `ceil(x)` for negative, `floor(x)` otherwise | 3 |
| `Fmod` | 2 (`dividend`, `divisor`) | `require_nonzero(divisor)` | IEEE `Remainder`, then preserve dividend zero | 3 |
| `Pow` | 2 (`base`, `exponent`) | `base > 0`; `exponent * log(base) <= 80` | `log(base)`, multiply exponent, gradual-underflow exp | 59 |
| `Sign` | 1 | none beyond finite | Ordered negative, positive, or original zero select | 2 |
| `Sigmoid` | 1 | `-80 <= x <= 80` | Sign-stable positive and negative exp quotients | 52 |
| `Tanh` | 1 | `-40 <= x <= 40` | `expm1(2*abs(x))/(expm1(2*abs(x))+2)`, copy sign, preserve zero | 48 |
| `Softplus` | 1 | `-80 <= x <= 80` | `max(x,0) + log1p(exp(-abs(x)))` | 83 |
| `Erf` | 1 | `-6 <= x <= 6` | Abramowitz-Stegun polynomial times `exp(-x²)`, copy sign, preserve zero | 43 |

The reusable weighted costs behind the table are:

```text
horner(first, n coefficients) = 2*n
emit_sine_cosine_core         = 39
emit_atan_polynomial          = 16
emit_exp_reconstruction(false)= 22
emit_exp_reconstruction(true) = 25
emit_exp_with_underflow       = 1(LT) + 1(Maximum) + 25 = 27
emit_expm1_core               = 40
emit_log_core                 = 29
emit_log1p_core               = 55
```

The `FLOPs/element` values include branch guards and final zero or sign
repairs where those repairs contain priced arithmetic or comparisons. A
`Require` itself is zero-priced, but the comparison that feeds it is priced.
This distinction is important for validation code that is visible in the
program but not counted as arithmetic work.

## Trigonometric construction

### Sine and cosine core

`emit_sine_cosine_core` computes both values once so `Tan` can reuse them:

1. `scaled = x * TWO_OVER_PI` and `nearest = RoundNearestEven(scaled)`.
2. `reduced_high = Fma(nearest, -PI_OVER_TWO_HIGH, x)` and
   `reduced = Fma(nearest, -PI_OVER_TWO_LOW, reduced_high)`. The split high and
   low constants reduce cancellation in the residual angle.
3. `squared = reduced * reduced`.
4. `horner(squared, 1.589691e-10, [-2.505076e-8, 2.7557314e-6,
   -1.984127e-4, 8.333334e-3, -1.6666667e-1])` produces the sine polynomial;
   `reduced_cubed * polynomial + reduced` is the odd sine approximation.
5. A second Horner chain with first coefficient `-1.135965e-11` and remaining
   coefficients `[2.0875723e-9, -2.7557314e-7, 2.4801588e-5,
   -1.3888889e-3, 4.1666668e-2]` is emitted from the source list in
   `program.rs:236-242`; it is combined with `-0.5` and `1.0` through two FMA
   operations for the even cosine approximation. The source literals are
   authoritative, including their exact binary32 rounding.
6. `nearest` is converted to I32 and masked with `3`. Equality tests for
   quadrants zero and two and an ordering test for the lower half select signs
   and swap sine or cosine across quadrants. Six `Select` instructions return
   the paired result.

The core intentionally emits both values even when the caller asks for only
one. This is why `Cos` and `Sin` have nearly identical static costs and why
`Tan` is a ratio over the same reduction rather than an independent tangent
approximation. The closed interval guards keep the Cody-Waite range reduction
within its contracted precision.

### `atan2`

`emit_atan_polynomial` evaluates an odd polynomial as
`x + x^3 * P(x²)`. `P` starts at `-1/15` and has six remaining FMA
coefficients `[1/13, -1/11, 1/9, -1/7, 1/5, -1/3]`, for a 16-FLOP helper.

`emit_atan2_core` first takes absolute values, swaps the axes when
`abs(y) > abs(x)`, and divides the smaller magnitude by the larger. Ratios
above `0.41421357` use `(ratio - 1)/(ratio + 1)` and add `pi/4` after the
polynomial; smaller ratios use the direct polynomial. A swap maps the result
with `pi/2 - angle`. `x < 0` reflects with `pi - angle`. Finally
`copy_sign_from` copies the sign bit of `y`. The preceding pair guard ensures
the divisor selected as `maximum` is nonzero for all in-domain lanes.

## Exponential family

### Normal reconstruction

`emit_exp_reconstruction(builder, x, false)` performs range reduction in base
two:

1. `nearest = RoundNearestEven(x * LOG_TWO_RECIPROCAL)`.
2. Subtract `nearest * LOG_TWO_HIGH` and `nearest * LOG_TWO_LOW` with two FMA
   instructions to obtain a small residual.
3. Evaluate the source's degree-seven residual Taylor chain with `horner` and
   multiply it by a bit-constructed normal `2^nearest`.
4. The exponent is converted to I32, biased by 127, shifted left 23 bits, and
   bitcast to F32. The normal path is valid for the `Exp` range `[-80,80]`.

`emit_exp_core` is exactly this normal path. `Exp`, `ExpMinusOne`, `Sigmoid`,
`Softplus`, and `Erf` call it only after their own range contracts keep the
reduction in range.

### Gradual underflow

`emit_exp_with_underflow` is the private implementation path behind the public
`exp_with_gradual_underflow_program` wrapper and is also used by `Pow`:

* `underflows = x < EXP_HALF_MIN_SUBNORMAL_LOG`, where the constant is the
  binary32 `ln(2^-150)` half-minimum-subnormal boundary. The selector is
  intentionally strict; the equality case is left to reconstruction, whose
  round-to-nearest-even result is positive zero at the half-subnormal tie.
* `bounded = Maximum(x, cutoff)` is constructed before range reduction. This
  prevents a finite input multiplied by `LOG2_E` from becoming negative
  infinity before `ConvertF32ToI32`.
* `emit_exp_reconstruction(..., true)` splits exponents below `-126`: it adds
  `64` before constructing a normal power, multiplies the normal result by the
  exact `2^-64` bit pattern, and otherwise uses scale `1.0`.
* A final `Select(underflows, 0.0, reconstructed)` chooses positive zero for
  values below the cutoff and the reconstructed value otherwise.

The branch-specific condition `x <= 0` is emitted by `emit_function` before
this helper. The universal finite check is still first, so positive finite,
NaN, and infinity inputs all report a fault. The lane continues through the
same instruction graph after a rejection, but only finite nonpositive values
are contracted.

`Pow` intentionally checks only the upper bound of its reconstructed
`power = exponent * log(base)`. A positive overflow to `+infinity` fails
`power <= 80`; a sufficiently negative finite product may become `-infinity`,
which the gradual-underflow helper clamps and returns as positive zero. This is
consistent with the declared upper-only relation and avoids an unnecessary
lower-range rejection for a result that is already underflowing.

### `expm1`

`emit_expm1_core` tests `abs(x) <= 0.25`. In the local branch it evaluates
`x * P(x)` with the source's degree-eight Horner coefficients, avoiding the
subtraction of nearly equal values. The other branch computes `emit_exp_core`
and subtracts one. `ExpMinusOne` then calls `preserve_zero`.

## Logarithmic family

### `log`

`emit_log_core` accepts positive normals and subnormals:

1. Compare against `f32::MIN_POSITIVE`. Subnormals are multiplied by
   `8_388_608.0` (`2^23`) and receive an exponent adjustment of `-23`.
2. Bitcast the normalized F32 to I32. Shift right 23 and mask `0xff` to get
   the biased exponent, subtract 127, then add the subnormal adjustment.
3. Mask the mantissa, OR in `0x3f800000` to form a `[1,2)` F32 mantissa.
4. If the mantissa exceeds `sqrt(2)`, halve it and increment the exponent. This
   keeps `y = (mantissa-1)/(mantissa+1)` small.
5. Evaluate the atanh series through `y^13` with a Horner chain in `y²`, form
   `2*y*series`, convert the exponent to F32, and add `exponent*ln(2)` in two
   FMA pieces using `LOG_TWO_HIGH` and `LOG_TWO_LOW`.

The positive guard is outside the helper. `Log` therefore rejects both zero
signs before this bit decomposition, while positive subnormal values remain
valid.

### `log1p`

`emit_log1p_core` uses a degree-12 local series for `abs(x) <= 0.25`. Outside
that region it adds one to `x` and calls `emit_log_core`. The branch is selected
with `Select`, not a host branch. `LogOnePlus` emits `x > -1` before the helper
and `preserve_zero` after it.

## Exact and stable elementary functions

* `Floor`, `Ceil`, and `RoundNearestEven` are one direct F32 scalar opcode each.
  Their only explicit domain check is the universal finite predicate.
* `emit_trunc` computes both floor and ceiling, tests `x < 0`, and selects the
  ceiling for negative values. This preserves the signed zero returned by the
  selected primitive.
* `emit_sign` computes `x < 0` and `x > 0`. A positive input selects `1.0`; a
  nonpositive input initially selects the original `x`; a negative input then
  selects `-1.0`. Both zeros therefore retain their input sign.
* `emit_sigmoid_core` computes both stable forms: `1/(1+exp(-x))` for
  nonnegative `x`, and `exp(x)/(1+exp(x))` for negative `x`. The `-80 <= x <=
  80` guard keeps both `emit_exp_core` calls in their normal range.
* `emit_tanh_core` evaluates the nonnegative magnitude through
  `expm1(2*abs(x))/(expm1(2*abs(x))+2)`, copies `x`'s sign, and then the outer
  `Tanh` branch restores signed zero. Its `-40 <= x <= 40` domain makes the
  doubled argument valid for `expm1`.
* `emit_softplus_core` computes `abs(x)`, `exp(-abs(x))`, and
  `log1p(exp(-abs(x)))`; it adds that correction to `Select(x > 0, x, 0)`. The
  outer range guard bounds the exponential argument and makes the result
  finite and stable for both signs.
* `emit_erf_core` evaluates the Abramowitz-Stegun 7.1.26 approximation. It
  forms `t = 1/(1 + 0.3275911*abs(x))`, evaluates the fixed degree-four
  Horner polynomial, multiplies that polynomial by `t` to form the degree-five
  factor, multiplies by `exp(-abs(x)^2)`, subtracts the tail from one, copies
  the sign, and finally preserves an input signed zero.
* `copy_sign_from` is representation-level sign transfer. It bitcasts the
  source F32, shifts right 31, negates the magnitude once, and selects the
  negated value for a nonzero sign bit. It does not call `Absolute`, so the
  magnitude's existing sign contract remains the caller's responsibility.

## Callers and lowering flow

The complete in-repository call graph is:

```text
MathFunction::try_from
  -> recipe_math::program::build
     -> ScalarProgramBuilder
     -> ScalarProgram::validate

ops::ScalarRecipe::Math(function)
  -> ops::lower_scalar
     -> MathFunction::try_from
     -> second ScalarProgram::validate

ops::Composer::inline_math(function, arguments)
  -> MathFunction::try_from
  -> input arity and dtype checks
  -> constants copied into the composer namespace
  -> instructions replayed through Composer::apply
  -> first math output returned inside one composite program

PrimitiveKind::Elementwise(Elementwise { program })
  -> recipe-primitives lower_elementwise
  -> KernelTemplate validation and scalar fault binding
  -> recipe-kernel lower_elementwise
  -> LLVM IR, HSACO, or cubin
```

Concrete callers are intentionally narrow:

| Caller | Functions used | How the program is consumed |
| --- | --- | --- |
| [`ops/src/scalar.rs`](../../../ops/src/scalar.rs), `ScalarRecipe::for_symbol` | `Atan2`, `Ceil`, `Cos`, `Exp`, `ExpMinusOne`, `Floor`, `Fmod`, `Log`, `LogOnePlus`, `Pow`, `Reciprocal`, `RoundNearestEven`, `ReciprocalSquareRoot`, `Sigmoid`, `Sign`, `Sin`, `Softplus`, `Tan`, `Tanh`, `Trunc` | Public source-qualified operation symbols become `ScalarRecipe::Math`; `lower_scalar` obtains the complete program. `Erf` and `ExpWithGradualUnderflow` are not registry symbols. |
| `ops/src/scalar.rs`, `Composer::inline_math` | `Softplus`, `Sigmoid`, `Exp`, `Sign`, `Tanh` | Composite losses, activations, and optimizer formulas inline the instruction list and keep one owner/value namespace. |
| `ops/src/materialize/attention_sequence_embedding.rs:292-372` | `Sin`, `Cos`, `Exp` | Positional encoding and softmax stages embed each program in an elementwise primitive. |
| `ops/src/materialize/graph_cluster_rl.rs:336-488` | `Exp`, `Log` | Gaussian and categorical probability materialization embeds elementwise stages. |
| `ops/src/materialize/loss_metrics.rs:308` | `Log` | KL-divergence materialization first makes targets positive, then runs Recipe log. |
| `training/src/inference.rs:4555-4600` | `ExpWithGradualUnderflow` | Stable sigmoid and softmax inference use the named public helper for subnormal-preserving exponentials. |
| Root facade advanced surface | all `MathFunction` values | `recipe::engine::math` reexports the crate; callers receive only `ScalarProgram` or `LanguageError`. |

`ops::lower_scalar` rejects descriptors whose lowering is primitive,
composition, workspace, non-calculation, or unsupported. A Math recipe is not
wrapped in another evaluator. `Composer::inline_math` copies constants and
replays each instruction, so domain `Require` operations remain present in the
composite program and cannot be bypassed. In inference, `stable_sigmoid` first
builds `-abs(logit)` and later selects the sign-dependent quotient. The
`stable_softmax` path max-shifts logits, requires a finite nonpositive shifted
argument through its surrounding graph, then uses the same gradual-underflow
program before reducing exponentials. Those graph-level preconditions are
separate from the math program's own finite and `x <= 0` guards.

## Validation and failure propagation

Construction and execution failures are distinct. The builder can reject a
program before any graph exists; a valid program can later report a runtime
domain fault; and a native stage can fail closed if an ABI or opcode contract
does not match.

| Boundary | Condition | Result |
| --- | --- | --- |
| `ScalarProgramBuilder::new` | Owner identity counter cannot advance | `LanguageErrorKind::InvalidScalarProgram`, `scalar builder identity space exhausted`. |
| `input`, `constant`, any `apply` | Scalar value ID counter overflows | `LanguageErrorKind::InvalidScalarProgram`, `scalar value identity space exhausted`. |
| `apply` | Operand expression belongs to another builder | `LanguageErrorKind::InvalidScalarProgram`, foreign scalar value detail. |
| `apply` | Arity or dtype signature is not accepted by `ScalarOpcode::result_dtype` | `LanguageErrorKind::InvalidScalarProgram` with opcode and operand dtypes. |
| `finish` | Output expression belongs to another builder | `LanguageErrorKind::InvalidScalarProgram`, foreign output detail. |
| `finish` or direct core validation | Duplicate ID, use before definition, wrong instruction type, wrong arity, missing output, or unknown output | `ScalarProgram::validate` returns one or more path-aware `ValidationCode` values, mapped to `LanguageErrorKind::InvalidScalarProgram`. |
| `ScalarProgram::try_from` | Any builder or validation failure | Propagates `LanguageError` unchanged through `MathFunction::try_from`. |
| `ops::lower_scalar` | Descriptor is not scalar, is unsupported, math construction fails, or final validation fails | `WrongLoweringKind`, `UnsupportedLowering`, or `InvalidScalarProgram`, attached to the operation ID. |
| `Composer::inline_math` | Arity or argument dtype differs, replay references an unknown value, an instruction result dtype changes, or the math program has no/unknown first output | `OperationErrorKind::InvalidScalarProgram`. |
| Primitive elementwise validation | Scalar program invalid, tensor input/output arity or dtype differs, broadcast shape differs, or no tensor input exists | `LanguageErrorKind::InvalidScalarProgram`, `ArityMismatch`, `DTypeMismatch`, or `ShapeMismatch`; constant-only elementwise maps are rejected. |
| Primitive work accounting | Per-instruction sum or element multiplication overflows `u64` | `LanguageErrorKind::WorkOverflow`. |
| Primitive lowering | Language graph is invalid, a tensor is missing, static access is invalid, the index-space/binding/fault/resource/scalar-slot/FLOP arithmetic overflows, or the lowered program contract is invalid | `InvalidLanguage`, `MissingTensor`, `InvalidStaticAccess`, `ArithmeticOverflow`, or `InvalidLoweredProgram`; checked programs receive `FaultReason::ArithmeticDomain` code `2`. |
| Kernel LLVM lowering | Template or target validation fails, the entry symbol or workgroup is invalid, an operand ID is absent, an opcode/type pair is not implemented, work or ABI arithmetic overflows, the stage publication contract is invalid, or the closed-module audit finds a prohibited declaration | `InvalidKernel`, `InvalidTarget`, `InvalidEntrySymbol`, `InvalidWorkgroupSize`, `UnknownScalarValue`, `UnsupportedOperation`, `ArithmeticOverflow`, `InvalidStageContract`, or `ProhibitedInterface`. |
| Runtime lane | Input is nonfinite or violates a function-specific `Require` | The lane publishes the preallocated I32 fault flag. The fault readback reports a device fault and the run fails; this is not a construction `Result`. |

`ScalarProgram::validate` itself accumulates errors. It inserts all inputs and
constants into one type map, checks every instruction's already-defined
operands and signature, then checks result uniqueness and outputs. The
language builder normally prevents these states, but the core validation is
repeated at kernel-template, primitive-stage, OGDL, and operation boundaries so
serialized or manually assembled values cannot bypass the contract.

The validation passes are intentionally layered rather than substituted:

1. `ScalarProgramBuilder::apply` resolves `ScalarOpcode::result_dtype` before
   appending each instruction and rejects a foreign `ScalarExpression` owner.
2. `ScalarProgramBuilder::finish` checks output ownership and calls
   `ScalarProgram::validate` on the assembled vectors.
3. `ops::lower_scalar` validates the completed program again after selecting a
   `ScalarRecipe`, including the `MathFunction` path and any composite replay.
4. `PrimitiveKernel::validate` calls program validation, then requires tensor
   input and output arity, dtypes, broadcast shape, and alias rules to agree
   with the scalar vectors. A constant-only elementwise primitive is rejected
   at this layer even though the bare scalar type can represent it.
5. `recipe-primitives` converts the elementwise primitive into a
   `KernelTemplate`, revalidates scalar and memory contracts, and derives the
   optional arithmetic-domain fault binding and resource bounds.
6. `recipe-kernel::lower_elementwise` validates the template again before
   looking up values and emitting LLVM. A hand-authored or decoded program
   therefore cannot reach native lowering through an unvalidated shortcut.

Core validation uses one definition map in vector order. Inputs and constants
must have unique IDs; every instruction operand must already be defined;
`validate_signature` requires exact arity and the result dtype returned by
`ScalarOpcode::result_dtype`; each result ID must be new; and every output ID
must be known. The validator reports `ScalarUseBeforeDefinition` before trying
to infer a type for an unknown operand, which keeps the error path meaningful.
The math builder's source order already satisfies these rules, but the repeated
passes protect OGDL decoding, operation composition, primitive materialization,
and native artifact reuse from stale or manually assembled values.

When a containing calculation graph is serialized, the OGDL scalar node keeps
the ordered `inputs`, `constants`, `instructions`, and `outputs` vectors. F32
constants are encoded as their bit-preserving literal values, not by a host
float round trip. The strict decoder rejects missing or unknown fields and
unknown opcode spellings, reconstructs the `ScalarProgram`, and invokes the
same validation before returning a graph. Primitive program digests likewise
include the scalar IDs, literal bits, opcodes, result dtypes, and operand order;
two mathematically equivalent but differently ordered math programs cannot
silently share a lowered artifact.

## Native lowering and evaluation

`recipe-kernel::lower_elementwise` is the first evaluator in the system. It
does not interpret the math function name. It consumes the validated ordered
`ScalarInstruction` list:

1. Compute one linear lane index and load each tensor input according to its
   finalized affine view. Inputs are paired with `program.inputs` by vector
   order.
2. Materialize constants as I32 immediates or exact F32 bitcasts.
3. Visit instructions in order. Each operand must already be in the emitter's
   value map; the result is inserted only after its opcode lowering succeeds.
4. Combine all `Require` and checked-I32 rejection conditions into one fault
   branch. A rejected lane atomically ORs one into the preallocated global
   fault flag and then rejoins the normal path.
5. Look up each scalar output, verify its dtype against the kernel output, and
   store through the output view.
6. Audit the closed LLVM module, build the buffer/fault/element-count ABI, and
   report the checked total FLOP count.

The LLVM matcher emits constrained round-to-nearest F32 arithmetic, direct
`fdiv` for F32 divide, constrained one-rounding FMA, representation-preserving
bit operations, target-independent F32 intrinsics for square root and rounding,
and no vendor math call. F32 arithmetic results that can be NaN are
canonicalized by the emitter; bit-level sign operations retain payload bits.
The native compiler later produces the target image, and CUDA or HSA submits
that image. No later layer reconstructs a `sin`, `exp`, or `log` call from the
`MathFunction` tag.

Deferred scalar-stage realization validates the generic publication and
rewrites its `atomicrmw or` of one into the stage contract's
`atomicrmw xchg` with fault code `2` and release ordering. If the generic
publication is absent, stage realization returns `InvalidStageContract`; it
does not accept a different fault channel or add a fallback evaluator.

For a scalar map, primitive lowering adds a fault binding exactly when
`program.requires_fault_flag()` is true. Core sets that result for
`Require(I32)` and for checked I32 divide, remainder, negate, or absolute.
Every generated math program contains `Require`, so every MathFunction stage
has the arithmetic-domain fault binding. Primitive lowering derives per-lane
FLOPs from the same instruction list, multiplies by logical output elements,
reserves four bytes per scalar input, constant, and instruction slot, and
bounds one atomic fault publication per lane when checked. Stage validation
recomputes these values and rejects a mismatched caller-supplied resource
contract.

After lowering, the planner and scheduler see only the immutable elementwise
stage and its `FlopCount`. The scheduler converts that work with the measured
device calculation rate and checked ceiling arithmetic for a calculation
window; it does not inspect `MathFunction`, count a polynomial again, or infer
a vendor latency. Transfers, addressing, constants, bit manipulation, and
fault publication remain resource bounds rather than additional model work.

## Cost and identity invariants

The following invariants are part of the artifact and scheduling boundary:

* `MathFunction::ALL` and the `emit_function` match must remain exhaustive over
  the same 22 variants. A new function requires a new contract and branch.
* Every math input and output is F32. Predicate intermediates are I32 and must
  be converted or selected explicitly; there is no implicit bool or F16 path.
* Every input gets `IsFinite` plus `Require` before function-specific work.
  Additional domain checks remain in the ordered program and are not metadata.
* `ScalarProgram` has exactly one output for a MathFunction. Composite callers
  may expose additional outputs only in their own Composer programs.
* Instruction order, constant bit patterns, opcode spellings, operand order,
  and result IDs are identity material. Reordering a mathematically equivalent
  sequence changes the canonical program and native artifact identity.
* `ScalarOpcode::flops()` is the sole scalar work price. Constants, checks,
  bit operations, conversions, selects, addressing, and fault publication are
  excluded exactly as core defines them; FMA is always two.
* Domain violations are fail-closed runtime events. They must not gain a host
  fallback, a vendor-library branch, a retry, or an alternate implementation.
* The public `exp_with_gradual_underflow_program` wrapper remains equivalent to
  `ScalarProgram::try_from(MathFunction::ExpWithGradualUnderflow)`. It is a
  naming boundary for inference, not a second implementation.

When changing a coefficient, range limit, special-value rule, or helper
sequence, update the corresponding `MathContract::algorithm` identity and
version in `contract.rs`, then recheck the static cost and all native callers.

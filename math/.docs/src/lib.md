# `recipe-math` facade

Source: [`math/src/lib.rs`](../../src/lib.rs)

```toml
[module]
path = "math/src/lib.rs"
kind = "crate-facade-for-deterministic-backend-neutral-scalar-math"
intent = "Define Recipe-owned f32 scalar functions once, with explicit finite domains, special-value rules, error claims, and versioned algorithm identities."
purpose = "Construct validated recipe_core::ScalarProgram values from a closed MathFunction enum and expose the contracts that govern those programs."
structure = "Two private implementation modules behind one explicit root re-export surface."
private_modules = ["contract", "program"]
crate_attributes = ["forbid(unsafe_code)", "deny(missing_debug_implementations)"]
state = "Per-call ScalarProgramBuilder construction; no math-owned mutable runtime state or cached program instances."
dependencies = ["recipe-core", "recipe-language"]
public_module_tree = false

[boundary]
inputs = ["MathFunction", "recipe_core::ScalarOpcode and DType through the language builder"]
outputs = ["validated recipe_core::ScalarProgram", "MathContract and its public contract records", "LanguageError on construction failure"]
owned_domain = "Recipe's deterministic scalar coefficients, range reductions, finite-domain admission, signed-zero behavior, and error metadata."
not_owned = ["tensor shapes and broadcasting", "operation symbol lookup", "graph and primitive wiring", "hardware probing", "placement and scheduling", "LLVM or native image lowering", "device allocation and execution"]
fault_rule = "Finite checks and domain checks are scalar Require instructions in the emitted program; a rejected lane sets the preallocated device fault flag."
```

## Intent and crate boundary

`recipe-math` is the single owner of Recipe's reusable scalar transcendental
and elementary math programs. It does not evaluate a host `f32`, call a CPU
math library, or provide a second implementation for a backend. Instead, it
describes one per-element calculation as a typed, acyclic
[`recipe_core::ScalarProgram`](../../../core/src/scalar.rs) made only from the
backend-neutral `DType`, `ScalarLiteral`, and `ScalarOpcode` vocabulary.

The crate root states the hard source boundary with
`#![forbid(unsafe_code)]` and `#![deny(missing_debug_implementations)]`. Its
only dependencies are `recipe-core`, which owns the scalar instruction and
program records, and `recipe-language`, which owns the checked
`ScalarProgramBuilder` and `LanguageError`. `recipe-math` owns the meaning of
each function, its coefficients and reduction strategy, the conditions under
which it may run, and the contract metadata that downstream compilation uses.

The output is still only a scalar calculation payload. A caller must put that
program in an elementwise primitive, bind tensor values and views, and pass it
through planning, target lowering, preparation, and execution. Math never
chooses an operation symbol, tensor layout, device, queue, schedule, native
ABI, or lifecycle phase. There is no public module path for either implementation
module and no global program cache. Every request constructs a fresh validated
program, so the instruction order and constants are visible to the caller.

## What `lib.rs` exposes

`lib.rs` declares the private modules in source order:

```text
contract, program
```

It then explicitly re-exports the contract module's public records at the
crate root:

```rust
pub use contract::{
    AlgorithmIdentity, ErrorBound, FiniteBound, FiniteDomain, FiniteInputDomain,
    MathContract, MathFunction, NonFiniteBehavior, SpecialValueBehavior,
};
```

`recipe_math::contract::...` and `recipe_math::program::...` are not public
paths. Each exported type remains owned by `contract.rs`; the root list is a
flat import facade, not a duplicate type or implementation.

The root has two construction entry points:

| Entry point | Result and role |
| --- | --- |
| `ScalarProgram::try_from(function)` | The blanket-visible `TryFrom<MathFunction>` implementation returns `Result<recipe_core::ScalarProgram, recipe_language::LanguageError>`. It is the general path for all 22 functions. |
| `exp_with_gradual_underflow_program()` | A named convenience wrapper returning the same result for `MathFunction::ExpWithGradualUnderflow`. It exists for stable sigmoid and softmax paths that need subnormal-preserving exponentials. |

`ScalarProgram` and `LanguageError` are imported privately in this crate. They
are not re-exported by `recipe_math`; callers name them through
`recipe_core` and `recipe_language` (or through a higher-level facade). The
named exponential helper is not an alternate algorithm: it delegates directly
to the same `TryFrom` implementation.

The root source regions are deliberately small and have one owner each:

| `math/src/lib.rs` region | Owner and responsibility |
| --- | --- |
| Lines 1-2 | Crate-wide unsafe and `Debug` guarantees. |
| Lines 4-10 | The crate contract: backend-neutral scalar instructions, versioned identities, finite domains, special values, error claims, and in-band `Require` checks. |
| Lines 12-16 | Private imports of `ScalarProgram` and `LanguageError`, followed by the two private modules. |
| Lines 18-21 | The complete explicit root re-export list from `contract.rs`. |
| Lines 23-37 | The named gradual-underflow constructor and its documented error and fault behavior. |
| Lines 39-43 | The one `TryFrom<MathFunction>` implementation, delegating all construction to `program::build`. |

`contract.rs` owns all public metadata types, the 22-variant enum, its static
domain arrays, and the `ALL`, `arity`, `contract`, `algorithm`, domain,
signed-zero, and error-bound queries. `program.rs` owns only the private
builder and emission helpers. This split means the root facade contains no
duplicated coefficients or domain logic.

## Public contract records

The public records in `contract.rs` are small, copyable descriptions of the
promise made by each generated program. They do not execute a function and do
not inspect hardware.

| Item | Public shape and meaning |
| --- | --- |
| `AlgorithmIdentity` | `{ name: &'static str, version: u32 }`. The name identifies one coefficient and reduction family; the version changes when that implementation identity changes. |
| `FiniteBound` | `Unbounded`, `Inclusive(f32)`, or `Exclusive(f32)`, used for one endpoint of an input interval. |
| `FiniteInputDomain` | `{ name, lower, upper, nonzero }` for one named input. The interval describes finite values; `nonzero` records a separate zero exclusion. |
| `FiniteDomain` | `{ inputs: &'static [FiniteInputDomain], relation: &'static str }`. `inputs` gives per-argument bounds and `relation` states cross-input conditions not expressible as independent intervals. |
| `NonFiniteBehavior` | Currently the one variant `RejectWithRequire`. A NaN or either infinity is rejected in the generated program by `IsFinite` followed by `Require`. |
| `SpecialValueBehavior` | `{ non_finite, signed_zero, domain_violation }`, documenting behavior outside the ordinary finite interval. `domain_violation` is the fixed statement that the preallocated fault flag is set through `Require`. |
| `ErrorBound` | `{ maximum_absolute, maximum_relative, note }`. A result conforms when either the absolute or relative bound holds; both zero means an exact composition of shipped scalar primitives. |
| `MathContract` | `{ domain, special_values, error, algorithm }`, the complete description returned by `MathFunction::contract()`. |
| `MathFunction` | The closed shipped function selector. It is `Copy`, ordered, hashable, and has the canonical `ALL` array. |

`MathFunction` exposes four `const` queries:

* `ALL` lists all variants in the canonical order shown below.
* `arity()` returns two only for `Atan2`, `Fmod`, and `Pow`; every other
  variant is unary.
* `contract()` combines the finite domain, special-value strings, error bound,
  and algorithm identity into one `MathContract`.
* `algorithm()` returns a stable name and version without constructing a
  program.

The contract records use static slices and strings. This makes metadata
available during compile-time registry construction and prevents a caller from
silently changing a function's domain or algorithm identity at runtime.

## Complete `MathFunction` contract

The following table is the current source-of-truth result of
`MathFunction::{arity, contract, algorithm}`. All rows first reject non-finite
inputs. The finite relation is then enforced by one or more `Require`
instructions before the mathematical output is selected.

| Function | Arity and finite relation | Signed-zero rule | Error bound (absolute / relative) | Algorithm identity |
| --- | --- | --- | --- | --- |
| `Reciprocal` | 1; finite `x != 0` | Both signed zeros are rejected | `0 / 0`, exact primitive composition | `recipe.math.reciprocal.ieee-divide-f32` v1 |
| `ReciprocalSquareRoot` | 1; finite `x > 0` | Both signed zeros are rejected | `0 / 0`, exact primitive composition | `recipe.math.rsqrt.sqrt-divide-f32` v1 |
| `Sin` | 1; `-8192 <= x <= 8192` | Input signed zero is preserved exactly | `2e-4 / 2e-4` | `recipe.math.sin.cody-waite-s11-f32` v1 |
| `Cos` | 1; `-8192 <= x <= 8192` | Either signed zero maps to positive one | `2e-4 / 2e-4` | `recipe.math.cos.cody-waite-c12-f32` v1 |
| `Tan` | 1; `-1.4 <= x <= 1.4` | Input signed zero is preserved exactly | `5e-4 / 5e-4` | `recipe.math.tan.cody-waite-ratio-f32` v1 |
| `Atan2` | 2; finite `y` and `x`, not both zero | Signed `y` zero selects the signed `x`-axis result; `(0, 0)` is rejected | `3e-5 / 3e-5` | `recipe.math.atan2.octant-atan15-f32` v1 |
| `Exp` | 1; `-80 <= x <= 80` | Either signed zero maps to positive one | `2e-6 / 5e-6` | `recipe.math.exp.bitpow2-taylor7-f32` v1 |
| `ExpWithGradualUnderflow` | 1; every finite `x <= 0` | Either signed zero maps to positive one | `2e-6 / 5e-6` | `recipe.math.exp.split-scale-subnormal-f32` v1 |
| `ExpMinusOne` | 1; `-80 <= x <= 80` | Input signed zero is preserved exactly | `3e-6 / 7e-6` | `recipe.math.expm1.hybrid-taylor8-f32` v1 |
| `Log` | 1; finite `x > 0`, including positive subnormals | Both signed zeros are rejected | `6e-6 / 6e-6` | `recipe.math.log.bitdecompose-atanh13-f32` v1 |
| `LogOnePlus` | 1; finite `x > -1` | Input signed zero is preserved exactly | `8e-6 / 8e-6` | `recipe.math.log1p.hybrid-series12-f32` v1 |
| `Floor` | 1; every finite binary32 value | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.floor.scalar-primitive-f32` v1 |
| `Ceil` | 1; every finite binary32 value | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.ceil.scalar-primitive-f32` v1 |
| `RoundNearestEven` | 1; every finite binary32 value | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.rne.scalar-primitive-f32` v1 |
| `Trunc` | 1; every finite binary32 value | Input signed zero is preserved exactly | `0 / 0`, exact primitive composition | `recipe.math.trunc.floor-ceil-select-f32` v1 |
| `Fmod` | 2; finite dividend and finite nonzero divisor | A zero dividend preserves its sign | `0 / 0`, exact IEEE remainder composition | `recipe.math.fmod.ieee-remainder-f32` v1 |
| `Pow` | 2; finite `base > 0` and `exponent * log(base) <= 80` | Either signed zero maps to positive one when in-domain | `3e-5 / 2e-4` | `recipe.math.pow.log-exp-subnormal-v3-f32` v3 |
| `Sign` | 1; every finite binary32 value | Input signed zero is preserved exactly | `0 / 0`, exact select/comparison composition | `recipe.math.sign.ordered-select-f32` v1 |
| `Sigmoid` | 1; `-80 <= x <= 80` | Either signed zero maps to positive `0.5` | `4e-6 / 6e-6` | `recipe.math.sigmoid.stable-exp-f32` v1 |
| `Tanh` | 1; `-40 <= x <= 40` | Input signed zero is preserved exactly | `8e-6 / 1e-5` | `recipe.math.tanh.stable-expm1-f32` v1 |
| `Softplus` | 1; `-80 <= x <= 80` | Either signed zero maps to positive `ln(2)` | `1e-5 / 1e-5` | `recipe.math.softplus.stable-log1p-f32` v1 |
| `Erf` | 1; `-6 <= x <= 6` | Input signed zero is preserved exactly | `2e-6 / 2e-6` | `recipe.math.erf.as-7.1.26-f32` v1 |

The `FiniteInputDomain` arrays behind the table preserve argument names. The
binary names are `(y, x)` for `Atan2`, `(dividend, divisor)` for `Fmod`, and
`(base, exponent)` for `Pow`; all other rows use `x`. `Pow`'s upper relation is
not an independent bound on the exponent. It is checked after the positive-base
logarithm is built, so the accepted set is exactly the relation returned by
`contract()`. `ExpWithGradualUnderflow` is intentionally different from
`Exp`: it has no finite lower bound and rejects positive finite values rather
than producing a positive overflow result.

The signed-zero strings are part of the contract, not explanatory decoration.
`program.rs` implements them with `preserve_zero`, sign-copy logic, explicit
comparisons, or the natural result of the selected primitive. Consumers that
serialize or compare algorithm contracts must retain these distinctions.

## Construction flow and fault semantics

`program::build` is the only implementation entry point, and it is
crate-private. `TryFrom<MathFunction>` delegates directly to it; the named
gradual-underflow helper delegates back through `TryFrom`. The construction
sequence is fixed:

```text
MathFunction
  -> ScalarProgramBuilder::new()
  -> arity() F32 inputs in argument order
  -> IsFinite + Require for every input
  -> function-specific domain Require instructions
  -> function-specific scalar opcode graph
  -> builder.finish([output])
  -> ScalarProgram::validate()
  -> Result<ScalarProgram, LanguageError>
```

Every input is `DType::F32`, including the two inputs of binary functions. The
builder allocates one ordered SSA namespace for inputs, constants, and
instructions. `finish` checks builder ownership, assembles one output, and
validates arity, use-before-definition, result dtypes, duplicate IDs, and the
existence of the output. A builder failure, invalid opcode signature, or
identity-space exhaustion is returned as `LanguageError`; there is no host
fallback or alternate implementation.

`IsFinite` produces an int32 predicate and `Require` consumes it. A false
predicate does not branch to host code and does not throw a Rust exception. It
records a fault in the preallocated device fault channel used by the eventual
kernel. The same mechanism handles zero divisors, nonpositive arguments, the
trigonometric intervals, the `Atan2` zero pair, and the `Pow` reconstructed
range. This is why the metadata says `RejectWithRequire` for every function's
non-finite behavior.

The generated program is an ordinary `ScalarProgram` after construction. It
does not carry a separate contract object, runtime callback, or function tag.
Callers that need the metadata call `MathFunction::contract()` beside the
program construction and retain the same enum value as the identity source.

## Program implementation map

The private `program` module contains no public math API. Its helpers all append
`ScalarOpcode` instructions to the caller-owned `ScalarProgramBuilder` and
return typed `ScalarExpression` values.

| Helper family | Implemented functions and method |
| --- | --- |
| Admission checks | `require_finite_inputs`, `require_nonzero`, `require_positive`, `require_closed_interval`, and `require_atan2_nonzero_pair` emit the explicit domain predicates. |
| Signed-zero and sign transfer | `preserve_zero` selects the original input when it compares equal to zero; `copy_sign_from` bitcasts the sign source, extracts its sign bit, and selects a negated or positive magnitude. |
| Polynomial evaluation | `horner` emits one chain of FMA instructions from a first coefficient and an ordered coefficient slice. The call order is retained in the final SSA program. |
| Sine and cosine | `emit_sine_cosine_core` performs Cody-Waite reduction with high and low `pi/2` pieces, evaluates odd/even polynomials, and selects signs/quadrants from an int32 reduction index. `Sin`, `Cos`, and the numerator of `Tan` share this core. |
| `Atan2` | `emit_atan2_core` compares absolute coordinates, reduces to a ratio, switches to `(r-1)/(r+1)` above `0.41421357`, evaluates a degree-15 odd atan polynomial, restores octants, reflects across negative `x`, and copies the sign of `y`. |
| Exponential | `emit_exp_reconstruction` rounds `x / ln(2)` to an integer, reconstructs a reduced residual with split high/low `ln(2)`, evaluates a degree-7 Taylor polynomial, and constructs `2^k` with integer bias, shift, and f32 bitcast. `Exp` uses the normal path. |
| Gradual underflow | `emit_exp_with_underflow` cuts off below `ln(2^-150)` at positive zero, clamps before f32-to-i32 conversion, and invokes reconstruction with subnormal support. Exponents below `-126` are split as `2^(k+64) * 2^-64`, leaving the final IEEE multiply to round subnormals. |
| `ExpMinusOne` | `emit_expm1_core` uses a degree-8 local series for `|x| <= 0.25` and `Exp(x) - 1` otherwise, then restores a signed zero. |
| Logarithms | `emit_log_core` scales positive subnormals, extracts exponent and mantissa bits, normalizes the mantissa around one, and evaluates an atanh series through `y^13` with split `ln(2)`. `emit_log1p_core` uses a degree-12 local series for `|x| <= 0.25` and `Log(1+x)` otherwise. |
| Rounding and sign | `emit_trunc` selects `Ceiling` for negative inputs and `Floor` otherwise. `emit_sign` selects `-1`, the original zero, or `1` from ordered comparisons. |
| Stable activations | `emit_sigmoid_core` chooses a sign-stable quotient, `emit_tanh_core` evaluates `expm1(2*abs(x))` and restores sign, and `emit_softplus_core` computes `max(x,0) + log1p(exp(-abs(x)))`. |
| Error function | `emit_erf_core` evaluates the Abramowitz-Stegun 7.1.26 rational form on `abs(x)`, multiplies its tail by Recipe `Exp(-x^2)`, subtracts from one, and restores the input sign. |

The implementation constants in `program.rs` are algorithm data, not runtime
configuration. They include split `ln(2)` pieces, Cody-Waite `pi/2` pieces,
the gradual-underflow cutoff, the 64-bit exponent split, and polynomial
coefficients. No backend crate supplies replacement coefficients.

## Ownership in the workspace

The dependency direction and the actual call sites make the ownership boundary
explicit:

| Layer or source | Relationship to `recipe-math` |
| --- | --- |
| `recipe-core` | Owns `DType`, `ScalarOpcode`, `ScalarProgram`, scalar validation, and the fault-flag meaning of `Require`. Math consumes those types and emits no new opcode. |
| `recipe-language` | Owns `ScalarProgramBuilder`, `ScalarExpression`, and `LanguageError`. Math uses the builder so opcode signatures and SSA ownership are checked while the graph is emitted. |
| `recipe-ops` scalar registry | Owns operation symbols and the `ScalarRecipe::Math(MathFunction)` variant. `ScalarRecipe::for_symbol` maps the canonical GPU symbols listed below, and `lower_scalar` calls `ScalarProgram::try_from(function)` then validates the result. |
| `recipe-ops` composer | Owns tensor-elementwise composition and inline remapping. `Composer::inline_math` rebuilds the math program into its own value namespace, checks argument count and `DType::F32`, preserves constants and opcodes, and returns an `OperationError` on any invalid program. |
| `recipe-ops` materializers | Own shape and stage wiring. Positional encoding constructs `Sin` and `Cos`; attention and reinforcement-learning materializers construct `Exp` and `Log`; the resulting programs are placed in `PrimitiveKind::Elementwise` stages. |
| `recipe-training` inference compiler | Owns graph-level stable inference. `stable_sigmoid` forms `-abs(logit)`, invokes `exp_with_gradual_underflow_program`, then builds the sign-dependent quotient. `stable_softmax` max-shifts logits, constrains the shifted exponent input to finite nonpositive values, invokes the same gradual program, and reduces exponentials. |
| root `recipe` facade | `src/facade.rs` re-exports the crate as `recipe::engine::math` for advanced callers. The root does not copy math names into its fluent declaration API. |
| kernel, primitive, planner, scheduler, prepare, executor, CUDA, and HSA crates | Consume the resulting `ScalarProgram` only after the math crate has finished. They own lowering, measured scheduling, resource realization, driver work, and execution, not mathematical definitions. |

The only workspace manifests that depend directly on `recipe-math` are the
root package, `ops`, and `training`. No other crate reaches into `contract.rs`
or `program.rs`; all callers use the root names and the two construction paths.

### Operation symbols mapped to `MathFunction`

`recipe-ops/src/scalar.rs` currently maps these source-qualified operation
symbols to the math enum:

```text
gpu_atan2          -> Atan2
gpu_ceil           -> Ceil
gpu_cos            -> Cos
gpu_exp            -> Exp
gpu_expm1          -> ExpMinusOne
gpu_floor          -> Floor
gpu_fmod           -> Fmod
gpu_log_into       -> Log
gpu_log1p          -> LogOnePlus
gpu_pow            -> Pow
gpu_reciprocal     -> Reciprocal
gpu_round          -> RoundNearestEven
gpu_rsqrt          -> ReciprocalSquareRoot
gpu_sigmoid_into   -> Sigmoid
gpu_sign_into      -> Sign
gpu_sin            -> Sin
gpu_softplus       -> Softplus
gpu_tan            -> Tan
gpu_tanh_into      -> Tanh
gpu_trunc          -> Trunc
```

`ExpWithGradualUnderflow` and `Erf` are in the public 22-function contract but
have no current `for_symbol` entry. The former is intentionally reached by
the explicit training inference helper; neither function is silently mapped to
an unrelated operation. The registry's `Math` dtype contract derives one or
two F32 inputs from `arity()` and one F32 output, so the enum remains the sole
source of scalar function arity.

## Non-responsibilities and handoff

`recipe-math` does not own any of the following:

* tensor dimensions, broadcasting, views, aliases, reductions, or external
  values;
* operation-surface compatibility names, operation descriptors, primitive
  kinds, or materialization ABI checks;
* f16, f64, integer math functions, host-side reference calculations, or
  vendor math libraries;
* target selection, LLVM emission, PTX or HSACO construction, artifact
  identities, or native driver interfaces;
* hardware probing, measured rates, placement, queues, transfer routes,
  memory allocation, lifecycle state, or run-time fault handling.

The handoff is one-way: a caller selects a `MathFunction`, asks for its
contract if metadata is needed, constructs one validated `ScalarProgram`, and
then gives that program to the owning elementwise or training compiler. The
downstream compiler may inline or bind the program, but it must not replace its
coefficients, weaken its `Require` predicates, or reinterpret its signed-zero
and algorithm identity rules. The final native run therefore retains the
math crate's backend-neutral semantics while later crates own only placement,
realization, and execution.

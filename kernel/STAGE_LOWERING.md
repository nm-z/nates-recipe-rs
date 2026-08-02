# Recipe stage lowering contract

`recipe-kernel::lower_stage` realizes one planner-owned `ProgramStage` into target
LLVM IR. It does not infer a different stage, choose a different algorithm, or
repair a stale artifact recipe. The lowered program and the complete
`ArtifactBuildRecipe` are both inputs because realization is a fail-closed
verification boundary.

## Inputs and validation

Before emitting IR, realization independently checks all of the following:

- canonical validation and digest of the complete `LoweredProgram`;
- canonical validation of the `ArtifactBuildRecipe`;
- target and lowering-option validity;
- source-kernel identity and stage ordinal;
- the planner's stage-scoped kernel-template identity;
- reserved artifact identity;
- the independently recomputed `recipe-planner-artifact-build-v1` digest;
- dispatch geometry and work bounds;
- private, shared, scratch, and workgroup resource bounds;
- binding order, dtype, access, extents, offset, strides, and storage size;
- the exact checked-path fault binding.

Any mismatch returns `InvalidStageContract`. Realization never silently lowers a
nearby or compatible-looking contract.

## Native ABI

Arguments are expanded in this fixed order:

1. readable data pointers in stage-binding order;
2. writable data pointers in stage-binding order;
3. the optional fault-flag pointer;
4. the dynamic `RunId`, for Philox stages only;
5. `element_count`.

A read-write binding therefore has distinct read and write pointer slots. Each
ABI slot occupies eight bytes. The HSA metadata inspector requires the dynamic
run value to be an eight-byte by-value `run_id` argument. CUDA and HSA launchers
bind `KernelArgument::RunId` from the run context; it is never embedded in an
artifact.

## Owned stage algorithms

All primitive stage kinds have deterministic Recipe-owned lowering:

- scalar maps use the complete scalar IR and strict constrained floating-point
  operations;
- fill and copy use the exact affine binding views;
- reductions use the planner's fixed tree, operator identity padding, and the
  lowest logical index for ties;
- local scans use the planner's fixed Blelloch upsweep/downsweep tree, while
  uniform-combine stages apply the preceding hierarchy level;
- contractions traverse contracted coordinates in canonical order and execute
  both workgroup barriers for every fixed reduction tile, including inactive
  lanes in a partial workgroup;
- gather and scatter implement reject, clamp, and wrap index policies;
- scatter and histogram atomics preserve the planned operation and ordering;
- histogram clear and accumulation use the exact bin mapping;
- stable sort uses a padded bitonic network, IEEE-754 total-order keys, and the
  original axis index as the stable tie-break;
- random stages use the Philox4x32-10 V1 contract described below.

Rejecting gather, scatter, and histogram paths publish their exact fault code
with a release `atomicrmw xchg`. The bounds branch precedes every payload address
calculation. Integer atomics are direct LLVM atomics. Floating-point add, minimum,
and maximum use Recipe-owned compare/exchange loops with constrained arithmetic,
IEEE total-order comparison, and canonical NaN output.

Generated modules declare only LLVM intrinsics. They do not call HIP, CUDA
Runtime, rocBLAS, rocSOLVER, rocFFT, MIOpen, RCCL, cuBLAS, cuSOLVER, cuFFT,
cuDNN, NCCL, or any other vendor math implementation.

## Philox4x32-10 V1

The four 32-bit counter words are:

```text
[element_low, element_high, low(run_id XOR stream), high(run_id XOR stream)]
```

The two key words fold the 128-bit seed and source-kernel identity:

```text
key_0 = low(seed_low)  XOR high(seed_high) XOR low(source_kernel)
key_1 = high(seed_low) XOR low(seed_high)  XOR high(source_kernel)
```

Each round uses multipliers `0xd2511f53` and `0xcd9e8d57`; the first nine key
advances use Weyl constants `0x9e3779b9` and `0xbb67ae85`. The helper is emitted
directly into the LLVM module and is unrolled to exactly ten rounds.

Distribution mappings are versioned with the Philox contract:

- `UniformF32` uses the upper 23 random bits to construct a value in `[0, 1)`.
- `BernoulliI32` compares a word with `floor(p * 2^32)`; `p = 1` is handled
  exactly.
- `UniformI32` uses unbiased multiply-high mapping. A low product below
  `(-range) mod range` retries with counter word three XORed with the retry
  ordinal.
- `NormalF32` uses two 24-bit uniforms in `(0, 1]` and the Recipe-owned
  Box-Muller V1 mapping.

Box-Muller V1 evaluates:

```text
sqrt(-2 * log(u1)) * cos(2*pi*u2)
```

The logarithm decomposes exponent and mantissa, then evaluates the odd atanh
series through `y^15`, where `y = (m - 1) / (m + 1)`. Cosine is reduced to
`[0, pi/2]` and evaluated by a Horner polynomial through `x^10`. Arithmetic uses
LLVM constrained operations with round-to-nearest-even, `strictfp`, IEEE f32
denormals, and canonical NaNs. The V1 approximation envelope is at most
`2e-4` absolute error relative to real-valued Box-Muller over its finite 24-bit
input grid; 24-bit input quantization is part of the distribution contract.

Changing the counter layout, key fold, retry collision rule, uniform mapping, or
normal approximation requires a new version. Recompiling V1 for AMDGPU and
NVPTX must preserve this algorithm and dynamic-run ABI.

# Integration boundary

`recipe-primitives` owns backend-neutral primitive lowering only. Its public
entry point accepts one validated `recipe_language::PrimitiveKernel` plus the
graph's tensor index and returns a self-validating `LoweredProgram`.

The lowered program retains:

- the source kernel ID and complete input/output alias matrix;
- every immutable stage in dependency order;
- the complete `recipe_core::KernelTemplate` for scalar-map stages;
- owned templates for reduction, scan, contraction, gather, scatter,
  histogram, stable sort, and Philox4x32-10 stages;
- typed static buffer views, scratch/fault lifetimes, fixed dispatch geometry,
  barriers, atomics/orderings, checked-fault behavior, exact resource bounds,
  and a domain-separated canonical digest.

The scoped identity of a lowered stage is `(source_kernel, StageId)`.
`ArtifactIdentity` is intentionally absent because its target, ABI, toolchain,
format, entry symbol, and realized resource envelope do not exist until target
realization. Fabricating those fields here would make the AOT contract lie.

The planner integration now:

1. consumes every stage, including honest zero-stage empty programs;
2. places external, scratch, and fault buffers while preserving declared
   lifetimes and alias contracts;
3. translates stage dependencies into calculation tasks and fixed
   synchronization edges;
4. emits one target-independent artifact build recipe per scoped stage
   identity; and
5. requires Realize and Finalize to bind the resulting artifact identity back
   to that exact stage contract.

Kernel integration must reuse the existing scalar `KernelTemplate` emitter for
`StageKind::ScalarMap` and add Recipe-owned emitters for each other
`StageKind`. Those emitters must implement the encoded trees, index guards,
memory orderings, total-order stable comparator, and Philox constants exactly;
they may not replace them with vendor libraries or runtime-selected algorithms.

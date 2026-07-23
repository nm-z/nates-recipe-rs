# Primitive integration

The planner consumes every validated `recipe-primitives::LoweredProgram`.
Elementwise, reduction, scan, contraction, gather, scatter, histogram, stable
sort, and Philox random-map stages all cross the same Draft boundary.

Each nonempty lowered stage becomes:

- one collision-checked stage-scoped `KernelTemplateId`;
- one scheduled calculation task with the lowered dependency edges;
- exact resident tensor, scratch, and preallocated fault-flag values;
- one reserved `ArtifactId`; and
- either an exactly matching prebuilt artifact or a target-independent
  `ArtifactBuildRecipe`.

The deferred recipe retains the lowered-program digest, source kernel, stage
ordinal, ordered typed views, access modes, dispatch geometry, operation
bounds, fault binding, and resource envelope. Realize must return an artifact
with the same reserved ID, stage identity, provenance, resources, and the
target discovered for the selected device. Finalize retains that build
contract beside the realized identity.

Zero-element programs are honest no-dispatch candidates: they emit no
calculation task, artifact, or fabricated stage.

No physical arena offsets or runtime/backend actions are performed here.
Arena layouts remain deterministic feasibility evidence until Finalize.

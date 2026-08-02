# `recipe-core`

`recipe-core` is Recipe's dependency-free contract crate. It contains the
typed values exchanged by discovery, graph lowering, candidate drafting,
target realization, static scheduling, arena packing, finalization, and
execution. It has no driver, compiler, allocator, filesystem, network, or
runtime integration. The crate models those boundaries and validates the
invariants that make a finalized execution bundle safe to consume.

The public surface is intentionally flat. `core/src/lib.rs` declares the ten
modules and re-exports their public items, so downstream crates normally import
contracts directly from `recipe_core` rather than from module paths.

## Position in the pipeline

The core data flow is:

```text
host and GPU probe evidence
        |
        v
Topology + DiscoveryProfile
        |
        +--> backend-neutral CalculationGraph and StaticCalculationProgram
        |          (language, primitives, ops, program)
        |
        +--> artifact catalog or deferred ArtifactBuildRecipe
        |
        v
DraftPlan
  exact tasks, values, aliases, images, resources, and logical arena objects
        |
        v
RealizationProfile
  realized artifacts, reservations, and post-warm capacity evidence
        |
        v
FinalizedBundle
  validated loop domains, physical arena layouts, resolved value locations,
  and immutable execution contracts
        |
        v
recipe-executor, native-executor, host, and remote backends
```

`recipe-core` owns the representations and cross-stage validation. The
algorithms that construct them live elsewhere: `recipe-probe` constructs
measured topology and discovery, `recipe-language` and `recipe-primitives`
construct calculation descriptions, `recipe-planner` emits drafts,
`recipe-scheduler` assigns windows and transfer lanes, `recipe-prepare` runs
the realization fixed point, and the executor crates consume only the
finalized product.

There is no implicit fallback between stages. Identities tie each value to the
topology, discovery profile, candidate, draft, realization, and final bundle
that produced it. Validation rejects a mixture of objects from different
stages or different measured systems.

## Manifest and module graph

`core/Cargo.toml` declares `recipe-core` version `0.1.0`, Rust edition 2024,
MIT licensing, and no dependencies. The crate forbids unsafe code and denies
missing `Debug` implementations.

The implementation dependency direction is deliberately one-way toward the
stage contracts:

```text
identity   ids   units   error
   |        |      |       |
   +--------+------+-------+--> artifact
   +--------+------+-------+--> topology
   +--------+------+-------+--> scalar
                 |            |
                 +-----------> schedule
artifact + topology + scalar + schedule + discovery
                 |
                 +-----------> plan
```

More precisely:

| Module | Depends on | Public responsibility |
| --- | --- | --- |
| `identity.rs` | `core::fmt` | Validated labels, 256-bit digests, and typed stage identities. |
| `ids.rs` | `core::fmt` | Opaque stable `u64` identifiers for every graph, resource, task, and run object. |
| `units.rs` | `core::fmt` | Typed byte, rate, operation, element, and schedule-time quantities with checked arithmetic. |
| `error.rs` | `core::fmt` | Machine-readable validation codes, path-aware errors, and the crate-private error accumulator. |
| `artifact.rs` | identity, ids, scalar, units, error | Target-independent build recipes and realized artifact identity/resource contracts. |
| `topology.rs` | identity, ids, units, error | Machines, nodes, storage devices, directed links, transport duplex, and property provenance. |
| `scalar.rs` | ids, units, error | F32/I32 scalar SSA, typed opcode rules, affine buffer access, and kernel templates. |
| `schedule.rs` | ids, scalar, topology, units, error | Lifecycle phases, loop domains, tasks, transfer lanes, metric readbacks, resources, and arena contracts. |
| `discovery.rs` | artifact, identity, ids, topology, units, error | Immutable measured capability snapshots associated with one topology. |
| `plan.rs` | all contract modules | Reservations, capacity accounting, Draft, Realization, Finalize, and cross-stage validation. |

`error::Validator` is `pub(crate)`. Callers receive `ValidationResult`, and a
single validation pass can aggregate many path-specific failures in
`ValidationErrors`.

## Identity and stable identifiers

### Text and digests

`identity::Label` is a nonempty, non-whitespace string. Construction is the
only place that enforces this lexical invariant; consumers use `as_str` and
`Display` without revalidating it. `identity::Digest` wraps exactly 32 bytes,
provides a `ZERO` value, and prints only a short prefix in `Debug`. It is a
caller-computed content or manifest digest. Core does not choose a hash
algorithm or compute digests.

The digest newtypes are:

* `TopologyIdentity` for one canonical topology;
* `DiscoveryIdentity` for one capability snapshot;
* `DraftIdentity` for one exact candidate draft;
* `CandidateIdentity` for one placement assignment;
* `RealizationIdentity` for one realized, stabilized candidate;
* `BundleIdentity` for one finalized execution product.

Each wrapper preserves the underlying digest and exposes `digest` and
`is_zero`. Core validators reject a zero identity wherever a stage object must
be bound to an actual artifact or profile. The producer crates compute the
domain-separated digests: probe and cluster derive topology/profile identity,
planner derives candidate and draft identity, prepare derives realization and
bundle identity, and transport/remote carry the values without redefining
them.

`artifact::TargetIdentity` separates backend, architecture, and ABI. A
`ToolchainIdentity` records toolchain name, version, and digest. These are
descriptive identities, not compiler or driver handles.

### Opaque IDs

`ids.rs` uses one macro to define strongly typed wrappers around `u64`:

```text
MachineId, NodeId, DeviceId, LinkId, TransportId, DuplexResourceId,
ValueId, ScalarValueId, KernelTemplateId, KernelInputId, KernelOutputId,
TaskId, ArtifactId, QueueSlotId, CompletionSlotId, MetricId, MetricSlotId,
ArenaObjectId, RunId
```

Every wrapper has `new`, `get`, ordering, hashing, and decimal `Display`. The
wrapper itself does not reject zero, because zero can be useful while decoding
or reporting an invalid object. Stage validators impose the relevant rule:
reserved IDs such as artifacts, stage templates, scalar outputs, and metrics
must be nonzero, and duplicate IDs are rejected in their owning collection.

## Units and checked arithmetic

`units.rs` prevents domain quantities from being confused with plain integers.

* `ByteCount` is an exact decimal byte quantity with checked add, subtract,
  multiply, and power-of-two `checked_align_up`.
* `ByteOffset` is an arena offset and has checked end computation against a
  `ByteCount`.
* `BytesPerSecond`, `FlopsPerSecond`, `TransferLaneCount`, and `ElementCount`
  reject zero at construction.
* `FlopCount` and `Nanoseconds` permit zero and provide checked accumulation.
* `transfer_time_ceil` and `calculation_time_ceil` compute integer nanoseconds
  using a u128 intermediate and a ceiling division. Overflow is reported as
  `UnitError::Overflow` rather than wrapping.

The scheduler consumes these functions with measured rates. Core does not
choose rates, benchmark hardware, or apply a policy to estimates. A
`Property<T>` in topology and discovery records whether a value is estimated,
measured, or explicitly overridden, and the scheduling validators admit only
the latter two.

## Validation errors

`error::ValidationCode` is the shared vocabulary for contract failures. It
covers identity and reference errors, topology ownership and duplex errors,
scalar typing, memory access, lifecycle ordering, task scheduling, capacity,
artifact/resource matching, arena allocation, value bindings, aliasing, and
lifetime overlap.

`ValidationError` stores a code, a dotted/indexed path, and a human-readable
message. `ValidationErrors` preserves every failure found in one pass and
implements `Display` by joining them with `; `. `Validator` supports required
conditions, unconditional errors, prefixing nested errors, and a final
`Result<(), ValidationErrors>`. This lets a Draft validation report failures
across topology, discovery, resources, tasks, arenas, images, and aliases in
one call instead of failing at the first field.

## Topology and measured discovery

### `topology.rs`

`Topology` is the complete static description of storage and transport:

* `Machine` names a machine by `MachineId` and `Label`.
* `Node` assigns a `NodeRole` (`Master` or `Worker`) and a set of devices to a
  machine. Exactly one master is required.
* `Device` identifies a `DeviceKind` (`GpuMemory`, `Ram`, or `Disk`), its
  machine, capacity, transfer rate, and optional calculation rate. Only GPU
  memory may expose a calculation rate.
* `DirectedLink` is one measured direction of a paired transport. Both
  directions share a `TransportId`; half-duplex directions share one
  `DuplexResourceId`, while full-duplex directions use distinct resources.
  Directional bandwidth and inflight lane counts may differ.
* `TransportKind::required_duplex` defines the transport's required duplex
  mode. Memory, PCIe, SAS, NVMe, and Ethernet are full duplex; SATA and WLAN
  are half duplex.

`Topology::validate` checks nonzero identity, unique machine/device/node/link
  IDs and machine names, machine/device ownership, one owner per device,
  exactly one master, complete device ownership, paired reverse links,
  transport-kind and duplex consistency, and unique capacity resources.
`validate_scheduling_properties` is a separate production gate. It rejects
estimated capacity, transfer, calculation, bandwidth, and lane properties
before Draft scheduling. Estimates may be used by probing only.

Lookup helpers (`machine`, `device`, `link`) intentionally return references to
the declarative vectors. `duplex_mode` recognizes only a valid reverse pair.
`validate_route` accepts an empty route only for a same-device copy; a
distinct-device route must contain links whose `from` endpoints chain from the
source to the destination.

### `discovery.rs`

`DiscoveryProfile` is the immutable capability snapshot for one topology. It
contains a `DiscoveryIdentity`, the associated `TopologyIdentity`, one
`DiscoveredDevice` per topology device, and one `DiscoveredLink` per topology
link.

`DiscoveredDevice` records availability, maximum submission queues, total
capacity, transfer capability, and an optional `CalculationCapability`.
Transfer capability includes measured rate, inflight lane count, asynchronous
submission, and whether transfers overlap calculation. GPU calculation
capability additionally records a `TargetIdentity`, FLOP rate, asynchronous
submission, maximum concurrent tasks, subgroup width, maximum workgroup size,
and workgroup-local memory limit. RAM and disk devices must not expose it.
`DiscoveredLink` records availability, bandwidth, directional lane count, and
asynchronous submission.

`DiscoveryProfile::validate` requires nonzero identity, exact topology identity
match, complete unique coverage of devices and links, availability, measured or
overridden schedulable properties, asynchronous transfer, and the device-kind
specific capability rules. Link measurements must equal the topology's
directional properties. The `device` lookup is used by planners and validators
to bind a task or artifact to measured capabilities.

`recipe-probe` constructs this profile from exhaustive host/GPU enumeration and
bounded measurements. Its engine assigns stable IDs, builds paired links, and
marks measurements as `PropertyProvenance::Measured`. `probe::codec` persists
the topology and discovery data. `recipe-cluster` validates member profiles
and remaps their identities into one complete measured cluster profile.

## Scalar programs and kernel templates

### `scalar.rs`

Recipe's calculation payload domain is intentionally only `DType::F32` and
`DType::I32`, both four bytes wide. `ScalarLiteral` preserves binary32 bits
without a host floating-point round trip. `ScalarOpcode` covers arithmetic,
comparisons, selection, bit operations and casts, checked integer operations,
fault-producing `Require`, finite/NaN predicates, and selected f32 functions.

`ScalarOpcode::arity` and `result_dtype` are the single signature authority.
The rules enforce same-type arithmetic, f32-only FMA and transcendental-like
operations, int32 predicates/bit operations, and int32 comparison results.
`flops` counts only arithmetic work used by the base scheduling equation: FMA
counts as two, comparisons and ordinary f32 arithmetic count as one, and
addressing, casts, bit manipulation, predicates, and validation operations
count as zero.

`ScalarProgram` is an ordered SSA-like list of inputs, constants,
instructions, and outputs. `validate` checks unique scalar IDs, use-before-
definition, opcode arity and result type, at least one output, and known
outputs. `dtype_of` resolves a value type. `requires_fault_flag` detects the
checked operations that need a preallocated device int32 fault channel.

`IndexSpace` stores nonempty dimensions and a checked product of element
counts. `StaticBufferAccess` is an immutable affine mapping with element-based
offsets and strides plus complete backing byte size. Read-only zero-stride
broadcasts are legal. Writable mappings must be injective, and every mapping
must have a nonoverflowing address span within `storage_bytes`.

`KernelTemplate` combines an index space, ordered typed `KernelInput` and
`KernelOutput` arguments, a `ScalarProgram`, and a complete input/output alias
matrix. Validation checks the scalar program, unique argument IDs, access
spans, argument/program arity and types, valid references, no duplicate alias
pairs, and one explicit `AliasRule` for every input/output pair. The alias
permissions are `Forbidden`, `MayAliasExact`, and `MustAliasExact`.

### Producers and lowerers

`recipe-language::ScalarProgramBuilder` is the safe constructor for this IR. It
assigns per-builder scalar IDs, rejects expressions from another builder,
delegates signatures to `ScalarOpcode`, and validates the completed
`ScalarProgram`. `recipe-language::CalculationGraph` owns tensors and
placement-free primitive kernels, uses `ValueId` and `KernelTemplateId`, and
validates producer uniqueness and graph acyclicity.

`recipe-primitives` lowers each `PrimitiveKernel` to a `LoweredProgram`. It
uses measured subgroup/workgroup/shared-memory limits, creates core
`KernelTemplate` values for scalar stages, and fixes all buffer views,
dispatch geometry, synchronization, atomics, fault contracts, and resource
bounds before target realization. `recipe-ops` and `recipe-training` build
the language-level graphs and scalar programs but do not add placement or
driver state.

## Artifact contracts

`artifact.rs` separates target-independent Draft intent from target-realized
images.

`ArtifactBuildRecipe` names a reserved `ArtifactId`, a stage-scoped
`KernelTemplateId`, the source kernel, build provenance, ordered argument
bindings, dispatch geometry, exact work bounds, optional fault flag, and
kernel-resource bounds. Its recipe deliberately contains no target, format,
entry symbol, or toolchain. Validation checks nonzero IDs and provenance,
nonempty geometry, exact ceiling workgroup count, exact drafted workgroup
resource, unique values, rank-matched affine views, and the exact int32 atomic
fault binding when present.

`ArtifactBuildAccess`, `ArtifactBuildView`, and `ArtifactBuildBinding` are the
target-independent ABI description. `ArtifactBuildAccess::reads` and
`writes` derive the ordered calculation inputs and outputs. `ArtifactWorkBounds`
retains FLOP, integer, and atomic operation bounds even when the scheduler's
base equation prices only FLOPs. `ArtifactBuildProvenance` binds a realized
image to a lowered program digest, stage ordinal, and contract digest.

`ArtifactIdentity` is the complete realized artifact identity: image digest,
format, target, toolchain, entry symbol, stage template, resource envelope,
and optional deferred-build provenance. Validation requires nonzero image and
toolchain digests, a nonzero workgroup limit, and valid deferred provenance.

`recipe-planner` creates one build recipe per lowered stage. It selects an
exact matching catalog artifact when available and otherwise places the recipe
in `DraftPlan::artifact_builds`. `recipe-kernel` and
`recipe-prepare::DeferredArtifactCompiler` lower deferred recipes to LLVM and
produce target-specific cubin or hsaco identities during Realize. Native
executor adapters validate the runtime image digest, target ABI, entry symbol,
workgroup geometry, and finalized stage mappings before loading it.

## Scheduling contracts

### Lifecycle and loop activation

`RunPhase` is exactly `Init`, `Loop`, or `Exit`. Core tasks retain one immutable
graph; repetition is represented by `LoopIterations` and `IterationDomain`,
not by cloning tasks or artifacts.

`LoopIterations::Finite` contains a nonzero `NonZeroU64`; `Unbounded` has no
invented terminal iteration. `LoopIteration` is a zero-based index paired with
the total. `IterationDomain` is a nonempty half-open arithmetic progression,
finite or unbounded, with a nonzero stride. Its `contains` and `is_within`
methods are used by program and Finalize validation.

`LoopTaskDomain` assigns one exact domain to one loop task. `LoopSchedule`
bundles the total iteration contract and assignments supplied to Finalize.
`recipe-program::StaticCalculationProgram` exposes the same domains at the
user-facing graph boundary, validates producer coverage and metric domains,
and serializes them in canonical OGDL without unrolling the graph.

### Values, images, and tasks

The schedule model is deliberately concrete:

* `ValueSpec` gives a value identity, F32/I32 type, exact bytes, resident
  device, and optional producer task.
* `ValueBinding` places a value at an offset inside a logical arena object;
  Finalize later adds the physical arena offset.
* `ValueAliasContract` records the selected storage relationship for a source
  alias pair.
* `InitDataImage` describes one packed admission image per required device.
  Members connect logical graph inputs to physical resident values and exact
  byte offsets inside the image.
* `ResolvedValueLocation` is the immutable physical result of combining a
  value binding with a validated `ArenaAllocation`.

`CalculationTask` is a loop-only GPU calculation with a stage template,
artifact, ordered resident inputs and outputs, optional fault flag, FLOP work,
and submission slots. `TransferTask` is one executor-visible physical hop or
an external admission/egress. Distinct-device internal transfers contain at
most one directed link; longer paths are dependency-chained tasks with
intermediate resident values. `TransferLaneClaim` records every selected
directed-link or external-transfer lane. `MetricTask` is an asynchronous
four-byte device readback with a `MetricPurpose` of user telemetry or mandatory
fault readback. It is a specialized transfer contract, not a third model kind
of work.

`Task` adds identity, phase, half-open `ScheduleWindow`, dependencies, and one
`TaskKind`. Windows must be nonempty. Dependencies must refer to existing tasks,
stay within lifecycle order, finish before the dependent window, and form an
acyclic graph. Init admission is `External -> Device` in `Init`; internal
copies are device-to-device; exit egress is `Device -> External` in `Exit`.

`ResourceManifest` fixes queue slots, completion slots, metric slots, pinned
staging, and scratch bytes before execution. Its validation ensures unique
slot IDs, known devices, and at most one staging/scratch entry per device.
`ScheduleWindow::overlaps` is the primitive used by the contention checks.

### Scheduler ownership

`recipe-scheduler::schedule` consumes a validated topology, discovery profile,
and `UnscheduledTask` list. It computes durations with
`calculation_time_ceil` and `transfer_time_ceil`, adds measured compute and
transfer lanes, respects queue/completion exclusivity, half-duplex resources,
and transfer/compute overlap capability, then emits windows and canonical lane
claims. The scheduler also chooses direct routes when a route is omitted, but
rejects a multi-hop result unless the planner has already split it into hop
tasks.

`recipe-scheduler::pack_arenas` consumes core `ArenaObject` lifetimes and a
realized `CapacityLedger`. It selects the lowest aligned legal offset, reuses
bytes only for nonoverlapping lifetimes, and emits one exact `ArenaLayout` per
device. Core validates the result again at Finalize.

## Draft, realization, and finalization

### Reservation and capacity ledgers

`EXACT_USER_RESERVATION` is the one authoritative one-billion-byte reservation
constant. `ReservationEvidence` requires that amount for non-GPU devices and
for GPUs with one or more enabled display connectors, and zero for a headless
GPU. `ReservationLedger` requires exactly one entry per topology device,
matching device kind and evidence, exact bytes, and no duplicate device.
`ReservationMechanism` distinguishes a held allocation from an enforced quota;
the realization backend supplies the mechanism and evidence.

`CapacityLedgerEntry` decomposes each device's total capacity into runtime
overhead, fragmentation, safety headroom, and Recipe-usable bytes. Every field
must be measured or explicitly overridden for production. The checked sum of
the exact reservation plus all four components must not exceed total capacity,
and every topology device needs one entry. `CapacityLedger` is an observation,
not a promise that a Draft will fit until arena packing succeeds.

### `DraftPlan`

`DraftPlan` is the exact offset-free candidate emitted by planning. It carries:

* `DraftIdentity`, `CandidateIdentity`, `DiscoveryIdentity`, and
  `TopologyIdentity`;
* all `ValueSpec` and `KernelTemplate` declarations;
* prebuilt `ArtifactIdentity` values and deferred `ArtifactBuildRecipe` values;
* fully lowered `Task` values and the `ResourceManifest`;
* logical `ArenaObject` lifetimes, `ValueBinding` mappings, alias contracts,
  packed init-image manifests, and one release per device.

`DraftPlan::validate` first validates topology structure and schedulable
properties, then the discovery snapshot and stage identities. It validates
resources, values, kernels, artifacts, and deferred builds, and then validates
tasks, arena objects, value bindings, aliases, init images, releases, task
dependency acyclicity, fault readbacks, upload cardinality, transfer lanes,
resource contention, and measured capability concurrency.

Important cross-field rules include:

* each calculation resolves to exactly one realized artifact or deferred build;
* calculation device, artifact target, stage template, ordered bindings, work,
  and optional fault flag agree;
* external admissions and egress occur only in their lifecycle phases;
* every topology device has exactly one init image admission and one release;
* each value has one aligned in-bounds binding whose lifetime contains every
  producer and user;
* alias permissions are checked against actual object/offset byte ranges;
* every checked calculation cohort has exactly one exclusive fault readback,
  directly dependent on all checked calculations and preceding every publish or
  exit task;
* overlapping tasks may not reuse queue/completion slots or transfer lanes,
  exceed measured lane/concurrent-task limits, or overlap calculation on a
  device that lacks the discovered overlap capability.

### `RealizationProfile`

`RealizationProfile` is the stable result of Realize for one unchanged Draft.
It records a new `RealizationIdentity`, the exact Draft/candidate/discovery/
topology identities, every prebuilt artifact plus every realized deferred
artifact, the unchanged `ResourceManifest`, the reservation ledger, and the
post-warm `CapacityLedger`.

Validation requires nonzero realization identity, unchanged upstream
identities, exact resource equality with the Draft, one realized artifact for
each prebuilt or deferred artifact and no extras, exact build provenance and
resource equality for deferred stages, target agreement with every task's
discovered calculation capability, and valid reservations and capacity
accounting. Realize may reject a candidate, but it cannot mutate a Draft to
make it fit.

### `FinalizedBundle`

`FinalizedBundle` is the only constructor-validated runtime product. Its fields
are private; callers receive accessors for identity, loop domains, stage
artifacts/builds, kernel templates, tasks, resources, reservations, arena
layouts, resolved value locations, aliases, and init images.

`finalize` uses one loop iteration. `finalize_with_loop_iterations` assigns
`IterationDomain::every` to each loop task. `finalize_with_loop_schedule`
accepts exact per-task domains. All forms validate the Draft, Realization,
domains, and arena layouts before resolving each value to a physical offset.
No bundle exists if any validation fails.

Arena validation requires one layout per topology device, one allocation per
logical object, correct device and power-of-two alignment, checked bounds,
exact maximum allocation end as layout size, capacity fit, and no overlap of
live objects. Value resolution then combines layout offset and object-relative
binding offset with checked arithmetic and rechecks device, alignment, and
arena bounds.

`FinalizedBundle::transfer_endpoints` resolves a transfer's logical device
values to `ResolvedValueLocation` while preserving `External` endpoints.
This is the bridge used by executors and remote ownership classification. It
does not allocate, copy, or inspect runtime memory.

## Major consumers and ownership boundaries

The following consumers are direct, observed users of the core contracts.

| Consumer | Core contracts used | What remains outside core |
| --- | --- | --- |
| `recipe-probe` | `Topology`, `DiscoveryProfile`, `Property`, IDs, labels, digests, target/toolchain identities | Host/GPU enumeration, benchmarks, profile codec, and cache invalidation. |
| `recipe-cluster` | Topology/discovery objects, IDs, duplex resources, measured properties, identities | Membership configuration, peer evidence validation, deterministic remapping, cluster hashing. |
| `recipe-language` | `DType`, `ValueId`, `KernelTemplateId`, scalar IR, byte/offset units | Graph assembly, tensor shape/layout semantics, scalar builder ownership, OGDL. |
| `recipe-primitives` | `KernelTemplate`, scalar IR, access types, `IndexSpace`, FLOP and byte units | Primitive lowering, collective trees, synchronization, atomics, fault contracts, resource formulas. |
| `recipe-ops` and `recipe-training` | scalar programs, graph IDs, iteration domains and metric IDs | Operation inventory, model/training semantics, composition and user declarations. |
| `recipe-planner` | topology/discovery, artifacts/build recipes, all Draft/task/value/arena/image types, identities | Finite placement enumeration, stage lowering, transfer-chain decomposition, deterministic hashing, candidate ranking. |
| `recipe-scheduler` | measured properties, tasks, windows, routes, lane claims, arena objects/layouts, units | Critical-path list scheduling, contention search, shortest measured route, arena packing. |
| `recipe-prepare` | measured profile, reservations/capacity, Draft, Realization, LoopSchedule, FinalizedBundle | Artifact catalog/provider, native realization, warmup, stabilization policy, capacity snapshots, stage hashing. |
| `recipe-kernel` and deferred preparation | `ArtifactBuildRecipe`, provenance, geometry, bindings, resource bounds, target/toolchain identities | LLVM lowering, offline toolchain invocation, cubin/hsaco inspection and image construction. |
| `recipe-executor` | `FinalizedBundle`, tasks, loop domains, resolved locations/endpoints, phases, metric purpose | Typestate lifecycle, pending-token state, backend work conversion, journaling, metric mailbox. |
| `recipe-native-executor` | finalized artifacts, build recipes, task ABI, slots, arenas, reservations, init images, values | CUDA Driver and ROCr/HSA resources, native module loading, native queues/events, warmup, bridges. |
| `recipe-host` | Draft/Finalized tasks, init images, values, endpoints, reservations, arena layouts | RAM/disk backing, host worker runtime, staged copies, disk file lifecycle. |
| `recipe-transport` and `recipe-remote` | digests, machine/profile identities, finalized tasks and resolved endpoints | Framed TCP, protocol sequencing, channel limits, master/worker ownership and cross-machine transfers. |

The executor boundary is intentionally closed. `recipe_executor::BackendWork`
has only init admission, calculation, internal transfer, metric, and exit
transfer variants. There is no core or executor task for discovery,
compilation, allocation, module loading, or topology mutation. Those activities
are complete before `FinalizedBundle` reaches a running backend.

## Invariants to preserve when changing core

1. Keep the calculation model reduced to GPU calculations plus transfers.
   Dependencies, queues, synchronization, routes, phases, and metrics order or
   realize those two kinds of work; they are not additional model semantics.
2. Keep all payload arithmetic in the explicit F32/I32 scalar domain and retain
   checked fault behavior through the preallocated int32 flag and mandatory
   readback contracts.
3. Preserve the immutable `Init -> Loop -> Exit` lifecycle. Init admission and
   exit egress are transfers, not hidden host setup or teardown tasks.
4. Do not let estimated topology properties reach scheduling. Add a measured
   or explicit override provenance at the source instead of a fallback in a
   consumer.
5. Keep Draft target-independent where the contract says so. Target, ABI,
   entry symbol, and toolchain identity belong to realized artifacts, while
   build provenance must prove the exact deferred stage.
6. Treat aliases, value lifetimes, lane claims, queue/completion slots, image
   packing, and arena offsets as independent validated contracts. Do not infer
   one from another in an executor.
7. Keep identities stable and explicit. If a field changes the semantic graph,
   placement, resource contract, or realized image, the producer's digest
   domain and every identity comparison must change with it.
8. Extend validation at the owning stage and aggregate a precise
   `ValidationCode`; do not add recovery branches that hide an invalid state.
9. Leave scheduling policy in `recipe-scheduler`, realization policy in
   `recipe-prepare`, and native behavior in the backend crates. Core should
   model and validate a new contract only when an observed stage boundary
   requires it.

## Source map

| File | Key symbols and reading order |
| --- | --- |
| `core/src/lib.rs` | Crate contract, module declarations, public re-exports. |
| `core/src/identity.rs` | `Label`, `Digest`, and six digest identity wrappers. |
| `core/src/ids.rs` | Stable ID macro and all typed ID names. |
| `core/src/units.rs` | Typed quantities, checked arithmetic, and ceil timing equations. |
| `core/src/error.rs` | `ValidationCode`, `ValidationError(s)`, `ValidationResult`, `Validator`. |
| `core/src/topology.rs` | Property provenance, machine/node/device/link model, topology and route validation. |
| `core/src/discovery.rs` | Device/link capabilities, measured snapshot validation, device lookup. |
| `core/src/scalar.rs` | Scalar type/opcode contracts, SSA validation, index spaces, affine accesses, alias matrix. |
| `core/src/artifact.rs` | Deferred stage build contracts and realized artifact identity/resource checks. |
| `core/src/schedule.rs` | Lifecycle, loop activation, tasks, endpoints, lane claims, slots, images, and arena declarations. |
| `core/src/plan.rs` | Reservations, capacity, Draft validation, Realization validation, Finalize, layout/value resolution. |

For a complete execution trace, read `recipe-planner::lower_candidate`, then
`recipe-scheduler::schedule` and `pack_arenas`, then
`recipe-prepare::Preparer::prepare_program`, and finally
`recipe-core::FinalizedBundle::finalize_with_loop_schedule`. The native and
remote executors should be read only after that sequence, because they consume
the closed contracts produced by it rather than constructing their own state.

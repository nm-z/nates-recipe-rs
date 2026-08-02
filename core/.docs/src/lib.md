# `recipe-core` facade

`core/src/lib.rs` is the public declaration facade for the `recipe-core` crate.
The crate is intentionally dependency-free. It contains the shared,
backend-neutral records that cross Recipe's static pipeline, together with the
validators that prove those records are internally consistent. It does not
probe hardware, choose a schedule, compile a kernel, load a driver artifact,
allocate a runtime resource, transfer file data, or execute a task.

The source-level crate contract is visible at the top of `core/src/lib.rs`:

```rust
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
```

Every public value therefore has a `Debug` implementation and the crate has no
unsafe or driver dependency. `core/Cargo.toml` has an empty dependency table.
The crate models the values exchanged by discovery, Draft, Realize, and
Finalize, and validates the invariants that connect those stages.

## What `lib.rs` exposes

The facade declares ten public modules in this order:

```text
artifact, discovery, error, identity, ids, plan, scalar, schedule, topology, units
```

It then flattens their public items at the crate root:

```rust
pub use artifact::*;
pub use discovery::*;
pub use error::{ValidationCode, ValidationError, ValidationErrors, ValidationResult};
pub use identity::{
    BundleIdentity, CandidateIdentity, Digest, DiscoveryIdentity, DraftIdentity,
    Label, LabelError, RealizationIdentity, TopologyIdentity,
};
pub use ids::*;
pub use plan::*;
pub use scalar::*;
pub use schedule::*;
pub use topology::*;
pub use units::*;
```

The root is the intended import surface used by the workspace, for example
`recipe_core::Topology`, `recipe_core::ScalarProgram`, and
`recipe_core::FinalizedBundle`. `Validator` and its mutating methods remain
crate-private. The error module deliberately reexports only the public
diagnostic types. The identity module reexports the six stage identity
wrappers, `Digest`, and `Label`; the typed numeric IDs are all reexported by
`ids::*`.

The wildcard reexports are not alternate implementations. Each type has one
owner, the module where it is declared, and the root merely gives consumers a
uniform path.

## Ownership and stage boundaries

The records in this crate form a one-way static pipeline:

1. A measured `Topology` describes machines, nodes, storage domains, and
   directed links. A `DiscoveryProfile` records the capabilities actually
   available for those exact topology objects.
2. `recipe-language`, `recipe-primitives`, `recipe-ops`, `recipe-math`, and
   `recipe-training` construct typed calculation graphs, scalar programs,
   primitive declarations, aliases, and loop domains. `recipe-primitives` and
   the planner turn those declarations into core kernel templates, while
   `recipe-program` supplies the static lifecycle envelope.
3. `recipe-planner` chooses placements and emits an offset-free `DraftPlan`.
   The Draft owns values, kernel templates, prebuilt artifact identities or
   target-independent `ArtifactBuildRecipe` records, tasks, submission
   resources, logical arena objects, value bindings, aliases, init images, and
   exit releases. It does not contain physical arena offsets for those objects.
4. `recipe-kernel` and `recipe-prepare` realize deferred artifacts against the
   current measured targets. `RealizationProfile` is keyed to the unchanged
   Draft and carries target-specific artifacts, reservation evidence, and
   measured post-warm capacity.
5. `FinalizedBundle::finalize`,
   `FinalizedBundle::finalize_with_loop_iterations`, or
   `FinalizedBundle::finalize_with_loop_schedule` combines the unchanged Draft,
   validated Realization, finalized arena layouts, and loop activation domains.
   Finalize resolves every value to an immutable arena location and returns a
   bundle whose fields are private and cannot be mutated into a different
   contract.
6. `recipe-executor`, `recipe-host`, `recipe-native-executor`, and
   `recipe-remote` consume the immutable bundle. They translate its closed task
   records into backend work, host byte copies, native submissions, or worker
   projections. Compilation, allocation, discovery, and topology mutation are
   outside the running boundary.

The core crate owns the data contracts and their cross-stage checks. The
surrounding crates own the algorithms that produce or consume those contracts:
probing owns measurements, scheduling owns timing and lane selection, planning
owns placement and Draft construction, preparation owns the fixed-point
realization loop, and backends own native resources and execution.

Core also owns no representation codec. `recipe-probe` encodes measured
profiles, `recipe-language` and `recipe-program` encode OGDL-derived graph and
loop declarations, `recipe-ingest` frames external data images, and
`recipe-remote`/`recipe-transport` encode authenticated wire manifests. Those
formats carry core IDs and identities but cannot change their validation rules.

## Module map

| Module | Owned surface | Boundary it enforces |
| --- | --- | --- |
| `artifact` | Target/toolchain identities, resource envelopes, deferred build recipes, and realized artifact identities | A deferred stage build is target-independent until Realize, and a realized image proves its digest, toolchain, resources, and optional deferred-build provenance. |
| `discovery` | Calculation and transfer capabilities plus `DiscoveryProfile` | Capabilities must cover every topology object, be available, asynchronous where required, and be measured or explicitly overridden before scheduling. |
| `error` | `ValidationCode`, `ValidationError`, `ValidationErrors`, and `ValidationResult` | Validators report all discovered contract failures with a machine-readable code, field path, and message. |
| `identity` | Nonempty `Label`, 256-bit `Digest`, and stage identity newtypes | Topology, discovery, Draft, candidate, realization, and bundle identities cannot be confused by type. |
| `ids` | Stable typed `u64` IDs for machines, devices, values, kernels, tasks, resources, artifacts, runs, and metrics | References carry their domain in the Rust type. Zero is representable for deterministic construction, then rejected by the contract that reserves it. |
| `plan` | Reservation and capacity ledgers, `DraftPlan`, `RealizationProfile`, `FinalizedBundle`, and their validators | The static lifecycle is identity-bound, resource-safe, and final only after layout and value-location validation. |
| `scalar` | `DType`, scalar literals/opcodes/SSA programs, index spaces, affine buffer views, kernel templates, and alias rules | Calculation payloads are limited to f32 and int32, typed and acyclic, with checked memory views and a complete alias matrix. |
| `schedule` | Lifecycle phases, loop iteration domains, task kinds, transfer lanes, metrics, submission slots, resource manifests, and arena records | The task graph is immutable and phase-aware; Draft stores logical bindings and lifetimes, while Finalize resolves physical addresses. |
| `topology` | Machines, node ownership, storage devices, transport kinds, duplex resources, links, provenance, routes, and lookups | Every device has one owner, exactly one master exists, links form valid reverse pairs, and route endpoints and duplex capacity are coherent. |
| `units` | Checked byte, offset, rate, FLOP, time, and element-count units plus timing equations | Domain arithmetic cannot silently overflow, and zero rates, transfer lanes, or index-space elements are rejected at construction. |

The following sections describe each public surface in enough detail to make
ownership and use-site expectations explicit.

## Exact public item inventory

The facade currently exports the following names. This list is intentionally
spelled out because the root wildcard reexports make a module-qualified
rustdoc search less obvious to callers.

### Artifact and discovery

```text
TargetIdentity
ToolchainIdentity
KernelResourceBounds
ArtifactBuildProvenance
ArtifactBuildAccess
ArtifactBuildView
ArtifactBuildBinding
ArtifactDispatchGeometry
ArtifactWorkBounds
ArtifactBuildRecipe
ArtifactIdentity

CalculationCapability
TransferCapability
DiscoveredDevice
DiscoveredLink
DiscoveryProfile
```

`ArtifactBuildAccess::{reads, writes}`,
`ArtifactBuildRecipe::validate`, `ArtifactIdentity::validate`,
`DiscoveryProfile::{validate, device}`, and
`ArtifactBuildRecipe`/`ArtifactIdentity` field access are the public entry
points in these modules. There are no public artifact or discovery factories;
the producing crates assemble the records and then call their validators.

### Identity, IDs, units, and topology

```text
Label, LabelError, Digest
TopologyIdentity, DiscoveryIdentity, DraftIdentity,
CandidateIdentity, RealizationIdentity, BundleIdentity

MachineId, NodeId, DeviceId, LinkId, TransportId, DuplexResourceId,
ValueId, ScalarValueId, KernelTemplateId, KernelInputId, KernelOutputId,
TaskId, ArtifactId, QueueSlotId, CompletionSlotId, MetricId, MetricSlotId,
ArenaObjectId, RunId

ByteCount, ByteOffset, BytesPerSecond, TransferLaneCount, FlopCount,
FlopsPerSecond, Nanoseconds, ElementCount, UnitError
transfer_time_ceil, calculation_time_ceil

PropertyProvenance, Property, DeviceKind, Machine, NodeRole, Node, Device,
TransportKind, DuplexMode, DirectedLink, Topology
```

`Topology` additionally exposes `validate`,
`validate_scheduling_properties`, `machine`, `device`, `link`,
`duplex_mode`, and `validate_route`. The typed unit methods are listed in the
`units` section below; typed IDs expose only `new` and `get` plus formatting and
trait implementations, so they cannot acquire domain logic accidentally.

### Scalar and schedule records

```text
DType, ScalarLiteral, ScalarOpcode
ScalarInput, ScalarConstant, ScalarInstruction, ScalarProgram
IndexSpace, StaticBufferAccess, KernelInput, KernelOutput
AliasPermission, AliasRule, KernelTemplate

RunPhase, LoopIterations, LoopIteration, IterationDomain,
LoopTaskDomain, LoopSchedule, ScheduleWindow, SubmissionSlots
ValueSpec, ValueBinding, ValueAliasContract
InitDataImageMember, InitDataImage, ResolvedValueLocation
CalculationTask, TransferEndpoint, TransferLaneClaim, TransferTask
ResolvedTransferEndpoint, ResolvedTransferEndpoints
MetricPurpose, MetricTask, TaskKind, Task
QueueSlot, CompletionSlot, MetricSlot, DeviceBytes, ResourceManifest
ArenaObject, ArenaRelease, ArenaAllocation, ArenaLayout
```

### Plan and validation records

```text
EXACT_USER_RESERVATION
ReservationMechanism, ReservationEvidence, ReservationEntry, ReservationLedger
CapacityLedgerEntry, CapacityLedger
DraftPlan, RealizationProfile, FinalizedBundle

ValidationCode, ValidationError, ValidationErrors, ValidationResult
```

`FinalizedBundle` is the only core record in this inventory whose fields are
private. Its accessor list is part of the read-only runtime contract and is
documented in the plan section.

### Scalar opcode domain

The `ScalarOpcode` enum is closed in the current source except for the
`#[non_exhaustive]` marker. Its variants are:

```text
Add, Subtract, Multiply, Divide, Remainder, Negate, Absolute,
Minimum, Maximum, Fma,
Equal, NotEqual, LessThan, LessThanOrEqual, GreaterThan, GreaterThanOrEqual,
Select, BitAnd, BitOr, BitXor, BitNot,
BitcastF32ToI32, BitcastI32ToF32,
ShiftLeft, ShiftRightLogical, ShiftRightArithmetic, Require,
IsFinite, IsNan, SquareRoot, Floor, Ceiling, RoundNearestEven,
ConvertF32ToI32, ConvertI32ToF32
```

`Divide` and `Remainder` use IEEE behavior for f32 and checked truncating
behavior for int32. `Require` normalizes an int32 truth value and publishes a
failure through the preallocated fault flag when it is zero. Comparisons
produce int32 truth values, `Select` uses an int32 condition, and the bitcasts
preserve the exact binary32 bit pattern. These semantics are part of the
backend-neutral contract, not a backend implementation detail.

## `artifact`: deferred and realized kernel contracts

`TargetIdentity` names a discovered calculation target by backend,
architecture, and ABI. `ToolchainIdentity` records the exact toolchain name,
version, and nonzero digest that produced an image. `KernelResourceBounds`
retains private bytes per lane, shared bytes per workgroup, scratch bytes per
dispatch, and the drafted maximum workgroup size.

`ArtifactBuildRecipe` is the target-independent request passed from Draft to
Realize. It reserves an `ArtifactId`, a collision-checked stage-scoped
`KernelTemplateId`, the source kernel identity, `ArtifactBuildProvenance`, an
ordered list of `ArtifactBuildBinding` records, fixed dispatch geometry,
operation bounds, an optional fault-flag value, and the resource envelope. A
recipe deliberately has no target, format, entry symbol, or toolchain. Its
bindings contain typed values and complete affine views. `ArtifactBuildAccess`
has `Read`, `Write`, `ReadWrite`, and `ReadWriteAtomic`; `reads()` and `writes()`
are the canonical predicates used to derive task input and output lists.

`ArtifactBuildRecipe::validate` checks nonzero reserved IDs and provenance
digests, nonempty launch geometry, exact ceiling division for workgroups, the
drafted workgroup/resource agreement, unique value bindings, equal extent and
stride ranks, and the exact four-byte int32 atomic binding required for an
optional fault flag.

`ArtifactIdentity` is the complete realized image manifest: ID, nonzero image
digest, format, target, toolchain, entry symbol, stage template, resource
envelope, and optional `ArtifactBuildProvenance`. Its validator checks the
digest/toolchain/resource minima and, when present, the deferred-build
digests. `recipe-kernel` supplies the LLVM/native lowering and
`recipe-prepare` supplies the target-specific identity. Core only validates the
manifest and its relationship to the Draft.

## `discovery`: capability snapshots bound to topology

`CalculationCapability` describes one GPU target, measured FLOP rate,
asynchronous calculation submission, maximum concurrent calculations, native
subgroup width, maximum workgroup width, and shared-memory capacity.
`TransferCapability` describes measured byte rate, transfer-lane capacity,
asynchronous submission, and whether transfers overlap calculation.
`DiscoveredDevice` adds availability, submission-queue capacity, total storage
capacity, and optional calculation capability. `DiscoveredLink` records
availability, measured directional bandwidth and lane capacity, and
asynchronous submission.

`DiscoveryProfile` is an immutable snapshot with its own nonzero
`DiscoveryIdentity` and the `TopologyIdentity` it describes. Its
`validate(&Topology)` method requires:

- exactly one discovered entry for every topology device and link, with no
  unknown IDs;
- availability, nonzero queue capacity, asynchronous transfer submission, and
  schedulable capacity/rate/lane properties;
- a calculation capability for every `GpuMemory` device, with schedulable rate,
  asynchronous submission, nonzero concurrency, power-of-two subgroup width,
  a workgroup limit containing at least one subgroup, and nonzero shared memory;
- no calculation capability on `Ram` or `Disk`; and
- link bandwidth and lane counts exactly equal to the corresponding topology
  direction.

`device(DeviceId)` is the read-only lookup used by planners, schedulers,
realizers, and validators. `recipe-probe` constructs and serializes these
records after bounded hardware measurement. `recipe-cluster` assembles them
from authenticated per-machine profiles. Core never discovers or measures a
device itself.

## `topology`: physical ownership and route contract

`Property<T>` pairs a value with `PropertyProvenance::{Estimated, Measured,
Override}`. Only measured and explicitly overridden values satisfy
`is_schedulable()`. Estimates may size a probe but cannot reach production
scheduling.

`DeviceKind` is `GpuMemory`, `Ram`, or `Disk`. A `Device` always has a capacity
and transfer rate, and only a GPU may have `calculation_rate`.
`Machine`, `Node`, and `NodeRole::{Master, Worker}` describe ownership. Every
configured machine must own at least one device, every device must be owned by
one node on its machine, and the topology must contain exactly one master.

`TransportKind` identifies the physical family (`Memory`, `Pcie`, `Sata`,
`Sas`, `Nvme`, `Ethernet`, or `Wlan`). `required_duplex()` maps SATA and WLAN
to half-duplex and the other families to full-duplex. `DirectedLink` is one
direction of a reverse-paired transport. Both directions share a
`TransportId`; half-duplex directions share one `DuplexResourceId`, while
full-duplex directions have distinct capacity resources. Lane counts are
directional.

`Topology::validate()` checks identity, unique machine/device/node/link IDs and
names, machine and node references, device kind rules, ownership, exactly two
reverse directions per transport, symmetric transport kind and duplex mode,
duplex resource rules, and resource ownership. Separate
`validate_scheduling_properties()` rejects every estimated capacity, rate, or
lane value before Draft scheduling.

The read-only `machine`, `device`, and `link` lookups are used throughout the
workspace. `duplex_mode(first, second)` recognizes a valid reverse pair and
returns its duplex mode. `validate_route(source, destination, route)` accepts
an empty route immediately for a same-device copy; otherwise a route must be
nonempty, use known links in contiguous order, and end at the requested
destination. `recipe-probe` is the producer of measured topology records,
`recipe-scheduler` consumes route and duplex facts, and the executor layers
must treat the topology as immutable after Finalize.

## `identity` and `ids`: stable names and typed references

`Label::new` rejects empty or all-whitespace strings and otherwise preserves the
caller-provided text. `LabelError` is a local constructor error. `Digest` is a
caller-computed 32-byte content or manifest digest with `ZERO`, `new`, `bytes`,
and `is_zero`; its `Debug` output intentionally shows only a short prefix.
The six identity wrappers are `TopologyIdentity`, `DiscoveryIdentity`,
`DraftIdentity`, `CandidateIdentity`, `RealizationIdentity`, and
`BundleIdentity`. Each wraps a `Digest`, exposes `new`, `digest`, and
`is_zero`, and prevents a topology identity from being passed where a Draft or
bundle identity is required. Digests are caller-computed opaque values: core
checks zero-ness and equality at stage boundaries, but does not choose or
recompute a hash algorithm.

The `ids` macro defines `MachineId`, `NodeId`, `DeviceId`, `LinkId`,
`TransportId`, `DuplexResourceId`, `ValueId`, `ScalarValueId`,
`KernelTemplateId`, `KernelInputId`, `KernelOutputId`, `TaskId`, `ArtifactId`,
`QueueSlotId`, `CompletionSlotId`, `MetricId`, `MetricSlotId`,
`ArenaObjectId`, and `RunId`. Each has `new`, `get`, `Display`, and the normal
copy/order/hash/debug traits. Constructors do not perform global uniqueness
or zero checks. The owning validator performs those checks in context, which
allows deterministic ID allocators in the planner and probe while keeping the
core types opaque to accidental cross-domain use.

## `scalar`: calculation payload and kernel template

The payload domain is intentionally closed to `DType::{F32, I32}`; both have a
four-byte representation. `ScalarLiteral` preserves f32 bits exactly through
`F32Bits(u32)` and carries signed `I32` values. `ScalarOpcode` covers arithmetic,
comparisons, selection, bit operations and casts, finite/NaN predicates,
rounding, square root, and checked `Require`. Its `arity`, `result_dtype`, and
`flops` functions are the single signature and scheduling-cost source. `Fma`
counts as two FLOPs, comparisons that produce a result count as one, and
addressing, validation predicates, representation changes, and bit operations
do not count as FLOPs.

`ScalarProgram` is an ordered, acyclic scalar SSA record of
`ScalarInput`s, `ScalarConstant`s, `ScalarInstruction`s, and nonempty outputs.
`validate()` rejects duplicate definitions, use before definition, wrong
arity, wrong result types, unknown outputs, and missing outputs. `dtype_of` is a
read-only lookup. `requires_fault_flag()` identifies checked int32 divide,
remainder, negate, absolute, and `Require` instructions that need the
preallocated device fault channel.

`IndexSpace::new` accepts a nonempty list of nonzero `ElementCount`s and checks
the product. `StaticBufferAccess` describes an offset, per-axis element
strides, and complete backing storage. `linear` and `contiguous` construct
canonical mappings. Validation checks rank, checked address arithmetic,
backing-size bounds, and injectivity for writable mappings. Zero strides are
valid only for read-only broadcast dimensions.

`KernelInput` and `KernelOutput` pair typed IDs with these views.
`AliasPermission::{Forbidden, MayAliasExact, MustAliasExact}` and `AliasRule`
form the complete input/output alias matrix. `KernelTemplate` combines an ID,
index space, ordered inputs and outputs, one scalar program, and every alias
rule. Its validator composes scalar validation, checks unique IDs and view
contracts, matches kernel and scalar arities/types, and requires exactly one
alias rule for every input/output pair. `recipe-language`, `recipe-math`,
`recipe-primitives`, `recipe-ops`, and `recipe-training` are the producers of
these records; `recipe-kernel` is the lowering consumer.

## `schedule`: immutable lifecycle records

`RunPhase::{Init, Loop, Exit}` is the only lifecycle phase domain. Init admits
external data images, Loop contains calculation and metric work, and Exit
egresses results and releases arenas. `LoopIterations` is either a nonzero
finite count or `Unbounded`; `LoopIteration` is a zero-based observation of one
execution. `IterationDomain` is a nonempty, zero-based, half-open arithmetic
progression with a nonzero stride. `every`, `first`, and `periodic` construct
canonical domains; `contains` and `is_within` are used by Finalize and the
executor without unrolling the graph. `LoopTaskDomain` attaches exactly one
domain to each loop task, and `LoopSchedule` is the complete sidecar supplied
to Finalize.

`ScheduleWindow` is a half-open nanosecond interval. `SubmissionSlots` pairs a
queue and completion slot. `ValueSpec` identifies a typed device-resident
payload and optional producer. `ValueBinding` is the Draft-time object-relative
placement; it deliberately does not contain a physical arena offset.
`ValueAliasContract` records the selected storage relationship for one source
alias pair.

`InitDataImageMember` and `InitDataImage` describe one packed external input
image per device, including logical input IDs, physical resident copies, typed
sizes, and image-relative offsets. `ResolvedValueLocation` is produced only by
Finalize and carries the resulting arena offset.

The three task payloads are:

- `CalculationTask`, a loop GPU calculation with a kernel template, artifact,
  ordered input/output values, optional int32 fault flag, FLOP work, and
  submission slots;
- `TransferTask`, a one-hop internal copy or a logical external admission or
  egress, with byte size, route, exact lane claims, and submission slots; and
- `MetricTask`, an asynchronous four-byte readback with `MetricPurpose::User`
  or `FaultReadback`, a metric and preallocated slot, and submission slots.

`TaskKind` and `Task` add the ID, phase, schedule window, and dependency list.
`TransferEndpoint` distinguishes `External` from a device/value endpoint;
`ResolvedTransferEndpoint(s)` replace the latter with finalized arena
locations. `TransferLaneClaim` is either one directed-link lane or one
external device lane. Internal paths are executor-visible one-hop transfers;
longer routes are dependency-chained tasks with intermediate resident values.

`QueueSlot`, `CompletionSlot`, `MetricSlot`, and `DeviceBytes` make resource
choices explicit. `ResourceManifest::validate` checks unique slots and known
device references and exposes lookup helpers. `ArenaObject` is a logical,
aligned, lifetime-bounded object; `ArenaRelease` names its device at Exit;
`ArenaAllocation` and `ArenaLayout` are the physical per-device layout supplied
to Finalize. The scheduler chooses windows, lanes, and layouts. Core validates
their relationship but never allocates the memory.

## `plan`: reservation, Draft, Realize, and Finalize

`EXACT_USER_RESERVATION` is the single one-billion-byte reservation constant.
`ReservationMechanism` distinguishes a held allocation from an enforced quota.
`ReservationEvidence` is either `NonGpu` or `GpuDisplay { enabled_connectors }`;
one billion bytes is required for non-GPU devices and displayed GPUs, while a
GPU with zero enabled display connectors has no display reservation. A
`ReservationEntry` carries the device, validated label, exact bytes, mechanism,
and evidence. `ReservationLedger::validate` requires one entry for every
topology device, matching evidence kind, no duplicates, and the exact evidence
derived byte count.

`CapacityLedgerEntry` records total capacity and the measured or overridden
runtime overhead, fragmentation, safety headroom, and Recipe-usable capacity.
`CapacityLedger::validate` requires one entry per device, schedulable
provenance for every field, and checked accounting in which the reservation
plus all four components does not exceed total capacity.

`DraftPlan` is the complete offset-free candidate:

```text
identity, candidate, discovery, topology
values, kernels
artifacts, artifact_builds
tasks, resources
arena_objects, value_bindings, value_aliases
init_images, releases
```

`DraftPlan::validate(topology, discovery)` first composes topology structural
and scheduling-property validation, discovery validation, identity checks, and
resource-manifest checks. It then validates unique values, devices and typed
byte sizes, every kernel template, the mutually exclusive realized/deferred
artifact catalog, deferred binding references, and all task relationships.
Task validation enforces phase ownership, GPU calculation placement, artifact
and target matching, value residency and typed byte sizes, exact checked-fault
bindings, external admission/egress phase rules, one-hop routes, transfer lane
claims, queue/completion ownership, metric slots, direct fault readbacks,
dependency phase and schedule order, acyclicity, resource contention, measured
concurrency limits, and transfer/calculation overlap capability.

The same validation pass checks arena-object alignment and lifetimes, exactly
one binding for every value, producer and consumer lifetime containment,
alias permissions both at the Draft value-contract level and at each kernel
invocation, one packed init image and one admission per device, nonoverlapping
image members with correct physical bindings, and exactly one Exit release per
device. Failures are accumulated rather than replaced by the first error.

`RealizationProfile` is the stable output of Realize and carries its own
identity plus the Draft, candidate, discovery, and topology identities it was
derived from. It owns the realized artifact list, unchanged resource manifest,
reservation ledger, and post-warm capacity ledger. Its validator requires all
stage identities to match, every prebuilt artifact identity to remain equal,
every deferred artifact to be present with matching stage/provenance/
resource data and discovered target, and valid reservations and capacity.

`FinalizedBundle` has private fields so callers cannot construct a partially
resolved runtime contract. The three constructors are the only public creation
paths:

- `finalize` applies the default one-iteration loop;
- `finalize_with_loop_iterations` assigns `IterationDomain::every` to every
  loop task; and
- `finalize_with_loop_schedule` accepts one exact domain for every loop task.

Finalize validates the bundle identity, the complete Draft and Realization,
all loop-domain assignments and domain matching for internal transfers and
fault readbacks, every per-device arena layout, and every resolved value
location. It then stores the immutable identities, artifacts, deferred builds,
kernels, tasks, resources, reservations, layouts, locations, aliases, and init
images. Accessors expose read-only slices and lookups for identities, artifacts,
kernels, tasks, resource and reservation manifests, layouts, value locations,
aliases, init images, and resolved transfer endpoints. `iteration_domain`,
`init_image`, `value_location`, and `transfer_endpoints` are intentionally
lookup-only and return `Option` when the requested ID is not part of the
finalized contract.

The validation call tree is deliberately compositional:

```text
FinalizedBundle::finalize_with_loop_schedule
  -> DraftPlan::validate
     -> Topology::validate
     -> Topology::validate_scheduling_properties
     -> DiscoveryProfile::validate
     -> ResourceManifest::validate
     -> KernelTemplate / ArtifactBuildRecipe / ArtifactIdentity validation
     -> task, dependency, route, lane, fault, contention, arena,
        binding, alias, init-image, and release checks
  -> RealizationProfile::validate
     -> realized-artifact, resource, reservation, and capacity checks
  -> loop-domain checks
  -> finalized arena-layout checks
  -> resolved value-location checks
  -> construct the private FinalizedBundle only if no error was accumulated
```

This order matters. A schedule consumer can rely on the stronger implication
that a `FinalizedBundle` has already passed all lower-level checks, while a
planner or serializer must call the relevant object validator before exposing
an intermediate record. No validator silently repairs, defaults, or substitutes
an invalid child object.

## `units`: checked domain arithmetic

The unit newtypes prevent raw integer values from crossing domain boundaries:

- `ByteCount` is an exact decimal byte size with checked add, subtract,
  multiply, and power-of-two `checked_align_up`.
- `ByteOffset` is an arena offset with checked end calculation.
- `BytesPerSecond` and `FlopsPerSecond` are nonzero rates.
- `TransferLaneCount` and `ElementCount` are nonzero capacities.
- `FlopCount` is an exact operation count with checked add and multiply.
- `Nanoseconds` is the integer schedule-time unit with checked addition.

`UnitError` reports overflow, zero rates, zero lanes, zero element counts, and
invalid alignment. `transfer_time_ceil` and `calculation_time_ceil` use checked
128-bit intermediate arithmetic and return a ceiling duration in nanoseconds;
the rate units guarantee a nonzero denominator. These functions are used by
`recipe-scheduler` and the root training path, while the core crate remains
agnostic about any particular backend clock or driver.

## Failure organization

Core has two deliberate failure layers.

### Local construction failures

Small constructors reject facts that can be proven without surrounding state:
`Label::new` returns `LabelError`; nonzero rates, transfer lanes, and element
counts return `UnitError`; byte/time arithmetic returns `Option` or
`UnitError`; `LoopIterations::new`, `IterationDomain::new`, and
`IterationDomain::periodic` return `Option` for invalid zero or empty domains;
and `IndexSpace::new` returns `UnitError` for an empty or overflowing product.
These APIs do not invent defaults.

### Contract validation failures

Every cross-object validator returns `ValidationResult<T>`, an alias for
`Result<T, ValidationErrors>`. A `ValidationError` contains exactly a
`ValidationCode`, a structured field path, and a human-readable message.
`ValidationErrors` exposes the ordered slice, consuming `into_vec`, and a
`contains` query for a category, and formats multiple errors as a semicolon-
separated diagnostic. `ValidationCode` is `#[non_exhaustive]`, so external
matches must include a wildcard.

The codes are grouped by the contracts that own them:

- identity and reference shape: `EmptyName`, `InvalidIdentity`,
  `DuplicateId`, `DuplicateName`, `UnknownReference`, `WrongKind`;
- topology ownership and transport: `MissingMaster`, `MultipleMasters`,
  `UnownedDevice`, `DeviceOwnedMultipleTimes`, `MachineMismatch`,
  `InvalidRoute`, `InvalidLaneClaim`, `InvalidTransport`, `InvalidDuplex`,
  `RouteEndpointMismatch`;
- capability evidence: `MissingRequiredObject`,
  `UnavailableRequiredObject`, `UnsupportedCapability`, `UnmeasuredProperty`;
- scalar and kernel programs: `MissingAliasRule`, `DuplicateAliasRule`,
  `UnknownScalarValue`, `ScalarUseBeforeDefinition`, `ScalarArity`,
  `ScalarTypeMismatch`, `DuplicateScalarValue`, `MissingScalarOutput`,
  `InvalidMemoryAccess`, `AddressOverflow`;
- lifecycle and task graph: `DependencyCycle`, `InvalidPhase`,
  `DependencyPhaseOrder`, `DependencyScheduleOrder`,
  `InvalidCalculationPlacement`, `InvalidIterationDomain`,
  `InvalidFaultReadback`, `InvalidExternalTransfer`, `InvalidDataImage`,
  `MissingUpload`, `DuplicateUpload`, `MissingRelease`, `DuplicateRelease`;
- capacity, identity preservation, and allocation: `DuplicateReservation`,
  `MissingReservation`, `WrongReservationSize`, `InvalidReservationName`,
  `CapacityOverflow`, `InsufficientCapacity`, `IdentityMismatch`,
  `ArtifactMismatch`, `ResourceMismatch`, `ResourceContention`,
  `CapabilityConcurrencyExceeded`, `DuplicateAllocation`,
  `MissingAllocation`, `AllocationOutOfBounds`, `AllocationMisaligned`,
  `DuplicateValueBinding`, `MissingValueBinding`, `ValueBindingOutOfBounds`,
  `ValueBindingMisaligned`, `ValueLifetimeMismatch`, `AliasViolation`,
  `LiveAllocationOverlap`, and `InvalidLifetime`.

`Validator` is the crate-private accumulator used by every module. Composite
validators append child errors with a field prefix, so a caller can identify
the exact object and field that failed. This is why `DraftPlan::validate` can
report topology, discovery, scalar, task, memory, and identity failures in one
pass without adding a second error taxonomy in planner or executor crates.
Those outer crates translate core validation text into their own domain error
types at their public boundaries, but they do not replace the core contract.

## Workspace consumption map

`recipe-core` is a direct dependency of the root `recipe` crate and these
workspace crates: `recipe-cluster`, `recipe-executor`, `recipe-host`,
`recipe-ingest`, `recipe-kernel`, `recipe-language`, `recipe-math`,
`recipe-native-executor`, `recipe-native-probe`, `recipe-ops`,
`recipe-planner`, `recipe-prepare`, `recipe-primitives`, `recipe-program`,
`recipe-probe`, `recipe-remote`, `recipe-scheduler`, `recipe-training`, and
`recipe-transport`. Their use is intentionally directional:

| Consumers | How they use the facade |
| --- | --- |
| `recipe-probe`, `recipe-native-probe` | `recipe-probe` constructs measured `Machine`, `Node`, `Device`, `DirectedLink`, `Topology`, and `DiscoveryProfile` records. `recipe-native-probe` supplies GPU descriptors, measurements, target/toolchain identities, and native driver evidence consumed by that assembly; neither changes core semantics. |
| `recipe-cluster` | Assemble per-machine profiles into one topology and discovery identity. It consumes duplex, transport, label, digest, and measured-property types and verifies network evidence before calling the core-shaped records complete. |
| `recipe-transport` | Carry authenticated `Digest` identities and measured `ByteCount`, `BytesPerSecond`, and `Property` values for peer benchmarking. It owns the connected channel and framing, not the topology contract. |
| `recipe-language` | Build and serialize calculation graphs from `ValueId`, `KernelTemplateId`, `DType`, `ByteCount`, `ByteOffset`, `ScalarProgram`, and alias permissions. It owns `PrimitiveKernel` and `PrimitiveAliasRule`; `recipe-primitives` and the planner construct core `KernelTemplate` values. |
| `recipe-math` | Generate finite numerical algorithms as `ScalarProgram` values using the core opcode/type domain and checked `Require` fault semantics. |
| `recipe-primitives` | Lower primitive declarations into core `KernelTemplate`, `ScalarProgram`, `StaticBufferAccess`, alias permissions, typed IDs, and FLOP/byte bounds. |
| `recipe-ops`, `recipe-training` | Emit operation, inference, and training graph declarations, scalar programs, alias permissions, typed IDs, init-image metadata, and loop domains for primitive lowering and planning. These crates own semantic lowering, not scheduling or execution. |
| `recipe-ingest` | Convert external datasets and model files to core `DType`, `ByteCount`, `ValueId`, `InitDataImage`, and content `Digest` records. It owns parsing and bounded snapshots, not device calculation. |
| `recipe-program` | Wrap a `CalculationGraph` in static `RunPhase` and `LoopIterations`/`IterationDomain` declarations, with explicit init and exit boundaries. |
| `recipe-scheduler` | Consume `Topology`, `DiscoveryProfile`, units, task kinds, and submission slots to compute deterministic windows and exact transfer lane claims; pack logical `ArenaObject`s into per-device `ArenaLayout`s. |
| `recipe-planner` | Select GPU placements, derive stable identities, construct `ValueSpec`, `Task`, `ResourceManifest`, `ArtifactIdentity` or `ArtifactBuildRecipe`, aliases, init images, releases, and the offset-free `DraftPlan`. |
| `recipe-kernel` | Read `KernelTemplate`, `ScalarProgram`, `StaticBufferAccess`, and deferred artifact recipes, then emit target LLVM/native images. It independently checks the recipe contract and returns target-specific lowering errors. |
| `recipe-prepare` | Own the fixed-point boundary: validate measured profiles and reservations, enumerate Draft candidates, realize deferred artifacts, warm maximum concurrency, collect stable `CapacityLedger`s, pack arenas, build `RealizationProfile`, and call `FinalizedBundle::finalize_with_loop_schedule`. |
| `recipe-executor` | Consume only immutable `FinalizedBundle` records and turn `TaskKind` values, resolved locations, loop domains, phases, metrics, and transfer endpoints into closed backend work. Its typestate lifecycle does not compile, allocate, or discover. |
| `recipe-host` | Use `ByteCount`, `DeviceId`, `TaskId`, and link routes for preallocated RAM/disk byte transport. It has no calculation payload or planning authority. |
| `recipe-native-executor` | Validate finalized artifact identities, arena layouts, submissions, task values, routes, metrics, and loop phases, then bind CUDA/HSA resources to the executor work boundary. It owns driver FFI and native execution, not core records. |
| `recipe-remote` | Project a `FinalizedBundle` onto a worker's machine/node ownership, serialize task and identity contracts, and enforce remote phase/resource checks over an already connected `recipe-transport` channel. |
| root `recipe` | Reexport `recipe_core` as `recipe::engine::core`, expose `ScalarProgram` through the operations facade, and use bundle/run identities and checked timing units in the public inference and training paths. |

`recipe-cuda`, `recipe-hsa`, `recipe-ogdl`, and `recipe-text` are workspace
members but do not declare a direct `recipe-core` dependency. CUDA/HSA own
driver-facing primitives, OGDL owns syntax, and text owns tokenizer/template
behavior; the crates that bridge those surfaces to Recipe's static contract
depend on core explicitly.

## Reading and changing this facade

When a new shared concept is needed, first identify the stage that owns its
invariants. Add the type to that module, add or extend the module validator,
and then add an intentional root reexport in `lib.rs`. Do not put scheduling,
driver, file-format, or backend policy in this crate merely because several
consumers currently mention the same ID. Conversely, do not duplicate a core
contract in a consumer when an existing validator or typed unit already owns
the rule.

The safe public path is therefore always:

```text
construct typed local values
  -> validate the owning object
  -> compose the next immutable stage
  -> validate cross-stage identities and resources
  -> finalize once
  -> consume read-only records in runtime crates
```

That path is the purpose of `core/src/lib.rs`: one flat, dependency-free,
backend-neutral vocabulary with one authoritative validation boundary.

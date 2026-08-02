# ID newtypes

`core/src/ids.rs` is the nominal identity boundary for the static Recipe
pipeline. It declares nineteen unrelated ID types with one macro:

```rust
pub struct Name(u64);
```

For every type, the inner field is private and the macro supplies
`Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, and `Hash`,
plus `const fn new(u64) -> Self`, `const fn get(self) -> u64`, and decimal
`Display`. There is no `Default`, `From<u64>`, `serde` implementation,
`repr` contract, or automatic wire format. `new` accepts every `u64`, including
zero and `u64::MAX`; the type itself does not know whether a value is valid,
unique, canonical, or in the right scope. Each statement about nonzero values,
allocation, ordering, or lifetime below is therefore a consumer contract at a
specific boundary.

The private field prevents accidental mixing of numeric identities. A
`DeviceId` cannot be passed where a `ValueId` is expected, even when both hold
the same number. `Ord` is ordinary numeric order, which is relied on by
`BTreeMap`, `BTreeSet`, binary search, deterministic tie-breaking, and
canonical encoders. `Display` emits only the decimal number, so diagnostics
must supply the surrounding field or type name when that context matters.

There is an unrelated `recipe_ogdl::NodeId`, a private `usize` graph index.
Source files that use both types alias the OGDL one as `OgdlNodeId`; it is not
the topology `recipe_core::NodeId` documented here.

## Where IDs are born and how they move

The pipeline has three identity families.

1. Probe and cluster construct topology IDs. A local measured profile starts
   its local machine, node, devices, links, transports, and duplex resources at
   one. Peer machines and worker nodes use the following positive ordinals.
   Cluster assembly allocates fresh positive IDs and remaps every member
   profile through typed `BTreeMap`s before adding inter-machine links.
2. Language, operation, and training compilers construct graph IDs. Source
   tensor and kernel IDs can come from a caller or Recipe OGDL. Training and
   inference graph compilers start `ValueId` and `KernelTemplateId` counters at
   one. Operation materializers receive caller-reserved, half-open value and
   kernel ranges (`IdentityNamespace`) and fail on overlap, exhaustion, or
   arithmetic overflow. Scalar builders start a separate one-based SSA
   counter for each scalar program.
3. Planner and runtime IDs are bundle-local. Planner `StableIdAllocator`s start
   task, value, and arena-object ranges at one, derive queue and completion
   slots from task IDs, derive metric slots from metric tasks, and derive a
   reserved artifact number from each collision-checked stage template. A
   `RunId` is a later execution epoch and deliberately does not participate in
   the immutable bundle identity.

The same numeric value can occur in different typed or lifetime namespaces.
For example, a planner normally gives a task, its queue slot, its completion
slot, a user metric slot, and a fault metric slot related numbers, and a
deferred stage currently gives its `ArtifactId` the same number as its
stage-scoped `KernelTemplateId`. Those equalities are deliberate derivations,
not conversions or permission to compare different types.

## Topology and discovery IDs

### `MachineId`

`MachineId` names a machine in one measured topology. It is the key of
`topology::Machine.id`, and the owner reference in every `Node.machine` and
`Device.machine`. Probe retains it in `MeasuredMachineOrigin`, where the ID is
paired with a machine fingerprint. `MeasuredProfile::resolve_local_inventory`
selects the current machine by exact fingerprint and then uses the retained
ID to associate every RAM, storage, and GPU origin. Native preparation checks
that the machine on native bindings is the resolved machine. Worker projection
uses it in `WorkerAssignment` and rejects an unknown machine or a node owned by
another machine.

The probe engine assigns the local machine `MachineId::new(1)` and assigns
peer machines from their deterministic peer enumeration using `index + 2`.
The same maps feed origins, topology, and discovery, so a device origin never
gets a different machine ID from the device it describes. Cluster assembly's
`IdAllocator` starts at one, remaps each member's machine IDs, and then adds
network edges; its checked increment reports `IdentityExhausted` instead of
wrapping.

`Topology::validate` requires every node and device to reference a known
machine, each configured machine to own at least one device, unique machine
IDs, and unique machine names. Profile validation requires exactly one machine
origin, unique stable fingerprints, and strictly increasing machine IDs in
origins and topology. A zero machine ID is not rejected by the ID type or by
the topology duplicate check itself; it becomes invalid only if a later
boundary imposes that policy. Machine IDs are encoded as little-endian `u64`
fields in the measured-profile codec and are included directly in
worker-projection hashes. The remote execution wire uses endpoint machine
digests instead of this topology ID.

### `NodeId`

`NodeId` names a logical run-ownership node, the key of `topology::Node.id`.
Nodes carry a `NodeRole`, one `MachineId`, and a list of owned `DeviceId`s.
Probe constructs the local master as `NodeId::new(1)` and peer workers as
`index + 2`; cluster assembly remaps node IDs with a separate allocator. Worker
projection resolves the assigned node, requires `NodeRole::Worker`, requires
its machine to equal the assigned `MachineId`, and rejects an empty device
list.

Topology validation requires unique node IDs, known machines, unique devices
within a node, every referenced device to exist and belong to the node's
machine, no device to be owned by two nodes, exactly one master, and every
topology device to have an owner. The measured-profile codec writes and reads
node IDs as little-endian `u64`s and requires topology node arrays to be
strictly increasing by ID. There is no inherent nonzero check. Node IDs do not
cross the remote wire; the remote side receives a prevalidated device/task
projection, while the local executor uses the node only to derive that
projection.

### `DeviceId`

`DeviceId` names one schedulable storage domain, the key of
`topology::Device`. It is also used in `Node.devices`, link endpoints,
discovery capabilities, measured RAM/storage/GPU origins, value residency,
arena layouts, reservations, queue ownership, native bindings, host backend
bindings, worker assignments, and remote worker-device manifests.

Probe's local allocator assigns RAM first, then storage, then GPUs, then peer
RAM, all from one positive counter. Cluster assembly allocates a new global
device range per member and remaps network gateway devices to those IDs.
Profile decoding wraps every binary device field with `DeviceId::new`.
Current-hardware resolution never falls back to capacity, product name,
ordinal, or benchmark similarity: it matches retained per-machine origin keys
and requires an exhaustive current inventory.

Topology validation requires unique device IDs, a known owning machine, one
node owner, and kind-specific calculation capability (`GpuMemory` must have a
calculation rate, RAM and disk must not). Discovery requires every topology
device exactly once, available, schedulable, and asynchronously transferable;
GPU devices additionally require a valid calculation capability. The profile
codec orders topology devices and discovery devices strictly by ID. Ingest
image packing rejects duplicate device manifests, and remote provisioning
sorts worker device IDs, rejects zero or duplicate IDs, and rejects unknown
devices. Native backends keep arenas, queues, artifacts, and staging keyed by
`DeviceId`, so a missing or unexpected device is a preparation or handoff
failure rather than a fallback lookup.

Topology and remote message fields encode a `DeviceId` as little-endian `u64`;
planner, preparation, worker-projection, and remote-program digests include
the numeric value. The core type itself does not reject zero, so the strict
nonzero policy is supplied by remote provisioning, selected native boundaries,
and any contract that treats zero as reserved.

### `LinkId`

`LinkId` names one directed measured edge, the key of
`topology::DirectedLink`. A physical bidirectional transport has two different
link IDs, one for each direction. Probe's `LinkBuilder` allocates consecutive
forward and reverse IDs; cluster assembly remaps member links and allocates
two new IDs for each inter-machine pair. `DiscoveredLink`, route vectors,
transfer lane claims, scheduler resources, worker external-transfer contracts,
and native bridge contracts all carry this ID.

Topology validation requires unique link IDs, known and distinct endpoint
devices, and exactly two reverse edges for each transport. Route validation
rejects unknown links, a link whose `from` differs from the current endpoint,
empty routes between distinct devices, and a route ending at the wrong device.
The Draft boundary requires an executor-visible internal transfer to contain at
most one link, and lane claims must be sorted, unique, in range, and exactly
cover the route. Worker projection and remote provisioning repeat the one-hop
and direction checks. Profile codec arrays of links and discovery links are
strictly increasing by link ID. Link IDs are encoded in profile bytes and
included in planner, Draft, and preparation hashes; they are not written
directly by the worker-projection digest or sent as remote message fields,
although a remote cross-transfer is derived from its one link.

### `TransportId`

`TransportId` names the physical transport family shared by the two directed
links in one bidirectional pair. Probe allocates one transport ID per pair;
cluster remaps it independently from link and resource IDs. It exists only as
`DirectedLink.transport` and as the topology validator's grouping key. Two
links with one transport ID must be exact reverses, have the same
`TransportKind`, and have the same duplex mode. A transport ID is required to
be nonzero. The same numeric value may be used by a link, resource, or another
member before remapping because these are distinct types.

The profile codec encodes the field as a little-endian `u64`; canonical array
ordering is by `LinkId`, not by transport ID. `Topology::duplex_mode` uses the
transport and endpoint pair to verify whether two link IDs are a valid half or
full duplex pair. Scheduling and remote code normally consume the link's
resolved duplex/resource facts rather than carrying `TransportId` further.

### `DuplexResourceId`

`DuplexResourceId` names one capacity token shared by opposing directions when
the transport is half duplex. It is the `DirectedLink.capacity_resource` field.
Probe gives a forward and reverse edge the same resource for half duplex and
distinct resources for full duplex. Cluster assembly remaps resources in a
separate typed map, preserving those equality or inequality relationships.

Topology validation requires a nonzero resource, permits a shared resource
only within one transport, requires equality for the two half-duplex edges,
and requires different resources for the two full-duplex edges. The static
scheduler turns a half-duplex resource into a
`HalfDuplexDirection(DuplexResourceId, LinkId)` reservation. Remote model
construction derives one `DuplexClaim` from each cross-machine link, sorts and
deduplicates claims, rejects zero or repeated resources, and builds a fixed
token array. Remote sessions assign the token to one `TaskId` at a time and
fail closed if ownership is missing, duplicated, or released by another task.

The profile codec writes the resource as little-endian `u64`. Remote program
digests and cross-transfer manifests hash resources, while link IDs remain an
internal derivation. No constructor-level nonzero guard exists; topology and
remote claim validation provide it.

## Graph and kernel IDs

### `ValueId`

`ValueId` is the identity of a graph tensor or a planner-resident physical
copy. Source graph tensors use it as `Tensor.id`; kernel inputs and outputs,
metric values, aliases, init-image members, artifact bindings, task endpoints,
arena bindings, and resolved locations all refer to it. A logical value can
have multiple physical copies, one per selected device. During transfer
lowering, the final destination keeps the first allocated physical value and
intermediate hop values receive stable offset IDs; only the final copy is
registered as the logical destination copy.

Language graph assembly indexes tensors by `BTreeMap<ValueId, _>`, rejects
duplicate tensor IDs, unknown boundary or kernel references, duplicate
producers, missing producers, and cycles, then sorts tensors by ID. Tensor
layout validation reports errors against the value ID but does not reserve
zero. `StaticCalculationProgram::validate` specifically rejects a zero metric
value, requires the value to be an existing four-byte scalar tensor, and
requires it to remain loop-internal rather than an external output.

Training and inference `GraphCompiler`s start value allocation at one and use
checked increments. The operation families for materialization, KNN, tree,
Bayesian, binary metrics, and K-means reserve caller-provided half-open value
ranges. They reject a declared boundary tensor in a reserved range and report
`IdentityNamespaceExhausted` on capacity or `u64` overflow. `IdentityNamespace`
ranges are checked independently for every materialized fragment before graph
assembly. Planner value allocation, route-hop offsets, arena groups, and init
images all use checked arithmetic and fail as invalid drafts when a reference
or location cannot be resolved.

`DraftPlan::validate` requires unique value IDs, known resident devices, and a
typed byte count. Every value binding and alias must resolve to a known value
and arena object; aliases enforce exact or non-overlapping storage according
to `AliasPermission`. Finalize resolves a value to one device, object, object
offset, and arena offset, rejecting wrong-device, out-of-bounds,
misaligned, or overflowing locations. Ingest's init-image packer uses
`BTreeMap<ValueId, _>`, rejects duplicate or unexpected sources, conflicting
replicated type/size contracts, duplicate logical or physical members, and
overlapping image ranges.

Recipe IR and program OGDL encode values as decimal text and decode with
`ValueId::new`; the resulting graph or program validation is the failure
boundary. Planner, primitive, kernel, preparation, and remote-program identity
hashes write the numeric value as little-endian `u64`. Value IDs are not
remote message fields. Worker projection binds values through the bundle and
task contracts but does not write each value into its own projection digest.
`ValueId` has no global namespace across graphs,
fragments, devices, or runs; the caller-reserved operation ranges and the
immutable bundle identity provide the scope.

### `ScalarValueId`

`ScalarValueId` is an intra-kernel SSA identity, not a graph tensor identity.
It names `ScalarInput`, `ScalarConstant`, `ScalarInstruction.result`, and
scalar-program outputs and operands. The scalar builder starts at one and
allocates in call order. `ScalarProgramBuilder` adds a private builder owner
token to each `ScalarExpression`, so an expression from one builder cannot be
applied to or returned by another builder. The operation scalar composer uses
the same one-based, checked local counter.

`ScalarProgram::validate` indexes definitions by `BTreeMap<ScalarValueId,
DType>`, rejects duplicate input/constant/instruction definitions, use before
definition, unknown outputs, invalid arity, and type signatures. It does not
reject zero, because zero is not a constructor-level sentinel for this local
SSA namespace. Kernel LLVM lowering uses a `BTreeMap<ScalarValueId, ValueRef>`
and reports an unknown scalar value when a definition is absent.

The Recipe IR OGDL codec encodes scalar IDs as decimal fields and wraps decoded
numbers with `ScalarValueId::new`. Primitive, planner, and kernel hashes write
all scalar IDs as little-endian `u64`, so changing an SSA numbering changes the
artifact identity even when the arithmetic is otherwise equivalent. A scalar
program's IDs may numerically overlap graph `ValueId`s without conflict; the
types and the enclosing kernel template keep the namespaces separate.

### `KernelTemplateId`

`KernelTemplateId` names a placement-free source kernel or a planner-derived
stage template. Source IDs are present in `PrimitiveKernel.id`, graph nodes,
program iteration domains, and training/inference graph compilers. Operation
materializers reserve kernel ranges just as they reserve value ranges.

Planner lowering derives one stage-scoped ID with the
`recipe-planner-stage-template-v1` digest domain from the lowered program
digest, source-kernel ID, and stage ordinal. It takes the first eight digest
bytes as a little-endian `u64`, rejects zero, and keeps a map of stage IDs to
`(source_kernel, stage_ordinal)` to reject collisions. The kernel stage
realizer independently recomputes the same identity and rejects a stale or
mutated build contract. Stage templates are stored in `KernelTemplate`, while
the original source ID remains in `ArtifactBuildRecipe.source_kernel`.

Graph validation requires unique kernel IDs and valid value references. A
`KernelTemplate` validates its scalar program, unique input and output IDs,
complete alias coverage, and alias references, but the core type itself does
not require a nonzero template ID. Program OGDL encodes source and iteration
domain IDs as decimal text. Primitive, planner, kernel-stage, preparation, and
bundle hashes include the ID as little-endian `u64`; graph and program maps use
numeric ordering for canonical topological and domain lookup.

Operation and training range allocators prevent independently materialized
fragments from colliding. Planner stage collision checks prevent different
source/stage pairs from sharing one derived ID. The source and stage-scoped
values intentionally share this Rust type, so the enclosing field and planner
collision map, not the number alone, communicate which role an ID has.

### `KernelInputId` and `KernelOutputId`

These are ordinal identities in a kernel's typed ABI, with separate input and
output namespaces. Primitive lowering creates them from vector positions using
`index + 1`, checked through `u64::try_from`; examples and native probe
benchmarks use the same one-based values. Planner source-alias conversion uses
the same checked `stable_argument_position` helper. The zero-based source
alias indices are converted to these one-based IDs before entering a
`KernelTemplate`.

`KernelTemplate::validate` requires unique input IDs and unique output IDs,
requires every alias rule to name an existing input and output, rejects
duplicate pairs, and requires the complete input/output alias matrix. Planner
and arena alias code converts an ID back with `get().saturating_sub(1)` and
then checks the vector bounds; a hand-built zero ID therefore fails at that
consumer rather than being rejected by `KernelInputId::new` itself.

Input and output IDs are not fields in the source OGDL alias syntax, whose
indices are `usize`; they are included in primitive, planner, and kernel
contract hashes as little-endian `u64`. They are local to one kernel template,
not global graph IDs, task IDs, or artifact IDs.

## Planned work and resource IDs

### `TaskId`

`TaskId` is the identity of one immutable `Task` in a Draft and FinalizedBundle.
It names calculations, transfers, metric readbacks, phase barriers,
dependencies, loop domains, journal events, backend requests, worker
projection tasks, remote program tasks, cross-machine transfers, and native
pending tokens. It is unique within one bundle or provisioned remote program,
not globally across bundles or runs.

Planner `StableIdAllocator::new("task")` starts at one. `next_submission`
allocates a task and uses its number as the initial queue and completion slot
number. A multi-hop transfer uses checked `stable_offset(base, hop, "task")`
values, preserving deterministic hop order. External egress first peeks the
next allocator value, trials candidates, then verifies allocation did not
change. Scheduler maps tasks by `BTreeMap<TaskId, _>`, uses `TaskId` as the
critical-path tie-break, and rejects duplicate IDs, unknown dependencies,
duplicate dependencies, invalid windows, and dependency cycles. Draft
validation repeats uniqueness, reference, phase, and schedule-order checks.

Remote codec fields carry task numbers as little-endian `u64`; decoded tasks
are looked up in sorted provisioned task arrays. Remote provisioning rejects
zero task IDs and duplicates, and every submit, poll, data, metric, and release
operation rejects an unknown, duplicate, inactive, or wrong-phase task. Native
and local executors key contracts, pending resources, completion ownership,
egress images, and errors by `TaskId`. Worker projection includes sorted task
IDs and dependencies in its digest. Preparation hashes task lists in ID order
and sorts dependency lists for its canonical identity.

`TaskId::new(0)` appears in host, native, and bridge error paths as a diagnostic
placeholder when no real task exists. It is not a valid planned task. The core
newtype and scheduler do not impose the nonzero rule, so remote provisioning,
worker/executor lifecycle, and backend handoff are the observed boundaries for
that policy.

### `ArtifactId`

`ArtifactId` names one realized native image or one deferred build reservation.
It appears in `ArtifactIdentity.id`, `ArtifactBuildRecipe.artifact`,
calculation tasks, stage placements, finalized bundles, native runtime maps,
candidate realization requests, and remote artifact manifests. The planner
currently constructs it with `ArtifactId::new(stage_template.get())`, which
keeps the same numeric stage identity while retaining a distinct Rust type.
Native catalog providers pass the reserved ID into runtime artifacts; native
error paths also use `ArtifactId::new(0)` when constructing a placeholder.

`ArtifactBuildRecipe::validate` requires nonzero artifact, stage-template, and
source-kernel IDs. Draft validation rejects duplicate artifact IDs, an artifact
that is both realized and deferred, duplicate deferred stage identities, and
calculation tasks whose artifact cannot resolve to exactly one identity or
build. The planner's artifact catalog validator explicitly rejects zero or
duplicate IDs. `ArtifactIdentity::validate` validates digest, toolchain, and
resource identity but does not itself add a separate nonzero check for `id`,
so planner, candidate, native, and remote boundaries remain important.

Native catalog construction sorts artifacts by ID and binary-searches them.
Candidate validation requires the static/deferred artifact sets to be disjoint,
requires each requested artifact exactly once, and checks target, resource,
provenance, runtime digest, and ABI against its ID. CUDA and HSA resources use
`BTreeMap<ArtifactId, _>` and fail on missing, unexpected, duplicate, or
mismatched artifacts. The remote manifest sorts IDs strictly, rejects zero
IDs or zero digests while proving the manifest, and hashes each ID and digest.
Artifact IDs are encoded as little-endian `u64` in that manifest and in all
planner, prepare, kernel-stage, and remote identity hashes. They are not part
of the user graph OGDL.

### `QueueSlotId` and `CompletionSlotId`

These are separate typed namespaces for preallocated submission and completion
resources. `SubmissionSlots` carries one of each on every calculation,
transfer, and metric task. Planner allocation initially derives both from a
task ID. Submission compaction then performs deterministic interval coloring:
the first task on each color supplies the stable physical number, and the
resulting queue and completion arrays are sorted by their own IDs. Queue
capacity is checked against measured per-device maximum submission queues.

`ResourceManifest::validate` requires unique queue IDs and unique completion
IDs, and requires each slot's device to exist. Draft task validation requires
the queue and completion to exist and both to belong to the task's submission
device. It does not require the two numbers to be equal or nonzero, even though
the planner normally pairs them. Scheduler reservations treat queue and
completion as independent exclusive resources. Native CUDA, HSA, and host
backends keep typed `BTreeMap`s, reject missing slots, and reject a completion
event or signal owned by another task. Queue and completion IDs are bundle-local
resource names, not driver stream or event handles.

They have no direct remote wire representation. The numeric values are included
in planner and preparation bundle hashes, which makes a different compaction
result a different immutable bundle. Their equality with a task number is an
implementation derivation, not a cross-type identity rule.

### `MetricId`

`MetricId` is a logical metric declaration identity. Public static programs
carry it in `MetricEmission`; training compilation assigns one-based ordinals
to metric bindings; program OGDL reads and writes it as a decimal field; and
planner user metrics preserve it. Planner-generated fault readbacks use the
readback task number as a synthetic metric number. The ID is then carried in
`MetricTask`, `MetricSlot`, executor/backend metric work, training observers,
and the live training presentation map.

`StaticCalculationProgram::validate` is the strict user-facing boundary: a
metric ID must be nonzero and unique, its value ID must be nonzero and refer to
an existing four-byte scalar tensor, and its activation domain must be within
the loop. Core Draft validation requires a metric task's slot to exist and to
be assigned to the same metric; fault readbacks require one int32 value and
one exclusive slot per fault cohort. The executor mailbox checks that a
completion's metric matches the metric bound to its slot. There is no
constructor-level nonzero rule and no global core uniqueness check spanning
user and synthetic metrics, so these stage-specific checks define validity.

Program OGDL encodes the ID as decimal text. Planner, preparation, and graph
identity hashes write it as little-endian `u64`; remote metric messages carry
the task ID and scalar payload, not the logical metric ID. A metric ID is not a
metric slot ID: the former names what the user selected, while the latter
names where the runtime stores one physical readback.

### `MetricSlotId`

`MetricSlotId` names that physical readback/mailbox slot. Planner creates a
user metric slot from its metric task ID and creates a fault-readback slot from
the readback task ID. `ResourceManifest.metrics` stores the slot-to-metric
association. Executor `MetricMailbox` indexes slots in a `BTreeMap`, retains
one newest user sample per slot, and exposes `try_take_metric`; host, native,
and training execution layers pass the slot alongside the logical metric.

Resource validation requires unique metric slot IDs. Draft validation requires
every metric task to resolve a slot whose `MetricId` matches, and fault
validation requires an exclusive slot for the one readback of each fault
cohort. The user mailbox is created only for `MetricPurpose::User`; fault slots
are consumed internally and become `FaultChecked` events. No constructor-level
nonzero check exists, and no remote message serializes a slot ID. Draft and
preparation hashes include the slot number, so changing the physical mailbox
layout changes the bundle identity. Slot IDs are bundle-local and may
numerically equal metric or task IDs without being interchangeable.

### `ArenaObjectId`

`ArenaObjectId` names one logical allocation object before Finalize chooses its
physical byte offset. It appears in `ArenaObject`, `ValueBinding`,
`ResolvedValueLocation`, `ArenaAllocation`, arena layouts, alias disjoint-set
groups, and native arena lookup. Planner's checked `StableIdAllocator::new
("arena object")` allocates image objects first and then one object for each
remaining alias group. Objects are grouped per device and packed by the
scheduler using lifetime overlap, alignment, and ascending ID tie-breaking.

Draft validation requires unique object IDs, known devices, nonzero power-of-two
alignment, and nonempty lifetimes. Finalize requires every object to have one
allocation, no allocation to be repeated, all object references to be known,
the allocation to be on the right device, aligned, in bounds, and nonoverlapping
for overlapping lifetimes. Value resolution then combines the object offset
and binding offset with checked arithmetic and rejects wrong-device,
out-of-bounds, misaligned, or overflowing locations. Native backends use the
resolved object and arena offsets, never inventing an allocation from the ID.

Arena object IDs have no direct wire or OGDL representation. They are included
in planner and preparation identity hashes as little-endian `u64` and are
looked up in ordered maps. The type does not reject zero; planner allocation
starts at one and Draft uniqueness/referral checks are the effective boundary.

### `RunId`

`RunId` is the runtime epoch that scopes an execution of an otherwise immutable
bundle. Training and inference call `next_run_id`, which combines the current
clock epoch, process identity, and an atomic sequence and clamps the result to
at least one. Warm native traces use positive stabilization-pass numbers as
temporary run IDs. The run is carried through `PreparedRun`, worker execution,
backend work, journal events, native CUDA/HSA launch arguments, training and
inference reports, and remote session typestates.

CUDA and HSA `KernelArgument::RunId` receives the dynamic `u64` value for
counter-based random stages. The run is intentionally not embedded in a
native artifact or bundle hash; repeating the same task IDs with a new run
epoch is valid and produces a distinct runtime context.

Remote framing places the run ID in every message header as little-endian
`u64`. `remote::validate_run_id` rejects zero, `SessionCore::new` requires a
strictly increasing run when a `RemoteChannel` is reused, and received traffic
with a different run poisons the session. Worker execution rejects a zero run
at preparation and rejects every later API call whose run differs from the
prepared value. The general executor receives a `RunId` and propagates it but
does not perform the same constructor-level check, so remote and worker
boundaries are the observed fail-closed guards. `RunId::new(0)` remains
constructible for diagnostics or an invalid caller input.

## Encoding, ordering, and identity proofs

The IDs have no shared serializer. The following explicit boundaries are the
ones that preserve their numeric values:

| Boundary | IDs | Representation and canonical rule |
| --- | --- | --- |
| Measured profile codec | `MachineId`, `NodeId`, `DeviceId`, `LinkId`, `TransportId`, `DuplexResourceId` | Manual little-endian `u64` fields. Decode wraps with `new`, then validates checksum, references, origins, and strict increasing arrays for origins, machines, nodes, devices, links, discovery devices, discovery links, and each node's device list. |
| Recipe IR OGDL | `ValueId`, `KernelTemplateId`, `ScalarValueId` | Decimal text fields. Decode wraps with `new`, then graph, kernel, and scalar validation supplies uniqueness and reference checks. |
| Static program OGDL | `KernelTemplateId`, `MetricId`, `ValueId` | Decimal text fields. Decode wraps with `new`, sorts domains and metrics by ID, and validates nonzero metric/value IDs and graph references. |
| Planner, primitive, kernel, prepare, and worker hashes | All graph, stage, artifact, task, resource, value, and topology IDs that participate in a digest | Explicit little-endian `u64` words in versioned SHA-256 domains. Any ID or ordering change changes the candidate, draft, artifact, projection, or program identity. |
| Remote wire and manifest proof | `RunId`, `DeviceId`, `TaskId`, `ArtifactId` | Header and message fields are little-endian `u64`. Manifest artifact entries are sorted and strictly increasing, nonzero, and hashed with their digests. Model validation supplies nonzero task/device/resource/run rules. |
| Diagnostics | Every ID | `Display` delegates to `u64`, producing an untagged decimal number. |

Canonical ordering is always numeric `Ord`, not allocation address. Profile
construction and cluster remapping make positive allocations deterministic;
graph and program surfaces sort IDs before lookup or serialization; scheduler
uses IDs to break ties; native catalogs and remote manifests binary-search or
sort IDs; and preparation hashes normalize task, dependency, alias, and
artifact order. IDs that are merely fields inside a typed resource, such as a
transport or duplex resource, need not have their own top-level array order,
because the enclosing link order and topology validation define their scope.

## Failure boundaries and the zero value

The macro is intentionally a data wrapper, not a validator. The effective
fail-closed sequence is:

1. Probe and cluster allocators construct positive topology IDs, profile codecs
   reconstruct them, and topology/discovery validation checks uniqueness,
   ownership, references, duplex relationships, and required capabilities.
2. Language, operation, and training compilers allocate graph and SSA IDs in
   checked ranges. OGDL decoding is followed by graph, scalar, kernel, and
   static-program validation rather than trusting parsed numbers.
3. Planner validates stage hash nonzero/collision rules, artifact sets, task
   references and phases, transfer links and claims, metric bindings, value
   locations, and arena objects before scheduling or Finalize.
4. Preparation and native realization require exact artifact, device, queue,
   completion, arena, target, and ABI matches. Remote provisioning adds the
   strict nonzero and canonical manifest rules needed by a wire protocol.
5. Execution scopes task/resource lookup by `RunId`, checks worker and remote
   ownership, and reports unknown or mismatched IDs instead of substituting a
   nearby object.

Zero is reserved only where a consumer says so. Observed strict zero checks
include topology transport and duplex-resource IDs, planner stage identities,
artifact-build IDs, planner artifact catalogs, static-program metric and value
IDs, remote artifact/task/device/resource/run IDs, and worker run IDs. Core
topology, scalar, kernel, resource-manifest, task, queue, completion, metric
slot, arena, and artifact-identity structures otherwise enforce uniqueness or
references without a universal zero check. The zero IDs constructed in native,
host, and bridge error paths are placeholders and must never enter a validated
bundle. This split is deliberate: each boundary knows whether zero is a valid
local number, a reserved sentinel, or malformed untrusted input.

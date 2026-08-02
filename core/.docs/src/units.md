# Recipe unit types

`recipe_core::units` is the dimensional boundary for Recipe's static execution
pipeline. It contains integer newtypes for byte quantities, measured rates,
operation work, schedule time, transfer concurrency, and parallel extents. The
tuple fields are private, so code outside `recipe-core` cannot accidentally
construct a value with the wrong unit or mix two raw integers. `core/src/lib.rs`
re-exports the module, which is why the other workspace crates normally import
these names directly from `recipe_core`.

The representation is deliberately exact and integer-only:

- byte and FLOP quantities are `u64` values;
- rates are nonzero `u64` values;
- transfer lane counts are nonzero `u32` values;
- schedule time is integer nanoseconds in a `u64`; and
- index extents are nonzero `u64` values.

`ByteCount` is a decimal byte quantity in the project contract. “Decimal” names
the unit convention, not a different integer representation: a value of 1024
means exactly 1024 bytes, and no binary-prefix conversion is performed. The
seed contract consequently writes values such as `1_000_000_000` bytes and
`125_000_000` bytes per second directly.

## The types and their dimensions

| Type | Raw representation and meaning | Zero allowed? | Checked operations |
| --- | --- | --- | --- |
| `ByteCount` | `u64`, an exact number of bytes in a payload, capacity, allocation, image, workspace, or resource bound | Yes, through `ZERO` or `new(0)` | `checked_add`, `checked_sub`, `checked_mul(u64)`, `checked_align_up` |
| `ByteOffset` | `u64`, a byte address relative to the beginning of an arena, object, or packed image | Yes, `new(0)` is the normal first offset | `checked_end(ByteCount)` |
| `BytesPerSecond` | nonzero `u64`, measured transfer bandwidth or memory throughput | No | `new`, `get`, ordering and equality from the derived traits |
| `TransferLaneCount` | nonzero `u32`, the number of same-direction transfers that may be in flight | No | `new`, `get`, ordering and equality |
| `FlopCount` | `u64`, exact floating-point operation work used by the base cost model | Yes, through `ZERO` or `new(0)` | `checked_add`, `checked_mul(u64)` |
| `FlopsPerSecond` | nonzero `u64`, measured calculation throughput | No | `new`, `get`, ordering and equality |
| `Nanoseconds` | `u64`, deterministic schedule time, not a wall-clock `Duration` | Yes, through `ZERO` or `new(0)` | `checked_add` |
| `ElementCount` | nonzero `u64`, one dimension or total element count in a native parallel index space | No | `new`, `get`, ordering and equality |

All of the structs derive `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq`,
`PartialOrd`, `Ord`, and `Hash` where the source declares those traits.
`ByteCount`, `FlopCount`, and `Nanoseconds` additionally derive `Default`; the
nonzero-domain types intentionally do not. There are no `From` conversions,
floating-point conversions, or implicit unit coercions. Crossing an integer
boundary is always explicit through `get`, `new`, or a checked standard-library
conversion such as `usize::try_from`.

### `ByteCount`

`ByteCount::new` and `get` are `const fn`. `ByteCount::ZERO` is the canonical
zero value. The normal arithmetic methods preserve the byte dimension:

- `checked_add` and `checked_sub` add or subtract two byte quantities;
- `checked_mul` multiplies a byte quantity by a raw scalar such as an element
  width, iteration count, or number of staged objects; and
- `checked_align_up` rounds a byte quantity upward to a valid byte alignment.

The methods return `Option` for ordinary overflow and never saturate. The
alignment method is the exception because it must distinguish a bad alignment
from an arithmetic overflow. It accepts only a nonzero power of two and uses
the exact integer operation `(value + alignment - 1) & !(alignment - 1)` after
checking the addition. A zero or non-power-of-two alignment returns
`UnitError::InvalidAlignment(value)`; an addition that cannot be represented
returns `UnitError::Overflow`.

`ByteCount` implements `Display` as `"<decimal> B"`. This is used in scheduler,
planner, preparation, and runtime error messages so diagnostics retain the
unit.

### `ByteOffset`

An offset is intentionally not a count. The only arithmetic supplied by the
type is:

```text
offset.checked_end(size) = ByteCount(offset + size)
```

The raw addition is checked and returns `None` on overflow. Returning a
`ByteCount` for the end makes half-open range checks explicit: the start is an
offset, while the end is an absolute byte count from the same arena origin.
Callers that need to compare two ranges re-wrap a checked offset with
`ByteCount::new(offset.get())`; there is no offset-plus-offset operation.

### Rates and transfer lanes

`BytesPerSecond::new` and `FlopsPerSecond::new` reject zero with
`UnitError::ZeroRate`. A zero rate would make the time equation undefined and
must not reach a schedule. `TransferLaneCount::new` rejects zero with
`UnitError::ZeroTransferLaneCount`; a zero-length lane group could not admit a
transfer. The lane count is directional. A forward link and its reverse may
have different values, and scheduler lane IDs are the exact range
`0..count.get()`.

The unit type does not encode provenance. A rate or capacity is wrapped in
`topology::Property<T>` when its origin (`Estimated`, `Measured`, or `Override`)
matters. The value type enforces dimensions and nonzero rates; `Property`
enforces whether that value may drive production scheduling.

### FLOP work and calculation rates

`FlopCount` is the work numerator in the base calculation equation. It counts
the conventional floating-point work selected by Recipe, not every operation
in a kernel. `ScalarOpcode::flops` counts ordinary arithmetic and comparisons as
one, FMA as two, and addressing, integer operations, representation changes,
validation predicates, and bit operations as zero. Primitive work formulas and
lowered stage resource bounds wrap their checked `u64` totals in `FlopCount`.

`FlopsPerSecond` is the measured denominator. The type is distinct from
`BytesPerSecond`, even though both are represented by a `u64`, so a memory rate
cannot be passed to a calculation timing call.

### `Nanoseconds`

`Nanoseconds` is schedule time, not a host timer and not a `Duration`. It has a
zero constant, a checked same-unit addition, and an explicit `get` for the
places that must call `Duration::from_nanos` or hash schedule data. A zero value
is useful as an origin and phase floor. Individual scheduler tasks are normally
forced to at least one nanosecond by the scheduler with `.max(1)`, while the
unit itself does not impose that policy.

`ScheduleWindow` in `core/src/schedule.rs` stores `start` and `end` as
`Nanoseconds` and treats the interval as half-open. It is valid exactly when
`start < end`; overlap is `start < other.end && other.start < end`.

### `ElementCount`

`ElementCount` is the nonzero extent domain used by a native index space. The
`IndexSpace::new` constructor in `core/src/scalar.rs` requires at least one
dimension, multiplies all dimension counts with checked `u64` arithmetic, and
stores the checked product as another `ElementCount`. A zero dimension or an
overflow returns `UnitError::ZeroElementCount` or `UnitError::Overflow`.

This is intentionally stricter than language-level shape metadata. A
`recipe_language::Shape` may contain a zero extent and is lowered as a
no-dispatch calculation. A native `IndexSpace` cannot contain zero because it
must describe a real launch domain and must never manufacture a fake one-lane
dispatch.

`StaticBufferAccess::linear` and `::contiguous` multiply element counts by
`DType::byte_width()` (currently four for both `F32` and `I32`) and return an
exact `ByteCount` storage bound. Strides and offsets remain expressed in
elements until a layout crosses into a byte address.

## Timing equations and overflow behavior

The two public timing functions are the only dimensional multiplication between
work and time:

```text
transfer_time_ceil(ByteCount bytes, BytesPerSecond rate)
    = ceil(bytes / rate seconds) in Nanoseconds

calculation_time_ceil(FlopCount flops, FlopsPerSecond rate)
    = ceil(flops / rate seconds) in Nanoseconds
```

`ratio_time_ceil` performs both equations in one implementation. It computes
`work * 1_000_000_000` in `u128`, adds `rate - 1`, divides by the positive rate,
and converts the result to `u64`. The wider intermediate prevents a premature
`u64` overflow. `UnitError::Overflow` is returned if the multiplication, the
ceiling addition, or the final `u64` conversion cannot be represented. The
typed rate constructors make the denominator nonzero before this private helper
is called. Zero work therefore produces zero nanoseconds; scheduler callers
that need a nonempty task window apply `.max(1)` at the scheduling boundary.

The system contract states the corresponding base equations as
`operation_FLOPs / measured_device_FLOPs` and
`bytes / measured_link_bandwidth`. The implementation's integer ceiling is the
conservative, deterministic realization of those equations. It prevents a
fractional nanosecond from shortening a dependency or resource reservation.

## `UnitError`

`UnitError` is `Copy`, `Clone`, `Debug`, `PartialEq`, and `Eq`, implements
`Display`, and implements `std::error::Error`. Its display strings are stable
and intentionally short:

| Variant | Display text | Produced by |
| --- | --- | --- |
| `Overflow` | `unit arithmetic overflow` | checked alignment, index products, byte products, offset ends, and timing conversion after their callers map the failure back to the unit layer |
| `ZeroRate` | `rate must be nonzero` | `BytesPerSecond::new(0)` and `FlopsPerSecond::new(0)` |
| `ZeroTransferLaneCount` | `transfer lane count must be nonzero` | `TransferLaneCount::new(0)` |
| `ZeroElementCount` | `element count must be nonzero` | `ElementCount::new(0)` and an empty `IndexSpace` dimension list |
| `InvalidAlignment(value)` | `alignment must be a nonzero power of two, got <value>` | `ByteCount::checked_align_up` |

`ByteCount::new`, `ByteOffset::new`, `FlopCount::new`, and
`Nanoseconds::new` deliberately do not reject zero. Those domains use zero as
a valid identity or sentinel. Callers add the policy appropriate to their
contract, for example, a nonempty init image, a nonzero arena allocation, or a
one-nanosecond scheduler task.

## Topology and discovery

The unit types enter the hardware model in `core/src/topology.rs`:

- `Device.capacity` is `Property<ByteCount>` and describes total storage for
  GPU memory, RAM, or disk.
- `Device.transfer_rate` is `Property<BytesPerSecond>`.
- A GPU device has an optional `Property<FlopsPerSecond>` calculation rate.
- Each `DirectedLink.bandwidth` is a directional
  `Property<BytesPerSecond>`.
- Each directional link also carries a directional
  `Property<TransferLaneCount>` and a duplex capacity-resource identity.

Topology validation checks object references, reverse link pairing, and duplex
resource ownership. Full-duplex reverse edges must have independent capacity
resources; half-duplex reverse edges must share one. The separate
`validate_scheduling_properties` pass rejects `Estimated` capacities, rates,
and lane counts before scheduling, while accepting `Measured` and explicit
`Override` values. The nonzero numeric invariants are supplied by constructors
when a topology is assembled or decoded.

`core/src/discovery.rs` carries the same dimensions in an immutable capability
snapshot. A `CalculationCapability` has a `Property<FlopsPerSecond>` and a
`ByteCount` shared-memory limit. `TransferCapability` and `DiscoveredLink`
carry `BytesPerSecond` and `TransferLaneCount` values. Discovery validation
requires available objects, asynchronous submission, nonzero queue and
calculation concurrency, nonzero power-of-two subgroup width, and nonzero
shared memory. It also requires every discovered bandwidth and lane property to
match the corresponding topology direction exactly.

## Probe and measurement path

### Seed estimates

`probe/src/seed.rs` parses `topology/contract.toml` into typed
`SeedEstimates`. Byte fields use `ByteCount::new`; transfer fields use
`BytesPerSecond::new`; calculation fields use `FlopsPerSecond::new`. A zero
rate is rejected while a zero capacity remains representable until the probe's
domain checks reject it. The seed parser also enforces the exact
`1_000_000_000`-byte reservation and requires all discovery and benchmark gates
to be enabled. These estimates only size bounded first-pass workloads. They
never drive a production schedule.

### Host, GPU, and peer probes

`probe/src/engine.rs` derives each `BoundedBenchmarkPlan.buffer_bytes` as a
`ByteCount`, clamped to the configured minimum and maximum. Host discovery
converts RAM, filesystem, and block-device counters to `ByteCount` after checked
unit conversions. The local benchmark computes

```text
total bytes * 1_000_000_000 / elapsed nanoseconds
```

with `u128`, then constructs a measured `BytesPerSecond`. A zero iteration
count, zero elapsed duration, arithmetic overflow, a rate that does not fit
`u64`, or a resulting zero rate is a benchmark error.

Native CUDA and HSA probing uses the same dimensions. Driver memory counters
become `ByteCount`; LDS/shared-memory limits are converted from checked
kilobyte values; host-to-device and device-to-host lane counts are constructed
as nonzero `TransferLaneCount`. `native-probe/src/benchmark.rs` creates a
nonzero `ElementCount` index space and contiguous f32 access for its FMA probe.
It counts the lowered kernel's `FlopCount`, multiplies by timed iterations with
checked `u128` arithmetic, and constructs a measured `FlopsPerSecond`.

Peer probes retain `DirectionalBenchmarkEvidence.total_bytes` as a
`ByteCount`, elapsed and sample times as raw integer nanoseconds, and derive
directional `BytesPerSecond` values from the exact byte and elapsed evidence.
Full-duplex probes measure both directions simultaneously; half-duplex probes
serialize them. The engine checks that the evidence's byte total, sample count,
duration bounds, and derived rate agree exactly before creating topology links.
`TransferLaneCount` remains directional when the measured descriptor is copied
into the link pair.

Cluster assembly in `cluster/src/assemble.rs` resolves peer RAM origins by their
stable identity, checks the exact capacity and transfer rate against the member
profile, preserves the directional `BytesPerSecond` and `TransferLaneCount`,
and assigns one shared or two independent duplex resources. Unit values survive
ID remapping unchanged.

## Serialization and hashing

The unit structs do not derive `serde::Serialize` or `Deserialize`, and there
is no generic unit serializer. Every boundary writes the underlying integer in
an explicit schema and reconstructs the type deliberately.

### Measured-profile codec

`probe/src/codec.rs` is the canonical profile codec. It writes:

- `ByteCount` properties as little-endian `u64` followed by a provenance tag;
- `BytesPerSecond` properties as little-endian `u64` plus provenance;
- `TransferLaneCount` properties as little-endian `u32` plus provenance; and
- `FlopsPerSecond` properties as little-endian `u64` plus provenance.

It writes benchmark buffer bytes, peer evidence totals, topology capacities,
link bandwidths, lane limits, calculation rates, and shared-memory byte limits
through those helpers. On decode, byte values are wrapped with `ByteCount::new`;
rates, lanes, and FLOP rates go through their nonzero constructors, so malformed
zero rate or lane fields fail as codec errors before a profile can be accepted.
The decoded topology and discovery profile are then validated for references,
duplex shape, availability, provenance, and capability invariants.

### TOML and OGDL

The seed contract is a hand-written TOML-like parser. Its numeric fields are
parsed as unsigned integers before the unit constructors are called. Semantic
OGDL in `language/src/ogdl.rs` writes tensor `storage_bytes` as decimal text and
decodes it back through `ByteCount::new`; shape extents and element offsets
remain raw element metadata until `Shape`, `TensorLayout`, or a kernel access
validates them. This is a format boundary, not a unit conversion.

### Transport and remote wire protocols

`transport/src/probe.rs` encodes bounded probe buffer bytes, memory capacity, and
transfer rates as fixed-width big-endian integers. It reconstructs rates through
`BytesPerSecond::new` and rejects zero capacities through
`MeasuredLocalMemory::new`. Directional completion frames validate both raw
rates with the same constructor.

`remote/src/codec.rs` encodes `InitBegin` and `InitEnd` byte counts as fixed-width
little-endian `u64` fields and decodes them with `ByteCount::new`. The codec
preserves the wire integer; the remote session and executor then compare it to
the finalized image contract and reject zero, truncated, oversized, or mismatched
images. Chunk offsets remain raw `u64` wire offsets because they are converted
to host indices only after checked range validation.

### Canonical identity material

Plan, profile, artifact, and realization hashes deliberately serialize unit
values through `get()`. The planner hashes tensor storage bytes, stage FLOPs,
task transfer bytes, schedule-window nanoseconds, arena object sizes and
alignments, object and image offsets, and capacity manifests. Preparation hashes
capacity-ledger byte fields and schedule start/end values. This preserves the
semantic unit value in the identity while keeping the hash format a canonical
integer schema. It does not erase the unit in the in-memory API.

## Scalar, kernel, and artifact flow

`IndexSpace` and `StaticBufferAccess` are the first core consumers of
`ElementCount` and `ByteCount`. `IndexSpace::new` multiplies nonzero dimensions
with checked arithmetic. `StaticBufferAccess::linear` and `::contiguous` multiply
the resulting element span by the four-byte dtype width. Address validation
checks stride products, the final element, and the backing `storage_bytes` bound;
overflow becomes `UnitError::Overflow` at the constructor boundary or a
`ValidationCode::AddressOverflow`/`InvalidMemoryAccess` error when validating an
already-built access.

Language primitive work is returned as `FlopCount`. Elementwise, reduction,
scan, contraction, gather/scatter, histogram, sort, and random formulas use
checked shape products and then wrap the total. Primitive lowering retains
`FlopCount` in each stage's resource bounds and `ByteCount` in shared, private,
scratch, and storage bounds. Artifact build recipes copy those exact values into
`ArtifactWorkBounds` and `KernelResourceBounds`; artifact validation requires a
checked int32 fault binding to occupy exactly `ByteCount::new(4)`.

Native CUDA and HSA argument assembly passes `ElementCount::get()` as the final
launch ABI element-count argument. It passes byte offsets and byte sizes only
after checking device alignment, backing range, and host integer conversion.

`kernel/src/llvm.rs` constructs that ABI from the same core values. It lowers a
scalar template's per-element opcode work, multiplies the checked total by the
`IndexSpace`'s `ElementCount`, and stores the result as `LoweredKernel.work`.
The final ABI has a by-value `element_count` argument and a typed
`KernelAbi.elements`; no byte count is used as a substitute for the launch
extent. `kernel/src/stage.rs` performs the equivalent stage-level conversion,
rejecting a zero logical lane count through `ElementCount::new` and preserving
the stage's `FlopCount`, `ByteCount` resource envelope, and four-byte fault
binding. The artifact verifier hashes and compares the exact storage-byte and
FLOP fields before it accepts realized LLVM metadata.

## Language and operation facades

The public language layer keeps shape extents and tensor layout metadata as raw
`u64` element quantities until it has enough context to choose a dtype. A
`Shape` multiplies extents with checked arithmetic and then `Shape::bytes` wraps
the product times `DType::byte_width()` in `ByteCount`. A `TensorLayout` stores
element offsets and strides; `byte_offset` performs the checked offset-elements
times dtype-width product and returns a `ByteOffset`. `Tensor::validate` checks
the complete layout span against its declared `storage_bytes` and fails closed
on byte-size overflow or an undersized backing object. Empty language shapes are
valid metadata, but they are not passed to `ElementCount` or a native launch.

Primitive and operation crates use `ByteCount` at the resource boundary rather
than exposing a second byte unit. Lowering converts scratch extents, shared
memory, private bounds, and fault buffers to `ByteCount`; it converts checked
per-element work and stage totals to `FlopCount`. `recipe_language::PrimitiveKernel::work`
and `recipe_primitives` lowering retain the exact FLOP count in graph digests,
stage resource bounds, and final `CalculationTask.work`.

The operation materializers (`bayes`, binary metrics, convolution, K-means,
KNN, pooling, tree, materialization, and the other operation modules) compute
workspace formulas in raw element counts, use checked products and sums, and
multiply by the declared word width before constructing `ByteCount`. Their
requirements and materializers both carry a `ByteCount` workspace limit and
compare the exact emitted total to that limit. A product overflow is reported
as the operation's workspace-arithmetic error; a mismatch between the formula
and emitted graph is a workspace-formula error; a total larger than the limit is
a workspace-limit error. `WorkspaceValue::bytes` intentionally returns `None`
for an `F32Elements` value because an element count cannot become bytes without
the dtype and layout context.

Metric and fault paths use the same fixed-width rule throughout the facade:
an `I32` or `F32` scalar occupies `ByteCount::new(4)`, and a metric must have
one element. The core plan, program validation, artifact validation, host
backend, and native executor all repeat that exact typed contract at their own
boundary rather than passing an untyped size.

## Planning and finalized address contracts

### Reservations and capacity accounting

`core/src/plan.rs` defines `EXACT_USER_RESERVATION` as
`ByteCount::new(1_000_000_000)`. `ReservationEvidence::required_bytes` returns
that exact count for every RAM or disk device and for a GPU with one or more
display connectors, and returns `ByteCount::ZERO` for the explicit zero-display
GPU exemption. Reservation validation requires one entry per device, matching
evidence kind, and the exact byte value.

`CapacityLedger::validate` sums reservation, runtime overhead, fragmentation,
safety headroom, and Recipe-usable bytes with `ByteCount::checked_add`. A
`None` is `ValidationCode::CapacityOverflow`; a representable sum larger than
`total` is `ValidationCode::CapacityOverflow` as well. Every ledger property
must be measured or explicitly overridden. Preparation computes
`total.checked_sub(reservation)` for the initial usable ceiling and fails with
an invalid-reservation error when the reservation does not fit.

Native realization observes live availability once per stabilized candidate.
`account_live_capacity` caps live bytes at the immutable initial capacity,
computes runtime overhead with checked subtraction, then computes usable bytes
with checked subtraction of the held headroom. A live counter below the required
headroom is a capacity mismatch, not a fallback to another unit or capacity
source.

### Draft values, images, and offsets

`core/src/schedule.rs` uses `ByteCount` for every value, transfer, image, arena
object, and resource-manifest size. It uses `ByteOffset` for object-relative,
image-relative, and finalized arena-relative addresses. It uses `FlopCount` for
`CalculationTask.work` and `Nanoseconds` for task windows and the final
`makespan`.

During lowering, `planner/src/planner.rs` packs each device's external inputs
and checked fault flags into one init image. The running image offset is a
`ByteCount`; each member receives `ByteOffset::new(offset.get())`, and each
addition is checked. The image is at least four bytes so the image value itself
can carry the required int32 contract. Planner-generated arena objects use a
16-byte `ByteCount` alignment and an initial zero `ByteOffset`. Empty storage
values are excluded from persistent resource peaks, but init images and metric
fault values are separately required to be nonempty where their contracts say
so.

Core plan validation uses `ByteOffset::checked_end` to prove that values fit
inside objects and images, to detect overlapping alias ranges, and to prove that
final allocations fit inside an arena layout. It checks scalar alignment against
`DType::byte_width()`, adds the finalized object offset and object-relative
offset with checked raw `u64` arithmetic, then wraps the result as a
`ByteOffset`. Address overflow is reported as `ValidationCode::AddressOverflow`;
out-of-bounds and live-overlap failures retain their more specific validation
codes. When an alias-range end cannot be represented, the conservative
`binding_ranges_overlap` helper treats the ranges as overlapping rather than
allowing an invalid alias.

### Planner resource peaks

The planner converts every scheduled `ScheduleWindow` into byte usage events for
pinned staging and scratch. It adds starts with `ByteCount::checked_add`, removes
ends with `checked_sub`, and records the maximum live `ByteCount` per device.
An addition overflow is a planner arithmetic failure; a subtraction underflow is
an invalid draft because an end event cannot remove bytes that were never live.
Arena size plus staging or scratch peaks is another checked byte addition and is
compared directly with `recipe_usable`.

## Scheduling

### Route cost

`scheduler/src/route.rs::shortest_route` charges each directed hop with
`transfer_time_ceil(transfer.bytes, link.bandwidth.value)`. It sums hop
nanoseconds with checked `u64` arithmetic, uses deterministic link-ID
tie-breaking, and returns a `Route.duration` in `Nanoseconds`. A same-device
route has no links and a one-nanosecond duration. A multi-link route is a
store-and-forward estimate only; the planner lowers it into one dependency-
chained one-hop task per link before an executor sees it.

### Static schedule

`scheduler/src/static_schedule.rs` computes task duration as follows:

- calculations call `calculation_time_ceil(calculation.work,
  capability.rate.value)`;
- internal one-hop transfers call `transfer_time_ceil(transfer.bytes,
  link.bandwidth.value)`;
- external admission and egress call `transfer_time_ceil` with the endpoint
  device transfer rate; and
- metric readback is assigned one nanosecond because it is a specialized
  four-byte transfer whose schedule slot is the unit of ordering.

The returned timing is clamped to at least one nanosecond for a schedulable
task. The scheduler then stores dependency ends, phase floors, resource
reservations, critical-path lengths, and `ScheduleWindow` boundaries as
`Nanoseconds`. Every `start.checked_add(duration)` and critical-path addition
maps `None` to `ScheduleErrorKind::ArithmeticOverflow`. Queue, completion,
compute, transfer, external-transfer, and half-duplex resources reserve the same
half-open nanosecond windows, so rates and byte sizes influence contention only
through the typed timing equation.

Transfer lane claims are derived from the nonzero `TransferLaneCount` range. A
half-duplex link also reserves its shared capacity resource; a full-duplex pair
uses separate resources. The persisted claim list must exactly cover the route's
directed links or the external endpoint's lane group.

### Arena packing

`scheduler/src/arena.rs::pack_arenas` groups `ArenaObject`s by device and uses
their `ScheduleWindow` lifetimes to decide whether bytes may be reused. Candidate
ends are computed with `ByteOffset::checked_end`, then aligned with
`ByteCount::checked_align_up`. The lowest legal aligned offset is selected with
stable ordering. Every allocation end, maximum layout size, and capacity
comparison remains a checked `ByteCount` operation. Invalid alignment, empty
lifetime, end overflow, or a layout over `recipe_usable` becomes a scheduler
error. The resulting `ArenaLayout.size` is exactly the maximum allocation end,
not an arbitrary capacity reservation.

## Preparation, execution, and accounting boundaries

The preparation crate carries `CapacityLedger` snapshots through stabilization,
hashes every byte field, and compares all retained snapshots by exact
`ByteCount` equality. Its diagnostics report byte deltas using the raw `get`
values but retain the ledger field name and unit context.

Host RAM and disk arenas accept a nonzero `ByteCount`, convert it explicitly to
`usize`, and retain the exact byte length. Host copy submission validates both
`offset.checked_add(bytes.get())` and the arena's `ByteCount` length before it
queues a copy. Metric readback and fault channels use the exact four-byte
`ByteCount::new(4)` contract.

The executor and native bridge convert finalized byte sizes and offsets to host
or driver integer widths with `try_from`; a value that does not fit is a
capacity or integer-conversion error. CUDA and HSA free-memory counters are
wrapped as `ByteCount` only after the driver value fits `u64`. Device pointer
addition, arena ranges, init image lengths, and staged transfer lengths are
checked again at the backend boundary. The bridge never substitutes a smaller
or wider raw integer when a typed contract cannot be represented.

The ingest and remote layers retain the same contract. Init-image packing uses
`ByteOffset::checked_end` and rejects empty or overflowing members. Remote
`InitBegin`, chunk, and `InitEnd` messages carry the finalized `ByteCount`; the
worker accepts a chunk only when its checked end is within that image and only
completes when the exact received byte count matches. Cross-machine transfer
specifications preserve the one-hop route, byte count, lane claim, and schedule
start nanoseconds established by the planner.

The top-level native training tuning path in `src/training.rs` also consumes the
same dimensions. It sums measured host transfer lane counts, chooses a staging
size from the maximum tensor `ByteCount`, totals external input and output
bytes with checked additions, and derives the largest graph `FlopCount`. It
selects the slowest measured `FlopsPerSecond` and transfer `BytesPerSecond`,
uses the two timing helpers, and converts the resulting `Nanoseconds` explicitly
to `Duration` for the watchdog. This is runtime policy derived from the profile;
it does not introduce a second rate or time representation.

## Workspace callsite map

The following map is the complete cross-crate ownership pattern for the unit
types. It is useful when tracing a value from a public declaration to a native
operation:

| Workspace area | Unit values carried or produced | Boundary responsibility |
| --- | --- | --- |
| `core` | All eight unit types | Defines dimensions, constructors, checked arithmetic, timing, and validation structures |
| `language` | `ByteCount`, `ByteOffset`, `FlopCount` | Converts shape and layout element metadata to typed storage and work; OGDL stores byte sizes as decimal text |
| `primitives` | `ByteCount`, `FlopCount`, `ElementCount` through kernel templates | Lowers operation formulas into stage storage and exact work bounds with checked products |
| `kernel` | `ElementCount`, `ByteCount`, `FlopCount` | Emits launch element counts, validates ABI ranges, and carries lowered work/resource contracts |
| `ops` | `ByteCount` | Computes exact workspace requirements and enforces caller limits |
| `probe` and `native-probe` | `ByteCount`, `BytesPerSecond`, `FlopsPerSecond`, `TransferLaneCount`, `ElementCount`, `FlopCount` | Parses seed values, measures real resources, validates evidence, and constructs measured profiles |
| `transport` and `cluster` | `ByteCount`, `BytesPerSecond`, `TransferLaneCount` | Encodes peer probe frames, authenticates directional evidence, and remaps measured network pairs |
| `planner` and `scheduler` | `ByteCount`, `ByteOffset`, `FlopCount`, `Nanoseconds` | Lowers copies and images, computes work and transfer durations, packs arenas, and ranks makespans |
| `prepare` | `ByteCount` | Holds the exact reservation and post-realization capacity ledger, including stable byte snapshots |
| `host`, `executor`, and `native-executor` | `ByteCount`, `ByteOffset`, `ElementCount` | Rechecks finalized ranges, performs explicit host/driver integer conversions, and submits ABI arguments |
| `remote` and `ingest` | `ByteCount`, `ByteOffset`, `Nanoseconds` | Carries exact image sizes, chunk ranges, cross-machine transfer starts, and packed input manifests |
| `training` and the top-level facade | `ByteCount`, `FlopCount`, `BytesPerSecond`, `FlopsPerSecond`, `Nanoseconds`, `TransferLaneCount` | Derives staging and watchdog policy from the same measured profile without changing unit semantics |

No row introduces a competing wrapper or a raw-byte substitute for a value that
already has a unit type. Raw integers appear only while parsing a schema,
hashing canonical content, crossing a driver or host ABI, or performing a
checked intermediate calculation. The next owning layer wraps the result again
or rejects it before it can enter the authoritative graph, profile, plan, or
runtime state.

## Error propagation by stage

The unit layer reports only dimensional construction and arithmetic failures.
Each stage maps those failures into its own domain without inventing a fallback:

- `core::scalar::IndexSpace` and static buffer constructors return
  `UnitError` directly; language and probe callers add graph or benchmark
  context to the same message.
- Seed, local probe, native probe, and transport constructors map zero rates,
  zero lanes, and arithmetic failures to contract or benchmark errors.
- Profile decode maps invalid rate, lane, or FLOP values to codec errors, then
  profile validation reports unavailable, unmeasured, wrong-kind, or resource
  mismatches.
- Route and static scheduling map timing overflow to
  `ScheduleErrorKind::ArithmeticOverflow`; invalid endpoints, routes, or empty
  lane groups remain their specific transfer or capability errors.
- Arena packing maps invalid alignment and checked end failures to scheduler
  arithmetic errors. Core finalized-plan validation instead emits precise
  `AllocationMisaligned`, `AllocationOutOfBounds`, `ValueBindingOutOfBounds`,
  `LiveAllocationOverlap`, or `AddressOverflow` validation codes.
- Planner byte-product, image, peak-usage, and total-capacity failures become
  `PlannerErrorKind::ArithmeticOverflow`, `InvalidDraft`, `CandidateInfeasible`,
  or `InvalidCapacity` according to the violated contract.
- Preparation and native execution turn a reservation subtraction failure or a
  live counter below headroom into an invalid reservation or capacity mismatch;
  they never reduce the requested bytes.

This split is important: a zero `ByteCount` can be valid at the unit layer but
invalid for an init image, arena, copy, or metric contract; a zero rate is
invalid before it reaches a timing equation; and a checked `None` can mean
arithmetic overflow, an invalid draft, or a validation error depending on the
stage that owns the surrounding invariant.

## Worked dimensional examples

These examples describe the integer behavior of the implementation, not a
separate test harness:

```text
ByteCount::new(13).checked_align_up(ByteCount::new(8))
    = ByteCount::new(16)

ByteOffset::new(12).checked_end(ByteCount::new(5))
    = Some(ByteCount::new(17))

transfer_time_ceil(ByteCount::new(1), BytesPerSecond::new(3)?)
    = Nanoseconds::new(333_333_334)

calculation_time_ceil(FlopCount::new(2), FlopsPerSecond::new(2)?)
    = Nanoseconds::new(1_000_000_000)

IndexSpace::new(vec![ElementCount::new(2)?, ElementCount::new(3)?])
    .elements()
    = ElementCount::new(6)
```

The alignment example rounds a byte count upward, while the timing examples
first scale seconds to nanoseconds and then apply an integer ceiling. The
`ByteOffset` result is a `ByteCount` end, not another offset. An index-space
product is an element count, and only a dtype-aware access constructor turns it
into bytes. Replacing any of these calls with arithmetic on raw integers would
lose the dimensional check that makes the call sites auditable.

The boundary cases are equally intentional. `ByteCount::new(0)` and
`Nanoseconds::new(0)` are valid identities. `ByteCount::new(u64::MAX)` can be
held but cannot be increased, aligned upward when the alignment needs an extra
mask, or extended by a nonzero `ByteCount`. `ByteOffset::new(u64::MAX)` can end
with a zero-size range but not with one additional byte. A zero work numerator
has zero theoretical time; a scheduled task or image that must be nonempty
receives its one-nanosecond or one-byte policy from the owning stage. A zero
rate, zero lane group, or zero native element count is rejected at construction
because no caller can assign a useful policy after the dimension has become
undefined.

## Practical rules for extending the system

1. Keep dimensional values typed at public boundaries. Do not add a raw `u64`
   field when the value is a byte count, byte offset, rate, FLOP count, or
   schedule time.
2. Use `get` only at a deliberate representation boundary, then use a checked
   conversion or checked arithmetic before constructing the destination type.
3. Use `ByteOffset::checked_end` for half-open byte ranges. Do not add offsets
   as if they were sizes, and do not compare a raw offset against a size without
   making the origin explicit.
4. Use the timing helpers for byte/rate and FLOP/rate equations. Do not divide
   in floating point or duplicate the nanoseconds-per-second scaling.
5. Preserve the nonzero constructors for rates, transfer lanes, and native
   element counts. Add a caller-level validation error when a zero byte count,
   zero duration, or empty image is forbidden by that caller's contract.
6. Serialize the raw integer only in the owning schema, and reconstruct through
   the appropriate constructor on decode. Include the typed value in canonical
   identities through `get` so a dimensional change changes the identity.

The result is one coherent flow: probe measures typed capacities, bandwidths,
FLOP rates, and lane counts; topology and discovery mark which measured values
are schedulable; planning turns element and FLOP work into byte resources and
typed tasks; scheduling converts work and bytes into conservative nanosecond
windows; arena placement and capacity accounting use checked byte ranges; and
execution consumes the same finalized byte and offset contracts without a unit
reinterpretation.

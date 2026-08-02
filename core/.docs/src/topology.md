# Topology model and measured execution pipeline

## Scope and authority

`core/src/topology.rs` is the dependency-free structural model for Recipe's
machines, logical nodes, storage devices, and directed transport links. It is
part of `recipe-core`, which also owns the typed IDs, units, identity wrappers,
and validation error types used by this model. The module has no driver handles,
filesystem paths, sockets, benchmark code, scheduler state, or serializer. It
stores caller-owned values and exposes explicit validators and lookups.

The topology is the authoritative graph for placement and routing. A
`DiscoveryProfile` in `core/src/discovery.rs` is the companion capability
snapshot for the same graph: it says which objects are currently available,
which asynchronous queues and capabilities they provide, and which measured
properties may be used by planning. The topology owns structural facts and
the discovery profile owns operational capability facts. Neither one may be
silently reconstructed from the other.

The accepted production lifecycle is:

```text
topology/contract.toml seed estimates
        -> bare-metal discovery and bounded measurement
        -> measured Topology + DiscoveryProfile + origins
        -> optional cluster assembly and deterministic ID remap
        -> prepare, planner, scheduler, realization, Finalize
        -> immutable init -> loop -> exit execution
```

The seed values only size bounded probe workloads. They are not a deployment
inventory and never supply production schedule rates.

## Core types

All IDs below are distinct `u64` newtypes from `core/src/ids.rs`. Their fields
are private, so an ID is copied and compared as a typed value rather than as an
unlabelled integer. `Label` from `core/src/identity.rs` rejects empty or
whitespace-only text. `TopologyIdentity` is a caller-computed nonzero digest
wrapper. The topology module deliberately does not derive or implement a
content hash, because each producer owns the identity recipe for its stage.

| Type | Fields and purpose | Data owner |
| --- | --- | --- |
| `Property<T>` | `value` plus explicit `PropertyProvenance` (`Estimated`, `Measured`, or `Override`) | The probe or an explicitly authorized profile producer |
| `Machine` | `MachineId` and unique display `Label` | Discovery or cluster assembly |
| `Node` | `NodeId`, `NodeRole`, owning `MachineId`, and its `DeviceId` list | Run ownership assembly |
| `Device` | `DeviceId`, machine, `DeviceKind`, capacity, transfer rate, and optional calculation rate | Topology producer from measured hardware |
| `DirectedLink` | One direction of a physical pair: IDs, transport kind, endpoints, bandwidth, lane count, and duplex capacity resource | Probe or cluster assembly |
| `Topology` | Nonzero identity and ordered vectors of machines, nodes, devices, and links | The profile producer; consumers borrow it immutably |

### Provenance and units

`Property<T>::is_schedulable` accepts only `Measured` and `Override`. An
`Estimated` property can be useful while sizing a probe, but it cannot reach a
production schedule. `BytesPerSecond`, `FlopsPerSecond`, and
`TransferLaneCount` are nonzero unit wrappers. `ByteCount` may be zero because
zero is a meaningful capacity or arithmetic identity. Unit constructors reject
zero rates and zero lane counts before topology validation is called.

`DeviceKind` has three values:

* `GpuMemory` is the only calculation target. It must carry a calculation rate.
* `Ram` is host or remote memory and cannot carry a calculation rate.
* `Disk` is storage and cannot carry a calculation rate.

`Device::is_calculation_target` is intentionally just the `GpuMemory` kind
check. Capability availability and asynchronous limits come from discovery,
not from this structural predicate.

`TransportKind::required_duplex` is the single structural mapping from a
transport family to its operation mode:

* `Sata` and `Wlan` require `DuplexMode::Half`.
* `Memory`, `Pcie`, `Sas`, `Nvme`, and `Ethernet` require `DuplexMode::Full`.

`DuplexMode` describes contention semantics, not merely the existence of a
reverse edge. A bidirectional transport always has two `DirectedLink` records.
Full duplex gives the directions separate capacity resources. Half duplex gives
both directions one shared capacity resource.

### Directed link identity

`DirectedLink::transport` identifies the physical pair. `id` identifies one
direction. `from` and `to` are storage-device endpoints, not machine or node
IDs. `bandwidth` and `maximum_inflight_transfers` are directional properties,
so the reverse record may have different measured values and lane capacity.
`capacity_resource` is the scheduler's contention key. The module-level
documentation also records the concurrency rule: a value greater than one is
the effective measured per-transfer rate while that many same-direction
transfers overlap; discovery must report one until concurrent measurement
establishes a larger value.

## Structural validation

`Topology::validate` aggregates every error it can observe in one pass through
`Validator` and returns `ValidationErrors`. Callers should report the complete
set rather than treating the first failure as a substitute for validation. The
checks are:

1. `identity` must not be zero (`InvalidIdentity`).
2. Machine IDs and machine names are unique (`DuplicateId` and
   `DuplicateName`). The `Label` type has already enforced nonempty names.
3. Every device ID is unique, its machine is known, and its kind agrees with
   `calculation_rate`. A GPU storage device requires a rate; RAM and disk must
   not have one (`UnknownReference`, `DuplicateId`, `MissingRequiredObject`,
   or `WrongKind`).
4. Every configured machine has at least one device. This is a storage-domain
   requirement, not a requirement that every node list be nonempty.
5. Node IDs are unique. Each node's machine is known. Its device list has no
   duplicate IDs, every referenced device exists, and every device belongs to
   the node's machine (`DuplicateId`, `UnknownReference`, and
   `MachineMismatch`).
6. A device may be owned by only one node. Every topology device must appear in
   an owner list (`DeviceOwnedMultipleTimes` and `UnownedDevice`).
7. There must be exactly one `NodeRole::Master`. Zero masters yields
   `MissingMaster`; more than one yields `MultipleMasters`. Other nodes may be
   workers.
8. Link IDs are unique; both endpoints exist and differ. Transport and capacity
   resource IDs are nonzero. The link's duplex mode must equal
   `TransportKind::required_duplex` (`InvalidRoute`, `UnknownReference`,
   `InvalidTransport`, and `InvalidDuplex`).
9. Each transport ID has exactly two directed edges. The two edges must be
   endpoint reverses with the same transport kind and duplex mode. Half-duplex
   edges must share their capacity resource; full-duplex edges must not. A
   capacity resource cannot be shared by two different transports.

The validator does not infer missing objects, repair endpoint order, synthesize
reverse links, or rewrite provenance. It is an invariant report, not a repair
operation.

### Scheduling-property validation

`Topology::validate_scheduling_properties` is deliberately separate from
structural validation. It requires capacity and transfer rate on every device,
an optional GPU calculation rate, and bandwidth and lane count on every link to
be schedulable (`Measured` or `Override`). It reports
`ValidationCode::UnmeasuredProperty` for an estimate. This is the guard used by
the scheduler, planner, probe profile codec, and runtime projections before
they consume measured timing or contention values.

### Lookup and duplex helpers

`machine`, `device`, and `link` perform stable-ID lookups over the owned vectors
and return borrowed records. They do not validate the surrounding topology.

`duplex_mode(first, second)` returns a mode only when the two link IDs name the
reverse edges of one transport, have matching duplex modes, and satisfy the
resource relationship for that mode. It returns `None` for unknown IDs,
non-reverse edges, asymmetric metadata, or an invalid resource pairing. The
helper is useful for a consumer comparing two links; `validate` remains the
authoritative whole-topology check.

`validate_route(source, destination, route)` walks a route in order. Every link
must exist and start at the current device; the final endpoint must equal the
requested destination. A same-device empty route is the only accepted empty
route. A transfer between distinct devices needs at least one link. Failures
are `UnknownReference`, `RouteEndpointMismatch`, or `InvalidRoute`. The method
does not find a route and does not enforce a shortest path. Route selection is a
planner or scheduler responsibility.

## Discovery companion and data ownership

`DiscoveryProfile` stores a `DiscoveryIdentity`, the `TopologyIdentity` it
describes, and one `DiscoveredDevice` and `DiscoveredLink` for every topology
object. A discovered device records availability, maximum submission queues,
total capacity, transfer capability, and optional GPU calculation capability.
A discovered link records availability, bandwidth, lane capacity, and
asynchronous-submission support.

`DiscoveryProfile::validate` requires a nonzero discovery identity and exact
topology identity equality. It then requires a unique, available entry for
every topology device and link, nonzero queue and capability limits, and
schedulable measured or overridden properties. GPU topology devices need a
calculation capability with asynchronous submission, nonzero concurrent-task
capacity, power-of-two nonzero subgroup lanes, a workgroup limit containing at
least one subgroup, and nonzero shared memory. Non-GPU devices must not expose
calculation capability. Discovered link properties must equal the corresponding
topology direction. Missing, duplicated, unknown, unavailable, unmeasured, or
unsupported entries are reported with the shared `ValidationCode` categories.

The split is intentional:

* `Topology` answers what exists, how it is owned, and how bytes can move.
* `DiscoveryProfile` answers whether it is available now and what measured
  limits the scheduler may use.
* `MeasuredOrigins` and peer evidence retain the host strings that the core
  topology intentionally omits.
* Plans and bundles retain only typed IDs and stage identities. Runtime code
  must use the profile and identity checks instead of rediscovering hardware.

## Probe and measured-profile construction

### Seed contract and inspection

`topology/contract.toml` is parsed by `probe/src/seed.rs` into a
`SeedContract`. It contains decimal byte/rate estimates, an exact per-storage
reservation, all-discovery and all-benchmark gates, cache invalidation facets,
and transport duplex/async declarations. The parser rejects wrong schema or
kind, non-exact reservation size, disabled gates, missing invalidation facets,
invalid transport definitions, and unknown fields. It deliberately has no
machine or device inventory fields.

`ProbeEngine::inspect` obtains a `HostInventory`, normalizes RAM, storage, and
network keys, and requires RAM and mounted storage to exist. It rejects unknown
host-memory references and non-asynchronous storage. It obtains an exhaustive
GPU inventory, requires at least one GPU, sorts GPU keys, and rejects unknown
host-memory references, non-asynchronous devices, or zero queue/task limits.
Peer sessions are described and sorted by session ID. Peer machine identities
must differ from the local machine, local memory and interface keys must exist,
transport and duplex metadata must agree with the local interface, and
submission must be asynchronous. Discovery never proceeds with a partial or
ambiguous inventory.

The concrete Linux `LocalSystemDiscovery` reads machine fingerprint fields from
procfs, sysfs, and `/etc`; enumerates NUMA RAM domains or a single procfs
fallback; groups mounted partitions by their physical block device; classifies
NVMe, SAS, SATA, Ethernet, and WLAN transport kinds; and selects a writable
benchmark root without changing the declared identity. `NativeGpuProbe`
enumerates every visible CUDA and ROCr device, retains exact backend target and
toolchain identities, and marks diagnostic single-backend modes non-exhaustive
so they cannot produce an accepted profile.

### Bounded measurements

`BenchmarkPlans::from_seed` converts theoretical estimates into bounded byte
buffers, eight iterations, and a two-second maximum. The buffer is clamped to
the configured four-KiB to 64-MiB bound. The host benchmark measures RAM copy
rate and storage write/read rates plus available filesystem capacity. Native
GPU benchmarking measures VRAM capacity, GPU FLOPs, GPU memory rate, and both
host-to-device and device-to-host rates. A peer session supplies authenticated
directional network evidence, remote-memory capacity/rate, and outbound and
inbound throughput.

Before topology construction, every capacity and rate must have exactly
`PropertyProvenance::Measured`. Peer evidence must use the current protocol,
nonzero distinct endpoint machine/profile digests, the execution mode implied by
duplex, the expected total byte count and sample count, a bounded elapsed time,
ordered sample statistics, and a rate derived from elapsed time. Missing
directions return `IncompletePeerMeasurement`; inconsistent evidence returns a
benchmark failure. A failed attempt cannot masquerade as a measured property.

### Probe identities and stable IDs

The cache identity is a canonical SHA-256 digest over the seed schema and all
seed values, invalidation facets and transport declarations, the local machine
fingerprint, every RAM/storage/network descriptor, every GPU descriptor and
toolchain identity, and every peer descriptor. It uses the
`recipe-probe-cache-v7` domain and profile schema 7. A changed machine, device,
driver, runtime ABI, firmware, link, toolchain, descriptor, or seed produces a
different cache identity.

`assign_ids` is deterministic for one normalized inspection. It starts device
IDs at one, assigns local machine ID one, assigns peer machine IDs in sorted
peer-session order starting at two, and assigns devices in RAM, storage, GPU,
then peer-RAM order. `build_origins` preserves each mapping with machine
fingerprints and scoped RAM, storage, and GPU keys. These origins are the
bridge from opaque core IDs back to exact discovery identities.

The standalone probe computes two profile digests with
`build_profile_digest`: `recipe-topology-v6` for `TopologyIdentity` and
`recipe-discovery-v6` for `DiscoveryIdentity`. Each digest includes the cache
digest and all measured values, including authenticated peer evidence. The
digest is nonzero by construction of the accepted profile and is stored in its
typed identity wrapper.

### Object construction map

`build_topology` creates the structural graph from the normalized inventory and
measurements. The mapping is exact:

| Source | Topology object and properties |
| --- | --- |
| Each local RAM domain | `DeviceKind::Ram`, measured capacity and RAM transfer rate |
| Each local storage domain | `DeviceKind::Disk`, measured capacity, and the lower of measured read and write rates as its general transfer rate |
| Each local GPU descriptor | `DeviceKind::GpuMemory`, measured capacity and memory rate, plus measured calculation rate |
| Each peer measurement | Remote `DeviceKind::Ram` with measured remote capacity and memory rate |
| Local RAM to storage | Two links: RAM to disk uses measured write rate and write lanes; disk to RAM uses measured read rate and read lanes |
| Local RAM to GPU | Two links using the measured host-to-device/device-to-host rates and descriptor lane limits |
| Local RAM to peer RAM | Two links using measured outbound/inbound rates and descriptor lane limits |

`LinkBuilder::push_pair` allocates one transport ID and two link IDs per pair.
It allocates one resource for half duplex and one resource per direction for
full duplex. Link IDs, transport IDs, and resource IDs start at one in the
normalized construction order. The local machine is the sole master node and
owns all local RAM, storage, and GPU devices. Each peer receives a worker node
owning its remote RAM device.

`build_discovery` creates an available capability entry for each device. RAM
uses its measured transfer rate and domain lane count. Storage uses the lower
of read/write rates and lane counts for its bidirectional transfer capability.
GPU uses measured memory and calculation rates, descriptor queue/task and
workgroup limits, and the descriptor's overlap flag. Peer RAM uses measured
remote-memory rate and the lower of outbound/inbound lane counts. Link entries
copy the topology direction's measured properties and the corresponding
asynchronous flag. The profile is rejected if topology, scheduling properties,
or discovery validation fails, then passed through the canonical profile codec
validator before it is returned.

## Origin-based realization and cache use

`MeasuredProfile::resolve_local_inventory` reopens a current machine by exact
retained origins. It first validates the persisted profile and requires
exhaustive current GPU enumeration. The current machine fingerprint must equal
one retained machine origin. Current RAM, storage, and GPU key sets must match
the profile exactly, with no missing, unexpected, duplicate, ordinal,
capacity, product-name, or performance-similarity fallback. Storage and GPU
descriptors must still point to retained current RAM keys; GPU targets must
still equal the measured discovery target. Any mismatch is an invalid profile
and requires a fresh probe.

`ProbeEngine::load_or_probe_and_store` computes this discovery-only cache
identity before loading. `ExplicitPathProfileCache` accepts only an absolute
caller-selected path in a canonical, user-owned private directory. It rejects
symlinks, insecure permissions, ownership changes, oversize files, checksum or
codec failures, and stale cache identity. Store is atomic and no-replace; an
existing identical profile is idempotent, while a different profile at the same
path is an error.

## Canonical profile serialization

`MeasuredProfileCodec` is the versioned binary serialization boundary for a
validated measured profile. Encoding calls `validate_profile` first. The byte
stream is:

```text
16-byte RECIPEPROFILE magic
u32 codec schema (7)
u32 profile schema (7)
u32 cache schema and 32-byte cache digest
origins
benchmark metadata
peer benchmark records and evidence
topology
discovery profile
32-byte SHA-256 payload checksum
```

Topology encoding includes its identity, ordered machines, ordered nodes with
roles and device lists, ordered devices with kind and property provenance, and
ordered directed links with transport, duplex, endpoints, measured bandwidth,
lane count, and capacity resource. Discovery encoding includes its identity,
the topology identity it claims, every device capability, every calculation
target and limit, and every link capability.

Decode enforces the 256-MiB profile limit, one-million-item limit, one-MiB label
limit, magic, checksum, codec schema, valid UTF-8 labels, nonzero unit rates and
lane counts, valid enum tags, and complete payload consumption. It reconstructs
the typed profile and runs the same full validation before returning it.

`validate_profile` adds invariants that are not solely structural:

* profile and cache schemas are current and the cache digest is nonzero;
* all benchmark plans are bounded and use the seed schema;
* origin records exactly cover machines and each RAM, storage, and GPU device,
  with matching machine hostnames, unique stable fingerprints, correct device
  kinds, and keys unique within a machine;
* every persisted property is `Measured`, not merely schedulable by override;
* origin vectors, topology vectors, node device lists, discovery vectors, and
  peer records are in strict canonical ID or session order;
* topology and discovery capacities, transfer rates, calculation rates,
  bandwidths, and lane counts match exactly;
* every cross-machine transport is one opposing pair, and every peer benchmark
  matches one authenticated endpoint pair, directional rates, and duplex
  execution evidence.

The codec therefore preserves both the measured values used for timing and the
identity provenance needed to prove that those values belong to the current
hardware.

## Cluster assembly

The `cluster` crate combines complete per-machine measured profiles. A
`ClusterConfiguration` contains only a master key, member keys and endpoint
addresses, and undirected network-pair membership. It deliberately contains no
hardware, rate, capacity, lane, transport, or topology values. At least two
members are required, member keys and addresses are unique, the master belongs
to membership, network pairs name distinct members, and the pair graph is
connected.

`MemberProfileIdentity::derive` canonical-encodes a per-machine profile, hashes
that profile for the endpoint profile identity, and hashes the retained stable
machine ID for the endpoint machine identity. The expected endpoint, cache,
topology, and discovery identities are stored in each `MemberSpec`. Assembly
rejects missing, duplicate, stale, unconfigured, multi-machine, duplicate-name,
or duplicate-profile submissions.

`MeasuredNetworkPair::from_probe` is the only accepted numeric source for an
inter-machine link. It requires distinct members, a bounded benchmark,
asynchronous transport, the duplex required by Ethernet or WLAN, measured
remote-memory properties, measured outbound and inbound rates, and authenticated
session-bound directional evidence. It rejects storage or PCIe as an
inter-machine network transport.

`resolve_pair` resolves each endpoint's RAM gateway through its retained RAM
origin key and checks the measured capacity and transfer rate against the exact
member profile device. It canonicalizes pair order by member key, swaps
directional rates and lanes when the evidence was reported from the second
member, and retains the session, link, remote machine, interface, driver,
firmware, benchmark, and evidence strings for identity hashing.

`remap_profile` then allocates fresh global IDs in sorted member order. It
remaps every member machine, node, device, link, transport, capacity resource,
origin, and discovery record. Only the configured master member retains the
master node role; all other member nodes become workers. It appends one measured
two-direction network pair per configured pair, with shared or independent
capacity resources according to duplex. The resulting topology and discovery
profile share the new topology identity.

Three cluster digests use the `recipe-cluster-*` domains and cluster schema:
topology identity, discovery identity, and cache/profile identity. They include
member expectations and addresses, common benchmark metadata, all measured
network properties and provenance, and all retained peer evidence and network
origin strings. Member profile origin data is bound through each member's
canonical profile and identity digests. `ClusterProfileCodec` delegates binary encoding
to `MeasuredProfileCodec` and additionally requires at least two machines and a
connected inter-machine transport graph. No input order may affect IDs or any
resulting identity.

## Prepare boundary

`prepare::Preparer::prepare_program` accepts a `MeasuredProfile`, not a seed
contract. It validates the complete profile, asks the realizer for one exact
reservation per topology device, and validates that reservation against device
kinds and the required user headroom. It creates an optimistic planning
capacity by subtracting only the exact reservation from each discovery total.
That capacity is not a runtime observation.

The planner consumes the topology and discovery profile to enumerate candidates.
The realizer and native candidate factory retain both exact values and their
`TopologyIdentity`/`DiscoveryIdentity`. Each maximum-concurrency warm pass and
capacity snapshot must use the same identities. A changed topology or discovery
profile rejects the candidate. Final capacity snapshots must be complete and
stable over the configured tail; the final arena packing is performed against
the stabilized snapshot. This preserves topology immutability while allowing
runtime capacity accounting to remain a separate measured ledger.

## Planner consumption

`planner::plan_program_candidates` begins by validating the graph, topology,
scheduling properties, discovery, reservations, and capacity. It uses topology
kind plus discovery calculation capability to form legal placement choices. A
non-GPU or undiscovered GPU cannot receive a calculation. `candidate_identity`
and the later draft hash include both topology and discovery digests, so a
different measured machine graph cannot reuse a candidate identity.

When a value needs a copy on another device, the planner enumerates simple
directed routes from each eligible resident source. It validates every candidate
with `Topology::validate_route`, trial-schedules it, and selects by completion
time, makespan, source device/value, and link-ID path. A selected path longer
than one link is lowered into a dependency chain of one transfer task per link,
with resident intermediate values. The scheduler never receives an unexpanded
multi-hop executor task. Same-device copies use an empty route.

External outputs are selected from non-overwritten resident copies and trial
scheduled as exit transfers. Queue and completion resources are compacted per
device against discovery's measured maximum queue count. Arena objects are
packed per topology device and checked against the planning capacity. The
resulting `DraftPlan` records only typed device and link IDs plus topology and
discovery identities, then validates the complete structural, route, lane,
contention, resource, and lifecycle contract.

## Scheduler consumption

`scheduler::schedule` and `shortest_route` both validate topology and measured
scheduling properties before doing any timing. `schedule` also validates the
discovery profile. `shortest_route` applies Dijkstra-style deterministic search
over directed links. Each hop costs `ceil(bytes / link.bandwidth)` in integer
nanoseconds, with lexicographic link-ID tie-breaking. A source equal to the
destination has an empty route and a one-nanosecond structural duration. The
scheduler's base equations are therefore the measured-device FLOP rate and
measured-link bandwidth, with no seed estimate fallback.

For calculations, the scheduler requires a known GPU storage device and an
available calculation capability. It charges measured FLOPs, reserves the
measured maximum-concurrent-task compute lanes, and reserves a no-overlap
resource when discovery says transfer and calculation cannot overlap.

For an internal transfer, an empty route may be filled only when the chosen
shortest path is one direct link. A multi-link route is an error at this layer;
the planner must have expanded it. The route is validated against endpoints,
duration uses that direction's measured bandwidth, every measured link lane is
represented as a resource group, and half-duplex links add their shared
capacity resource. Both endpoint devices contribute no-overlap resources when
their discovery capability forbids overlap. External admission or egress has
no internal route, uses the endpoint discovery transfer rate and lanes, and
claims one external lane.

Critical-path list scheduling reserves queue and completion slots, compute
lanes, link lanes, external lanes, half-duplex opposing-direction resources,
and no-overlap resources. Two directions contend exactly when their links share
the same half-duplex capacity resource. A full-duplex pair can overlap because
its resources differ. The completed schedule persists the exact transfer lane
claims that the core `DraftPlan` validator later checks against topology lane
counts and routes.

`pack_arenas` is the other scheduler entry point that consumes topology. It
rejects unknown-device arena objects, invalid alignment or lifetime, arithmetic
overflow, missing capacity entries, and arena sizes above the measured usable
capacity. It emits deterministic per-device layouts; topology supplies the
complete device set and capacity ledger supplies the post-reservation limit.

## Identity handoff and runtime projections

`DraftPlan`, `RealizationProfile`, and `FinalizedBundle` each retain a
`TopologyIdentity` and a `DiscoveryIdentity`. Their validators compare those
identities to the borrowed current topology/profile and reject identity changes.
The final bundle contains no mutable topology and no route reconstruction API;
it contains finalized value locations, one-hop transfer tasks, lane claims,
arena layouts, and the identities that prove which measured graph produced
them.

The executor worker projection validates the current topology and scheduling
properties, compares the bundle topology identity, verifies worker machine/node
ownership, and checks every local device, arena, reservation, init image, task,
route, and lane claim against the same topology. Remote provisioning performs
the same identity and one-hop route checks before turning cross-machine links
into transport claims. Native candidate sessions retain topology and discovery
identities in their immutable initial-capacity snapshot and reject any later
observation from a different profile. Runtime backends therefore consume
authoritative IDs and finalized claims, never a fresh best-effort discovery.

## Failure map

| Boundary | Topology-related failure | Result |
| --- | --- | --- |
| Seed parsing | Wrong schema, disabled gate, missing invalidation facet, invalid transport, unknown field | `ProbeError::Contract` |
| Host/GPU/peer inspection | Partial inventory, duplicate stable key, unknown RAM/interface, non-async capability, non-exhaustive GPU set | `ProbeError::Discovery` or `IncompleteGpuEnumeration` |
| Measurement | Zero work, missing direction, non-measured property, inconsistent authenticated sample evidence | `ProbeError::Benchmark`, `MissingMeasurement`, or `IncompletePeerMeasurement` |
| Topology construction | Broken ownership, device kind, reverse-link, transport, duplex, or identity invariant | `ProbeError::InvalidProfile` wrapping `ValidationErrors` |
| Profile codec/cache | Bad magic/schema/checksum, noncanonical order, stale identity, origin mismatch, topology/discovery mismatch, unsafe cache path | `ProbeError::Cache` or `InvalidProfile` |
| Cluster submission | Missing/stale member, duplicate pair, unauthenticated evidence, unmatched RAM origin, disconnected graph, remap exhaustion | `ClusterError` |
| Prepare | Invalid measured profile, reservation or capacity mismatch, changed topology/discovery identity, unstable post-warm capacity | `PrepareError` or candidate rejection |
| Planner | No measured GPU, no directed route, route trial infeasible, queue/capacity shortage, candidate identity collision | `PlannerError` |
| Scheduler | Invalid topology/discovery, unknown placement, unsupported capability, multi-hop executor route, no route, resource or arithmetic overflow | `ScheduleError` |
| Draft/finalize/runtime | Unknown device/link, invalid lane claim, half-duplex overlap, topology or discovery identity mismatch | `ValidationErrors`, projection error, or backend rejection |

Every boundary fails closed. No layer removes an unavailable device, replaces a
measured value with a seed estimate, invents a reverse edge, retries through an
alternate route, or mutates the topology after planning.

## Data-ownership rules for future changes

* Add structural facts to `Topology` only when they are required to identify
  ownership or a physical route. Keep measured limits and asynchronous
  capability in `DiscoveryProfile`.
* Preserve directional values. Do not collapse opposing rates or lane counts
  into one symmetric field. A single physical pair has two directed records.
* Preserve `PropertyProvenance` through every transformation. Probe-produced
  profiles are exclusively measured; an override is an explicit producer choice,
  never an implicit fallback.
* Resolve current hardware through retained machine and domain origins. Never
  match by ordinal, product name, capacity, or rate similarity.
* Treat topology and discovery identities as content boundaries. Any changed
  object, measurement, capability, origin, or authenticated transport evidence
  must produce a new profile identity and a new preparation pipeline.
* Keep route decomposition in the planner. Scheduler and executor-visible
  transfers are one directed-link hops with explicit intermediate values.
* Run `Topology::validate`, `validate_scheduling_properties`, and
  `DiscoveryProfile::validate` before consuming timing, lanes, capacity, or
  ownership. These functions report errors; they do not repair data.

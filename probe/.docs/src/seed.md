# Probe seed contract

`recipe-probe` has one deliberately narrow configuration boundary. The
checked-in [`topology/contract.toml`](../../../topology/contract.toml) is a
theoretical seed for the first bounded benchmark pass. `probe/src/seed.rs`
parses that text into a typed [`SeedContract`](../../src/seed.rs), and
`ProbeEngine` uses only the resulting bounds while it discovers the current
machine and measures it. The seed never supplies a machine list, a GPU list,
a production rate, or a scheduler value. Those values come from the current
discovery and completed measurements.

The two files are a pair:

* `topology/contract.toml` is the human-maintained source of the seed values,
  required policy declarations, cache invalidation facets, and transport
  shape declarations.
* `probe/src/seed.rs` is the executable contract. It owns schema and kind
  checks, the small TOML-like parser, conversion to typed units, the allowed
  field set, and all semantic rejection rules.

The parser is intentionally not a general TOML dependency. A valid contract
must be explicit and complete, so an omitted, disabled, ambiguous, or unknown
field fails before discovery or benchmarking begins.

## Contract boundary and type graph

The public types are re-exported by `probe/src/lib.rs`:

| Type | Meaning | Runtime role |
| --- | --- | --- |
| `CONTRACT_SCHEMA: u32` | Seed schema, currently `1` | Also recorded in `BenchmarkMetadata.seed_schema` and checked when a measured profile is encoded or decoded. |
| `CONTRACT_KIND: &str` | Required kind, `probe-seed-estimates` | Prevents another TOML document from being accepted as this contract. |
| `SeedEstimates` | Eleven decimal capacity, transfer, and calculation estimates | Four fields size first-pass plans. All eleven fields participate in cache identity. |
| `ProbePolicy` | Seven required discovery, benchmark, and preparation gates | Parsed and required to be all `true`; no later engine branch varies on them. |
| `IdentityFacet` | One of seven cache invalidation dimensions | Stored as a complete `BTreeSet` and included in the cache digest. |
| `SeedDuplex` | `Full` or `Half` transport sharing model | Retained for each transport and included in the cache digest. |
| `TransportSeed` | A validated `Label` and its duplex mode | Describes the transport shapes the contract permits; it does not enumerate live links. |
| `SeedContract` | Complete immutable typed seed | Passed by shared reference to inspection, planning, identity hashing, and profile metadata. |

`SeedEstimates` has this exact field order in the Rust value and in the seed
hash:

| Rust field | TOML key | Unit | Used for a bounded plan? |
| --- | --- | --- | --- |
| `ethernet_rate` | `estimates.ethernet_bytes_per_second` | `BytesPerSecond` | Yes, network plan buffer divided by `8`. |
| `disk_capacity` | `estimates.disk_bytes` | `ByteCount` | Yes, storage plan buffer divided by `16_384`. |
| `sata_rate` | `estimates.sata_bytes_per_second` | `BytesPerSecond` | No current plan formula; still parsed and hashed. |
| `gpu_memory_capacity` | `estimates.gpu_vram_bytes` | `ByteCount` | Yes, GPU plan buffer divided by `1024`. |
| `pcie_rate` | `estimates.pcie_bytes_per_second` | `BytesPerSecond` | No current plan formula; still parsed and hashed. |
| `gpu_calculation_rate` | `estimates.gpu_flops_per_second` | `FlopsPerSecond` | No current plan formula; still parsed and hashed. |
| `gpu_memory_rate` | `estimates.gpu_transfer_bytes_per_second` | `BytesPerSecond` | No current plan formula; still parsed and hashed. |
| `ram_capacity` | `estimates.ram_bytes` | `ByteCount` | Yes, RAM plan buffer divided by `1024`. |
| `ram_rate` | `estimates.ddr_bytes_per_second` | `BytesPerSecond` | No current plan formula; still parsed and hashed. |
| `cpu_reference_rate` | `estimates.cpu_reference_flops_per_second` | `FlopsPerSecond` | No current plan formula; still parsed and hashed. |
| `ram_transfer_rate` | `estimates.ram_transfer_bytes_per_second` | `BytesPerSecond` | No current plan formula; still parsed and hashed. |

This distinction is intentional. A field can remain part of the normative
seed and the cache identity without being allowed to masquerade as a measured
property. The current engine does not use the unused estimate fields to fill a
topology or discovery property.

The unit types come from `recipe-core`:

* `ByteCount` is an exact `u64` decimal byte count. Its constructor accepts
  zero, so a zero capacity can be represented in the typed seed. Individual
  discovery and benchmark paths reject zero when it cannot support their live
  operation.
* `BytesPerSecond` and `FlopsPerSecond` are nonzero `u64` rates. Their
  constructors return `UnitError::ZeroRate` for zero, which the seed parser
  converts to a contract error.
* `Label` rejects empty and whitespace-only strings. It does not normalize a
  nonempty value, so transport names are retained exactly as supplied.
* `BTreeSet<IdentityFacet>` and the `BTreeMap` used while parsing provide
  deterministic ordering for validation and hashing.

`SeedContract` contains no `MachineFingerprint`, `HostInventory`,
`GpuDescriptor`, path, driver, runtime, firmware, or measured rate. Those are
discovery values in `probe::model`, not contract input.

## Checked-in contract

The embedded file is the default used by `recipe probe` when `--contract` is
not supplied. Its complete scalar content is:

```toml
schema = 1
kind = "probe-seed-estimates"

[estimates]
ethernet_bytes_per_second = 125_000_000
disk_bytes = 1_000_000_000_000
sata_bytes_per_second = 600_000_000
gpu_vram_bytes = 12_000_000_000
pcie_bytes_per_second = 16_000_000_000
gpu_flops_per_second = 380_000_000_000
gpu_transfer_bytes_per_second = 432_000_000_000
ram_bytes = 48_000_000_000
ddr_bytes_per_second = 90_000_000_000
cpu_reference_flops_per_second = 150_000_000_000
ram_transfer_bytes_per_second = 90_000_000_000

[reservation]
bytes_per_storage_device = 1_000_000_000

[probe]
command = "recipe probe"
environment = "bare-metal"
discover_machines = true
discover_devices = true
discover_links = true
benchmark_capacity = true
benchmark_calculation = true
benchmark_transfer = true
require_measured_profile_for_prepare = true
invalidate_on = [
    "machine",
    "device",
    "driver",
    "runtime-abi",
    "firmware",
    "link",
    "artifact-toolchain",
]
```

The file declares six transport tables. The parser retains only the name and
duplex after validating the other two fields:

| Tables | `directions` | `duplex` | `issue` |
| --- | --- | --- | --- |
| `transport.pcie`, `transport.nvme`, `transport.sas`, `transport.ethernet` | `"both"` | `"full"` | `"async"` |
| `transport.sata`, `transport.wlan` | `"both"` | `"half"` | `"async"` |

The table names are collected from a `BTreeSet`, so the resulting
`SeedContract.transports` order is lexicographic (`ethernet`, `nvme`, `pcie`,
`sas`, `sata`, `wlan`) regardless of the order of the tables in the file.

The comments in the checked-in file state the key intent: these are human
specification starting values, they size the first bounded pass, and they are
not deployment inventory or measured production performance.

## Loading paths and callers

There are two production entry points that load the seed, plus the direct
`SeedContract` API:

```text
recipe probe [options]
  -> parse_probe_options
  -> require_bare_metal
  -> SeedContract::read(--contract PATH)
       or SeedContract::parse(include_str!("../topology/contract.toml"))
  -> private state and host discovery
  -> native configuration and NativeGpuProbe
  -> ProbeEngine::current_cache_identity / load_or_probe_and_store
  -> measured profile and active native receipt

training or inference preparation without an active receipt
  -> current_native_inputs
  -> embedded SeedContract::parse(topology/contract.toml)
  -> discovery-only ProbeEngine::current_cache_identity
  -> exact identity-named measured profile lookup
```

### `SeedContract::read`

`read(path)` performs one filesystem operation, `std::fs::read_to_string`,
then delegates to `parse`. A read failure becomes
`ProbeError::Io { operation: "read", path, message }`. Reading never changes
the file or the parsed value.

### `SeedContract::parse`

`parse(input)` is the complete semantic constructor:

1. `parse_assignments` converts text to a `BTreeMap<String, Assignment>`.
2. The schema and kind are required and checked against the two constants.
3. All eleven estimate values are converted to typed units.
4. The storage reservation is converted and must equal exactly
   `ByteCount::new(1_000_000_000)`.
5. The probe command and environment must be exactly `recipe probe` and
   `bare-metal`.
6. All seven policy booleans are parsed and every one must be `true`.
7. `probe.invalidate_on` is parsed into the known seven-facet set, and the set
   must equal the complete set.
8. Transport names are discovered from assignment keys. Every discovered
   transport must provide valid directions, issue, and duplex values.
9. `reject_unknown_fields` rejects every key outside the exact contract and
   the three allowed fields under each transport table.
10. The typed `SeedContract` is returned only after every check succeeds.

The order matters for diagnostics. For example, a missing required key is
reported before an unrelated unknown field is considered. Unknown fields are
still rejected even when they look like hardware or production-rate
inventory, because those values must come from discovery or measurement.

## The strict text parser

`parse_assignments` implements only the syntax needed by the checked-in file:

* Input is processed line by line with one-based line numbers.
* `strip_comment` removes `#` and the rest of a line unless the scanner is
  between quote characters. The scanner only tracks quote toggles; it does not
  implement escaped quotes.
* Empty or comment-only lines are skipped.
* A line whose trimmed form starts with `[` and ends with `]` changes the
  current table section. An empty table name is a contract error.
* Every other nonempty line must contain `=`. The key and value are trimmed;
  an empty key is an error. A section prefix is joined with `.` to form the
  full key.
* A value beginning with `[` and not containing `]` starts a multiline array.
  Subsequent trimmed lines are appended with a space until a `]` appears. An
  unfinished array reports `unterminated array` at its starting line.
* `insert_assignment` rejects an already present full key, including a key
  repeated through two table declarations.

The value helpers are intentionally narrow:

* `parse_string` accepts only a value that begins and ends with `"`; it strips
  those outer quotes and does not unescape content.
* `parse_string_array` requires outer brackets, splits on commas, trims each
  entry, ignores empty entries, and parses each remaining item as a quoted
  string. An empty array therefore becomes an empty vector and fails whichever
  semantic completeness check consumes it.
* `parse_u64` removes underscore characters before parsing an unsigned decimal
  integer. Negative text, nonnumeric text, and overflow are errors. This means
  separators are accepted as notation, but no unit suffixes or floating-point
  values are accepted.
* `parse_u32` performs a checked conversion from the parsed `u64`.
* `boolean` accepts exactly the unquoted tokens `true` and `false`.
* `bytes` wraps the unsigned integer in `ByteCount`.
* `rate` and `flop_rate` call the nonzero unit constructors and convert a zero
  rate into `ProbeError::Contract`.

`reject_unknown_fields` recognizes the exact top-level, `estimates`,
`reservation`, and `probe` keys listed in the source. A transport key is
allowed only when it has the form `transport.<nonempty name>.<directions|duplex|issue>`;
transport names containing a dot are rejected by this check. Any other key
gets a line-numbered error explaining that machine, device, link, and
production-rate inventory is discovered automatically.

## Semantic invariants and errors

The parser uses `ProbeError::Contract { line: Option<usize>, message }` for
contract failures. Its display form is `contract line N: ...` when a source
line is known, or `contract: ...` otherwise. The contract-level invariants
are:

| Invariant | Rejection |
| --- | --- |
| `schema` is `1` | `unsupported schema N; expected 1` |
| `kind` is `probe-seed-estimates` | Kind mismatch with both expected and received text |
| Every estimate key is present | `missing required key ...` |
| Every rate is nonzero | The typed rate constructor reports `rate must be nonzero` |
| Storage reservation is exactly one billion bytes | `reservation.bytes_per_storage_device must be exactly 1_000_000_000` |
| Command and environment identify bare-metal `recipe probe` | One contract error for the pair |
| All seven policy gates are true | `all discovery, benchmark, and measured-profile gates must be enabled` |
| Every invalidation facet is present and known | Unknown facet or incomplete facet set error |
| At least one transport exists | `at least one transport seed is required` |
| Every transport is bidirectional and asynchronous | Transport-specific directions/issue error |
| Every transport duplex is `full` or `half` | Transport-specific invalid-duplex error |
| Every transport label is nonempty | `Label::new` error converted to a contract error |
| No duplicate or unknown field exists | Line-numbered duplicate or unsupported-field error |

The parser does not silently repair bad input. It does not infer a missing
transport, choose a later value after a malformed value, or fall back to a
source default when `--contract PATH` was explicitly selected.

`ProbeError` also distinguishes later stages:

* `Io` is used for seed file reads and other filesystem operations.
* `Discovery` reports incomplete or contradictory current inventory.
* `IncompleteGpuEnumeration` reports a native inventory that did not prove
  exhaustive coverage.
* `Benchmark` reports failed or inconsistent measurements.
* `MissingMeasurement` reports a property that is still estimated rather than
  measured.
* `IncompletePeerMeasurement` identifies a peer and missing direction.
* `InvalidProfile` reports topology, discovery, identity, or codec validation
  failures.
* `Cache` reports profile ownership, path, encoding, identity, or atomic-store
  failures.

The seed parser itself returns only `Contract` or `Io`; it does not turn
discovery or benchmark failures into contract failures.

## Native configuration is a separate source

The seed and the native probe configuration are intentionally different
records. `src/cli.rs::native_config` builds `NativeProbeConfig` after seed
loading and host discovery. Its values come from command-line path overrides
or fixed source defaults, not from `topology/contract.toml`:

| Native value | Source and role |
| --- | --- |
| PCI sysfs root | Fixed `/sys/bus/pci/devices`; used for vendor preflight and GPU PCI identity. |
| CUDA Driver candidates | `--cuda-driver` replaces the fixed candidate list; selected only when NVIDIA hardware exists. |
| ROCr/HSA candidates | `--hsa-runtime` replaces the fixed candidate list; selected only when AMD hardware exists. |
| LLVM `opt`, `llc` | Explicit `--llvm-opt`/`--llvm-llc` or fixed candidate lists; required. |
| LLVM `ld.lld`, NVIDIA `ptxas` | Explicit or fixed candidates; optional until their backend is actually used. |
| PTX ISA | Source value `74`. |
| HSA code-object version | Source value `6`. |
| Kernel release | Label `auto-pinned-local-tools-and-benchmark-v3`. |
| FMA chain length | Source value `64`, used by the Recipe-owned native FLOP benchmark. |
| Scratch parent | Private state root `scratch`, verified as an absolute private directory. |
| `host_memory_key` | First discovered RAM domain, not a seed estimate. |

`NativeProbeConfig` is identity-bearing native state. Its libraries, tools,
digests, target ABI values, and scratch policy are captured in the active
native receipt. They must not be added to `SeedEstimates`, and the seed parser
does not inspect or hash them.

### Current field-consumption audit

The current checkout has a small, auditable set of seed consumers:

| Seed field | Consumers | Not a consumer |
| --- | --- | --- |
| `schema` | Parser check, cache digest, `BenchmarkMetadata.seed_schema`, profile codec check | It does not select a profile version by itself. |
| `ethernet_rate` | Network bounded-plan buffer and cache digest | It does not become a network bandwidth property. |
| `disk_capacity` | Storage bounded-plan buffer and cache digest | It does not become disk capacity or reserve storage. |
| `gpu_memory_capacity` | GPU bounded-plan buffer and cache digest | It does not become GPU memory capacity. |
| `ram_capacity` | RAM bounded-plan buffer and cache digest | It does not become RAM capacity. |
| Other seven estimates | Parser typed value and cache digest | No current plan, topology, discovery, or scheduler path reads them. |
| `reservation_per_storage_device` | Parser exact-value check and cache digest | No current engine or local benchmark reserves this amount. |
| `policy` | Parser completeness check | No `ProbeEngine` branch reads a policy flag after parsing. |
| `invalidation` | Parser exact-set check and cache digest | It is not a partial invalidation selector. |
| `transports` | Parser validation and cache digest | The engine does not use the declarations to enumerate or classify live links. |

The checked-in transport tables therefore document the contract shape and
identity boundary, while `LocalSystemDiscovery` classifies actual mounted
storage and network interfaces and `NativeGpuProbe` supplies actual GPU link
metadata. The seed cannot cause a live SATA, PCIe, Ethernet, or WLAN object to
be invented.

The `probe` CLI options are parsed as strict option/value pairs. `--contract`
and `--profile` are single-use; library options are repeatable and preserve
candidate order; missing values, duplicate single-use options, unknown options,
and non-UTF-8 option names fail before `run_probe` starts.

## Bounded plan derivation

`ProbeEngine::BenchmarkPlans::from_seed` is the only current consumer of seed
estimates for benchmark sizing. It creates four `BoundedBenchmarkPlan` values:

```text
RAM:     seed.estimates.ram_capacity.get()         / 1024
storage: seed.estimates.disk_capacity.get()        / 16_384
GPU:     seed.estimates.gpu_memory_capacity.get()  / 1024
network: seed.estimates.ethernet_rate.get()        / 8
```

`bounded_plan` then applies the hard bounds shared by every category:

```text
MINIMUM_BUFFER_BYTES       = 4 * 1024
MAXIMUM_BUFFER_BYTES       = 64 * 1024 * 1024
DEFAULT_ITERATIONS         = 8
MAXIMUM_BENCHMARK_DURATION = 2 seconds
buffer_bytes = suggested_bytes.clamp(4 KiB, 64 MiB)
```

With the checked-in estimates, integer division produces these initial
buffers. None of them requires clamping:

| Plan | Calculation | Buffer |
| --- | --- | ---: |
| RAM | `48_000_000_000 / 1_024` | `46_875_000` bytes |
| storage | `1_000_000_000_000 / 16_384` | `61_035_156` bytes |
| GPU | `12_000_000_000 / 1_024` | `11_718_750` bytes |
| network | `125_000_000 / 8` | `15_625_000` bytes |

The seed cannot create an unbounded plan. `BoundedBenchmarkPlan::is_bounded`
requires a nonzero buffer, nonzero iteration count, and nonzero duration;
profile codec validation repeats that requirement when a profile is loaded.

The arithmetic constants are plan policy, not TOML fields. The estimates are
the only user-editable inputs to this first-pass sizing formula. A value below
the minimum still schedules a 4 KiB pass; a value above the maximum still
schedules at most 64 MiB. Integer division rounds down before clamping.

### Host measurements

`LocalHostBenchmarks` receives the plan and the discovered domain:

* RAM allocates a source and destination of exactly `plan.buffer_bytes`, copies
  up to `plan.iterations` times while the two-second deadline remains, and
  derives bytes per second from total bytes and elapsed nanoseconds.
* Storage creates one private temporary file below the discovered writable
  benchmark root, performs bounded `sync_data` writes, performs bounded reads,
  measures each direction independently, obtains available filesystem bytes
  with `statvfs`, and removes the temporary file on drop.
* A zero iteration count or zero elapsed time is a benchmark error. Overflow
  in total-byte or rate accounting is also an error.

The capacity values returned by these paths are current domain values or
available filesystem capacity, wrapped in `PropertyProvenance::Measured`.
The seed's capacity estimates are not copied into the profile.

### Native GPU measurements

`NativeGpuProbe` is the `GpuDiscovery` and `GpuBenchmarkIo` implementation
passed to the engine by the CLI. It constructs both CUDA Driver and ROCr/HSA
backends, marks the combined inventory exhaustive, and re-discovers an exact
descriptor before each benchmark. A diagnostic single-backend probe is
explicitly non-exhaustive and therefore cannot produce an accepted profile.

For each exact GPU descriptor, the native backend uses the same bounded plan
for:

1. host-to-device transfer;
2. device-to-host transfer, including byte-for-byte verification;
3. device-to-device transfer, including verification; and
4. a Recipe-owned dependent f32 FMA kernel, lowered with the configured target
   and toolchain, realized as cubin or HSACO, launched through the native
   driver, and verified to produce a finite changed value.

`native-probe/src/benchmark.rs::time_bounded` stops after the plan iteration
count or deadline. Transfer and FLOP rates are calculated from completed work
and elapsed time with checked `u128` arithmetic, then converted to nonzero
typed rates. Native benchmark capacity, calculation rate, memory rate, and
both host directions are all returned as measured properties.

The GPU helper also caps the compute buffer at 4 MiB and aligns it to f32
width. This is a native-kernel implementation limit inside the already bounded
plan; it does not change the seed or the recorded plan metadata. A buffer that
cannot hold one f32 is a benchmark error.

### Peer measurements

The command-line probe supplies an empty peer slice, so a normal local
`recipe probe` measures no remote session. The engine still supports explicit
`PeerSession` callers. Each peer gets a fresh `PeerBenchmarkControl` with the
network plan's two-second absolute deadline. `benchmark_controlled` converts a
failed attempt to structured failure evidence, and `into_measurement` refuses
to let a failed attempt become a measurement.

Peer validation requires measured remote-memory capacity and rate, measured
outbound and inbound rates, current protocol schema, nonzero distinct local
and remote endpoint machine/profile digests, duplex-consistent execution
(simultaneous for full duplex and serialized for half duplex), exact total
bytes, exact sample count, ordered timing statistics, and a duration-derived
rate. Missing directions return `IncompletePeerMeasurement`; contradictory
evidence returns `Benchmark`.

## `ProbeEngine` call graph

The engine methods and their responsibilities are:

```text
ProbeEngine::new
  stores HostDiscovery, GpuDiscovery, HostBenchmarkIo, GpuBenchmarkIo references

ProbeEngine::current_cache_identity(seed, peers)
  -> inspect(seed, peers)
       -> discover_host
       -> normalize_host
       -> discover_all GPUs
       -> validate_gpu_inventory
       -> describe each peer and sort by session_id
       -> validate_peer_descriptors
       -> build_cache_identity(seed, host, gpus, peers)

ProbeEngine::probe(seed, peers)
  -> inspect
  -> BenchmarkPlans::from_seed
  -> benchmark every RAM domain, storage domain, GPU, and peer
  -> validate every returned measurement and evidence
  -> build_profile_digest("recipe-topology-v6", cache_identity, measurements)
  -> build_topology
  -> build_origins
  -> build_profile_digest("recipe-discovery-v6", cache_identity, measurements)
  -> build_discovery
  -> validate topology, scheduling properties, and discovery
  -> construct MeasuredProfile with BenchmarkMetadata(seed.schema)
  -> codec::validate_profile

ProbeEngine::load_or_probe_and_store(seed, peers, cache)
  -> current_cache_identity
  -> cache.load(identity)
  -> return the validated profile on an exact hit
  -> otherwise probe_and_store

ProbeEngine::probe_and_store(seed, peers, cache)
  -> probe
  -> cache.store(fully validated profile)
```

### Inspection invariants

`inspect` is discovery-only and therefore safe for cache-key computation. It
sorts RAM, storage, network, and GPU descriptors by stable key before any
identity or ID assignment. It requires:

* at least one RAM domain and one mounted storage domain;
* unique RAM, storage, network, and GPU labels;
* each storage domain to reference a known RAM key and expose asynchronous
  submission;
* an exhaustive GPU inventory with at least one GPU;
* each GPU to reference known host RAM, expose asynchronous submission, and
  report nonzero queue and concurrent-task limits;
* unique peer session and peer machine identities;
* each peer to refer to a known local RAM and interface, agree with that
  interface's transport and duplex, and expose asynchronous submission.

Discovery contradictions are `ProbeError::Discovery`; a non-exhaustive native
GPU result is `ProbeError::IncompleteGpuEnumeration`.

## Profile construction and measured state

The profile is not a copy of the seed. `ProbeEngine::probe` converts completed
measurements into the two authoritative runtime descriptions:

* `Topology` receives measured device capacities, transfer rates, optional GPU
  calculation rates, directional link bandwidths, and measured inflight lane
  counts. RAM, disk, GPU memory, and peer RAM become typed devices. Full duplex
  links receive independent capacity resources; half duplex reverse links share
  one resource.
* `DiscoveryProfile` receives measured device capabilities, queue and
  concurrency limits, asynchronous submission flags, calculation targets, and
  link capabilities. It is validated against the topology.
* `MeasuredOrigins` retains machine fingerprints and stable RAM, storage, and
  GPU discovery keys because the core topology intentionally stores opaque IDs
  rather than host strings.
* `BenchmarkMetadata` records the seed schema and the four exact bounded plans
  used for the profile. It records the plan provenance, not the theoretical
  estimate values as production properties.
* `peer_benchmarks` retains authenticated endpoint and timing evidence for each
  measured peer.

`assign_ids` is deterministic after inspection normalization. Local machine ID
is `1`; peer machine IDs start at `2` in sorted session order. Device IDs are
assigned in RAM, storage, GPU, then peer-RAM order. The IDs are profile-local,
while `MeasuredOrigins` maps them back to stable discovery keys.

Before returning, the engine runs `Topology::validate`,
`Topology::validate_scheduling_properties`, `DiscoveryProfile::validate`, and
`codec::validate_profile`. The scheduling validation rejects `Estimated`
capacities, rates, and lane counts. The codec validation additionally requires
profile schema `7`, cache schema `7`, a nonzero cache digest, bounded metadata,
canonical ordering, complete origins, topology/discovery agreement, peer
evidence consistency, and measured provenance throughout the profile.

This is the core safety rule:

```text
seed estimate -> bounded work only
completed benchmark -> PropertyProvenance::Measured
measured profile -> scheduling and native preparation
```

No path from `SeedEstimates` enters `recipe-scheduler` or a production plan.
The scheduler receives the measured topology and discovery profile through the
prepare boundary.

## Cache identity and seed identity implications

`build_cache_identity` starts a canonical SHA-256 digest with domain
`recipe-probe-cache-v7` and profile schema `7`, then calls `hash_seed` before
hashing current discovery. `hash_seed` includes, in order:

1. `seed.schema`;
2. all eleven estimate values in `SeedEstimates` field order;
3. `reservation_per_storage_device`;
4. every `IdentityFacet` in the sorted `BTreeSet`; and
5. every transport name and its retained `SeedDuplex` in sorted transport
   order.

The parser fixes policy booleans to all `true`, and fixes each transport's
`directions` and `issue` to `both` and `async`. Those values are not separately
hashed because no valid parsed contract can vary them. Transport names and
duplex are hashed, so adding, removing, renaming, or changing a transport
changes the cache identity. Changing any estimate changes the identity even if
the changed field is not currently used by `BenchmarkPlans::from_seed`.

After the seed, the digest includes current machine fingerprint fields, every
normalized RAM, storage, network, GPU, and peer descriptor, native target and
toolchain identity, queue and concurrency limits, link identities, duplex, and
asynchronous capabilities. A changed machine, device, driver, runtime ABI,
firmware, link, toolchain, stable discovery key, or seed produces a different
identity. The complete invalidation facet set documents that intended cache
boundary; it is not a user-controlled partial invalidation switch.

The profile's topology and discovery digests are separate canonical digests
(`recipe-topology-v6` and `recipe-discovery-v6`) over the cache identity plus
the actual measured values and peer evidence. A new benchmark result therefore
changes the published profile identities even when discovery is unchanged.

`MeasuredProfile::is_cache_valid_for` accepts only profile schema `7` and exact
`CacheIdentity` equality. `ExplicitPathProfileCache` requires an absolute,
private, owned regular file in a canonical private directory, validates the
encoded profile and checksum, and rejects a stale identity. There is no
newest-file, ordinal, or estimate-similarity fallback.

## Stable identity and current-inventory resolution

The seed participates in cache identity, but it is not used to identify a
device. `build_origins` records exact current discovery values:

* local and peer machine fingerprints;
* RAM domain keys, including peer remote-memory keys;
* storage domain keys; and
* GPU descriptor keys.

`MeasuredProfile::resolve_local_inventory` validates the profile, requires an
exhaustive current GPU inventory, matches the current machine by complete
fingerprint, then matches RAM, storage, and GPU domains by scoped stable keys.
It rejects missing or newly visible keys, duplicate current keys, changed
host-memory associations, and changed GPU targets. Capacity, product name,
ordinal, and benchmark similarity are never fallback selectors. A changed
identity requires a fresh `recipe probe`.

This is why a seed capacity cannot accidentally select a different device
whose capacity happens to be similar. The stable origin mapping and the cache
digest both use discovered identity, while the seed only changes bounded work
and the digest namespace.

## End-to-end `recipe probe` behavior

The real CLI path is `recipe probe [OPTIONS]`:

1. `require_bare_metal` rejects container markers, container cgroups, and
   nested PID namespaces before any seed or native work.
2. The explicit `--contract PATH` is read and parsed, or the checked-in
   `topology/contract.toml` is embedded and parsed at compile time.
3. A private state root and `scratch` directory are created or verified.
4. `LocalSystemDiscovery::with_benchmark_roots` discovers the current machine,
   RAM, mounted storage, and network and supplies the state root as a possible
   writable disk benchmark root. The root chooses where a temporary file can
   live; it does not declare disk identity, capacity, transport, or rate.
5. `native_config` selects and hashes native tools and backend candidates, then
   `NativeGpuProbe::new` opens both native discovery backends.
6. `ProbeEngine::current_cache_identity` performs the discovery-only identity
   pass. Unless `--profile PATH` is supplied, the CLI derives
   `state_root/profiles/measured-v<schema>-<digest>.recipe-profile` from this
   identity.
7. `load_or_probe_and_store` loads and validates an exact cache hit. On a miss,
   it runs the bounded RAM, storage, GPU, and peer benchmark path and stores
   only the fully validated measured profile.
8. `ActiveNativeReceipt::capture` pins the profile identity, host RAM origin,
   PCI root, scratch directory, native libraries, compiler tools, PTX and HSA
   ABI values, release, and FMA chain. The receipt is written atomically below
   the private state root.
9. CLI output reports profile path, `validated-cache` or `fresh-measurement`,
   cache/topology/discovery digests, and object counts. It does not print the
   seed estimates, measured rates, native paths, or raw inventory.

A successful output line or zero exit status is not the measurement proof. The
proof is the independently validated profile contents: measured properties,
current identities, topology/discovery agreement, and exact benchmark
evidence.

## The preparation handoff

Training and inference call `with_current_native_preparation`. The handoff
first calls `current_native_inputs`:

* If an active receipt exists, its recorded profile identity and native config
  are reopened and revalidated. The current host must still contain the
  recorded RAM origin.
* If no receipt exists, the embedded seed is parsed and the current discovery
  identity is recomputed with an empty peer list. This resolves the exact
  identity-named profile path; it does not invent a profile or run a hidden
  benchmark.

The preparation layer then loads that exact profile, resolves the current
exhaustive inventory by machine fingerprint and stable RAM, storage, and GPU
keys, and lends matching CUDA/HSA bindings and native target specifications to
the preparation callback. Capacity, product name, ordinal, and benchmark
similarity are never selectors. A missing or newly visible domain, changed GPU
target, changed host memory origin, changed native library/tool digest, or
nonmatching profile identity fails closed and asks for a fresh `recipe probe`.

This keeps the seed out of the runtime state while still allowing the default
CLI to find the profile produced by the same contract. The active receipt pins
the native environment; the measured profile pins the measured machine and
performance; the seed only determines the bounded work and its identity.

## What changing the seed means

Changing a seed value is a contract change, not a way to override hardware:

* changing one of the four plan-driving estimates changes the first benchmark
  buffers and the cache identity;
* changing one of the seven currently unused estimates still changes the cache
  identity, but does not alter the current plan formula;
* changing the reservation value, invalidation facet set, transport name, or
  transport duplex changes the cache identity, and most such edits are rejected
  outright by the parser;
* disabling a discovery, benchmark, or preparation gate is rejected rather than
  selecting a partial probe mode;
* adding a machine, device, link, or production-rate field is rejected because
  live discovery owns those values;
* changing native paths, tools, PTX, HSA, release, or FMA settings belongs in
  native configuration and the active receipt, not in this seed.

After a valid contract change, an old profile is stale by exact digest. The
correct end-to-end action is to run `recipe probe` again, complete the bounded
measurements on the current bare-metal system, validate the new profile, and
let preparation consume that new measured state.

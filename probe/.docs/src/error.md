# Probe error contract

`recipe-probe` exposes one fallible result type for discovery, bounded
measurement, profile construction, cache encoding, cache decoding, and exact
reopening of a measured machine:

```rust
pub type ProbeResult<T> = Result<T, ProbeError>;
```

`ProbeError` is `Clone`, `Debug`, `PartialEq`, and `Eq`, is marked
`#[non_exhaustive]`, and implements `Display` and `std::error::Error`.  The
enum carries the first concrete failure observed at a boundary.  It does not
carry a partial inventory or a partially valid profile.  Every `?` in the
probe pipeline therefore stops the current operation and leaves the caller to
decide whether to report the error or abort the larger operation.

The assigned source of truth is `probe/src/error.rs`.  The construction sites
listed below include the production native adapters in `native-probe`, because
they return the same public `recipe_probe::ProbeError` to `ProbeEngine` and to
native preparation.

## Shape, constructors, and formatting

The variants are:

| Variant | Payload | `Display` form | Meaning |
| --- | --- | --- | --- |
| `Contract` | `line: Option<usize>`, `message: String` | `contract line N: MESSAGE` or `contract: MESSAGE` | The seed contract is not the accepted strict TOML-shaped contract. |
| `Discovery` | `String` | `discovery: MESSAGE` | A required machine, device, link, identity, or runtime surface could not be established. |
| `Benchmark` | `String` | `benchmark: MESSAGE` | A bounded measurement could not be performed, verified, or represented. |
| `IncompleteGpuEnumeration` | none | `GPU discovery did not prove exhaustive enumeration` | A GPU source deliberately or accidentally did not prove that all visible GPUs were returned. |
| `MissingMeasurement` | `String` | `missing measured property: NAME` | A benchmark returned a property whose provenance was not `Measured`. |
| `IncompletePeerMeasurement` | `peer: String`, `direction: &'static str` | `peer PEER has no measured DIRECTION throughput` | A peer result omitted one of the two required directional throughputs. |
| `InvalidProfile` | `String` | `invalid measured profile: MESSAGE` | A newly constructed profile or a current inventory cannot satisfy the measured-profile identity contract. |
| `Cache` | `String` | `profile cache: MESSAGE` | A profile cache path, serialized profile, or cache/profile invariant is invalid. Codec messages are prefixed with `codec:` inside this payload. |
| `Io` | `operation: &'static str`, `path: PathBuf`, `message: String` | `OPERATION PATH: MESSAGE` | An operating-system operation failed at a path. |

`ProbeError::contract(line, message)` and `ProbeError::io(operation, path,
error)` are the only constructors.  `contract` owns the message string and
retains an optional source line.  `io` converts the supplied displayable
source error to text, owns the path, and does not retain the original error as
an `Error::source`.  Consequently `ProbeError::source()` is always `None`.
The textual operation, path, and source message are the complete I/O evidence.

The enum is re-exported by `probe/src/lib.rs`, so callers use
`recipe_probe::ProbeError` and `recipe_probe::ProbeResult` without reaching
into the module.  The `#[non_exhaustive]` marker requires external consumers
to keep a wildcard arm when matching variants.

## Variant ledger

### `Contract`

#### Construction

Only `probe/src/seed.rs` constructs this variant, directly or through the
`ProbeError::contract` helper.  `SeedContract::read` first maps a file read to
`Io`, then passes the text to `SeedContract::parse`; it never turns an I/O
failure into a contract failure.

`SeedContract::parse` is intentionally a strict, finite pipeline.  The
following helpers all return `Contract` on their invalid branch, and their
callers propagate it with `?`:

- `parse_assignments` rejects an empty table name, a line without `key = value`,
  an empty key, and an unterminated multiline array.  `insert_assignment`
  rejects duplicate keys.  These syntax errors retain the input line in
  `Some(line)`.
- `required` reports every absent required key.  Missing-key errors have no
  source line because there is no assignment from which to recover one.
- `parse_string` requires a quoted string and retains the assignment line.
  `parse_string_array` applies the same rule to each comma-separated element.
- `parse_u64` reports an invalid unsigned integer with the assignment line;
  `parse_u32` adds the 32-bit range check and retains the assignment line.
  `bytes` propagates those failures unchanged.  `rate` and `flop_rate` wrap a
  `recipe-core` unit-constructor failure as a no-line contract message named
  for the field.
- `boolean` accepts only the literal `true` or `false` and reports the field
  line otherwise.
- The top-level parser rejects an unsupported schema, the wrong `kind`, a
  reservation other than exactly `1_000_000_000` bytes, a command/environment
  other than `recipe probe` on `bare-metal`, any disabled discovery or
  measurement gate, an incomplete `probe.invalidate_on` set, an unknown cache
  facet, no transport table, non-bidirectional or non-asynchronous transport,
  and an invalid transport duplex.  These semantic failures use no line
  because they describe the complete contract rather than one lexical token.
- A transport name that cannot become a `recipe_core::Label` is converted to a
  no-line contract error with the core validation text.
- `reject_unknown_fields` rejects every key outside the exact contract key set
  and the three allowed fields under each `transport.<name>` table.  It retains
  the offending assignment line and explicitly explains that inventory and
  production rates are discovered, not entered by the user.

#### Propagation and consequence

`SeedContract::parse` is called by the CLI before a `ProbeEngine` exists.  A
contract error stops `recipe probe` before private-state setup, local
discovery, native runtime opening, cache identity calculation, or benchmarking.
No inventory, identity, profile, cache file, or active native receipt is
created from a rejected contract.  A valid `SeedContract` contains only bounds
and policy; it cannot itself supply a production device or rate.

### `Discovery`

`Discovery` means that a required identity or capability was not proven.  It is
used for local host discovery, native GPU discovery, runtime reopening, exact
binding, and structural identity checks.  A missing optional backend is
represented by an empty discovery result only when PCI inspection proves that
the vendor accelerator is absent.  Once matching hardware exists, a missing,
changed, unreadable, or non-exhaustive runtime surface is fatal.

#### Local host construction sites

`probe/src/local.rs` constructs `Discovery` in these paths:

- `LocalSystemDiscovery::discover_host` rejects an empty RAM result after
  `discover_ram` and labels all discovered machine fields.  `discover_ram`
  rejects an invalid NUMA node name and invalid `TransferLaneCount` values.
- `parse_memtotal` rejects a `MemTotal` line with no parseable value, missing
  `MemTotal`, or a KiB-to-byte overflow.  `read_trimmed` rejects an empty
  procfs/sysfs/identity file.  `read_first_available` turns failure of every
  stable-machine-identity candidate into `no stable machine identity is
  available`.
- `discover_storage` rejects an invalid sector count, sector-to-byte
  overflow, inconsistent metadata for one physical identity, and lane-count
  construction failures.  `physical_block_device` rejects a partition whose
  parent chain ends unexpectedly and a resolved block path without both
  `size` and `dev`.  `select_storage_benchmark_root` rejects a physical disk
  for which no mounted directory is on the same device and accepts a bounded
  access probe.  `directory_accepts_probe_file` returns `Discovery` after 64
  colliding temporary names.  Permission-denied and read-only directories are
  a normal `Ok(false)` candidate result, not a discovery error.
- `discover_network` rejects a non-UTF-8 interface name.  Interface files and
  labels propagate their own `Discovery` or `Io` result.
- `label` maps `recipe_core::Label::new` failures to `Discovery` text.

Several discovery reads are intentionally best effort and therefore do not
construct an error: `discover_ram` ignores a failed `read_dir` and falls back
to `/proc/meminfo`; `read_first_available` tries the next identity file after
any error; `join_available` drops unreadable optional firmware fields; and a
network driver symlink that cannot be canonicalized receives the explicit
`network-driver-unreported` identity.  Those choices are observable behavior,
not hidden `Io` conversions.

#### `ProbeEngine` inspection and invariant checks

`ProbeEngine::inspect` in `probe/src/engine.rs` propagates host discovery,
normalization, native GPU discovery, and peer descriptor failures.  It then
constructs `Discovery` for the following failed invariants:

- no GPU descriptor was returned;
- `normalize_host` found no RAM or no mounted storage, duplicate RAM/storage/
  network labels, a storage domain pointing at unknown RAM, or a storage path
  without asynchronous submission;
- `validate_gpu_inventory` found duplicate GPU keys, an unknown host-memory
  key, synchronous submission, zero submission queues, or zero concurrent task
  capacity;
- `validate_peer_descriptors` found duplicate peer sessions or machine
  identities, a peer equal to the local machine, an unknown local RAM or
  interface key, a transport or duplex contradiction with the local interface,
  or a peer without asynchronous submission; and
- `unique_labels` found any duplicate identity in those collections.

These checks run before a cache identity is accepted.  A failed check leaves
the inspection without a `CacheIdentity`, so no cache lookup or measurement can
use a partial inventory.

#### Native backend identity and discovery

The `native-probe` crate returns the same variant through the `GpuDiscovery` and
`GpuBenchmarkIo` implementations used by `ProbeEngine`:

- `identity::selected_library` rejects relative candidates, non-regular
  candidates or targets, and failures to inspect, canonicalize, or hash an
  existing library.  Missing candidates are skipped so an absent vendor can be
  reported as no backend.  `identity::label` maps core label errors to
  `Discovery`; `required_tools` requires the pinned LLVM linker for AMD or
  PTX assembler for NVIDIA and rejects an unknown backend.
- `identity::pci_accelerator_present` rejects a non-absolute PCI root, PCI
  directory enumeration errors, and non-hex vendor/class files.  It returns
  `false` when no matching accelerator exists.  `pci_surface` rejects a
  non-absolute root, a non-directory device path, driver resolution failures,
  and a required identity surface with no readable file.  Optional firmware
  and link files tolerate `NotFound` and `PermissionDenied`; other reads are
  `Io`.
- `NativeGpuProbe::enabled_display_connectors` rejects an origin without a
  canonical PCI BDF, a non-directory PCI root, invalid UTF-8 DRM names,
  invalid connector state, or a connector-count overflow.  Missing `drm`
  returns zero connectors.  `validate_config` rejects a zero FMA chain or a
  relative kernel scratch directory.  `discover_all` rejects duplicate keys
  emitted by the CUDA and HSA backends.
- `CudaBackend::open` rejects hardware with no configured CUDA Driver library,
  a non-UTF-8 library path, driver load failure, or exhaustive-driver
  discovery failure.  `descriptor` rejects PCI identity disagreement,
  compute-capability conversion and target validation failures, invalid labels,
  and lane-count construction failures.  `parse_pci_bus_id` rejects missing or
  malformed domain, bus, device, or function fields and functions above 7.
- `HsaBackend::new` rejects code-object version zero.  `with_runtime` rejects
  AMD hardware/runtime disappearance after initialization, library identity
  changes, and runtime open failures.  `descriptor` rejects a GPU without
  dispatch capability, stable UUID, PCI address, AMD properties, queue limits,
  exact artifact target, wavefront or ISA limits, KFD node, capacity, or
  identity labels.  `exact_target`, `hsa_capacity`, `hsa_target_tail`, and
  `driver_lds_capacity` reject non-AMDGPU or ambiguous targets, absent or
  malformed capacity surfaces, missing AMDGPU triple prefixes, and zero or
  overflowing LDS capacity.
- `bindings::binding_error` deliberately aliases exact-reopening failures to
  `Discovery`.  `with_native_execution_bindings`, `partition_expected`,
  `realize_cuda`, `realize_hsa`, and `require_all_reopened` reject an unsupported
  backend, duplicate expected or reopened keys, a measured backend with no
  matching runtime surface, an absent deployment/context/session/queue, an
  ambiguous HSA host allocator, or any measured GPU that was not reopened.
  `bindings` never substitutes an ordinal, name, capacity, or performance
  match.

Native discovery errors propagate through `NativeGpuProbe::discover_all` to
`ProbeEngine::inspect`, or through `with_native_execution_bindings` to
`NativePreparationError::Probe`.  A failed exact reopen produces no borrowed
CUDA/HSA binding scope and no partially realizable target plan.

### `Benchmark`

`Benchmark` means that a bounded measurement did not produce trustworthy,
representable evidence.  It is terminal for that probe attempt.  The profile is
never assembled from the successful devices before the failed device or peer.

#### Plans, host measurements, and peer measurements in `probe`

- `PeerBenchmarkControl::for_plan` returns `Benchmark` if adding the plan
  duration to `Instant::now()` overflows.
- The default `PeerSession::benchmark_controlled` first checks cancellation and
  deadline, calls the session's `benchmark`, and converts a returned
  `ProbeError` into structured `PeerBenchmarkFailureKind::Transport` evidence.
  `PeerBenchmarkAttempt::into_measurement` is the conversion boundary back to
  `Benchmark`: a `Measured` attempt passes through, while a `Failed` attempt
  becomes `peer benchmark failed during PHASE (KIND): DETAIL`.
- `LocalHostBenchmarks::benchmark_ram` and `benchmark_storage` reject an
  unbounded plan, a buffer that cannot fit `usize`, no timed iterations, zero
  elapsed time, work/rate arithmetic overflow, a rate that cannot fit `u64`,
  or an invalid `BytesPerSecond`.  Storage seek, write, sync, and read failures
  are `Io`; temporary-file creation after 64 collisions is `Benchmark`.
  `filesystem_available_bytes` reports an available-byte multiplication
  overflow as `Benchmark`.  Temporary files are removed by `Drop` even when a
  benchmark returns an error.
- `ProbeEngine::validate_ram_measurement`, `validate_storage_measurement`, and
  `validate_gpu_measurement` call `require_measured`; their provenance failure
  is `MissingMeasurement`, not `Benchmark`.  A benchmark that returns a
  measured property with an invalid value reaches the relevant benchmark
  arithmetic or native adapter and returns `Benchmark`.
- `validate_peer_measurement` emits `IncompletePeerMeasurement` when outbound
  or inbound throughput is `None`.  It emits `Benchmark` for zero or wrong
  authenticated endpoint digests, identical local and remote endpoint
  machines, duplex execution contrary to the descriptor, byte-count or
  duration conversion overflow, inconsistent sample statistics, or a rate
  inconsistent with duration-derived evidence.

#### Native GPU helpers and adapters

`native-probe/src/benchmark.rs` is the shared construction layer for CUDA and
HSA.  `time_bounded` rejects an unbounded plan or no timed work;
`transfer_rate`, `calculation_rate`, and `rate_per_second` reject work/rate
overflow, zero elapsed time, or rates outside the core integer representation;
`compute_buffer_bytes`, `plan_bytes`, and `capacity` reject an unusable or
non-addressable buffer and zero reported capacity; `fma_template` rejects an
invalid index space/access or an empty FMA chain; and `verify_compute_output`
rejects wrong buffer lengths, a non-finite result, or an unchanged first
value.

`NativeGpuProbe::benchmark_gpu` rejects an unbounded plan, multiple native
backends claiming one descriptor, or a descriptor that changed between
discovery and measurement.  The CUDA adapter adds errors for a vanished
backend, a buffer larger than discovered memory, CUDA allocation/copy/event/
launch failures, transfer verification failure, non-cubin or wrong-entry
artifacts, grid/workgroup conversion or overflow, and a copy or kernel that
exceeds its deadline.  `cuda_benchmark_error` converts every
`recipe_cuda::CudaError` to a `Benchmark` message, while
`kernel_benchmark_error` converts every Recipe lowering/build error to the same
variant.

The HSA adapter adds errors for a vanished or ambiguous GPU, no CPU memory
agent, a buffer larger than available capacity, HSA allocation/access/copy/
queue/session failures, transfer verification failure, non-HSACO or mismatched
kernarg metadata, missing user-mode queue, pointer/offset/size conversion
failures, and a timed-out operation.  `hsa_benchmark_error` and
`kernel_benchmark_error` preserve the operation or artifact prefix while
mapping the underlying error to `Benchmark`.

`complete_cuda` and `complete_hsa` do not return at the first deadline while a
native operation is still live.  They continue low-rate cleanup polling so a
pending token does not outlive its buffers, then return `Benchmark` after the
operation completes and the original deadline is known to have been exceeded.
The cleanup error is therefore still a benchmark failure, not a successful
measurement or a resource-release fallback.

#### Transport conversion

`transport/src/probe.rs` does not construct `ProbeError` while a TCP peer
attempt is running.  `TcpPeerSession::run_benchmark` maps each
`TransportError` to `PeerBenchmarkFailure` with a phase and one of
`Cancelled`, `Deadline`, `Identity`, `Integrity`, `Protocol`, or `Transport`.
Transport configuration, frame, sequence, connection, I/O, capacity, and
`TransportError::Probe` failures all become the structured `Transport` kind.
The public `PeerSession::benchmark` method calls
`PeerBenchmarkAttempt::into_measurement`, which is the single boundary that
turns this structured failed attempt into `ProbeError::Benchmark`.

This separation preserves phase and failure kind for controlled callers while
still enforcing the profile invariant that a failed attempt can never be
treated as a measured throughput.

### `IncompleteGpuEnumeration`

`ProbeEngine::inspect` checks `GpuInventory::exhaustive` immediately after
`GpuDiscovery::discover_all`.  A false value returns this unit variant before
the empty-device check, sorting, descriptor validation, peer inspection, or
cache identity calculation.  `NativeGpuProbe::new` sets `exhaustive = true`;
the explicit `cuda_diagnostic` and `hsa_diagnostic` constructors set it to
`false` by design.  A diagnostic can therefore exercise one backend, but it
cannot produce an accepted measured profile.  The consequence is deliberate:
no profile, cache entry, or receipt may claim complete hardware coverage from a
non-exhaustive source.

### `MissingMeasurement`

`probe/src/engine.rs::require_measured` constructs this variant whenever a
property's provenance is `Estimated` or `Override` instead of
`PropertyProvenance::Measured`.  It is reached for RAM capacity and transfer
rate, storage capacity/read/write rates, GPU capacity/calculation/memory and
host-device/device-host rates, peer remote-memory properties, and peer
directional rates.  It is also reached by the corresponding validation
wrappers before topology construction.

The message names the exact property, for example `GPU gpu0 calculation
rate`.  The state consequence is that the measurement set is rejected before
its value can enter a topology, discovery profile, cache identity, or artifact
plan.  Codec validation has a separate persisted-profile check with the same
invariant but returns `Cache` with a `codec:` prefix, because that failure is
about serialized cache data rather than the live benchmark result.

### `IncompletePeerMeasurement`

`validate_peer_measurement` is the only direct constructor.  It extracts
`PeerMeasurement::outbound_rate` and `inbound_rate` separately and returns
`IncompletePeerMeasurement { peer, direction: "outbound" }` or
`{ direction: "inbound" }` when either option is absent.  The peer identifier
is owned as a `String`; the direction is a static string so the `Display`
message is stable.

The caller is `ProbeEngine::probe`, after `PeerBenchmarkAttempt::into_measurement`
and before the measurement is appended to `Measurements::peers`.  Therefore a
single missing direction stops the entire profile and cannot create a one-way
topology link or a guessed reverse rate.

### `InvalidProfile`

This variant is reserved for profile identity and cross-object agreement, not
for malformed bytes in a cache file.

`ProbeEngine::probe` maps the following final checks to `InvalidProfile`:

- `Topology::validate` failure, reported as `topology validation failed: ...`;
- `Topology::validate_scheduling_properties` failure, reported as
  `topology scheduling validation failed: ...`;
- `DiscoveryProfile::validate` failure, reported as
  `discovery validation failed: ...`; and
- the final `crate::codec::validate_profile` call, reported as
  `constructed profile validation failed: ...`.

`build_topology` repeats the topology validation before returning and maps a
failure to `constructed topology is invalid: ...`.  This is a construction
guard, not a second successful path.  All of these errors are propagated by
`ProbeEngine::probe`, so `probe_and_store` never calls `ProfileCache::store`.

`probe/src/resolve.rs` uses `profile_mismatch` to construct the same variant
when a validated profile is applied to current hardware.  It rejects a
non-exhaustive current GPU inventory, an unmatched machine fingerprint,
missing or unexpected stable RAM/storage/GPU keys, duplicate live keys,
missing host-memory associations, a wrong topology device kind, a missing GPU
calculation capability, or a changed GPU target.  `MeasuredProfile::resolve_local_inventory`
first calls `validate_profile`; a malformed profile therefore fails before any
current inventory is associated.

The consequence is fail-closed realization.  No ordinal, name, capacity,
benchmark similarity, or newly visible domain is substituted.  Native
preparation maps this variant to `NativePreparationError::Probe`, and the
binding callback is never invoked with a partial scope.

### `Cache`

`Cache` covers two closely related surfaces: secure explicit-file handling in
`probe/src/cache.rs`, and canonical profile codec validation in
`probe/src/codec.rs`.

#### Explicit path cache

`cache_error` is the local constructor.  `ExplicitPathProfileCache::new`
rejects a relative path and a path without a file name.  `parent` rejects a
missing or non-canonical/symlink parent, a non-directory parent, group/other
permissions, or ownership other than the effective user.  The corresponding
canonicalize and metadata operating-system failures are `Io`.

`load_existing` returns `Ok(None)` only when the target does not exist.  Every
other invalid target is terminal: a symlink or non-regular file, insecure
permissions, wrong ownership, a changed device/inode after open, a file over
`MAXIMUM_PROFILE_BYTES`, a length that cannot fit `usize`, a stale
`CacheIdentity`, or a decoded profile rejected by the codec.  Open, metadata,
and read failures are `Io`.  A missing target is the only cache miss and is the
only condition that allows a caller to perform a fresh probe.

`store` first encodes and validates the complete profile.  It returns `Cache`
when an existing identity-named target contains a different profile or when
64 same-directory temporary names all collide.  It returns `Io` for temporary
file write/sync, atomic `NOREPLACE` installation, permission, and directory
sync failures.  An existing byte-identical profile is idempotent and returns
`Ok(())`; a failed temporary write is removed by `PendingFile::drop`.  The
atomic install never replaces a different cache file.

#### Canonical profile codec

`codec_error` wraps every codec validation or representation failure as
`ProbeError::Cache(format!("codec: {message}"))`.  Direct `Cache` construction
also reports encoded or input profile size over `MAXIMUM_PROFILE_BYTES`.

`MeasuredProfileCodec::decode` rejects a truncated input, checksum mismatch,
magic mismatch, unsupported codec schema, unconsumed payload bytes, invalid
lengths or strings, invalid enum/provenance tags, invalid unit values, and
every malformed field surfaced by `Decoder`.  `Decoder::raw` detects offset
overflow and truncation; `length` enforces `MAXIMUM_ITEMS`; `label` enforces
`MAXIMUM_STRING_BYTES`, UTF-8, and `Label` validity; property readers enforce
the core unit constructors.

`validate_profile` and its helpers enforce the persisted measured-profile
invariants.  They reject an unsupported profile/cache schema or zero cache
digest; unbounded benchmark metadata or non-canonical durations; inconsistent
peer protocol, endpoint, directional evidence, derived rates, or execution;
unknown, duplicate, missing, or wrongly typed origin metadata; non-strict
canonical ordering of every origin, peer, topology, node-device, or discovery
collection; estimated or overridden persisted properties; topology/discovery
device or link disagreement; and cross-machine transport records that are not
exact opposing pairs or do not match authenticated peer rates and duplex
evidence.  `require_measured` in this module intentionally returns `Cache`,
not `MissingMeasurement`, because the invalid state is already persisted or is
about to be persisted.

Encoding runs the same limits in reverse.  `Encoder::length` and `Encoder::label`
reject over-limit collections or strings and values that cannot fit canonical
integer widths; benchmark duration encoding rejects an unrepresentable
nanosecond count.  `MeasuredProfileCodec::encode` validates before encoding and
`encode_unchecked` enforces the final byte limit after the checksum is added.

Cache and codec errors propagate through `ProfileCache::load` or `store`,
`ProbeEngine::load_or_probe_and_store`, and the root CLI.  A malformed,
insecure, stale, or semantically invalid existing file is not silently treated
as a miss, so the CLI does not overwrite it or run an alternate probe.  Only a
genuine `NotFound` target returns `Ok(None)` and permits a fresh measured
profile.

### `Io`

`ProbeError::io` is used when the failed operation itself is the evidence.  It
records an operation label, the exact path, and the source error's display text.
It is not used for a semantic absence that the surrounding protocol handles as
a normal branch.

The production call sites are:

| Area | Operations represented as `Io` |
| --- | --- |
| `SeedContract::read` | Read the contract file. |
| `LocalSystemDiscovery` | Read procfs/sysfs identity and memory files, read mountinfo, canonicalize block paths, enumerate network directories, remove an access-probe file, seek/write/sync/read benchmark files, create a temporary benchmark file, and query `statvfs`. |
| `ExplicitPathProfileCache` | Canonicalize or inspect the parent, inspect/open/read the target, inspect the opened inode, write/sync the temporary, install with atomic `NOREPLACE`, set temporary permissions, create a temporary, and sync the parent directory. |
| `native-probe::identity` | Inspect/canonicalize/read native backend libraries, enumerate PCI devices, read PCI identity fields, inspect PCI roots, resolve a PCI driver, and read a non-optional PCI identity-surface file. |
| `native-probe::native` | Inspect a GPU PCI root, enumerate DRM devices/connectors, and read connector state. |
| `native-probe::hsa` | Read KFD GPU properties for LDS capacity. |

Some OS errors are deliberately consumed before this boundary.  Cache target
`NotFound` is `Ok(None)`, temporary-name `AlreadyExists` is retried up to the
bounded collision count, storage permission/read-only candidates are skipped,
missing optional library candidates are skipped, and missing optional PCI
surface files are omitted.  Other failures remain visible as `Io` or the
semantic `Discovery`/`Cache` error selected by the caller.  No `Io` variant
contains a fallback path or a substitute state.

## Propagation boundaries and callers

### Live `recipe probe` command

The root CLI (`src/cli.rs::run_probe`) maps every `ProbeError` to its
`Display` string.  The end-to-end sequence is:

1. `SeedContract::read` or `SeedContract::parse` validates the strict seed.
2. `LocalSystemDiscovery::discover_host` establishes the host inventory.
3. `NativeGpuProbe::new` validates native configuration, then
   `ProbeEngine::current_cache_identity` runs host/GPU/peer inspection only.
4. The CLI chooses the identity-derived profile path and creates an explicit
   private cache.
5. `ProbeEngine::load_or_probe_and_store` repeats identity inspection and
   loads an exact cache hit, or calls `probe` for all RAM, storage, GPU, and
   peer measurements.
6. `probe` builds topology, origins, and discovery data, validates them, then
   `probe_and_store` atomically stores the complete encoded profile.
7. Only after the profile succeeds does the CLI write the active native
   receipt and print identities.

At any `ProbeError`, the `Result<(), String>` returned by the CLI is an error:
there is no profile result, cache store, active receipt, or success summary for
that invocation.  A cache `NotFound` miss is the one intentional branch that
continues to step 5's fresh probe.  A malformed or insecure existing cache
does not fall through to a fresh probe.

### Native preparation and realization

`src/native_prepare.rs` wraps `ProbeError` in
`NativePreparationError::Probe`.  The wrapper preserves the source through
`std::error::Error::source`, and its display prefix is
`native probe/profile validation failed: ...`.  The wrapper is used by
`load_cached_measured_profile`, `with_native_preparation`, and
`with_current_native_preparation` for cache, native discovery, exact inventory
resolution, and binding errors.

`with_native_preparation_from_probe` first discovers all native GPUs, resolves
the current host and GPU keys against the validated profile, requires a
complete local GPU scope, reopens CUDA/HSA bindings, and only then invokes the
preparation callback.  Any `ProbeError` means no callback scope is handed out.
The callback can still return a separate `NativePreparationError`, but that is
not converted back into `ProbeError`.

### Peer session path

`transport::TcpPeerSession` returns `PeerBenchmarkAttempt`, retaining phase and
structured failure kind while transport operations run.  `ProbeEngine::probe`
converts a failed attempt to `ProbeError::Benchmark` at
`PeerBenchmarkAttempt::into_measurement`, then validates measured endpoint,
sample, duration, and rate evidence.  The profile cannot contain a peer record
until both directions have measured provenance and consistent evidence.

### Exact resolution path

`MeasuredProfile::resolve_local_inventory` is called by native binding
reopening.  It first invokes `validate_profile`, then rejects any current
machine, RAM, storage, GPU, host-memory association, or GPU target mismatch as
`InvalidProfile`.  This is a caller-facing realization failure, not a cache
repair request.  The caller must rerun `recipe probe` to obtain a new measured
identity.

## State and invariant summary

- A `Contract` failure means no probe inputs are admitted.
- A `Discovery` or `IncompleteGpuEnumeration` failure means the inventory is
  not complete enough to identify the machine and all devices.
- A `Benchmark`, `MissingMeasurement`, or `IncompletePeerMeasurement` failure
  means no measured value is admitted to topology construction.
- An `InvalidProfile` failure means a profile and live state cannot be joined
  without guessing or silently covering a device.
- A `Cache` failure means serialized state, cache security, or cache identity
  is invalid.  It is not a permission to overwrite or ignore the file.
- An `Io` failure means the named operation failed and its path/source text is
  the complete available evidence.

The common invariant is fail closed at the first invalid transition.  Callers
may format the error, wrap it, or request a fresh probe after a genuine cache
miss, but no caller may turn one of these variants into a partial inventory,
estimated persisted property, guessed device association, or successful native
execution scope.

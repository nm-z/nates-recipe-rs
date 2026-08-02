---
document: recipe_probe.local
source: probe/src/local.rs
kind: linux-local-host-discovery-and-bounded-benchmarks
authority:
  - probe/src/local.rs
  - probe/src/model.rs
  - probe/src/engine.rs
  - probe/src/error.rs
  - src/cli.rs
  - src/native_prepare.rs
  - core/src/identity.rs
  - core/src/topology.rs
  - core/src/units.rs
---

# Paired local discovery and host benchmarks

## Intent and boundary

[`probe/src/local.rs`](../../src/local.rs) is the Linux bare-metal host half of
`recipe-probe`. It contains two concrete implementations that are deliberately
paired at the `recipe_probe::model` boundary:

```text
LocalSystemDiscovery  --HostDiscovery-->  HostInventory
LocalHostBenchmarks   --HostBenchmarkIo-> RamMeasurement / StorageMeasurement
```

`LocalSystemDiscovery` reads the current machine's procfs and sysfs identity and
enumerates its RAM, mounted block storage, and non-loopback network interfaces.
`LocalHostBenchmarks` consumes the RAM and storage descriptors produced by that
discovery and runs bounded in-process copy and temporary-file measurements. The
two types do not discover GPUs, open a GPU runtime, benchmark network links, or
construct a profile themselves. GPU discovery and measurement are supplied by
`recipe-native-probe`; peer throughput is supplied by explicit
`PeerSession` implementations; profile assembly and validation belong to
`ProbeEngine`.

The local module therefore has one narrow purpose: turn the host-visible Linux
state into the host descriptors and host measurements required by the generic
probe engine. It preserves stable discovered labels and physical relationships,
while marking benchmark outputs as measured `Property` values. It does not
choose a scheduler policy, infer a device from capacity, or retain a cache.

The module is Unix-specific. It imports Unix metadata device numbers and file
permission extensions, and it calls `rustix::fs::statvfs`; there is no Windows or
portable fallback in this crate.

## Source map

The line ranges below are navigation aids into the current implementation. The
behavioral contract is the code in those regions and the trait contracts in
`probe/src/model.rs`.

| Region | Source | Responsibility |
| --- | --- | --- |
| `LocalSystemDiscovery`, defaults, `discover_host` | `local.rs:22-79` | Hold proc/sys/etc roots, select benchmark roots, and assemble one host inventory. |
| `with_benchmark_roots` | `local.rs:81-93` | Replace only the ordered benchmark-root preference list while retaining `/proc`, `/sys`, and `/etc`. |
| RAM discovery | `local.rs:95-144` | Enumerate NUMA node meminfo files, or use `/proc/meminfo` as the only fallback. |
| Storage discovery | `local.rs:146-257` | Parse mountinfo, resolve mounted partitions to physical block devices, group mounts, and choose a writable benchmark root. |
| Network discovery | `local.rs:259-307` | Enumerate non-loopback sysfs interfaces and retain link, driver, firmware, and transport labels. |
| Physical block resolution | `local.rs:310-343` | Walk partition parents and require physical `size` and `dev` files. |
| Benchmark-root selection and access probe | `local.rs:345-428` | Match candidate directories to the discovered physical disk and test create/remove access. |
| `LocalHostBenchmarks` | `local.rs:430-508` | Run bounded RAM copies and bounded disk writes, syncs, and reads. |
| Temporary benchmark file | `local.rs:510-541` | Allocate a unique hidden file and remove it on drop. |
| Filesystem capacity and rate accounting | `local.rs:543-580` | Read `statvfs`, derive nonzero bytes-per-second values, and reject unbounded plans. |
| Text, label, and mount helpers | `local.rs:584-644` | Parse `MemTotal`, read nonempty strings, join optional fields, validate labels, and unescape mount paths. |
| Host trait boundary | `model.rs:30-85`, `model.rs:124-159` | Define the descriptors, bounded plan, measurements, and caller-facing traits. |
| Engine use | `engine.rs:63-160`, `engine.rs:163-238` | Discover, normalize, benchmark, validate measured provenance, build, cache, and publish the profile. |
| CLI construction | `src/cli.rs:876-947`, `src/cli.rs:949-1009` | Construct the local pair for `recipe probe` and current native preparation. |

## End-to-end role and callers

### The public `recipe probe` path

The zero-argument production command is assembled in `src/cli.rs::run_probe`.
The complete local portion is:

```text
recipe probe
  -> run(arguments)
  -> parse_probe_options
  -> require_bare_metal
  -> SeedContract::read or embedded topology/contract.toml
  -> private_state_root
  -> ensure_private_directory(state_root/scratch)
  -> LocalSystemDiscovery::with_benchmark_roots([state_root])
  -> LocalSystemDiscovery::discover_host       (select host_memory_key)
  -> NativeGpuProbe::new
  -> LocalHostBenchmarks
  -> ProbeEngine::new(host, native, host_benchmarks, native)
  -> ProbeEngine::current_cache_identity       (discovery-only inspection)
  -> ExplicitPathProfileCache::new
  -> ProbeEngine::load_or_probe_and_store
       -> current_cache_identity                (fresh inspection)
       -> cache hit: return validated profile
       -> cache miss: probe_and_store
            -> probe
                 -> inspect                          (another discovery)
                 -> local RAM benchmark per RAM domain
                 -> local storage benchmark per storage domain
                 -> native GPU benchmark per GPU
                 -> peer benchmark per supplied session
                 -> topology and discovery validation
                 -> profile validation
            -> cache.store
  -> ActiveNativeReceipt::capture and write
  -> print profile and identity summary
```

`run_probe` passes an empty peer slice, so this command does not run a network
peer benchmark. It still discovers local network interfaces because those
interfaces are part of the machine inventory and cache identity. The same
`LocalSystemDiscovery` reference is used for every engine inspection and the
same `LocalHostBenchmarks` value is used for each host measurement.

There are multiple discovery-only calls by design. The first call supplies the
first local RAM key to native configuration. The explicit
`current_cache_identity` call computes the profile filename identity before the
cache object is opened. `load_or_probe_and_store` computes the identity again
before deciding whether the cache is valid. A cache miss enters `probe`, whose
`inspect` call discovers again before measurements. Thus the usual cache-hit
path calls `discover_host` three times, while a fresh-measurement path calls it
four times. The local benchmark methods run only in the fresh path, once per
discovered RAM domain and once per discovered storage domain.

The command-level bare-metal gate is outside this module. `require_bare_metal`
rejects `/.dockerenv`, `/run/.containerenv`, container markers in
`/proc/1/cgroup`, and a nested PID namespace in `/proc/self/status`. Calling
`LocalSystemDiscovery` directly does not perform that gate.

### Current native preparation caller

`src/native_prepare.rs::with_current_native_preparation` calls
`src/cli.rs::current_native_inputs`, which is the other direct production caller
of the local pair. That path is preparation-scoped, not a fresh measurement:

```text
with_current_native_preparation
  -> current_native_inputs
       -> require_bare_metal
       -> private_state_root
       -> LocalSystemDiscovery::with_benchmark_roots([state_root])
       -> discover_host
       -> active receipt present:
            verify receipt.host_memory_key is in current RAM keys
            reopen the recorded native configuration
       -> no active receipt:
            parse embedded seed
            ensure scratch directory
            choose first current RAM key
            construct NativeGpuProbe and LocalHostBenchmarks
            ProbeEngine::current_cache_identity
            derive identity-named profile path
  -> ExplicitPathProfileCache::load(exact identity)
  -> native inventory and profile local-inventory resolution
  -> native preparation callback
```

When a receipt exists, `current_native_inputs` only invokes local discovery and
checks the recorded RAM origin; it does not instantiate `LocalHostBenchmarks`.
When the receipt is absent, it constructs the same pair in order to calculate
the current discovery identity, but `current_cache_identity` is discovery-only
and still does not run a RAM or disk benchmark. The later preparation code
reopens the exact measured profile and associates current domains using the
retained discovery keys. It never treats a capacity or product-name match as a
replacement identity.

`rg` over the workspace finds no other caller of `LocalSystemDiscovery` or
`LocalHostBenchmarks`. There is no second local-host implementation and no
local test adapter in the production call graph.

### Engine boundary

`ProbeEngine::new` stores four borrowed trait objects. The CLI passes the local
discovery as `HostDiscovery`, the native probe as both `GpuDiscovery` and
`GpuBenchmarkIo`, and `LocalHostBenchmarks` as `HostBenchmarkIo`.

`ProbeEngine::inspect` calls `discover_host`, sorts RAM, storage, and network
keys, requires at least one RAM and one mounted storage domain, and rejects
duplicate labels. It also verifies that every storage domain points to one
discovered RAM key and advertises asynchronous submission. GPU and peer
validation occurs beside this host validation. Only after this inspection does
`ProbeEngine::probe` derive four bounded plans and call the host benchmark
methods.

For each RAM domain, the engine calls `benchmark_ram`, requires both returned
properties to have exactly `PropertyProvenance::Measured`, and stores the
result under the domain key. For each storage domain it does the same for
capacity, read rate, and write rate. The host results are then combined with
native GPU and optional peer results to construct the topology and discovery
profile. Local RAM becomes a `DeviceKind::Ram` device. Local storage becomes a
`DeviceKind::Disk` device whose schedulable rate is the measured minimum of read
and write rates. Each storage domain contributes a pair of directional links
between its host RAM device and disk, using the measured write rate for host to
disk and read rate for disk to host. The local descriptors' lane counts and
duplex values become the corresponding link and discovery capabilities.

The engine validates topology, scheduling properties, discovery, and then the
whole `MeasuredProfile` before the cache receives it. A local benchmark error
therefore aborts the complete profile; there is no partial host profile or
estimated-property fallback.

## The paired types and their data model

### `LocalSystemDiscovery`

The struct has four private fields:

| Field | Default | Meaning |
| --- | --- | --- |
| `proc_root` | `/proc` | Kernel hostname, OS release, global memory fallback, and mount table. |
| `sys_root` | `/sys` | DMI identity, NUMA memory nodes, block-device links, and network interfaces. |
| `etc_root` | `/etc` | Preferred machine identity source, `/etc/machine-id`. |
| `benchmark_roots` | empty | Ordered caller-provided directories considered before `HOME`, current directory, and mounts. |

`Default` is the only constructor that fixes procfs, sysfs, and etc roots.
`with_benchmark_roots` uses struct update syntax from that default and changes
only the benchmark-root vector. A configured root is a location preference, not
an assertion about a disk. The selector canonicalizes the directory and checks
its `st_dev` against the physical disk represented by the storage group before
accepting it.

### `HostInventory` output

`discover_host` returns the `HostInventory` required by
`HostDiscovery::discover_host`:

```text
HostInventory {
    machine: MachineFingerprint {
        hostname,
        stable_id,
        runtime_abi,
        firmware,
    },
    ram: Vec<RamDomain>,
    storage: Vec<StorageDomain>,
    network: Vec<NetworkInterface>,
}
```

The local implementation does not assign Recipe `MachineId` or `DeviceId`.
`ProbeEngine::assign_ids` assigns IDs after the host and GPU inventories have
been normalized. The retained local keys and machine fingerprint are the
identity inputs used to associate those later IDs with a current machine.

## Host identity discovery

`discover_host` reads the machine fields in this order:

| Inventory field | Input and behavior | Failure or fallback |
| --- | --- | --- |
| `machine.hostname` | `read_trimmed(proc_root/sys/kernel/hostname)` | Read failure or empty content is `ProbeError::Io` or `ProbeError::Discovery`. |
| `machine.stable_id` | `read_first_available([etc_root/machine-id, sys_root/class/dmi/id/product_uuid])` | Each read error is ignored while trying the next path; if neither yields a nonempty string, `Discovery("no stable machine identity is available")`. |
| `machine.runtime_abi` | `read_trimmed(proc_root/sys/kernel/osrelease)` | Read failure or empty content is an error. |
| `machine.firmware` | `join_available([sys_root/class/dmi/id/bios_vendor, bios_version, bios_date], "|")` | Individual read failures and empty values are ignored. If all are unavailable, literal `firmware-unreported` is used. |

`read_trimmed` reads a UTF-8 string, applies `str::trim`, rejects the empty
result, and returns the trimmed owned value. The machine labels are then passed
through `Label::new`, which rejects an empty or whitespace-only value. The
helper maps that label error to `ProbeError::Discovery`.

The machine fingerprint is not a display-only record. `ProbeEngine` hashes all
four fields into the `recipe-probe-cache-v7` cache identity. The fingerprint is
also retained in `MeasuredMachineOrigin`, and
`MeasuredProfile::resolve_local_inventory` requires an exact fingerprint match
before it associates current RAM, storage, and GPU descriptors with the stored
machine ID.

## RAM discovery

`discover_ram` first tries the NUMA node directory
`sys_root/devices/system/node`.

1. If `read_dir` succeeds, entries are converted to paths and retained only
   when the final name is `node` followed by one or more ASCII digits.
2. The paths are sorted lexicographically.
3. For each path, `path/meminfo` is read and parsed by `parse_memtotal`.
4. The node name becomes the `RamDomain.key`.
5. The `MemTotal` value in KiB is multiplied by 1024 with checked arithmetic and
   becomes `capacity_hint`.
6. The link label is `memory-link:{node-name}`.
7. `maximum_inflight_transfers` is `TransferLaneCount::new(1)`.

The node directory is intentionally not a required source. If `read_dir` fails,
or it succeeds but produces no accepted node paths, the implementation reads
`proc_root/meminfo` and emits one domain with key `memory0` and link identity
`memory-link:memory0`. A failed node `meminfo` read or malformed `MemTotal` in
an otherwise nonempty node list is an error, not a fallback to global memory.
Individual directory-entry read errors are discarded by `filter_map(Result::ok)`;
the code does not report them.

`parse_memtotal` scans lines for the substring `MemTotal:`. For the first match,
it scans the remainder for the first token that parses as `u64`, then multiplies
that number by 1024. It does not require a `kB` unit token and does not reject
additional text after the number. It returns a discovery error when a matching
line has no parseable number, when multiplication overflows, or when no matching
line exists.

The RAM capacity is called a `capacity_hint` in the descriptor because it is a
discovery value. `LocalHostBenchmarks::benchmark_ram` returns this same value in
the measurement's `capacity` field, marked `Measured`. This is the actual
behavior: the local RAM benchmark measures copy throughput, but it does not
measure installed capacity independently of the procfs or NUMA hint.

## Storage discovery

`discover_storage` uses `/proc/self/mountinfo` as its mount source and groups
mounted partitions by their resolved physical block-device path. The operation
is deterministic and rejects a physical identity whose repeated mount records
disagree about capacity or transport.

### Mountinfo filtering and physical identity

For every line, the implementation splits on whitespace and locates the first
`-` separator between mount options and filesystem metadata. Lines without a
separator, with fewer than five pre-separator fields, or without two fields
after the separator are skipped. For retained lines:

1. `fields[2]` is the mountinfo major:minor value.
2. `sys_root/dev/block/{major:minor}` is skipped if it does not exist.
3. The sysfs link is canonicalized. Canonicalization failure is an I/O error.
4. `physical_block_device` climbs from a partition while a `partition` marker
   exists. The resulting path must have both `size` and `dev` files.
5. The physical `size` file is parsed as a sector count. Checked multiplication
   by 512 converts sectors to bytes. Zero-byte devices are skipped.
6. The mount point in `fields[4]` is decoded by `unescape_mount`.
7. `fields[separator + 2]` is the filesystem source used only as a fallback
   display name when the physical path has no UTF-8 basename.

The physical path string is the grouping identity and is retained as the
storage `driver` label. The physical basename, or the source fallback, becomes
the `name` label. The physical `dev` file supplies the stable major:minor text
used in the public storage key.

`physical_block_device` reports a discovery error if a partition has no parent,
or if the final physical path lacks `size` or `dev`. The sector parser reports a
discovery error for malformed numbers or checked capacity overflow. A zero
capacity record is ignored rather than emitted.

### Transport and descriptor fields

Transport classification is intentionally simple and based on the canonical
physical path string:

| Physical path contains | `TransportKind` | `LinkDuplex` |
| --- | --- | --- |
| `/nvme` | `Nvme` | `Full` |
| `/sas` | `Sas` | `Full` |
| anything else | `Sata` | `Half` |

The code does not query a separate transport database or infer a rate from the
path. For each physical group it emits:

```text
key                         = block:{physical major:minor}
name                        = physical basename or mount source
benchmark_root              = selected canonical writable directory
capacity_hint               = physical sectors * 512
host_memory_key             = the first discovered RAM key
driver                      = canonical physical sysfs path
firmware                    = join(model, rev, firmware_rev, "|")
                              or storage-firmware-unreported
link_identity               = storage-link:{canonical physical path}
transport_kind              = Nvme, Sas, or Sata as above
duplex                      = Full for Nvme/Sas, Half for Sata
maximum_concurrent_reads    = 1
maximum_concurrent_writes   = 1
asynchronous_submission     = true
```

The first mount record initializes a `PhysicalStorage` group. Later records for
the same physical path must have the same byte capacity and transport kind;
otherwise the function returns `Discovery("inconsistent physical storage
identity ...")`. Each record adds its major:minor value to
`mounted_devices` and its decoded mount point to the ordered `mounts` set.
After grouping, each group must obtain a benchmark root. The resulting storage
domains are sorted by `key` before return.

The storage `host_memory_key` is copied from the argument supplied by
`discover_host`, which always passes `ram[0].key` after confirming RAM is not
empty. The local discovery therefore models every local disk as attached to
the first discovered host RAM domain; it does not derive a NUMA attachment from
mountinfo or sysfs.

### Benchmark-root selection

`select_storage_benchmark_root` considers candidates in this exact order:

1. The vector passed to `with_benchmark_roots`, in caller order.
2. `$HOME`, when the environment variable is present.
3. The current working directory, when `current_dir` succeeds.
4. Every mount point collected for the physical group, in `BTreeSet` order.

Each candidate is canonicalized. Missing, inaccessible, or otherwise
uncanonicalizable candidates are skipped. Canonical paths are deduplicated. A
candidate must be a directory, and its Unix `st_dev` major:minor, obtained with
`MetadataExt::dev` and `rustix::fs::major/minor`, must belong to the group's
`mounted_devices`. This is the physical-disk guard: a writable directory on a
different disk cannot be selected merely because it is convenient.

`directory_accepts_probe_file` then tries up to 64 unique hidden names of the
form `.recipe-probe-access-{pid}-{nonce}.tmp`. It opens each with
`write + create_new` and mode `0600`, closes it, and removes it. A successful
create and remove returns `true`. `AlreadyExists` advances the process-local
atomic nonce. Permission denied and read-only-filesystem errors return `false`,
which lets selection continue to the next candidate. Other errors are returned
as `ProbeError::Io`; exhaustion of all 64 names is a discovery error. Failure
to remove an access-test file is also an I/O error.

If every candidate is skipped or rejected, storage discovery fails with
`Discovery("physical disk {identity} has no user-writable benchmark directory
on any mounted filesystem")`. The access-test file is not the benchmark file;
it only proves that the later temporary file can be attempted in that
directory.

### Mount escaping

`unescape_mount` decodes the four octal escapes used by mountinfo:

```text
\040 -> space
\011 -> tab
\012 -> newline
\134 -> backslash
```

No other escape is decoded. The function operates on the whitespace-split
mountpoint token after mountinfo has escaped embedded whitespace.

## Network interface discovery

`discover_network` reads `sys_root/class/net`, propagates a directory-read error,
sorts the entries by path, and excludes exactly the interface named `lo`.
Entries whose names are not valid UTF-8 are a discovery error. For every other
interface it reads:

| Field | Source |
| --- | --- |
| `name` | sysfs directory basename |
| `address` | `address`, trimmed and nonempty |
| `ifindex` | `ifindex`, trimmed and nonempty |
| `driver` | canonical `device/driver` path, or `network-driver-unreported` if canonicalization fails |
| `firmware` | joined `device/firmware_version` and `device/uevent`, or `network-firmware-unreported` |

Wireless detection is the existence of `path/wireless`. A wireless interface is
`TransportKind::Wlan` and `LinkDuplex::Half`; every other interface is
`TransportKind::Ethernet` and `LinkDuplex::Full`. Both kinds set
`asynchronous_submission` to `true`.

The identity strings are deterministic combinations of the live address and
index:

```text
key           = net:{ifindex}:{address}
link_identity = network-link:{ifindex}:{address}
```

The implementation does not read link speed, carrier state, MTU, or perform a
network transfer. An empty non-loopback interface list is allowed by this local
function and by `normalize_host`; network throughput is absent from the local
benchmark pair and can only be supplied by explicit peer sessions.

## Benchmark plan contract

The trait boundary gives both host methods the same `BoundedBenchmarkPlan`:

```text
buffer_bytes:      ByteCount
iterations:        u32
maximum_duration:  Duration
```

`BoundedBenchmarkPlan::is_bounded` is true exactly when all three are nonzero.
`LocalHostBenchmarks` calls `require_bounded` before allocating any benchmark
buffer or temporary file. An unbounded plan returns
`ProbeError::Benchmark("benchmark plan must have nonzero byte, iteration, and
duration bounds")`.

The production engine derives the plan from the seed contract, not from local
hardware values. `BenchmarkPlans::from_seed` computes suggested byte counts as
follows and then `bounded_plan` clamps each value to 4 KiB through 64 MiB,
sets eight iterations, and sets a two-second maximum duration:

| Host plan | Seed expression |
| --- | --- |
| RAM | `seed.estimates.ram_capacity / 1024` |
| storage | `seed.estimates.disk_capacity / 16_384` |

The engine applies the same plan shape to GPU and peer paths. The seed values
only bound the experiment. They are not copied into the output as estimates,
and the local implementation does not read seed values itself.

Direct callers can supply any nonzero plan accepted by `is_bounded`; the local
methods additionally require that `buffer_bytes` fit `usize`. The engine's
clamp is the normal CLI path and is not re-applied inside `local.rs`.

## RAM benchmark

`LocalHostBenchmarks::benchmark_ram(domain, plan)` performs this sequence:

1. Require a bounded plan.
2. Convert `plan.buffer_bytes.get()` to `usize`; conversion failure is a
   benchmark error before allocation.
3. Allocate a source vector filled with byte `0xa5` and a zero-filled
   destination vector of the same length.
4. Capture one `Instant`.
5. While `iterations < plan.iterations` and elapsed time is below the plan
   duration, copy the entire source into the destination and pass the
   destination to `black_box`, then increment the completed-iteration count.
6. Convert the count and elapsed duration with `observed_rate`.
7. Return `RamMeasurement` with `capacity = measured(domain.capacity_hint)` and
   `transfer_rate = measured(rate)`.

The copy is an in-process host-memory operation. There is no thread pool,
parallel copy, NUMA pinning, cache flush, explicit alignment, or independent
capacity query. `black_box` prevents the destination from being discarded as a
dead result. The loop has both an iteration bound and a wall-clock bound. It may
finish fewer than the requested iterations when the duration expires, but a
valid rate requires at least one completed iteration and a nonzero elapsed
duration.

The capacity output is the discovery hint described in
[RAM discovery](#ram-discovery), wrapped as `PropertyProvenance::Measured`. The transfer-rate output
is derived from the timed copy. `ProbeEngine` accepts it only because the local
method marks both properties measured; it does not independently distinguish
the capacity source.

## Storage benchmark

`LocalHostBenchmarks::benchmark_storage(domain, plan)` performs a write phase,
then a read phase, against one temporary file in `domain.benchmark_root`:

1. Require a bounded plan and convert the byte count to `usize`.
2. Create a unique read/write temporary file with
   `TemporaryProbeFile::create`.
3. Allocate a buffer filled with byte `0x5a`.
4. Start the write timer. For each write while the iteration and duration bounds
   hold, seek to offset zero, write the entire buffer, call `sync_data`, and
   increment the write count.
5. Convert write count and duration to `write_rate`.
6. Allocate a zero-filled read buffer. Start the read timer. For each read while
   the bounds hold, seek to offset zero, `read_exact` the full buffer, pass it to
   `black_box`, and increment the read count.
7. Convert read count and duration to `read_rate`.
8. Query `filesystem_available_bytes(domain.benchmark_root)` using `statvfs`.
9. Return all three values as measured properties.

The file is reused for all writes and reads in this call. `sync_data` makes
each write wait for the file's data to be synchronized before its iteration is
counted. Reads are performed after the write phase, and every read requests the
full buffer from offset zero. The implementation does not call `fsync` on the
parent directory, truncate between iterations, randomize offsets, bypass the
page cache, or perform concurrent I/O. The measured rate is the aggregate
bytes-per-second over completed full-buffer operations.

`TemporaryProbeFile::create` uses a process-local atomic nonce and up to 64
names of the form `.recipe-probe-{pid}-{nonce}.tmp`. It opens with
`read + write + create_new`; an existing name advances the nonce, while any
other open error is returned as `ProbeError::Io`. Exhausting names is a
benchmark error. The `Drop` implementation attempts `remove_file` and ignores
that removal error. Therefore a successful benchmark normally leaves no file,
but cleanup failure during drop is not surfaced to the caller.

`filesystem_available_bytes` calls `rustix::fs::statvfs` on the selected root
and multiplies `f_bavail` by `f_frsize` with checked arithmetic. The result is
the bytes available to the calling user, not the physical disk capacity from
sysfs. `statvfs` failure and multiplication overflow are benchmark I/O or
benchmark errors respectively. A zero resulting `ByteCount` is representable
by the type; the local helper does not reject it directly.

## Rate accounting and provenance

`observed_rate(bytes, iterations, elapsed)` is shared by both host benchmark
methods. Its exact calculation is:

```text
if iterations == 0 or elapsed == zero:
    Benchmark("bounded benchmark completed no measurable work")

total = bytes * iterations * 1_000_000_000       (checked u128)
rate = total / elapsed.as_nanos()
rate = u64::try_from(rate)
BytesPerSecond::new(rate)
```

Overflow in the checked multiplication, conversion beyond `u64`, or a zero
rate rejected by `BytesPerSecond::new` is a benchmark error. Integer division
is used, so a nonzero workload can still fail if the computed bytes-per-second
rounds down to zero. The elapsed duration is captured around the complete
operation loop, including seeks, copies, writes, syncs, reads, and
`black_box` calls performed inside that loop.

The helper `measured(value)` is a one-line constructor for
`Property::new(value, PropertyProvenance::Measured)`. It is used for every
field in `RamMeasurement` and `StorageMeasurement`, including capacity fields
that originate from discovery or `statvfs`. `Property::is_schedulable` permits
measured and override values, but the probe engine and profile codec require
the stricter measured provenance for a persisted measured profile.

## Identity, ordering, and state invariants

### Stable local identities

The local pair emits labels, not numeric Recipe IDs. Their identity sources are:

| Domain | Stable key or identity | Other retained identity facets |
| --- | --- | --- |
| Machine | hostname, stable ID, runtime ABI, firmware | Exact four-field `MachineFingerprint`. |
| NUMA RAM | node name, or `memory0` fallback | `memory-link:{name}`, capacity hint, one lane. |
| Physical storage | `block:{physical dev major:minor}` | Canonical physical path, model/revision firmware, `storage-link:{path}`, transport, duplex, benchmark root, mount major:minor set. |
| Network | `net:{ifindex}:{address}` | `network-link:{ifindex}:{address}`, name, driver, firmware, transport, duplex. |

`ProbeEngine::build_cache_identity` hashes the machine fingerprint, every RAM
field, every storage field including `benchmark_root`, and every network field.
Consequently changing the selected writable benchmark directory invalidates the
cache even when its physical device is unchanged. The benchmark measurements
are not part of the cache identity; they enter later topology and discovery
digests. `MeasuredProfile::resolve_local_inventory` uses the retained RAM and
storage keys, exact machine fingerprint, and current GPU keys to reopen a
profile. It never uses capacity, product name, ordinal, or rate similarity as
a fallback selector.

### Ordering and uniqueness

RAM node paths are sorted before descriptor construction. Storage groups are a
`BTreeMap` keyed by physical path and the final descriptors are sorted by
`block:{major:minor}`. Network paths are sorted before construction. The engine
sorts all three vectors again by their public keys and rejects duplicate RAM,
storage, or network labels. This gives profile hashing and cache identity a
canonical order independent of directory-entry order.

The local module itself does not check duplicate generated labels. A duplicate
network address and ifindex can therefore be emitted from unusual sysfs state;
`ProbeEngine::normalize_host` is the caller that rejects the resulting duplicate
key. Likewise, a physical storage key collision that does not share the same
physical identity is rejected at engine normalization or profile validation,
not by `local.rs` alone.

### Attachment and capability invariants

* `discover_host` refuses to continue without one RAM domain and passes the
  first RAM key to storage discovery.
* Every emitted storage domain references that first RAM key, has one read lane
  and one write lane, and reports asynchronous submission.
* Every RAM domain has one nonzero transfer lane because
  `TransferLaneCount::new(1)` is checked.
* Every network domain reports asynchronous submission, with full duplex for
  wired interfaces and half duplex for wireless interfaces.
* Local storage transport and duplex are path classifications, not measured
  throughput or a promise that the kernel supports concurrent operations.
* The host benchmark does not mutate descriptors or persistent profile state. It
  advances only its process-local temporary-file nonce and returns values keyed
  by the descriptor reference supplied by the engine.
* A benchmark's temporary file is scoped to one storage call and is not a
  profile artifact.

The trait documentation in `model.rs` says host discovery must enumerate every
locally usable RAM, mounted storage, and network domain or return an error
rather than a partial inventory. The implementation follows that contract for
many required reads, but its actual fallbacks and skips are narrower and should
be read literally: inaccessible NUMA node directories fall back to global
memory, malformed mountinfo lines and missing sysfs block links are skipped,
optional firmware fields are omitted, and successfully read network entries are
excluded only when their name is `lo` (directory-entry read errors are also
discarded by the iterator).

## Error boundary

All local failures use `ProbeResult<T>`, an alias for `Result<T, ProbeError>`.
The local code produces these variants:

| Variant | Local causes |
| --- | --- |
| `ProbeError::Discovery` | Empty required text, malformed memory or sector values, overflow converting memory or storage capacity, invalid labels or node names, missing physical block files, inconsistent physical identity, no benchmark directory, access-name exhaustion, and invalid generated assumptions. |
| `ProbeError::Benchmark` | Unbounded plan, buffer size not fitting `usize`, no measurable work, rate arithmetic or conversion failure, temporary-file name exhaustion, filesystem available-byte overflow, and other benchmark-level failures. |
| `ProbeError::Io` | A named filesystem operation fails, including read, read-directory, canonicalize, remove, create, seek, write, sync, read-exact, and `statvfs`. |

`read_first_available` and `join_available` intentionally suppress individual
read errors. That means an unreadable `/etc/machine-id` can be followed by a
usable DMI UUID, and a missing BIOS vendor can coexist with a reported BIOS
version. If all stable-ID candidates fail, the caller receives the generic
`no stable machine identity is available` discovery error rather than the last
I/O detail.

Errors from local methods propagate through `ProbeEngine` without a retry or a
substitute implementation. In `run_probe`, the CLI converts the `ProbeError`
to text and exits the command. `ProbeEngine::probe` does not store a partial
measurement. `ExplicitPathProfileCache::store` is reached only after all local,
GPU, peer, topology, discovery, and profile checks succeed.

The current native preparation path wraps local text errors as
`NativePreparationError::LocalConfiguration`. Once it has an exact profile,
current local inventory mismatch is reported by
`MeasuredProfile::resolve_local_inventory` as a probe/profile mismatch and
preparation stops. No local identity is silently reassigned.

## Filesystem and hardware inputs

The following table is the complete set of direct external inputs read by
`local.rs`; values are read anew on each invocation and are not cached inside
the module.

| Input | Used by | Purpose |
| --- | --- | --- |
| `/proc/sys/kernel/hostname` | machine discovery | Hostname label. |
| `/etc/machine-id` | stable-ID fallback | Preferred machine identity. |
| `/sys/class/dmi/id/product_uuid` | stable-ID fallback | Alternate machine identity. |
| `/proc/sys/kernel/osrelease` | machine discovery | Runtime ABI label. |
| `/sys/class/dmi/id/bios_vendor`, `bios_version`, `bios_date` | machine discovery | Optional joined firmware label. |
| `/sys/devices/system/node/node*/meminfo` | NUMA RAM discovery | Per-node `MemTotal`. |
| `/proc/meminfo` | RAM fallback | Global `MemTotal` when NUMA enumeration yields no domain. |
| `/proc/self/mountinfo` | storage discovery | Mounted filesystem records and mount points. |
| `/sys/dev/block/{major:minor}` | storage discovery | Resolve a mounted block device. |
| Physical block `partition`, `size`, `dev` | storage discovery | Resolve parent device, capacity sectors, and physical major:minor. |
| Physical `device/model`, `device/rev`, `device/firmware_rev` | storage discovery | Optional storage firmware label. |
| Candidate directories from configuration, `$HOME`, current directory, and mounts | storage root selection | Canonicalize, inspect `st_dev`, and test create/remove access. |
| `/sys/class/net/*` and each interface's `address`, `ifindex`, `device/driver`, `device/firmware_version`, `device/uevent`, `wireless` | network discovery | Interface labels and transport/duplex classification. |
| `Instant::now`, host allocator, file system, and `statvfs` | host benchmarks | Timed copies, writes, syncs, reads, and available-byte measurement. |

The module does not read PCI, CUDA, HSA, GPU counters, network sockets, or
kernel performance counters. `rustix::fs::major` and `minor` only decode the
Unix device number returned by metadata; they do not query a separate hardware
database.

## What the local pair does not do

The following are deliberately outside the source boundary:

* It does not create `MachineId`, `DeviceId`, `LinkId`, `TransportId`, or
  profile digests. The engine assigns IDs and builds topology after measurement.
* It does not discover or benchmark GPU devices. `NativeGpuProbe` owns those
  traits and is passed separately to `ProbeEngine`.
* It does not establish peer sessions or measure Ethernet/WLAN throughput.
  `HostInventory.network` is descriptive metadata only in this path.
* It does not read or write the measured-profile cache, active native receipt,
  or kernel scratch paths. The CLI and cache implementations own those files.
* It does not turn the seed contract into policy. The engine derives and records
  bounded plans, while this module only validates the plan shape.
* It does not retry a failed read or benchmark, choose a different physical
  disk after a benchmark failure, or fabricate an estimated value.

These exclusions are important to the paired design. The host implementation
is replaceable at the trait boundary for another explicitly owned environment,
but the production CLI supplies this concrete Linux implementation and relies
on its physical benchmark-root check and measured-property marking.

## Checks for this documentation and source boundary

The assigned change is documentation only. The focused structural checks are:

```bash
git diff --check -- probe/.docs/src/local.md
cargo check -p recipe-probe
```

`cargo check` verifies that the documented symbols and source boundary still
compile. It does not prove host hardware measurements. A real `recipe probe`
run additionally requires the CLI bare-metal gate, a complete native GPU
runtime, at least one discovered GPU, mounted writable storage on each physical
disk, and a valid seed contract. When those prerequisites are absent, the
production command is expected to stop at the corresponding real failure rather
than silently substitute local data.

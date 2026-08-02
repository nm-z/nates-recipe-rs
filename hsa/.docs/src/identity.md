<!--
Intent: document the exact conversion boundary in hsa/src/identity.rs and
trace every producer, consumer, hash/equality rule, and failure that gives
those values their meaning. The module itself does not query ROCr, discover
agents, calculate a profile digest, or own a runtime resource. It converts
raw ABI values and validated identity strings into typed values; discovery
constructs the surrounding agent record and the probe and execution layers
enforce the resulting contract.
-->

# HSA identity

`hsa/src/identity.rs` is the small, shared vocabulary for ROCr agent, queue,
ISA, and memory-pool identity. It sits between raw values returned by the HSA
ABI and the public `AgentDescription`, `Session`, `Queue`, allocation, probe,
and native-executor APIs. The module has no FFI calls and no global state. Its
fallible work validates raw enum or boolean values and parses agent UUID or AMD
ISA text; its PCI constructor is a direct bit extraction. All public identity
values are re-exported by `hsa/src/lib.rs`.

## Intent and parseable contract

The following is the contract implemented by `hsa/src/identity.rs`:

```text
hsa_identity:
  raw_enum_conversions:
    DeviceType:
      raw_i32: {0: Cpu, 1: Gpu, 2: Dsp, 3: Aie}
      unknown: Error::InvalidAttribute(field="HSA_AGENT_INFO_DEVICE")
    Profile:
      raw_i32: {0: Base, 1: Full}
      unknown: Error::InvalidAttribute(field=caller_field)
      reverse: {Base: 0, Full: 1}
    QueueKind:
      raw_u32: {0: MultiProducer, 1: SingleProducer, 2: Cooperative}
      unknown: Error::InvalidAttribute(field="HSA_AGENT_INFO_QUEUE_TYPE")
      reverse: {MultiProducer: 0, SingleProducer: 1, Cooperative: 2}
    MemorySegment:
      raw_i32: {0: Global, 1: ReadOnly, 2: Private, 3: Group}
      unknown: Error::InvalidAttribute(field="HSA_AMD_MEMORY_POOL_INFO_SEGMENT")
    MemoryLocation:
      raw_i32: {0: Cpu, 1: Gpu}
      unknown: Error::InvalidAttribute(field="HSA_AMD_MEMORY_POOL_INFO_LOCATION")
  c_bool:
    raw_u8: {0: false, 1: true}
    unknown: Error::InvalidAttribute(field=caller_field)
  memory_pool_flags:
    storage: one u32, including unknown future bits
    known_bits:
      KERNARG_INITIALIZATION: abi::MEMORY_POOL_GLOBAL_FLAG_KERNARG_INIT
      FINE_GRAINED: abi::MEMORY_POOL_GLOBAL_FLAG_FINE_GRAINED
      COARSE_GRAINED: abi::MEMORY_POOL_GLOBAL_FLAG_COARSE_GRAINED
      EXTENDED_SCOPE_FINE_GRAINED: abi::MEMORY_POOL_GLOBAL_FLAG_EXTENDED_FINE_GRAINED
    contains(flag): (bits & flag) == flag
    unknown_bits: bits & !KNOWN
  agent_uuid:
    grammar: DEVICE-BODY
    device: exactly CPU | GPU | DSP | AIE
    body: exactly XX, or exactly 16 ASCII hexadecimal digits
    parsed_digits: 16 digits -> eight bytes, two digits per byte
    display: original input string, unchanged
  amd_isa:
    prefix: exactly amdgcn-amd-amdhsa--
    architecture: gfx + nonempty lowercase ASCII letters/digits/hyphens
    feature: name + sign, where name is nonempty lowercase ASCII letters/digits/underscore
    sign: exactly + or -
    feature_order: retained; duplicate names rejected
    display: original input string, unchanged
  pci_address:
    source: HSA domain plus low BDF bits
    bus: bdf_id bits 8..15
    device: bdf_id bits 3..7
    function: bdf_id bits 0..2
    display: lowercase domain:bus:device.function, zero-padded minimum widths
  hashing_and_equality:
    rule: all derives are structural; identity.rs defines no custom hash
    consequence: raw strings, feature order, and unknown flag bits remain identity data
```

The raw conversions are `pub(crate)` because only the reviewed discovery and
session paths may interpret ROCr values. The parsed identity types and their
accessors are public so callers can retain exact identities without rebuilding
or re-parsing strings. `Error::InvalidAttribute` and
`Error::InvalidIdentity` are the only errors created directly by this module.

## Structure

The source is intentionally organized from scalar ABI values to compound
identities (`hsa/src/identity.rs` line ranges are included for navigation):

| Source range | Type or function | Representation and purpose |
| --- | --- | --- |
| 5-28 | `DeviceType` | Agent class enum and raw HSA agent-device conversion. |
| 30-56 | `Profile` | Base/full executable profile and two-way raw mapping. |
| 58-87 | `QueueKind` | Queue producer discipline and two-way raw mapping. |
| 89-112 | `MemorySegment` | Global, read-only, private, or group pool segment. |
| 114-133 | `MemoryLocation` | Optional CPU or GPU pool location. |
| 135-157 | `MemoryPoolFlags` | Opaque u32 flag set with known-bit queries. |
| 159-187 | `RoundingModes`, `c_bool` | Three boolean ISA rounding capabilities and strict C-bool decoding. |
| 189-279 | `AgentUuidBody`, `AgentUuid` | Device-prefixed stable-or-unavailable agent identity. |
| 281-383 | `IsaFeature`, `IsaTarget` | Exact AMD HSA target with ordered feature modifiers. |
| 385-412 | `PciAddress` | Numeric PCI domain/BDF identity and canonical display. |

Every type derives `Clone`, `Debug`, `Eq`, and `Hash` (with `Copy` where its
representation is copyable), and `PartialEq`. No type has a hand-written
`Hash`, `Ord`, digest, serialization, normalization, or cross-runtime lookup.
The derived implementation compares all stored fields, including private raw
strings and unknown flag bits.

## Raw enum and boolean conversion

### `DeviceType`

`DeviceType` classifies an HSA agent as `Cpu`, `Gpu`, `Dsp`, or `Aie`. The
`from_raw(i32)` conversion accepts the ROCr values 0 through 3. Any other value
returns `Error::InvalidAttribute` with the fixed field
`"HSA_AGENT_INFO_DEVICE"`. A signed negative value is represented in the
error after the source code casts it to `u32` and then `u64`; it is not treated
as a separate negative case.

`discovery::discover_agent` obtains the raw value from
`AGENT_INFO_DEVICE` before constructing any other agent identity. The result is
stored in `AgentDescription.device_type`. `AgentUuid::from_str` later produces
a device type from the UUID prefix, and discovery rejects a mismatch between
the two values. `Session::into_session` accepts only `DeviceType::Gpu`.
`native-probe` partitions CPU agents for host allocation and filters GPU agents
for device descriptors. `native-executor::hsa::validate_binding` requires the
selected host allocator to be `Cpu`.

### `Profile`

`Profile::from_raw(i32, field)` accepts 0 as `Base` and 1 as `Full`, using the
caller-supplied field label in `Error::InvalidAttribute` for unknown values.
Discovery passes `"HSA_AGENT_INFO_PROFILE"` and stores the result in
`AgentDescription.profile`. `Profile::as_raw` is the exact reverse mapping.
`Session::load_hsaco` passes that raw value to
`hsa_executable_create_alt`; the profile therefore controls the HSA executable
creation call and is not merely descriptive metadata.

### `QueueKind`

`QueueKind::from_raw(u32)` maps 0 to `MultiProducer`, 1 to `SingleProducer`,
and 2 to `Cooperative`. Unknown values return
`Error::InvalidAttribute` with `"HSA_AGENT_INFO_QUEUE_TYPE"`. `as_raw` maps
the enum back to the same values for `hsa_queue_create`.

Discovery reads the advertised kind only when the three queue limits are not
all zero, then stores it in `QueueCapabilities.advertised_kind`. Queue creation
allows a requested kind according to that advertised kind:

```text
advertised SingleProducer -> only requested SingleProducer
advertised MultiProducer  -> requested MultiProducer or SingleProducer
advertised Cooperative    -> only requested Cooperative
```

`Session::create_queue` sends the requested kind through `as_raw`, then parses
the returned queue object's kind with `from_raw`. A returned unknown kind,
zero or non-power-of-two size, null base address, or missing kernel-dispatch
feature produces `Error::InvalidQueueReturned` after the newly created queue is
destroyed when possible. The queue retains the requested kind in `QueueCore`
because ROCr may report its read-only protocol field as `MultiProducer` even
when `SingleProducer` was requested. Both `dispatch_prepared` and
`dispatch_after` require the retained kind to be `SingleProducer`, otherwise
they return `Error::InvalidDispatch`.

The native probe and HSA smoke workload request `SingleProducer` queues. The
native executor creates one such queue for each realized HSA queue slot. A
requested kind that is not allowed by discovery returns
`Error::UnsupportedQueueKind` before the FFI call.

### `MemorySegment` and `MemoryLocation`

`MemorySegment::from_raw(i32)` accepts 0 `Global`, 1 `ReadOnly`, 2 `Private`,
and 3 `Group`, reporting unknown values against
`"HSA_AMD_MEMORY_POOL_INFO_SEGMENT"`. `MemoryLocation::from_raw(i32)` accepts
0 `Cpu` and 1 `Gpu`, reporting unknown values against
`"HSA_AMD_MEMORY_POOL_INFO_LOCATION"`. Both use the same signed-to-unsigned
error conversion as `DeviceType`.

`discovery::discover_memory_pool` requires a valid segment. It treats location
as optional because `pool_info_optional` converts only
`STATUS_ERROR_INVALID_ARGUMENT` into `None`; a returned value is still decoded
strictly by `MemoryLocation::from_raw`. The segment controls whether global
flags are queried: `global_flags` is `Some` only for `Global`, and `None` for
every other segment.

HSA allocation helpers select only `Global` pools with runtime allocation and a
matching flag. The native probe's capacity path further requires
`location == Some(MemoryLocation::Gpu)` for local GPU capacity. A missing
optional location therefore prevents that pool from being selected and lets
the probe use its documented AMD available-memory fallback when applicable.

## Memory-pool flags and C booleans

### `MemoryPoolFlags`

`MemoryPoolFlags` is an opaque wrapper around one raw `u32`.
`from_raw` stores every bit, including bits unknown to this version of Recipe;
it never masks, rejects, or rewrites a newer ROCr capability report. The four
known constants are imported from `hsa/src/abi.rs`:

| Constant | ABI bit | Meaning used by callers |
| --- | ---: | --- |
| `KERNARG_INITIALIZATION` | 1 | Pool may provide initialized kernarg memory. |
| `FINE_GRAINED` | 2 | Pool is eligible for coherent fine-grained allocation. |
| `COARSE_GRAINED` | 4 | Pool is eligible for coarse-grained allocation. |
| `EXTENDED_SCOPE_FINE_GRAINED` | 8 | Pool is accepted as fine-grained by Recipe. |

`bits()` returns the unchanged raw value. `contains(flag)` is the exact subset
test `(self.bits() & flag) == flag`; callers pass one of the nonzero known
constants. `unknown_bits()` returns all bits outside the four-bit `KNOWN` mask.
The wrapper derives `Default`, which is an all-zero flag set, and structural
`Eq`/`Hash` over all 32 bits.

The discovery record stores flags for global pools. `hsa/src/execution.rs`
uses them in `matching_pool` and the `allocate_coarse`, `allocate_fine`, and
`allocate_kernarg` methods on both `DiscoveredAgent` and `Session`:

```text
coarse -> Global + runtime_allocation + COARSE_GRAINED
fine   -> Global + runtime_allocation + FINE_GRAINED or EXTENDED_SCOPE_FINE_GRAINED
kernarg -> Global + runtime_allocation + KERNARG_INITIALIZATION
```

The dispatch paths additionally require a supplied kernarg allocation to retain
the kernarg flag, to be accessible by the queue agent, large enough, and
aligned. A missing matching pool produces `Error::NoMatchingMemoryPool`; an
invalid index or nonallocatable pool is rejected by the lower allocation path.
Unknown bits do not make a pool invalid and do not satisfy any known predicate.

`native-probe/src/hsa.rs::hsa_capacity` selects the largest allocatable global,
GPU-located, coarse-grained pool as the local capacity source. The native probe
binding path requires a CPU allocator with both an allocatable kernarg pool and
an allocatable fine or extended-fine pool. `native-executor/src/hsa.rs` repeats
the same requirements in `validate_binding`; it returns an artifact-mismatch
error if the reopened allocator no longer has those identities/capabilities.

### `RoundingModes` and `c_bool`

`RoundingModes` is a public three-field capability record:

```text
default       <- values[0]
toward_zero   <- values[1]
nearest_even  <- values[2]
```

`from_c_array(field, [u8; 3])` calls `c_bool` for each byte. `c_bool` accepts
only 0 and 1 and otherwise returns `Error::InvalidAttribute` with the caller's
field and the byte value. The same field label is used for all three array
positions, so the caller must use the surrounding attribute context when
diagnosing an invalid index.

Discovery applies this strict conversion to ISA machine-model support,
base/full profile support, default and base-profile rounding modes, fast f16,
memory-pool runtime-allocation permission, and all-agent accessibility. The
execution path applies it to the kernel dynamic-callstack symbol attribute.
An invalid boolean is a hard discovery or kernel-metadata failure, never a
truthy fallback.

## Agent UUID identity

### Representation

`AgentUuid` stores three values:

```text
raw: String                 # exact caller/ROCr text
device_type: DeviceType     # decoded uppercase prefix
body: AgentUuidBody          # Unavailable or eight parsed bytes
```

`AgentUuidBody::Unavailable` represents the literal body `XX`. A
`Value([u8; 8])` body is produced from sixteen hexadecimal digits, with each
adjacent pair decoded high nibble first. `body()` copies the enum, while
`device_type()` and `as_str()` expose the parsed class and exact original text.
`Display` writes `raw` without canonicalizing hexadecimal case or changing the
prefix.

### Grammar and validation

`FromStr for AgentUuid` performs these checks in order:

1. `split_once('-')` must find one separator. Missing it returns
   `InvalidIdentity(kind="agent UUID", reason="expected DEVICE-BODY")`.
2. The body after the first separator must not contain another `-`; an extra
   separator returns `reason="contains an extra separator"`.
3. The prefix must be exactly `CPU`, `GPU`, `DSP`, or `AIE`; unknown or
   differently cased prefixes return `reason="unknown device prefix"`.
4. The body is `Unavailable` only for exact uppercase `XX`. Every other body
   must be exactly 16 bytes and every byte must satisfy
   `is_ascii_hexdigit()`. Failure returns
   `reason="body must be XX or exactly 16 hexadecimal digits"`.
5. The validated body is split into eight two-byte chunks and decoded by
   `hex_nibble`. The helper is `unreachable!` for non-hex input, but validation
   occurs first, so a normal invalid string returns the typed error.

The parser does not trim whitespace, accept lowercase `xx`, accept a UUID with
more or fewer than one separator, or normalize hexadecimal case. `AgentUuid`
derives `Eq` and `Hash` over `raw`, `device_type`, and `body`. Consequently,
two strings with the same numeric bytes but different raw hexadecimal casing
are different `AgentUuid` values even though their `AgentUuidBody::Value`
values compare equal. This is intentional: the raw ROCr spelling is retained as
the external identity key.

### Production and consumption

`discovery::discover_agent` reads the fixed-size AMD UUID attribute through
`agent_string::<21>`, parses it, and stores it in `AgentIdentity.uuid`. The
fixed array must contain a NUL terminator and valid UTF-8; missing NUL is an
`InvalidAttribute`, invalid UTF-8 is `InvalidUtf8`, and the parser's malformed
text errors propagate unchanged. `agent_string` keeps the bytes before the
first NUL and does not validate bytes after it. Discovery then compares the UUID's decoded
device type with `HSA_AGENT_INFO_DEVICE`; disagreement returns
`InvalidIdentity(reason="device prefix disagrees with HSA_AGENT_INFO_DEVICE")`.

The HSA native probe uses `AgentUuidBody::Value` as the stable-ID gate for GPU
descriptors. `Unavailable` is not accepted for a measured GPU. A descriptor's
key is exactly:

```text
hsa:{agent_uuid.Display}@{PciAddress.Display}
```

The key is retained in `GpuDescriptor`, included in probe cache identity input,
looked up during reopen, and compared byte-for-byte through the retained raw
UUID spelling. Rediscovery selects the GPU by `uuid.as_str()` and rejects a
missing or nonunique match. CPU agents are not selected by the GPU key, but
their UUIDs participate in deterministic host-allocator ordering together with
NUMA node, name, and vendor name.

## AMD ISA identity

### Representation

`IsaFeature` stores a private feature `name: String` and an `enabled: bool`.
`name()` borrows the original validated name and `enabled()` reports whether its
modifier was `+` (`true`) or `-` (`false`).

`IsaTarget` stores:

```text
raw: String                    # exact ROCr ISA name
architecture: String           # first colon-delimited component
features: Vec<IsaFeature>      # remaining components, in source order
```

It is documented as an exact identity, not a normalized capability. `as_str`
and `Display` return `raw`; `architecture()` and `features()` expose the
parsed views. Structural equality and hashing include the raw string, the
architecture, and the ordered feature vector.

### Grammar and validation

`FromStr for IsaTarget` requires the exact triple prefix
`amdgcn-amd-amdhsa--`. The remaining tail is split on `:`. The first component
must begin with lowercase `gfx`, have a nonempty suffix, and contain only
lowercase ASCII letters, ASCII digits, or `-`. This accepts identities such as
`gfx90a` and `gfx10-3`, but rejects `gfx`, uppercase spellings, underscores,
and any other punctuation.

Each later component is a feature modifier. It must contain at least two bytes;
the final byte is removed as the sign, and the name before it must be nonempty
and contain only lowercase ASCII letters, ASCII digits, or `_`. The sign must
be `+` or `-`. A name already present in the feature vector is rejected, even
if the second modifier uses the opposite sign. The source order is retained.

The parser returns `InvalidIdentity(kind="AMD ISA")` with one of these reasons:

```text
expected amdgcn-amd-amdhsa-- prefix
architecture must be a lowercase gfx target identity
feature must have a name followed by + or -
malformed feature modifier
duplicate feature modifier
```

As written, the implementation calls `component.split_at(component.len() - 1)`
before validating that the component is ASCII. A feature component ending in a
multi-byte UTF-8 character can therefore panic on a non-character byte
boundary instead of returning `InvalidIdentity`. ASCII malformed input follows
the typed error paths above. This is an implementation failure mode, not an
accepted identity form.

### Production and consumption

`discovery::discover_isa` reads each ISA name after bounded allocation and NUL
handling. It accepts either ROCr's length-without-NUL or length-with-trailing-
NUL convention, truncates at the first NUL, rejects nonzero bytes after that
NUL, and rejects invalid UTF-8. For GPU agents it immediately parses the name
as `IsaTarget`; non-GPU agents retain the raw ISA name but set
`IsaDescription.amd_target` to `None`.

`DiscoveredAgent::into_session` requires a GPU to expose at least one ISA and
requires every discovered ISA to have an `amd_target`. Thus a GPU with no exact
AMD target returns `Error::UnsupportedAgent` before a session is created.

`native-probe::exact_target` selects the artifact target from all parsed GPU
ISAs. It first rejects any non-AMDGPU target, then computes the distinct raw
target strings and a list of targets whose architecture does not end in
`-generic`:

```text
one non-generic target                  -> select it
no non-generic target, one raw identity -> select the sole identity
no targets                              -> discovery error: no artifact target
more than one non-generic target       -> discovery error with the count
multiple generic raw identities        -> discovery error: ambiguous generic targets
```

Duplicate occurrences of the same non-generic target count as more than one in
the `specific` list and are therefore rejected as ambiguous. If one specific
target exists, extra generic targets do not change the selection. The selected
raw string becomes the HSA `TargetIdentity.architecture` and the binding's
`target_id`; `hsa_target_tail` removes the exact triple prefix when constructing
the AMD kernel target.

During native realization, `native-probe/src/bindings.rs` stores that exact
target string in each `RealizedHsa`. `native-executor::hsa::validate_binding`
requires the reopened session to advertise an ISA whose `as_str()` exactly
equals the binding target. The runtime artifact's target and code-object
version are separately checked against the binding before HSACO inspection and
loading. A changed or ambiguous ISA identity is an artifact/discovery failure,
not a fallback to a generic target.

## PCI identity

`PciAddress` is a four-field numeric value with public fields:

```text
domain: u32
bus: u8
device: u8
function: u8
```

`from_hsa(domain, bdf_id)` extracts bus, device, and function from the low BDF
bits shown in the contract block. Bits above the extracted fields are ignored
and the domain is retained unchanged. The constructor is `pub(crate)` and is
called only for GPU agents while discovery reads the AMD domain and BDF
attributes. There is no range or consistency check beyond the field widths.

`Display` emits `{:04x}:{:02x}:{:02x}.{}` using lowercase hexadecimal and a
minimum width, so a domain larger than four hex digits is not truncated. The
native HSA probe uses this exact spelling for PCI sysfs lookup and for the GPU
descriptor key. `PciAddress` also derives structural equality and hashing; no
string round trip is used to compare numeric addresses.

## Agent-description construction pipeline

`hsa/src/discovery.rs::discover_agent` is the sole producer of a complete
`AgentDescription`. The identity values are constructed in this order:

```text
raw HSA agent
  -> DeviceType::from_raw(AGENT_INFO_DEVICE)
  -> feature_bits (retained raw bit field)
  -> Profile::from_raw(AGENT_INFO_PROFILE, "HSA_AGENT_INFO_PROFILE")
  -> AgentUuid::from_str(AMD_AGENT_INFO_UUID string)
       -> compare UUID prefix device type with DeviceType
  -> queue limits and QueueKind::from_raw, then queue-limit validation
  -> for GPU: driver node, domain, BDF -> PciAddress, AMD GPU properties
  -> iterate ISAs -> bounded name read -> optional IsaTarget parse and booleans
  -> iterate memory pools -> MemorySegment, optional MemoryLocation,
       global MemoryPoolFlags, allocation and accessibility booleans
  -> optional first-ISA wavefront size when kernel-dispatch feature is present
  -> AgentIdentity + AgentDescription
```

The surrounding `AgentIdentity` retains the raw name, vendor name, UUID, NUMA
node, optional KFD driver node, and optional PCI address. GPU-only properties
are `None` for non-GPU agents. Identity conversion errors abort discovery of
that agent and therefore abort the `Runtime::discover` result; there is no
partially accepted agent record.

Queue capability construction has one deliberate absence rule: if maximum
queues, minimum packets, and maximum packets are all zero, `queue` is `None`
and no queue kind is queried. If any one is nonzero, all three must be nonzero,
both packet limits must be powers of two, and minimum must not exceed maximum.
An inconsistent tuple returns `Error::InvalidQueueSize`. This validation is
separate from `QueueKind::from_raw`, so an invalid kind is reported only after
the limits make a queue capability record eligible for construction.

Memory-pool optional attributes use a narrower absence rule. A location or
maximum-allocation query is absent only when ROCr returns
`STATUS_ERROR_INVALID_ARGUMENT`. Any other HSA status is an `Error::Hsa`, and
any returned location value still passes strict `MemoryLocation::from_raw`.

## Consumers and enforcement boundary

The identity types remain data until a consumer applies a domain rule. The
current consumers are:

| Consumer | Identity values | Enforcement |
| --- | --- | --- |
| `hsa/src/session.rs` | `DeviceType`, `IsaTarget`, `QueueKind` | GPU-only session admission, exact AMD ISA requirement, queue size/kind checks, requested-kind retention, returned queue validation. |
| `hsa/src/execution.rs` | `Profile`, `MemorySegment`, `MemoryPoolFlags`, `QueueKind`, `c_bool` | HSA executable profile, allocation class selection, kernarg flag/access checks, single-producer dispatch requirement, kernel boolean decoding. |
| `native-probe/src/hsa.rs` | `AgentUuidBody`, `PciAddress`, `IsaTarget`, `MemoryLocation`, `MemorySegment`, `MemoryPoolFlags`, `QueueKind`, `DeviceType` | Stable GPU key, PCI sysfs origin, one exact artifact target, capacity source, CPU/GPU agent selection, benchmark queue. |
| `native-probe/src/bindings.rs` | `DeviceType`, `MemorySegment`, `MemoryPoolFlags`, agent NUMA and UUID strings | CPU host allocator partitioning and deterministic order, fine/kernarg capability, exact reopen and host allocator selection. |
| `native-executor/src/hsa.rs` | `DeviceType`, `MemorySegment`, `MemoryPoolFlags`, exact target string, `QueueKind` | Reopened allocator and target validation, runtime artifact match, single-producer queue creation. |
| `hsa/examples/execute_smoke.rs` | `DeviceType`, `QueueKind` | Picks first CPU/GPU agents and requests a single-producer queue for diagnostic execution. |

`hsa/src/lib.rs` exposes the identity types but not the raw conversion
functions. Higher layers therefore cannot silently reinterpret a ROCr integer
with a second mapping; they must use the already validated discovery record.

## Hashing, equality, and identity propagation

There is no cryptographic hashing in this module. The `Hash` derives are useful
for maps and sets and are exactly the same structural equality relation:

* Scalar enums hash their variant.
* `MemoryPoolFlags` hashes all 32 raw bits, including unknown bits.
* `RoundingModes` hashes all three booleans.
* `AgentUuidBody::Value` hashes the eight decoded bytes.
* `AgentUuid` hashes `raw`, `device_type`, and `body`, so raw spelling is part
  of equality even when decoded bytes are the same.
* `IsaFeature` hashes its name and sign.
* `IsaTarget` hashes `raw`, `architecture`, and the ordered feature vector.
* `PciAddress` hashes the four numeric fields.

The probe layer turns these low-level identities into persisted higher-level
identity input without rehashing the HSA objects directly. In particular,
`native-probe::HsaBackend::descriptor` forms `GpuDescriptor.key` from the raw
UUID display and canonical PCI display, and `probe::engine::build_cache_identity`
hashes that key, target labels, driver/runtime/firmware/link labels, and
toolchain identity into `CacheIdentity`. The measured profile's topology and
discovery digests then include that cache digest. A UUID spelling change, a PCI
address change, a target change, or a driver/runtime identity change therefore
invalidates the cache and the later profile identities through normal canonical
digest construction.

`native-probe::NativeGpuProbe::discover_all` sorts descriptor keys and rejects
duplicate adjacent keys. `benchmark_gpu` re-discovers every backend and
requires exactly one backend to emit a descriptor equal to the measured one;
missing or changed identity is a benchmark error, and two claiming backends is
also an error. `native-probe::realize_hsa` uses a `BTreeSet` of expected keys to
reject duplicate reopened GPUs and requires every expected key to be reopened.
These are consumer-level identity validations, not alternate hash functions.

The exact ISA target is similarly propagated as a string into
`TargetIdentity.architecture`, the native artifact identity, and the HSA
binding. The native executor compares the string exactly and does not infer
compatibility from architecture prefixes, feature subsets, or a generic target.

## Failure map

The following table distinguishes conversion failures from later capability or
identity failures:

| Stage | Condition | Result |
| --- | --- | --- |
| Raw enum conversion | Unknown device, profile, queue kind, segment, or location code | `Error::InvalidAttribute` with the corresponding ABI field. |
| C boolean conversion | Any value other than 0 or 1 | `Error::InvalidAttribute`; discovery or metadata construction stops. |
| UUID string read | No NUL in the fixed agent buffer | `Error::InvalidAttribute` with the field and buffer width. |
| UUID string read | Invalid UTF-8 before the NUL | `Error::InvalidUtf8`. |
| UUID parser | Missing separator, extra separator, unknown prefix, or invalid body | `Error::InvalidIdentity(kind="agent UUID", precise reason)`. |
| Agent cross-check | UUID prefix class differs from `HSA_AGENT_INFO_DEVICE` | `Error::InvalidIdentity` with the disagreement reason. |
| ISA name read | Length above 4096, allocation failure, nonzero data after NUL, invalid UTF-8 | `Error::InvalidAttribute`, `AllocationFailed`, or `InvalidUtf8`. |
| ISA parser | Wrong triple, malformed gfx architecture, malformed feature, duplicate feature | `Error::InvalidIdentity(kind="AMD ISA", precise reason)`. |
| ISA parser edge | Non-ASCII feature ending reaches `split_at` at a non-character boundary | Panic in the current implementation, not a typed error. |
| Queue capability discovery | Partially zero, non-power-of-two, or reversed queue limits | `Error::InvalidQueueSize`. |
| Queue construction | Requested size outside discovered range or not a power of two | `Error::InvalidQueueSize`. |
| Queue construction | Requested producer discipline not allowed by advertised kind | `Error::UnsupportedQueueKind`. |
| Queue construction | ROCr returns null or malformed queue fields | `Error::NullQueue` or `Error::InvalidQueueReturned`; newly created resources are destroyed when safe. |
| Session admission | Agent is not GPU, lacks kernel-dispatch bit, or lacks exact AMD ISA records | `Error::UnsupportedAgent`. |
| Pool selection | No allocatable global pool satisfies coarse, fine, or kernarg flags | `Error::NoMatchingMemoryPool`. |
| Pool metadata | Invalid segment/location or invalid allocation/accessibility boolean | `Error::InvalidAttribute`. |
| Pool optional query | ROCr returns invalid argument | `None` for that optional attribute; no guessed value is inserted. |
| Allocation/dispatch | A pool index is invalid, nonallocatable, or a kernarg allocation lacks the kernarg flag | `Error::InvalidMemoryPoolIndex`, `MemoryPoolNotAllocatable`, or `InvalidDispatch`. |
| Native HSA descriptor | GPU UUID unavailable, PCI absent, target ambiguous, or required capabilities absent | `ProbeError::Discovery`; no descriptor is emitted. |
| Native reopen | Descriptor key changed, duplicated, missing, or claimed by multiple backends | `ProbeError::Discovery` or `ProbeError::Benchmark`. |
| Native executor binding | Host allocator class/pools or exact target no longer match | `Error::ArtifactMismatch`; execution does not fall back to another agent or target. |

The parser errors are deliberately specific because they identify malformed
external identity. Later errors preserve the distinction between an identity
that is syntactically valid but unavailable (`AgentUuidBody::Unavailable`), a
capability that is absent, and a valid measured identity that changed during
reopen. No caller turns an invalid identity into a synthetic UUID, a generic
ISA, an arbitrary queue kind, or a guessed memory location.

## Source-of-truth map

```text
hsa/src/identity.rs       conversion, parsing, representation, Display, derives
hsa/src/discovery.rs      ROCr producers and AgentDescription assembly
hsa/src/session.rs        GPU admission, queue request and returned-kind validation
hsa/src/execution.rs      profile use, pool selection, kernarg and dispatch checks
hsa/src/error.rs          InvalidIdentity, InvalidAttribute, queue and agent diagnostics
native-probe/src/hsa.rs   GPU key, exact target, capacity, benchmark and UUID selection
native-probe/src/bindings.rs
                          reopened-agent matching and CPU host allocator selection
native-probe/src/native.rs
                          descriptor uniqueness and identity-change checks
native-executor/src/hsa.rs
                          allocator/target validation and single-producer queue realization
hsa/src/lib.rs            public re-export surface
```

The identity module should remain a single conversion and representation layer.
New ROCr values must either be added to the explicit mapping with a reviewed
consumer rule or remain rejected/retained according to the existing policy.
Adding a fallback parser, normalizing raw identity strings, or duplicating the
same mapping in a caller would break the exact reopen and profile-identity
guarantees described above.

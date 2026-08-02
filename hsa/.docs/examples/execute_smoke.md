```toml
[example]
path = "hsa/examples/execute_smoke.rs"
crate = "recipe-hsa"
entrypoint = "main() -> Result<(), Box<dyn std::error::Error>>"
required_feature = "live-hsa"
default_branch = "CPU fine allocation -> GPU coarse allocation -> CPU fine allocation"
optional_branch_trigger = "RECIPE_HSA_SMOKE_COPY_HSACO is present"
optional_branch_symbol = "RECIPE_HSA_SMOKE_SYMBOL is required when the HSACO path is present"
completion = "all submitted operations complete, allocations and executable close, runtime shuts down"
acceptance = false

[inputs]
bytes = "u8 values 0 through 255 inclusive, exactly 256 bytes"
copy_offsets = "source and destination offsets are zero"
copy_size = "bytes.len(), exactly 256"
wait_timeout = "5 seconds for every Pending token"
dispatch_grid = "bytes.len() / 4, exactly 64"
dispatch_workgroup = "64"

[outputs]
default_stdout = "ROCr fine→coarse→fine asynchronous copy smoke passed"
optional_stdout = "optional HSACO kernel dispatch smoke passed"
success_status = "process returns Ok(())"
failure_status = "the first error is boxed and returned; later steps are not attempted"
```

# hsa/examples/execute_smoke.rs

## Purpose and source contract

This file is a standalone diagnostic binary for the public recipe-hsa HSA
surface. It opens the dynamically loaded ROCr runtime, discovers agents and
memory pools, selects the first discovered CPU and GPU, creates a GPU session,
performs two asynchronous copies through CPU fine-grained and GPU
coarse-grained allocations, and independently compares the returned bytes.
When both smoke environment variables are configured, it additionally loads an
in-memory HSACO object, resolves one kernel, dispatches that kernel behind an
HSA dependency barrier, copies the kernel output back, and compares it with the
same 256-byte input.

The source has one production path in main
(hsa/examples/execute_smoke.rs:5-136). Main uses ? for every fallible
operation. The first error returns a boxed recipe_hsa::Error or
std::io::Error, so no later operation or success line is executed. There is no
test-only entrypoint, fake runtime, synthetic HSA object, or proxy around the
public calls. Cargo.toml registers the example with
required-features = ["live-hsa"] (hsa/Cargo.toml:13-16).

The README command must be run from the hsa package directory:

```text
cargo run --features live-hsa --example execute_smoke
```

From the workspace root, the equivalent package-qualified command is:

```text
cargo run -p recipe-hsa --features live-hsa --example execute_smoke
```

This example is diagnostic only. A successful process and its success lines
prove the measured copy or optional kernel path on that invocation, but are not
the complete real-data Recipe acceptance gate (hsa/README.md:37-43).

## Required runtime and environment

recipe-hsa is compiled only for the 64-bit HSA large-model ABI. On a
non-64-bit target, hsa/src/lib.rs:7-10 emits a compile error. A live run also
requires an installed ROCr shared library exporting every symbol loaded by
hsa/src/loader.rs:168-270, a visible CPU agent, a visible GPU agent, and the
GPU capabilities and allocatable pools described below. Compilation alone does
not prove any of those runtime conditions.

The optional branch is selected by the presence of
RECIPE_HSA_SMOKE_COPY_HSACO, read with std::env::var_os at
hsa/examples/execute_smoke.rs:58. Its value is an operating-system path and is
passed to std::fs::read; it may be non-UTF-8. If the variable is absent, the
loader, queue, dispatch, and verification code in lines 58-126 is skipped.
When it is present, RECIPE_HSA_SMOKE_SYMBOL is read with std::env::var
(:59-60). Missing and non-UTF-8 values both become the same I/O error,
RECIPE_HSA_SMOKE_SYMBOL is required. The symbol is not inferred from the file
and there is no fallback symbol name.

The HSACO must contain a kernel for the selected GPU agent whose first two
kernarg fields are destination and source pointers and whose body copies the
256 input bytes from the source allocation to the output allocation. The Rust
smoke checks only that ROCr reports kernarg_segment_size >= 16; it does not
inspect code semantics, element type, or bounds. The kernel must therefore
honor the exact dispatch and argument contract described in the optional path.

## End-to-end call graph

The real public path is:

```text
Runtime::open_default
  -> Runtime::open
  -> Api::load
  -> dlopen, dlsym for all required ROCr symbols, hsa_init
Runtime::discover
  -> discovery::discover
  -> hsa_system_get_info
  -> hsa_iterate_agents and per-agent ISA/pool enumeration
first CPU and first GPU DiscoveredAgent
  -> GPU DiscoveredAgent::into_session
  -> Session { SharedFault, SignalPool }
CPU/GPU allocations and access grants
  -> hsa_amd_memory_pool_allocate
  -> hsa_amd_agents_allow_access
host seed
  -> Allocation::copy_from_host
upload and download
  -> Session::copy_async
  -> hsa_amd_memory_async_copy + completion signal
  -> Pending::wait / hsa_signal_wait_scacquire
  -> Allocation::copy_to_host + byte equality
optional HSACO branch
  -> Session::load_hsaco
  -> Executable::kernel
  -> Session::create_queue
  -> prerequisite Session::copy_async
  -> Queue::dispatch_after
  -> barrier-and packet -> kernel packet
  -> Pending::wait for dispatch and prerequisite
  -> Queue::close
  -> verification Session::copy_async and byte equality
explicit close and drop order
  -> allocation free, executable/reader destroy, queue destroy,
     signal-pool retirement, hsa_shut_down
```

The HSA loader is not a process-global singleton. Runtime owns one Arc<Api>
and one balanced hsa_init reference; child objects borrow the runtime and
retain API arcs as needed. Rust lifetimes prevent runtime.close() while a
discovered agent, session, allocation, queue, executable, or pending token is
still borrowed.

## Runtime open and loader path

### Runtime::open_default

Runtime::open_default (hsa/src/runtime.rs:20-32) tries these candidates in
order:

1. libhsa-runtime64.so.1
2. libhsa-runtime64.so

It retries only Error::LibraryOpen. A missing library therefore advances to
the next candidate, and if both fail the error from the second candidate is
returned. A missing symbol, a failed hsa_init, or any other error stops the
candidate loop immediately. The static candidate names cannot contain an
interior NUL.

### Runtime::open and Api::load

Runtime::open calls Api::load (hsa/src/runtime.rs:34-50). Library::open uses
dlopen(path, RTLD_NOW | RTLD_LOCAL) and reports Error::LibraryOpen with the
copied dlerror text when the handle is null. Api::load then calls dlsym for the
complete ABI set. Each lookup clears and immediately reads dlerror; a missing
or null address is Error::MissingSymbol with the library name, symbol name, and
loader detail. The function pointers are transmuted only after the static ABI
aliases in hsa/src/abi.rs have fixed their signatures.

The required symbol set is:

```toml
required_symbols = [
  "hsa_init",
  "hsa_shut_down",
  "hsa_status_string",
  "hsa_system_get_info",
  "hsa_iterate_agents",
  "hsa_agent_get_info",
  "hsa_agent_iterate_isas",
  "hsa_isa_get_info_alt",
  "hsa_queue_create",
  "hsa_queue_destroy",
  "hsa_amd_agent_iterate_memory_pools",
  "hsa_amd_memory_pool_get_info",
  "hsa_code_object_reader_create_from_memory",
  "hsa_code_object_reader_destroy",
  "hsa_executable_create_alt",
  "hsa_executable_destroy",
  "hsa_executable_load_agent_code_object",
  "hsa_executable_freeze",
  "hsa_executable_get_symbol_by_name",
  "hsa_executable_symbol_get_info",
  "hsa_amd_memory_pool_allocate",
  "hsa_amd_memory_pool_free",
  "hsa_amd_agents_allow_access",
  "hsa_amd_memory_async_copy",
  "hsa_signal_create",
  "hsa_signal_destroy",
  "hsa_signal_load_scacquire",
  "hsa_signal_store_screlease",
  "hsa_signal_wait_scacquire",
  "hsa_queue_load_read_index_scacquire",
  "hsa_queue_load_write_index_relaxed",
  "hsa_queue_store_write_index_screlease",
]
```

After all symbols resolve, Runtime::open calls hsa_init. A non-success status
returns Error::Hsa { operation = "hsa_init", message = None }, because status
text is not safe to query before initialization. On success it returns
Runtime { active = true }. Api::check normally converts nonzero statuses to
Error::Hsa and asks hsa_status_string for a copied message. The async-copy
submission uses Api::check_status_only so a post-realization submit error
retains only the numeric status and does not allocate a diagnostic string.

## Discovery, selection, and session creation

runtime.discover() first calls Runtime::ensure_active, then
discovery::discover (hsa/src/runtime.rs:52-56). Discovery records the current
system HSA and AMD extension versions and timestamp frequency. It collects
agents through a synchronous callback, then builds one DiscoveredAgent per
handle. Every agent is queried for device kind, feature bits, profile, UUID,
queue limits and kind, identity strings, ISA descriptions, and memory pools.
GPU agents additionally expose AMD PCI, chip, clock, memory, scratch, SDMA, and
XCC properties. ISA names are parsed as exact AMD targets for GPUs; memory pool
descriptions retain global flags, runtime-allocation limits, alignment, location,
and accessibility. Queue ranges must be nonzero powers of two with
minimum_packets <= maximum_packets, or discovery returns
Error::InvalidQueueSize.

The callback collectors convert a panic while pushing a handle into
Error::CallbackPanicked and always check the final ROCr traversal status.
Malformed raw values become InvalidAttribute, InvalidUtf8, or InvalidIdentity;
checked vector reservations can return AllocationFailed. An optional pool
attribute that ROCr rejects with STATUS_ERROR_INVALID_ARGUMENT is recorded as
None, not guessed. The discovery result owns no runtime shutdown; its agents
borrow the active Runtime.

discovery.into_agents() consumes the result and returns the discovered vector
(hsa/src/discovery.rs:146-152). The example scans it in ROCr enumeration order
(hsa/examples/execute_smoke.rs:8-16):

| observed AgentDescription.device_type | action |
| --- | --- |
| first DeviceType::Cpu | retained in cpu |
| first DeviceType::Gpu | retained in gpu |
| later CPU or GPU | ignored because a slot is already set |
| DSP, AIE, or any other type | ignored |

Missing slots are converted to local errors before any session or allocation:
no ROCr CPU agent (:17) and no ROCr GPU agent (:18). These are not
recipe_hsa::Error values, so they have no HSA status code.

gpu.into_session() consumes only the selected GPU record
(hsa/examples/execute_smoke.rs:19). DiscoveredAgent::into_session
(hsa/src/session.rs:145-199) requires an active runtime, GPU device type, the
kernel-dispatch feature bit, at least one ISA, and an exact AMD target on every
discovered GPU ISA. It queries the system timestamp frequency again, creates
one SharedFault, and initializes a session-scoped SignalPool with the runtime
API, fault state, and timestamp frequency. A rejected agent returns
UnsupportedAgent; a failed timestamp query returns Error::Hsa.

The retained cpu value remains a DiscoveredAgent for CPU-owned allocations and
access grants. The session borrows the selected GPU record's runtime and owns
the GPU raw agent and its discovered pool handles. Session::description() later
exposes the immutable queue and ISA capabilities used by the optional branch.

## Base path allocations and access grants

The byte vector and three allocations are created at lines 21-24:

| Rust binding | owner | public call | selected pool predicate | size |
| --- | --- | --- | --- | --- |
| bytes | host Vec<u8> | (0..=255).collect() | none | 256 bytes |
| source | CPU agent | cpu.allocate_fine(256) | global pool with FINE_GRAINED or EXTENDED_SCOPE_FINE_GRAINED | 256 bytes |
| device | GPU session | session.allocate_coarse(256) | global pool with COARSE_GRAINED | 256 bytes |
| destination | CPU agent | cpu.allocate_fine(256) | same fine-grained predicate | 256 bytes |

allocate_fine, allocate_coarse, and allocate_kernarg first choose the first
matching discovered global pool, then call allocate_from
(hsa/src/execution.rs:623-655). The selected pool must permit runtime
allocation, the size must be nonzero, and any discovered maximum aggregate
allocation must contain the request. hsa_amd_memory_pool_allocate returns a
non-null pointer or an error. Each Allocation records its pool index, owner,
global flags, size, and an access set initially containing only the owner.

The three grants at lines 26-28 call the exact one-agent form of
hsa_amd_agents_allow_access:

```text
session.grant_access(&source)       -> source owner CPU + GPU session agent
cpu.grant_access(&device)            -> device owner GPU + CPU agent
session.grant_access(&destination)  -> destination owner CPU + GPU session agent
```

ROCr treats a grant as replacement of the direct-access set, while always
retaining the allocation's pool owner. The Rust allocation records the owner
plus the supplied agent after a successful call. This produces the mutual
access relation required by Session::copy_async: for either copy endpoint, the
destination owner must access the source and the source owner must access the
destination. A missing fine/coarse pool returns NoMatchingMemoryPool; an
allocation failure, null pointer, or HSA access-grant status stops the path.

## Host seed and base asynchronous copies

### Host seed

The first unsafe block (hsa/examples/execute_smoke.rs:30-34) writes all 256
bytes into the CPU fine-grained source allocation with
Allocation::copy_from_host(0, &bytes). Its safety contract is satisfied here:
the allocation came from a host-accessible fine-grained pool and no device
operation references it yet. The method still checks the range and can return
CopyOutOfBounds; the known zero offset and 256-byte allocation make this
invocation valid.

### Upload

Session::copy_async(&device, 0, &source, 0, bytes.len())
(hsa/examples/execute_smoke.rs:36) performs these checks in
hsa/src/execution.rs:1577-1650:

1. The runtime is active and the session fault state is healthy.
2. The size is nonzero and both ranges fit their 256-byte allocations.
3. The CPU and GPU owners have mutual direct access through the grants above.
4. The session signal pool acquires or creates a completion signal initialized
   to one.
5. A keepalive retains both allocation Rc values until terminal completion.
6. hsa_amd_memory_async_copy is submitted with destination owner GPU, source
   owner CPU, no input dependency list, and the completion signal.

The returned Pending owns the signal and keepalive. If submission fails, the
unsubmitted signal is released and the allocation keepalive is dropped. If a
queue callback poisons the session immediately after a successful submission,
the operation is moved to deferred retirement and SessionPoisoned is returned.

upload.wait(Duration::from_secs(5)) (:37) polls the signal with acquire
semantics, then waits in active-wait chunks of at most one millisecond. Signal
value 0 is complete, a positive value remains pending, and a negative value
returns Error::AsyncSignal { value }. A callback fault returns
SessionPoisoned. A still-positive signal at the five-second boundary returns
WaitOutcome::TimedOut; the example turns that outcome into io::ErrorKind::TimedOut
with the exact message upload timed out (:38). The drop(upload) at :40 is
therefore a terminal release on success, or a deferred-retirement transfer if
an earlier error or timeout caused unwinding.

### Download

The second copy (hsa/examples/execute_smoke.rs:42-46) reverses the endpoints:
destination is CPU fine destination, source is GPU coarse device, both at offset
zero and size 256. The same validation, signal, keepalive, submission, and wait
mechanics apply. A non-complete five-second wait becomes the exact error
download timed out. drop(download) releases the completed token.

### Independent host observation

After both waits, the example allocates observed = vec![0_u8; 256] and reads
the CPU fine destination with the second unsafe block (:48-52). The safety
precondition is the completed system-scope download. Allocation::copy_to_host
checks the range before copying. The only correctness assertion is the
independently derived end state observed == bytes (:54), not a signal value,
status, or internal event. A mismatch returns the exact I/O error
round-trip copy mismatch and prevents the optional branch and cleanup calls
that follow the ? boundary.

## Optional HSACO loader and kernel resolution

When RECIPE_HSA_SMOKE_COPY_HSACO is present, the branch starts after the base
byte comparison (hsa/examples/execute_smoke.rs:58-126):

1. Read the symbol environment variable. Missing or non-UTF-8 values return
   RECIPE_HSA_SMOKE_SYMBOL is required.
2. Read the entire HSACO file into Vec<u8>. File open, read, permission, and
   path errors propagate as std::io::Error.
3. Call Session::load_hsaco(&hsaco).

Session::load_hsaco (hsa/src/execution.rs:974-1059) requires an active, healthy
session and a nonempty byte slice. It retains the bytes in Arc<[u8]> while it
performs this exact ROCr sequence:

```text
hsa_code_object_reader_create_from_memory(hsaco pointer, length)
hsa_executable_create_alt(session profile, nearest rounding, null options)
hsa_executable_load_agent_code_object(executable, GPU agent, reader)
hsa_executable_freeze(executable, null options)
```

Any failed step destroys the already-created reader or executable in dependency
order before returning its Error::Hsa. An empty file returns EmptyCodeObject.
After freeze, session health is checked again and the returned Executable owns
the executable, reader, backing bytes, and GPU agent identity.

Executable::kernel(&symbol) (:63) converts the UTF-8 symbol to a NUL-terminated
CString, so an interior NUL returns NameContainsNul. It calls
hsa_executable_get_symbol_by_name, queries symbol type and kernel object, and
rejects a non-kernel symbol with SymbolNotKernel or a zero object with
InvalidKernel. It then reads:

```toml
[kernel_metadata]
kernarg_segment_size = "u32"
kernarg_segment_alignment = "u32, nonzero power of two"
group_segment_size = "u32"
private_segment_size = "u32"
dynamic_callstack = "validated ROCr boolean"
```

An invalid alignment returns InvalidKernel. The clone at :64 keeps the metadata
independent of the borrow. The example requires metadata.kernarg_segment_size
>= 16; otherwise it returns the local error
copy smoke kernel must accept destination and source pointers (:65-67).

## Optional output, kernarg, and dispatch preparation

The branch allocates a GPU coarse output of 256 bytes (:68), grants CPU access
to it (:69), allocates a CPU kernarg pool allocation sized exactly to the
kernel metadata (:70), and grants the GPU session access to the kernarg
allocation (:71). The mutable arguments vector is zero-filled to the full
metadata size (:72). The first 16 bytes are populated in native endian order:

```text
arguments[0..8]   = output.as_ptr() as u64
arguments[8..16]  = device.as_ptr() as u64
arguments[16..]   = zero bytes, if the metadata is larger than 16
```

The kernarg host write (:77-80) is safe because the allocation came from the
CPU kernarg-capable pool and no queue packet has been published yet. The range
is checked by copy_from_host; allocation, access, or host-write errors stop the
branch.

The queue size is read from the selected GPU's immutable description
(hsa/examples/execute_smoke.rs:82-86). A missing queue capability is a local
I/O error, GPU has no queue capability. The example requests
QueueConfig::new(minimum_queue, QueueKind::SingleProducer) and calls
session.create_queue (:87). The constructor sets private and group segment
sizes to u32::MAX, asking ROCr for its normal limits.

Session::create_queue (hsa/src/session.rs:230-335) rechecks runtime health,
requires a discovered queue capability, accepts only a power-of-two size in the
discovered range, and allows SingleProducer when the advertised kind is
SingleProducer or MultiProducer. An advertised Cooperative queue does not
accept this request. ROCr queue creation installs a callback that permanently
poisons the session on asynchronous status, then validates the returned queue
pointer, ring base, power-of-two size, kernel-dispatch feature, and kind. The
failure variants are UnsupportedAgent, InvalidQueueSize, UnsupportedQueueKind,
Error::Hsa { operation = "hsa_queue_create" }, NullQueue, and
InvalidQueueReturned.

## Optional dependency copy and AQL dispatch

### Prerequisite copy

Before the kernel dispatch, the branch submits a second
Session::copy_async(&device, 0, &source, 0, 256) (:88). This repeats the
CPU-to-GPU copy so that the dispatch has a live completion signal to depend on;
the source already contains the expected bytes from the base path. The Pending
is retained as prerequisite, and prerequisite.dependency() clones its
session-scoped signal record (:89). The clone does not consume or recycle the
signal, and it keeps the prerequisite signal alive until every dependent
consumer has released its reference.

### Queue::dispatch_after

The call at lines 90-95 supplies the resolved kernel, the kernarg allocation,
DispatchGeometry::one_dimensional(64, 64), and a one-element dependency slice.
The geometry constructor sets:

```toml
dimensions = 1
grid = [64, 1, 1]
workgroup = [64, 1, 1]
dynamic_group_bytes = 0
dynamic_private_bytes = 0
barrier = false
```

Queue::dispatch_after (hsa/src/execution.rs:2117-2304) validates runtime health,
single-producer publication, executable and queue agent identity, geometry
against every exact ISA limit, dependency session identity, and the current
dependency signal. It requires a kernarg allocation when metadata says the
kernel has a nonzero kernarg segment, requires the discovered
KERNARG_INITIALIZATION pool flag, checks GPU access, allocation size, and
metadata alignment, and rejects a dynamic-callstack kernel without an explicit
private-byte allowance. It checks segment-size arithmetic for overflow.

For one dependency, barrier_packet_count(1) is one, so the queue needs exactly
two packets. dispatch_after acquires one completion signal for the kernel and
one signal for the barrier, lowers the dependency into one five-input-capacity
barrier-AND packet, and appends one kernel packet. The keepalive retains the
queue core, executable, kernarg allocation, original dependency signal, and
barrier signal. Every packet is written with an invalid header first, then its
valid system-acquire/system-release header is published, the write index is
released, and the doorbell is rung. A full ring returns QueueFull without
advancing the producer index. Signal allocation, geometry, dependency, and
status failures leave all unsubmitted signals releasable.

The returned dispatch token is the kernel completion signal. The device-side
barrier prevents the kernel packet from becoming executable until the
prerequisite signal reaches zero; the host does not wait for the prerequisite
before publishing the dispatch.

### Dispatch wait, prerequisite wait, and queue close

dispatch.wait(Duration::from_secs(5)) (:97-99) must return WaitOutcome::Complete.
A timeout becomes the exact I/O error dispatch timed out; a negative completion
signal or session poison propagates its typed HSA error. drop(dispatch) (:100)
releases the dispatch keepalive after terminal completion, including the queue
and executable references.

The example then waits separately on prerequisite (:101-103). This is ordered
after dispatch only for host teardown. Device execution already enforced the
dependency. A timeout becomes dispatch prerequisite timed out; dropping the
token (:104) releases its allocation references. Only after both tokens are
terminal does queue.close() (:105) have a unique QueueCore and call
hsa_queue_destroy. A live pending token would make Queue::close return
ResourceBusy { resource = "HSA queue" }.

## Optional verification and independent result

The verification copy (hsa/examples/execute_smoke.rs:107-111) submits
output -> destination with the same zero offsets and 256-byte size. CPU access
to output was granted before dispatch, GPU access to destination was granted in
the base path, and the queue and dispatch tokens have been dropped, so the copy
endpoint access checks and resource ownership are valid. Its five-second timeout
message is verification copy timed out.

The destination is read again with copy_to_host under the completed system-scope
copy precondition (:112-116). The comparison at :117-119 is the optional
branch's only correctness assertion. A mismatch returns the exact I/O error
kernel copy mismatch. A successful comparison drops kernel, then explicitly
closes executable, kernarg, and output (:121-125) and prints:

```text
optional HSACO kernel dispatch smoke passed
```

Dropping kernel first is required because Kernel retains an Rc of the
executable. Executable::close then destroys the frozen executable before the
code-object reader and backing HSACO bytes. Allocation::close requires the
allocation's Rc to be unique; the prior token drops establish that condition.

## Explicit teardown and deferred-retirement behavior

After the optional branch, or immediately after the base comparison when the
branch is absent, lines 128-134 close resources in dependency order:

```text
destination.close()
device.close()
source.close()
drop(session)
drop(cpu)
runtime.close()
println!("ROCr fine-to-coarse-to-fine asynchronous copy smoke passed")
```

Allocation::close calls hsa_amd_memory_pool_free only when its internal Rc is
unique. A live pending token or other clone returns ResourceBusy and does not
guess whether the device has stopped. drop(session) releases the session's
SignalPool. Completed signal records are either returned to the pool or
destroyed. A positive signal outside the explicit retirement set emits the
terminal leak diagnostic rather than destroying a device-visible signal.
Unresolved retired operations are retained and reported by
SignalPoolInner::drop; the diagnostic asks callers to use
Session::drain_retirements before dropping a session, but this example does not
call that API because all normal-path waits have completed.

drop(cpu) only releases the discovery record. runtime.close() consumes the
active runtime, sets it inactive before calling hsa_shut_down, and reports a
shutdown status error if ROCr rejects the call. If an earlier ? returns, Rust
unwinds the same ownership graph automatically: incomplete Pending values move
their signals and allocation/executable/queue keepalives into deferred
retirement, then the runtime's Drop performs a best-effort hsa_shut_down. That
early-error path does not print either success line.

Asynchronous queue faults are recorded by the callback installed at queue
creation. The callback stores status, optional source queue id, and an epoch in
SharedFault, marks the session permanently poisoned, and wakes waiters. A later
copy_async, dispatch, or pending poll returns SessionPoisoned with that fault
data. Poisoning has no recovery, retry, alternate queue, or second submission
path.

## Failure boundary matrix

The following table lists the errors reachable from this source path. Errors
from a ? return immediately through Box<dyn Error>; local I/O errors retain
their exact message.

| source step | direct failure | observable result |
| --- | --- | --- |
| Runtime::open_default | both sonames unavailable | final LibraryOpen from libhsa-runtime64.so |
| dynamic symbol resolution | required ROCr symbol absent | MissingSymbol with symbol and dlerror detail |
| hsa_init | non-success status | Hsa { operation = "hsa_init", message = None } |
| system or agent discovery | status, malformed enum/string/identity, callback panic, allocation reserve | typed Hsa, InvalidAttribute, InvalidUtf8, InvalidIdentity, CallbackPanicked, or AllocationFailed |
| agent selection | no CPU or no GPU | local no ROCr CPU agent or no ROCr GPU agent |
| into_session | non-GPU, no kernel dispatch, incomplete AMD ISA identity | UnsupportedAgent |
| pool selection | no matching fine/coarse/kernarg pool | NoMatchingMemoryPool |
| allocation | inactive runtime, invalid size/pool, ROCr allocation status, null pointer | RuntimeClosed, InvalidAllocationSize, InvalidMemoryPoolIndex, MemoryPoolNotAllocatable, Hsa, or NullAllocation |
| access grant | ROCr access status | Hsa { operation = "hsa_amd_agents_allow_access" } |
| host copy | range outside allocation | CopyOutOfBounds |
| async-copy preflight | inactive/poisoned session, zero size, out of range, missing mutual access | RuntimeClosed, SessionPoisoned, or InvalidDispatch |
| async-copy submit | signal acquisition or ROCr async-copy status | signal-pool error or Hsa { operation = "hsa_amd_memory_async_copy" } |
| copy wait | negative signal, queue poison, positive signal at five seconds | AsyncSignal, SessionPoisoned, or local TimedOut message |
| byte observation | bytes differ | round-trip copy mismatch |
| HSACO environment | symbol absent or non-UTF-8 | RECIPE_HSA_SMOKE_SYMBOL is required |
| HSACO read | path, permissions, short read, or other filesystem error | std::io::Error from std::fs::read |
| HSACO realization | empty code object, reader/executable/load/freeze status | EmptyCodeObject or typed Hsa |
| symbol lookup | interior NUL, non-kernel symbol, zero object, bad metadata | NameContainsNul, SymbolNotKernel, or InvalidKernel |
| optional metadata guard | kernarg segment smaller than two pointers | copy smoke kernel must accept destination and source pointers |
| optional queue selection | no queue capability | GPU has no queue capability |
| queue creation | unsupported kind/size, null or malformed queue, ROCr status | UnsupportedAgent, InvalidQueueSize, UnsupportedQueueKind, NullQueue, InvalidQueueReturned, or Hsa |
| dispatch preflight | wrong session/agent, geometry/ISA limit, dependency state, kernarg contract, segment overflow | InvalidDispatch, AsyncSignal, or SessionPoisoned |
| dispatch enqueue | required packets exceed ring or ring is occupied | InvalidProgressRequest or QueueFull |
| dispatch/prerequisite/verify waits | negative signal, poison, five-second positive signal | typed HSA error or the exact local timeout message for that operation |
| queue close | pending token still retains queue | ResourceBusy { resource = "HSA queue" } |
| optional byte observation | kernel output differs | kernel copy mismatch |
| executable/allocation close | live Rc clone or ROCr destroy/free status | ResourceBusy or typed Hsa |
| runtime shutdown | ROCr rejects hsa_shut_down | Hsa { operation = "hsa_shut_down" }; runtime is already inactive |

The source has no catch-all conversion that turns a failed transition into a
success. The only expected non-error completion values are the two explicit
WaitOutcome::Complete checks and the two independent byte comparisons.

## Validation evidence

The exact default production entrypoint was run from hsa/ on 2026-08-02:

```text
cargo run --features live-hsa --example execute_smoke
```

It compiled recipe-hsa with live-hsa, opened ROCr, discovered at least one CPU
and GPU, completed both asynchronous copies, read back all 256 bytes, and
printed:

```text
ROCr fine→coarse→fine asynchronous copy smoke passed
```

No RECIPE_HSA_SMOKE_COPY_HSACO artifact was available in the repository or
/tmp during this trace, so the optional reader/symbol/dispatch branch was
characterized from its real production code but not claimed as a live HSACO run.
A future optional run must provide a two-pointer copy kernel and should verify
the optional HSACO kernel dispatch smoke passed line plus the final base-path
line.

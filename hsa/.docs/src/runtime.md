# HSA runtime boundary

[`hsa/src/runtime.rs`](../../src/runtime.rs) is the public owner of one
balanced ROCr initialization reference. It loads the reviewed function table
from [`hsa/src/loader.rs`](../../src/loader.rs), calls `hsa_init`, exposes a
borrowed discovery entry point, and calls `hsa_shut_down` exactly once for its
own successful initialization. The type is deliberately scoped. It is not a
process-global runtime, a singleton registry, a scheduler, or a second HSA
execution policy.

The public facade re-exports [`Runtime`](../../src/lib.rs#L34-L34) from
`recipe_hsa`. The rest of this document describes the concrete state, call
order, lifetime boundary, loader surface, users, and failure behavior in the
current implementation.

## Intent and contract

The module has one narrow intent:

```text
input                 explicit shared-library path or one of two default sonames
load                  dlopen, resolve the complete reviewed ROCr ABI, retain Library in Api
initialize            call hsa_init through Api::init
live state            Runtime { api: Arc<Api>, active: true }
use                   discover topology and construct scoped HSA resources
shutdown              consume Runtime, mark inactive, call hsa_shut_down once
fallback              Drop calls hsa_shut_down when explicit close was not reached
global state          none in this crate; every Runtime owns one balanced reference
```

The two fields are intentionally small:

| Field | Visibility | Meaning | State rule |
| --- | --- | --- | --- |
| `api: Arc<Api>` | `pub(crate)` | The resolved function pointers and the `Library` that keeps the defining shared object loaded. | Constructed only after `Api::load` succeeds. `Arc` clones are put in scoped child resources so function pointers remain valid during teardown. |
| `active: bool` | private | Whether this `Runtime` still owns its successful `hsa_init` reference. | Set to `true` only after `hsa_init` returns `STATUS_SUCCESS`; set to `false` before either explicit or fallback shutdown. |

`active` is per object, not a process-wide counter. There is no `static`
runtime, `Once`, hidden cache, mutable global, or cross-object active registry
in `runtime.rs`. ROCr may maintain its own internal process state, but this
crate models each `Runtime` value as one independently balanced initialization
reference.

## Lifecycle state machine

The successful and failed transitions are deliberately one-way:

```text
Runtime::open_default() or Runtime::open(path)
        |
        +-- Api::load and hsa_init fail -> Err; no Runtime is returned
        |
        +-- hsa_init == STATUS_SUCCESS -> Runtime(active = true)
                                              |
                                  discover and scoped child operations
                                              |
                         Runtime::close(self)
                                              |
                         active = false before FFI
                                              |
                        hsa_shut_down result
                         /                    \
                      Ok(())                Err(Hsa)
                         |                    |
                    self is dropped        self is dropped
                    without another call   without another call

Runtime(active = true) --Drop--> active = false, hsa_shut_down status ignored
Runtime(active = false) --Drop--> no HSA call
```

The important invariants are:

1. A `Runtime` value is returned only after a successful `hsa_init`.
2. `close` consumes the value, writes `active = false` before invoking the
   shutdown pointer, and routes the returned status through `Api::check`.
3. A shutdown error is not retried. The value is already inactive, so its
   destructor cannot call `hsa_shut_down` a second time.
4. `Drop` is a best-effort fallback. It cannot report a shutdown status, so the
   explicit `close` path is the observable teardown path.
5. Rust borrows prevent consuming or dropping the `Runtime` while
   `Discovery`, `DiscoveredAgent`, `Session`, or any execution object carrying
   the `'runtime` marker is live.

There is no public method that reactivates a closed object. A caller that needs
another HSA lifetime constructs another `Runtime` and resolves the library
again.

## Initialization and dynamic loading

### Default candidate policy

`Runtime::open_default` at
[`runtime.rs#L21-L32`](../../src/runtime.rs#L21-L32) tries exactly these
sonames, in order:

1. `libhsa-runtime64.so.1`
2. `libhsa-runtime64.so`

The method stores the most recent `Error::LibraryOpen`. It returns the first
successful `Runtime`. It tries the second candidate only when the first
`Runtime::open` returned `Error::LibraryOpen`, which means `dlopen` could not
produce a handle. A missing symbol, an `hsa_init` failure, or any other typed
error stops the candidate loop immediately. If neither candidate opens, the
last `LibraryOpen` is returned. The final `expect` is justified by the fixed,
nonempty candidate array and is not a recovery branch.

This is a loader-path fallback only. It does not retry `hsa_init`, substitute a
different ABI, or synthesize discovery data.

### Explicit path and `Api::load`

`Runtime::open(path)` at
[`runtime.rs#L34-L50`](../../src/runtime.rs#L34-L50) performs this exact
sequence:

```text
Runtime::open(path)
  -> Api::load(path)
       -> Library::open(path)
       -> Library::symbol(...) once for every required symbol
       -> Api { _library, function pointers }
  -> Arc<Api>
  -> unsafe { (api.init)() }
  -> Runtime { api, active: true }
```

`Library::open` in [`loader.rs#L27-L56`](../../src/loader.rs#L27-L56)
retains a display copy of the path, converts the Unix `OsStr` bytes to a
NUL-terminated `CString`, clears the thread-local `dlerror`, and calls
`dlopen(path, RTLD_NOW | RTLD_LOCAL)`. An interior NUL returns
`Error::PathContainsNul` before `dlopen`. A null handle returns
`Error::LibraryOpen { path, detail }`; `detail` is copied from `dlerror`, or
uses the literal `dynamic loader returned a null handle` when the loader did
not provide text.

`Library::symbol` in [`loader.rs#L58-L88`](../../src/loader.rs#L58-L88)
clears `dlerror`, calls `dlsym` with a static NUL-terminated symbol name, and
immediately consumes the loader diagnostic. A diagnostic becomes
`Error::MissingSymbol` even if the returned address is non-null. With no
diagnostic, a null address is also `MissingSymbol`. Successful addresses are
transmuted by the local `symbol!` macro to the reviewed ABI aliases from
[`abi.rs`](../../src/abi.rs). The `Api` stores the resulting function pointers
and the owning `Library` in one value.

The complete required ABI is resolved before `hsa_init`, including functions
that a discovery-only caller will not use:

| Area | Required symbols |
| --- | --- |
| Lifecycle and status | `hsa_init`, `hsa_shut_down`, `hsa_status_string` |
| System, agents, and ISAs | `hsa_system_get_info`, `hsa_iterate_agents`, `hsa_agent_get_info`, `hsa_agent_iterate_isas`, `hsa_isa_get_info_alt` |
| Queues | `hsa_queue_create`, `hsa_queue_destroy`, `hsa_queue_load_read_index_scacquire`, `hsa_queue_load_write_index_relaxed`, `hsa_queue_store_write_index_screlease` |
| Memory-pool discovery and allocation | `hsa_amd_agent_iterate_memory_pools`, `hsa_amd_memory_pool_get_info`, `hsa_amd_memory_pool_allocate`, `hsa_amd_memory_pool_free`, `hsa_amd_agents_allow_access` |
| Code objects and executables | `hsa_code_object_reader_create_from_memory`, `hsa_code_object_reader_destroy`, `hsa_executable_create_alt`, `hsa_executable_destroy`, `hsa_executable_load_agent_code_object`, `hsa_executable_freeze`, `hsa_executable_get_symbol_by_name`, `hsa_executable_symbol_get_info` |
| Copies and completion signals | `hsa_amd_memory_async_copy`, `hsa_signal_create`, `hsa_signal_destroy`, `hsa_signal_load_scacquire`, `hsa_signal_store_screlease`, `hsa_signal_wait_scacquire` |

The table has 32 symbols. Resolving all 32 up front makes the `Api` a complete
reviewed boundary. There is no lazy symbol lookup after `Runtime::open`
returns.

If `Api::load` fails while resolving a symbol, its local `Library` drops and
calls the one matching `dlclose`; no partially populated `Api` escapes. If
`hsa_init` returns a non-success status, `Runtime::open` returns
`Error::Hsa { operation: "hsa_init", status, message: None }`. It intentionally
does not call `hsa_status_string` before successful initialization. The
`Arc<Api>` then drops, which releases the loader handle.

### Loader lifetime and status conversion

`Library` has an explicit `Drop` implementation that calls `dlclose` exactly
once for its successful `dlopen`. It is marked `Send` and `Sync` because POSIX
permits concurrent use of the handle and the post-construction symbol state is
immutable. `Api` itself is shared with `Arc` and is never rebuilt or mutated
after `Runtime::open`.

`Api::check` at
[`loader.rs#L272-L281`](../../src/loader.rs#L272-L281) maps
`STATUS_SUCCESS` to `Ok(())`. Any other status becomes `Error::Hsa` and calls
`hsa_status_string` to obtain optional runtime-owned text. If the status-string
query fails or returns a null pointer, the error remains valid with
`message: None`. `check_status_only` at
[`loader.rs#L283-L297`](../../src/loader.rs#L283-L297) intentionally omits
that allocation and text lookup for post-realization live-loop submissions;
those errors retain the numeric status with `message: None`.

## Public methods and their exact effects

| Method | Preconditions | HSA work | Result and state effect |
| --- | --- | --- | --- |
| `open_default()` | None beyond the process being able to call the loader. | Tries the two default sonames, resolves the complete `Api`, and calls `hsa_init`. | `Ok(Runtime(active = true))`, or the first non-loader error, or the last `LibraryOpen`. |
| `open(path)` | `path` must be representable as a C string and identify a compatible ROCr library. | Calls `dlopen`, all 32 `dlsym` lookups, then `hsa_init`. | Returns a live runtime only after initialization succeeds. |
| `discover(&self)` | `active` must be true. | Delegates to `discovery::discover(self)` with a `&Runtime` borrow. | Returns a complete `Discovery<'runtime>` snapshot, or the first required query, callback, allocation, identity, or metadata error. |
| `close(self)` | The consumed value must still have `active = true`; normal callers cannot invoke it twice. | Sets `active = false`, calls `hsa_shut_down`, then calls `Api::check`. | Reports `Ok(())` or `Error::Hsa`; no later destructor shutdown is attempted. |
| `ensure_active(&self)` | Crate-private guard used by child constructors and submissions. | No HSA call. | `Ok(())` when active, otherwise `Error::RuntimeClosed`. |
| `Drop::drop(&mut self)` | Runs for every returned `Runtime`, including early-return paths. | If active, sets it false and calls `hsa_shut_down` once. | Discards the shutdown status, then releases the `Arc<Api>` held by the runtime. |

`discover` does not cache or retain a global inventory. It creates a new
snapshot each time. The snapshot owns ordinary Rust descriptions and raw
handle values, while each `DiscoveredAgent` retains a borrow of the same
`Runtime` so the HSA initialization reference cannot end beneath it.

## Scoped ownership graph

The lifetime graph is the primary shutdown safety mechanism:

```text
Runtime
  &Runtime
    Discovery<'runtime>
      DiscoveredAgent<'runtime>
        into_session()
          Session<'runtime>
            Queue<'session, 'runtime>
            Allocation<'runtime>
            Executable<'session, 'runtime>
              Kernel<'session, 'runtime>
            Pending<'session, 'runtime>
            PreparedPending<'session, 'runtime>
            Dependency<'session, 'runtime>
```

The concrete ownership roles are:

| Object | Runtime relationship | Why the relationship matters |
| --- | --- | --- |
| `Discovery<'runtime>` | Stores `Vec<DiscoveredAgent<'runtime>>`; each agent stores `&'runtime Runtime`. | Discovery metadata cannot outlive initialization. `into_agents` moves the vector without erasing the borrow. |
| `DiscoveredAgent<'runtime>` | Stores raw agent and memory-pool handles plus `&'runtime Runtime`. | `into_session` and allocation methods can only use handles while the runtime borrow is live. |
| `Session<'runtime>` | Stores `&'runtime Runtime`, exact raw agent/pool handles, a per-session `SharedFault`, and a `SignalPool`. | Session operations first use the runtime guard and then the session fault guard. The signal pool is not global. |
| `Allocation<'runtime>` | Stores `PhantomData<&'runtime Runtime>` and an `Arc<Api>` in its inner owner. | The marker keeps the runtime borrow alive; the `Arc` keeps the loader function pointers valid for allocation teardown. |
| `Queue<'session, 'runtime>` | Borrows a `Session`; its `QueueCore` owns an `Arc<Api>` and the raw queue. | Queue callbacks and packet publication stay within the session and runtime scope. |
| `Executable<'session, 'runtime>` and `Kernel` | Executable borrows a session; kernel carries an `Rc` to executable internals. | A kernel cannot outlive the executable or its session. The executable keeps its reader backing and `Arc<Api>` until destruction. |
| `Pending`, `PreparedPending`, and `Dependency` | Carry session and runtime phantom lifetimes; keep signals and referenced queues, executables, allocations, and dependencies alive with `Rc`. | Device-visible work cannot release a resource while the runtime scope is still in use. Deferred retirement retains unresolved work instead of guessing that shutdown occurred. |

The `Arc<Api>` copies do not create a second runtime or a second
initialization reference. They only extend the dynamic-library and function
pointer storage lifetime until each child teardown path has finished. The
borrowed `Runtime` remains the authority for whether new operations are
allowed.

## Discovery as the first runtime user

`Runtime::discover` calls `ensure_active` and then
[`discovery::discover`](../../src/discovery.rs#L154-L189). That engine reads
the current ROCr system and agent topology, not a cached or synthetic
description:

1. It queries HSA version, AMD extension version, and timestamp frequency with
   `hsa_system_get_info`.
2. It calls `hsa_iterate_agents` and reserves the exact result count.
3. It expands each agent through `hsa_agent_get_info`, ISA enumeration and
   `hsa_isa_get_info_alt`, then AMD memory-pool enumeration and
   `hsa_amd_memory_pool_get_info`.
4. It preserves exact names, UUIDs, PCI identities, queue limits, ISA limits,
   memory-pool flags, and optional extension attributes in owned descriptions.
5. It returns one complete `Discovery` only after every agent has been
   expanded. A failure aborts the result; no partial `Discovery` is exposed.

Every required query routes status through `Api::check`, so a ROCr status is
retained as `Error::Hsa` with its operation annotation and optional status
string. Typed conversion failures remain distinct from driver statuses. For
example, invalid device or profile values become `InvalidAttribute`, malformed
UUID or ISA identities become `InvalidIdentity`, invalid UTF-8 becomes
`InvalidUtf8`, and inconsistent queue bounds become `InvalidQueueSize`.

`Discovery::into_agents` is the hand-off from inspection to use. A caller may
select a CPU agent for host allocations and consume a GPU agent with
`DiscoveredAgent::into_session`. The selected record still carries the
original runtime borrow; consuming the vector does not detach it.

## Runtime guard coverage and users

The runtime guard is applied at the operation boundaries that can create or
submit new HSA work. The following table is an inventory of the current
callers of `ensure_active`:

| User entry point | Guard location | Subsequent runtime work | Important additional checks |
| --- | --- | --- | --- |
| `Runtime::discover` | `runtime.rs#L53-L56` | Full discovery engine and all system, agent, ISA, and pool queries. | No partial snapshot is returned. |
| `DiscoveredAgent::into_session` | `session.rs#L145-L199` | Timestamp query and construction of the session's signal pool. | Agent must be GPU, support kernel dispatch, and expose exact AMD ISA identities. |
| `DiscoveredAgent::allocate*` and `Session::allocate*` | `execution.rs#L525-L577` through `allocate_from` | `hsa_amd_memory_pool_allocate`; wraps the pointer in `AllocationInner`. | Pool index, runtime-allocation capability, nonzero size, and maximum size are checked first. |
| `DiscoveredAgent::grant_access` and `Session::grant_access*` | `execution.rs#L657-L739` | `hsa_amd_agents_allow_access`; records the exact direct-access set. | Exact-set calls reject objects from different runtime identities. |
| `Session::available_memory_bytes` | `session.rs#L205-L223` | `hsa_agent_get_info(AMD_MEMORY_AVAIL)`. | Session fault state must be healthy. |
| `Session::create_queue` | `session.rs#L230-L335` | `hsa_queue_create`; validates the returned queue layout and callback state. | Session fault, discovered queue bounds, producer kind, power-of-two size, and kernel-dispatch feature are checked. |
| `Session::load_hsaco` | `execution.rs#L974-L1059` | Code-object reader creation, executable creation, agent load, and freeze. | Empty code objects, symbol and ABI errors, and post-freeze session poison fail the operation; partial handles are destroyed on each failure. |
| `Session::prepare_pending` | `execution.rs#L1460-L1499` | Acquires a signal and reserves fixed keepalive vectors before the live loop. | Allocation and dependency capacities must reserve successfully. |
| `Session::copy_async` and `copy_async_prepared` | `execution.rs#L1500-L1650` | `hsa_amd_memory_async_copy` with a completion signal. | Runtime and session health, nonzero size, bounds, access grants, and prepared-token ownership are checked. |
| `Queue::dispatch`, `dispatch_after`, and `dispatch_prepared` | `execution.rs#L1962-L2088` and `#L2117-L2280` | Writes AQL packet bodies, publishes headers and indices, rings the doorbell, and retains keepalive resources. | Runtime and session health, queue kind, agent identity, geometry, kernarg ABI, dependency session, and queue capacity are checked. |
| `Queue::progress_capacity` | `execution.rs#L2100-L2111` | Reads queue indices without publishing a packet. | Required packet count and bounded probe budget are validated. |

Several public methods deliberately do not call `ensure_active` themselves:
`Session::description`, `fault`, `ensure_healthy`, `poll_retirements`, and
`drain_retirements`; queue metadata and `Queue::close`; allocation pointer,
host-copy, and `Allocation::close`; executable symbol lookup and
`Executable::close`; and `Pending`, `PreparedPending`, and `Dependency`
polling or reset methods. Their safety comes from the same scoped lifetime and
owned-resource invariants, not from an alternate runtime state. Some of these
methods read signals or call native destructors through their retained
`Arc<Api>`, but no public safe path can keep the runtime borrow after the
resource is dropped.

This distinction is intentional:

* Creation and submission boundaries reject an inactive runtime before issuing
  new work.
* Completion observation and ordered teardown operate on resources already
  realized in the active scope.
* A destructor never retries a failed native destroy. Explicit consuming close
  methods report status or `ResourceBusy`; `Drop` discards the status and keeps
  any resource alive when releasing it would be unsafe.

## External owners in the workspace

The HSA crate examples are direct users:

* `hsa/examples/discover.rs` opens the default runtime, scopes
  `runtime.discover()`, prints the owned descriptions, and calls explicit
  `runtime.close()`.
* `hsa/examples/execute_smoke.rs` opens and discovers once, consumes CPU and
  GPU records into allocations and a session, runs asynchronous copies and an
  optional HSACO dispatch, closes queues, executables, and allocations, then
  calls `runtime.close()`.

The production owner is `native-probe`:

1. `HsaBackend` stores `runtime: RefCell<Option<HsaRuntimeState>>`, where the
   state pairs a `PinnedLibrary` identity with one `recipe_hsa::Runtime`
   ([`native-probe/src/hsa.rs#L67-L149`](../../../native-probe/src/hsa.rs#L67-L149)).
2. `with_runtime` checks PCI accelerator presence and the configured ROCr
   library identity before borrowing the state. If no runtime is open and no
   matching hardware or library exists, the backend returns `None` or a typed
   discovery error according to that hardware state. If an already-open
   runtime's hardware or library disappears or changes, it returns a
   `ProbeError` rather than reopening a different identity.
3. The first matching call invokes `Runtime::open` with the selected explicit
   library path. Later discovery and benchmark calls reuse that same scoped
   runtime and pass `&Runtime` into their closures.
4. `with_native_execution_bindings` keeps the backend, runtime, discovered
   agents, sessions, and host allocators alive for one callback. The returned
   `NativeExecutionBindings<'cuda, 'hsa>` borrows HSA sessions and agents only
   for that preparation/execution callback; it does not leak a runtime handle
   into a later placement path.

`native-executor` consumes the resulting `HsaBinding` and `Session` borrows;
it does not construct another `Runtime`. This keeps library loading,
initialization, identity checks, and shutdown in the probe-owned scope.

## Shutdown and teardown ordering

The normal user shape is:

```rust
let runtime = recipe_hsa::Runtime::open_default()?;
{
	let discovery = runtime.discover()?;
	// Select agents, create sessions, queues, allocations, and submissions.
	// Drive or drain all pending work and close child resources here.
	drop(discovery);
}
runtime.close()?;
```

The inner scope is not cosmetic. `Discovery<'runtime>` and every child object
borrow the runtime, so the compiler requires those borrows to end before
`Runtime::close(self)` can consume it. The same rule applies when
`native-probe` invokes its callback: all `HsaBinding` values and sessions must
be destroyed before the callback returns and before the backend can release
its stored runtime state.

Child teardown is layered below the runtime:

| Resource | Explicit operation | Fallback behavior |
| --- | --- | --- |
| Allocation | `Allocation::close` requires the sole `Rc` and calls `hsa_amd_memory_pool_free`. | `AllocationInner::drop` calls the same destroy and ignores its status. |
| Queue | `Queue::close` requires no pending-token keepalive and calls `hsa_queue_destroy`. | `QueueCore::drop` destroys best effort; a failed callback-context destroy leaks the callback context rather than permit use-after-free. |
| Executable | `Executable::close` requires no live kernel `Rc`, destroys executable before reader, and retains HSACO backing if a destroy fails. | `ExecutableInner::drop` performs the same ordered best-effort cleanup. |
| Completion signals and pending work | `Pending` completion or `Session::drain_retirements` releases signal and keepalive resources only after terminal observation. | Incomplete drops enter session-owned deferred retirement. Unresolved work is retained and a terminal leak diagnostic is emitted rather than guessing that shutdown stopped device access. |
| Runtime | `Runtime::close` marks inactive, calls `hsa_shut_down`, and reports status. | `Runtime::drop` marks inactive, calls `hsa_shut_down`, and discards status. |

The runtime itself does not inspect or forcibly destroy child resources. Rust
borrows and child keepalive references enforce the order before shutdown is
reachable. `Arc<Api>` is released only after each child owner and finally the
runtime are dropped; then `Library::drop` calls `dlclose`.

If a caller returns early through `?`, Rust drops the local child values and
then runs the runtime fallback. This path still performs one shutdown, but any
shutdown status is not observable through `Drop`. Callers that need an
observable teardown result should scope children and call `close` explicitly.

## Failure boundaries

The runtime boundary preserves the first concrete failure. It does not retry a
different implementation or replace missing state with a default:

| Boundary | Typed result or behavior | Recovery policy |
| --- | --- | --- |
| Non-64-bit target | Crate-level compile error from `lib.rs` | No runtime path is built for an unsupported pointer width. |
| Explicit path contains an interior NUL | `Error::PathContainsNul { path }` | Rejected before `dlopen`. |
| `dlopen` fails | `Error::LibraryOpen { path, detail }` | `open_default` may try its second fixed soname; `open(path)` returns directly. |
| A required `dlsym` fails | `Error::MissingSymbol { library, symbol, detail }` | No alternate ABI or candidate is attempted after this error. |
| `hsa_init` fails | `Error::Hsa { operation: "hsa_init", status, message: None }` | The failed `Api` unloads; no `Runtime` is returned and initialization is not retried. |
| Runtime guard sees inactive state | `Error::RuntimeClosed` | The operation stops before issuing a new HSA call. |
| Discovery status is non-success | `Error::Hsa` with operation annotation and optional status text | Discovery aborts without returning a partial snapshot. |
| Discovery metadata is malformed | `InvalidAttribute`, `InvalidUtf8`, `InvalidIdentity`, `InvalidQueueSize`, `AllocationFailed`, or callback-specific typed errors | The exact invalid field or allocation is reported; values are not guessed. |
| Session or execution status is non-success | `Error::Hsa` from `Api::check`, or numeric-only `Error::Hsa` from `check_status_only` | The native operation stops; callers retain or retire already submitted resources according to the operation token. |
| Session queue callback reports an asynchronous fault | Per-session `SharedFault` becomes permanently poisoned; later guarded operations return `SessionPoisoned`. | No runtime-wide poison or cross-session fallback is introduced. |
| `hsa_shut_down` in explicit close | `Error::Hsa { operation: "hsa_shut_down", ... }` | Runtime is already inactive; there is no retry or second shutdown. |
| `hsa_shut_down` in `Drop` | Status is ignored. | The destructor cannot return an error; the dynamic library is released after `Api` owners are gone. |

The status-string helper itself is best effort. A failed
`hsa_status_string`, a null returned message pointer, or a status encountered
on a post-realization path leaves `Error::Hsa.message` as `None`; the numeric
status and operation remain authoritative.

## Direct proof path

The smallest live proof of this boundary is the discovery example:

```text
cargo run -p recipe-hsa --example discover
```

It exercises `open_default`, complete symbol resolution, `hsa_init`, active
guarding, full discovery, a scoped runtime borrow, explicit `hsa_shut_down`,
and the final dynamic-library release. The `execute_smoke` example exercises
the same runtime plus session, allocation, queue, executable, signal, and
copy users. Neither example adds a mock runtime or a synthetic loader. A
machine without a compatible ROCr installation produces the typed failure at
the first unavailable boundary.

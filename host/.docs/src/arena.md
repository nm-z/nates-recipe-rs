 # `recipe-host` arenas

 `host/src/arena.rs` is the host backend's byte-storage primitive. It owns one
 preallocated backing range for one Recipe `DeviceId`, exposes byte offsets
 rather than addresses, and provides exact-range reads and writes for the host
 backend and the heterogeneous transfer bridge. It does not choose a layout,
 reserve capacity, schedule work, or perform payload calculation.

 ## Scope and ownership

 The public surface is re-exported by `host/src/lib.rs`:

 | Item | Meaning |
 | --- | --- |
 | `ArenaKind::{Ram, Disk}` | The backing representation. |
 | `DiskFileSpec` | A caller-selected path for one disk arena file. |
 | `Arena` | A cloneable handle containing the owning `DeviceId`, kind, and an `Arc<Backing>`. |

 `Backing` is `pub(crate)`. Its two forms are:

 - `Ram { storage: Mutex<Box<[u8]>>, bytes: u64 }`, an owned host byte slice;
 - `Disk { file: File, path: PathBuf, bytes: u64 }`, an open file and the path
   retained for cleanup when the backing is released.

 `Arena::clone` copies the device and kind metadata and increments the backing
 `Arc`; it does not allocate a second range. Runtime jobs and bridge jobs also
   clone this `Arc`, so the backing remains alive while asynchronous work still
   refers to it. `Backing::len` returns the stored byte count. `Debug` reports
   the device, kind, and byte count, but never exposes storage addresses or file
   handles.

 ## `DiskFileSpec`

 `DiskFileSpec::new(path)` converts its argument to a `PathBuf` and accepts it
 only when `path.file_name()` is present. It does not touch the filesystem or
 validate the parent directory. `path()` returns the exact stored path. Disk
 bindings are normally created in preparation with a deterministic, run-scoped
 filename; the later arena constructor performs the first filesystem I/O.

 ## Allocation contract

 Both constructors require a nonzero `ByteCount`. A zero request returns
 `Error::InvalidConfiguration` and creates no backing.

 ### RAM

 `Arena::ram(device, bytes)` performs the following ordered operations:

 1. Reject `ByteCount::ZERO` with `"RAM allocation must be nonzero"`.
 2. Convert the `u64` count to `usize`. A count that cannot fit the host
    address space returns `InvalidConfiguration` with
    `"RAM arena size does not fit the host address space"`.
 3. Call `Vec::try_reserve_exact`. Allocation failure returns
    `InvalidConfiguration("RAM arena allocation failed")`.
 4. Resize the vector to the requested length with zero bytes and convert it to
    `Box<[u8]>`.

 The resulting `Arena` stores the original `u64` count, `ArenaKind::Ram`, and a
 new `Arc<Backing::Ram>`. RAM initialization is explicit and zero-filled.

 ### Disk

 `Arena::disk(device, spec, bytes)` delegates to `create_allocated_file`:

 1. Reject zero bytes with `InvalidConfiguration("disk allocation must be nonzero")`.
 2. Move the path out of `spec` and open it with read/write access,
    `create_new(true)`, and no truncation. Any open or path error is mapped to
    `Error::Io { operation: "create disk arena", kind }` by the common I/O
    mapper. `create_new` prevents an overlapping run or a second arena using
    the same path from silently overwriting an existing file.
 3. Call `fallocate` with an empty flag set, offset zero, and the requested
    `u64` length. On failure, close the file, try to remove the path, and return
    `Error::Io { operation: "allocate disk extent", kind }`.
 4. Call `File::sync_data`. On failure, close the file, try to remove the path,
    and return the mapped `Error::Io` with operation
    `"sync disk allocation"`.

 On success, the open file, exact requested count, and path are placed in
 `Backing::Disk`. The module does not write an explicit initialization pattern
   to disk; it only allocates and synchronizes the extent.

 ## Address and range semantics

 An arena address is always a `(DeviceId, u64 offset, byte count)` checked
 against the arena's stored length. There is no public base pointer or raw file
 offset API.

 `validate_backing_range(backing, offset, bytes)` accepts exactly the half-open
 interval `[offset, offset + bytes)` when `offset.checked_add(bytes)` succeeds
 and the end is at most `backing.len()`. Overflow and out-of-range ends both
 return `Error::RangeOverflow`.

 `host_range` is used for RAM slices. It converts both offset and length to
 `usize`, checks `start + length`, and returns `RangeOverflow` if either
 conversion or the addition fails. Disk I/O keeps offsets as `u64` and does not
 use this conversion.

 `write_exact` and `read_exact` first convert the source or destination slice
 length to `u64`, validate the backing range, and then dispatch by backing kind:

 - RAM locks the `Mutex<Box<[u8]>>`, converts the checked range to a Rust slice,
   and uses `copy_from_slice`. A poisoned mutex returns `Error::Poisoned`.
 - Disk uses `FileExt::write_at` or `FileExt::read_at` in a loop. Short positive
   operations advance the `u64` offset with checked addition until the complete
   slice is transferred. A zero-byte write is `Error::Io` with kind `WriteZero`;
   a zero-byte read is `Error::Io` with kind `UnexpectedEof`; operating-system
   errors use operations `"write host arena"` and `"read host arena"`.

 Direct arena reads and writes permit a zero-length slice when its offset is in
 range, because `validate_backing_range` accepts an empty interval. The
 asynchronous `PendingCopy` path adds a separate nonzero-size rule before it
 reaches an arena. Neither arena method interprets the `DeviceId`; ownership
 and endpoint-device checks are performed by the caller.

 The hidden `bridge_read_exact` and `bridge_write_exact` methods are thin
 forwarding wrappers. `native-executor/src/bridge.rs` uses them from
 `HostStageJob::Read` and `HostStageJob::Write` after the bridge has established
 the native pointer lifetime and exact byte count.

 ## Lifetime and release

 `Arena::close(self)` requires sole ownership of the backing. It calls
 `Arc::try_unwrap` and maps any remaining clone to `Error::SlotBusy`. This is a
   lifetime error, not a capacity retry: the consuming `Arena` handle is gone,
   while the remaining clones keep the backing alive until their jobs finish.

 With the sole backing owner, `close_backing` behaves as follows:

 - RAM returns `Ok(())` and the boxed storage is dropped.
 - Disk calls `sync_data`, then removes the stored path. Sync failures map to
   `Error::Io { operation: "sync disk file", kind }`; removal failures map to
   `Error::Io { operation: "remove disk file", kind }`.

 `Backing::Drop` is the best-effort fallback for disk cleanup. It calls
 `sync_data` and `remove_file` while ignoring both results. Therefore an
 explicit successful `close` reports cleanup errors, while ordinary drop still
 attempts cleanup without producing a result. `close_backing` consumes the
 backing, and the subsequent drop of that value can make a second best-effort
 cleanup attempt.

 ## Call graph and data flow

 The arena module is reached through these concrete paths:

 | Caller | Arena operation | Required invariant |
 | --- | --- | --- |
 | `HostResources::allocate_arena` in `host/src/backend.rs` | Selects `Arena::ram` for a `HostDeviceBinding::Ram` or `Arena::disk` for a `HostDeviceBinding::Disk`, using the finalized `ArenaLayout.size`. | The binding exists for `layout.device`; one finalized layout exists per device. |
 | `Backend for HostBackend::allocate_arena` | Records `PhysicalCall::AllocateArena { device, bytes }`, then delegates to the resource. | The allocation event and requested size describe the finalized layout, not an independently chosen size. |
 | `HostBackend::release_arena` and `HostResources` release paths | Check `arena.device() == device`, then call `arena.close()`. | Release ownership must match the device and all asynchronous clones must be gone. |
 | `HostResources::prepare_pending` and candidate pending preparation | Creates RAM arenas for init admission, exit egress, and four-byte metric staging. | Staging is preallocated per pending task; it is not a model arena layout. |
 | `HostResources::submit` | Writes init images into staging, copies staging to a checked device location, copies metrics from a checked host arena to staging, and reads completed egress or metric staging. | `checked_arena` verifies the endpoint device and `offset + bytes <= arena.bytes()`. |
 | `Runtime::PendingCopy` in `host/src/runtime.rs` | Retains `Arc<Backing>` in each queued `Job` and executes RAM/RAM, RAM/disk, disk/RAM, or disk/disk copies. | Source and destination ranges are checked before submission; the job keeps both backings alive through completion. |
 | `HostStageJob` in `native-executor/src/bridge.rs` | Clones a host arena and invokes the hidden exact bridge read or write for a cross-backend hop. | The bridge's endpoint contract supplies the device, offset, pointer lifetime, and byte count. |
 | `LocalArena` and `LocalBackend` in `native-executor/src/local.rs` | Wraps host arenas alongside CUDA and HSA arenas, projects only the host owner, and dispatches allocation/release by device class. | A local arena's class and `device()` must match the partition owner. |

 The normal lifecycle is:

 1. Preparation constructs `HostDeviceBinding` values and validates paths,
    devices, reservations, and capacity policy without allocating final arenas.
 2. Candidate realization preallocates runtime slots and task staging arenas.
    The production local candidate then allocates each candidate layout for its
    maximum-concurrency warm trace.
 3. Warm candidate arenas are released before capacity observation and final
    handoff. A disk path can therefore be created again for the finalized run
    only after the warm arena has been closed or dropped.
 4. `executor/src/executor.rs` allocates one arena for each finalized layout
    during initialization, before the init phase and loop. Init, loop, and exit
    transfers use only those retained arenas and pre-realized pending tokens.
 5. Teardown drains the arena map, records `ReleaseArena`, checks ownership, and
    closes each backing before destroying host resources. A release or cleanup
    error is surfaced as the run's teardown failure; there is no alternate
    backing or retry path.

 ## Layout, reservation, and capacity boundaries

 `Arena` does not own scheduler accounting. The division of responsibility is:

 - `recipe-planner` creates logical `ArenaObject` values with device, byte
   count, alignment, and lifetime. `recipe-scheduler::pack_arenas` chooses the
   lowest legal aligned offsets, reuses bytes only for non-overlapping
   lifetimes, and emits one `ArenaLayout` per device.
 - Core finalization validates that every object is allocated once on its
   device, every offset satisfies alignment and bounds, `layout.size` equals the
   maximum allocation end, live allocations do not overlap, and the layout is
   no larger than `CapacityLedgerEntry.recipe_usable`. Planner capacity checks
   also include auxiliary staging or scratch peaks.
 - `ReservationLedger` supplies one exact user headroom entry per topology
   device. In `host/src/backend.rs`, `validate_reservations` requires an entry
   for each configured host binding and rejects every mechanism except
   `ReservationMechanism::EnforcedQuota`. The arena constructor neither holds
   that quota nor subtracts reservation bytes from its requested size.
 - `HostBackendConfig::available_bytes` and `HostResources::available_bytes`
   are observation APIs, not allocation guarantees. RAM reads numeric
   `MemAvailable` from `/proc/meminfo` and multiplies by 1024. Disk queries
   `statvfs(path.parent())` and returns `f_bavail * f_frsize`, with checked
   multiplication. Missing bindings, malformed memory data, invalid parents,
   operating-system failures, and arithmetic overflow are returned as host
   errors.
 - `LocalCandidateFactory` captures initial per-device availability, verifies it
   is at least the reservation headroom, warms the candidate, releases warm
   arenas, and observes live availability. It computes measured runtime
   overhead as `initial - min(initial, live)` and Recipe-usable capacity as
   `min(initial, live) - reservation.bytes`; a live value below headroom rejects
   the candidate. These values feed the capacity ledger and are independent of
   `Backing::bytes`.

 Thus a finalized layout can be rejected during `Arena::ram` or
 `Arena::disk` construction if current host resources have changed, even though
 the measured immutable candidate fit. The allocation error remains visible to
 the executor and is not converted into a fallback representation.

 ## Failure and invariant matrix

 | Condition | Result |
 | --- | --- |
 | Zero RAM or disk size | `InvalidConfiguration` from the corresponding constructor. |
 | RAM size does not fit `usize` or RAM allocation fails | `InvalidConfiguration` with the specific allocation detail. |
 | `DiskFileSpec` has no filename | `InvalidPath`. |
 | Disk path already exists, parent is absent, or open fails | `Io { operation: "create disk arena", ... }`. |
 | Disk extent allocation or initial sync fails | Cleanup is attempted; `Io` reports `"allocate disk extent"` or `"sync disk allocation"`. |
 | Offset plus byte count overflows or exceeds stored length | `RangeOverflow` from the arena API. Host backend endpoint checks may instead return `OutOfBounds { device }`. |
 | `u64` range cannot become a RAM `usize` range | `RangeOverflow`. |
 | RAM storage mutex is poisoned | `Poisoned`. |
 | Disk read or write returns an OS error, zero progress, or unexpected EOF | `Io` with the host read/write operation and the corresponding `ErrorKind`. |
 | `close` sees outstanding `Arc` clones | `SlotBusy`; the backing remains owned by the surviving clone(s). |
 | Sole-owner disk close cannot sync or remove the path | `Io` with `"sync disk file"` or `"remove disk file"`; drop still retries best-effort. |
 | Host binding, owner, finalized task, endpoint, reservation mechanism, or capacity contract is invalid | Rejection occurs in the backend, planner, scheduler, or local executor before or around arena use; `Arena` does not add a second validation or fallback path. |

 The key invariant is that every successful operation carries an already
 validated byte range over one backing, while all policy decisions about which
 device, how many bytes, how offsets are packed, and how much capacity must be
 preserved remain outside `host/src/arena.rs`.

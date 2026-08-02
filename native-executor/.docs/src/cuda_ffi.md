# CUDA launch-parameter FFI boundary

## Scope

`native-executor/src/cuda_ffi.rs` is the narrow, crate-private adapter that turns
Recipe's validated kernel ABI into the `kernelParams` array consumed by the CUDA
Driver `cuLaunchKernel` call. It is not a CUDA loader, module owner, allocator,
queue owner, completion primitive, or ABI validator. The implementation is only
the `ParameterBlock` type and its five methods.

The source of truth for this page is:

| Contract | Source |
| --- | --- |
| `ParameterBlock` layout and unsafe slice construction | [`native-executor/src/cuda_ffi.rs`](../../src/cuda_ffi.rs#L1-L66) |
| Direct caller, ABI packing, and arena checks | [`native-executor/src/cuda.rs`](../../src/cuda.rs#L696-L737), [`native-executor/src/cuda.rs`](../../src/cuda.rs#L1906-L2040) |
| CUDA wrapper and raw Driver call | [`cuda/src/runtime.rs`](../../../cuda/src/runtime.rs#L574-L673), [`cuda/src/ffi.rs`](../../../cuda/src/ffi.rs#L10-L86) |
| Kernel ABI order and eight-byte slots | [`kernel/src/stage.rs`](../../../kernel/src/stage.rs#L334-L477), [`kernel/STAGE_LOWERING.md`](../../../kernel/STAGE_LOWERING.md#L28-L42) |
| Finalized ABI validation | [`native-executor/src/plan.rs`](../../src/plan.rs#L354-L640) |
| Native error variants and formatting | [`native-executor/src/error.rs`](../../src/error.rs#L7-L97) |

`cuda_ffi` is private in [`native-executor/src/lib.rs`](../../src/lib.rs#L17-L27).
No public caller can construct or submit a `ParameterBlock` directly.
The crate denies `unsafe_op_in_unsafe_fn`, so the implementation keeps each
unsafe operation explicit: one block reconstructs keepalive references, one
block reconstructs the parameter slice, and one block calls the reviewed
`recipe-cuda` enqueue method.

## Boundary contract

The complete boundary has these fixed properties:

| Property | Contract |
| --- | --- |
| Host slot storage | One `u64` per ABI argument, in one boxed slice. |
| `kernelParams` entries | One mutable raw pointer to the corresponding host `u64`, represented internally as `*mut u8` and reinterpreted as `*mut c_void` at the wrapper boundary. |
| Device pointer values | `DeviceBuffer::device_ptr()` returns the CUDA Driver's 64-bit `CUdeviceptr`; that integer is copied into a host slot. The entry pointer is a pointer to the integer slot, not the device address itself. |
| Scalar values | `RunId`, `LoopIteration`, and `ElementCount` are explicit 64-bit by-value slots. |
| Slot order | Readable buffers, writable buffers, optional fault flag, optional run ID, optional loop iteration, then element count. |
| Launch call | `Stream::enqueue_launch` passes the parameter array as `kernelParams` and passes a null `extra` pointer. |
| Completion | This path records no event. A calculation is complete only when the assigned stream reports idle through `Stream::poll_idle`. |
| Allocation phase | Blocks, pointers, and keepalive capacity are created while `realize_device` prepares resources, before executable work. |
| Reuse | A block is keyed by `CompletionSlotId` and reused only after the prior task on that slot is terminal. |
| Ownership | The block retains non-owning raw pointers to arena `DeviceBuffer`s. `CudaArena` and `DeviceResources` own the real allocations, streams, functions, and modules. |

The raw CUDA declaration is an `unsafe extern "C"` function with the shape
`CuLaunchKernel(function, grid_x, grid_y, grid_z, block_x, block_y, block_z,
shared_bytes, stream, kernel_params, extra)`, where both trailing arguments are
`*mut *mut c_void` ([`cuda/src/ffi.rs`](../../../cuda/src/ffi.rs#L74-L86)). The
wrapper's `enqueue_launch` supplies `kernel_params` from the mutable slice and
sets `extra` to null ([`cuda/src/runtime.rs`](../../../cuda/src/runtime.rs#L639-L672)).

`recipe-cuda` obtains this function pointer from the required
`DriverSymbol::LaunchKernel` entry. The dynamic loader resolves the C name
`cuLaunchKernel` with `dlsym`, checks that the pointer is present, and converts
the pointer to the exact `CuLaunchKernel` function-pointer type. A missing symbol
fails Driver loading before any `ParameterBlock` can be created
([`cuda/src/ffi.rs`](../../../cuda/src/ffi.rs#L91-L130),
[`cuda/src/ffi.rs`](../../../cuda/src/ffi.rs#L216-L249),
[`cuda/src/ffi.rs`](../../../cuda/src/ffi.rs#L317-L443)). The call is made through
the CUDA Driver API only; this path does not use the CUDA Runtime API.

`Stream::enqueue_launch` enters the stream's `Context`, invokes the resolved
function pointer, and pops the context before returning. Context entry or pop
failure is reported by the CUDA wrapper; a non-success launch status is retained
as the exact numeric Driver status by `check_status_only`
([`cuda/src/runtime.rs`](../../../cuda/src/runtime.rs#L21-L43),
[`cuda/src/driver.rs`](../../../cuda/src/driver.rs#L83-L109)).

CUDA copies the value addressed by each `kernelParams[i]` before the wrapper
returns. The kernel still executes asynchronously. Consequently, host slot
storage must be initialized and correctly aligned for the call, while referenced
device allocations, the function, its module, and the stream must remain live
until stream idle ([`cuda/src/runtime.rs`](../../../cuda/src/runtime.rs#L574-L588),
[`cuda/src/runtime.rs`](../../../cuda/src/runtime.rs#L632-L638)).

## Kernel ABI and packing

### ABI order

Recipe lowering constructs a stage signature in this exact order:

1. Every readable data binding, in finalized input order.
2. Every writable data binding, in finalized output order. A read-write binding
   therefore has two distinct pointer positions when it is both an input and an
   output.
3. One `FaultFlag` pointer when the stage has checked fault reporting.
4. One `RunId` value for Philox stages when present.
5. One `LoopIteration` value for Philox stages or iteration-dependent index maps
   when present.
6. One final `ElementCount` value, always present.

`StageSignature::new` emits the corresponding LLVM parameter strings and
`KernelArgument` values in that order. `StageSignature::abi` records
`argument_bytes = arguments.len() * 8` and `argument_alignment = 8`
([`kernel/src/stage.rs`](../../../kernel/src/stage.rs#L374-L432),
[`kernel/src/stage.rs`](../../../kernel/src/stage.rs#L448-L476)). The same final
element-count suffix is emitted by the elementwise lowering path
([`kernel/src/llvm.rs`](../../../kernel/src/llvm.rs#L332-L371)).

For a machine-readable index model, let:

```text
R = number of readable buffer arguments
W = number of writable buffer arguments
F = 0 or 1, depending on the fault flag
U = 0 or 1, depending on RunId
I = 0 or 1, depending on LoopIteration
N = R + W + F + U + I + 1
```

Then the validated slot ranges are:

| Slot range | `KernelArgument` |
| --- | --- |
| `0 .. R` | readable `Buffer` arguments |
| `R .. R + W` | writable `Buffer` arguments |
| `R + W .. R + W + F` | `FaultFlag` when `F = 1` |
| `R + W + F .. R + W + F + U` | `RunId` when `U = 1` |
| `R + W + F + U .. R + W + F + U + I` | `LoopIteration` when `I = 1` |
| `N - 1 .. N` | `ElementCount` |

`N` is the current `argument_count` passed to `ParameterBlock::enqueue`; a
completion-slot block may have a larger capacity when another task sharing the
slot has a longer ABI.

Every slot occupies eight bytes. Pointer slots contain a 64-bit device address;
the host pointer passed through `kernelParams` points at that eight-byte address.
The `FaultFlag` pointee is a device `i32` at a four-byte-aligned arena offset,
but its launch slot is still eight bytes. `RunId`, `LoopIteration`, and
`ElementCount` are `i64` in the generated signature and are written as `u64`
bits by `ParameterBlock`.

### Finalized validation before packing

`ExecutionPlan::validate_calculation_abi` runs before `realize_device` creates
any block. It independently verifies:

- stage identity, workgroup width, and element count;
- the exact number of buffer operands plus the optional fault, run, loop, and
  element-count suffixes;
- at most one `RunId` and at most one `LoopIteration`;
- every buffer argument's input/output position, dtype, access, backing byte
  count, and required arena alignment;
- the fault flag's device, `i32` type, four-byte size, and four-byte alignment;
- canonical suffix positions for `RunId` and `LoopIteration`; and
- `ElementCount` as the final argument.

The validation rejects an incompatible runtime artifact as `Error::ArtifactMismatch`
before the Driver module or function is used. It is the reason the packing loop
can match on `KernelArgument` without rechecking dtype or access metadata
([`native-executor/src/plan.rs`](../../src/plan.rs#L354-L640)). The FFI layer itself
does not inspect cubin metadata or infer a Rust type from a raw pointer.

## `ParameterBlock` representation

The type is:

```text
ParameterBlock {
    values:    Box<[u64]>,
    parameters: Box<[*mut u8]>,
    keepalive: Vec<*const DeviceBuffer<'static>>,
}
```

The fields are private and the type is `pub(crate)`.

### Line-level implementation map

| Lines in `cuda_ffi.rs` | Operation |
| --- | --- |
| 1-5 | Module purpose, `recipe_cuda` imports, and native `Error`/`Result` imports. |
| 7-12 | `ParameterBlock` declaration and its three storage fields. |
| 14-27 | `new`: allocate zeroed `u64` values, derive one pointer per value, and reserve keepalive capacity. |
| 29-31 | `len` and `reset_keepalive`. |
| 33-39 | Bounds-checked `set_value`. |
| 41-44 | Lifetime-erasing, non-owning `retain`. |
| 46-66 | Unsafe keepalive and parameter-slice reconstruction, Driver enqueue, and the debug length assertion. |

There are no other constructors, trait implementations, tests, or callers in
this module. All behavior that is not visible in this map belongs to the caller
or to `recipe-cuda`'s wrapper.

| Field | Meaning | Required invariant |
| --- | --- | --- |
| `values` | Fixed host-side storage for every argument value. | Its length never changes while the block is live. Each element is eight-byte aligned because it is a `u64`. |
| `parameters` | Fixed array of pointers, where entry `i` addresses `values[i]`. | The two boxed slices have equal lengths. Every pointer remains valid while `values` remains allocated. |
| `keepalive` | Non-owning pointers to every `DeviceBuffer` whose device address was packed in the current submission. | Pointers are valid arena-buffer references until the assigned stream is idle. Clearing the vector does not release a buffer. |

`values` and `parameters` are separate boxed allocations. Moving the
`ParameterBlock` moves the boxes, not the allocations, so pointers in
`parameters` continue to address the same `values` elements. Replacing or
resizing `values` would invalidate them, but no method does that. The type has no
custom `Drop`; dropping `keepalive` drops raw pointer values only and never frees
CUDA memory.

### `new(argument_count)`

`new` allocates `argument_count` zeroed `u64` slots. It then takes one mutable
pointer to each slot, casts it to `*mut u8`, and stores the pointer array in a
boxed slice. The keepalive vector is created with capacity
`argument_count.saturating_sub(1)`. The subtraction is a capacity hint, not a
semantic limit. A valid ABI always has a final `ElementCount` slot, so at most
`argument_count - 1` slots can require a device-buffer keepalive. The vector can
still grow if an invalid caller pushes more pointers.

`new` validates no ABI shape and returns no `Result`. A zero count is representable
and produces empty slices. The caller must establish a valid count before unsafe
submission.

### `len()`

`len` returns the number of value slots, not a byte count. It is used by
`fill_invocation` to ensure that a preallocated completion-slot block is large
enough for the current artifact ABI.

### `reset_keepalive()`

`reset_keepalive` calls `Vec::clear`. It removes the prior submission's raw
references while retaining the vector allocation. It does not zero `values`,
rebuild `parameters`, shrink capacity, or synchronize the stream. It is safe to
call only after the prior use of the block is terminal, which the scheduler and
completion-slot contract provide before a new calculation is packed.

### `set_value(index, value)`

`set_value` writes one `u64` slot. An out-of-range index returns
`Error::IntegerOverflow { field: "CUDA launch argument index" }`; it never writes
through an unchecked index. Valid packing obtains indices from
`abi.arguments.iter().enumerate()`, and `fill_invocation` first proves
`invocation.len() >= abi.arguments.len()`, so the error is a malformed internal
state or a bad caller input rather than a normal launch outcome.

### `retain(buffer)`

`retain` stores the address of the referenced `DeviceBuffer` after casting its
borrow to `*const DeviceBuffer<'static>`. The cast erases the Rust lifetime; it
does not extend ownership and does not make the buffer `static`. The method does
not check context identity, allocation state, duplicate references, or stream
completion. Those properties are supplied by the arena owner and checked at the
next boundary where a `DeviceBuffer` reference is reconstructed.

The name `keepalive` describes the asynchronous reference contract, not an
owning smart pointer. Dropping or clearing the vector never calls
`DeviceBuffer::free`; only the executor-owned `CudaArena` can release that
allocation. Conversely, an arena must not be released merely because the vector
is still nonempty, since the raw entries are not a Rust ownership count.

The method is intentionally internal. Its safe signature cannot express the
required asynchronous borrow, so the caller must uphold the lifetime contract
described in [Unsafe invariants](#unsafe-invariants).

### `unsafe enqueue(stream, function, config, argument_count)`

`enqueue` performs no packing. It projects the already prepared storage into the
two slices expected by `Stream::enqueue_launch`:

```text
keepalive_refs = &[&DeviceBuffer<'static>]
parameters     = &mut [*mut c_void]
```

The first projection uses `slice::from_raw_parts` over the raw pointer vector.
The second uses `slice::from_raw_parts_mut` over the pointer array and the caller
supplied `argument_count`. The method then calls
`stream.enqueue_launch(function, config, parameters, keepalive_refs)` and returns
the `recipe_cuda::Result<()>` unchanged. A debug assertion checks the internal
`parameters.len() == values.len()` relation after the call.

The casts rely only on pointer representations, not on a typed reinterpretation
of the pointed-to values: `*mut u8`, `*mut c_void`, and `*const DeviceBuffer`
have the corresponding raw-pointer representations on the required 64-bit
target. The pointee type of each `values` slot remains `u64`; CUDA receives its
address and reads eight bytes from that address. `recipe-cuda` rejects non-Linux
and non-64-bit builds at compile time, which is the platform contract behind the
CUDA Driver pointer aliases ([`cuda/src/lib.rs`](../../../cuda/src/lib.rs#L1-L14)).

The `argument_count` parameter is deliberately separate from `self.len()`. It
allows one completion-slot block, sized for the maximum ABI of all tasks using
that slot, to submit a shorter ABI without passing stale trailing slots. The
unsafe precondition is `argument_count <= self.parameters.len()` and every
passed slot must already contain the value required by the current cubin ABI.
Passing a larger count makes `from_raw_parts_mut` create a slice beyond the
allocation and is undefined behavior. Passing a smaller count is valid and
ignores the unused tail.

### Edge behavior

| Case | Observed behavior | Production status |
| --- | --- | --- |
| `argument_count == 0` | `new` creates empty arrays; `enqueue` ultimately passes a null `kernelParams` pointer. | Not a valid Recipe kernel ABI because `ElementCount` is mandatory. |
| `0 < argument_count < self.len()` | Only the prefix is passed. Tail values and pointer entries remain allocated but are not visible to CUDA. | Valid for a shorter ABI sharing a completion slot. |
| `argument_count == self.len()` | Every slot is passed. | Normal when the block has one ABI size. |
| `argument_count > self.len()` | The raw slice constructor is out of bounds. | Impossible through validated `invocation_sizes`; calling `enqueue` this way is undefined behavior. |
| `set_value(index == len)` | Returns `Error::IntegerOverflow` and writes nothing. | Indicates an internal ABI/count mismatch. |
| Same arena retained for multiple pointer slots | Multiple raw references are stored. | Valid for read/write aliases; capacity counts references, not unique allocations. |
| `reset_keepalive` before stream idle | Clears the only recorded non-owning references. | Forbidden by the completion-slot lifecycle because it can permit premature arena reuse or release. |
| Context mismatch | `Stream::enqueue_launch` returns `CudaError::ResourceContextMismatch` before `cuLaunchKernel`. | A finalized binding should make this unreachable. |
| Closed stream or context | The wrapper returns `CudaError::ContextClosed`. | A resource-lifecycle violation, not a retry case. |

The table distinguishes representable states from valid production states. The
FFI type intentionally does not add defensive branches for impossible states;
the finalized plan and executor lifecycle establish the preconditions once.

## Packing call chain

### Preparation and allocation

`CudaResources::realize` validates the immutable plan and calls `realize_device`
for each bound CUDA device. The device realization loads each distinct cubin
module once, resolves each ABI entry to a `Function`, and stores functions in
`LoadedArtifact`. Modules are boxed so their addresses remain stable; artifacts
are dropped before modules during teardown
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L1817-L1847)).

`Module::function` initially returns `Function<'module, 'context>`, borrowing the
module for the local lookup scope. Realization explicitly transmutes that value
to `Function<'context, 'context>`. The source safety proof is that the module is
in a stable `Box` owned by `DeviceResources`, the artifact table is dropped before
the module table, and functions never escape those resources. This is the module
and function lifetime proof that the later `ParameterBlock::enqueue` call relies
on; `ParameterBlock` does not repeat or replace it
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L1832-L1847)).

`invocation_sizes` visits calculation tasks for the device and records the
maximum `abi.arguments.len()` for each `CompletionSlotId`. One
`ParameterBlock::new(count)` is then created per entry
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L1906-L1934),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L1850-L1853)). This is a
pre-loop allocation. There is no `ParameterBlock` allocation in submission or
polling.

The finalized plan disallows overlapping tasks from sharing a completion slot.
Therefore a block can be reused for non-overlapping tasks with different ABI
lengths, while a block cannot be reset while a previous launch on that slot is
still running ([`core/src/plan.rs`](../../../core/src/plan.rs#L1142-L1175)).

### `fill_invocation`

`submit_calculation` obtains the artifact and the block selected by the planned
completion slot, then calls `fill_invocation` before computing launch geometry
and invoking the FFI boundary
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L696-L737)). The packing
algorithm is deterministic:

```text
require block.len() >= abi.arguments.len()
block.reset_keepalive()
locations = work.inputs followed by work.outputs
fault_consumed = false

for (index, argument) in abi.arguments:
    Buffer:
        location = next(locations)
        arena = checked_arena(location.device, location.bytes)
        value = checked(arena.buffer.device_ptr() + location.arena_offset)
        block.retain(arena.buffer)
    FaultFlag:
        require work.fault_flag exists and has not been consumed
        arena = checked_arena(fault_location.device, fault_location.bytes)
        value = checked(arena.buffer.device_ptr() + fault_location.arena_offset)
        block.retain(arena.buffer)
    RunId:          value = work.run.get()
    LoopIteration:  value = work.iteration.index()
    ElementCount:   value = abi.elements.get()
    block.set_value(index, value)

require no locations remain
require fault_consumed == work.fault_flag.is_some()
```

`checked_arena` proves that the device exists, the location's device matches the
arena, `arena_offset + bytes` does not overflow, and the complete range fits in
the arena allocation. `DeviceBuffer::device_ptr` rejects a closed allocation;
the subsequent pointer addition is checked for `u64` overflow
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L1936-L2040)). The buffer
argument metadata is not reinterpreted here because the finalized plan already
proved its dtype, access, bytes, and alignment. `fill_invocation` does prove
operand count, fault-flag presence and uniqueness, and exact consumption of all
locations.

### Packing proof for one valid submission

Let `M_s` be the pre-realized block length for completion slot `s`, and let `N`
be the current ABI length. Preparation establishes `M_s >= N`. The following
facts then hold for one successful call:

1. `new(M_s)` established `values.len() == parameters.len() == M_s` and made
   `parameters[i]` address `values[i]` for every `i` in `0 .. M_s`.
2. `fill_invocation` writes one value for every `i` in `0 .. N`. Buffer and
   fault slots are checked device addresses, and scalar slots are checked `u64`
   values from the current work item or ABI.
3. `fill_invocation` appends one keepalive pointer for every pointer slot. The
   vector capacity is at least `N - 1` because the final slot is
   `ElementCount`, so valid packing does not allocate in the live loop.
4. `enqueue` creates `parameters[0 .. N]`. Since `N <= M_s`, the raw slice is
   within the boxed pointer allocation, and every element points to initialized
   eight-byte storage.
5. The finalized ABI validation proves the cubin interprets those eight-byte
   slots in the same order and with the same pointer/scalar kinds.
6. `Stream::enqueue_launch` copies the slot values before returning. The arena
   map, loaded function and module, and stream remain owned until idle, so every
   asynchronous device access remains valid.

The proof is inductive across loop repetition: `prepare_loop_pending` permits a
new submission only for a `Ready` token or a terminal token that has been
rearmed. Thus `reset_keepalive` cannot run while the previous launch on that
completion slot is active.

### Launch and completion

After packing, `submit_calculation` derives a one-dimensional grid from
`abi.elements` and `abi.workgroup_lanes`, creates a `LaunchConfig`, and invokes
the unsafe block method with exactly `abi.arguments.len()`
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L720-L737)). The result is
recorded as `PendingSubmission::Stream`, not an event-backed pending token.

`CudaResources::poll` queries the selected stream with `poll_idle`. A pending
status leaves the block and its keepalive pointers untouched. A complete status
marks the `CudaPending` terminal, after which a loop token may be rearmed. The
same block is therefore reset only between terminal completion and the next
iteration ([`native-executor/src/cuda.rs`](../../src/cuda.rs#L554-L577),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L860-L930)).

Dropping an active `CudaPending` is not a completion operation. Its drop path
forgets an active native token rather than pretending the stream is idle. The
resource teardown path still polls every stream and rejects an active stream, so
abandoning a pending token cannot be used to bypass the `ParameterBlock`
keepalive contract ([`native-executor/src/cuda.rs`](../../src/cuda.rs#L1467-L1483),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L2203-L2219)).

The backend adapter is reached through the normal production boundary:
`executor` calls `Backend::submit` or `Backend::submit_loop_iteration`,
`LocalBackend` dispatches `LocalPending::Cuda` to `CudaResources::submit`, and
the calculation arm reaches `ParameterBlock::enqueue`
([`native-executor/src/local.rs`](../../src/local.rs#L1803-L1832),
[`native-executor/src/local.rs`](../../src/local.rs#L1869-L1919)). No internal
test harness or alternate launch path exists.

### Teardown

The block does not free any CUDA resource. `destroy_devices` first requires every
stream to report complete, then destroys available completion events, streams,
functions, modules, pinned buffers, and scratch allocations in the established
resource order. An active stream returns `Error::CudaContract("CUDA stream
remained active during teardown")`, and an active completion slot returns
`Error::ResourceContention` ([`native-executor/src/cuda.rs`](../../src/cuda.rs#L2203-L2237)).
`CudaResources::destroy` calls `ensure_healthy` before entering this routine
([`native-executor/src/cuda.rs`](../../src/cuda.rs#L644-L647),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L988-L993)); a
poisoned resource set returns `Error::BackendPoisoned` without attempting an
explicit second teardown. This preserves the original live-operation failure
and leaves ordered cleanup to the surrounding executor failure path.
Arena ownership remains outside the block. The executor's lifecycle must release
arenas only after all pending calculations have reached terminal completion.

## Unsafe invariants

These are the obligations at the two raw-slice casts and at the downstream CUDA
call. They are part of the contract, not optional defensive checks.

| Invariant | Established by | If violated |
| --- | --- | --- |
| `values` has one live, initialized eight-byte slot for every passed argument. | `ParameterBlock::new`, `fill_invocation`, and `set_value`. | Driver reads uninitialized or out-of-range host memory; a count beyond the slice is undefined behavior. |
| Every `parameters[i]` points to `values[i]` and has valid pointer alignment. | Boxed `u64` allocation and one-time pointer construction in `new`. | CUDA reads the wrong bytes or invalid host memory. |
| Pointer-array casts preserve element size and alignment. | The target is 64-bit and all three raw-pointer types are used only as pointer storage; no casted pointee is dereferenced by Rust. | The C call receives a malformed `kernelParams` array. |
| `argument_count <= self.parameters.len()`. | `submit_calculation` passes the validated ABI length; `invocation_sizes` allocated the per-slot maximum. | `from_raw_parts_mut` is out of bounds and undefined behavior. |
| The first `argument_count` values match the cubin entry ABI exactly. | Finalized artifact validation plus `fill_invocation`'s ordered match. | The Driver cannot diagnose a Rust-side type mismatch; the kernel may receive corrupted arguments or fail at launch. |
| Every keepalive pointer is a valid, aligned `DeviceBuffer` reference for the whole stream operation. | `retain` is called only with arena buffers; arena and pending lifetimes are owned by the executor. | Reconstructing `&DeviceBuffer` is undefined behavior and asynchronous use can become use-after-free. |
| Keepalive buffers and the stream, function, and module share one live CUDA context. | `CudaResources::validate_binding`, `checked_arena`, and `Stream::enqueue_launch`'s `same_context` checks. | The wrapper returns `CudaError::ResourceContextMismatch`, or an invalid caller reaches the unsafe boundary. |
| The block is not reset while its previous launch is active. | Non-overlapping completion-slot validation and terminal stream polling. | `reset_keepalive` can erase the only references protecting an in-flight kernel. |
| The function and module outlive the queued kernel. | `LoadedArtifact` borrows stable boxed modules; artifact entries are dropped before modules. | CUDA may execute through an unloaded function or module. |
| The stream is polled to idle before any retained arena or module is destroyed. | Executor lifecycle and `destroy_devices` stream checks. | Device memory, code, or host storage can be released while the Driver is still accessing it. |

`ParameterBlock` itself cannot prove the asynchronous rows in this table. Its
`unsafe enqueue` marker and private visibility force the proof to remain at the
CUDA resource boundary.

## Error propagation

The block and its caller expose a small, exact error surface:

| Site | Error | Meaning |
| --- | --- | --- |
| `set_value` | `Error::IntegerOverflow { field: "CUDA launch argument index" }` | The requested slot does not exist. |
| `fill_invocation` length check | `Error::ResourceContention` | A completion-slot block is smaller than the finalized ABI. |
| `fill_invocation` operand or fault checks | `Error::Protocol` | Work and the validated artifact disagree about buffer or fault operands. |
| `checked_arena` | `MissingDevice`, `ArenaMismatch`, `ValueMismatch`, or `IntegerOverflow` | A resolved location cannot be mapped to a checked range in its CUDA arena. |
| `DeviceBuffer::device_ptr` | `recipe_cuda::CudaError::ContextClosed`, wrapped as `Error::Cuda` | The arena buffer was already closed. |
| Launch geometry before the block call | `Error::IntegerOverflow` or `Error::Cuda(InvalidInput)` | The finalized element count and workgroup width cannot form a `u32` grid or a nonzero `Dim3`. |
| `Stream::enqueue_launch` preflight | `CudaError::ResourceContextMismatch` or `CudaError::ContextClosed`, wrapped as `Error::Cuda` | Stream, function, or retained buffer contexts are incompatible or closed. |
| Raw Driver call | `CudaError::DriverCall` with operation `cuLaunchKernel`, wrapped as `Error::Cuda` | The Driver returned a non-success numeric status. The status-only path does not allocate error-name or error-string text. |
| `CudaResources::poll` on a stream-backed calculation | `Error::Cuda`, then `BackendPoisoned` on the next operation | A failed `cuStreamQuery` is converted through `self.poison`; no second completion mechanism is attempted. |
| Parent `CudaResources::submit` dispatch | Original error, then `poisoned = true` | A failure from `submit_calculation` or another native submission arm poisons the backend; later operations return `Error::BackendPoisoned { backend: "CUDA" }`. Pre-dispatch pending and task-contract validation errors return before this poisoning branch. |

`ParameterBlock::enqueue` does not translate or suppress a CUDA error. It returns
the wrapper result unchanged, and `CudaResources::submit` performs the crate
conversion and dispatch-error poisoning policy ([`native-executor/src/cuda.rs`](../../src/cuda.rs#L480-L510),
[`native-executor/src/error.rs`](../../src/error.rs#L79-L97)). There is no retry,
alternate argument layout, event fallback, or host-side substitute launch.

Errors that prevent the block from being reached remain upstream and observable:

| Phase | Examples |
| --- | --- |
| Driver load | Missing required `cuLaunchKernel` symbol, failed `dlopen`, or a symbol lookup error. |
| Artifact realization | Wrong CUDA target or digest, malformed/non-CUDA cubin, missing entry symbol, or a failed `cuModuleLoadData`/`cuModuleGetFunction` call. |
| Pending preparation | Missing queue or completion slot, task contract mismatch, or duplicate preparation. |
| Work validation | Wrong task class, submission slots, device, route, or finalized operand contract. |

These phases fail before `ParameterBlock::enqueue`; they do not create a fallback
parameter layout. The corresponding checks are in
`ExecutionPlan::validate_scoped`, `realize_device`, and
`CudaResources::prepare_pending` ([`native-executor/src/plan.rs`](../../src/plan.rs#L129-L205),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L1665-L1847),
[`native-executor/src/cuda.rs`](../../src/cuda.rs#L438-L477)).

## Review checklist

For any change at this boundary, verify the following mechanically:

1. The generated `KernelAbi.arguments` order still matches the `fill_invocation`
   match order.
2. Every slot remains exactly eight bytes and eight-byte aligned in host storage.
3. The `argument_count` passed to `enqueue` is the current ABI length and never
   exceeds the preallocated block length.
4. `keepalive` capacity remains sufficient for every pointer argument without a
   loop-time allocation.
5. The block is reset only after stream idle and before all current slots are
   rewritten.
6. No code changes the owner of arena buffers, modules, functions, or streams
   to `ParameterBlock` without updating the lifetime proof.
7. Driver errors remain observable as `Error::Cuda`; do not add a fallback path.

The smallest faithful model is therefore:

```text
validated KernelAbi
        + finalized CalculationWork
        + checked CUDA arenas
        -> fill_invocation(ParameterBlock)
        -> [u64 values, pointers-to-values, non-owning buffer keepalives]
        -> Stream::enqueue_launch(..., kernelParams, extra = null)
        -> CUDA stream execution
        -> poll_idle
        -> terminal block reuse or teardown
```

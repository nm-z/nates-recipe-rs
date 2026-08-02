# recipe-hsa

`recipe-hsa` is Recipe's small, reviewed ROCr/HSA boundary. It dynamically
loads the ROCr runtime, discovers exact agents, ISA identities, and memory
pools, and owns HSA runtime, allocation, executable, signal, and queue
teardown.

The execution slice loads in-memory HSACO objects, resolves exact kernel
symbols, submits single-producer AQL kernel packets, performs asynchronous AMD
memory copies, and returns owned completion tokens. Completion tokens expose
clonable dependency handles. Dependent dispatch lowers any fan-in into a
deterministic tree of five-input AQL barrier-AND packets, then publishes the
kernel behind the terminal barrier without a host wait.

Queue backpressure is reported without advancing the write index. Callers can
probe it through an explicitly bounded, nonblocking progress API. Packet bodies
are populated before release publication of the header, release publication of
the write index, and release ringing of the doorbell.

Dropped incomplete tokens move into a session-owned deferred-retirement set.
Callers can poll or bounded-drain that set; queue poison fails logical
dependents and an unresolved final drop emits a terminal leak diagnostic rather
than releasing device-visible signals or allocations unsafely.

This crate deliberately contains no global runtime, scheduler policy, HIP path,
or vendor operation-library integration. Higher layers must turn discovery into
a finalized static plan and expose these operations only in the appropriate
typestate phase.

The raw ABI declarations in `src/abi.rs` are checked against the public ROCr
headers for the 64-bit HSA large model. Rust compilation checks their structural
use; runtime claims require a complete public Recipe workload on real hardware.

The standalone live smoke command is diagnostic only:

```text
cargo run --features live-hsa --example execute_smoke
```

It can diagnose fine-to-coarse-to-fine asynchronous copies. An exact
two-pointer copy-kernel HSACO can additionally diagnose executable loading and
AQL dispatch by setting `RECIPE_HSA_SMOKE_COPY_HSACO` and
`RECIPE_HSA_SMOKE_SYMBOL`; neither invocation is acceptance evidence by itself.

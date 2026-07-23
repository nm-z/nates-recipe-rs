# recipe-remote

`recipe-remote` is the bounded execution protocol between one already
provisioned Recipe master and worker. It deliberately does not open sockets,
discover hardware, compile kernels, read artifacts, or perform file I/O. The
caller supplies an already connected `recipe_transport::RuntimeChannel`, the
same `ProvisionedProgram` on both peers, and a local `WorkerDriver`.

## Measured-profile boundary

Values in `topology/contract.toml` are theoretical seed estimates for choosing
safe probe workloads. They are not user configuration and must not directly
drive a production schedule.

The intended boundary is:

1. `recipe probe` measures the bare-metal machines, devices, links, bandwidth,
   concurrency, and calculation rates.
2. Draft and Realize use those measured properties to produce the static DAG,
   transfer schedule, reservations, arena layouts, and native artifacts.
3. `ProvisionedProgram` commits those profile-derived facts. The handshake
   proves the exact endpoint profile identities, bundle, draft, realization,
   artifacts, transfer contracts, and program digest before `prepare`.

Both peers therefore fail closed if they were provisioned from different probe
profiles or scheduler results. The protocol never asks a user to type topology
rates or schedules.

## Lifecycle

The public typestates permit only:

```text
Handshake -> Init -> Run -> Exit -> Complete
                  \-> Cancel -> Complete
```

- Handshake is symmetric and proves protocol version, required capabilities,
  endpoint/profile identities, fixed limits, and all plan digests.
- Init accepts exactly one checksummed, chunked logical arena image for each
  worker device and acknowledges it exactly once.
- Run admits only provisioned task IDs. Control, metrics, and scheduled user
  data use fixed-capacity nonblocking transport lanes.
- Opposing full-duplex transfers progress independently. Opposing half-duplex
  transfers that name the same finalized capacity resource share one token.
- Exit releases each worker arena exactly once and waits for its exact
  acknowledgment before the terminal acknowledgment is flushed.

Wire encoding is manual, canonical, bounded by `RemoteLimits`, and allocation
free after session construction. Per-lane sequence numbers plus the nonzero
`RunId` poison malformed, replayed, wrong-run, wrong-lane, and out-of-order
traffic.

## Worker boundary

`WorkerDriver` receives only finalized `RunId`, `DeviceId`, `TaskId`, digest,
and bounded byte-slice contracts. It has no CPU calculation callback, closure,
compiler hook, discovery hook, or vendor-math surface. A native CUDA Driver API
or ROCR/HSA executor can implement it without HIP, cuBLAS, rocBLAS, or other
vendor math libraries.

Terminal driver faults take a distinct cleanup path. Before a fault frame is
sent, `cleanup_after_fault` must quiesce native operations and release all
locally realized resources. A cleanup failure is combined with the primary
fault, so receipt of an ordinary terminal driver fault proves that local
cleanup already completed. Requested cancellation continues to use the normal
per-arena release and acknowledgment protocol.

## Native executor integration

`ExecutorWorkerDriver` binds a validated `recipe-executor::WorkerProjection`
for one exact machine/node assignment. It proves that the wire
`ProvisionedProgram` has the same devices, image sizes, runtime task roles, and
cross-machine transfer contracts before any run begins.

The associated `WorkerBackend` extension pre-realizes external-transfer
tokens, exposes nonblocking ingress and egress, and provides fatal native
quiescence. Ordinary local task submission, polling, arena ownership, and
resource destruction continue through the closed `Backend` ABI. The adapter
never binds foreign-machine devices or executes a task without a matching
master command.

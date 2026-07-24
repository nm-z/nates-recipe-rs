# Native executor remaining integration work

The CUDA and HSA adapters now implement both
`recipe_executor::sealed::Sealed` and `recipe_executor::Backend`.
`BackendWork` remains closed, and no compiler, loader, allocator, discovery, or
topology mutation was added to the loop handle.

The strict-loop storage work is complete. `recipe-executor` borrows finalized
operand, route, and lane-claim slices, and uses caller-owned pending, physical
call, journal, and exit-image storage prepared before `LoopStarted`.
`recipe-hsa` likewise accepts pre-realized submission slots whose packets,
keepalive storage, dependency storage, and completion signals are acquired
before init. CUDA Driver loop errors use the status-only error path.

CUDA pending tokens carry an operation lifetime even though the token stores
only an event and a phantom borrow. The bridge confines the reviewed lifetime
erasure to one function, tracks the exact completion slot, prevents teardown
before terminal completion, and leaks an abandoned in-flight token instead of
allowing referenced resources to be destroyed.

`LocalBackend` now composes the host, CUDA, and HSA adapters over one finalized
bundle. Bind assigns every finalized device and task to exactly one partition,
rejects CPU calculation, and projects the executor-owned arena map into each
child without loop allocation. `StagedCrossBackend` owns pre-realized one-hop
work crossing Host, CUDA, HSA, or CUDA-context boundaries. It uses one
fixed-capacity host worker per selected task plus pinned CUDA or fine-grained
HSA staging, native asynchronous source/destination copies, and pre-created
completion tokens. Submit and poll allocate nothing and never wait for host
work. `RejectCrossBackend` remains available for homogeneous deployments.
Physical accounting stays at the composite boundary, and destroy attempts every
child in deterministic bridge/HSA/CUDA/host order.

The pre-final `CandidateSessionFactory` boundary and
`ValidatedCandidateFactory` enforce exact topology, discovery, Draft,
artifact, reservation, warm-pass, and capacity evidence before Finalize.
`recipe-prepare` can therefore drive a dependency-neutral native session
factory without weakening its fixed-point state machine.

`LocalCandidateFactory` now realizes the candidate's host runtime and pending
pool, CUDA/HSA modules and functions, queues, completion objects, metric and
egress buffers, pinned staging, and scratch before Finalize. It captures live
availability once at init and enforces one-GB RAM/disk headroom plus one-GB
headroom only on GPUs with an enabled display connector; headless GPUs carry an
explicit zero-byte exemption. These are scheduler quotas, not dummy
allocations. `LocalPreparedSession::into_backend` validates the unchanged
finalized bundle and every child/bridge contract before moving those same
objects into one-shot prepared backend states. Bind attaches finalized
addresses and allocates the final packed arenas; it does not load or realize a
second copy. Candidate mismatch destroys bridge/HSA/CUDA/host resources in a
deterministic order. `PreparedNativeSession::into_parts` is the corresponding
preparation-to-runtime ownership handoff.

Checked scalar kernels carry an immutable `FaultFlag` arena location through
finalized calculation work, and both adapters bind that exact ABI argument.
The remaining work is a preplanned readback/control operation that surfaces a
set flag without adding an unscheduled loop transfer or host branch.

The remaining integration work is:

1. Make poisoned-run teardown ordered and total. Native resources currently
   refuse destruction while unhealthy; cancellation, terminal observation, and
   release must remain possible after a device or transport failure.
2. Execute the live-hardware matrix (NVIDIA K80/M60 and AMD V340L), including
   init, concurrent loop work, scheduled metrics, exit readback, and ordered
   release. Mock tests prove contract validation but do not substitute for the
   driver and ISA acceptance runs.

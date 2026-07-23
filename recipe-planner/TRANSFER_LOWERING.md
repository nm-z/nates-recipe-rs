# Physical transfer lowering

The planner evaluates every simple directed route from each ready resident
source to the requested destination. A route is costed as the exact sequence
that the executor will see:

- one `TransferTask` per directed link;
- one preallocated intermediate `ValueSpec` at every internal route endpoint;
- an explicit dependency from each hop to the preceding hop;
- a queue and completion slot owned by the hop's source device; and
- one statically selected lane claim for the hop's single link.

Trial scheduling uses the complete hop chain, so measured bandwidth, existing
link contention, per-link inflight lanes, half-duplex direction conflicts,
compute/transfer overlap, source readiness, final readiness, and total
makespan all participate in source and route selection.

The final destination retains the first available physical `ValueId`. Temporary
route values follow it in deterministic route order. Only the final value is
registered as the logical tensor copy; intermediates remain physical arena
objects whose lifetimes are derived from their producer and consuming hop.

Same-device copies, when required by a caller, remain explicit transfer tasks
with an empty route. The scheduler rejects a task containing more than one
directed link, so a validated Draft cannot expose a composite transfer to an
executor.

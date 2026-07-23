# Recipe system contract

Status: normative  
Accepted: 2026-07-23

This document turns the human-written system specification and the accepted
recommendations in `streamed-questing-wozniak-audited.md` into implementation
rules. If legacy behavior conflicts with this document, the legacy behavior is
not precedent.

## Scope

Recipe is a GPU-only calculation system with an ahead-of-run, immutable
calculation-and-transfer schedule. It supports AMD and NVIDIA GPUs with equal
semantic behavior and supports values moving between vendors, devices,
machines, and nodes through explicit routes.

The implementation languages and artifact forms are:

- Rust for host orchestration and public APIs;
- Zig and LLVM IR for owned kernel generation;
- AMDGPU ISA objects and HSACO code objects for AMD;
- PTX assembly and cubins containing generated SASS for NVIDIA; and
- small reviewed assembly kernels where a backend ABI or semantic fallback
  requires them.

Generated SASS in a validated cubin satisfies the NVIDIA SASS artifact
requirement. Recipe does not encode undocumented SASS instructions directly.

HIP and vendor operation libraries are prohibited. In particular, the shipped
implementation may not call, link, load, or generate calls to rocBLAS,
rocSOLVER, rocFFT, MIOpen, RCCL, cuBLAS, cuSOLVER, cuFFT, cuDNN, NCCL, or their
HIP wrappers. AI, DL, ML, solver, FFT, and collective behavior is composed from
Recipe-owned scalar programs, kernel templates, transfers, and control edges.

## Normative terms

- **Cluster**: the complete configured collection of machines, devices, links,
  and nodes.
- **Machine**: a host such as `engi`, `archy`, or `sentry`. Names are
  configuration values, not variants in code.
- **Device**: a schedulable storage domain: VRAM/GPU, RAM, or disk. A CPU
  controls work but is not a calculation device.
- **Node**: a logical role bound to devices for a run. There is exactly one
  master node and zero or more worker nodes.
- **Calculation**: payload arithmetic. Every production calculation executes on
  a GPU and uses only f32 or int32 payload values.
- **Transfer**: an explicit movement of bytes between storage devices.
- **Asynchronous**: submission returns a completion token before the submitted
  work is required to complete. Waiting is a separate action.
- **Concurrent**: two independent ready tasks may be active during overlapping
  time intervals when the selected device and resources support it.
- **Parallel**: a nontrivial index space is divided among multiple GPU lanes or
  workgroups. Cardinality-one work and true dependency chains are structural
  exceptions, not fake parallel work.
- **Bidirectional**: both directed edges exist. It does not by itself imply
  simultaneous use.
- **Full duplex**: opposing directed edges use independent capacity resources
  and may overlap.
- **Half duplex**: opposing directed edges share one capacity resource and may
  not overlap.
- **Interoperability**: a value produced on one vendor is consumed in a
  calculation on the other vendor. Coexistence alone is insufficient.
- **Upload**: one logical admission of the packed external data image into a
  device's run arena during `init`.
- **Download**: external result or file egress. Downloads are forbidden during
  the loop.
- **Free**: one logical release of a device's complete run arena during `exit`.
  Destruction of runtime objects is accounted separately.
- **GB**: exactly `1_000_000_000` bytes. Binary quantities must be named GiB.

## Contract decisions

### C1: scalar and in-place calculation

Scalar operations form a typed intra-kernel SSA-like program. Kernel templates
apply scalar programs over a parallel index space and declare whether each
input may alias each output. Reductions, scans, contractions, barriers, atomics,
and transfers are not scalar operations.

### C2: significant-digit guarantee

The lossless claim is an ingestion and representation round-trip guarantee:

- finite decimal values containing at most six significant digits and within
  the finite f32 range round-trip through the f32 calculation representation at
  that decimal precision; and
- in-range decimal integers containing at most nine significant digits
  round-trip through int32 exactly.

Arithmetic accuracy is defined per operation. No claim is made that arbitrary
floating-point operation sequences are lossless.

### C3: quantity units

Capacity and bandwidth inputs use decimal bytes and seconds. The estimate
fixture therefore contains 1 GB = `1_000_000_000` bytes and 1 GbE =
`125_000_000` bytes/second.

### C4 and C14: required devices

`recipe probe` discovers every VRAM, RAM, disk, and directed link on each
participating machine. Every device in the resulting accepted profile
participates in the run; a CPU is not a calculation target. An unavailable
machine or a device that disappears from a cached profile makes preparation
incomplete and fails closed; it is not silently removed. A different discovered
topology produces a different profile and requires a new preparation pipeline.

Hardware product names are fixtures. Backend selection is based on discovered
capabilities and exact artifact identity, never a product-name branch. Users do
not manually enter device identities, capacities, rates, or links.

### C5: exact user reservation

Before Recipe's arena is finalized, every required VRAM, RAM, and disk storage
device holds a named user-owned allocation or enforceable quota of exactly
`1_000_000_000` bytes. Recipe runtime overhead, driver use, fragmentation, and
safety headroom are separate ledger entries. Extra unused capacity does not
change the reservation's size.

The reservation protects capacity within Recipe's accounting domain. Recipe
does not claim it can prevent an unrelated process or the operating system from
consuming globally free memory.

### C6: one upload and one free

Each required device receives exactly one logical external data-image admission
during `init` and exactly one logical run-arena release during `exit`. Packing,
chunking, physical DMA calls, driver allocations, resource-pool destruction,
and staged internal transfers are recorded separately.

### C7: loop transfers

The loop may transfer only bytes already owned by the run or produced by a
scheduled operation. External dataset/model/file ingress and result/file egress
are forbidden. Disk, RAM, staging, peer, and network movement must appear as
preplanned transfer nodes.

### C8: control and artifact traffic

Discovery, artifact compilation/loading, module initialization, kernel
arguments, dependency tokens, completion messages, and control messages are not
user-data uploads. All code loading, lazy initialization, argument storage,
resource pools, and plan transport complete before `init`. The loop may use only
preallocated instances.

### C9: ahead-of-run boundary

Public `prepare` is the complete fixed-point pipeline:

1. discover every required capability;
2. draft the exact DAG, artifacts, placement, routes, geometry, resource bounds,
   and logical memory demand;
3. realize exactly that draft by compiling where permitted, loading, warming,
   creating resource pools, stabilizing opaque allocation, and recording
   capacity; and
4. finalize immutable arena offsets and the execution bundle without changing
   any drafted choice.

If realization or finalization cannot fit, that candidate is destroyed and a
new draft is attempted. `init` may bind realized resources, allocate the fixed
arenas, and admit the data images; it may not replan.

The loop may not place, route, compile, load, allocate, resize, discover, or
change topology. Data-dependent behavior uses a predeclared bounded template
and device-resident predicates.

### C10: compilation policy

Offline builds and the Realize portion of `prepare` may compile artifacts,
including PTX-to-cubin compilation or driver JIT when an exact artifact policy
explicitly permits it. Every resulting identity and toolchain version is
hashed. Compilation and JIT are prohibited during Finalize, `init`, the loop,
and `exit`.

Deployment profiles may require precompiled cubins and HSACOs. In particular,
the R470 `sm_37`/`sm_52` deployment uses pinned CUDA 11.x-compatible offline
cubins rather than a current-toolkit assumption.

### C11: property provenance

Every capacity, bandwidth, FLOP rate, and transfer rate is paired with one of
`Estimated`, `Measured`, or `Override`. The human property table in
`topology/contract.toml` is the deterministic set of theoretical seed estimates
used to size the first bounded probe workloads. It is not a deployment profile.

`recipe probe` discovers the machine on bare metal and benchmarks its actual
capacity, calculation rate, memory-transfer rate, and directed-link bandwidth.
It emits a versioned, hashed measured profile. Normal preparation requires that
measured profile and the scheduler consumes its measured values. An explicit
override is for controlled testing or an administrator-approved exceptional
deployment; it is never produced by silently rewriting an estimate.

A measured profile retains each machine fingerprint and the exact discovered
RAM-domain, storage-domain, and GPU key associated with every topology device.
Preparation and backend realization reopen devices by those identities, never
by product name, ordinal, capacity, or performance similarity. These keys are
probe output; a user does not enter them.

A cached measured profile is invalidated by a machine, device, driver,
runtime-ABI, firmware, link, or artifact-toolchain identity change.

### C12: assembly and artifact meaning

PTX is NVIDIA assembly source. A cubin containing inspected generated SASS
satisfies the SASS requirement. AMD artifacts contain inspected AMDGPU ISA.
Recipe owns the source-to-artifact pipeline, validates symbols and metadata, and
may include hand-written ISA kernels. Direct encoding of undocumented SASS is
not required.

### C13: public operation surface

The compatibility boundary includes the root `Data`, `Model`, `Train`, and
`Infer` behavior; parsing, encoding, tokenization, model formats, preprocessing,
training, inference, and chat; and every currently exported GPU operation
family. The owned operation manifest includes:

- scalar expressions, conversion, elementwise, creation, shape, indexing,
  gather/scatter, search, set, sort, encoding, padding, and foreach;
- reduction, scan, histogram, statistics, metrics, loss, and distance;
- contraction/GEMM, solver, FFT, convolution, pooling, activation,
  normalization, embedding, attention, recurrent/sequence/SSM, MoE, optimizer,
  forward, backward, inference, quantization, and diffusion;
- graph, clustering, Bayesian, SVM, RL, tree/forest, CatBoost, XGBoost, and
  LightGBM behavior; and
- model loading, safetensors, GGUF, dequantization, tokenization, KV state,
  sampling, and chat.

The manifest is finite: it covers the shipped repository surface, not every
algorithm that could ever be called AI, DL, or ML. New operations must be
composed through the same owned primitives.

### C15: repeated and overlapping runs

Every run has a unique `RunId` and the state sequence
`Prepared -> Initialized -> Running -> Exited`. Repeated runs repeat the entire
lifecycle. Multiple runs may overlap only when preparation assigns disjoint
arenas, reservations, queue slots, dependency tokens, and runtime-object
budgets. Otherwise preparation rejects overlap.

### C16: asynchronous, concurrent, and parallel calculations

Every calculation uses nonblocking GPU submission. Independent ready tasks
overlap when the realized capability profile permits it. Every nontrivial index
space is data-parallel. Cardinality-one tasks and dependency-serialized chains
retain nonblocking GPU submission but are recorded structural exceptions to
internal parallelism or mutual concurrency.

### C17: host activity classification

CPU work may perform orchestration, parsing, decompression, protocol framing,
artifact validation, topology discovery, scheduling, and construction of
shapes, strides, indexes, and metadata. Production transformations of
calculation payload values—including numerical preprocessing and token scoring—
are calculations and therefore execute on GPUs.

Independent CPU or high-precision mathematical implementations are test-only
oracles and are not shipped runtime calculation paths.

### C18: payload types

Calculation payload values are exclusively f32 and int32. Raw file bytes,
quantized encodings, booleans, IDs, indexes, addresses, shapes, strides,
protocol metadata, and opaque handles may use suitable non-payload types. They
cannot enter payload arithmetic without an explicit checked conversion to f32
or int32.

### C19: native interface ownership

On AMD, Recipe calls ROCr/HSA and submits HSA AQL. ROCr owns KFD interaction;
Recipe does not maintain a competing raw-KFD queue or memory path.

On NVIDIA, Recipe uses the CUDA Driver API. It does not use the CUDA Runtime
API. The callable symbol set is capability-gated, including an R470-safe subset
that does not require `cuModuleGetLoadingMode`.

Operating-system APIs, Ethernet/WLAN sockets, file APIs, and the pinned Zig,
LLVM, lld, and ptxas build tools are allowed in their declared phases. No
allowed interface permits a prohibited vendor operation library.

### C20: scheduler equations

The scheduler applies the specified equations to the active probed profile:

```text
calculation_time = operation_FLOPs / measured_device_FLOPs
transfer_time    = bytes / measured_link_bandwidth
```

Theoretical values from `topology/contract.toml` seed the probe only. They do
not compete with or overwrite its measured output. A separately named extended
performance policy may add measured launch latency or device-memory traffic,
but the base scheduler remains the two equations above with deterministic
tie-breaking.

### C21: numerical semantics

The base scalar contract is:

- f32 storage and ordinary arithmetic use IEEE-754 binary32,
  round-to-nearest-ties-to-even;
- subnormal inputs and results are preserved; a device must use a semantic
  fallback or be rejected if the selected operation cannot meet that rule;
- `Mul` followed by `Add` does not contract; `Fma` is a separate explicit
  operation with one rounding;
- positive and negative zero are preserved by ordinary IEEE operations;
- invalid operations produce a canonical quiet NaN; operations otherwise
  propagate infinity according to their documented IEEE definition;
- transcendental and approximate functions declare their finite domain,
  maximum error, special-value behavior, and fallback individually;
- int32 add, subtract, and multiply use two's-complement wrapping;
- int32 division truncates toward zero; division by zero and
  `INT_MIN / -1` are rejected by a checked operation rather than invoking
  target-specific behavior;
- shifts are explicit logical/arithmetic operations and mask the count to five
  bits;
- f32/int32 bitcasts preserve all 32 representation bits and are distinct from
  numerical conversion;
- a scalar `Require` normalizes an int32 truth value and atomically records a
  device fault in preallocated storage when the value is zero; it does not
  branch into host work or allocate an exception object in the loop;
- f32-to-int32 conversion is explicit: NaN becomes zero, finite out-of-range
  values saturate, and finite in-range values truncate toward zero;
- int32-to-f32 conversion uses round-to-nearest-ties-to-even;
- reductions use a statically fixed tree and recorded order;
- conflicting non-atomic writes are invalid; atomic operations define their
  ordering and result domain; and
- random operations use a Recipe-owned counter-based generator keyed by seed,
  run, operation, and element index. Scheduling order does not change output.

Operation-specific contracts may narrow domains or tolerances but may not
silently weaken these base semantics.

## Probe seed fixture

`topology/contract.toml` declares the theoretical starting estimates:

| Resource | Capacity/rate |
|---|---:|
| Ethernet | 125,000,000 bytes/s |
| Disk | 1,000,000,000,000 bytes |
| SATA III | 600,000,000 bytes/s |
| GPU VRAM | 12,000,000,000 bytes |
| PCIe 3.0 | 16,000,000,000 bytes/s |
| GPU calculation | 380,000,000,000 FLOP/s |
| GPU VRAM transfer | 432,000,000,000 bytes/s |
| CPU RAM | 48,000,000,000 bytes |
| DDR5 | 90,000,000,000 bytes/s |
| CPU reference rate | 150,000,000,000 FLOP/s |
| RAM transfer | 90,000,000,000 bytes/s |

The CPU reference rate is descriptive and may be used for cost reporting; it
does not make the CPU a legal calculation target.

PCIe, NVMe, SAS, and Ethernet are full duplex when the discovered route says
opposing directions have independent resources. SATA and WLAN are half duplex
and represent both directions sharing one resource.

`engi`, `archy`, and `sentry` and the RX 7700 XT, V340L, two Tesla M60s, and two
Tesla K80s are project acceptance fixtures, not product branches or values a
user must configure. Each participating host runs `recipe probe`; cluster
discovery combines the resulting profiles. Recipe does not infer the V340L's
ISA or capabilities from its product name.

## Lifecycle

Machine profiling precedes preparation. The only legal run state transition is:

```text
recipe probe -> measured DiscoveryProfile
             -> draft -> realize -> finalize
             -> init -> loop -> exit
```

`prepare` comprises the first four stages. Each stage consumes the prior typed
state. A running handle exposes no compiler, loader, allocator, topology
mutation, external input, or general download API.

Metrics use preallocated slots and a bounded, nonblocking channel. When the
consumer falls behind, the newest value for each metric replaces its older
unconsumed value. Metric egress never backpressures calculations. Simultaneous
control ingress and metric egress use independently scheduled directions.

## Failure policy

Unsupported semantics, missing hardware, incompatible artifacts, unavailable
required machines, exhausted capacity, unresolved routes, driver-symbol
mismatches, asynchronous device faults, negative HSA signals, and watchdog
expiry all fail closed before or during the affected run. They never cause
implicit CPU calculation, device deselection, hidden serialization that violates
a required overlap, hidden ingress/egress, or selection of a near-match
artifact.

## Requirement-to-validator map

| Requirement | Validator |
|---|---|
| Hierarchy and stable ownership | topology identity and ownership validator |
| Any node/machine/device sequence | route reachability and placement validator |
| Decimal representation guarantee | bounded f32/int32 ingestion round-trip validator |
| f32/int32-only calculations | scalar-program and value-domain validator |
| GPU-only calculations | placement validator |
| Async calculations/transfers | backend capability and submission validator |
| Full/half-duplex behavior | directed-resource contention validator |
| Static AOT schedule | draft/finalize immutability validator |
| Exact one-GB reservation | capacity-ledger validator |
| One init upload/device | lifecycle event validator |
| Zero loop ingress/egress | running-capability and event validator |
| One exit free/device | lifecycle event validator |
| No HIP/vendor math | dependency, source, symbol, IR, and artifact validators |
| AMD/NVIDIA parity | shared semantic-contract validator |
| Cross-vendor interoperability | producer/consumer route validator |
| Repeated runs | run-identity and typestate validator |
| Metrics exception | bounded nonblocking metric-channel validator |

Engineering tests exercise each validator. A ceremonial proof-of-work report is
not a product deliverable.

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

At preparation start, Recipe captures each required device's live available
capacity exactly once. The scheduler then enforces a Recipe allocation ceiling
derived from that immutable init snapshot. It does not consume the protected
bytes with a dummy allocation.

- Every RAM and disk device retains exactly `1_000_000_000` bytes.
- A GPU with one or more live DRM connectors retains exactly
  `1_000_000_000` bytes for the user's display context.
- A GPU with zero live display connectors records an explicit zero-byte
  headroom exemption.

Recipe-owned runtime overhead is charged inside the allocation ceiling before
arena capacity is finalized. Driver use, fragmentation, and the user headroom
remain separate ledger evidence. Extra unused capacity does not change the
headroom.

The reservation protects capacity within Recipe's accounting domain. Recipe
does not claim it can prevent an unrelated process or the operating system from
consuming globally free memory. Before a model is saved, all transient VRAM,
RAM, disk pools, staged data, generated artifacts, and run arenas are released.
The save path is rechecked against live disk availability so the temporary
atomic output and the retained user headroom both fit.

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

Independent CPU or high-precision mathematical implementations may be external
acceptance oracles, but are not shipped runtime calculation paths.

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

### C22: parameterless activation definitions

The parameterless activation declarations have one canonical binary32 definition in training, inference, and saved
architecture:

- `.leak()` is leaky ReLU with the exact f32 constant `alpha = 0.01`; at zero it selects the nonpositive branch.
- `.sigmoid()` and `.tanh()` use the versioned Recipe scalar-math programs and their declared finite domains, fault
  behavior, and signed-zero rules.
- `.selu()` is `lambda * elu(x, alpha)` with exact f32 constants `alpha = 1.6732632` and `lambda = 1.050701`; at zero
  its derivative selects the nonpositive ELU branch.
- `.elu()` is ELU with the exact f32 constant `alpha = 1.0`; at zero it selects the nonpositive branch.
- `.log()` is the signed logarithm `sign(x) * ln(abs(x))` for finite `x != 0`. Positive and negative zero are both
  domain faults. Its derivative is `1 / abs(x)` throughout that domain.
- `.ln()` is the ordinary natural logarithm `ln(x)` for finite `x > 0`. Positive zero, negative zero, and every
  negative input are domain faults. Its derivative is `1 / x` throughout that domain.

Their backward programs are the analytic derivatives of those exact forward definitions. The activation identity is
serialized in the semantic model; constants are part of the corresponding Recipe-owned scalar program and are not
runtime options. New semantic models serialize `.log()` as `signed-logarithm` and `.ln()` as `natural-logarithm`.
The historical `logarithm` token decodes only as the provisional legacy transform `sign(x) * ln(1 + abs(x))`, so an
old model retains its old calculation and can never be silently reinterpreted as either current declaration.

### C23: distilled vector dtype authority

Semantic distillation selects the smallest lossless fixed-width calculation representation for every vector. `F32`
maps to Recipe `f32`; `I32`, relative-time, dictionary, and ordinal encodings map to Recipe `int32`. Text and byte
encodings remain variable-width and acquire no scalar dtype unless a declared operation supplies a typed lowering.

That selected dtype is immutable source metadata. Training and validation inputs retain it exactly; a loss may emit an
explicit GPU conversion when its mathematical domain is `f32`, but that conversion does not rewrite the vector schema.
Task and output width derive from the saved semantic type, encoding, loss, and fitted dictionary—not from host casting.
Metric calculation tensors and probability outputs are `f32`; multiclass label codes remain `int32`. Semantic models
serialize the target encoding, resume validates it, and target-free inference exposes both its `f32` prediction dtype
and the source target dtype used to interpret the saved task.

### C24: reversible structural GGUF v3 conversion

`recipe convert SOURCE.gguf MODEL.ogdl` and the reverse conversion implement the GGUF v3 format and every active
GGML tensor encoding in the upstream 2026-07-29 format snapshot. Both byte orders, all metadata scalar types, nested
typed arrays, zero-rank and zero-extent tensors, and every current encoded block layout are in scope. A newer GGUF
version or unknown tensor or metadata type is rejected; a current construct is never classified as unsupported merely
to reduce implementation scope.

The structural OGDL records version, byte order, alignment, exact binary length, metadata and tensor ordering, typed
metadata values, dimensions, tensor types, offsets, and named fields for every encoded tensor block. IEEE values are
written as exact sign, exponent, and fraction components; packed quantization values are written as their named scale,
code, mask, index, or other layout fields. It contains no copied source image, raw-byte node, opaque payload, or
base64/hex encoding. Required zero padding is reconstructed from declared layout rather than embedded in the OGDL.
Metadata keys remain exact bounded nonempty UTF-8 strings; Recipe does not reject punctuation such as the hyphen in
current upstream `command-r.*` namespaces merely because it violates the documentation's preferred naming convention.

For unedited converter output, `.gguf -> .ogdl -> .gguf` must reproduce every source byte, including field and tensor
order, scalar bit patterns, offsets, alignment gaps, optional canonical padding of tensor-free files, and optional zero
padding from the final tensor end to the next declared-alignment boundary. Conversion is
streamed in bounded passes and retains descriptors plus at most one tensor block or scalar chunk, not the complete
binary or expanded structural text. CLI outputs are private, created without overwrite, synchronized on success, and
removed after a failed conversion.

### C25: parameterless binary focal objective

The public `focal` objective consumes final linear logits and exact binary targets after semantic target lowering. Its
fixed constants are `alpha = 0.25` and `gamma = 2.0`; they are part of the serialized objective identity and are not
runtime tuning controls. For signed logit `u = z` when `y = 1` and `u = -z` when `y = 0`, Recipe computes
`p_t = sigmoid(u)`, `q = sigmoid(-u)`, and `a = softplus(-u)`, then emits `loss = alpha * q^2 * a`. The logit gradient
is `-alpha * q^2 * (gamma * p_t * a + q)` for `y = 1` and its positive counterpart for `y = 0`.

Forward and backward use the same Recipe-owned binary32 scalar program. Binary validation reports the selected focal
loss while probability metrics continue to consume `sigmoid(z)`. Semantic checkpoints encode
`binary-focal-with-logits-alpha-0.25-gamma-2` so resume cannot silently change either constant or the logits domain.

### C26: learned PReLU activation

Every ordered `.prelu()` occurrence owns one distinct binary32 scalar `alpha`, initialized to the exact f32 value
`0.25`. Its forward definition is `x` when `x > 0` and `alpha * x` otherwise. Its backward definition is `upstream`
for the input when `x > 0` and `alpha * upstream` otherwise; the per-element slope contribution is zero when `x > 0`
and `upstream * x` otherwise. Therefore zero follows the nonpositive input-gradient branch and contributes exactly
zero to the slope gradient.

Recipe reduces every slope contribution over the complete logical training partition before optional global gradient
clipping and the AdamW transition. Repeated declarations never alias: each scalar has its own parameter, first moment,
second moment, update, and ordered resume ordinal. Semantic models serialize the activation order and all three saved
scalar images. Target-free inference admits the saved scalar for the matching layer and occurrence and never replaces
it with the initialization constant.

### C27: target-free inference terminal

`.infer().evaluate()` is an execution terminal, not a declaration-only success. It atomically consumes the immediately
preceding data and model declarations, requires a semantic model loaded from `.ogdl`, distills the ordered source set
under bounded ingest limits, applies only the feature schema, dictionaries, normalization state, topology, parameters,
and prediction interpretation saved in that model, and compiles one immutable inference program. Training targets,
splits, and newly fitted normalization state do not enter inference.

Declared row and column exclusions remain target-free selection policy. Predicates are evaluated against the original
row before excluded helper columns are removed; retained rows and columns preserve source order, and the saved model
schema is applied only after that selection. An excluded required model feature therefore fails as missing rather than
being silently restored.

Native inference uses the same measured-profile, hardware-derived host tuning, artifact compiler, planner, and
`init -> loop -> exit` lifecycle as training. All feature and parameter bytes enter during `init`; inference has exactly
one loop iteration and no loop ingress, egress, user metrics, optimizer state, or target values; the prediction is
copied only by the finalized `exit` transfer. `.evaluate()` returns only after ordered teardown, with the completed
journal and the exact validated f32 prediction image.

The terminal always streams one prediction record per source row. Binary models report their probability, regression
models report their scalar value, and multiclass models report every probability plus the lowest-index maximum class.
Saved label bytes are emitted with reversible byte escapes. `Time` and realized `Device` logging is post-exit host
reporting and never adds a loop transfer. Structural GGUF conversion does not imply GGUF execution: each GGUF model
architecture requires an explicit Recipe semantic lowering, and an unrecognized architecture fails before native
preparation.

### C28: optional training horizon and graceful stop

`.epochs(count)` declares a finite nonzero number of full-partition optimizer updates. Omitting `.epochs(...)`
declares an unbounded training horizon: the calculation program, planned task domains, finalized bundle, executor,
and semantic `.ogdl` model all retain the literal state `unbounded`. They may not substitute `i32::MAX`, `u64::MAX`,
or another numeric sentinel.

An unbounded run requires a host graceful-stop source. Recipe's public terminal installs its SIGINT source before
native execution; lower execution entrypoints reject an unbounded program when no stop source is supplied. SIGINT is
observed only after a complete epoch. The executor then retains that epoch's recurrent parameter and AdamW state,
runs the ordinary `exit` lifecycle and ordered teardown, flushes the last completed live-metric row even when it is
off cadence, and performs only the independently declared `.save(...)` exports.

A finite endpoint is required for linear-to-zero, cosine, and exponential decay. For an omitted epoch bound, plain
`.lr(rate)` means optional finite warmup followed by the constant base rate; `.cos()` and `.exp()` fail before native
preparation because their endpoint is undefined. The device warmup coordinate saturates after the declared warmup
instead of using the absolute epoch as an indefinitely increasing int32 value. Post-training temperature scaling is
not a phase of an unbounded loop and therefore requires a finite epoch bound.

### C29: declared output topology

The final declared model block is the loss input. Its logical width must equal the task output width derived from the
saved target semantics: one for scalar regression and binary classification, and the complete fitted class count
including the reserved unseen-label route for categorical cross entropy. A mismatch is rejected before graph
construction. Recipe never inserts, trains, executes, or serializes an output projection that the model did not
declare.

A final pooling block is not an implicit output layer and is rejected. Binary and multiclass classification losses
consume raw logits, so the final declared layer or residual output must have no post-output activation or
normalization. Regression consumes the exact ordered operations the user declared on its final output. The semantic
model serializes that same visible topology. Older semantic models that explicitly contain a legacy output-adapter
record remain decodable for inference and resume compatibility, but compiling a new model never creates one.

### C30: channelwise one-dimensional convolution and pooling

Before the first structured spatial block, each prepared row has logical shape `[feature_width, 1]`. Structured values
use length-major, channel-minor order: logical coordinate `[row, position, channel]` is flattened only at the boundary
of an ordinary dense layer. `.conv(filters, kernel)` is a valid, stride-one one-dimensional convolution with no implicit
padding. For input `[rows, input_length, input_channels]`, it owns a weight tensor
`[kernel, input_channels, filters]` and bias `[filters]`, and emits
`[rows, input_length - kernel + 1, filters]`. A kernel wider than the current logical length is rejected before graph
construction. Repeated convolution blocks consume the preceding block's complete channel dimension; they are not
independent scalar convolutions over a reinterpreted flat width.

The forward contraction sums exactly the kernel and input-channel axes, adds the channel bias, then applies the
declared activation. Backward computes input, weight, bias, and learned-PReLU gradients over the complete training
partition. Input gradients use prepared checked coordinates and a validity mask followed by a deterministic
contraction; they do not require an atomic scatter whose accumulation order could vary. Validation repeats the same
saved logical geometry with updated parameters.

Channelwise maximum pooling partitions only the logical length axis into consecutive groups of the declared size,
including a bounded final short group, and preserves the channel count. Its backward route uses the saved lowest-index
winner for each group and channel. When `.pool(size).layer(neurons)` declares group-to-neuron routing, divisible counts
map contiguous equal group ranges to neurons; otherwise every neuron consumes every pooled group.

Semantic-model version 9 records each convolution declaration, resolved geometry, ordered activation identity,
parameters, and AdamW moments. Resume admits those images in canonical block order without changing the compiled
program. Target-free inference admits only saved parameter images and reconstructs the same checked windows and
logical shapes. Versions 5 through 8 remain decodable under their prior convolution-free topology contracts.

### C31: bounded ordered data distillation and preparation

`.data(sources)` preserves declaration order and `.set(source)` appends exactly one source. Each source may be a regular
file, recursively traversed directory, or recursively nested ZIP. Directory and archive members use deterministic path
order; symbolic links, archive-root escapes, malformed containers, empty sources, and excessive nesting fail closed.
The configured byte, record, vector, and field limits apply to the aggregate declared source set and to expanded
members, so splitting one logical input across files or compression cannot bypass admission bounds.

Specialized structural readers cover delimited text, JSON, encoded images, GGUF, safetensors, XLSX, and PPTX. Other
UTF-8 files remain lossless text samples and other files remain lossless binary samples. Source context is retained as
ordinary typed vectors whenever multiple files or container members are combined. Format recognition never performs
host numeric model calculation or silently decodes opaque model tensors into f32 payloads.

Target and exclusion names resolve against the original ordered vector list. Row predicates execute before excluded
helper columns are removed; retained rows preserve source order, and the declared train fraction partitions only after
selection. Every declared target remains a distinct ordered semantic vector and partition-matrix column. Semantic
encoding is fitted only from the training partition and records reserved validation routes without refitting. Data
normalization is a model calculation over the complete prepared partition, not a host mutation performed by the
loader.

The dense multi-target objective contract is C34. Other model families may reject a multi-target declaration until
their own output semantics are defined, but no compiler may merge, reorder, drop, or otherwise reinterpret the
prepared target vectors.

### C32: deterministic K-means feature reduction

`.kmeans(clusters)` consumes the current row-major `[rows, features]` value and emits `[rows, clusters]`, with one
rooted L2 distance to each centroid. A fresh run initializes centroid `c` from training row `c mod rows`, preserving
prepared row order and cycling deterministically when clusters outnumber rows. Each training epoch performs exactly
one Lloyd transition over every training row: assignment chooses the minimum prior-centroid distance, an exact tie
chooses the lowest centroid index, each nonempty centroid becomes the mean of every assigned row, and an empty
centroid retains its prior value. The block's output for that epoch is recomputed against those updated centroids.

The centroid transition is loop-carried unsupervised state, not an optimizer parameter or a differentiable assignment.
Supervised backward treats the updated centroids as fixed and differentiates every emitted L2 distance with respect to
the block input. Validation and inference never update centroids; they compute distances against the final saved
centroid tensor. Counts are represented exactly in f32, so a partition exceeding 16,777,216 rows is rejected rather
than silently losing mean semantics; cluster counts outside `1..=i32::MAX` are likewise rejected.

When `.kmeans(clusters).layer(neurons)` is declared, equal widths use identity routing, divisible expansion or
contraction uses contiguous equal cluster ranges, and non-divisible widths use ordinary full connectivity. The same
mask applies to the dense weight, both AdamW moments, validation, checkpoint validation, resume, and inference.
Semantic-model version 10 records the declaration, input width, routing, and exact updated centroid tensor. Resume
admits that centroid tensor without inventing optimizer moments; a fresh run uses the deterministic initialization.

### C33: deterministic all-output KNN reduction

`.knn(neighbors)` is one standalone terminal model. Its immutable reference set is the exact prepared training
partition; it has no objective, optimizer, learning rate, epoch loop, gradient policy, iterative metrics, or native
training kernel. It emits one independently typed result for every declared target in declaration order. Numeric
targets emit one f32 uniform mean. Categorical, ordinal, temporal, text, image, and binary targets emit one int32 mode
whose saved dictionary decodes back to the exact semantic value. A missing target value excludes only that reference
from that target's reduction. The effective neighbor count for an output is `min(neighbors, known references)` and an
output with no known training reference is rejected.

All outputs share one finite-f32 rooted-L2 distance calculation over the saved typed scalar/one-hot feature lowering.
Declared data normalization is applied to query and reference features in that saved coordinate system. Ascending
distance selection is stable in reference order; an exact vote tie chooses the lowest saved class code. Mixed output
dtypes are preserved through native execution and public reporting. Post-reduction activations or normalizations are
invalid because they would reinterpret independent heterogeneous outputs.

The canonical `.ogdl` model records every reference feature bit, per-output value and known mask, exact decoder,
feature schema/lowering, normalization declaration, neighbor count, and reference order. A missing `.resume(...)`
model starts fresh. An existing compatible KNN model continues by appending the current training references after the
saved references; duplicates remain because observation multiplicity is statistical weight and source-row indexes are
not global identities. Saved rows retain tie precedence. Row-derived label dictionaries preserve all saved codes and
append previously unseen current labels. Neighbor count, normalization, topology, row-free vector schema, target order,
and feature lowering must match exactly or resume is rejected.

### C34: ordered dense multi-target objectives

For `k >= 2` declared numeric targets, one dense objective consumes one row-major `[rows, k]` target matrix whose
columns retain the user's declaration order. Every target must have a fixed int32 or binary32 source representation;
int32 values are admitted only when conversion to f32 is exact. The final declared model width must be exactly `k`.
If any declared target is missing on a row, that complete row is unsupervised for the objective; Recipe does not train
different heads from different row subsets.

BCE and focal treat the `k` coordinates as independent binary labels, require exact zero or one in every supervised
cell, and reduce pointwise loss over `supervised_rows * k`. Their validation loss, AUROC, AUPRC, Brier score,
calibration error, and threshold recall are computed independently per target and macro-averaged in declaration order;
accuracy compares the lowest-index row argmax. Metrics requiring both binary classes are unavailable unless every
target column's known validation rows contain both classes.

Cross entropy treats the declared targets as one joint class structure. Every supervised row must be exactly one-hot;
the row-wise stable softmax couples all `k` logits, loss is reduced per supervised row, and accuracy compares the
lowest-index argmax. MSE, MAE, and Huber treat the columns as one ordered regression vector and reduce pointwise loss
over `supervised_rows * k`; R2 is calculated over the flattened ordered matrix.

Semantic-model version 11 records the exact target source-index order, every source dtype, the objective family, and
the effective width. Duplicate target identities are invalid, and resume requires exact order, schema, objective, and
topology equality. Inference emits one `[rows, k]` f32 matrix: independent probabilities for BCE/focal, joint softmax
probabilities for cross entropy, or ordered values for regression. Public reporting names every column from the saved
target order instead of inventing positional labels.

### C35: deterministic terminal tree and forest models

`.lgbm(depth)`, `.cbst(depth)`, and `.xgbst(depth)` are terminal supervised model blocks, not differentiable blocks
that may be followed by another model block. Each standalone declaration contains exactly one tree.
`.forest(trees).lgbm(depth)`, `.forest(trees).cbst(depth)`, and `.forest(trees).xgbst(depth)` contain exactly the
declared nonzero number of trees; the nested call selects the construction family. The output width is exactly the
declared task width. Trees consume the prepared f32 feature matrix after semantic feature encoding, without inventing
a separate host categorical path.

Every candidate threshold is the supervised mean of its feature in the candidate node. Split gain is summed across
outputs using `left_sum^2 / (left_count + 1) + right_sum^2 / (right_count + 1) - parent_sum^2 / (parent_count + 1)`;
empty children are invalid, and deterministic maximum reduction order resolves exact gain ties. XGBoost construction
is level-wise with a separately selected node-local feature and threshold. CatBoost construction is symmetric: one
feature is selected for a complete depth and uses that feature's supervised global mean threshold. LightGBM
construction is leaf-wise best-first: each transition selects the one positive-gain node-feature pair across all
currently reachable internal leaves. It performs at most `min(2^depth - 1, supervised_rows - 1)` such transitions;
unfilled internal positions retain a finite dummy-left route so the persisted complete-tree layout remains executable.

A one-tree model uses the prepared training rows directly. A multi-tree forest gives each tree one deterministic
full-size bootstrap sample, with replacement, from a distinct Recipe Philox stream. Tree structures are built once in
`init` and are not rebuilt between epochs. Exact feature-threshold ties route left. Predictions sum leaf values in
tree order with scale `1.0`; forests are not averaged and have no hidden boosting rounds, learning-rate shrinkage, or
upstream-library defaults. Leaf values start at zero and are the only learned tree parameters. They are optimized
jointly under the declared loss and AdamW over each complete training partition, with ordinary saved first and second
moments.

Semantic-model version 12 records the family, exact tree count and depth, input/output widths, complete split-feature
and finite-threshold tensors, leaf parameters, and both AdamW moments. Resume requires exact declaration and schema
compatibility and restores both fixed structure and leaf state. Validation and inference traverse that saved structure
without rebuilding or resampling it.

### C36: first observed categorical Bayesian conditional

The first executable `.bayes(child, [parents])` case contains exactly one declaration. The child is the sole declared
target, every parent is a declared feature, and child and parents must all be dictionary-categorical vectors. Parent
order is the literal declaration order. Every prepared training row must contain known child and parent observations;
this case rejects latent nodes, missing training observations, additional conditionals, continuous distributions, and
generic loss or optimizer declarations rather than assigning them implied semantics.

The semantic `.ogdl` model stores the exact child/parent source identities, names, canonical byte-label dictionaries,
prepared training-row order, and raw int32 observation codes. It does not store a host-fitted probability or count
table. Each parent reserves one additional inference code after its known dictionary. The native inference graph packs
the ordered parent codes into a checked mixed-radix configuration, histograms every `(configuration, child-class)`
observation, gathers the query configuration's counts, and emits
`(observed_class_count + 1) / (observed_configuration_total + child_classes)` for every child class. This is fixed
Laplace-one smoothing.
The child output contains only its saved known classes; there is no reserved child prediction.

A missing or previously unseen query parent label uses that parent's same reserved route. If the resulting parent
configuration was not observed, every class count is zero and the posterior is uniform. Exact probability ties select
the lowest saved child code for reported class identity. Preparation has no iterative training run, training kernel,
objective, optimizer, learning rate, epoch, or training metric. A missing resume model starts fresh; a compatible
existing model appends current observations after saved observations, preserving multiplicity as evidence. Resume
rejects any child, parent, declaration-order, source-schema, or dictionary drift. Public inference executes the
histogram and posterior graph through the ordinary measured native lifecycle and returns one f32
`[query_rows, child_classes]` matrix.

### C37: fixed-token embedding and causal self-attention

The first executable `.embed(dimensions).vocab(vocabulary)` case treats the prepared feature-column order as one
fixed sequence axis per row. Every feature must retain exact int32 storage and be a token ID in `0..vocabulary`;
numeric normalization, tokenizer fitting, padding, and implicit sequence-boundary inference are invalid for this
case. Exactly one embedding block must be first. Its learned f32 table has shape `[vocabulary, dimensions]`, begins
from Recipe's deterministic Philox normal stream scaled by `sqrt(2 / dimensions)`, and gathers one vector per token.
The logical result is `[rows, sequence, dimensions]`; later non-sequence blocks consume its row-wise flattened view.
Backward propagation atomically accumulates repeated-token gradients into the one shared table.

The first executable `.attn(heads)` case is optional, appears exactly once immediately after that embedding, and
means causal multi-head self-attention. `heads` is nonzero and must divide `dimensions`; `head_dimension` is exactly
`dimensions / heads`. Recipe learns four bias-free `[dimensions, dimensions]` matrices in query, key, value, and
output order. Scores are `Q * K^T / sqrt(head_dimension)`. A query position observes exactly key positions less than
or equal to itself, and stable softmax is calculated independently for every row, head, and query. The concatenated
head context is projected by the output matrix and preserves `[rows, sequence, dimensions]`. There is no implied
position vector, padding mask, cross-attention input, configurable mask, bias, dropout, or second attention block.

Training performs the complete analytic backward pass through output projection, value aggregation, softmax,
scaled scores, all Q/K/V projections, and the embedding table, then applies the ordinary full-partition AdamW
transition. Validation replays the same fixed geometry. Semantic-model version 13 records sequence length, embedding
dimensions and vocabulary, head geometry, the table and four attention matrices, and every first and second moment.
Resume requires exact schema and topology compatibility. Target-free inference accepts the saved exact-int32 token
schema and executes the same causal graph with the saved parameters.

### C38: first scalar-sequence vanilla RNN

The first executable `.rnn(width)` case is exactly one leading vanilla recurrent block over ordinary prepared numeric
features. Each row is an independent fixed sequence; feature-column order is time order and each time step contains
one f32 scalar after the declared data normalization. Categorical expansion, embedding/attention input, a preceding
model block, a second recurrent block, missing sequence boundaries, and chained recurrent-block operations are not
silently reinterpreted as this case.

For hidden width `H`, Recipe owns an input weight `[1, H]`, recurrent weight `[H, H]`, and bias `[H]`. Every row starts
from exact `h_0 = 0` and applies `h_t = tanh(x_t W_x + h_(t-1) W_h + b)` in feature-column order. The same parameters
are shared by every step and row. Only `h_last` leaves the block as `[rows, H]`; no full-sequence output, external
initial state, final-state export, padding mask, stateful session, or state carry between rows, epochs, training runs,
or inference calls exists. Input and recurrent weights begin from distinct deterministic Philox normal streams scaled
by `sqrt(2 / fan_in)` and bias begins at exact zero.

Training unrolls the fixed sequence into the static graph and performs complete reverse-time differentiation through
every tanh and recurrent edge. Input-weight, recurrent-weight, and bias gradients sum across every step and supervised
row before the ordinary full-partition AdamW transition. Validation replays the same zero-state recurrence.
Semantic-model version 14 stores the sequence length, hidden width, all three parameters, and every first and second
moment. Resume requires exact row-free schema, sequence geometry, and block topology compatibility. Target-free
inference restores those saved parameters and returns only the final hidden state to downstream saved blocks.

### C39: first scalar-sequence reset-before GRU

The first executable `.gru(width)` case has the same row and sequence boundary as C38: exactly one leading recurrent
block consumes one normalized numeric scalar per feature column, starts each independent row from exact `h_0 = 0`,
and emits only `h_last`. A preceding model block, a second recurrent block, categorical expansion, full-sequence
output, padding, cross-row or cross-run state, and chained recurrent-block operations are not implied.

For hidden width `H`, Recipe owns separate reset, update, and candidate input weights `[1, H]`, recurrent weights
`[H, H]`, and biases `[H]`. Distinct deterministic Philox streams initialize every weight and every bias starts at
exact zero. Recipe uses the reset-before equations `r_t = sigmoid(x_t W_xr + h_(t-1) W_hr + b_r)`,
`z_t = sigmoid(x_t W_xz + h_(t-1) W_hz + b_z)`,
`n_t = tanh(x_t W_xn + (r_t * h_(t-1)) W_hn + b_n)`, and
`h_t = (1 - z_t) * n_t + z_t * h_(t-1)`. The update gate therefore retains the prior state when it approaches one.

Training unrolls that exact graph and performs complete reverse-time differentiation through the hidden blend,
candidate, reset product, and both sigmoid gates. All nine parameter gradients sum across every step and supervised
row before the ordinary full-partition AdamW transition. Validation replays the same equations. Semantic-model
version 15 stores sequence length, hidden width, all nine parameters, and every first and second moment. Resume
requires exact row-free schema, sequence geometry, and topology compatibility; target-free inference reloads those
images and reproduces the same final-state recurrence.

### C40: first scalar-sequence zero-cell LSTM

The first executable `.lstm(width)` case uses the same row and sequence boundary as C38 and C39: exactly one leading
recurrent block consumes one normalized numeric scalar per feature column, starts every independent row from exact
`h_0 = 0` and `c_0 = 0`, and emits only `h_last`. A preceding model block, a second recurrent block, categorical
expansion, full-sequence output, final-cell export, padding, cross-row or cross-run state, and chained recurrent-block
operations are not implied.

For hidden width `H`, Recipe owns separate input, forget, output, and candidate input weights `[1, H]`, recurrent
weights `[H, H]`, and biases `[H]`. Distinct deterministic Philox streams initialize every weight and every bias
starts at exact zero. Recipe uses `i_t = sigmoid(x_t W_xi + h_(t-1) W_hi + b_i)`,
`f_t = sigmoid(x_t W_xf + h_(t-1) W_hf + b_f)`,
`o_t = sigmoid(x_t W_xo + h_(t-1) W_ho + b_o)`,
`g_t = tanh(x_t W_xg + h_(t-1) W_hg + b_g)`,
`c_t = f_t * c_(t-1) + i_t * g_t`, and `h_t = o_t * tanh(c_t)`.

Training unrolls that exact graph and performs complete reverse-time differentiation through both hidden and cell
state, all four gates, and every recurrent edge. All twelve parameter gradients sum across every step and supervised
row before the ordinary full-partition AdamW transition. Validation replays the same zero-state equations.
Semantic-model version 16 stores sequence length, hidden width, all twelve parameters, and every first and second
moment. Resume requires exact row-free schema, sequence geometry, and topology compatibility; target-free inference
reloads those images and reproduces the same final-hidden-state recurrence.

### C41: first named GGUF llama execution instrument

The first executable `.load("model.gguf")` case is a bounded, little-endian GGUF-v3 model whose
`general.architecture` is exactly `llama`. It requires dense F32 tensors, no mixture-of-experts tensors, equal query
and key/value head counts, an even head dimension, and RoPE across the complete head. Quantized tensors, GQA,
partial rotary dimensions, and other GGUF architectures or incompatible llama variants are rejected before native
preparation rather than being interpreted through this graph.

The target-free input table must contain exactly one vector of whitespace-separated exact int32 token IDs. Its
ordered cells form one sequence, which must be nonempty, fit the model context, and contain only IDs in the saved
vocabulary. Recipe does not fit or invoke a tokenizer, infer padding, split the sequence, sample a token, or retain KV
state between calls. Evaluation returns the raw `[sequence, vocabulary]` f32 logits for every input position.

Each block executes saved RMSNorm, adjacent-pair RoPE, scaled causal multi-head self-attention, the residual update,
saved RMSNorm, parallel SwiGLU, and the second residual update. The final saved RMSNorm and output projection produce
the logits; absent output weight uses the token embedding table. Optional saved biases and multiplicative scales are
applied in their llama graph positions. RoPE uses the saved base and scaling metadata, and no user-facing model
geometry or execution controls are introduced. The hardware acceptance runner compiles the checked-in corpus through
the public load boundary and rejects native logits whose normalized mean-square error against the pinned llama.cpp
oracle is not below `1e-3`. Dated backend-specific observations belong in the acceptance record; compilation or a
result from another backend does not establish current hardware proof.

### C42: repeated observed categorical Bayesian targets

The next executable Bayesian instrument accepts one or more repeated `.bayes(child, [parents])` declarations. The
children must exactly equal `.target(...)` in the same order. Every child and parent is an observed
dictionary-categorical vector, every parent remains an inference feature, every conditional has at least one parent,
and every retained training row contains all observations. A feature parent may be shared by multiple conditionals
and is prepared only once for target-free inference.

Each declaration independently retains its ordered parent schemas, child dictionary, complete reference-row order,
and raw parent/child codes. Semantic-model version 1 remains the canonical singular image. Version 2 stores two or
more conditionals in repeated-call order. Resume requires the complete ordered set of schemas to match, then appends
saved-before-current observations to every conditional. It stores no host-fitted counts or probabilities.

Inference runs the C36 mixed-radix histogram and Laplace-one posterior graph independently for each conditional. The
result matrices are concatenated on the device into one f32 `[query_rows, sum(child_classes)]` matrix; adjacent class
ranges and terminal output records follow declaration order. Missing or unseen parent labels retain C36's reserved
route. A declared target child cannot be another conditional's inference parent in this instrument: ancestral
prediction, evidence propagation, or marginalization is not silently selected. Numeric distributions, custom priors,
latent nodes, incomplete training observations, and generic objectives require their own concrete declaration and
execution contracts. The cookbook's `bayes_multi` recipe completed the two-output path on native HSA on 2026-07-30.

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
| All-output KNN reduction | KNN reference, codec, resume, inference, and mixed-output validators |
| Ordered dense multi-target objectives | target-matrix, loss, validation, checkpoint-v11, resume-order, inference, and facade validators |
| Terminal tree and forest models | family split, deterministic segment reduction, checkpoint-v12, resume, inference, and facade validators |
| Observed categorical Bayesian conditional | schema, raw-observation codec, mixed-radix, histogram, posterior, resume, inference, and facade validators |
| Fixed-token embedding and causal attention | exact-int32 schema, gather/scatter, head geometry, causal softmax, full backward, checkpoint-v13, resume, inference, and facade validators |
| Scalar-sequence vanilla RNN | numeric scalar schema, fixed unroll, zero-state recurrence, reverse-time gradients, checkpoint-v14, resume, inference, and facade validators |
| Scalar-sequence reset-before GRU | numeric scalar schema, reset/update/candidate equations, full reverse-time gate gradients, checkpoint-v15, resume, inference, and facade validators |
| Scalar-sequence zero-cell LSTM | numeric scalar schema, input/forget/output/candidate equations, full hidden/cell reverse-time gradients, checkpoint-v16, resume, inference, and facade validators |
| Named GGUF llama execution | bounded model/token admission, exact tensor and metadata contracts, Recipe-owned graph lowering, all-position raw-logit output, public dispatch, and llama.cpp parity validator |
| Repeated observed categorical Bayesian targets | ordered target/declaration binding, v2 codec, exact multi-resume, shared-parent preparation, independent native posteriors, device probability packing, and public reporting validators |

Applicable claims are exercised through public end-to-end workflows over real datasets and real native hardware.
Static validation remains build hygiene; constructed values, mocks, and restated implementation details are not
accepted as architectural proof.

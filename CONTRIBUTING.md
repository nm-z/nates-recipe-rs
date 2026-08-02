# Contributing to Recipe

## Design Philosophy
Recipe uses measurements from the user's hardware, and from the script - derives an optimized GPU kernel ahead of execution. This kernel is immutable throught its `init -> loop -> exit` lifecycle.

All calculation is GPU-only, transfers are scheduled AOT, since primitives are
implemented by recipe using no vendored ML AMD/NVIDIA libs. Behavior and configuration is semantically evaluated. Recipe ensures invalid states are impossible, rather than enforce a strict runtime convention.

Kernel generation may lower Recipe-owned operations through either:
- the LLVM IR directly
- or through the MLIR 
Recipe retains its own internal semantic interpetations, cost modeling, scheduling, and AMD/NVIDIA parity.

Do not add inline, unit, mock, or synthetic-data tests: a Recipe test must execute the public workflow end to end on a real dataset and real CUDA or HSA hardware, proving observed correctness, performance, or an architectural invariant. Use Rust compilation and type checking for structural validity; only real training and inference runs count as evidence against logical errors.

## Data Semantics
Recipe treats all digitally stored, learnable data as vectors belonging to six
semantically detected types: 
- numeric
- temporal
- categorical
- ordinal
- text
- image
Every other apparent data type is a container, a structural
relationship, or a composition of these six. Do not introduce additional
semantic types to describe storage formats or data structures.

Every filesystem object is treated as a list of feature vectors. Recipe
automates loading by first attempting to parse each vector semantically as one
of the six types. When semantic parsing is ambiguous, a categorical encoding
model labels the vector from training examples that pair features with their
smallest lossless encoded datatype. This classification and encoding pipeline
identifies user-provided data types.

## Feature Generation Is Banned
Recipe will not implement, invent, or derive new features from data.
All feature engineering operations are considered to be one of the following:
- Feature generation (FG)
- Feature transformation (FT)
- Feature reduction (FR)
Users who wish to generate features, may do so externally, store them in their dataset, and import that dataset to Recipe as ordinary input.

Recipe may perform FT and FR.

The reasoning is intent: reduction and transformation operate on signal from the data supplied
features. Recipe will not implement features for the user to train on derived features.

## No Performance-Reducing Configuration
The system derives all execution parameters from the hardware
and the model. Do not add user-facing configuration APIs for any of the following:

.batch()             system knows VRAM, computes what fits
.tile_size()         system probes optimal tile per shape
.chunk_size()        same as tile, different name
.num_threads()       system reads available_parallelism
.num_streams()       system knows CU count and occupancy
.precision()         system reads model dtype, probes GPU rates
.cache_size()        system reads LDS/L2/IC from hardware
.prefetch_depth()    system determines from memory latency
.block_size()        system computes from tile and wave width
.grid_size()         derived from problem size and tile
.memory_limit()      system probes claimable_bytes
.context_length()    system reads from model metadata
.rope_base()         system reads from model metadata
.head_dim()          system reads from model metadata
.vocab_size()        system reads from model metadata

Do not add any parameter the system can derive from hardware
or model metadata.

Deriving an execution parameter does not mean subdividing work merely because
the backend has a subdivision mechanism. Recipe must first determine whether
the complete operation fits on the measured hardware. Batching and tiling are
fallbacks for operations that do not fit, not defaults applied to operations
that already fit. When subdivision is necessary, Recipe derives it; the user
does not configure it.

### Batching

Do not divide a dataset into smaller launches when the complete training set
fits. A failure that established this rule used:

```text
1168 rows x 80 columns x 4 bytes = about 370 KB
```

That dataset fit in VRAM roughly 32,000 times over. The system nevertheless
selected one row per launch, planned 1,168 launches instead of one for every
epoch, and expanded 100,000 epochs into 116,800,000 training iterations. The
production `RunJournal` then attempted to reserve 26,513,600,107 logical-event
slots, requiring 1.272 TB for a debug trace, and killed the program before
training began.

The user-facing `.batch()` API is banned independently of that failure. Asking
the user how many rows belong in a launch asks them to predict available VRAM
and the backend's realized memory requirements. Recipe measures and knows those
values; the user does not need to supply them.

### Tiling

Tiling is batching along another axis. Do not subdivide a matrix that fits in
the relevant measured LDS capacity. If the complete operation does not fit,
Recipe derives the necessary tiling from the realized shape and hardware
profile. The user never supplies tile, block, grid, or chunk sizes.

## Intermediate Checkpointing Is Banned

Do not save periodic intermediate copies of a model that is already live in
memory and still training. This does not prohibit the user from deliberately
saving the resulting model; it prohibits an automatic intermediate-checkpoint
mechanism and its configuration surface.

## Automatic Stopping Is Banned

Do not implement early stopping. Early stopping guesses that training should
end because a selected metric plateaued; the user can observe the training run
and stop it with Ctrl+C.

An explicit `.epochs(n)` declaration is valid and exact: training stops after
exactly `n` complete passes over the training set. It is optional. When
`.epochs(...)` is absent, Recipe trains indefinitely while reporting the live
loss, until the user stops the run with Ctrl+C. Recipe must not invent an
implicit epoch bound.

These rules have one shared philosophy: do not subdivide something that fits,
ask the user for something the system knows, save something that does not need
saving, or automatically stop something the user can stop themselves.

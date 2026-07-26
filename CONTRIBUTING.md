# Contributing to Recipe

## Design Philosophy
Recipe uses measurements from the user's hardware, and from the script - derives an optimized GPU kernel ahead of execution. This kernel is immutable throught its `init -> loop -> exit` lifecycle.

All calculation is GPU-only, transfers are scheduled AOT, since primitives are
implemented by recipe using no vendored ML AMD/NVIDIA libs. Behavior and configuration is semantically evaluated. Recipe ensures invalid states are impossible, rather than enforce a strict runtime convention.

Kernel generation may lower Recipe-owned operations through either:
- the LLVM IR directly
- or through the MLIR 
Recipe retains its own internal semantic interpetations, cost modeling, scheduling, and AMD/NVIDIA parity.

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

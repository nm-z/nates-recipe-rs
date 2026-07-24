# Contributing to Recipe

Keep this document deliberately thin. `system-contract.md` is normative;
`AGENTS.md` contains repository layout, commands, and testing guidance.

## Design Philosophy

Recipe measures real hardware, builds the complete computation-and-transfer DAG
ahead of execution, and runs an immutable `init -> loop -> exit` lifecycle.
Payload calculation is GPU-only, transfers are explicit, primitives are
Recipe-owned, and AMD/NVIDIA behavior must remain semantically equal. Prefer
designs that make invalid states impossible over runtime convention.

Kernel generation may lower Recipe-owned operations through LLVM IR directly
or through MLIR. MLIR is compiler infrastructure, not an operation library:
Recipe retains its own semantics, cost model, scheduling, and AMD/NVIDIA
parity.

## Data Semantics

Recipe treats all digitally stored, learnable data as vectors belonging to six
exhaustive semantic types: **numeric, temporal, categorical, ordinal, text, and
image**. Every other apparent data type is a container, a structural
relationship, or a composition of these six. Do not introduce additional
semantic types to describe storage formats or data structures.

Every filesystem object is treated as a list of feature vectors. Recipe
automates loading by first attempting to parse each vector semantically as one
of the six types. When semantic parsing is ambiguous, a categorical encoding
model labels the vector from training examples that pair features with their
smallest lossless encoded datatype. This classification and encoding pipeline
identifies user-provided data; it does not generate features.

## First Law: Feature Generation Is Banned

Recipe does not invent or derive new features from a user's data. Automated
feature engineering, feature crosses, polynomial features, lag or rolling
aggregates, and domain-derived signals are outside Recipe's scope.

Users may generate features externally, store them in their dataset, and bring
that dataset to Recipe as ordinary input.

Recipe may perform:

- **feature reduction**, including explicit selection, pruning, or dimensional
  reduction; and
- **feature transformation**, including explicit scaling, normalization,
  encoding, or mathematical transformation.

The boundary is intent: reduction and transformation operate on user-supplied
features under an explicit plan. Recipe must not search for, propose, or create
additional derived features.

## Contribution Standard

Keep changes narrow and preserve the system contract. Do not introduce HIP,
vendor math libraries, dynamic loop behavior, hidden CPU payload calculation,
or automatic feature generation. Add focused tests for changed behavior and
record any intentional contract change explicitly.

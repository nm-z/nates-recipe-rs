# Dependency-closure audit

## Intent and scope

`audit/src/dependency.rs` turns an explicitly supplied Cargo resolve graph into
dependency findings. It has two responsibilities:

1. Parse the subset of Cargo metadata needed to identify packages and directed
   package-to-package edges.
2. Validate and walk the closure of caller-selected root package IDs, applying
   the package-name policy and returning deterministic blocking findings.

The module is deliberately a fact consumer. It does not run Cargo, read a
manifest, inspect a package's source, inspect linker arguments, or inspect an
ELF. Those facts are collected by the CLI or by a build system and are passed
to the audit crate. Source, linker, and ELF policy are separate modules. A
package name is the only dependency-level policy input here. It does not read
or cross-check `Cargo.lock`; the supplied metadata document and exact roots are
authoritative for this audit invocation. The parser does not authenticate the
document against Cargo or verify manifest paths on disk, so metadata provenance
and target selection remain the caller's responsibility.

The implementation is in [`audit/src/dependency.rs`](../../src/dependency.rs).
The public types are re-exported by [`audit/src/lib.rs`](../../src/lib.rs),
while `DependencyGraph::audit` remains crate-private and is reached through
the top-level `recipe_audit::audit` function.
Within the current workspace, the executable parser call is in `audit/src/main.rs`
and the library call is in `audit/src/lib.rs`; no other crate calls this graph
walk. `recipe-kernel` has a separate narrow LLVM declaration check and points
to `recipe-audit` for dependency, linker, load, and final-ELF policy rather than
duplicating this implementation.

## Position in the real input path

The executable path is `recipe-audit` in
[`audit/src/main.rs`](../../src/main.rs):

1. `Cli::parse` accepts one `--metadata` path and one or more exact
   `--package-id` values. The option relationship is checked before this
   module is called: metadata requires at least one package ID, and a package
   ID without metadata is rejected. The CLI also rejects an empty package ID
   string before constructing the graph during normal execution (the help
   short-circuit returns before that check); repeated nonempty IDs are allowed
   and become repeated roots, which graph traversal later deduplicates.
2. `read_bounded_text` requires an absolute regular file no larger than
   64 MiB, then reads it as UTF-8. The public metadata parser has no file or
   byte limit of its own because it receives `&str`; it also has no explicit
   package, node, or edge count limit. The CLI's file bound is the only size
   guard on its normal path.
3. `DependencyGraph::from_cargo_metadata_json` parses the text and validates
   the resulting graph. On success, the graph is stored in
   `AuditInput.dependencies`. If `--metadata` is omitted, this field is
   `None` and no dependency audit runs.
4. `recipe_audit::audit` first validates the other `AuditInput` facts, audits
   sources, then calls `graph.audit()`, then audits linker inputs and ELF
   facts. A source or collection error that occurs earlier stops the call
   before the dependency graph is reached. A graph error stops the whole
   audit; any findings accumulated by earlier stages are local to that call and
   no partial report is returned. `AuditInput::validate` does not validate the
   optional dependency graph; that responsibility belongs to `graph.audit()`.
5. Dependency findings are merged with findings from the other modules,
   globally sorted and deduplicated, and then legacy grants are applied by the
   library. In `next` mode grants are not accepted. In `legacy` mode an exact
   grant can change a dependency finding from `blocking` to `grandfathered`.
   A dependency grant must use the normalized manifest path, category
   `dependency`, line `0`, and the exact package-name symbol; any other
   coordinates are unused and make the legacy configuration stale.
6. The CLI serializes the resulting `AuditReport` as pretty JSON. A report
   with any blocking finding exits with status 1. An error such as malformed
   metadata exits with status 2 after printing the error and usage text.

The graph itself is never serialized into the report. Package IDs, edges, and
roots are input-only state; the output exposes only the normalized finding
coordinates and their dispositions.

The dependency path therefore has two kinds of failure: malformed or
inconsistent evidence is an `AuditError`, while a valid graph containing a
reachable prohibited package is a normal report containing a blocking finding.

## Data model

### `DependencyPackage`

`DependencyPackage` is the minimum identity and display record for one Cargo
package:

| Field | Meaning | Use in the audit |
| --- | --- | --- |
| `id` | Cargo's exact package ID string | Node key and edge endpoint |
| `name` | Cargo package name | Input to `classify_dependency` |
| `manifest_path` | Manifest location | Finding path, with `\\` changed to `/` |

`DependencyPackage::new` converts all three inputs to `String` and normalizes
only the manifest path with `model::normalize_display_path`. It does not reject
empty values. Empty-value checks happen later in `DependencyGraph::validate`.
The constructor does not canonicalize, resolve, require absoluteness, or check
that the path exists. A direct graph can therefore report a relative or
synthetic manifest path, provided it is nonempty. The CLI's native `--scope`
containment checks do not apply to dependency manifest paths, which commonly
point into a registry outside that scope.

### `DependencyEdge`

`DependencyEdge` is one directed `package_id -> dependency_id` relation. The
direction is important: walking from a root follows the edge's
`dependency_id`, so the audit computes dependencies of the selected roots, not
packages that depend on them. `DependencyEdge::new` stores both IDs verbatim
and performs no validation or normalization. Package and edge IDs are compared
as exact strings: they are not trimmed, case-folded, path-normalized, or
resolved through Cargo's package names.

The derived ordering is package ID followed by dependency ID. This allows the
metadata parser to use a `BTreeSet` for edge construction and produce a sorted,
duplicate-free vector. A graph built directly with `DependencyGraph::new` does
not get that insertion-time deduplication; its later validation rejects a
duplicate edge.

### `DependencyGraph`

`DependencyGraph` owns three caller-visible vectors:

* `packages`: package records keyed by their `id` during validation.
* `edges`: directed package-to-dependency relations.
* `root_package_ids`: the exact package IDs whose transitive closure is in
  scope.

Root selection is by the full `id` string, not by package name, version, or
manifest path. For Cargo metadata this means callers must pass the complete
metadata ID, including its source or path portion when present.
The graph layer does not require a root to be a workspace member or the
metadata document's `resolve.root`; any package ID present in `packages` is a
valid explicit root.

`DependencyGraph::new` only stores these vectors. It deliberately permits
callers to construct an invalid graph so that `audit` can fail closed at the
same boundary used for parsed metadata. The type derives equality and debug
traits, but it has no serialization contract of its own.
Derived equality compares the vectors, including their order; two graphs with
the same logical closure but different package or direct-edge ordering need not
be `Eq`, even though the audit findings are normalized and deterministic.
Because the fields are public, callers can mutate a graph after construction;
there is no cached validity flag. Every call to `audit` revalidates the current
vectors.

The parser does not retain the intermediate `serde_json::Value`: required
strings are copied into owned package and edge fields before the value goes out
of scope. During auditing, the package map, adjacency lists, queue, and visited
set borrow IDs from the graph. Findings clone the manifest path and package
name, so the returned vector is independent of the graph and no graph mutation
occurs during classification. The parser also clones the caller's root slice
into owned `root_package_ids`; no caller buffer is retained.
Because audit builds only local maps, queues, sets, and finding vectors, calling
`audit` repeatedly on an unchanged valid graph is observationally idempotent.

### API visibility

`DependencyPackage`, `DependencyEdge`, and `DependencyGraph` are public types
re-exported from the crate root. Their constructors and the metadata parser are
public, so build tooling can inject either a hand-built graph or Cargo JSON.
`DependencyGraph::validate` and `DependencyGraph::audit` are private to the
audit crate. External callers must submit the graph through the public
`recipe_audit::audit` function, which supplies the report-level mode, grants,
and cross-source deduplication boundary.

## Cargo metadata parsing

`from_cargo_metadata_json` consumes a Cargo metadata document represented as a
`serde_json::Value`. It uses the caller's roots rather than discovering roots
from the document. Cargo's `resolve.root` field, if present, is ignored.
There is no explicit duplicate-JSON-key check: serde's object decoding leaves
the value selected by its normal map semantics before these field checks run.

The parser performs these steps in order:

1. Reject an empty `root_package_ids` slice with
   `InvalidCargoMetadata("at least one exact root package id is required")`.
2. Parse JSON. Any syntax or value-decoding error becomes
   `InvalidCargoMetadata` with serde's error text.
3. Require a top-level JSON object and an integer `version` whose
   `Value::as_u64` value is exactly `1`. A JSON string or decimal number such
   as `"1"` or `1.0` is not accepted. Other metadata versions are not
   accepted, and unknown top-level fields are ignored.
4. Require `packages` to be an array. Every element must be an object with
   nonempty string fields `id`, `name`, and `manifest_path`. Missing fields,
   nulls, non-string values, and empty strings are rejected by
   `required_string`. Unknown package fields are ignored. Each package is
   constructed immediately, so backslashes in its manifest path become
   forward slashes. `Nonempty` means only `String::is_empty` is checked;
   whitespace-only IDs, names, or paths are accepted and remain significant.
   Cargo naming grammar, source URL syntax, and path existence are not checked
   here. Any package-level dependency declarations are ignored; the resolve
   graph is the sole edge source.
5. Require a `resolve` object and a `resolve.nodes` array. The absence of this
   graph is rejected explicitly because metadata produced with `--no-deps`
   cannot prove the dependency closure. The parser does not require the nodes
   array to be nonempty, and it does not require a node for every package or
   even for a selected root. A hand-built document with an empty nodes array
   can therefore pass this shape check when its root is present in `packages`,
   yielding a graph whose reachable closure contains only that root. Cargo's
   current `--no-deps` output sets `resolve` to JSON `null`, so it is rejected
   at the object check; the general invariant still depends on the caller
   supplying populated resolve nodes for complete dependency evidence.
6. For each node, require an object, a nonempty string `id`, and a
   `dependencies` array. Resolve node IDs must be unique within the metadata
   document. Every dependency array element must itself be a nonempty string,
   interpreted as the target package ID. The parser does not read `dep_kinds`,
   features, target conditions, checksums, source URLs, or any other Cargo
   fields. Every listed edge is therefore audited equally; target or feature
   filtering must happen before metadata is supplied if a caller needs a
   narrower graph. Unread object fields at the resolve and node levels are
   ignored just like unread top-level and package fields. Dependency aliases
   are likewise irrelevant: classification uses the `name` from the target
   package record, not a manifest alias. Package target kind, feature set, and
   whether a dependency is used for build, host, or runtime purposes are not
   policy dimensions here. Cargo's parallel `deps` object array is not read;
   only the node's `dependencies` string array creates edges.
7. Insert each edge into a `BTreeSet`, then collect the set into the graph.
   Repeated identical edges in the JSON therefore collapse silently and are
   not reported as malformed metadata. The resulting edge vector is ordered by
   source ID and then target ID.
8. Construct the graph with the caller's root list and call `validate`. A
   validation error is propagated unchanged, so duplicate package IDs and
   missing edge endpoints surface as `InvalidDependencyGraph`, even though the
   parser's API documentation groups malformed metadata under the broader
   metadata description.

Parsing success is not an audit pass. The parser only returns a validated graph;
policy classification and finding creation happen later when that graph is
placed in `AuditInput.dependencies` and evaluated by `recipe_audit::audit`.

The parser checks the graph's references, but it does not require a one-to-one
set of package records and resolve nodes. A resolve node with no edges can be
absent from `packages` without being noticed unless it is also a selected root;
similarly, an isolated package record need not have a resolve node. Any node
that contributes an edge is checked because `validate` checks that edge's
source and target IDs exist. An edge may therefore point to a package record
that has no corresponding resolve node; that package is still reachable and
classified as a leaf. Conversely, an isolated resolve node with no package
record contributes no edge and is silently ignored.

## Graph validation and invariants

`DependencyGraph::validate` is called by the metadata parser and again by the
runtime audit. It returns a `BTreeMap<&str, &DependencyPackage>` keyed by
package ID, so later traversal can borrow the original package records without
copying them. Validation is ordered as follows:

1. At least one root ID is required.
2. Every package must have nonempty `id`, `name`, and `manifest_path` fields.
   Duplicate package IDs are rejected.
3. Every selected root ID must be present in the package map.
4. Every edge source and target must be present in the package map.
5. Every edge pair must be unique. A direct graph containing the same
   `package_id -> dependency_id` twice receives `InvalidDependencyGraph`.

Validation does not require normalized manifest paths, existing files,
acyclicity, a node for every package, a package for every isolated node, or
unique root IDs. A self-edge and a cycle are valid graph structure. Repeated
root IDs are harmless because traversal deduplicates visited IDs. The parser's
earlier `BTreeSet` means only the direct-construction path can reach the
duplicate-edge validation in normal use. Validation covers every supplied edge
before reachability is computed, so a dangling or duplicate edge in an
otherwise unreachable subgraph still fails the audit rather than being ignored.
The same global rule applies to package field emptiness and duplicate package
IDs; only a valid graph is eligible for root-scoped filtering.

Both the parser and `audit` use `?` at each check, so validation is fail-fast.
The first violated condition determines the error: an empty root slice is
reported before JSON is parsed, an empty package field is reported before a
duplicate package ID, and a missing edge source is reported before that edge's
missing target or duplicate status.
The module has no panic-based fallback for malformed graph data; all of these
shape and reference failures are returned as `AuditError` values.

## Auditing the reachable closure

`DependencyGraph::audit` first calls `validate`, so no graph traversal starts
from an unverified root or dangling edge. It then builds an adjacency map from
borrowed string slices:

* each edge appends its dependency ID under its package ID;
* each adjacency list is sorted and deduplicated;
* the map's `BTreeMap` keys and all sets provide stable ordering.

The roots seed a `VecDeque`. A breadth-first walk repeatedly removes one ID,
skips it if it is already in the `BTreeSet` of `visited` IDs, and otherwise
queues all of its outgoing dependency IDs. This is a transitive-closure walk,
not a depth-limited search. A cycle terminates because an ID is queued and
expanded at most once. A package that is not reachable from any selected root
is never classified, even if its name is prohibited. If the same package is
reachable through several paths, it produces at most one finding.

The roots themselves are part of the visited closure. A selected root whose
own package name is prohibited therefore produces a finding even when it has no
outgoing edges.

After the walk, each visited ID is looked up in the validated package map. The
package name is passed to `policy::classify_dependency`; only
`InterfaceClassification::Prohibited` survives the filter. Allowed and unknown
classifications intentionally produce no finding. Each prohibited package is
converted to:

Validation guarantees that every visited root or edge target has a package
record, so the source's `filter_map` does not drop a valid visited ID. It simply
projects the borrowed map lookup into the finding pipeline.

| Finding field | Value |
| --- | --- |
| `category` | `FindingCategory::Dependency` |
| `path` | package manifest path, slash-normalized again by `Finding::blocking` |
| `line` | `0`, because this is graph evidence rather than a text line |
| `symbol` | package name exactly as stored in the package record |
| `disposition` | `FindingDisposition::Blocking` |

The finding does not include the root ID, dependency chain, package ID, or
policy enum. Consumers receive the package manifest path and name only; they
must retain the input graph if they need to explain which root made a package
reachable.

The local findings are sorted and deduplicated before being returned. The
top-level audit performs the same operation after combining this vector with
source, linker, and ELF findings. Finding equality includes category, path,
line, symbol, and disposition, so the same prohibited package reached via
multiple edges is collapsed while distinct package IDs with distinct display
facts remain distinguishable. Distinct package IDs that normalize to the same
path and carry the same prohibited name can still collapse to one display
finding, because package ID is not part of `Finding`.

### Example closure

For roots `app` and edges `app -> clean`, `app -> rocblas-sys`, and
`rocblas-sys -> helper`, the visited set is `{app, clean, rocblas-sys,
helper}`. If `unused -> hip-sys` is present but `unused` is not reachable,
`hip-sys` is ignored. If `rocblas-sys` is the only prohibited reachable name,
the result is one blocking dependency finding at the `rocblas-sys` manifest,
with line `0` and symbol `rocblas-sys`.

## Dependency policy boundary

`policy::classify_dependency` lowercases the package name and replaces `_`
with `-`, then compares the normalized value against exact package families.
It does not trim whitespace and does not search arbitrary substrings.
The normalized spelling is used only for classification; the finding's
`symbol` remains the original package `name`, so an input such as `HIP_SYS`
is reported with that exact spelling.

The exact allowed names are `cuda-driver`, `cuda-driver-sys`, `hsa`,
`hsa-sys`, `rocr`, and `rocr-sys`. They return an allowed CUDA Driver or ROCr
classification and therefore do not create dependency findings.
Versions and registry or path sources embedded in a package ID do not affect
classification. Two reachable versions of the same prohibited package name
are each policy matches, subject to finding deduplication by their display
coordinates.

The prohibited families are:

| Family set | Prohibited interface |
| --- | --- |
| `hip`, `hipblas`, `hipcub`, `hipfft`, `hiprand`, `hiprtc`, `hipsolver`, `hipsparse` | HIP |
| `cudart`, `cuda-runtime` | CUDA Runtime |
| `hsakmt`, `kfd` | Direct KFD |
| `rocblaslt`, `rocblas`, `rocsolver`, `rocfft`, `miopen`, `rccl`, `cublas`, `cusolver`, `cufft`, `cudnn`, `nccl` | The corresponding vendor operation library |

For these families, `has_package_family` accepts only the bare family or one
of the exact suffixes `-sys`, `-src`, `-bindings`, `-runtime`,
`-runtime-sys`, `-static`, or `-wrapper`. A name that merely contains a family
substring, or uses another suffix, remains unknown. The exact allowed names
are checked first, so an allowed CUDA Driver or ROCr package is not turned into
a prohibited result by a broader family check.

Representative package-name outcomes are:

| Input name | Normalized policy result |
| --- | --- |
| `HIP_SYS` | prohibited HIP, while the finding symbol remains `HIP_SYS` |
| `rocblas_sys` | prohibited operation library |
| `cuda_driver_sys` | allowed CUDA Driver |
| `hippo` | unknown, because `-po` is not an accepted family suffix |
| `hsa-runtime64` | unknown at the package layer, despite the library spelling being allowed elsewhere |

The interface family returned by policy is not copied into the finding. Every
prohibited package, whether HIP, KFD, CUDA Runtime, or an operation library,
uses the single `dependency` finding category and the package name as its
symbol. In the policy enum, both `rocblaslt` and `rocblas` map to
`NativeInterface::RocBlas`; that implementation detail does not alter the
dependency finding.

This package-name policy is intentionally narrower than source and artifact
policy. For example, a neutral-name package that links `libcudart` is not
identified by this graph walk; supplied linker or ELF facts are responsible for
that evidence. Conversely, a reachable package named `rocblas-sys` blocks even
when this module has no information about which Cargo target or feature caused
it to resolve. Library-only spellings such as `amdhip64`, `cuda`, or
`hsa-runtime64` are not automatically prohibited as package names; they are
handled by linker and ELF policy unless a package family listed above matches.
The graph supplied by the caller is the scope of this layer.

## Failure and pass conditions

### Errors that prevent a report

The following conditions fail closed:

* empty caller roots, reported as `InvalidCargoMetadata` by the JSON parser and
  `InvalidDependencyGraph` by a direct graph audit;
* invalid JSON, a non-object top level, or a metadata version other than 1;
* missing or incorrectly typed `packages`, `resolve`, `resolve.nodes`, package
  objects, node objects, or node dependency arrays;
* missing, non-string, or empty package IDs, names, manifest paths, node IDs,
  or dependency IDs in metadata (`InvalidCargoMetadata`), plus empty package
  fields in a direct graph (`InvalidDependencyGraph`);
* duplicate resolve node IDs (`InvalidCargoMetadata`);
* duplicate package IDs, missing selected roots, missing edge endpoints, or
  duplicate direct-graph edges (`InvalidDependencyGraph`).

The module does not turn these failures into findings. They propagate through
`recipe_audit::audit` and the CLI emits an error instead of a JSON report.
At the executable boundary these errors are printed as
`recipe-audit: <displayed error>` followed by usage text and return status 2.
For example, a non-object top level reports `top-level value must be an
object`, a JSON `null` resolve value reports `resolve graph is required;
metadata cannot use --no-deps`, a duplicate package ID reports
`invalid dependency graph: duplicate package id ...`, and a dangling edge
reports `invalid dependency graph: edge target is absent: ...`.

### Valid reports

An empty reachable prohibited set is clean for this category. A reachable
prohibited package is not an execution error: it yields a blocking finding and
causes the combined report to fail unless an exact legacy grant later
grandfathers that finding. Allowed packages, unknown package names, unreachable
packages, and duplicate traversal paths do not produce dependency findings.

At the CLI boundary, the dependency contribution follows this status table:

| Dependency result | Combined report | Exit status |
| --- | --- | --- |
| Valid graph with no prohibited reachable package | `passed` can remain `true` | `0` if no other finding blocks |
| Valid graph with a prohibited reachable package | One or more `blocking` findings | `1` unless all are grandfathered |
| Valid graph with an exact legacy grant for each finding | Matching findings become `grandfathered` | `0` if no other finding blocks |
| Malformed metadata or invalid graph | No report is returned | `2` from `recipe-audit` |

The walk and output are deterministic for a given graph and root set. Sorted
adjacency, a visited set, ordered maps/sets, sorted findings, and final
deduplication prevent input edge order or graph cycles from changing the
result. The graph storage is linear in validated packages and edges. Traversal
builds adjacency from every validated edge, then touches each reachable vertex
and outgoing edge once, with logarithmic ordered-map/set operations used for
validation, visited tracking, and deterministic output.
Validation and traversal use ordered collections, so their dominant operations
are `O(log V)` map/set lookups plus adjacency and finding sorts; the graph walk
does not recurse and therefore does not consume call-stack space proportional
to dependency depth.
`Finding` derives ordering from category, path, line, symbol, and disposition;
because `Dependency` is the first `FindingCategory` variant, a combined report
sorts dependency findings before later source, linker, and artifact categories
when their other coordinates do not otherwise determine the order.

## Evidence and maintenance boundary

The dependency module's source of truth is the code, not a generated Cargo
lockfile or a policy summary. When changing this module, preserve the exact
caller-selected roots, fail-closed validation, directed closure semantics,
line-zero graph findings, and deterministic ordering. Changes to prohibited or
allowed package families belong in `audit/src/policy.rs`; changes to source or
artifact evidence belong in their respective modules. A build system that
needs dependency evidence should provide complete `--format-version 1`
metadata and exact root IDs, then consume the returned report rather than
recreating the walk or interpreting package names independently.
The normal Cargo producer command is `cargo metadata --format-version 1` with
the desired feature and platform selection; `--no-deps` produces no resolve
object and is rejected by the parser. The producer must persist that JSON at an
absolute path for the CLI and pass IDs copied exactly from its `packages` or
`resolve.root` values.

### Source map

| Region | Responsibility |
| --- | --- |
| `dependency.rs:7-59` | Public package, edge, and graph records plus constructors |
| `dependency.rs:61-146` | Cargo metadata parser and parser-to-graph validation handoff |
| `dependency.rs:148-193` | Reachability walk, policy filter, and finding projection |
| `dependency.rs:195-247` | Structural graph validation and error boundaries |
| `dependency.rs:249-253` | Required nonempty JSON string helper |
| `main.rs:35-53` | CLI metadata and exact-root option boundary |
| `lib.rs:42-79` | Combined audit ordering, deduplication, grants, and report creation |
| `model.rs:91-129,261-370` | Finding coordinates, `AuditInput.dependencies`, report pass state, and path normalization |
| `policy.rs:227-260` | Package-name normalization and family classification |

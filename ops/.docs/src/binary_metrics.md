# Binary classification metrics

## Ownership and boundary

ops/src/binary_metrics.rs materializes one GPU-only calculation-graph fragment for
binary-classification validation metrics. It owns the metric formulas, static
resource formula, tensor and threshold validation, contiguous identity allocation,
and graph validation. The public exports are re-exported by ops/src/lib.rs.

This module is a direct fragment API. It does not add an OperationDescriptor or a
symbol to OperationRegistry, and it does not dispatch through
materialize_composition. The only registry operations used by the in-tree
training caller are the separate accuracy operations described in Registry
relationship.

The authoritative implementation is ops/src/binary_metrics.rs. Source anchors
below use the current repository paths and line ranges so that this document can
be checked against the implementation.

## Public types

### RecallAtOutput

RecallAtOutput pairs one statically requested threshold with its caller-owned
output tensor:

~~~text
threshold_bits: u32
output: Tensor
~~~

RecallAtOutput::new(threshold, output) stores threshold.to_bits(). The threshold()
accessor reconstructs the exact f32 bit pattern. Threshold identity is therefore
bit-pattern identity, not a decimal-string identity.

### BinaryClassificationMetricRequest

Every request boundary is a caller-declared Tensor:

| Field | Required contract | Meaning |
| --- | --- | --- |
| probabilities | F32, rank one, shape [n] | Predicted probability for each example. |
| targets | F32, rank one, shape exactly equal to probabilities | Binary target values. Runtime validation requires each value to be finite and exactly 0 or 1. |
| per_element_bce | F32, rank one, shape exactly equal to probabilities | Existing nonnegative per-example loss vector. It is not recomputed from logits. The training Focal path supplies focal losses in this slot even though the field retains the historical name. |
| mean_bce | F32, shape [1] | Mean of the supplied per-example loss vector. |
| auroc | F32, shape [1] | Tie-aware area under the receiver-operating-characteristic curve. |
| auprc | F32, shape [1] | Tie-aware, non-interpolated average precision. |
| brier_score | F32, shape [1] | Mean squared probability error. |
| expected_calibration_error | F32, shape [1] | Equal-width calibration error over [0, 1]. |
| recall_at | Each output F32, shape [1]; thresholds are finite and in [0, 1] | One output for each static threshold. |
| calibration_bins | 1..=256 | Number of equal-width calibration bins. |
| tree_lanes | Power of two in 1..=1024 | Fixed reduction and scan tree width. |
| identity_namespace | Caller-reserved value and kernel ranges | Identity source for all intermediates and kernels emitted by this fragment. |
| workspace_limit | ByteCount | Maximum intermediate storage allowed by the request. |

All input and output tensor IDs must be distinct, including every recall output.
Boundary flags supplied by the caller are normalized for this fragment: the three
inputs are marked external_input, and all metric outputs are marked
external_output (binary_metrics.rs:151-155, 556-567). The caller remains the
owner of the final assembled graph boundary flags.

### BinaryMetricRequirements

binary_metric_requirements(n, r, b) returns the exact reservation needed for a
population of n examples, r recall thresholds, and b calibration bins:

~~~text
intermediate_values = 24 + 2*r + 5*b
kernels            = 24 + 3*r + 4*b
vector_bytes       = n * 4                         # F32 or I32 payload width
workspace_bytes    = (19 * vector_bytes + 20)
                     + r * (vector_bytes + 4)
                     + b * (2 * vector_bytes + 12)
~~~

Every addition and multiplication is checked. The function validates dimensions
before doing arithmetic (binary_metrics.rs:79-149).

### BinaryMetricMaterialization

Successful materialization returns:

~~~text
graph: CalculationGraph
intermediate_values: Vec<ValueId>
kernels: Vec<KernelTemplateId>
workspace_bytes: ByteCount
identity_namespace: IdentityNamespace
~~~

graph contains the normalized boundary declarations, all contiguous
intermediate tensors, and every primitive node. The two ID vectors are the exact
IDs emitted by the fragment in allocation order.

## Registry relationship

The canonical registry still contains the source-qualified legacy operations
gpu_accuracy_into and gpu_argmax_accuracy_into. Their concrete lowering is
owned by ops/src/materialize/loss_metrics.rs and their recipes are listed in
ops/src/composition.rs:

| Registry symbol | Used by binary validation for | Semantics |
| --- | --- | --- |
| gpu_accuracy_into | Single binary target | Threshold predictions and targets at 0.5, count matches with a fixed reduction tree, then divide by the checked element count. |
| gpu_argmax_accuracy_into | Multi-target binary output | Select prediction and dense-target row argmaxes, compare class indices, sum matches, and divide by row count. |

The custom fragment itself emits Elementwise, Reduce, Scan, Sort, Gather,
IndexMap, and Histogram primitives directly. There is no registry symbol for
AUROC, AUPRC, Brier score, ECE, or recall-at in this module. This separation is
intentional: registry accuracy is one operation in the caller, while this module
emits the complete multi-metric graph fragment.

## Static limits and reservation

The module publishes these limits (binary_metrics.rs:12-16):

~~~text
MAX_EXACT_BINARY_EXAMPLES = 9_999_999
MAX_CALIBRATION_BINS     = 256
MAX_RECALL_THRESHOLDS    = 256
~~~

MAX_EXACT_BINARY_EXAMPLES is the population ceiling chosen for exact integer
counts represented in binary32 while retaining the repository's fewer-than-seven-
significant-figures contract. A request with n == 0, n > 9_999_999, more than
256 thresholds, or zero or more than 256 bins fails with
UnsupportedConcreteShape.

The formulas count only fragment-owned resources. Boundary tensors and their
caller-owned IDs are not included. The base fragment has 19 vector-sized
intermediates and five [1] scalar intermediates, giving
19 * (4*n) + 20 bytes. Each recall adds one vector and one scalar. Each
calibration bin adds two vectors and three scalars: confidence members, target
members, confidence sum, target sum, and one bin contribution.

The base kernel count includes the final fixed scalar sum for ECE. The base
pipeline before that final sum emits 23 kernels, each recall emits three kernels,
and each bin emits four kernels. The final ECE sum has one kernel regardless of
the number of bins.

Before allocating identities or intermediates, materialize_binary_classification_metrics
checks that the requirements fit both identity_namespace.value_capacity() and
identity_namespace.kernel_capacity(), and that the exact workspace fits
workspace_limit (binary_metrics.rs:156-190). A capacity may be larger than the
formula; MetricEmitter::finish still requires the exact emitted counts and exact
workspace total.

## Request validation

Validation is performed by validate_request before any graph node is emitted
(binary_metrics.rs:698-780). The checks are ordered as follows:

1. Every input and output tensor passes Tensor::validate. Invalid layouts,
   storage spans, or byte sizes become InvalidMaterializationRequest.
2. probabilities, targets, and per_element_bce must all be F32 rank-one
   vectors with identical Shape values. Their layouts may differ, but each
   tensor's own layout and storage contract must be valid.
3. The shared element count n must satisfy the static population limit.
4. tree_lanes must be a nonzero power of two no greater than 1024.
5. The five fixed outputs and every recall_at output must be F32 with exact
   shape [1].
6. Every recall threshold must be finite and numerically in [0, 1]. A second
   threshold with the same u32 bit pattern is rejected. Distinct +0.0 and
   -0.0 bit patterns are therefore distinct requests even though their numeric
   comparisons are equal.
7. No boundary tensor ID may repeat across inputs, fixed outputs, and recall
   outputs.

The runtime values are validated in the first graph node, not on the host. The
validated_inputs_program requires each probability to be finite and in [0, 1],
each target to be finite and exactly 0 or 1, and each supplied loss to be finite
and nonnegative. It then selects each value from itself so that the Require
expressions remain live in the graph (binary_metrics.rs:940-1018). There is no
clipping, replacement, or fallback for invalid device data. A failed Require is
an execution-time failure, not an OperationError returned during materialization.

## Emitted graph

MetricEmitter allocates value IDs contiguously from identity_namespace.first_value()
and kernel IDs contiguously from identity_namespace.first_kernel(). Its half-open
ranges are checked for u64 overflow. Every intermediate is a contiguous tensor
with external_input == false and external_output == false; every emitted primitive
has an AliasPermission::Forbidden rule for every input/output pair
(binary_metrics.rs:519-645, 1365-1377).

The fixed base sequence is:

| Step | Primitive | Inputs and outputs | Purpose |
| ---: | --- | --- | --- |
| 1 | Elementwise | probabilities, targets, per_element_bce -> valid_probabilities, valid_targets, valid_losses | Device-side domain checks described above. |
| 2 | Reduce(Sum, axis 0) | valid_losses -> loss_sum | Sum supplied losses. |
| 3 | Elementwise | loss_sum -> mean_bce | Divide by n. |
| 4 | Elementwise | valid_probabilities, valid_targets -> brier_elements | Compute (p - y)^2 per example. |
| 5 | Reduce(Sum, axis 0) | brier_elements -> brier_sum | Sum squared errors. |
| 6 | Elementwise | brier_sum -> brier_score | Divide by n. |
| 7 | Reduce(Sum, axis 0) | valid_targets -> positive_count | Count positive targets in F32. |
| 8 | Sort(Descending, stable, emit_indices) | valid_probabilities -> sorted_probabilities, sorted_indices | Stable descending score order with I32 source indices. |
| 9 | Gather(Reject, axis 0) | valid_targets, sorted_indices -> sorted_targets | Reorder targets into score order. |
| 10 | IndexMap(start=0, element_step=1) | no inputs -> positions | Emit positions 0..n-1. |
| 11 | IndexMap(start=-1, element_step=1) | no inputs -> previous_indices | Emit predecessor indices. |
| 12 | Gather(Clamp, axis 0) | sorted_probabilities, previous_indices -> previous_probabilities | Clamp the predecessor of position zero to index zero. |
| 13 | Elementwise | sorted_probabilities, previous_probabilities, positions -> group_starts | Mark position zero or a score change. |
| 14 | Scan(Sum, Inclusive, axis 0) | group_starts -> one_based_group_ids | Prefix-sum group starts. |
| 15 | Elementwise | one_based_group_ids -> group_ids | Convert to zero-based group IDs. |
| 16 | Histogram(unweighted, SC) | group_ids -> group_counts | Count examples per score group. |
| 17 | Histogram(weighted, SC) | group_ids, sorted_targets -> group_positives | Count positives per score group. |
| 18 | Scan(Sum, Inclusive, axis 0) | group_counts -> cumulative_counts | Prefix-sum group counts. |
| 19 | Scan(Sum, Inclusive, axis 0) | group_positives -> cumulative_positives | Prefix-sum group positives. |
| 20 | Elementwise | five group/count inputs -> auroc_contributions, auprc_contributions | Compute tie-aware rank credit and precision contributions. |
| 21 | Reduce(Sum, axis 0) | auroc_contributions -> auroc_numerator | Sum AUROC numerator. |
| 22 | Reduce(Sum, axis 0) | auprc_contributions -> auprc_numerator | Sum AUPRC numerator. |
| 23 | Elementwise | positive_count, auroc_numerator, auprc_numerator -> auroc, auprc | Apply class-count normalizations and class-presence guards. |

All reductions and scans use the request's tree_lanes. Histograms use
AtomicOrdering::SequentiallyConsistent. The stable sort plus grouped histograms
avoids an O(n^2) pair matrix while preserving exact score-tie semantics.

For each requested threshold, three additional nodes are emitted:

~~~text
Elementwise(valid_probabilities, valid_targets -> hits)
Reduce(Sum, hits -> true_positives)
Elementwise(true_positives, positive_count -> recall_output)
~~~

For each calibration bin b, with B = calibration_bins, four nodes are emitted:

~~~text
Elementwise(valid_probabilities, valid_targets -> confidence_members, target_members)
Reduce(Sum, confidence_members -> confidence_sum)
Reduce(Sum, target_members -> target_sum)
Elementwise(confidence_sum, target_sum -> contribution)
~~~

The final node is one Elementwise fixed scalar sum over the B contributions.
fixed_sum_program combines inputs pairwise by levels, carrying an unpaired
input to the next level. B >= 1 is guaranteed by dimension validation, so its
zero-input error is unreachable from a valid request.

MetricEmitter::finish checks the exact intermediate count, kernel count, and
workspace total against BinaryMetricRequirements, constructs the graph, and calls
CalculationGraph::validate. Duplicate producers, missing producers, unknown
tensors, invalid primitive shapes, cycles, or invalid alias matrices are
reported as GraphMaterializationFailed (binary_metrics.rs:647-695).
## Metric definitions

Let p_i be validated probabilities, y_i validated binary targets, l_i the
validated supplied loss, and n the population.

### Mean supplied loss

~~~text
mean_bce = (sum_i l_i) / n
~~~

The graph never reads logits and never recomputes BCE. In the training Focal path,
the same output is the mean of the focal loss vector passed as per_element_bce.

### Brier score

~~~text
brier_score = (sum_i (p_i - y_i)^2) / n
~~~

### AUROC

After stable descending sort, equal adjacent probabilities form one score group.
For group g, let:

~~~text
c_g = group count
q_g = group positive count
C_g = cumulative count through g
P_g = cumulative positives through g
P   = total positives
N   = n - P
~~~

The per-group program computes:

~~~text
group_negatives       = c_g - q_g
cumulative_negatives  = C_g - P_g
negatives_below       = N - cumulative_negatives
half_ties             = 0.5 * group_negatives
rank_credit           = negatives_below + half_ties
auroc_contribution    = q_g * rank_credit
~~~

The final value is:

~~~text
auroc = sum_g auroc_contribution / (P * N)
~~~

This is the tie-aware Mann-Whitney rank definition: a positive receives half
credit against negatives with the same score and full credit against lower-score
negatives. It requires both P > 0 and N > 0; the normalization program uses
Require for that condition.

### AUPRC

Using the same score groups:

~~~text
precision_g          = P_g / C_g
auprc_contribution   = q_g * precision_g
auprc                = sum_g auprc_contribution / P
~~~

The result is non-interpolated average precision with ties grouped before the
precision step. The same normalization program requires P > 0, and it also
requires N > 0 because AUROC and AUPRC are normalized together in one kernel.

### Recall at a static threshold

For threshold t:

~~~text
predicted_i = 1[p_i >= t]
true_positive_i = y_i * predicted_i
recall_at(t) = sum_i true_positive_i / P
~~~

The threshold comparison is inclusive. normalize_positive_count_program uses
Require(P > 0). Recall thresholds are compiled into scalar programs and cannot
be supplied dynamically at execution time.

### Expected calibration error

Bin b covers [b/B, (b+1)/B) except that the final bin covers
[(B-1)/B, 1]. For each bin:

~~~text
confidence_sum_b = sum_{i in b} p_i
target_sum_b     = sum_{i in b} y_i
contribution_b   = abs(confidence_sum_b - target_sum_b) / n
ECE              = sum_b contribution_b
~~~

The implementation uses equal-width boundaries computed as f32 constants,
lower-inclusive membership, upper-exclusive membership for non-final bins, and
upper-inclusive membership for the final bin. Empty bins contribute zero without
special handling.

## Standalone materialization

materialize_binary_classification_metrics(request) follows this sequence:

1. Validate all boundary contracts and dynamic-domain declarations.
2. Compute the exact requirements for the request's n, recall count, and bin
   count.
3. Reject insufficient value or kernel capacities and insufficient workspace.
4. Build normalized boundary declarations and check that none lies in the
   reserved intermediate value range.
5. Emit the fixed, recall, and calibration stages above.
6. Verify exact emitted counts and bytes, then validate the complete graph.

The returned graph is self-contained: its three inputs are external inputs, its
fixed and recall outputs are external outputs, and every intermediate is produced
by exactly one node. A caller that assembles several fragments must reconcile
their temporary boundary flags with the final graph boundary sets, as
CalculationGraph::assemble specifies.

## Appending to an existing graph

append_binary_classification_metrics(graph, request) is the graph-assembly
helper (binary_metrics.rs:442-517). It preserves caller-owned boundary tensor
declarations and appends only fragment intermediates plus all fragment nodes.
Its checks are:

1. The caller tensor list has no repeated IDs.
2. Every request boundary tensor is already declared by the caller graph.
3. For every boundary ID, id, dtype, shape, layout, and storage bytes exactly
   match. External flags are deliberately ignored.
4. No emitted intermediate ID is already a caller tensor ID.
5. No emitted kernel ID is already a caller kernel ID.
6. No existing caller node already produces any requested metric output ID.

Boundary absence or a storage mismatch is InvalidMaterializationRequest.
Intermediate or kernel collision is IdentityNamespaceOverlap. An existing
producer for a requested output is GraphMaterializationFailed. The helper does
not validate the entire caller graph after extension; the caller must run
CalculationGraph::validate after all independently materialized fragments have
been appended.

## Training callers and observed outputs

### Public training configuration

src/training.rs::binary_validation_config creates a BinaryValidationConfig when a
binary training declaration requests AUROC, AUPRC, Brier, calibration error, or
binary accuracy. The public facade uses 15 calibration bins and no recall
thresholds by default. Binary validation is only accepted with BCE or Focal loss.
The public logging names map as follows:

| Training metric kind | Presentation label |
| --- | --- |
| ValidationMeanBce | validation_loss |
| Accuracy | accuracy |
| AuRoc | auroc |
| AuPrc | auprc |
| BrierScore | brier |
| ExpectedCalibrationError | calibration_error |
| RecallAt { threshold_bits } | recall@<threshold> |

Recall-at values are supported by the compiled training model type, but the
facade helper above constructs an empty threshold list unless a lower-level
caller supplies thresholds directly.

### Single-target validation path

training/src/compile.rs::compile_validation performs validation on
IterationDomain::every(bounds.training_iterations), so one immutable graph is
evaluated at every training iteration in the validation phase. It:

1. Compiles validation blocks to logits.
2. Applies the owned sigmoid operation to get probabilities.
3. Emits BCE-with-logits or the canonical focal program to obtain the per-element
   loss vector.
4. Extracts the sole matrix column into rank-one probability, target, and loss
   tensors.
5. Calls materialize_binary_metrics, which creates scalar output tensors,
   reserves exactly binary_metric_requirements, builds a
   BinaryClassificationMetricRequest, and inserts the returned graph.
6. Separately materializes registry operation gpu_accuracy_into with tree_lanes;
   this is the accuracy member of BinaryMetricOutputs and is not emitted by the
   custom fragment.

BinaryMetricOutputs contains mean_bce, accuracy, auroc, auprc, brier_score,
expected_calibration_error, and one RecallMetricOutput per requested threshold.
BinaryValidationOutputs additionally retains validation logits, the metric domain,
and optional post-training temperature-scaling state.

### Multi-target validation path

For a MultiTargetBinaryClassification task,
materialize_multi_target_binary_metrics extracts every matrix column with an
index-map, gather, and row reduction. It materializes one complete binary metric
fragment per column, then computes the arithmetic mean across columns for mean
loss, AUROC, AUPRC, Brier, ECE, and each matching recall threshold. Accuracy is
instead emitted once through registry operation gpu_argmax_accuracy_into over
the full probability and dense-target matrices. Temperature scaling is rejected
for multiple binary target columns.

### Validation availability and one-class partitions

Before graph construction, validate_validation_config requires a nonempty
prepared validation partition, a binary task, BCE or Focal loss, and valid
threshold bits. When known validation rows exist it also calls the binary metric
resource formula. binary_validation_metric_status then requires every target
column to contain at least one 0 and one 1. If known validation targets are
absent or a column has a single class, the status is Unavailable; validation
inputs and metric graph are omitted rather than materialized. This
prevents the shared AUROC/AUPRC normalization guards from being reached with an
undefined class denominator. The standalone ops API does not preflight class
composition, so a direct one-class request materializes but fails at the runtime
Require guard.

The resulting metric values are registered by training_metric_bindings as
TrainingMetricBinding entries on the validation domain. The executor reads those
[1] F32 values through the normal metric observation path; they are not
host-computed summaries.

## Failure matrix

| Failure kind | Materialization cause |
| --- | --- |
| InvalidMaterializationRequest | Invalid tensor layout/storage, wrong input or output dtype, duplicate boundary ID, invalid tree lanes, invalid threshold, missing or mismatched append boundary, or a caller storage contract mismatch. |
| UnsupportedConcreteShape | Empty or over-limit population, rank other than one for an input, unequal input shapes, scalar output not exactly [1], too many recall thresholds, or calibration bins outside 1..=256. |
| IdentityNamespaceOverlap | A boundary ID falls inside the reserved intermediate range, or append finds a reused intermediate or kernel ID. |
| IdentityNamespaceExhausted | Reserved value or kernel capacity is smaller than the exact requirement, or a namespace range end overflows u64. |
| WorkspaceLimitExceeded | Exact formula exceeds request.workspace_limit. |
| WorkspaceArithmeticOverflow | Checked resource arithmetic, vector byte calculation, or incremental workspace sum overflows u64. |
| WorkspaceFormulaMismatch | The emitted contiguous tensor bytes differ from the static formula. This is an internal graph/materializer invariant failure. |
| GraphMaterializationFailed | Scalar-program construction, shape construction, primitive emission, duplicate caller tensor, existing output producer, or final CalculationGraph::validate failure. |

The direct metric functions construct OperationError values without attaching a
registry OperationId. Runtime Require failures occur after graph construction
and therefore are observed by execution rather than returned in this table.

## Invariants to preserve

The following properties are part of the binary metric contract:

- Inputs are validated on device and never silently repaired.
- All payload calculations use canonical F32 and I32 tensors and Recipe
  primitives. No host metric loop or separate metric implementation exists.
- Every metric output is a caller-owned [1] F32 tensor. Recall thresholds are
  static and bit-preserved.
- Sorts are stable and descending; equal scores are grouped before AUROC and
  AUPRC contributions are computed.
- Reduction and scan widths are explicit, bounded, and power-of-two.
- Histograms use sequentially consistent atomics, and every kernel forbids
  input/output aliasing.
- Resource reservations are exact, checked before graph allocation, and verified
  again after emission.
- Independently materialized fragments must use disjoint intermediate and kernel
  namespaces and must be validated after assembly.
- Training validation evaluates the immutable graph on its declared validation
  domain, then exposes the resulting scalar values through metric bindings.

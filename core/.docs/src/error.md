# Validation errors

`core/src/error.rs` is the shared failure vocabulary for the immutable Recipe
model.  It does not perform validation itself.  The public model types in
`core/src/artifact.rs`, `discovery.rs`, `plan.rs`, `scalar.rs`,
`schedule.rs`, and `topology.rs` construct `ValidationError` values through
the crate-private `Validator`, then return them as `ValidationResult`.

The vocabulary is deliberately separate from driver, parser, planner, and CLI
errors.  A core validation failure says that a value crossing a static
pipeline boundary does not satisfy its declared contract.  It is therefore a
rejection of the input or intermediate product, not an execution status and
not a request to recover with another value.

## Structure and accumulation

### `ValidationCode`

`ValidationCode` is a `Copy`, ordered, hashable, non-exhaustive enum.  Its 66
members are the machine-readable categories listed in the reference below.
The enum is public from `recipe_core` (`core/src/lib.rs`), while no other
workspace crate currently matches a code directly.  Consumers normally keep
the complete error text and classify the outer error they own.

Two members are presently declarations only:

* `EmptyName` has no `ValidationCode::EmptyName` construction in the
  workspace.  Name parsing uses `LabelError` instead.
* `InvalidReservationName` has no construction in the workspace.  Native
  preparation reports a bad generated `Label` as its own
  `NativePrepareError::InvalidConfiguration`.

The `#[non_exhaustive]` attribute means a downstream match must retain a
wildcard for future categories.

### `ValidationError`

`ValidationError` has three public fields:

* `code: ValidationCode`, the stable category;
* `path: String`, the dotted location within the value being checked; and
* `message: String`, the precise contract statement and observed values.

`ValidationError::new` accepts any `Into<String>` for the path and message.
All current core construction reaches it through `Validator::require` or
`Validator::error`; there are no direct call sites elsewhere in the
workspace.

### `ValidationErrors`

`ValidationErrors` owns a private `Vec<ValidationError>`.  It is `Clone`,
`Debug`, `PartialEq`, and `Eq`, and implements `std::error::Error`.

* `from_error` wraps one error.  It is an available constructor, but no
  workspace call site currently uses it.
* `as_slice` borrows the ordered errors, `into_vec` consumes the collection,
  and `contains` checks whether one code is present.  These helpers are part
  of the public API; the current workspace uses `into_vec` internally from
  `Validator::append`, while external paths generally format the collection.
* `Display` preserves validation order.  Each item is rendered as
  `` `{:?} at {path}: {message}` `` and adjacent items are joined with
  `"; "`.  The `Debug` rendering of `ValidationCode` is therefore the
  machine-readable name visible in user-facing text.

### `ValidationResult`

`ValidationResult<T = ()>` is `Result<T, ValidationErrors>`.  The default
`()` makes all `validate` methods read as a contract check, while constructors
such as `FinalizedBundle::finalize_with_loop_schedule` return a value on
success.

### `Validator`

`Validator` is crate-private and owns the mutable error vector for one pass.

* `require(condition, code, path, message)` appends one error only when the
  condition is false.
* `error(code, path, message)` appends unconditionally.
* `append(prefix, errors)` consumes a nested `ValidationErrors` and prefixes
  every child path.  An empty child path becomes exactly `prefix`; otherwise
  the result is `prefix.child`.
* `finish` returns `Ok(())` only for an empty vector, otherwise an
  `Err(ValidationErrors)` containing every appended error in order.
* `into_errors` has the same empty/nonempty split but returns
  `Option<ValidationErrors>`.  Finalization uses it after collecting several
  independent checks before deciding whether to construct the immutable
  bundle.

The validators continue through independent checks.  A missing lookup can
cause a dependent check to be skipped, but it does not erase the original
error or introduce a substitute object.  Nested failures are accumulated by
`append`, so a caller receives the complete observed set for that pass.

### Construction index

This is the source-level index of every core constructor.  The reference table
below explains each code's contract; this index records which validator owns
the construction.

| Core entry point | Checks that construct `ValidationError` values |
| --- | --- |
| `ArtifactBuildRecipe::validate` (`core/src/artifact.rs:114-211`) | `InvalidIdentity`, `MissingRequiredObject`, `ResourceMismatch`, `DuplicateId` |
| `ArtifactIdentity::validate` (`core/src/artifact.rs:229-265`) | `InvalidIdentity`, `MissingRequiredObject` |
| `Topology::validate` (`core/src/topology.rs:144-417`) | `InvalidIdentity`, `DuplicateId`, `DuplicateName`, `UnknownReference`, `MissingRequiredObject`, `WrongKind`, `MachineMismatch`, `DeviceOwnedMultipleTimes`, `MissingMaster`, `MultipleMasters`, `UnownedDevice`, `InvalidRoute`, `InvalidTransport`, `InvalidDuplex` |
| `Topology::validate_scheduling_properties` (`core/src/topology.rs:425-462`) | `UnmeasuredProperty` |
| `Topology::validate_route` (`core/src/topology.rs:492-526`) | `UnknownReference`, `RouteEndpointMismatch`, `InvalidRoute` |
| `DiscoveryProfile::validate` (`core/src/discovery.rs:62-271`) | `InvalidIdentity`, `IdentityMismatch`, `UnknownReference`, `DuplicateId`, `UnavailableRequiredObject`, `UnsupportedCapability`, `UnmeasuredProperty`, `MissingRequiredObject`, `WrongKind`, `ResourceMismatch` |
| `ScalarProgram::validate` and `validate_signature` (`core/src/scalar.rs:251-389`) | `DuplicateScalarValue`, `ScalarUseBeforeDefinition`, `ScalarArity`, `ScalarTypeMismatch`, `MissingScalarOutput`, `UnknownScalarValue` |
| `StaticBufferAccess::validate` (`core/src/scalar.rs:472-554`) | `InvalidMemoryAccess`, `AddressOverflow` |
| `KernelTemplate::validate` (`core/src/scalar.rs:596-704`) | nested scalar errors, `DuplicateId`, `ScalarArity`, `ScalarTypeMismatch`, `UnknownReference`, `DuplicateAliasRule`, `MissingAliasRule` |
| `ResourceManifest::validate` (`core/src/schedule.rs:486-554`) | `DuplicateId`, `UnknownReference` |
| `ReservationLedger::validate` (`core/src/plan.rs:67-126`) | `UnknownReference`, `DuplicateReservation`, `WrongKind`, `WrongReservationSize`, `MissingReservation` |
| `CapacityLedger::validate` (`core/src/plan.rs:151-224`) | `UnknownReference`, `DuplicateId`, `UnmeasuredProperty`, `CapacityOverflow`, `MissingRequiredObject` |
| `DraftPlan::validate` and `validate_tasks` (`core/src/plan.rs:253-871`) | nested topology/discovery/resource/kernel/artifact/build errors, `InvalidIdentity`, `IdentityMismatch`, `DuplicateId`, `UnknownReference`, `WrongKind`, `InvalidLifetime`, `DependencyPhaseOrder`, `DependencyScheduleOrder`, `InvalidPhase`, `InvalidCalculationPlacement`, `ArtifactMismatch`, `ResourceMismatch`, `InvalidExternalTransfer`, `InvalidRoute`, `InvalidFaultReadback`, `MissingUpload`, `DuplicateUpload`, `DependencyCycle` |
| Draft helper validators (`core/src/plan.rs:873-1442`) | `validate_fault_readbacks`, `validate_transfer_endpoint`, `validate_transfer_lane_claims`, `validate_resource_contention`, `validate_compute_transfer_overlap`, and `validate_submission` construct `InvalidFaultReadback`, `UnknownReference`, `ResourceMismatch`, `InvalidLaneClaim`, `ResourceContention`, and `CapabilityConcurrencyExceeded` |
| Draft storage helpers (`core/src/plan.rs:1444-2055`) | `validate_arena_objects`, `validate_value_bindings`, `validate_init_images`, `validate_alias_bindings`, `validate_alias_pair`, and `validate_releases` construct `DuplicateId`, `UnknownReference`, `AllocationMisaligned`, `InvalidLifetime`, `DuplicateValueBinding`, `ResourceMismatch`, `ValueBindingMisaligned`, `ValueBindingOutOfBounds`, `ValueLifetimeMismatch`, `MissingValueBinding`, `InvalidDataImage`, `MissingUpload`, `LiveAllocationOverlap`, `MissingRequiredObject`, `DuplicateAliasRule`, `ScalarArity`, `AliasViolation`, `MissingRelease`, and `DuplicateRelease` |
| `RealizationProfile::validate` and `validate_realized_artifacts` (`core/src/plan.rs:2072-2228`) | `InvalidIdentity`, `IdentityMismatch`, `DuplicateId`, `ArtifactMismatch`, `ResourceMismatch`, plus nested artifact/reservation/capacity errors |
| `validate_loop_domains` and `FinalizedBundle::finalize_with_loop_schedule` (`core/src/plan.rs:2230-2517`) | `InvalidIdentity`, `DuplicateId`, `UnknownReference`, `InvalidPhase`, `InvalidIterationDomain`, plus nested Draft/Realization errors |
| `validate_layouts` and `resolve_value_locations` (`core/src/plan.rs:2632-2866`) | `UnknownReference`, `DuplicateId`, `DuplicateAllocation`, `ResourceMismatch`, `AllocationMisaligned`, `AllocationOutOfBounds`, `InsufficientCapacity`, `LiveAllocationOverlap`, `MissingRequiredObject`, `MissingAllocation`, `AddressOverflow`, `ValueBindingOutOfBounds`, `ValueBindingMisaligned` |

## Validation entry points and propagation

The following are the concrete core entry points and the workspace boundaries
that consume their results.

* `Topology::validate`, `validate_scheduling_properties`, and
  `validate_route` are used by probe profile construction and codec loading,
  planner admission and transfer lowering, scheduler route and schedule
  creation, native candidate admission, worker projection, and remote
  provisioning.  Probe maps the formatted collection to
  `ProbeError::InvalidProfile` (or a codec error).  Planner uses
  `PlannerErrorKind::InvalidTopology` or `InvalidDraft`; scheduler uses
  `ScheduleErrorKind::InvalidTopology` or `InvalidTransfer`.  A native
  candidate retains the collection in `CandidateRequestError`, then the
  validated factory exposes it as `CandidateFailure::CandidateRejected` text.
  Worker projection deliberately discards the individual collection and
  returns `WorkerProjectionError::InvalidTopology`.  Remote provisioning
  maps either topology result to the fixed
  `RemoteError::InvalidConfiguration` message.
* `DiscoveryProfile::validate` is run by probe codec/engine, planner,
  scheduler, and native candidate admission.  Those paths map it to
  `ProbeError::InvalidProfile`, `PlannerErrorKind::InvalidDiscovery`,
  `ScheduleErrorKind::InvalidDiscovery`, or
  `CandidateRequestError::InvalidDiscovery` respectively.
* `ArtifactBuildRecipe::validate` is used while drafting plans, in planner
  build-recipe construction, and by the kernel stage contract.  The native
  candidate boundary compares deferred-build fields as part of its own
  candidate checks, but does not call this core validator directly.
  `ArtifactIdentity::validate` is used in the planner artifact catalog, core
  Draft/Realization checks, native preparation, and native candidate
  admission.  The resulting collections become
  `PlannerErrorKind::InvalidDraft` or `InvalidArtifact`,
  `LoweringErrorKind::InvalidStageContract` text,
  `NativePrepareError::InvalidArtifact`, or
  `CandidateRequestError::InvalidArtifact`.
* `ScalarProgram::validate` and `KernelTemplate::validate` are used by the
  language scalar builder and primitive validator, the operation scalar
  builder, primitive lowered-program validation, kernel lowering, planner
  lowering, and the nested checks in `DraftPlan` and `FinalizedBundle`.
  Language and operations map the formatted collection to their
  `InvalidScalarProgram` error kinds.  `recipe-primitives` stores it as a
  `ProgramValidationError`, `recipe-kernel` maps it to a lowering error, and
  planner reports it as an invalid graph or draft.
* `ResourceManifest::validate` is called by Draft validation and therefore by
  planner, native candidate, and Finalize boundaries.  `ReservationLedger` and
  `CapacityLedger` are also called directly by planner admission, preparation,
  native preparation, native candidate validation, and Realization.  Planner
  classifies them as invalid reservation or capacity inputs.  Preparation and
  native preparation retain the formatted detail in their corresponding
  `PrepareError` or `NativePrepareError` kinds.  Native candidate admission
  retains the reservation collection as
  `CandidateRequestError::InvalidReservations`.
* `DraftPlan::validate` is run when planner constructs each candidate, again
  at native candidate admission, and by Finalize.  Planner reports
  `PlannerErrorKind::InvalidDraft`; native realization turns it into a
  candidate rejection; Finalize prefixes the nested paths with `draft`.
* `RealizationProfile::validate` is run by preparation after capacity
  stabilization and by Finalize.  Preparation records a failure as the
  candidate's final-capacity rejection detail; Finalize prefixes it with
  `realization` and refuses to construct `FinalizedBundle`.
* `FinalizedBundle::finalize`,
  `finalize_with_loop_iterations`, and `finalize_with_loop_schedule` return a
  `ValidationResult<FinalizedBundle>`.  Preparation maps a failure to
  `PrepareErrorKind::Finalization`; the native local warm-address path maps it
  to its warm-preparation error.  The finalizer never returns a partially
  validated bundle.

The outer errors all implement `Display`, so the normal user-facing path is
the exact `ValidationErrors` text embedded in a higher-level error.  No
workspace code currently branches on an individual `ValidationCode` after it
leaves `recipe-core`.

The direct call graph is:

| Core method | Callers that receive its `ValidationResult` |
| --- | --- |
| `Topology::validate` | `probe::engine::probe`, `probe::engine::build_topology`, `probe::codec::validate_profile`, `planner::plan_program_candidates`, `scheduler::shortest_route`, `scheduler::schedule`, `native_executor::CandidateRealizationRequest::validate`, `executor::WorkerProjection::derive`, `remote::ProvisionedProgram::from_bundle`, and `DraftPlan::validate` |
| `Topology::validate_scheduling_properties` | The same profile/planner/scheduler/worker/remote callers, plus `DraftPlan::validate` |
| `Topology::validate_route` | `planner::build_transfer_chain`, `scheduler::prepare_transfer`, and `DraftPlan::validate`'s internal-transfer branch |
| `DiscoveryProfile::validate` | `probe::engine::probe`, `probe::codec::validate_profile`, `planner::plan_program_candidates`, `scheduler::schedule`, `native_executor::CandidateRealizationRequest::validate`, and `DraftPlan::validate` |
| `ArtifactBuildRecipe::validate` | `planner::lower_candidate` build construction, `DraftPlan::validate`, and `kernel::stage::validate_contract` |
| `ArtifactIdentity::validate` | `planner::validate_artifact_catalog`, `prepare::production::validate_native_artifact`, `native_executor::validate_artifacts`, and Draft/Realization nested checks |
| `ScalarProgram::validate` | `language::ScalarProgramBuilder::finish`, `language::primitive::validate_elementwise`, `ops::scalar::lower_scalar`, `ops::scalar::Composer::finish`, and `KernelTemplate::validate` |
| `KernelTemplate::validate` | `kernel::llvm::lower_elementwise`, `primitives::validate::validate_kind`, `planner::lower_programs`, `DraftPlan::validate`, and Finalize through Draft |
| `ResourceManifest::validate` | `DraftPlan::validate`, then planner, native candidate, and Finalize through that Draft call |
| `ReservationLedger::validate` | `planner::plan_program_candidates`, `prepare::Preparer::prepare_program_validated`, `prepare::production::exact_reservation_plan`, `prepare::production::NativeCandidateRealizer::realize`, `native_executor::CandidateRealizationRequest::validate`, and Realization |
| `CapacityLedger::validate` | `planner::plan_program_candidates`, `prepare::optimistic_planning_capacity`, `prepare::validate_observation`, native candidate capacity snapshots, and Realization |
| `DraftPlan::validate` | `planner::lower_candidate`, `native_executor::CandidateRealizationRequest::validate`, and `FinalizedBundle::finalize_with_loop_schedule` |
| `RealizationProfile::validate` | `prepare::validate_observation` and `FinalizedBundle::finalize_with_loop_schedule` |
| `FinalizedBundle::finalize*` | `prepare::Preparer::prepare_program_validated` and `native_executor::local::provisional_warm_bundle` |

## `ValidationCode` reference

Each row names every construction site by the core validator that emits it,
the exact contract represented by that site, and the direct propagation
surface.  When a row lists several fields, each field uses the same code but
has its own `path` and message in the source.

| Code | Construction and failure semantics | Direct propagation |
| --- | --- | --- |
| `EmptyName` | Declared in the enum only.  No core validator constructs it. | None in the workspace. |
| `InvalidIdentity` | `ArtifactBuildRecipe::validate` rejects zero artifact, stage, source-kernel, program-digest, or contract-digest identities. `ArtifactIdentity::validate` rejects zero artifact/toolchain/build digests. `Topology::validate` and `DiscoveryProfile::validate` reject zero profile identities. `DraftPlan`, `RealizationProfile`, and Finalize reject zero draft/candidate/realization/bundle identities. | Travels through the owning validator's outer surface, as described above. There is no repair or generated replacement identity. |
| `DuplicateId` | Rejects repeated machine, device, node, local node-device, link, discovered device/link, artifact-build binding value, Draft value/kernel/artifact/build/template, task/dependency, resource slot or per-device resource entry, init-image device/member, realization artifact, loop-domain task, finalized layout device, or other indexed object. | Probe, planner, scheduler, preparation, native candidate, and finalization preserve it in their respective formatted validation detail. |
| `DuplicateName` | `Topology::validate` requires unique machine `Label` values. | Topology consumers classify the enclosing topology as invalid. |
| `UnknownReference` | Emitted whenever a machine, device, link, task, value, kernel, artifact, slot, arena object, image member, alias endpoint, release, loop-domain task, or layout object is absent from the authoritative index. `validate_transfer_endpoint` and resource-manifest checks are the common helpers. | The enclosing topology, discovery, Draft, schedule, realization, or finalization error carries the path to the missing reference. |
| `WrongKind` | Topology rejects calculation rates on RAM/Disk devices. Discovery rejects calculation capability on non-GPU storage. Reservation validation rejects evidence whose GPU/non-GPU kind does not match the device. Draft rejects value byte storage that is not a whole number of its declared typed elements. | The corresponding profile, reservation, or Draft wrapper reports the formatted failure. |
| `MissingMaster` | `Topology::validate` emits this when no node has `NodeRole::Master`; the path is `nodes` and the message requires exactly one master. | Probe, planner, scheduler, native candidate, worker projection, and remote provisioning reject the topology. |
| `MultipleMasters` | `Topology::validate` emits this when more than one master node is present, including the observed count. | Same topology consumers as `MissingMaster`. |
| `UnownedDevice` | `Topology::validate` requires every declared storage device to occur in a node's ownership map. | Same topology consumers. |
| `DeviceOwnedMultipleTimes` | `Topology::validate` records a second node owner for one device, naming the previous owner. | Same topology consumers. |
| `MachineMismatch` | `Topology::validate` rejects a node device whose physical device belongs to a different machine than the node. | Same topology consumers. |
| `MissingRequiredObject` | Used for required but absent fields or objects: GPU calculation rate, machine devices, discovery GPU capability and required device/link records, nonzero build dispatch/workgroup/rank, artifact workgroup limit, capacity entries, init-image manifests, finalized arena layouts, and other mandatory stage objects. | The enclosing validator returns all missing-object failures; outer wrappers classify the enclosing profile, Draft, reservation/capacity, or Finalize boundary. |
| `UnavailableRequiredObject` | Discovery requires every declared device and link to report `available`. | Probe profile validation and planner/scheduler/native candidate discovery validation reject the profile. |
| `UnsupportedCapability` | Discovery rejects zero submission queues, synchronous transfer or calculation paths, zero calculation concurrency, non-power-of-two subgroup width, a workgroup smaller than one subgroup, or zero shared-memory capability. | Discovery failures become `InvalidProfile`, `InvalidDiscovery`, or candidate rejection. |
| `UnmeasuredProperty` | Topology scheduling validation and discovery validation reject estimated properties in production: capacity, transfer rate, calculation rate, bandwidth, and measured concurrency. Capacity validation applies the same rule to total, overhead, fragmentation, headroom, and Recipe-usable values. | Probe reports an unschedulable/invalid profile; planner reports invalid topology or capacity; preparation/native preparation report invalid measured profile. |
| `InvalidRoute` | Topology rejects a directed self-link. Draft transfer validation rejects an external admission/egress with an internal route or an executor-visible internal transfer with more than one link. `validate_route` also emits it when a distinct-device transfer has no link. | Scheduler maps route failure to `InvalidTransfer`; planner maps route lowering failure to `InvalidDraft`; nested Draft paths retain the route prefix. |
| `InvalidLaneClaim` | Draft transfer lane validation requires strict sorted uniqueness, one claim per directed link, an in-range measured lane index, no external lane on an internal transfer, no link lane on external transfer, and exactly one endpoint-device lane for external admission/egress. | Draft validation and Finalize report the transfer path; planner and native candidate expose it through invalid Draft/candidate text. |
| `InvalidTransport` | Topology requires each transport identity to have exactly two directed edges, reverse endpoints, equal transport kind, and one consistent transport identity. | Topology consumers reject the profile. |
| `InvalidDuplex` | Topology rejects zero capacity resources, a kind/duplex mismatch, asymmetric duplex modes, half-duplex edges that do not share one resource, full-duplex edges that share one, or a resource reused by another transport. | Topology consumers reject the profile. |
| `RouteEndpointMismatch` | `Topology::validate_route` reports a link whose `from` is not the current device and a final route endpoint different from the requested destination. | Scheduler, planner, and Draft route checks retain the route path and formatted mismatch. |
| `MissingAliasRule` | `KernelTemplate::validate` requires one explicit alias relationship for every input/output pair. | Language primitive validation, kernel lowering, planner, primitive program validation, and nested Draft checks report the enclosing invalid kernel/program. |
| `DuplicateAliasRule` | `KernelTemplate::validate` rejects a repeated input/output alias pair. Draft alias contracts use the same code for a repeated value pair. | Same kernel/program/Draft surfaces as `MissingAliasRule`. |
| `UnknownScalarValue` | `ScalarProgram::validate` rejects an output ID absent from inputs, constants, and instruction results. | Language and operation scalar builders map it to `InvalidScalarProgram`; nested kernel and planner paths retain it in their formatted collection. |
| `ScalarUseBeforeDefinition` | `ScalarProgram::validate` rejects an instruction operand not present in the definitions accumulated before that instruction. | Same scalar-program consumers. |
| `ScalarArity` | Scalar signature validation rejects an opcode operand count different from `ScalarOpcode::arity`. Kernel validation compares kernel input/output counts to the scalar program. Draft task validation compares calculation input/output counts to its kernel template. | Language, operations, primitives, kernel, planner, and Draft wrappers expose the formatted arity detail. |
| `ScalarTypeMismatch` | Scalar signature validation rejects an opcode result type not implied by operand types. Kernel validation compares kernel argument/output types to scalar types. | Same scalar and kernel consumers as `ScalarArity`. |
| `DuplicateScalarValue` | `ScalarProgram::validate` rejects an input, constant, or instruction result that redefines an existing scalar value ID. | Language and operation scalar builders, then all nested kernel/planner consumers. |
| `MissingScalarOutput` | `ScalarProgram::validate` requires at least one scalar output. | Language and operation scalar builders map it to `InvalidScalarProgram`; later consumers retain the detail. |
| `InvalidMemoryAccess` | `StaticBufferAccess::validate` rejects rank mismatch, a mapping whose required bytes exceed storage, or a writable mapping whose strides overlap or broadcast. | `KernelTemplate::validate` prefixes the path with its input/output access; language, primitives, kernel, planner, and Draft consumers carry it outward. |
| `DependencyCycle` | Draft task validation performs Kahn traversal and emits this when not every task is visited. | Planner reports invalid Draft, native candidate rejects it, and Finalize rejects a candidate whose Draft remains cyclic. |
| `InvalidPhase` | Draft requires calculation and metric tasks in `Loop`; loop-domain assignments may name only loop tasks. | Planner-produced Drafts, native candidate admission, and Finalize preserve the task/domain path. Scheduler has a separate lifecycle error and does not construct this core code. |
| `DependencyPhaseOrder` | Draft rejects a dependency whose predecessor is in a later lifecycle phase. | Invalid Draft through planner, native candidate, and Finalize. |
| `DependencyScheduleOrder` | Draft rejects a dependency whose schedule window ends after the dependent task starts. | Invalid Draft through planner, native candidate, and Finalize. |
| `InvalidCalculationPlacement` | Draft requires calculations on GPU storage and requires every calculation input/output value to be resident on that GPU. | Planner emits invalid Draft; native candidate and Finalize propagate the collection. |
| `InvalidIterationDomain` | Finalize loop-domain validation rejects a domain outside the loop, a missing/non-loop assignment, a loop task without a domain, a fault readback with a different domain, or an internal transfer consumer with a different domain. | `PrepareErrorKind::Finalization` or native warm-finalization detail; nested Draft checks use the same code for the finalized schedule. |
| `InvalidFaultReadback` | Draft fault validation requires exactly one readback per checked fault cohort, an exclusive metric slot, a direct dependency from every checked calculation, and a dependency before every publication. It also rejects a readback naming an unused flag. | Invalid Draft in planner/native candidate and finalization failure in preparation. |
| `InvalidExternalTransfer` | Draft allows external admission only in `Init`, external egress only in `Exit`, and rejects external-to-external tasks. | Planner/native candidate/finalization report the task path. Scheduler performs a separate transfer-shape check and reports its own `ScheduleErrorKind::InvalidTransfer`; it does not construct this core code. |
| `InvalidDataImage` | Init-image validation requires nonzero image bytes, exact correspondence with the device's sole admission task, producer identity from that task, nonzero logical member IDs, producer identity for physical members, and offsets that resolve to the physical binding. | Invalid Draft through planner/native candidate and `PrepareErrorKind::Finalization` at finalization. |
| `MissingUpload` | Draft emits this when a device lacks its required external init admission task or its init image lacks a corresponding admission. | Planner candidate rejection, native candidate rejection, or finalization failure. |
| `DuplicateUpload` | Draft emits this when a device has more than one external init admission. | Same Draft/finalization surfaces as `MissingUpload`. |
| `MissingRelease` | Draft release validation requires one exit arena release for every topology device. | Planner, native candidate, and Finalize reject the candidate. |
| `DuplicateRelease` | Draft release validation rejects more than one exit arena release for a device. | Same release consumers. |
| `DuplicateReservation` | `ReservationLedger::validate` rejects more than one user reservation entry for one device. | Planner invalid reservation, preparation/native preparation invalid reservation, and native candidate invalid reservations. |
| `MissingReservation` | `ReservationLedger::validate` requires an entry for every topology device. | Same reservation consumers. |
| `WrongReservationSize` | Reservation validation requires the exact byte count implied by `ReservationEvidence::required_bytes`, including the zero-byte display-disabled GPU case. | Same reservation consumers. |
| `InvalidReservationName` | Declared in the enum only.  No core validator constructs it. | None in the workspace. |
| `CapacityOverflow` | Capacity validation reports either checked-add overflow while accounting reservation, overhead, fragmentation, headroom, and Recipe-usable bytes, or an accounted total greater than the declared total. | Planner invalid capacity, preparation invalid measured profile, native candidate capacity rejection, and Realization/Finalize nested capacity failure. |
| `InsufficientCapacity` | Final arena layout validation rejects a layout larger than the measured Recipe-usable capacity. | Finalization maps it to `PrepareErrorKind::Finalization`; native capacity snapshot/finalization paths retain the detail. |
| `IdentityMismatch` | Discovery requires its topology identity to match the supplied topology. Draft requires its topology/discovery identities to match the inputs. Realization requires draft, candidate, topology, and discovery identities to remain unchanged. | Probe/planner/scheduler/native candidate wrappers classify the enclosing profile; Finalize prefixes the Draft or Realization path. |
| `ArtifactMismatch` | Draft task validation binds a calculation to the exact kernel/artifact identity and discovered target. Realization requires every drafted prebuilt/deferred artifact exactly once, unchanged prebuilt identities, matching deferred provenance/kernel/target, and no extra artifacts. | Planner invalid Draft/Artifact, native candidate invalid artifact or candidate rejection, native preparation invalid artifact, and Finalize nested realization failure. |
| `ResourceMismatch` | This code covers declared contract disagreement: dispatch ceiling or workgroup size, artifact/build bindings and work, discovery direction/rate/concurrency, value/device/type/byte relationships, transfer endpoints, submission slots, task fault contracts, init-image contracts, realization resources, and resolved arena/value devices. | The enclosing profile, Draft, realization, planner, native preparation, candidate, or Finalize boundary carries the exact path and message. |
| `ResourceContention` | Draft rejects overlapping tasks sharing a queue, completion slot, transfer lane, or half-duplex transport, and rejects transfer/calculation overlap when discovery does not permit it. | Planner invalid Draft, native candidate rejection, and finalization failure. |
| `CapabilityConcurrencyExceeded` | Draft event sweeps reject more simultaneous directed-link transfers, external transfers, or GPU calculations than the measured capability allows. | Invalid Draft through planner/native candidate and Finalize. |
| `DuplicateAllocation` | Final arena validation rejects an arena object allocated more than once. | Finalization and native warm-address validation. |
| `MissingAllocation` | Final arena validation requires every Draft arena object to have a finalized allocation. | Finalization and native warm-address validation. |
| `AllocationOutOfBounds` | Final arena validation rejects an allocation whose checked end overflows or exceeds its arena layout size. | Finalization and native warm-address validation. |
| `AllocationMisaligned` | Draft arena objects require a nonzero power-of-two alignment; final allocations must satisfy each object's alignment. | Planner/Draft and Finalize/native layout validation. |
| `DuplicateValueBinding` | Draft value validation rejects more than one arena binding for a value. | Planner invalid Draft, native candidate rejection, and Finalize. |
| `MissingValueBinding` | Draft value validation requires every value to have an arena binding; init-image validation requires image and physical members to be bound. | Planner/native candidate invalid Draft and finalization failure. |
| `ValueBindingOutOfBounds` | Draft binding validation requires a value's object-relative end within its object. Init-image members must end within the packed image. Finalized resolution requires the arena end within the layout. | Planner/native candidate invalid Draft, finalization, and native warm-address detail. |
| `ValueBindingMisaligned` | Draft bindings and init-image offsets must align to the value/member scalar width; finalized resolved offsets must align to the payload type. | Same Draft, Finalize, and native warm-address surfaces. |
| `ValueLifetimeMismatch` | Draft binding validation requires the arena object's lifetime to contain the value's producer and every task that references the value. | Planner invalid Draft, native candidate rejection, and Finalize. |
| `AliasViolation` | Draft alias validation evaluates `Forbidden`, `MayAliasExact`, and `MustAliasExact` against object, offset, and byte-range overlap. | Planner invalid Draft, native candidate rejection, and Finalize. |
| `AddressOverflow` | Scalar buffer mapping reports overflow in required-byte address arithmetic. Finalize reports overflow when adding an object offset to a value binding or computing a value end in the arena. | Kernel/scalar consumers expose it in invalid program text; preparation/native finalization expose it as finalization or warm-address detail. |
| `LiveAllocationOverlap` | Final arena layouts reject overlapping allocations whose object lifetimes overlap. Init-image packing rejects overlapping member ranges. | Finalization and native warm-address validation. |
| `InvalidLifetime` | Draft rejects empty task schedule windows and empty arena-object lifetimes. | Planner invalid Draft, native candidate rejection, and Finalize. |

## Failure boundary summary

The collection is authoritative only until an outer crate deliberately
classifies it.  The classifications observed in this workspace are:

* probe and profile codec: `ProbeError::InvalidProfile` or a codec error with
  the complete formatted collection;
* language and operations: `InvalidScalarProgram` with the complete formatted
  scalar collection;
* primitives and kernel: primitive program validation and direct LLVM kernel
  lowering include the complete collection; the kernel stage-contract guard
  reduces its boolean check to a fixed invalid-stage message;
* planner: `PlannerError` kinds such as `InvalidTopology`,
  `InvalidDiscovery`, `InvalidReservation`, `InvalidCapacity`,
  `InvalidArtifact`, or `InvalidDraft`;
* scheduler: `ScheduleErrorKind::InvalidTopology`,
  `InvalidDiscovery`, or `InvalidTransfer`;
* preparation and native preparation: typed preparation kinds whose message
  contains the collection, including candidate rejection and finalization
  details;
* native candidate admission: `CandidateRequestError` retains topology,
  discovery, Draft, reservation, and artifact collections.  Its `source`
  implementation exposes only the first four categories through the standard
  error chain; `InvalidArtifact` is rendered by `Display` but intentionally
  has no source;
* worker projection: `WorkerProjectionError::InvalidTopology` intentionally
  drops the individual collection after validating the topology; and
* remote provisioning: `RemoteError::InvalidConfiguration` intentionally
  emits one fixed configuration message for either topology validation pass.

Every boundary is fail-closed.  A nonempty `ValidationErrors` prevents the
validated product from being constructed or admitted to the next phase.  No
consumer treats a validation code as permission to substitute an object,
retry a pass, or continue with an invalid state.

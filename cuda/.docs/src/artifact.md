 # CUDA artifact identities and compatibility

 [`cuda/src/artifact.rs`](../../src/artifact.rs) is the pure identity boundary
 for a realized NVIDIA cubin. It does not load a module, create a context,
 inspect an ELF section, compile a kernel, or choose a device. It defines two
 identity snapshots and one accumulated compatibility check:

 * `ArtifactIdentity` describes the bytes and the build/runtime policy attached
   to those bytes.
 * `DeploymentIdentity` describes the exact Driver/device reopening that will
   consume the bytes.
 * `validate_artifact_compatibility` compares the snapshots and the byte image,
   returning every detected `ArtifactIssue` in one `ArtifactCompatibilityError`.

 The module is private in `recipe-cuda`, but all of these values and the
 validator are re-exported by [`cuda/src/lib.rs`](../../src/lib.rs#L16-L37).
 The crate's Driver, discovery, context, and runtime modules own the native
 handles. This module only borrows their value-level identity types, so a
 compatibility failure cannot leave a partially loaded CUDA module behind.

 ## Boundary and identity domains

 Recipe has two artifact identity types with different responsibilities. The
 backend-neutral [`recipe_core::ArtifactIdentity`](../../../core/src/artifact.rs#L214-L266)
 is part of a finalized graph and carries an artifact ID, semantic target and
 ABI labels, entry symbol, kernel template, resources, and build provenance.
 The CUDA [`recipe_cuda::ArtifactIdentity`](../../src/artifact.rs#L21-L29) is a
 Driver compatibility record. Preparation stores both records and checks that
 their digest, target, entry, and format agree before the native executor gets
 a `RuntimeArtifact`. A CUDA identity does not replace the finalized graph
 identity, and this module does not validate the graph or kernel ABI.

 The direct value dependencies are:

 | Value | Defined by | Role in this module |
 | --- | --- | --- |
 | `ComputeCapability` | [`discovery.rs`](../../src/discovery.rs#L51-L63) | Ordered `(major, minor)` SM target. It is compared exactly, not by a nearest architecture rule. |
 | `DriverVersion` | [`discovery.rs`](../../src/discovery.rs#L65-L106) | Ordered raw Driver version used for minimum/maximum checks. Negative raw values are rejected at construction. |
 | `DeviceUuid` and `DeviceInfo` | [`discovery.rs`](../../src/discovery.rs#L31-L49), [`discovery.rs`](../../src/discovery.rs#L124-L135) | Stable device membership and the target copied into a deployment snapshot. |
 | `Discovery` | [`discovery.rs`](../../src/discovery.rs#L137-L148) | One immutable Driver version, capability set, and device list from a reopening. |
 | `DriverSymbol` | [`ffi.rs`](../../src/ffi.rs#L91-L214) | An exact Driver API symbol that a cubin policy requires. |
 | `DriverCapabilities` | [`ffi.rs`](../../src/ffi.rs#L258-L271) | The set resolved while the Driver was loaded; `supports` performs a set membership check. |

 The six `ToolchainIdentity` strings are deliberately plain strings. The
 struct has no constructor and performs no normalization: `zig_version`,
 `llvm_version`, `ptx_isa_version`, `ptxas_version`,
 `cuda_toolkit_version`, and `cubin_format` are copied exactly from the
 preparation policy. Compatibility only requires the observed values to be
 nonempty after trimming and requires expected and observed values to be equal.
 The validator does not parse version syntax or infer one version from another.

 `ArtifactIdentity` contains:

 | Field | Meaning and check |
 | --- | --- |
 | `sha256: [u8; 32]` | Digest declared for the complete cubin byte slice. It is checked against the bytes and against `expected.sha256`. |
 | `target: ComputeCapability` | Exact SM major/minor expected by the artifact. It is compared to the expected record and the reopened deployment target. |
 | `toolchain: ToolchainIdentity` | Exact six-string build identity. Every string participates in expected/observed comparison. |
 | `minimum_driver` | Inclusive lower bound for the deployed Driver. |
 | `maximum_driver: Option<DriverVersion>` | Optional inclusive upper bound. `None` means no upper-bound check. |
 | `required_driver_symbols: BTreeSet<DriverSymbol>` | Complete required API set. The set must equal the expected set, and every observed symbol must be present in the reopened Driver capabilities. |

 `DeploymentIdentity` contains the reopening-side values:

 | Field | Origin and use |
 | --- | --- |
 | `driver_version` | `Discovery.driver_version`, obtained from `cuDriverGetVersion` during `Driver::discover`. |
 | `device_uuid` | The selected `DeviceInfo.uuid`. It protects deployment membership and context binding, but is not an artifact field. |
 | `target` | The selected `DeviceInfo.compute_capability`; it is the target used by the compatibility check. |
 | `driver_capabilities` | A clone of `Discovery.capabilities`, which is the required and optional symbol inventory resolved by `ffi::Api::load`. |

 ### Constructing a deployment identity

 `DeploymentIdentity::from_discovery(discovery, device)` is an existence check
 over one snapshot ([`artifact.rs`](../../src/artifact.rs#L39-L55)). It returns
 `None` unless one entry in `discovery.devices` has both the same private
 `DeviceOrdinal` and the same `DeviceUuid` as `device`. On success it copies
 `discovery.driver_version` and `discovery.capabilities`, then copies the
 passed device's UUID and compute capability.

 The membership predicate is intentionally ordinal plus UUID. It does not
 compare the candidate's name, PCI string, memory, attributes, or compute
 capability. In production the `DeviceInfo` is taken directly from the same
 `Discovery` being passed, and the preceding native-probe descriptor checks
 the PCI and measured profile identity. The constructor itself does not
 re-probe or open a context. A caller that fabricates or mixes a `DeviceInfo`
 can therefore only be rejected for failed ordinal/UUID membership here; the
 native executor's later context check also requires the context device UUID
 and compute capability to equal this deployment record.

 The only production caller is
 [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L268-L317):
 `realize_cuda` reopens the configured Driver, obtains a fresh `Discovery`,
 matches each reopened descriptor to the measured profile, calls
 `from_discovery`, and then creates a `Context`. A `None` result is converted
 to a binding error, so no CUDA binding is lent to preparation. The resulting
 identity is cloned into `CudaBinding` and remains borrowed for the
 preparation/execution scope.

 ## Compatibility error vocabulary

 `ArtifactField` names exactly the fields compared by `compare_identity`:

 * `Sha256`
 * `Target`
 * `ZigVersion`
 * `LlvmVersion`
 * `PtxIsaVersion`
 * `PtxasVersion`
 * `CudaToolkitVersion`
 * `CubinFormat`
 * `MinimumDriver`
 * `MaximumDriver`
 * `RequiredDriverSymbols`

 `ArtifactIssue` is the complete public failure vocabulary:

 | Issue | Produced when |
 | --- | --- |
 | `EmptyArtifact` | `cubin.is_empty()` is true. |
 | `InvalidCubinMagic` | The bytes do not start with the four-byte ELF prefix `0x7fELF`. An empty slice receives both this issue and `EmptyArtifact`. |
 | `DigestMismatch { declared, computed }` | SHA-256 of the complete byte slice differs from `observed.sha256`; the fields preserve both digests. |
 | `IdentityFieldMismatch { field }` | One field named by `ArtifactField` differs between `expected` and `observed`. One issue is emitted for each differing field. |
 | `EmptyIdentityField { field }` | An observed toolchain or format string is empty after `trim()`. |
 | `InvalidDriverRange { minimum, maximum }` | `observed.maximum_driver` is present and lower than `observed.minimum_driver`. |
 | `DeviceTargetMismatch { artifact, device }` | `observed.target` differs from `deployment.target`. |
 | `DriverTooOld { artifact_minimum, deployed }` | `deployment.driver_version < observed.minimum_driver`. |
 | `DriverTooNew { artifact_maximum, deployed }` | An observed maximum exists and `deployment.driver_version >` it. |
 | `MissingDriverSymbol { symbol }` | `deployment.driver_capabilities.supports(symbol)` is false for an observed required symbol. |

 `ArtifactCompatibilityError` owns a public `Vec<ArtifactIssue>` and derives
 `Clone`, `Debug`, `Eq`, and `PartialEq`. Its `Display` output is only
 `CUDA artifact identity is incompatible (N issue(s))`; callers that need the
 actionable field, digest, version, target, or symbol must inspect `issues`.
 It implements `std::error::Error` without a source error because all failures
 are value-level compatibility findings, not Driver calls.

 ## Validator algorithm

 [`validate_artifact_compatibility`](../../src/artifact.rs#L124-L188) is pure
 with respect to CUDA state. It accepts a borrowed byte image, expected and
 observed CUDA identities, and a deployment identity. It allocates one local
 issue vector, never mutates an input, and never invokes the Driver. The checks
 run in this exact order, and all checks run even after an earlier issue:

 1. If the byte slice is empty, append `EmptyArtifact`.
 2. Require `cubin.starts_with(b"\\x7fELF")`; otherwise append
    `InvalidCubinMagic`. This is only a four-byte envelope check.
 3. Compute `Sha256::digest(cubin)` over every byte. If that result differs
    from `observed.sha256`, append `DigestMismatch` with the declared and
    computed arrays.
 4. Call `compare_identity(expected, observed, &mut issues)`. The helper
    evaluates the digest, target, all six toolchain strings, both Driver bounds,
    and the complete required-symbol set in the `ArtifactField` order above.
    It appends one `IdentityFieldMismatch` for each false comparison; it does
    not stop at the first difference.
 5. Call `validate_nonempty(observed, &mut issues)`. The helper checks the six
    toolchain strings only, using `text.trim().is_empty()`. It checks the
    observed record, not the expected record, and does not reject whitespace
    around an otherwise nonempty string.
 6. If an observed maximum exists and is below the observed minimum, append
    `InvalidDriverRange`. The range check uses `DriverVersion`'s derived total
    ordering, which compares the encoded raw Driver value.
 7. Compare the observed compute capability with the deployment capability.
    A difference appends `DeviceTargetMismatch`.
 8. Compare the deployment Driver version with the observed bounds. A version
    below the minimum appends `DriverTooOld`; a version above a present maximum
    appends `DriverTooNew`. Equality at either bound is accepted.
 9. Iterate the observed `BTreeSet<DriverSymbol>` and call
    `DriverCapabilities::supports` for each symbol. Every absent symbol gets
    its own `MissingDriverSymbol` issue.

 The function returns `Ok(())` only when the vector is empty. Otherwise it
 returns `Err(ArtifactCompatibilityError { issues })`. Because the vector is
 accumulated, a caller can report a corrupt byte image, multiple identity
 differences, an invalid range, a target mismatch, a Driver range failure,
 and missing symbols together rather than discovering them one at a time.

 The two identity arguments have separate meanings. `expected` is the policy
 the caller wants to use, while `observed` is the metadata attached to the
 bytes being considered. The byte digest is independently recomputed and is
 compared to `observed.sha256`; `expected.sha256 == observed.sha256` is a
 separate identity comparison. This distinction matters when a caller receives
 metadata from an untrusted or stale artifact catalog. The current native
 executor passes the same CUDA identity for both arguments after its stronger
 backend-neutral plan checks, so `IdentityFieldMismatch` is not expected on
 that production path, but the public function preserves the distinct API for
 direct validation and other preparation boundaries.

 ### What this check deliberately does not do

 The validator is a compatibility envelope, not a complete native-image
 inspector. In particular it does not:

 * parse ELF headers, sections, relocations, SASS, or cubin metadata beyond the
   four-byte magic;
 * verify the kernel entry symbol, argument ABI, element count, workgroup size,
   resource bounds, or target-specific image contents;
 * compare a cubin to a device UUID;
 * open a Driver, context, module, stream, or event;
 * recompile a missing image or choose a fallback target;
 * normalize, parse, or semantically interpret the six toolchain strings; or
 * validate the backend-neutral finalized graph identity.

 The executor performs the missing image/ABI work at its own boundary. A
 separate [`inspect_cubin`](../../../kernel/src/artifact.rs#L449-L488) check is run before
 `Module::load_cubin`; the finalized `ExecutionPlan` verifies artifact IDs,
 image digest, entry symbol, target ABI, workgroup bounds, and the complete
 calculation ABI ([`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs#L227-L352)).
 UUID membership is established by `DeploymentIdentity::from_discovery` and
 context pairing is checked by `validate_binding`, not by the cubin identity
 record.

 ## How preparation produces the identity

 The identity is materialized only during native preparation, after discovery,
 target selection, lowering, and Realize. The path is intentionally split into
 backend-neutral and CUDA-specific records:

 ```text
 measured profile and reopened CUDA binding
     -> DeploymentIdentity { Driver version, UUID, SM, symbols }
     -> cuda_spec / CudaArtifactPolicy
     -> Realize cubin image and compute SHA-256
     -> recipe_core::ArtifactIdentity + recipe_cuda::ArtifactIdentity
     -> RuntimeArtifactKind::Cuda { identity }
     -> native executor validation and module load
 ```

 `src/native_prepare.rs::cuda_spec` receives a measured descriptor and a
 `DeploymentIdentity` ([`src/native_prepare.rs`](../../../src/native_prepare.rs#L651-L702)).
 It converts the deployment SM major/minor into an `NvidiaTarget`, requires the
 measured target to use backend `nvidia-cuda-driver`, ABI `elf64-cubin`, and the
 same architecture, and requires every symbol in
 `REQUIRED_DRIVER_SYMBOLS` to be present in `deployment.driver_capabilities`.
 It then creates a `CudaArtifactPolicy` with:

 * the CUDA toolchain identity from `cuda_toolchain_identity`;
 * `minimum_driver = deployment.driver_version`;
 * `maximum_driver = Some(deployment.driver_version)` in the current production
   policy; and
 * the required-symbol set copied from `REQUIRED_DRIVER_SYMBOLS`.

 `cuda_toolchain_identity` is also explicit about fields retained for the
 legacy `recipe-cuda` shape. It records
 `zig_version = "not-used-recipe-rust-owned-ir"`, an LLVM string containing the
 configured release and verifier/LLVM-codegen SHA-256 values, the configured
 PTX ISA, the pinned PTX assembler digest, and
 `cuda_toolkit_version = "not-claimed-pinned-ptxas-only"`; its format is
 `elf64-cubin` ([`src/native_prepare.rs`](../../../src/native_prepare.rs#L747-L770)).
 These are ordinary strings to `recipe-cuda`; their construction policy lives
 in preparation.

 `recipe-prepare::DeferredArtifactCompiler` lowers and builds each deferred
 stage during `BuildPhase::Realize`, or validates an explicitly supplied
 prebuilt cubin with `inspect_cubin` before using it
 ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L254-L460)).
 `native_artifact_from_image` computes the image digest, builds the finalized
 `recipe_core::ArtifactIdentity`, calls `runtime_kind`, and then constructs a
 `RuntimeArtifact` with the same image and ABI
 ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L549-L575)).
 For NVIDIA, `runtime_kind` creates this module's `ArtifactIdentity` with:

 * the image SHA-256;
 * `ComputeCapability::new(target.sm_major, target.sm_minor)`;
 * the `CudaArtifactPolicy` toolchain and Driver range; and
 * a clone of the required Driver symbol set
 ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L577-L607)).

 Before an image enters the catalog, `NativeArtifact::new` and
 `NativeArtifactCatalog::new` call `validate_native_artifact`. That separate
 preparation check verifies the core identity itself, runtime ID and digest,
 runtime entry symbol, target ABI, and CUDA target/digest relationship
 ([`prepare/src/production.rs`](../../../prepare/src/production.rs#L53-L129),
 [`prepare/src/production.rs`](../../../prepare/src/production.rs#L655-L722)).
 It does not replace the deployment-time validator: it has no deployment
 argument and therefore cannot check the live Driver version, target, or symbol
 capability set.

 ## Native consumer and load order

 `recipe-native-executor` is the only current production caller of
 `validate_artifact_compatibility`, at
 [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1665-L1815).
 The call occurs in `realize_device`, before any module is loaded:

 1. `CudaResources::realize` rejects duplicate, missing, or unexpected device
    bindings, checks finalized submission-queue capacity, and calls
    `validate_binding` for each binding
    ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L348-L421)).
 2. `validate_binding` requires the context's discovered UUID and compute
    capability to equal `binding.deployment.device_uuid` and
    `binding.deployment.target` ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1633-L1646)).
 3. `realize_device` selects only calculation artifacts assigned to that CUDA
    device. A missing runtime image returns `Error::MissingArtifact`, and a
    `RuntimeArtifactKind` other than `Cuda` returns `Error::ArtifactMismatch`.
 4. It calls
    `validate_artifact_compatibility(&runtime.bytes, identity, identity,
    &binding.deployment)`. Passing the same identity as expected and observed
    makes the expected/observed comparison a consistency pass, while the live
    deployment target, Driver range, and required-symbol checks remain active.
    A compatibility error is wrapped as `Error::ArtifactMismatch` with the
    logical `ArtifactId` and `error.to_string()`. That wrapper keeps the issue
    count, but the native executor's string conversion does not retain the
    public `issues` vector. Direct callers of the CUDA function can retain it.
 5. It derives the deployment SM number, calls `recipe_kernel::inspect_cubin`
    with that SM and the immutable ABI entry symbol, and rejects an inspected
    entry that differs from the ABI.
 6. It groups logical artifacts by `RuntimeArtifact` content digest. Distinct
    artifact IDs may share one exact image, but a digest group with different
    bytes is rejected. The image is loaded once with
    `Module::load_cubin`; each logical entry is then resolved with
    `Module::function` and retained in `LoadedArtifact`.

 Modules and functions are created in Realize/preparation, before the
 `init -> loop -> exit` lifecycle. `DeviceResources` retains one module per
 distinct cubin digest and one resolved function per logical artifact. Stable
 `Box<Module>` ownership keeps the borrowed function values valid. Teardown
 drops functions before unloading modules and requires streams to be idle and
 completion events to be available
 ([`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1817-L1847),
 [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L2203-L2245)).
 There is no loop-time compatibility check, load, compilation, or identity
 fallback.

 The backend-neutral `ExecutionPlan` runs before `realize_device`. Its
 `validate_runtime_artifact` checks runtime ID, image digest, ABI entry, format
 versus target ABI, nonzero workgroup width, and the finalized maximum; its
 `validate_target` checks the core target/backend pair, CUDA architecture
 against the Driver identity, and Driver identity digest against the runtime
 image ([`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs#L261-L352)).
 `validate_calculation_abi` then checks stage identity, elements, operands,
 dtype/access, backing bytes, alignment, fault flag, dynamic argument positions,
 and the final element-count argument
 ([`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs#L354-L640)).
 Those checks explain why the CUDA compatibility validator intentionally stays
 focused on the Driver-facing envelope.

 ## Failure propagation and invariants

 The failure boundary is fail-closed. There is no nearest-SM selection, Driver
 version fallback, missing-symbol downgrade, retry, or alternate artifact path
 in this module. The surrounding layers preserve the same rule:

 | Boundary | Failure and consequence |
 | --- | --- |
 | `DeploymentIdentity::from_discovery` | `None` means the selected ordinal/UUID is not in the exact reopening snapshot; `realize_cuda` returns a binding error and does not lend the context. |
 | `cuda_spec` | A target/backend/ABI mismatch, absent required Driver symbol, or invalid SM conversion returns `NativePreparationError::IdentityMismatch`; no build specification is created. |
 | `NativeArtifact::new` / catalog | A core identity, runtime ID/digest/entry/ABI, target, or CUDA identity inconsistency returns `InvalidArtifact`; the image cannot enter the catalog. |
 | `validate_artifact_compatibility` | Every byte, identity, target, range, and symbol issue is accumulated in `ArtifactCompatibilityError`; no native object is opened. |
 | `ExecutionPlan` | A runtime/core identity or ABI mismatch returns `Error::ArtifactMismatch` before device resources are realized. |
 | `CudaResources::realize` | Deployment/context mismatch, a non-CUDA runtime kind, cubin inspection failure, or digest/byte inconsistency aborts realization before `LoopStarted`. |
 | module/function load | A Driver `CudaError` is returned by the runtime wrapper; no later submission is allowed from a partially valid resource set. |

 Important invariants visible at this boundary are:

 * A realized `RuntimeImage` computes a content digest from its bytes, and the
   compatibility check independently recomputes SHA-256 from the exact `&[u8]`
   being loaded.
 * The observed Driver range is checked for internal ordering before the live
   Driver is compared with it. An invalid range can therefore be reported
   together with `DriverTooOld` or `DriverTooNew` if the live version also lies
   outside the malformed bounds.
 * `maximum_driver = None` has no upper-bound issue. A present maximum is
   inclusive, and `maximum < minimum` is invalid.
 * Required symbols are a set, so duplicate declarations collapse before the
   validator and each absent symbol is reported once.
 * Device UUID is deployment/context identity, not artifact portability
   identity. The same target cubin may be compatible with multiple devices of
   that target when their Driver version and symbol policy satisfy the record.
 * The validator does not decide whether a missing native file should be
   rebuilt. Prebuilt selection and Realize compilation are owned by
   `recipe-prepare`; if no prebuilt bundle is supplied, preparation builds one
   in Realize and then creates the identity from its bytes.
 * Driver capabilities are captured when the `Discovery` snapshot is made.
   `DeploymentIdentity` clones that set; `validate_artifact_compatibility` does
   not perform a late `dlsym` or mutate the Driver's capability inventory.

 ## Source and caller map

 | Responsibility | Source |
 | --- | --- |
 | CUDA identity structs, deployment membership, issue vocabulary, pure validator, and private helpers | [`cuda/src/artifact.rs`](../../src/artifact.rs#L1-L259) |
 | Public re-export | [`cuda/src/lib.rs`](../../src/lib.rs#L16-L27) |
 | Driver symbol enum, required/optional symbol sets, and capability membership | [`cuda/src/ffi.rs`](../../src/ffi.rs#L91-L271) |
 | Driver version, compute capability, UUID, device info, discovery snapshot | [`cuda/src/discovery.rs`](../../src/discovery.rs#L31-L148) |
 | Fresh Driver/discovery and deployment construction | [`native-probe/src/bindings.rs`](../../../native-probe/src/bindings.rs#L268-L317) |
 | Deployment-backed CUDA policy and toolchain strings | [`src/native_prepare.rs`](../../../src/native_prepare.rs#L651-L770) |
 | Native image digest, core identity, CUDA runtime identity construction | [`prepare/src/production.rs`](../../../prepare/src/production.rs#L549-L607) |
 | Pre-catalog identity consistency | [`prepare/src/production.rs`](../../../prepare/src/production.rs#L655-L722) |
 | Finalized runtime/core identity and ABI checks | [`native-executor/src/plan.rs`](../../../native-executor/src/plan.rs#L129-L352) |
 | Sole production compatibility caller, cubin inspection, module/function load | [`native-executor/src/cuda.rs`](../../../native-executor/src/cuda.rs#L1665-L1847) |

 The direct call graph is therefore:

 ```text
 Driver::discover
   -> Discovery { driver_version, capabilities, devices }
   -> DeploymentIdentity::from_discovery
   -> CudaBinding { context, deployment }
   -> CudaResources::realize / realize_device
   -> validate_artifact_compatibility
      -> compare_identity
      -> validate_nonempty
      -> DriverCapabilities::supports
   -> inspect_cubin
   -> Module::load_cubin / Module::function
 ```

 `compare_identity` and `validate_nonempty` are private callees. No other
 workspace source currently calls the public validator, and no code in
 `artifact.rs` calls a native Driver operation. A structural check such as
 `cargo check -p recipe-cuda` verifies the type and FFI boundary only; actual
 compatibility evidence comes from the production probe, preparation, and
 CUDA execution path on the measured Driver deployment.

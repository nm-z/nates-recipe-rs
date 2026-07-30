use std::convert::Infallible;

use recipe_core::{
	AliasPermission, ArtifactId, ByteCount, BytesPerSecond, CalculationCapability, Device, DeviceId, DeviceKind,
	Digest, DiscoveredDevice, DiscoveryIdentity, FlopsPerSecond, IterationDomain, KernelResourceBounds,
	KernelTemplateId, Label, Machine, MachineId, Node, NodeId, NodeRole, Property, ReservationEntry,
	ReservationEvidence, ReservationMechanism, ScalarInput, ScalarInstruction, ScalarOpcode, ScalarProgram,
	ScalarValueId, TargetIdentity, ToolchainIdentity, TopologyIdentity, TransferCapability, TransferLaneCount,
	ValueId,
};
use recipe_language::{
	CalculationNode, Elementwise, PrimitiveAliasRule, PrimitiveKernel, PrimitiveKind, Shape, Tensor,
};
use recipe_program::KernelIterationDomain;

use super::*;

fn label(value: &str) -> Label {
	Label::new(value).unwrap()
}

fn measured<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Measured)
}

fn fixture() -> (
	Topology,
	DiscoveryProfile,
	CalculationGraph,
	Vec<ArtifactIdentity>,
) {
	let machine = MachineId::new(1);
	let gpu = DeviceId::new(1);
	let gpu_b = DeviceId::new(2);
	let topology_identity = TopologyIdentity::new(Digest::new([1; 32]));
	let target = TargetIdentity {
		backend: label("test-native"),
		architecture: label("test-arch"),
		abi: label("test-abi"),
	};
	let topology = Topology {
		identity: topology_identity,
		machines: vec![Machine {
			id: machine,
			name: label("measured-host"),
		}],
		nodes: vec![Node {
			id: NodeId::new(1),
			role: NodeRole::Master,
			machine,
			devices: vec![gpu, gpu_b],
		}],
		devices: [gpu, gpu_b]
			.into_iter()
			.map(|id| Device {
				id,
				machine,
				kind: DeviceKind::GpuMemory,
				capacity: measured(ByteCount::new(3_000_000_000)),
				transfer_rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
				calculation_rate: Some(measured(FlopsPerSecond::new(1_000_000_000).unwrap())),
			})
			.collect(),
		links: Vec::new(),
	};
	let discovery = DiscoveryProfile {
		identity: DiscoveryIdentity::new(Digest::new([2; 32])),
		topology: topology_identity,
		devices: [gpu, gpu_b]
			.into_iter()
			.map(|device| DiscoveredDevice {
				device,
				available: true,
				maximum_submission_queues: 64,
				total_capacity: measured(ByteCount::new(3_000_000_000)),
				transfer: TransferCapability {
					rate: measured(BytesPerSecond::new(1_000_000_000).unwrap()),
					maximum_inflight_transfers: measured(TransferLaneCount::new(1).unwrap()),
					asynchronous_submission: true,
					overlaps_calculation: true,
				},
				calculation: Some(CalculationCapability {
					target: target.clone(),
					rate: measured(FlopsPerSecond::new(1_000_000_000).unwrap()),
					asynchronous_submission: true,
					maximum_concurrent_tasks: 2,
					subgroup_lanes: 32,
					maximum_workgroup_lanes: 256,
					maximum_shared_memory_per_workgroup: ByteCount::new(64 * 1024),
				}),
			})
			.collect(),
		links: Vec::new(),
	};
	let input = ScalarValueId::new(1);
	let output = ScalarValueId::new(2);
	let graph = CalculationGraph {
		tensors: vec![
			Tensor::contiguous(
				ValueId::new(1),
				recipe_core::DType::F32,
				Shape::new(vec![16]).unwrap(),
				true,
				false,
			)
			.unwrap(),
			Tensor::contiguous(
				ValueId::new(2),
				recipe_core::DType::F32,
				Shape::new(vec![16]).unwrap(),
				false,
				true,
			)
			.unwrap(),
		],
		nodes: vec![CalculationNode {
			kernel: PrimitiveKernel {
				id: KernelTemplateId::new(1),
				inputs: vec![ValueId::new(1)],
				outputs: vec![ValueId::new(2)],
				alias_rules: vec![PrimitiveAliasRule {
					input: 0,
					output: 0,
					permission: AliasPermission::Forbidden,
				}],
				kind: PrimitiveKind::Elementwise(Elementwise {
					program: ScalarProgram {
						inputs: vec![ScalarInput {
							id: input,
							dtype: recipe_core::DType::F32,
						}],
						constants: Vec::new(),
						instructions: vec![ScalarInstruction {
							result: output,
							dtype: recipe_core::DType::F32,
							opcode: ScalarOpcode::Negate,
							operands: vec![input],
						}],
						outputs: vec![output],
					},
				}),
			},
		}],
	};
	let artifact = |id: u64| ArtifactIdentity {
		id: ArtifactId::new(id),
		digest: Digest::new([u8::try_from(id).unwrap(); 32]),
		format: label("test-abi"),
		target: target.clone(),
		toolchain: ToolchainIdentity {
			name: label("recipe-test-toolchain"),
			version: label("1"),
			digest: Digest::new([9; 32]),
		},
		entry_symbol: label("recipe_kernel_1"),
		kernel_template: KernelTemplateId::new(1),
		resources: KernelResourceBounds {
			private_bytes_per_lane: ByteCount::ZERO,
			shared_bytes_per_workgroup: ByteCount::ZERO,
			scratch_bytes_per_dispatch: ByteCount::ZERO,
			maximum_workgroup_lanes: 256,
		},
		build: None,
	};
	(topology, discovery, graph, vec![artifact(1), artifact(2)])
}

#[derive(Debug)]
struct TestArtifacts {
	artifacts: Vec<ArtifactIdentity>,
}

impl ArtifactProvider for TestArtifacts {
	type Catalog = Vec<ArtifactIdentity>;
	type Error = Infallible;

	fn resolve(
		&mut self,
		_graph: &CalculationGraph,
		_topology: &Topology,
		_discovery: &DiscoveryProfile,
	) -> Result<Self::Catalog, Self::Error> {
		Ok(self.artifacts.clone())
	}

	fn identities<'a>(&self, catalog: &'a Self::Catalog) -> &'a [ArtifactIdentity] {
		catalog
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestMode {
	FirstCapacityRejected,
	AlwaysUnstable,
}

#[derive(Debug)]
struct TestRealizer {
	mode: TestMode,
	realized: usize,
	destroyed: usize,
}

#[derive(Debug)]
struct TestSession {
	_attempt: usize,
}

impl CandidateRealizer<Vec<ArtifactIdentity>> for TestRealizer {
	type Session = TestSession;
	type Error = Infallible;

	fn reservation_plan(
		&self,
		topology: &Topology,
		_discovery: &DiscoveryProfile,
	) -> Result<ReservationLedger, Self::Error> {
		Ok(ReservationLedger {
			entries: topology
				.devices
				.iter()
				.map(|device| ReservationEntry {
					device: device.id,
					name: label(&format!("recipe-user-{}", device.id.get())),
					bytes: EXACT_USER_RESERVATION,
					mechanism: ReservationMechanism::HeldAllocation,
					evidence: match device.kind {
						DeviceKind::GpuMemory => ReservationEvidence::GpuDisplay {
							enabled_connectors: 1,
						},
						DeviceKind::Ram | DeviceKind::Disk => ReservationEvidence::NonGpu,
					},
				})
				.collect(),
		})
	}

	fn realize(
		&mut self,
		candidate: &PlannedCandidate,
		catalog: &Vec<ArtifactIdentity>,
		reservations: &ReservationLedger,
		policy: PreparationPolicy,
	) -> Result<RealizedCandidate<Self::Session>, RealizeFailure<Self::Error>> {
		self.realized += 1;
		let usable = match (self.mode, self.realized) {
			(TestMode::FirstCapacityRejected, 1) => ByteCount::ZERO,
			_ => ByteCount::new(1_000_000_000),
		};
		let stable_capacity = capacity(usable, reservations);
		let pass_count = usize::try_from(policy.stabilization_passes.get()).unwrap();
		let mut snapshots = vec![stable_capacity; pass_count];
		if self.mode == TestMode::AlwaysUnstable {
			snapshots[pass_count - 1] = capacity(ByteCount::new(999_999_999), reservations);
		}
		let artifacts = realized_artifacts(candidate, catalog);
		Ok(RealizedCandidate {
			session: TestSession {
				_attempt: self.realized,
			},
			observation: RealizationObservation {
				artifacts,
				resources: candidate.draft.resources.clone(),
				reservations: reservations.clone(),
				capacity_snapshots: snapshots,
			},
		})
	}

	fn destroy(&mut self, _session: Self::Session) -> Result<(), Self::Error> {
		self.destroyed += 1;
		Ok(())
	}
}

fn realized_artifacts(candidate: &PlannedCandidate, catalog: &[ArtifactIdentity]) -> Vec<ArtifactIdentity> {
	let exemplar = &catalog[0];
	let mut artifacts = candidate.draft.artifacts.clone();
	artifacts.extend(
		candidate
			.draft
			.artifact_builds
			.iter()
			.map(|build| ArtifactIdentity {
				id: build.artifact,
				digest: build.provenance.contract_digest,
				format: exemplar.format.clone(),
				target: exemplar.target.clone(),
				toolchain: exemplar.toolchain.clone(),
				entry_symbol: label(&format!("recipe_stage_{}", build.kernel_template.get())),
				kernel_template: build.kernel_template,
				resources: build.resources.clone(),
				build: Some(build.provenance),
			}),
	);
	artifacts
}

fn capacity(usable: ByteCount, reservations: &ReservationLedger) -> CapacityLedger {
	let accounted = EXACT_USER_RESERVATION.checked_add(usable).unwrap();
	let headroom = ByteCount::new(3_000_000_000)
		.checked_sub(accounted)
		.unwrap();
	CapacityLedger {
		entries: reservations
			.entries
			.iter()
			.map(|entry| CapacityLedgerEntry {
				device: entry.device,
				total: measured(ByteCount::new(3_000_000_000)),
				runtime_overhead: measured(ByteCount::ZERO),
				fragmentation: measured(ByteCount::ZERO),
				safety_headroom: measured(headroom),
				recipe_usable: measured(usable),
			})
			.collect(),
	}
}

fn planned_fixture() -> (
	Topology,
	DiscoveryProfile,
	PlannedCandidate,
	Vec<ArtifactIdentity>,
	ReservationLedger,
	CapacityLedger,
) {
	let (topology, discovery, graph, artifacts) = fixture();
	let reservations = ReservationLedger {
		entries: topology
			.devices
			.iter()
			.map(|device| ReservationEntry {
				device: device.id,
				name: label(&format!("recipe-user-{}", device.id.get())),
				bytes: EXACT_USER_RESERVATION,
				mechanism: ReservationMechanism::HeldAllocation,
				evidence: match device.kind {
					DeviceKind::GpuMemory => ReservationEvidence::GpuDisplay {
						enabled_connectors: 1,
					},
					DeviceKind::Ram | DeviceKind::Disk => ReservationEvidence::NonGpu,
				},
			})
			.collect(),
	};
	let capacity = capacity(ByteCount::new(1_000_000_000), &reservations);
	let search = plan_candidates(
		&graph,
		&topology,
		&discovery,
		&artifacts,
		&reservations,
		&capacity,
	)
	.unwrap();
	let candidate = search.ranked_candidates()[0].clone();
	(
		topology,
		discovery,
		candidate,
		artifacts,
		reservations,
		capacity,
	)
}

fn stable_observation(
	candidate: &PlannedCandidate,
	artifacts: &[ArtifactIdentity],
	reservations: &ReservationLedger,
	capacity: &CapacityLedger,
) -> RealizationObservation {
	RealizationObservation {
		artifacts: realized_artifacts(candidate, artifacts),
		resources: candidate.draft.resources.clone(),
		reservations: reservations.clone(),
		capacity_snapshots: vec![capacity.clone(); 3],
	}
}

#[test]
fn rejects_post_warm_capacity_then_finalizes_the_next_candidate() {
	let (topology, discovery, graph, artifacts) = fixture();
	let mut preparer = Preparer::new(
		TestArtifacts { artifacts },
		TestRealizer {
			mode: TestMode::FirstCapacityRejected,
			realized: 0,
			destroyed: 0,
		},
	);
	let result = preparer
		.prepare_validated(&graph, &topology, &discovery)
		.unwrap();
	assert_eq!(result.attempted_candidates(), 2);
	assert_eq!(result.rejections().len(), 1);
	assert_eq!(
		result.rejections()[0].stage,
		CandidateRejectionStage::FinalCapacity
	);
	assert_eq!(preparer.realizer().realized, 2);
	assert_eq!(preparer.realizer().destroyed, 1);
	assert_eq!(result.bundle().realization(), result.realization().identity);
}

#[test]
fn program_preparation_finalizes_its_repeat_bound_and_sparse_domain() {
	let (topology, discovery, graph, artifacts) = fixture();
	let iterations = LoopIterations::new(5).unwrap();
	let domain = IterationDomain::new(1, 5, 2).unwrap();
	let program = StaticCalculationProgram::new(
		graph,
		iterations,
		vec![KernelIterationDomain {
			kernel: KernelTemplateId::new(1),
			domain,
		}],
	)
	.unwrap();
	let mut preparer = Preparer::new(
		TestArtifacts { artifacts },
		TestRealizer {
			mode: TestMode::FirstCapacityRejected,
			realized: 0,
			destroyed: 0,
		},
	);
	let result = preparer
		.prepare_program_validated(&program, &topology, &discovery)
		.unwrap();
	assert_eq!(result.bundle().loop_iterations(), iterations);
	assert!(!result.bundle().loop_domains().is_empty());
	assert!(
		result.bundle()
			.loop_domains()
			.iter()
			.all(|assignment| assignment.domain == domain)
	);
}

#[test]
fn unstable_realizations_are_destroyed_until_finite_exhaustion() {
	let (topology, discovery, graph, artifacts) = fixture();
	let mut preparer = Preparer::new(
		TestArtifacts { artifacts },
		TestRealizer {
			mode: TestMode::AlwaysUnstable,
			realized: 0,
			destroyed: 0,
		},
	);
	let error = preparer
		.prepare_validated(&graph, &topology, &discovery)
		.unwrap_err();
	assert_eq!(error.kind, PrepareErrorKind::CandidateExhaustion);
	assert_eq!(error.rejections.len(), 2);
	assert!(
		error.rejections
			.iter()
			.all(|rejection| rejection.stage == CandidateRejectionStage::Stabilize)
	);
	assert!(error.rejections.iter().all(|rejection| {
		rejection.detail.contains("device 1")
			&& rejection.detail.contains("recipe_usable")
			&& rejection.detail.contains("final decreased by 1")
	}));
	assert_eq!(preparer.realizer().realized, 2);
	assert_eq!(preparer.realizer().destroyed, 2);
}

#[test]
fn candidate_exhaustion_display_retains_actionable_rejection_details() {
	let candidate = CandidateIdentity::new(Digest::new([9; 32]));
	let error = PrepareError::new(
		PrepareErrorKind::CandidateExhaustion,
		"all 1 finite candidates were rejected",
	)
	.with_rejections(vec![CandidateRejection {
		candidate,
		stage: CandidateRejectionStage::Realize,
		detail: "artifact 17 failed to load".to_owned(),
	}]);
	let display = error.to_string();
	assert!(display.contains("CandidateIdentity"));
	assert!(display.contains("Realize"));
	assert!(display.contains("artifact 17 failed to load"));
}

#[test]
fn rejects_an_impossible_stability_policy_before_side_effects() {
	let (_, _, _, artifacts) = fixture();
	let preparer = Preparer::new(
		TestArtifacts { artifacts },
		TestRealizer {
			mode: TestMode::FirstCapacityRejected,
			realized: 0,
			destroyed: 0,
		},
	);
	let error = preparer
		.with_policy(PreparationPolicy {
			stabilization_passes: NonZeroU32::new(2).unwrap(),
			stable_tail: NonZeroU32::new(3).unwrap(),
		})
		.unwrap_err();
	assert_eq!(error.kind, PrepareErrorKind::InvalidPolicy);
}

#[test]
fn deferred_artifact_and_stage_tampering_fails_closed() {
	let (topology, discovery, candidate, artifacts, reservations, capacity) = planned_fixture();
	let observation = stable_observation(&candidate, &artifacts, &reservations, &capacity);
	let policy = PreparationPolicy::default();
	assert!(
		validate_observation(
			&topology,
			&discovery,
			&candidate.draft,
			&observation,
			policy,
		)
		.is_ok()
	);
	let rejected = |observation: &RealizationObservation| {
		assert!(validate_observation(&topology, &discovery, &candidate.draft, observation, policy,).is_err());
	};

	let mut tampered = observation.clone();
	tampered.artifacts.clear();
	rejected(&tampered);

	let mut tampered = observation.clone();
	tampered.artifacts.push(tampered.artifacts[0].clone());
	rejected(&tampered);

	let mut tampered = observation.clone();
	tampered.artifacts[0].kernel_template = KernelTemplateId::new(
		tampered.artifacts[0]
			.kernel_template
			.get()
			.checked_add(1)
			.unwrap(),
	);
	rejected(&tampered);

	let mut tampered = observation.clone();
	tampered.artifacts[0].target.architecture = label("tampered-architecture");
	rejected(&tampered);

	let mut tampered = observation.clone();
	tampered.artifacts[0].resources.private_bytes_per_lane = ByteCount::new(1);
	rejected(&tampered);

	let mut tampered = observation.clone();
	tampered.artifacts[0].build = None;
	rejected(&tampered);

	for provenance_field in 0..3 {
		let mut tampered = observation.clone();
		let provenance = match tampered.artifacts[0].build.as_mut() {
			Some(provenance) => provenance,
			None => panic!("fixture artifact must carry deferred-build provenance"),
		};
		match provenance_field {
			0 => provenance.program_digest = Digest::new([31; 32]),
			1 => {
				provenance.stage_ordinal = provenance.stage_ordinal.checked_add(1).unwrap();
			}
			2 => provenance.contract_digest = Digest::new([32; 32]),
			_ => unreachable!("the test iterates exactly three provenance fields"),
		}
		rejected(&tampered);
	}

	let mut tampered = observation.clone();
	tampered.resources.queues[0].device = DeviceId::new(2);
	rejected(&tampered);

	let mut tampered_draft = candidate.draft.clone();
	tampered_draft.artifact_builds[0].bindings[0]
		.view
		.storage_bytes = ByteCount::new(1);
	assert!(tampered_draft.validate(&topology, &discovery).is_err());

	let mut tampered_draft = candidate.draft.clone();
	tampered_draft.artifact_builds[0].dispatch.workgroups = tampered_draft.artifact_builds[0]
		.dispatch
		.workgroups
		.checked_add(1)
		.unwrap();
	assert!(tampered_draft.validate(&topology, &discovery).is_err());

	let mut tampered_draft = candidate.draft.clone();
	tampered_draft.artifact_builds[0].kernel_template = KernelTemplateId::new(
		tampered_draft.artifact_builds[0]
			.kernel_template
			.get()
			.checked_add(1)
			.unwrap(),
	);
	assert!(tampered_draft.validate(&topology, &discovery).is_err());

	let mut tampered_draft = candidate.draft.clone();
	tampered_draft.value_aliases[0].permission = AliasPermission::MustAliasExact;
	assert!(tampered_draft.validate(&topology, &discovery).is_err());
}

#[test]
fn canonical_identities_commit_deferred_alias_stage_and_realized_fields() {
	let (topology, discovery, candidate, artifacts, reservations, capacity) = planned_fixture();
	let policy = PreparationPolicy::default();
	let observation = stable_observation(&candidate, &artifacts, &reservations, &capacity);
	let realization_identity = hash_realization(&candidate.draft, &observation, policy);
	let realization = RealizationProfile {
		identity: realization_identity,
		draft: candidate.draft.identity,
		candidate: candidate.draft.candidate,
		discovery: discovery.identity,
		topology: topology.identity,
		artifacts: observation.artifacts.clone(),
		resources: observation.resources.clone(),
		reservations: observation.reservations.clone(),
		capacity: capacity.clone(),
	};
	let layouts = pack_arenas(&topology, &candidate.draft.arena_objects, &capacity).unwrap();
	let bundle_identity = hash_bundle(
		&topology,
		&discovery,
		&candidate.draft,
		&realization,
		&layouts,
	);
	let draft_changed = |draft: &DraftPlan| {
		assert_ne!(
			realization_identity,
			hash_realization(draft, &observation, policy)
		);
		assert_ne!(
			bundle_identity,
			hash_bundle(&topology, &discovery, draft, &realization, &layouts)
		);
	};

	let mut tampered = candidate.draft.clone();
	tampered.artifact_builds[0].source_kernel = KernelTemplateId::new(99);
	draft_changed(&tampered);

	let mut tampered = candidate.draft.clone();
	tampered.artifact_builds[0].provenance.stage_ordinal = tampered.artifact_builds[0]
		.provenance
		.stage_ordinal
		.checked_add(1)
		.unwrap();
	draft_changed(&tampered);

	let mut tampered = candidate.draft.clone();
	tampered.artifact_builds[0].bindings[0].view.offset_elements = tampered.artifact_builds[0].bindings[0]
		.view
		.offset_elements
		.checked_add(1)
		.unwrap();
	draft_changed(&tampered);

	let mut tampered = candidate.draft.clone();
	tampered.artifact_builds[0].work.integer_operations = tampered.artifact_builds[0]
		.work
		.integer_operations
		.checked_add(1)
		.unwrap();
	draft_changed(&tampered);

	let mut tampered = candidate.draft.clone();
	tampered.value_aliases[0].permission = AliasPermission::MustAliasExact;
	draft_changed(&tampered);

	let mut tampered = candidate.draft.clone();
	let calculation = tampered
		.tasks
		.iter_mut()
		.find_map(|task| match &mut task.kind {
			recipe_core::TaskKind::Calculation(calculation) => Some(calculation),
			recipe_core::TaskKind::Transfer(_) | recipe_core::TaskKind::Metric(_) => None,
		});
	let calculation = match calculation {
		Some(calculation) => calculation,
		None => panic!("fixture must contain a stage calculation"),
	};
	calculation.kernel_template = KernelTemplateId::new(calculation.kernel_template.get().checked_add(1).unwrap());
	draft_changed(&tampered);

	let mut tampered_observation = observation.clone();
	tampered_observation.artifacts[0].digest = Digest::new([44; 32]);
	assert_ne!(
		realization_identity,
		hash_realization(&candidate.draft, &tampered_observation, policy)
	);

	let mut reordered = observation.clone();
	reordered.artifacts.reverse();
	reordered.resources.queues.reverse();
	reordered.resources.completions.reverse();
	assert_eq!(
		realization_identity,
		hash_realization(&candidate.draft, &reordered, policy)
	);
}

use core::fmt;

use recipe_core::{
	ArenaAllocation, ArenaLayout, ArenaObject, ArenaObjectId, ArenaRelease, ByteOffset, BytesPerSecond,
	CandidateIdentity, CapacityLedger, CapacityLedgerEntry, CompletionSlot, CompletionSlotId, Device, DeviceKind,
	Digest, DiscoveredDevice, DiscoveryIdentity, DiscoveryProfile, DraftIdentity, DraftPlan, EXACT_USER_RESERVATION,
	InitDataImageMember, Label, Machine, MachineId, MetricSlot, MetricTask, Nanoseconds, Node, NodeId, NodeRole,
	Property, PropertyProvenance, QueueSlot, QueueSlotId, RealizationIdentity, RealizationProfile, ReservationEntry,
	ReservationEvidence, ReservationLedger, ReservationMechanism, ResourceManifest, ScheduleWindow, SubmissionSlots,
	Task, Topology, TopologyIdentity, TransferCapability, TransferLaneClaim, TransferLaneCount, TransferTask,
	ValueBinding, ValueSpec,
};
use recipe_executor::{
	ArenaSet, BackendPoll, BackendWork, PendingRequest, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus,
	TransferWork, WorkClass,
};
use recipe_ingest::{Delimiter, HeaderMode, IngestLimits, TableRequest, parse_table};

use super::*;
use crate::{
	CheckpointDecodeLimits, compile_prepared_inference, decode_checkpoint, prepare_checkpoint_inference_table,
};

const DEVICE: DeviceId = DeviceId::new(1);
const PHYSICAL_OUTPUT: ValueId = ValueId::new(500);
const INIT: TaskId = TaskId::new(1);
const USER_METRIC: TaskId = TaskId::new(2);
const EXIT_WITHOUT_METRIC: TaskId = TaskId::new(2);
const EXIT_WITH_METRIC: TaskId = TaskId::new(3);

struct InferenceBundleFixture {
	bundle: FinalizedBundle,
	exit: TaskId,
	user_metric: Option<TaskId>,
}

#[derive(Debug)]
struct JournalBackend;

#[derive(Debug)]
struct JournalResource;

#[derive(Debug)]
struct JournalArena {
	device: DeviceId,
}

#[derive(Debug)]
struct JournalPending {
	task: TaskId,
	class: WorkClass,
}

#[derive(Debug)]
struct JournalBackendError;

impl fmt::Display for JournalBackendError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("journal test backend failed")
	}
}

impl std::error::Error for JournalBackendError {}

impl recipe_executor::sealed::Sealed for JournalBackend {}

fn record(calls: &mut PhysicalCallBatch, call: PhysicalCall) -> Result<(), JournalBackendError> {
	calls.try_push(call).map_err(|_| JournalBackendError)
}

impl Backend for JournalBackend {
	type Arena = JournalArena;
	type Resource = JournalResource;
	type Pending = JournalPending;
	type Error = JournalBackendError;

	const MAX_NON_POLL_PHYSICAL_CALLS: usize = 1;

	fn bind_resources(
		&mut self,
		_bundle: &FinalizedBundle,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Resource, Self::Error> {
		record(physical_calls, PhysicalCall::BindResources)?;
		Ok(JournalResource)
	}

	fn prepare_pending(
		&mut self,
		_resource: &mut Self::Resource,
		request: PendingRequest,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Pending, Self::Error> {
		record(
			physical_calls,
			PhysicalCall::PreparePending { task: request.task },
		)?;
		Ok(JournalPending {
			task: request.task,
			class: request.class,
		})
	}

	fn allocate_arena(
		&mut self,
		_resource: &mut Self::Resource,
		layout: &ArenaLayout,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<Self::Arena, Self::Error> {
		record(
			physical_calls,
			PhysicalCall::AllocateArena {
				device: layout.device,
				bytes: layout.size,
			},
		)?;
		Ok(JournalArena {
			device: layout.device,
		})
	}

	fn submit(
		&mut self,
		_resource: &mut Self::Resource,
		_arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: BackendWork<'_>,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		assert_eq!(pending.task, work.task());
		assert_eq!(pending.class, work.class());
		let call = match work {
			BackendWork::InitAdmission(work) => PhysicalCall::AdmissionChunk {
				task: work.task,
				device: work.destination.device,
				bytes: work.bytes,
				chunk_index: 0,
			},
			BackendWork::ExitTransfer(work) => PhysicalCall::SubmitExitTransfer { task: work.task },
			BackendWork::Calculation(_) | BackendWork::InternalTransfer(_) | BackendWork::Metric(_) => {
				panic!("journal test bundle has no loop work")
			}
		};
		record(physical_calls, call)
	}

	fn poll(
		&mut self,
		_resource: &mut Self::Resource,
		pending: &mut Self::Pending,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<BackendPoll, Self::Error> {
		record(
			physical_calls,
			PhysicalCall::Poll {
				task: pending.task,
				status: PhysicalPollStatus::Complete,
			},
		)?;
		Ok(BackendPoll::Complete { metric: None })
	}

	fn collect_exit(
		&mut self,
		_resource: &mut Self::Resource,
		_arenas: ArenaSet<'_, Self::Arena>,
		pending: &mut Self::Pending,
		work: TransferWork<'_>,
		destination: &mut [u8],
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		assert_eq!(pending.task, work.task);
		destination.fill(0xA5);
		record(
			physical_calls,
			PhysicalCall::CollectExit {
				task: work.task,
				bytes: work.bytes,
			},
		)
	}

	fn release_arena(
		&mut self,
		_resource: &mut Self::Resource,
		device: DeviceId,
		arena: Self::Arena,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		assert_eq!(arena.device, device);
		record(physical_calls, PhysicalCall::ReleaseArena { device })
	}

	fn destroy_resources(
		&mut self,
		_resource: Self::Resource,
		physical_calls: &mut PhysicalCallBatch,
	) -> Result<(), Self::Error> {
		record(physical_calls, PhysicalCall::DestroyResources)
	}
}

fn digest(byte: u8) -> Digest {
	Digest::new([byte; 32])
}

fn label(text: &str) -> Label {
	Label::new(text).unwrap()
}

fn property<T>(value: T) -> Property<T> {
	Property::new(value, PropertyProvenance::Override)
}

fn window(start: u64, end: u64) -> ScheduleWindow {
	ScheduleWindow {
		start: Nanoseconds::new(start),
		end: Nanoseconds::new(end),
	}
}

fn compiled_binary_inference() -> CompiledInference {
	let checkpoint = decode_checkpoint(
		&crate::checkpoint::encoded_test_checkpoint_fixture(),
		CheckpointDecodeLimits::default(),
	)
	.unwrap();
	let table = parse_table(
		b"extra,\"feature\nbytes\"\nignored,1.5\nignored,2.5\n",
		TableRequest::new(
			Delimiter::Comma,
			HeaderMode::Present,
			IngestLimits::new(4_096, 32, 16, 1_024).unwrap(),
		),
	)
	.unwrap();
	let prepared = prepare_checkpoint_inference_table(checkpoint, &table).unwrap();
	compile_prepared_inference(&prepared).unwrap()
}

fn inference_bundle(dtype: DType, bytes: ByteCount, with_user_metric: bool) -> InferenceBundleFixture {
	let machine = MachineId::new(1);
	let topology_identity = TopologyIdentity::new(digest(1));
	let topology = Topology {
		identity: topology_identity,
		machines: vec![Machine {
			id: machine,
			name: label("machine"),
		}],
		nodes: vec![Node {
			id: NodeId::new(1),
			role: NodeRole::Master,
			machine,
			devices: vec![DEVICE],
		}],
		devices: vec![Device {
			id: DEVICE,
			machine,
			kind: DeviceKind::Ram,
			capacity: property(ByteCount::new(2_000_000_008)),
			transfer_rate: property(BytesPerSecond::new(1).unwrap()),
			calculation_rate: None,
		}],
		links: vec![],
	};
	let discovery_identity = DiscoveryIdentity::new(digest(2));
	let discovery = DiscoveryProfile {
		identity: discovery_identity,
		topology: topology_identity,
		devices: vec![DiscoveredDevice {
			device: DEVICE,
			available: true,
			maximum_submission_queues: 1,
			total_capacity: property(ByteCount::new(2_000_000_008)),
			transfer: TransferCapability {
				rate: property(BytesPerSecond::new(1).unwrap()),
				maximum_inflight_transfers: property(TransferLaneCount::new(1).unwrap()),
				asynchronous_submission: true,
				overlaps_calculation: false,
			},
			calculation: None,
		}],
		links: vec![],
	};
	let queue = QueueSlotId::new(1);
	let completion = CompletionSlotId::new(1);
	let metric = MetricId::new(1);
	let metric_slot = recipe_core::MetricSlotId::new(1);
	let submission = SubmissionSlots { queue, completion };
	let resources = ResourceManifest {
		queues: vec![QueueSlot {
			id: queue,
			device: DEVICE,
		}],
		completions: vec![CompletionSlot {
			id: completion,
			device: DEVICE,
		}],
		metrics: with_user_metric
			.then_some(MetricSlot {
				id: metric_slot,
				metric,
			})
			.into_iter()
			.collect(),
		pinned_staging: vec![],
		scratch: vec![],
	};
	let mut tasks = vec![Task {
		id: INIT,
		phase: RunPhase::Init,
		window: window(0, 1),
		dependencies: vec![],
		kind: TaskKind::Transfer(TransferTask {
			source: TransferEndpoint::External,
			destination: TransferEndpoint::Device {
				device: DEVICE,
				value: PHYSICAL_OUTPUT,
			},
			bytes,
			route: vec![],
			lane_claims: vec![TransferLaneClaim::External {
				device: DEVICE,
				lane: 0,
			}],
			submission,
		}),
	}];
	if with_user_metric {
		tasks.push(Task {
			id: USER_METRIC,
			phase: RunPhase::Loop,
			window: window(1, 2),
			dependencies: vec![INIT],
			kind: TaskKind::Metric(MetricTask {
				purpose: MetricPurpose::User,
				metric,
				value: PHYSICAL_OUTPUT,
				slot: metric_slot,
				submission,
			}),
		});
	}
	let exit = if with_user_metric {
		EXIT_WITH_METRIC
	} else {
		EXIT_WITHOUT_METRIC
	};
	tasks.push(Task {
		id: exit,
		phase: RunPhase::Exit,
		window: if with_user_metric {
			window(2, 3)
		} else {
			window(1, 2)
		},
		dependencies: vec![if with_user_metric { USER_METRIC } else { INIT }],
		kind: TaskKind::Transfer(TransferTask {
			source: TransferEndpoint::Device {
				device: DEVICE,
				value: PHYSICAL_OUTPUT,
			},
			destination: TransferEndpoint::External,
			bytes,
			route: vec![],
			lane_claims: vec![TransferLaneClaim::External {
				device: DEVICE,
				lane: 0,
			}],
			submission,
		}),
	});
	let draft_identity = DraftIdentity::new(digest(3));
	let candidate = CandidateIdentity::new(digest(4));
	let draft = DraftPlan {
		identity: draft_identity,
		candidate,
		discovery: discovery_identity,
		topology: topology_identity,
		values: vec![ValueSpec {
			id: PHYSICAL_OUTPUT,
			dtype,
			bytes,
			device: DEVICE,
			producer: Some(INIT),
		}],
		kernels: vec![],
		artifacts: vec![],
		artifact_builds: vec![],
		tasks,
		resources: resources.clone(),
		arena_objects: vec![ArenaObject {
			id: ArenaObjectId::new(1),
			device: DEVICE,
			bytes,
			alignment: ByteCount::new(4),
			lifetime: if with_user_metric {
				window(0, 3)
			} else {
				window(0, 2)
			},
		}],
		value_bindings: vec![ValueBinding {
			value: PHYSICAL_OUTPUT,
			object: ArenaObjectId::new(1),
			object_offset: ByteOffset::new(0),
		}],
		value_aliases: vec![],
		init_images: vec![InitDataImage {
			device: DEVICE,
			image: PHYSICAL_OUTPUT,
			bytes,
			members: vec![InitDataImageMember {
				logical: PHYSICAL_OUTPUT,
				physical: PHYSICAL_OUTPUT,
				dtype,
				bytes,
				image_offset: ByteOffset::new(0),
			}],
		}],
		releases: vec![ArenaRelease { device: DEVICE }],
	};
	let reservations = ReservationLedger {
		entries: vec![ReservationEntry {
			device: DEVICE,
			name: label("user"),
			bytes: EXACT_USER_RESERVATION,
			mechanism: ReservationMechanism::EnforcedQuota,
			evidence: ReservationEvidence::NonGpu,
		}],
	};
	let zero = property(ByteCount::ZERO);
	let capacity = CapacityLedger {
		entries: vec![CapacityLedgerEntry {
			device: DEVICE,
			total: property(ByteCount::new(2_000_000_008)),
			runtime_overhead: zero,
			fragmentation: zero,
			safety_headroom: zero,
			recipe_usable: property(ByteCount::new(1_000_000_008)),
		}],
	};
	let realization = RealizationProfile {
		identity: RealizationIdentity::new(digest(5)),
		draft: draft_identity,
		candidate,
		discovery: discovery_identity,
		topology: topology_identity,
		artifacts: vec![],
		resources,
		reservations,
		capacity,
	};
	let bundle = FinalizedBundle::finalize(
		BundleIdentity::new(digest(6)),
		&topology,
		&discovery,
		draft,
		realization,
		vec![ArenaLayout {
			device: DEVICE,
			size: bytes,
			allocations: vec![ArenaAllocation {
				object: ArenaObjectId::new(1),
				offset: ByteOffset::new(0),
			}],
		}],
	)
	.unwrap();
	InferenceBundleFixture {
		bundle,
		exit,
		user_metric: with_user_metric.then_some(USER_METRIC),
	}
}

fn planned_output(
	inference: &CompiledInference,
	fixture: &InferenceBundleFixture,
) -> (TaskId, ValueId, DeviceId, ValueId) {
	(
		fixture.exit,
		inference.output().value(),
		DEVICE,
		PHYSICAL_OUTPUT,
	)
}

#[test]
fn finalized_inference_output_mapping_is_exact_and_fail_closed() {
	let inference = compiled_binary_inference();
	let fixture = inference_bundle(DType::F32, ByteCount::new(8), false);
	let planned = planned_output(&inference, &fixture);
	let mapped = map_inference_output(&inference, &fixture.bundle, [planned]).unwrap();
	assert_eq!(mapped.task, fixture.exit);
	assert_eq!(mapped.logical, inference.output().value());
	assert_eq!(mapped.source.device, DEVICE);
	assert_eq!(mapped.source.value, PHYSICAL_OUTPUT);
	assert_eq!(mapped.source.dtype, DType::F32);
	assert_eq!(mapped.bytes, ByteCount::new(8));

	assert!(matches!(
		map_inference_output(
			&inference,
			&fixture.bundle,
			std::iter::empty::<(TaskId, ValueId, DeviceId, ValueId)>(),
		),
		Err(InferenceExecutionError::UnexpectedPredictionOutput { task, value: None }) if task == fixture.exit
	));
	assert!(matches!(
		map_inference_output(
			&inference,
			&fixture.bundle,
			[(planned.0, planned.1, planned.2, ValueId::new(501))],
		),
		Err(InferenceExecutionError::PredictionOutputSourceMismatch { task }) if task == fixture.exit
	));
	assert!(matches!(
		map_inference_output(&inference, &fixture.bundle, [planned, planned]),
		Err(InferenceExecutionError::DuplicatePredictionOutput { value }) if value == inference.output().value()
	));
	assert!(matches!(
		map_inference_output(
			&inference,
			&fixture.bundle,
			[(fixture.exit, ValueId::new(u64::MAX), DEVICE, PHYSICAL_OUTPUT)],
		),
		Err(InferenceExecutionError::UnexpectedPredictionOutput { task, .. }) if task == fixture.exit
	));

	let wrong_dtype = inference_bundle(DType::I32, ByteCount::new(8), false);
	assert!(matches!(
		map_inference_output(
			&inference,
			&wrong_dtype.bundle,
			[planned_output(&inference, &wrong_dtype)]
		),
		Err(InferenceExecutionError::PredictionOutputDTypeMismatch {
			expected: DType::F32,
			actual: DType::I32,
		})
	));
	let wrong_size = inference_bundle(DType::F32, ByteCount::new(4), false);
	assert!(matches!(
		map_inference_output(&inference, &wrong_size.bundle, [planned_output(&inference, &wrong_size)]),
		Err(InferenceExecutionError::PredictionOutputSizeMismatch {
			expected,
			actual,
		}) if expected == ByteCount::new(8) && actual == ByteCount::new(4)
	));
}

#[test]
fn inference_rejects_user_metrics_but_accepts_a_metric_free_finalized_bundle() {
	let metric_free = inference_bundle(DType::F32, ByteCount::new(8), false);
	reject_inference_user_metrics(&metric_free.bundle).unwrap();

	let with_metric = inference_bundle(DType::F32, ByteCount::new(8), true);
	let metric_task = with_metric.user_metric.unwrap();
	let error = reject_inference_user_metrics(&with_metric.bundle).unwrap_err();
	let InferenceExecutionError::InvalidInferenceBoundary { detail } = error else {
		panic!("user metric produced the wrong inference error: {error}");
	};
	assert!(detail.contains(&metric_task.to_string()));
}

fn raw_transfer_task(id: u64, phase: RunPhase, source: TransferEndpoint, destination: TransferEndpoint) -> Task {
	Task {
		id: TaskId::new(id),
		phase,
		window: window(0, 1),
		dependencies: vec![],
		kind: TaskKind::Transfer(TransferTask {
			source,
			destination,
			bytes: ByteCount::new(4),
			route: vec![],
			lane_claims: vec![],
			submission: SubmissionSlots {
				queue: QueueSlotId::new(1),
				completion: CompletionSlotId::new(1),
			},
		}),
	}
}

#[test]
fn inference_loop_external_scan_rejects_both_external_directions() {
	let value = TransferEndpoint::Device {
		device: DEVICE,
		value: PHYSICAL_OUTPUT,
	};
	let admission = raw_transfer_task(10, RunPhase::Loop, TransferEndpoint::External, value);
	let egress = raw_transfer_task(11, RunPhase::Loop, value, TransferEndpoint::External);
	let ordinary_init = raw_transfer_task(12, RunPhase::Init, TransferEndpoint::External, value);

	assert_eq!(
		inference_loop_external_transfer(&[ordinary_init, admission]),
		Some(TaskId::new(10))
	);
	assert_eq!(
		inference_loop_external_transfer(&[egress]),
		Some(TaskId::new(11))
	);
	let metric_free = inference_bundle(DType::F32, ByteCount::new(8), false);
	reject_inference_loop_external_transfers(&metric_free.bundle).unwrap();
}

fn initialized_journal_run(
	run: RunId,
	fixture: &InferenceBundleFixture,
) -> recipe_executor::InitializedRun<JournalBackend> {
	PreparedRun::prepare_recoverable(
		run,
		fixture.bundle.clone(),
		JournalBackend,
		Watchdog::new(4).unwrap(),
	)
	.unwrap()
	.initialize_recoverable([DeviceImage::new(DEVICE, PHYSICAL_OUTPUT, vec![0; 8])])
	.unwrap()
}

#[test]
fn inference_executor_failure_retains_typed_run_state_after_ordered_teardown() {
	let fixture = inference_bundle(DType::F32, ByteCount::new(8), false);
	let run = RunId::new(17);
	let running = initialized_journal_run(run, &fixture)
		.start_loop_recoverable()
		.unwrap();
	let source = ExecutorError::LifecycleInvariant {
		detail: "injected inference lifecycle failure",
	};
	let error = inference_executor_failure(running.fail(source));

	let InferenceExecutionError::Executor {
		source: actual,
		failure,
	} = error
	else {
		panic!("recoverable executor failure produced the wrong inference error")
	};
	assert_eq!(actual, source);
	assert_eq!(failure.run(), run);
	assert_eq!(failure.bundle(), fixture.bundle.identity());
	assert!(failure.cleanup_completed());
	let journal = failure.journal().unwrap();
	assert!(matches!(
		&journal.physical_calls()[journal.physical_calls().len() - 2..],
		[
			PhysicalCall::ReleaseArena { device: DEVICE },
			PhysicalCall::DestroyResources,
		]
	));
	assert!(
		!journal
			.logical_events()
			.iter()
			.any(|event| matches!(event, recipe_executor::LogicalEvent::Exited { .. }))
	);
}

#[test]
fn post_exit_prediction_validation_failure_retains_the_completed_journal() {
	let fixture = inference_bundle(DType::F32, ByteCount::new(8), false);
	let run = RunId::new(18);
	let mut running = initialized_journal_run(run, &fixture)
		.start_loop_recoverable()
		.unwrap();
	assert_eq!(running.poll().unwrap(), LoopStatus::Complete);
	let exited_loop = running
		.into_exited_loop()
		.unwrap_or_else(|_| panic!("completed test loop did not transition"));
	let exited = exited_loop.exit_recoverable().unwrap();
	let bundle = exited.bundle_identity();
	let (_backend, _metrics, _outputs, journal) = exited.into_parts();
	let error = post_exit_inference_failure(
		InferenceExecutionError::MissingPredictionOutput,
		run,
		bundle,
		journal,
	);

	let InferenceExecutionError::PostExitValidation { source, failure } = error else {
		panic!("post-exit validation produced the wrong inference error")
	};
	assert!(matches!(
		*source,
		InferenceExecutionError::MissingPredictionOutput
	));
	assert_eq!(failure.run(), run);
	assert_eq!(failure.bundle(), bundle);
	assert!(failure.cleanup_completed());
	assert!(matches!(
		failure.journal().unwrap().logical_events().last(),
		Some(recipe_executor::LogicalEvent::Exited { run: exited }) if *exited == run
	));
}

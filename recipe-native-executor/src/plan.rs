use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recipe_core::{
	ArtifactId, ArtifactIdentity, ByteCount, DeviceId, FinalizedBundle, InitDataImage, SubmissionSlots, Task, TaskId,
	TaskKind, ValueId,
};
use recipe_kernel::{ArtifactDigest, BufferAccess, KernelAbi, KernelArgument};

use crate::{Error, Result};

const CUDA_BACKEND: &str = "nvidia-cuda-driver";
const CUDA_ABI: &str = "elf64-cubin";
const HSA_BACKEND: &str = "amd-rocr-hsa";
const HSA_ABI_PREFIX: &str = "elf64-amdgpu-code-object-v";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeArtifactKind {
	Cuda {
		identity: recipe_cuda::ArtifactIdentity,
	},
	Hsa {
		target_id: String,
		code_object_version: u8,
	},
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeImage {
	bytes: Arc<[u8]>,
	digest: ArtifactDigest,
}

impl RuntimeImage {
	#[must_use]
	pub fn new(bytes: Arc<[u8]>) -> Self {
		let digest = ArtifactDigest::of(&bytes);
		Self { bytes, digest }
	}

	#[must_use]
	pub fn bytes(&self) -> &Arc<[u8]> {
		&self.bytes
	}

	#[must_use]
	pub const fn digest(&self) -> ArtifactDigest {
		self.digest
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeArtifact {
	pub(crate) id: ArtifactId,
	pub(crate) bytes: Arc<[u8]>,
	pub(crate) digest: ArtifactDigest,
	pub(crate) abi: KernelAbi,
	pub(crate) kind: RuntimeArtifactKind,
}

impl RuntimeArtifact {
	#[must_use]
	pub fn new(id: ArtifactId, bytes: Arc<[u8]>, abi: KernelAbi, kind: RuntimeArtifactKind) -> Self {
		Self::from_image(id, RuntimeImage::new(bytes), abi, kind)
	}

	#[must_use]
	pub fn from_image(id: ArtifactId, image: RuntimeImage, abi: KernelAbi, kind: RuntimeArtifactKind) -> Self {
		Self {
			id,
			bytes: image.bytes,
			digest: image.digest,
			abi,
			kind,
		}
	}

	#[must_use]
	pub const fn id(&self) -> ArtifactId {
		self.id
	}

	#[must_use]
	pub fn bytes(&self) -> &Arc<[u8]> {
		&self.bytes
	}

	#[must_use]
	pub const fn digest(&self) -> ArtifactDigest {
		self.digest
	}

	#[must_use]
	pub const fn abi(&self) -> &KernelAbi {
		&self.abi
	}

	#[must_use]
	pub const fn kind(&self) -> &RuntimeArtifactKind {
		&self.kind
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactContract {
	pub(crate) identity: ArtifactIdentity,
	pub(crate) runtime: RuntimeArtifact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlannedSubmission {
	pub(crate) task: TaskId,
	pub(crate) device: DeviceId,
	pub(crate) slots: SubmissionSlots,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InitImageContract {
	pub(crate) device: DeviceId,
	pub(crate) image: ValueId,
	pub(crate) bytes: ByteCount,
}

impl From<&InitDataImage> for InitImageContract {
	fn from(manifest: &InitDataImage) -> Self {
		Self {
			device: manifest.device,
			image: manifest.image,
			bytes: manifest.bytes,
		}
	}
}

#[derive(Clone, Debug)]
pub(crate) struct ExecutionPlan {
	artifacts: BTreeMap<ArtifactId, ArtifactContract>,
	submissions: BTreeMap<TaskId, PlannedSubmission>,
	devices: BTreeSet<DeviceId>,
}

impl ExecutionPlan {
	pub(crate) fn validate(bundle: &FinalizedBundle, runtime_artifacts: Vec<RuntimeArtifact>) -> Result<Self> {
		let devices = bundle
			.arena_layouts()
			.iter()
			.map(|layout| layout.device)
			.collect::<BTreeSet<_>>();
		Self::validate_scoped(bundle, runtime_artifacts, devices, None)
	}

	pub(crate) fn validate_partition(
		bundle: &FinalizedBundle,
		runtime_artifacts: Vec<RuntimeArtifact>,
		devices: BTreeSet<DeviceId>,
		tasks: &BTreeSet<TaskId>,
	) -> Result<Self> {
		Self::validate_scoped(bundle, runtime_artifacts, devices, Some(tasks))
	}

	fn validate_scoped(
		bundle: &FinalizedBundle,
		runtime_artifacts: Vec<RuntimeArtifact>,
		devices: BTreeSet<DeviceId>,
		tasks: Option<&BTreeSet<TaskId>>,
	) -> Result<Self> {
		let mut runtime_by_id = BTreeMap::new();
		for runtime in runtime_artifacts {
			let id = runtime.id;
			ensure(
				runtime_by_id.insert(id, runtime).is_none(),
				Error::DuplicateArtifact { artifact: id },
			)?;
		}

		let selected_artifacts = tasks.map(|tasks| {
			bundle.tasks()
				.iter()
				.filter(|task| tasks.contains(&task.id))
				.filter_map(|task| match &task.kind {
					TaskKind::Calculation(calculation) => Some(calculation.artifact),
					TaskKind::Transfer(_) | TaskKind::Metric(_) => None,
				})
				.collect::<BTreeSet<_>>()
		});
		let mut artifacts = BTreeMap::new();
		for identity in bundle
			.artifacts()
			.iter()
			.filter(|identity| match &selected_artifacts {
				Some(selected) => selected.contains(&identity.id),
				None => true,
			}) {
			let runtime = runtime_by_id
				.remove(&identity.id)
				.ok_or(Error::MissingArtifact {
					artifact: identity.id,
				})?;
			validate_artifact_contract(bundle, identity, &runtime)?;
			artifacts.insert(
				identity.id,
				ArtifactContract {
					identity: identity.clone(),
					runtime,
				},
			);
		}
		reject_unexpected_artifact(runtime_by_id.keys().next().copied())?;

		let submissions = plan_submissions(bundle)?;
		for task in tasks.into_iter().flatten() {
			let submission = submissions.get(task).ok_or(Error::Protocol {
				task: *task,
				detail: "partition task has no immutable native submission",
			})?;
			ensure(
				devices.contains(&submission.device),
				Error::MissingDevice {
					device: submission.device,
				},
			)?;
		}
		Ok(Self {
			artifacts,
			submissions,
			devices,
		})
	}

	pub(crate) fn runtime_artifacts(&self) -> impl ExactSizeIterator<Item = &RuntimeArtifact> {
		self.artifacts.values().map(|contract| &contract.runtime)
	}

	pub(crate) fn artifact_contract(&self, artifact: ArtifactId) -> Option<&ArtifactContract> {
		self.artifacts.get(&artifact)
	}

	pub(crate) fn submission(&self, task: TaskId) -> Option<PlannedSubmission> {
		self.submissions.get(&task).copied()
	}

	pub(crate) fn devices(&self) -> impl ExactSizeIterator<Item = DeviceId> + '_ {
		self.devices.iter().copied()
	}
}

pub(crate) fn validate_artifact_contract(
	bundle: &FinalizedBundle,
	identity: &ArtifactIdentity,
	runtime: &RuntimeArtifact,
) -> Result<()> {
	validate_runtime_artifact(identity, runtime)?;
	let mismatch = |detail: String| Error::ArtifactMismatch {
		artifact: identity.id,
		detail,
	};

	for task in bundle.tasks() {
		let TaskKind::Calculation(calculation) = &task.kind else {
			continue;
		};
		match calculation.artifact == identity.id {
			true => {
				ensure(
					calculation.kernel_template == identity.kernel_template,
					mismatch(format!(
						"task {} names kernel template {} instead of {}",
						task.id, calculation.kernel_template, identity.kernel_template
					)),
				)?;
				validate_calculation_abi(bundle, task, &runtime.abi, identity.id)?;
			}
			false => continue,
		}
	}
	Ok(())
}

pub(crate) fn validate_runtime_artifact(identity: &ArtifactIdentity, runtime: &RuntimeArtifact) -> Result<()> {
	let mismatch = |detail: String| Error::ArtifactMismatch {
		artifact: identity.id,
		detail,
	};
	ensure(
		runtime.id == identity.id,
		mismatch(format!(
			"runtime id {} differs from manifest id {}",
			runtime.id, identity.id
		)),
	)?;
	let digest = runtime.digest.bytes();
	ensure(
		digest == identity.digest.bytes(),
		mismatch("image SHA-256 differs from finalized identity".to_owned()),
	)?;
	ensure(
		runtime.abi.entry_symbol == identity.entry_symbol.as_str(),
		mismatch(format!(
			"ABI entry {:?} differs from finalized entry {:?}",
			runtime.abi.entry_symbol,
			identity.entry_symbol.as_str()
		)),
	)?;
	ensure(
		identity.format.as_str() == identity.target.abi.as_str(),
		mismatch(format!(
			"artifact format {:?} differs from target ABI {:?}",
			identity.format.as_str(),
			identity.target.abi.as_str()
		)),
	)?;
	ensure(
		runtime.abi.workgroup_lanes != 0
			&& runtime.abi.workgroup_lanes <= identity.resources.maximum_workgroup_lanes,
		mismatch(format!(
			"ABI workgroup size {} exceeds finalized maximum {}",
			runtime.abi.workgroup_lanes, identity.resources.maximum_workgroup_lanes
		)),
	)?;
	validate_target(identity, runtime, digest)?;
	Ok(())
}

fn validate_target(identity: &ArtifactIdentity, runtime: &RuntimeArtifact, digest: [u8; 32]) -> Result<()> {
	let mismatch = |detail: String| Error::ArtifactMismatch {
		artifact: identity.id,
		detail,
	};
	match &runtime.kind {
		RuntimeArtifactKind::Cuda { identity: driver } => {
			ensure(
				identity.target.backend.as_str() == CUDA_BACKEND && identity.target.abi.as_str() == CUDA_ABI,
				mismatch("CUDA image is paired with a non-CUDA finalized target".to_owned()),
			)?;
			ensure(
				identity.target.architecture.as_str() == driver.target.to_string(),
				mismatch(format!(
					"CUDA target {:?} differs from driver identity {}",
					identity.target.architecture.as_str(),
					driver.target
				)),
			)?;
			ensure(
				driver.sha256 == digest,
				mismatch("CUDA driver identity digest differs from image".to_owned()),
			)
		}
		RuntimeArtifactKind::Hsa {
			target_id,
			code_object_version,
		} => {
			let expected_abi = format!("{HSA_ABI_PREFIX}{code_object_version}");
			ensure(
				identity.target.backend.as_str() == HSA_BACKEND && identity.target.abi.as_str() == expected_abi,
				mismatch("HSACO image is paired with a non-HSA finalized target".to_owned()),
			)?;
			ensure(
				identity.target.architecture.as_str() == target_id,
				mismatch(format!(
					"HSACO target {:?} differs from runtime target {target_id:?}",
					identity.target.architecture.as_str()
				)),
			)
		}
	}
}

fn validate_calculation_abi(
	bundle: &FinalizedBundle,
	task: &Task,
	abi: &KernelAbi,
	artifact: ArtifactId,
) -> Result<()> {
	let mismatch = |detail: String| Error::ArtifactMismatch { artifact, detail };
	let TaskKind::Calculation(calculation) = &task.kind else {
		return Err(mismatch(format!("task {} is not a calculation", task.id)));
	};
	let (expected_elements, expected_operands) = match bundle.artifact_build(calculation.artifact) {
		Some(build) => {
			ensure(
				build.kernel_template == calculation.kernel_template,
				mismatch(format!(
					"task {} stage identity differs from its finalized build recipe",
					task.id
				)),
			)?;
			ensure(
				abi.workgroup_lanes == build.dispatch.workgroup_lanes,
				mismatch(format!(
					"task {} ABI workgroup width {} differs from finalized build width {}",
					task.id, abi.workgroup_lanes, build.dispatch.workgroup_lanes
				)),
			)?;
			let operands = build
				.bindings
				.iter()
				.filter(|binding| build.fault_flag != Some(binding.value) && binding.access.reads())
				.map(|binding| {
					(
						binding.dtype,
						binding.view.storage_bytes,
						BufferAccess::Read,
					)
				})
				.chain(
					build.bindings
						.iter()
						.filter(|binding| {
							build.fault_flag != Some(binding.value) && binding.access.writes()
						})
						.map(|binding| {
							(
								binding.dtype,
								binding.view.storage_bytes,
								BufferAccess::Write,
							)
						}),
				)
				.collect::<Vec<_>>();
			(build.dispatch.logical_lanes, operands)
		}
		None => {
			let template = bundle.kernel(calculation.kernel_template).ok_or_else(|| {
				mismatch(format!(
					"task {} has neither a finalized build recipe nor kernel template",
					task.id
				))
			})?;
			let operands = template
				.inputs
				.iter()
				.map(|input| (input.dtype, input.access.storage_bytes, BufferAccess::Read))
				.chain(template.outputs.iter().map(|output| {
					(
						output.dtype,
						output.access.storage_bytes,
						BufferAccess::Write,
					)
				}))
				.collect::<Vec<_>>();
			(template.index_space.elements().get(), operands)
		}
	};
	ensure(
		abi.elements.get() == expected_elements,
		mismatch(format!(
			"task {} ABI element count {} differs from finalized stage count {}",
			task.id,
			abi.elements.get(),
			expected_elements
		)),
	)?;
	let buffer_arguments = calculation
		.inputs
		.len()
		.checked_add(calculation.outputs.len())
		.ok_or_else(|| mismatch(format!("task {} argument count overflowed", task.id)))?;
	let run_id_arguments = abi
		.arguments
		.iter()
		.filter(|argument| matches!(argument, KernelArgument::RunId))
		.count();
	let loop_iteration_arguments = abi
		.arguments
		.iter()
		.filter(|argument| matches!(argument, KernelArgument::LoopIteration))
		.count();
	ensure(
		run_id_arguments <= 1,
		mismatch(format!(
			"task {} ABI repeats its dynamic run-id argument",
			task.id
		)),
	)?;
	ensure(
		loop_iteration_arguments <= 1,
		mismatch(format!(
			"task {} ABI repeats its loop-iteration argument",
			task.id
		)),
	)?;
	let expected_arguments = buffer_arguments
		.checked_add(usize::from(calculation.fault_flag.is_some()))
		.ok_or_else(|| mismatch(format!("task {} argument count overflowed", task.id)))?;
	let expected_arguments = expected_arguments
		.checked_add(run_id_arguments)
		.ok_or_else(|| mismatch(format!("task {} argument count overflowed", task.id)))?;
	let expected_arguments = expected_arguments
		.checked_add(loop_iteration_arguments)
		.ok_or_else(|| mismatch(format!("task {} argument count overflowed", task.id)))?;
	let expected_arguments = expected_arguments
		.checked_add(1)
		.ok_or_else(|| mismatch(format!("task {} argument count overflowed", task.id)))?;
	ensure(
		abi.arguments.len() == expected_arguments,
		mismatch(format!(
			"task {} has {} ABI arguments, expected {expected_arguments}",
			task.id,
			abi.arguments.len()
		)),
	)?;

	let locations = calculation
		.inputs
		.iter()
		.chain(&calculation.outputs)
		.map(|value| {
			bundle.value_location(*value)
				.copied()
				.ok_or_else(|| mismatch(format!("task {} has no resolved value {value}", task.id)))
		})
		.collect::<Result<Vec<_>>>()?;
	ensure(
		locations.len() == expected_operands.len(),
		mismatch(format!(
			"task {} operand count differs from finalized stage mappings",
			task.id
		)),
	)?;
	for (index, ((location, (expected_dtype, expected_bytes, expected_access)), argument)) in locations
		.iter()
		.zip(&expected_operands)
		.zip(&abi.arguments)
		.enumerate()
	{
		let KernelArgument::Buffer {
			access,
			dtype,
			alignment,
		} = argument
		else {
			return Err(mismatch(format!(
				"task {} argument {index} is not a buffer",
				task.id
			)));
		};
		ensure(
			*dtype == location.dtype && *dtype == *expected_dtype,
			mismatch(format!(
				"task {} argument {index} dtype {:?} differs from resolved {:?} or template {:?}",
				task.id, dtype, location.dtype, expected_dtype
			)),
		)?;
		ensure(
			*access == *expected_access,
			mismatch(format!(
				"task {} argument {index} access {:?} differs from finalized {:?}",
				task.id, access, expected_access
			)),
		)?;
		ensure(
			*alignment != 0
				&& alignment.is_power_of_two()
				&& location
					.arena_offset
					.get()
					.is_multiple_of(u64::from(*alignment)),
			mismatch(format!(
				"task {} argument {index} does not meet {}-byte alignment",
				task.id, alignment
			)),
		)?;
		ensure(
			location.bytes == *expected_bytes,
			mismatch(format!(
				"task {} argument {index} has {} backing bytes, finalized stage requires {}",
				task.id,
				location.bytes.get(),
				expected_bytes.get()
			)),
		)?;
	}
	match calculation.fault_flag {
		Some(fault_flag) => {
			let location = bundle.value_location(fault_flag).ok_or_else(|| {
				mismatch(format!(
					"task {} fault flag {fault_flag} has no finalized location",
					task.id
				))
			})?;
			ensure(
				location.device == calculation.device
					&& location.dtype == recipe_core::DType::I32
					&& location.bytes == recipe_core::ByteCount::new(4)
					&& location.arena_offset.get().is_multiple_of(4),
				mismatch(format!(
					"task {} fault flag {fault_flag} is not an aligned device int32",
					task.id
				)),
			)?;
			ensure(
				matches!(
					abi.arguments.get(buffer_arguments),
					Some(KernelArgument::FaultFlag)
				),
				mismatch(format!(
					"task {} ABI omits its finalized fault flag",
					task.id
				)),
			)
		}
		None => Ok(()),
	}?;
	let run_id_index = buffer_arguments + usize::from(calculation.fault_flag.is_some());
	match run_id_arguments {
		0 => Ok(()),
		1 => ensure(
			matches!(abi.arguments.get(run_id_index), Some(KernelArgument::RunId)),
			mismatch(format!(
				"task {} ABI run id is not in its canonical suffix position",
				task.id
			)),
		),
		2.. => Err(mismatch(format!(
			"task {} ABI repeats its dynamic run-id argument",
			task.id
		))),
	}?;
	let loop_iteration_index = run_id_index
		.checked_add(run_id_arguments)
		.ok_or_else(|| mismatch(format!("task {} argument index overflowed", task.id)))?;
	match loop_iteration_arguments {
		0 => Ok(()),
		1 => ensure(
			matches!(
				abi.arguments.get(loop_iteration_index),
				Some(KernelArgument::LoopIteration)
			),
			mismatch(format!(
				"task {} ABI loop iteration is not in its canonical suffix position",
				task.id
			)),
		),
		2.. => Err(mismatch(format!(
			"task {} ABI repeats its loop-iteration argument",
			task.id
		))),
	}?;
	ensure(
		matches!(abi.arguments.last(), Some(KernelArgument::ElementCount)),
		mismatch(format!(
			"task {} ABI does not end in element count",
			task.id
		)),
	)
}

fn plan_submissions(bundle: &FinalizedBundle) -> Result<BTreeMap<TaskId, PlannedSubmission>> {
	let mut result = BTreeMap::new();

	for task in bundle.tasks() {
		let (device, slots) = match &task.kind {
			TaskKind::Calculation(calculation) => (calculation.device, calculation.submission),
			TaskKind::Transfer(transfer) => (
				transfer_submission_device(task.id, transfer.source, transfer.destination)?,
				transfer.submission,
			),
			TaskKind::Metric(metric) => {
				let location = bundle
					.value_location(metric.value)
					.ok_or(Error::ValueMismatch {
						value: metric.value,
						detail: "metric value has no finalized arena location",
					})?;
				(location.device, metric.submission)
			}
		};
		validate_slots(bundle, task.id, device, slots)?;
		result.insert(
			task.id,
			PlannedSubmission {
				task: task.id,
				device,
				slots,
			},
		);
	}
	Ok(result)
}

fn transfer_submission_device(
	task: TaskId,
	source: recipe_core::TransferEndpoint,
	destination: recipe_core::TransferEndpoint,
) -> Result<DeviceId> {
	match source {
		recipe_core::TransferEndpoint::Device { device, .. } => Ok(device),
		recipe_core::TransferEndpoint::External => match destination {
			recipe_core::TransferEndpoint::Device { device, .. } => Ok(device),
			recipe_core::TransferEndpoint::External => Err(Error::Protocol {
				task,
				detail: "external-to-external transfer has no native device",
			}),
		},
	}
}

fn validate_slots(bundle: &FinalizedBundle, task: TaskId, device: DeviceId, slots: SubmissionSlots) -> Result<()> {
	ensure(
		bundle.resources()
			.queue(slots.queue)
			.is_some_and(|slot| slot.device == device),
		Error::MissingQueue {
			task,
			queue: slots.queue,
		},
	)?;
	ensure(
		bundle.resources()
			.completion(slots.completion)
			.is_some_and(|slot| slot.device == device),
		Error::MissingCompletion {
			task,
			completion: slots.completion,
		},
	)
}

fn ensure(valid: bool, error: Error) -> Result<()> {
	match valid {
		true => Ok(()),
		false => Err(error),
	}
}

fn reject_unexpected_artifact(artifact: Option<ArtifactId>) -> Result<()> {
	match artifact {
		Some(artifact) => Err(Error::UnexpectedArtifact { artifact }),
		None => Ok(()),
	}
}

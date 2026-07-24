use recipe_executor::{BackendWork, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus};

use crate::{Error, Result};

pub(crate) fn record(batch: &mut PhysicalCallBatch, call: PhysicalCall) -> Result<()> {
	batch.try_push(call)
		.map_err(|_| Error::PhysicalAccountingOverflow)
}

#[must_use]
pub(crate) fn submission_call(work: &BackendWork<'_>) -> PhysicalCall {
	match work {
		BackendWork::InitAdmission(work) => PhysicalCall::AdmissionChunk {
			task: work.task,
			device: work.destination.device,
			bytes: work.bytes,
			chunk_index: 0,
		},
		BackendWork::Calculation(work) => PhysicalCall::SubmitCalculation { task: work.task },
		BackendWork::InternalTransfer(work) => PhysicalCall::SubmitInternalTransfer { task: work.task },
		BackendWork::Metric(work) => PhysicalCall::SubmitMetric {
			task: work.task,
			slot: work.slot,
		},
		BackendWork::ExitTransfer(work) => PhysicalCall::SubmitExitTransfer { task: work.task },
	}
}

#[must_use]
pub(crate) const fn completion_poll_call(task: recipe_core::TaskId, status: PhysicalPollStatus) -> PhysicalCall {
	PhysicalCall::Poll { task, status }
}

#[cfg(test)]
mod tests {
	use super::*;
	use recipe_core::{
		ArenaObjectId, ByteCount, ByteOffset, CompletionSlotId, DType, DeviceId, MetricId, MetricSlotId,
		QueueSlotId, ResolvedValueLocation, SubmissionSlots, TaskId, ValueId,
	};
	use recipe_executor::{BackendWork, MetricWork};

	#[test]
	fn one_logical_metric_submission_is_one_physical_submit_record() {
		let work = BackendWork::Metric(MetricWork {
			task: TaskId::new(9),
			iteration: recipe_core::LoopIterations::ONE
				.iteration(0)
				.expect("one loop iteration has index zero"),
			metric: MetricId::new(3),
			slot: MetricSlotId::new(4),
			purpose: recipe_core::MetricPurpose::User,
			value: ResolvedValueLocation {
				value: ValueId::new(8),
				dtype: DType::F32,
				device: DeviceId::new(2),
				bytes: ByteCount::new(4),
				object: ArenaObjectId::new(7),
				object_offset: ByteOffset::new(0),
				arena_offset: ByteOffset::new(16),
			},
			submission: SubmissionSlots {
				queue: QueueSlotId::new(5),
				completion: CompletionSlotId::new(6),
			},
		});
		assert_eq!(
			submission_call(&work),
			PhysicalCall::SubmitMetric {
				task: TaskId::new(9),
				slot: MetricSlotId::new(4),
			}
		);
	}
}

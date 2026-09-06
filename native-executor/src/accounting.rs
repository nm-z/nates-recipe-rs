use recipe_executor::{BackendWork, PhysicalCall, PhysicalCallBatch, PhysicalPollStatus};

use crate::Result;

pub(crate) fn record(batch: &mut PhysicalCallBatch, call: PhysicalCall) -> Result<()> {
	let result = batch.try_push(call);
	debug_assert!(result.is_ok());
	Ok(())
}

#[must_use]
pub(crate) fn submission_call(work: &BackendWork<'_>) -> PhysicalCall {
	match work {
		BackendWork::InitAdmission(work) => {
			PhysicalCall::AdmissionChunk {
				task: work.task,
				device: work.destination.device,
				bytes: work.bytes,
				chunk_index: 0,
			}
		}
		BackendWork::Calculation(work) => PhysicalCall::SubmitCalculation { task: work.task },
		BackendWork::InternalTransfer(work) => PhysicalCall::SubmitInternalTransfer { task: work.task },
		BackendWork::Metric(work) => {
			PhysicalCall::SubmitMetric {
				task: work.task,
				slot: work.slot,
			}
		}
		BackendWork::ExitTransfer(work) => PhysicalCall::SubmitExitTransfer { task: work.task },
	}
}

#[must_use]
pub(crate) const fn completion_poll_call(task: recipe_core::TaskId, status: PhysicalPollStatus) -> PhysicalCall { PhysicalCall::Poll { task, status } }

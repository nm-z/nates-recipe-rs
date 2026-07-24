use std::collections::BTreeMap;

use recipe_core::{LoopIteration, MetricId, MetricPurpose, MetricSlot, MetricSlotId, Task, TaskId, TaskKind};

use crate::error::{ExecutorError, Result};

/// Calculation-domain value copied into a preallocated metric slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricValue {
	F32(f32),
	I32(i32),
}

/// One completed metric emission.
#[derive(Clone, Debug, PartialEq)]
pub struct MetricSample {
	pub sequence: u64,
	pub iteration: LoopIteration,
	pub task: TaskId,
	pub slot: MetricSlotId,
	pub metric: MetricId,
	pub value: MetricValue,
}

/// Bounded, nonblocking, capacity-one mailbox per finalized metric slot.
///
/// Publishing never waits. A new value replaces an older unconsumed value in
/// the same slot while values for other metrics remain independent.
#[derive(Clone, Debug)]
pub struct MetricMailbox {
	slots: BTreeMap<MetricSlotId, SlotState>,
	next_sequence: u64,
}

#[derive(Clone, Debug)]
struct SlotState {
	metric: MetricId,
	sample: Option<MetricSample>,
}

impl MetricMailbox {
	pub(crate) fn new(slots: &[MetricSlot], tasks: &[Task]) -> Self {
		Self {
			slots: slots
				.iter()
				.filter(|slot| {
					tasks.iter().any(|task| {
						matches!(
							&task.kind,
							TaskKind::Metric(metric)
								if metric.slot == slot.id && metric.purpose == MetricPurpose::User
						)
					})
				})
				.map(|slot| {
					(
						slot.id,
						SlotState {
							metric: slot.metric,
							sample: None,
						},
					)
				})
				.collect(),
			next_sequence: 0,
		}
	}

	pub(crate) fn publish(
		&mut self,
		iteration: LoopIteration,
		task: TaskId,
		slot: MetricSlotId,
		metric: MetricId,
		value: MetricValue,
	) -> Result<bool> {
		let state = self
			.slots
			.get_mut(&slot)
			.ok_or(ExecutorError::BackendProtocol {
				task,
				detail: "metric completion names an unplanned slot",
			})?;
		if state.metric != metric {
			return Err(ExecutorError::BackendProtocol {
				task,
				detail: "metric completion names the wrong metric for its slot",
			});
		}
		let sequence = self.next_sequence;
		self.next_sequence = self
			.next_sequence
			.checked_add(1)
			.ok_or(ExecutorError::MetricSequenceOverflow)?;
		let replaced = state
			.sample
			.replace(MetricSample {
				sequence,
				iteration,
				task,
				slot,
				metric,
				value,
			})
			.is_some();
		Ok(replaced)
	}

	/// Takes the latest value without waiting.
	pub fn try_take(&mut self, slot: MetricSlotId) -> Option<MetricSample> {
		self.slots.get_mut(&slot)?.sample.take()
	}

	#[must_use]
	pub fn pending_len(&self) -> usize {
		self.slots
			.values()
			.filter(|state| state.sample.is_some())
			.count()
	}

	#[must_use]
	pub fn capacity(&self) -> usize {
		self.slots.len()
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.pending_len() == 0
	}
}

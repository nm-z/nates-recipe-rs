use crate::OperationDescriptor;

use super::{Emitter, FamilyDispatch, MaterializationRequest};

pub(super) fn supports(_descriptor: OperationDescriptor) -> bool {
	false
}

pub(super) fn dispatch(_request: &MaterializationRequest<'_>, _emitter: &mut Emitter<'_>) -> FamilyDispatch {
	FamilyDispatch::NotOwned
}

use super::{
	Emitter, FamilyDispatch, MaterializationRequest, emit_radix_two_fft, emit_triangular_solve, operation_error, };
use crate::{OperationDescriptor, OperationErrorKind};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_fft_c2c_1d", "gpu-core/src/linalg.rs:867"),
	("gpu_tri_solve", "gpu-core/src/kernels.rs:2263"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool { OPERATIONS.contains(&(descriptor.symbol, descriptor.source)) }

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	if !supports(request.descriptor) { return FamilyDispatch::NotOwned; }
	let result = match request.descriptor.symbol {
		"gpu_fft_c2c_1d" => emit_radix_two_fft(request, emitter),
		"gpu_tri_solve" => emit_triangular_solve(request, emitter),
		symbol => { Err(operation_error( request.descriptor.id, OperationErrorKind::GraphMaterializationFailed,
				format!("solver/FFT dispatch is incomplete for {symbol}"),
			)) } }; FamilyDispatch::Owned(result) }

use super::{Emitter, FamilyDispatch, MaterializationRequest, emit_adagrad_update, emit_adam_update, emit_batch_normalization_backward, emit_batch_normalization_forward, emit_batch_normalization_inference, emit_batch_normalization_running_update, emit_gradient_clip_norm, emit_lamb_phase_one, emit_lamb_phase_two, emit_layer_normalization, emit_layer_normalization_backward, emit_lion_update, emit_momentum_update, emit_nadam_update, emit_rms_normalization, emit_rms_normalization_backward, emit_rmsprop_update, operation_error};
use crate::{OperationDescriptor, OperationErrorKind};

const OPERATIONS: &[(&str, &str)] = &[
	("gpu_adagrad_update", "gpu-core/src/optimizers.rs:149"),
	("gpu_adam_update", "gpu-core/src/kernels.rs:6421"),
	("gpu_adamw_update", "gpu-core/src/kernels.rs:6454"),
	("gpu_batchnorm_backward", "gpu-core/src/kernels.rs:6388"),
	("gpu_batchnorm_forward", "gpu-core/src/kernels.rs:6322"),
	("gpu_batchnorm_inference", "gpu-core/src/kernels.rs:6355"),
	("gpu_bn_update_running", "gpu-core/src/attention.rs:454"),
	("gpu_grad_clip_norm", "gpu-core/src/kernels.rs:6670"),
	("gpu_lamb_phase1", "gpu-core/src/optimizers.rs:174"),
	("gpu_lamb_phase2", "gpu-core/src/optimizers.rs:217"),
	("gpu_layernorm_backward_f32", "gpu-core/src/nn_f32.rs:339"),
	(
		"gpu_layernorm_backward_full_into",
		"gpu-core/src/kernels.rs:3491",
	),
	("gpu_layernorm_f32", "gpu-core/src/nn_f32.rs:313"),
	("gpu_layernorm_into", "gpu-core/src/kernels.rs:3205"),
	("gpu_layernorm_opt_into", "gpu-core/src/kernels.rs:3233"),
	("gpu_lion_update", "gpu-core/src/optimizers.rs:243"),
	("gpu_momentum_update", "gpu-core/src/optimizers.rs:95"),
	("gpu_nadam_update", "gpu-core/src/optimizers.rs:273"),
	("gpu_rmsnorm", "gpu-core/src/attention.rs:287"),
	("gpu_rmsnorm_backward", "gpu-core/src/attention.rs:313"),
	("gpu_rmsnorm_f64", "gpu-core/src/infer_ops.rs:335"),
	("gpu_rmsnorm_f64_nogamma", "gpu-core/src/infer_ops.rs:360"),
	("gpu_rmsprop_update", "gpu-core/src/optimizers.rs:121"),
];

pub(super) fn supports(descriptor: OperationDescriptor) -> bool { OPERATIONS.contains(&(descriptor.symbol, descriptor.source)) }

pub(super) fn dispatch(request: &MaterializationRequest<'_>, emitter: &mut Emitter<'_>) -> FamilyDispatch {
	if !supports(request.descriptor) {
		return FamilyDispatch::NotOwned;
	}
	let result = match request.descriptor.symbol {
		"gpu_adagrad_update" => emit_adagrad_update(request, emitter),
		"gpu_adam_update" => emit_adam_update(request, emitter, false),
		"gpu_adamw_update" => emit_adam_update(request, emitter, true),
		"gpu_batchnorm_backward" => emit_batch_normalization_backward(request, emitter),
		"gpu_batchnorm_forward" => emit_batch_normalization_forward(request, emitter),
		"gpu_batchnorm_inference" => emit_batch_normalization_inference(request, emitter),
		"gpu_bn_update_running" => emit_batch_normalization_running_update(request, emitter),
		"gpu_grad_clip_norm" => emit_gradient_clip_norm(request, emitter),
		"gpu_lamb_phase1" => emit_lamb_phase_one(request, emitter),
		"gpu_lamb_phase2" => emit_lamb_phase_two(request, emitter),
		"gpu_layernorm_f32" | "gpu_layernorm_into" | "gpu_layernorm_opt_into" => emit_layer_normalization(request, emitter),
		"gpu_layernorm_backward_f32" | "gpu_layernorm_backward_full_into" => emit_layer_normalization_backward(request, emitter),
		"gpu_lion_update" => emit_lion_update(request, emitter),
		"gpu_momentum_update" => emit_momentum_update(request, emitter),
		"gpu_nadam_update" => emit_nadam_update(request, emitter),
		"gpu_rmsnorm" | "gpu_rmsnorm_f64" | "gpu_rmsnorm_f64_nogamma" => emit_rms_normalization(request, emitter),
		"gpu_rmsnorm_backward" => emit_rms_normalization_backward(request, emitter),
		"gpu_rmsprop_update" => emit_rmsprop_update(request, emitter),
		symbol => {
			Err(operation_error(
				request.descriptor.id,
				OperationErrorKind::GraphMaterializationFailed,
				format!("optimizer/normalization dispatch is incomplete for {symbol}"),
			))
		}
	};
	FamilyDispatch::Owned(result)
}

use anyhow::Context;
use gpu_core::kernels;
use gpu_core::memory::GpuBuffer;
use recipe_infer::{Loss, Scratch};
use std::sync::atomic::{AtomicUsize, Ordering};

pub static INTERRUPTED: AtomicUsize = AtomicUsize::new(0);

pub extern "C" fn on_sigint(_sig: i32) {
	let Some(_second) = Some(()).filter(|_probe| INTERRUPTED.swap(1, Ordering::SeqCst) != 0)
	else {
		return;
	};
	gpu_core::sys::exit_now(130);
}

pub struct StepScalars {
	pub neg_lr: GpuBuffer,
	pub inv_n: GpuBuffer,
	pub two_inv_n: GpuBuffer,
	pub zero: GpuBuffer,
}

pub fn loss_grad_into(
	lossfn: Loss,
	out: &GpuBuffer,
	y: &GpuBuffer,
	da: &GpuBuffer,
	n: usize,
	total: usize,
	sc: &Scratch,
	ss: &StepScalars,
) -> anyhow::Result<()> {
	match lossfn {
		Loss::Mse => {
			kernels::gpu_sub_scale_into(out, y, &ss.two_inv_n, total, da).context("mse")?;
		}
		Loss::Mae => {
			kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).context("mae sub")?;
			kernels::gpu_sign_into(da, total, da).context("mae sign")?;
			kernels::gpu_scale_inplace(&ss.inv_n, total, da).context("mae scale")?;
		}
		Loss::Huber => {
			kernels::gpu_sub_scale_into(out, y, &sc.c_one, total, da).context("huber sub")?;
			kernels::gpu_clamp_into(da, &sc.c_neg_one, &sc.c_one, total, da)
				.context("huber clamp")?;
			kernels::gpu_scale_inplace(&ss.inv_n, total, da).context("huber scale")?;
		}
		Loss::Ce => {
			let k = total / n;
			kernels::gpu_softmax_rows_into(out, n, k, da).context("ce softmax")?;
			kernels::gpu_sub_scale_into(da, y, &ss.inv_n, total, da).context("ce grad")?;
		}
		Loss::Bce => {
			kernels::gpu_bce_grad_into(out, y, &ss.inv_n, total, da).context("bce")?;
		}
		Loss::Focal => {
			gpu_core::losses::gpu_focal_grad_into(
				out,
				y,
				&sc.c_focal_gamma,
				&sc.c_focal_alpha,
				&ss.inv_n,
				total,
				da,
			)
			.context("focal")?;
		}
	}
	Ok(())
}

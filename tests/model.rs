use gpu_core::kernels;
use recipe::train::StepScalars;
use recipe::{Loss, Metric, Model, Train, mse};
use recipe_infer::*;
use std::sync::LazyLock;

fn rss_bytes() -> usize {
	let statm = std::fs::read_to_string("/proc/self/statm").expect("read /proc/self/statm");
	let pages: usize = statm
		.split_whitespace()
		.nth(1)
		.expect("statm resident field")
		.parse()
		.expect("statm resident pages");
	pages * 4096
}

#[test]
fn builders_are_owned_not_leaked() {
	let path = std::env::temp_dir()
		.join(format!("recipe_builder_own_{}.arff", std::process::id()));
	std::fs::write(
		&path,
		"@relation t\n@attribute a numeric\n@attribute y numeric\n@data\n1,2\n3,4\n",
	)
	.expect("write arff");
	let p = path.to_str().expect("temp path utf8");

	let cycle = |p: &str| {
		let _m = Model::new().layer(64).relu().layer(1).loss(mse).lr(0.01);
		let _t = Train::new().epochs(10).log_every(2);
		let _d = recipe::dataset::Data::load(p).split(0.5).exclude("none").target("y");
	};
	const WARM: usize = 5_000;
	const N: usize = 50_000;
	for _ in 0..WARM {
		cycle(p);
	}
	let before = rss_bytes();
	for _ in 0..N {
		cycle(p);
	}
	let growth = rss_bytes().saturating_sub(before);
	let _ = std::fs::remove_file(&path);
	assert!(
		growth < 8 << 20,
		"builders leak: RSS grew {growth} B across {N} build cycles (owned builders keep it flat)"
	);
}

fn upload(x: &ndarray::Array2<f64>) -> (GpuBuffer, usize, usize) {
	let s = x.as_standard_layout();
	let sl = s.as_slice().expect("upload: non-contiguous");
	let b = GpuBuffer::alloc(sl.len()).expect("upload");
	b.load(sl).expect("upload");
	(b, x.nrows(), x.ncols())
}

fn consts_buf() -> GpuBuffer {
	let b = GpuBuffer::alloc(SCRATCH_CONSTS.len()).expect("consts");
	b.load(&SCRATCH_CONSTS).expect("consts");
	b
}

static CHURN: LazyLock<recipe::dataset::Dataset> = LazyLock::new(|| {
	const TRAIN: &str = "datasets/playground-series-s6e3/train.csv";
	assert!(
		std::path::Path::new(TRAIN).exists(),
		"{TRAIN} missing — it is committed via Git LFS; run `git lfs pull`",
	);
	let data = recipe::dataset::Data::load(TRAIN).target("Churn");
	data.datasets().train
});

fn step_scalars(lr: f64, n: usize) -> StepScalars {
	let inv = 1.0 / n as f64;
	StepScalars {
		neg_lr: { let __up = &[-lr]; let __ub = GpuBuffer::alloc(__up.len()).expect("neg_lr"); __ub.load(__up).expect("neg_lr"); __ub },
		inv_n: { let __up = &[inv]; let __ub = GpuBuffer::alloc(__up.len()).expect("inv_n"); __ub.load(__up).expect("inv_n"); __ub },
		two_inv_n: { let __up = &[2.0 * inv]; let __ub = GpuBuffer::alloc(__up.len()).expect("two_inv_n"); __ub.load(__up).expect("two_inv_n"); __ub },
		zero: { let __up = &[0.0]; let __ub = GpuBuffer::alloc(__up.len()).expect("zero"); __ub.load(__up).expect("zero"); __ub },
	}
}

fn attn_backward(
	p: &LayerParams,
	h: &GpuBuffer,
	da: &GpuBuffer,
	da_below: &GpuBuffer,
	n: usize,
	sc: &Scratch,
	ss: &StepScalars,
) {
	let d = p.dim;
	let heads = p.heads;
	let s = p.in_dim / d;
	let m = n * s;
	kernels::gpu_linear_backward_full_into(
		da,
		&sc.a_ctx,
		&p.wo,
		&sc.reduce_ws,
		&sc.dw_partials,
		m,
		d,
		d,
		&sc.a_dctx,
		&sc.a_gw,
		&sc.a_dbias,
	).expect("attn wo backward");
	kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wo).expect("sgd wo");
	kernels::gpu_flash_attention_backward_into(
		&sc.a_q,
		&sc.a_k,
		&sc.a_v,
		&sc.a_ctx,
		&sc.a_dctx,
		&sc.a_lse,
		n,
		s,
		d,
		heads,
		&sc.a_dsum,
		&sc.a_dq,
		&sc.a_dk,
		&sc.a_dv,
	).expect("flash attn backward");
	gpu_core::rope::gpu_rope_qk_heads_inplace(&sc.c_neg_one, &sc.c_rope_theta, m, d, heads, s, &sc.a_dq, &sc.a_dk).expect("rope backward");
	kernels::gpu_linear_backward_full_into(
		&sc.a_dq,
		h,
		&p.w,
		&sc.reduce_ws,
		&sc.dw_partials,
		m,
		d,
		d,
		da_below,
		&sc.a_gw,
		&sc.a_dbias,
	).expect("attn wq backward");
	kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.w).expect("sgd wq");
	kernels::gpu_linear_backward_full_into(
		&sc.a_dk,
		h,
		&p.wk,
		&sc.reduce_ws,
		&sc.dw_partials,
		m,
		d,
		d,
		&sc.a_dctx,
		&sc.a_gw,
		&sc.a_dbias,
	).expect("attn wk backward");
	kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wk).expect("sgd wk");
	kernels::gpu_add_inplace(&sc.a_dctx, m * d, da_below).expect("attn dh add k");
	kernels::gpu_linear_backward_full_into(
		&sc.a_dv,
		h,
		&p.wv,
		&sc.reduce_ws,
		&sc.dw_partials,
		m,
		d,
		d,
		&sc.a_dctx,
		&sc.a_gw,
		&sc.a_dbias,
	).expect("attn wv backward");
	kernels::gpu_sgd_update(&sc.a_gw, &ss.neg_lr, d * d, &p.wv).expect("sgd wv");
	kernels::gpu_add_inplace(&sc.a_dctx, m * d, da_below).expect("attn dh add v");
}

fn backward_step(
	loss: Loss,
	params: &[LayerParams],
	x: &GpuBuffer,
	ybuf: &GpuBuffer,
	n: usize,
	sc: &Scratch,
	ss: &StepScalars,
) {
	let last = params.len() - 1;
	let cc = concat_layer(params);
	let (da_cur, da_next) = (&sc.da_a, &sc.da_b);
	recipe::train::loss_grad_into(
		loss,
		&sc.acts[last],
		ybuf,
		da_cur,
		n,
		n * params[last].out_dim,
		sc,
		ss,
	)
	.expect("loss grad");
	sc.mark_bwd(params.len());
	let mut flip = false;
	for l in (0..params.len()).rev() {
		let (in_dim, out_dim) = (params[l].in_dim, params[l].out_dim);
		let m = n * out_dim;
		let da = if flip { da_next } else { da_cur };
		let da_below = if flip { da_cur } else { da_next };
		if params[l].kind == LayerKind::Embed {
			let p = &params[l];
			kernels::gpu_scale_inplace(&ss.zero, p.vocab * p.dim, &sc.embed_grad).expect("embed zero");
			kernels::gpu_scatter_add(x, da, n * p.in_dim, p.dim, &sc.embed_grad).expect("embed scatter");
			kernels::gpu_sgd_update(&sc.embed_grad, &ss.neg_lr, p.vocab * p.dim, &p.w).expect("sgd embed");
			sc.mark_bwd(l);
			flip = !flip;
			continue;
		}
		if params[l].kind == LayerKind::Attn {
			let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
			attn_backward(&params[l], a_prev, da, da_below, n, sc, ss);
			sc.mark_bwd(l);
			flip = !flip;
			continue;
		}
		if params[l].kind == LayerKind::Conv {
			let p = &params[l];
			let (cin, k, stride) = (p.conv_cin, p.conv_k, p.conv_stride);
			let lin = p.in_dim / cin;
			let cout = p.out_dim / ((lin - k) / stride + 1);
			let lout = (lin - k) / stride + 1;
			let grad = match p.act {
				Activation::Relu => { kernels::gpu_relu_backward_into(da, &sc.acts[l], m, &sc.dz).expect("relu bwd"); &sc.dz }
				Activation::Sigmoid => { kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], m, &sc.dz).expect("sigmoid bwd"); &sc.dz }
				Activation::LeakyRelu => { kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &sc.c_leaky_alpha, m, &sc.dz).expect("leaky bwd"); &sc.dz }
				Activation::PRelu => {
					kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &p.palpha, m, &sc.dz).expect("prelu bwd");
					kernels::gpu_relu_into(&sc.preact[l], m, &sc.prelu_t0).expect("prelu relu");
					kernels::gpu_copy_into(&sc.preact[l], m, &sc.prelu_t1).expect("prelu copy");
					kernels::gpu_sub_inplace(&sc.prelu_t0, m, &sc.prelu_t1).expect("prelu sub");
					kernels::gpu_mul_inplace(da, m, &sc.prelu_t1).expect("prelu mul");
					kernels::gpu_reduce_sum_cols_into(&sc.prelu_t1, &sc.reduce_ws, m, 1, &sc.prelu_scalar).expect("prelu reduce");
					kernels::gpu_sgd_update(&sc.prelu_scalar, &ss.neg_lr, 1, &p.palpha).expect("sgd prelu");
					&sc.dz
				}
				Activation::Tanh => { kernels::gpu_tanh_backward_into(da, &sc.acts[l], m, &sc.dz).expect("tanh bwd"); &sc.dz }
				Activation::Elu => { gpu_core::k_gapact::gpu_elu_backward(da, &sc.preact[l], &sc.c_elu_alpha, m, &sc.dz).expect("elu bwd"); &sc.dz }
				Activation::Selu => { gpu_core::k_gapact::gpu_selu_backward(da, &sc.preact[l], &sc.c_selu_alpha, &sc.c_selu_lambda, m, &sc.dz).expect("selu bwd"); &sc.dz }
				Activation::Silu => { kernels::gpu_silu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("silu bwd"); &sc.dz }
				Activation::Gelu => { kernels::gpu_gelu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("gelu bwd"); &sc.dz }
				Activation::Linear => da,
			};
			let a_prev = if l == 0 { x } else { &sc.acts[l - 1] };
			kernels::gpu_conv1d_backward_filter_into(
				grad, a_prev, &sc.conv_temp, &sc.reduce_ws,
				n, cin, lin, cout, k, stride, sc.conv_wg,
				&sc.dw,
			).expect("conv filter bwd");
			kernels::gpu_scale_inplace(&ss.zero, cout, &sc.db).expect("conv db zero");
			kernels::gpu_conv1d_backward_bias_into(grad, n, cout, lout, &sc.db).expect("conv bias bwd");
			if l > 0 {
				kernels::gpu_conv1d_backward_data_into(grad, &p.w, n, cin, lin, cout, k, stride, da_below).expect("conv data bwd");
			}
			kernels::gpu_sgd_update(&sc.dw, &ss.neg_lr, cout * cin * k, &p.w).expect("sgd conv w");
			kernels::gpu_sgd_update(&sc.db, &ss.neg_lr, cout, &p.b).expect("sgd conv b");
			sc.mark_bwd(l);
			flip = !flip;
			continue;
		}
		let grad = match params[l].act {
			Activation::Relu => {
				kernels::gpu_relu_backward_into(da, &sc.acts[l], m, &sc.dz).expect("relu bwd");
				&sc.dz
			}
			Activation::Sigmoid => {
				kernels::gpu_sigmoid_backward_into(da, &sc.acts[l], m, &sc.dz).expect("sigmoid bwd");
				&sc.dz
			}
			Activation::LeakyRelu => {
				kernels::gpu_leaky_relu_backward_into(
					da,
					&sc.acts[l],
					&sc.c_leaky_alpha,
					m,
					&sc.dz,
				).expect("leaky bwd");
				&sc.dz
			}
			Activation::PRelu => {
				kernels::gpu_leaky_relu_backward_into(da, &sc.acts[l], &params[l].palpha, m, &sc.dz).expect("prelu bwd");
				kernels::gpu_relu_into(&sc.preact[l], m, &sc.prelu_t0).expect("prelu relu");
				kernels::gpu_copy_into(&sc.preact[l], m, &sc.prelu_t1).expect("prelu copy");
				kernels::gpu_sub_inplace(&sc.prelu_t0, m, &sc.prelu_t1).expect("prelu sub");
				kernels::gpu_mul_inplace(da, m, &sc.prelu_t1).expect("prelu mul");
				kernels::gpu_reduce_sum_cols_into(
					&sc.prelu_t1,
					&sc.reduce_ws,
					m,
					1,
					&sc.prelu_scalar,
				).expect("prelu reduce");
				kernels::gpu_sgd_update(
					&sc.prelu_scalar,
					&ss.neg_lr,
					1,
					&params[l].palpha,
				).expect("sgd prelu");
				&sc.dz
			}
			Activation::Tanh => {
				kernels::gpu_tanh_backward_into(da, &sc.acts[l], m, &sc.dz).expect("tanh bwd");
				&sc.dz
			}
			Activation::Elu => {
				gpu_core::k_gapact::gpu_elu_backward(
					da,
					&sc.preact[l],
					&sc.c_elu_alpha,
					m,
					&sc.dz,
				).expect("elu bwd");
				&sc.dz
			}
			Activation::Selu => {
				gpu_core::k_gapact::gpu_selu_backward(
					da,
					&sc.preact[l],
					&sc.c_selu_alpha,
					&sc.c_selu_lambda,
					m,
					&sc.dz,
				).expect("selu bwd");
				&sc.dz
			}
			Activation::Silu => {
				kernels::gpu_silu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("silu bwd");
				&sc.dz
			}
			Activation::Gelu => {
				kernels::gpu_gelu_backward_into(da, &sc.preact[l], m, &sc.dz).expect("gelu bwd");
				&sc.dz
			}
			Activation::Linear => da,
		};
		let at_concat = Some(l) == cc.map(|t| t.pf);
		let a_prev = if l == 0 {
			x
		} else if at_concat {
			&sc.concat
		} else {
			&sc.acts[l - 1]
		};
		if out_dim == 1 {
			kernels::gpu_splitk_dw_into(a_prev, grad, &sc.dw_partials, n, 1, in_dim, &sc.dw).expect("splitk dw");
			kernels::gpu_reduce_sum_cols_into(grad, &sc.reduce_ws, n, 1, &sc.db).expect("db reduce");
			if l > 0 {
				kernels::gpu_dger_into(grad, &params[l].w, n, in_dim, da_below).expect("dger");
			}
		} else if l > 0 {
			kernels::gpu_linear_backward_full_into(
				grad,
				a_prev,
				&params[l].w,
				&sc.reduce_ws,
				&sc.dw_partials,
				n,
				out_dim,
				in_dim,
				da_below,
				&sc.dw,
				&sc.db,
			).expect("linear full bwd");
		} else {
			kernels::gpu_linear_backward_weights_only_into(
				grad,
				a_prev,
				&sc.reduce_ws,
				&sc.dw_partials,
				n,
				out_dim,
				in_dim,
				&sc.dw,
				&sc.db,
			).expect("linear weights bwd");
		}
		kernels::gpu_sgd_update(&sc.dw, &ss.neg_lr, in_dim * out_dim, &params[l].w).expect("sgd w");
		kernels::gpu_sgd_update(&sc.db, &ss.neg_lr, out_dim, &params[l].b).expect("sgd b");
		if let Some(ConcatDims { pf, a, c }) = cc
			&& l == pf
		{
			kernels::gpu_slice_lead_into(da_below, n, a + c, a, &sc.concat_dgrad).expect("concat slice");
			kernels::gpu_copy_into(&sc.concat_dgrad, n * a, da_below).expect("concat copy");
		}
		sc.mark_bwd(l);
		flip = !flip;
	}
}

#[test]
fn gpu_metrics_match_cpu_reference() {
	let train: &recipe::dataset::Dataset = &CHURN;
	gpu_core::hip::set_device(0).expect("set_device");
	let x = &train.x;
	let y = &train.y;
	let n = x.nrows();
	let d = x.ncols();

	let h = 8usize;
	let w1 = GpuBuffer::alloc(d * h).expect("w1 alloc");
	kernels::gpu_randn(d * h, 11, &w1).expect("w1");
	let b1 = { let __up = &vec![0.0f64; h]; let __ub = GpuBuffer::alloc(__up.len()).expect("b1"); __ub.load(__up).expect("b1"); __ub };
	let w2 = GpuBuffer::alloc(h).expect("w2 alloc");
	kernels::gpu_randn(h, 22, &w2).expect("w2");
	let b2 = { let __up = &[0.0f64; 1]; let __ub = GpuBuffer::alloc(__up.len()).expect("b2"); __ub.load(__up).expect("b2"); __ub };
	let params = vec![
		LayerParams {
			kind: LayerKind::Dense,
			w: w1,
			b: b1,
			in_dim: d,
			out_dim: h,
			act: Activation::Relu,
			dim: 0,
			vocab: 0,
			wk: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wv: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wo: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			heads: 0,
			palpha: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
		conv_cin: 0, conv_k: 0, conv_stride: 0,
		},
		LayerParams {
			kind: LayerKind::Dense,
			w: w2,
			b: b2,
			in_dim: h,
			out_dim: 1,
			act: Activation::Sigmoid,
			dim: 0,
			vocab: 0,
			wk: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wv: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wo: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			heads: 0,
			palpha: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
		conv_cin: 0, conv_k: 0, conv_stride: 0,
		},
	];

	let (xbuf, nn, _d) = upload(x);
	assert_eq!(nn, n);
	let ybuf = { let __up = y.as_slice().expect("y contig"); let __ub = GpuBuffer::alloc(__up.len()).expect("ybuf"); __ub.load(__up).expect("ybuf"); __ub };
	let consts = consts_buf();
	let sc = Scratch::new_full(&params, n, &consts).expect("scratch");
	forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
	let last = params.len() - 1;
	let p = download_vec(&sc.acts[last], n);
	let ybar = y.iter().sum::<f64>() / n as f64;
	let ss_tot = y.iter().map(|v| (v - ybar).powi(2)).sum::<f64>();

	let ss_res: f64 = (0..n).map(|i| (y[i] - p[i]).powi(2)).sum();
	let r2_ref = 1.0 - ss_res / ss_tot;
	let acc_ref =
		(0..n).filter(|&i| (p[i] >= 0.5) == (y[i] >= 0.5)).count() as f64 / n as f64;
	let mse_ref = (0..n).map(|i| (p[i] - y[i]).powi(2)).sum::<f64>() / n as f64;
	let mae_ref = (0..n).map(|i| (p[i] - y[i]).abs()).sum::<f64>() / n as f64;
	let huber_ref = (0..n)
		.map(|i| {
			let r = p[i] - y[i];
			if r.abs() <= 1.0 {
				0.5 * r * r
			} else {
				r.abs() - 0.5
			}
		})
		.sum::<f64>() / n as f64;
	let eps = 1e-7;
	let bce_ref = -(0..n)
		.map(|i| {
			let pc = p[i].clamp(eps, 1.0 - eps);
			y[i] * pc.ln() + (1.0 - y[i]) * (1.0 - pc).ln()
		})
		.sum::<f64>() / n as f64;

	let close = |a: f64, b: f64, what: &str| {
		let tol = 1e-6 * a.abs().max(b.abs()).max(1.0);
		assert!(
			(a - b).abs() <= tol,
			"{what}: gpu={a} cpu={b} diff={}",
			(a - b).abs()
		);
	};
	let out = &sc.acts[last];
	let metric = |m: Metric, loss: Loss| -> f64 {
		let LossScale { sign, div } =
			metric_gpu_into(loss, m, out, &ybuf, &sc, n, 1, ss_tot, &sc.metric_scalar).expect("metric");
		let v = sign * download_scalar(&sc.metric_scalar) / div;
		if m == Metric::R2 { 1.0 - v } else { v }
	};
	close(metric(Metric::R2, Loss::Mse), r2_ref, "R2");
	close(metric(Metric::Accuracy, Loss::Mse), acc_ref, "Accuracy");
	close(metric(Metric::Loss, Loss::Mse), mse_ref, "MSE");
	close(metric(Metric::Loss, Loss::Mae), mae_ref, "MAE");
	close(metric(Metric::Loss, Loss::Huber), huber_ref, "Huber");
	close(metric(Metric::Loss, Loss::Bce), bce_ref, "BCE");

	eprintln!(
		"OK n={n} d={d}  R2={r2_ref:.6}  acc={acc_ref:.6}  mse={mse_ref:.6}  mae={mae_ref:.6}  huber={huber_ref:.6}  bce={bce_ref:.6}"
	);
}

#[test]
fn fit_loop_memory_flat() {
	let train: &recipe::dataset::Dataset = &CHURN;
	gpu_core::hip::set_device(0).expect("set_device");
	let x = &train.x;
	let y = &train.y;
	let n = x.nrows();
	let d = x.ncols();

	let (xraw, _, _) = upload(x);
	let seg = kernels::gpu_reduce_sum_cols_workspace_bytes(n, d);
	let ws = GpuBuffer::alloc_bytes(((seg + 255) & !255usize) + d * 8).expect("reduce ws");
	let mean = GpuBuffer::alloc(d).expect("mean");
	kernels::gpu_reduce_mean_cols(&xraw, n, d, &ws, &mean).expect("mean");
	let var = GpuBuffer::alloc(d).expect("var");
	kernels::gpu_reduce_var_cols(&xraw, n, d, &ws, &var).expect("var");
	let eps = { let __up = &[1e-8]; let __ub = GpuBuffer::alloc(__up.len()).expect("eps"); __ub.load(__up).expect("eps"); __ub };
	kernels::gpu_add_scalar_inplace(&eps, d, &var).expect("var eps");
	let std = GpuBuffer::alloc(d).expect("std");
	kernels::gpu_sqrt(&var, d, &std).expect("std");
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub_into(&xraw, &mean, n * d, d, &xc).expect("center");
	let xbuf = GpuBuffer::alloc(n * d).expect("scale");
	kernels::gpu_broadcast_div(&xc, &std, n * d, d, &xbuf).expect("scale");
	let ybuf = { let __up = y.as_slice().expect("y contig"); let __ub = GpuBuffer::alloc(__up.len()).expect("ybuf"); __ub.load(__up).expect("ybuf"); __ub };

	let h = 16usize;
	let mk = |fan_in: usize, units: usize, seed: u32, act: Activation| {
		let scale = (2.0 / fan_in as f64).sqrt();
		let w = GpuBuffer::alloc(fan_in * units).expect("randn alloc");
		kernels::gpu_randn(fan_in * units, seed as usize, &w).expect("randn");
		let sbuf = { let __up = &[scale]; let __ub = GpuBuffer::alloc(__up.len()).expect("he scale"); __ub.load(__up).expect("he scale"); __ub };
		kernels::gpu_scale_inplace(&sbuf, fan_in * units, &w).expect("he scale");
		let b = { let __up = &vec![0.0f64; units]; let __ub = GpuBuffer::alloc(__up.len()).expect("b"); __ub.load(__up).expect("b"); __ub };
		LayerParams {
			kind: LayerKind::Dense,
			w,
			b,
			in_dim: fan_in,
			out_dim: units,
			act,
			dim: 0,
			vocab: 0,
			wk: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wv: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wo: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			heads: 0,
			palpha: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
		conv_cin: 0, conv_k: 0, conv_stride: 0,
		}
	};
	let params = vec![
		mk(d, h, 11, Activation::Relu),
		mk(h, 1, 22, Activation::Sigmoid),
	];
	let last = params.len() - 1;

	let consts = consts_buf();
	let sc_ref = Scratch::new_infer(&params, n, &consts).expect("scratch");
	forward_into(&params, &xbuf, None, n, &sc_ref.acts, &sc_ref).expect("forward");
	let out_ref = download_vec(&sc_ref.acts[last], n);
	drop(sc_ref);
	let sc = {
		let _t_scratch = gpu_core::memory::tag_scope("scratch");
		Scratch::new_full(&params, n, &consts).expect("scratch")
	};
	forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
	let out_into = download_vec(&sc.acts[last], n);
	let fwd_diff = out_ref
		.iter()
		.zip(&out_into)
		.map(|(a, b)| (a - b).abs())
		.fold(0.0, f64::max);
	assert!(
		fwd_diff < 1e-9,
		"forward_into not deterministic, maxdiff={fwd_diff}"
	);

	let ybar = y.iter().sum::<f64>() / n as f64;
	let ss_tot: f64 = y.iter().map(|v| (v - ybar).powi(2)).sum();
	let ss = step_scalars(0.5, n);

	forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
	backward_step(Loss::Mse, &params, &xbuf, &ybuf, n, &sc, &ss);

	const EPOCHS: usize = 10;
	let mut r2s = Vec::with_capacity(EPOCHS);
	{
		let _alloc_guard = gpu_core::memory::AllocGuard::freeze();
		for _ in 0..EPOCHS {
			forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
			backward_step(Loss::Mse, &params, &xbuf, &ybuf, n, &sc, &ss);
			kernels::gpu_ss_res_into(&sc.acts[last], &ybuf, n, &sc.metric_scalar).expect("ss_res");
			r2s.push(1.0 - download_scalar(&sc.metric_scalar) / ss_tot);
		}
	}

	assert!(r2s.iter().all(|v| v.is_finite()), "non-finite R²: {r2s:?}");
	assert!(
		r2s[EPOCHS - 1] > r2s[0],
		"R² did not rise: first={} last={}",
		r2s[0],
		r2s[EPOCHS - 1]
	);

	eprintln!("R2 first={:.6} last={:.6}", r2s[0], r2s[EPOCHS - 1]);
}

#[test]
fn ping_pong_gradients_match_per_layer() {
	let train: &recipe::dataset::Dataset = &CHURN;
	gpu_core::hip::set_device(0).expect("set_device");
	let x = &train.x;
	let y = &train.y;
	let n = x.nrows();
	let d = x.ncols();

	let (xraw, _, _) = upload(x);
	let seg = kernels::gpu_reduce_sum_cols_workspace_bytes(n, d);
	let ws = GpuBuffer::alloc_bytes(((seg + 255) & !255usize) + d * 8).expect("reduce ws");
	let mean = GpuBuffer::alloc(d).expect("mean");
	kernels::gpu_reduce_mean_cols(&xraw, n, d, &ws, &mean).expect("mean");
	let var = GpuBuffer::alloc(d).expect("var");
	kernels::gpu_reduce_var_cols(&xraw, n, d, &ws, &var).expect("var");
	let eps = { let __up = &[1e-8]; let __ub = GpuBuffer::alloc(__up.len()).expect("eps"); __ub.load(__up).expect("eps"); __ub };
	kernels::gpu_add_scalar_inplace(&eps, d, &var).expect("var eps");
	let std = GpuBuffer::alloc(d).expect("std");
	kernels::gpu_sqrt(&var, d, &std).expect("std");
	let xc = GpuBuffer::alloc(n * d).expect("center");
	kernels::gpu_broadcast_sub_into(&xraw, &mean, n * d, d, &xc).expect("center");
	let xbuf = GpuBuffer::alloc(n * d).expect("scale");
	kernels::gpu_broadcast_div(&xc, &std, n * d, d, &xbuf).expect("scale");
	let ybuf = { let __up = y.as_slice().expect("y contig"); let __ub = GpuBuffer::alloc(__up.len()).expect("ybuf"); __ub.load(__up).expect("ybuf"); __ub };

	let h = 16usize;
	let lr = 0.01;
	let mk = |fan_in: usize, units: usize, seed: u32, act: Activation| {
		let scale = (2.0 / fan_in as f64).sqrt();
		let w = GpuBuffer::alloc(fan_in * units).expect("randn alloc");
		kernels::gpu_randn(fan_in * units, seed as usize, &w).expect("randn");
		let sbuf = { let __up = &[scale]; let __ub = GpuBuffer::alloc(__up.len()).expect("he scale"); __ub.load(__up).expect("he scale"); __ub };
		kernels::gpu_scale_inplace(&sbuf, fan_in * units, &w).expect("he scale");
		let b = { let __up = &vec![0.0f64; units]; let __ub = GpuBuffer::alloc(__up.len()).expect("b"); __ub.load(__up).expect("b"); __ub };
		LayerParams {
			kind: LayerKind::Dense,
			w,
			b,
			in_dim: fan_in,
			out_dim: units,
			act,
			dim: 0,
			vocab: 0,
			wk: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wv: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			wo: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
			heads: 0,
			palpha: { let __up = &[0.0f64]; let __ub = GpuBuffer::alloc(__up.len()).expect("d"); __ub.load(__up).expect("d"); __ub },
		conv_cin: 0, conv_k: 0, conv_stride: 0,
		}
	};

	let params = vec![
		mk(d, h, 11, Activation::Relu),
		mk(h, 1, 22, Activation::Sigmoid),
	];
	let last = params.len() - 1;
	let init_w: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
		.collect();
	let init_b: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.b, p.out_dim))
		.collect();

	let ss = step_scalars(lr, n);
	let consts = consts_buf();
	let sc = Scratch::new_full(&params, n, &consts).expect("scratch");
	forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
	backward_step(Loss::Mse, &params, &xbuf, &ybuf, n, &sc, &ss);
	let pp_w: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
		.collect();
	let pp_b: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.b, p.out_dim))
		.collect();

	for (l, p) in params.iter().enumerate() {
		p.w.load(&init_w[l]).expect("restore w");
		p.b.load(&init_b[l]).expect("restore b");
	}

	forward_into(&params, &xbuf, None, n, &sc.acts, &sc).expect("forward");
	let mut ref_da: Vec<GpuBuffer> = Vec::new();
	let mut ref_dz: Vec<GpuBuffer> = Vec::new();
	let mut ref_dw: Vec<GpuBuffer> = Vec::new();
	let mut ref_db: Vec<GpuBuffer> = Vec::new();
	let max_ws = params
		.iter()
		.map(|p| kernels::gpu_reduce_sum_cols_workspace_bytes(n, p.out_dim))
		.max()
		.unwrap_or(0);
	let ref_ws = GpuBuffer::alloc_bytes(
		max_ws.max(kernels::gpu_reduce_sum_cols_workspace_bytes(n, 1)),
	)
	.expect("ref ws");
	let ref_partials = GpuBuffer::alloc(
		params
			.iter()
			.map(|p| kernels::gpu_splitk_dw_partials_elems(n, p.in_dim, p.out_dim))
			.max()
			.unwrap_or(1)
			.max(1),
	)
	.expect("ref partials");
	for p in &params {
		ref_da.push(GpuBuffer::alloc(n * p.out_dim).expect("ref da"));
		ref_dz.push(GpuBuffer::alloc(n * p.out_dim).expect("ref dz"));
		ref_dw.push(GpuBuffer::alloc(p.in_dim * p.out_dim).expect("ref dw"));
		ref_db.push(GpuBuffer::alloc(p.out_dim).expect("ref db"));
	}
	recipe::train::loss_grad_into(
		Loss::Mse,
		&sc.acts[last],
		&ybuf,
		&ref_da[last],
		n,
		n * params[last].out_dim,
		&sc,
		&ss,
	)
	.expect("loss grad");
	for l in (0..params.len()).rev() {
		let (in_dim, out_dim) = (params[l].in_dim, params[l].out_dim);
		let m = n * out_dim;
		let grad = match params[l].act {
			Activation::Relu => {
				kernels::gpu_relu_backward_into(
					&ref_da[l],
					&sc.acts[l],
					m,
					&ref_dz[l],
				).expect("relu bwd");
				&ref_dz[l]
			}
			Activation::Sigmoid => {
				kernels::gpu_sigmoid_backward_into(
					&ref_da[l],
					&sc.acts[l],
					m,
					&ref_dz[l],
				).expect("sigmoid bwd");
				&ref_dz[l]
			}
			Activation::Linear => &ref_da[l],
			_ => unreachable!("this test builds only relu/sigmoid/linear layers"),
		};
		let a_prev = if l == 0 { &xbuf } else { &sc.acts[l - 1] };
		if out_dim == 1 {
			kernels::gpu_dgemv_into(a_prev, grad, n, in_dim, 1, &ref_dw[l]).expect("dgemv");
			kernels::gpu_reduce_sum_cols_into(grad, &ref_ws, n, 1, &ref_db[l]).expect("db reduce");
			if l > 0 {
				kernels::gpu_dger_into(grad, &params[l].w, n, in_dim, &ref_da[l - 1]).expect("dger");
			}
		} else if l > 0 {
			kernels::gpu_linear_backward_full_into(
				grad,
				a_prev,
				&params[l].w,
				&ref_ws,
				&ref_partials,
				n,
				out_dim,
				in_dim,
				&ref_da[l - 1],
				&ref_dw[l],
				&ref_db[l],
			).expect("linear full bwd");
		} else {
			kernels::gpu_linear_backward_weights_only_into(
				grad, a_prev, &ref_ws, &ref_partials, n, out_dim, in_dim, &ref_dw[l], &ref_db[l],
			).expect("linear weights bwd");
		}
		kernels::gpu_sgd_update(&ref_dw[l], &ss.neg_lr, in_dim * out_dim, &params[l].w).expect("sgd w");
		kernels::gpu_sgd_update(&ref_db[l], &ss.neg_lr, out_dim, &params[l].b).expect("sgd b");
	}
	let ref_w: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.w, p.in_dim * p.out_dim))
		.collect();
	let ref_b: Vec<Vec<f64>> = params
		.iter()
		.map(|p| download_vec(&p.b, p.out_dim))
		.collect();

	let mut max_w_diff = 0.0f64;
	let mut max_b_diff = 0.0f64;
	for (l, p) in params.iter().enumerate() {
		let _ = p;
		let wd = pp_w[l]
			.iter()
			.zip(&ref_w[l])
			.map(|(a, b)| (a - b).abs())
			.fold(0.0, f64::max);
		let bd = pp_b[l]
			.iter()
			.zip(&ref_b[l])
			.map(|(a, b)| (a - b).abs())
			.fold(0.0, f64::max);
		eprintln!("layer {l}: W maxdiff={wd:.2e}  b maxdiff={bd:.2e}");
		if wd > max_w_diff {
			max_w_diff = wd;
		}
		if bd > max_b_diff {
			max_b_diff = bd;
		}
	}
	assert!(
		max_w_diff < 1e-10,
		"weights mismatch after backward: max abs diff = {max_w_diff:.2e}"
	);
	assert!(
		max_b_diff < 1e-10,
		"biases mismatch after backward: max abs diff = {max_b_diff:.2e}"
	);
}

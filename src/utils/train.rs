use gpu_core::log::{Write, acc, data, device, epoch, gpu, loss, lr, prompt, r2, save, time};
use crate::dataset::{Dataset, collapse_onehot};
use crate::model::{ModelInner, Train};
use anyhow::Context;
use gpu_core::kernels;
use gpu_core::memory::{GpuBuffer, Stage};
use recipe_infer::{
	LayerSpec, Loss, Metric, PlanMode, SCRATCH_CONSTS, Scaler, Scratch, concat_layer,
	load_ogdl, load_ogdl_str, metric_gpu_into, pinned_vocab, plan_layer_params,
	zscore_apply_views,
};
use std::io::IsTerminal;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) static INTERRUPTED: std::sync::atomic::AtomicUsize = AtomicUsize::new(0);
extern "C" fn on_sigint(_sig: i32) {
	let Some(_second) = Some(()).filter(|_probe| INTERRUPTED.swap(1, Ordering::SeqCst) != 0)
	else {
		return;
	};
	unsafe { libc::_exit(130) };
}
#[derive(Clone, Copy)]
enum Logged {
	Yes,
	No,
}
enum YesNo {
	Yes,
	No,
}
#[derive(Clone, Copy)]
enum Resumed {
	Yes,
	No,
}
enum Halt {
	Stop,
	Continue,
}
struct YPrep {
	y_flat: Vec<f64>,
	ss_tot: f64,
}
struct EpochMeta {
	epoch: usize,
	elapsed: f64,
	logged: Logged,
}
pub(crate) struct EvalInput {
	pub x: GpuBuffer,
	pub x_cat: Option<GpuBuffer>,
	pub n: usize,
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
pub(crate) fn metrics_line(metrics: &[Metric], vals: &[f64]) -> String {
	let mut parts: Vec<String> = Vec::new();
	for mi in 0..metrics.len().min(vals.len()) {
		let m = metrics[mi];
		let Some(_w) = m.fmt().width.checked_sub(1) else {
			continue;
		};
		let num = m.render(vals[mi]);
		parts.push(format!("{} {num}", m.fmt().label));
	}
	parts.join("  ")
}
impl ModelInner {
	pub(crate) fn fit(
		&self,
		dat: &Dataset,
		cfg: &Train,
		resume: Option<&str>,
		net: Option<std::sync::Arc<Vec<crate::wire::Conn>>>,
	) -> anyhow::Result<()> {
		let hip_snap = Some(())
			.filter(|_probe| cfg.metrics.contains(&Metric::Hip))
			.map(|_probe| gpu_core::callspy::snapshot());
		let led_snap = hip_snap.map(|_snap| gpu_core::memory::xfer_calls());
		let start = std::time::Instant::now();
		let classify = self.loss.is_classification();
		let rerun = !self.params.borrow().is_empty();
		let checkpoint_path = cfg.resume.as_deref().map(Train::resolve);
		let checkpointing = checkpoint_path.is_some();
		let plotting = !cfg.plot.is_empty() && std::io::stdout().is_terminal();
		let plot_ys: Vec<Metric> = cfg
			.plot
			.iter()
			.copied()
			.filter(|&m| m != Metric::Epoch && m != Metric::Time)
			.collect();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats =
			embed_first && dat.text_cols.is_empty() && !dat.onehot_groups.is_empty();
		let coll = Some(())
			.filter(|_probe| embed_cats)
			.map(|_probe| collapse_onehot(dat));
		let collapsed_vocab = coll.as_ref().map_or(0, |c| c.vocab);
		let effective_x = coll.as_ref().map_or(&dat.x, |c| &c.x);
		let effective_text = match &coll {
			Some(c) => &c.embed_cols,
			None => &dat.text_cols,
		};
		let cat_cols: Vec<usize> = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| {
				Vec::from_iter(
					(0..effective_x.ncols()).filter(|c| !effective_text.contains(c)),
				)
			})
			.unwrap_or_default();
		let c_cat = cat_cols.len();
		let xinput = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| effective_x.select(ndarray::Axis(1), effective_text))
			.unwrap_or_else(|| effective_x.clone());
		let n = xinput.nrows();
		let d = xinput.ncols();
		let vocab = match pinned_vocab(&self.specs) {
			Some(v) => v,
			None => Some(())
				.filter(|_probe| embed_first)
				.map(|_probe| {
					Some(())
						.filter(|_p| embed_cats)
						.map(|_p| collapsed_vocab)
						.unwrap_or_else(|| {
							xinput.iter().cloned().fold(0.0f64, f64::max) as usize + 1
						})
				})
				.unwrap_or(0),
		};
		let cat = Some(())
			.filter(|_probe| embed_first && c_cat > 0)
			.map(|_probe| effective_x.select(ndarray::Axis(1), &cat_cols));
		let c = cat.as_ref().map_or(0, |m| m.ncols());
		let d_sc = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| c)
			.unwrap_or(d);
		let resumed = resume.map(load_ogdl).transpose()?.unwrap_or_default();
		let mut did_resume = Some(())
			.filter(|_probe| !resumed.is_empty())
			.map(|_probe| Resumed::Yes)
			.unwrap_or(Resumed::No);
		let source = match did_resume {
			Resumed::Yes => resumed,
			Resumed::No => match Some(()).filter(|_probe| rerun) {
				Some(_re) => {
					let m = self.saved_ogdl.borrow();
					let mirror = m.as_ref().ok_or_else(|| {
						anyhow::anyhow!("rerun without host weight mirror")
					})?;
					load_ogdl_str(&mirror.text).map_err(|e| {
						anyhow::anyhow!("rerun: parse host weight mirror: {e}")
					})?
				}
				None => Vec::new(),
			},
		};
		let ask_overwrite = |what: &str| -> YesNo {
			use std::io::Write as _;
			drop(Write::err("resume"));
			drop(Write::err("    data does not match"));
			drop(Write::err(&format!("        {what}")));
			drop(Write::err(&format!("        file path={}", resume.unwrap_or(""))));
			drop(Write::err(&format!("        data path={}", dat.source)));
			let Some(_tty) = Some(()).filter(|_probe| std::io::stdin().is_terminal()) else {
				return YesNo::No;
			};
			Write::line(
				prompt,
				"overwrite checkpoint with random weights? [y/N] ",
			);
			std::io::stderr().flush().ok();
			let mut line = String::new();
			std::io::stdin().read_line(&mut line).ok();
			match line.trim() {
				"y" | "Y" | "yes" | "YES" => YesNo::Yes,
				_other => YesNo::No,
			}
		};
		let warm = Some(PlanMode::Warm)
			.filter(|_m| matches!(did_resume, Resumed::Yes) || rerun)
			.unwrap_or(PlanMode::Fresh);
		let plan = match plan_layer_params(&self.specs, d, c_cat, vocab, &source, warm) {
			Ok(p) => p,
			Err(what) => {
				let overwrite = matches!(did_resume, Resumed::Yes)
					&& matches!(ask_overwrite(&what), YesNo::Yes);
				match Some(()).filter(|_probe| overwrite) {
					Some(_ow) => {
						did_resume = Resumed::No;
						plan_layer_params(
							&self.specs,
							d,
							c_cat,
							vocab,
							&[],
							PlanMode::Fresh,
						)
						.map_err(|e| anyhow::anyhow!(e))?
					}
					None => match did_resume {
						Resumed::Yes => anyhow::bail!(
							"checkpoint mismatch — user declined overwrite"
						),
						Resumed::No => anyhow::bail!(
							"rerun: host weight mirror does not match this run — {what}"
						),
					},
				}
			}
		};
		let out_dim = plan.out_dim_last();
		let n_targets = dat.n_targets.max(1);
		let expand_ce = classify && n_targets == 1 && out_dim > 1;
		let k = Some(())
			.filter(|_probe| expand_ce)
			.map(|_probe| out_dim)
			.unwrap_or(n_targets);
		if out_dim != k {
			Write::err(format!(
			"output layer has {out_dim} units but there are {n_targets} target column(s) — make the last .layer({n_targets})"
			))?;
		}
		let yp = {
			let ys =
				dat.y.as_slice()
					.ok_or_else(|| anyhow::anyhow!("train: y contiguous"))?;
			let mut yd = ys.to_vec();
			match Some(()).filter(|_probe| !classify && !rerun) {
				Some(_fresh) => {
					let ymean = yd.iter().sum::<f64>() / yd.len() as f64;
					let yvar = yd.iter().map(|v| (v - ymean).powi(2)).sum::<f64>()
						/ yd.len() as f64;
					let ystd = (yvar + 1e-8).sqrt();
					for v in yd.iter_mut() {
						*v = (*v - ymean) / ystd;
					}
					*self.yscaler.borrow_mut() = Some(recipe_infer::YScaler {
						mean: ymean,
						std: ystd,
					});
				}
				None => {
					let ys_opt = *self.yscaler.borrow();
					Some(())
						.filter(|_probe| !classify && rerun)
						.and(ys_opt)
						.map(|ysc| {
							for v in yd.iter_mut() {
								*v = (*v - ysc.mean) / ysc.std;
							}
						})
						.unwrap_or(());
				}
			}
			let total = yd.len() as f64;
			let ybar = yd.iter().sum::<f64>() / total;
			let ss_tot: f64 = yd.iter().map(|v| (v - ybar).powi(2)).sum();
			let y_flat = match Some(()).filter(|_probe| expand_ce) {
				Some(_oh) => {
					let mut oh = vec![0.0f64; n * out_dim];
					for i in 0..yd.len() {
						let idx = yd[i];
						let cc = idx as usize;
						Some(())
							.filter(|_probe| idx.is_finite() && cc < out_dim)
							.map(|_probe| {
								oh[i * out_dim + cc] = 1.0;
							})
							.unwrap_or(());
					}
					oh
				}
				None => yd,
			};
			YPrep { y_flat, ss_tot }
		};
		let ss_tot = yp.ss_tot;
		let epochs = cfg.epochs.max(1);
		let stop_metric = Some(())
			.filter(|_probe| classify)
			.map(|_probe| Metric::Accuracy)
			.unwrap_or(Metric::R2);
		let mut ring_row: Vec<Metric> = vec![stop_metric];
		let ckpt_loss = Some(())
			.filter(|_probe| checkpointing)
			.map(|_probe| Metric::Loss);
		for m in cfg
			.metrics
			.iter()
			.copied()
			.chain(ckpt_loss)
			.chain(plot_ys.iter().copied())
		{
			let keep = matches!(m, Metric::Loss | Metric::R2 | Metric::Accuracy)
				&& !ring_row.contains(&m);
			ring_row.extend(Some(m).filter(|_probe| keep));
		}
		let n_rows = ring_row.len();
		let n_timed = 0usize;
		let zf: recipe_infer::ZFit = match Some(()).filter(|_probe| d_sc == 0) {
			Some(_empty) => recipe_infer::ZFit {
				mean: Vec::new(),
				std: Vec::new(),
				scaled: Vec::new(),
			},
			None => {
				let src = match Some(()).filter(|_probe| embed_first) {
					Some(_ef) => {
						cat.as_ref().ok_or_else(|| anyhow::anyhow!("cat matrix"))?
					}
					None => &xinput,
				};
				let src_std = src.as_standard_layout();
				let sl = src_std
					.as_slice()
					.ok_or_else(|| anyhow::anyhow!("scale src contiguous"))?;
				match Some(()).filter(|_probe| rerun) {
					Some(_re) => {
						let sc_host = self.scaler.borrow();
						let s = sc_host
							.as_ref()
							.ok_or_else(|| anyhow::anyhow!("rerun without scaler"))?;
						if s.mean.len() != d_sc {
							Write::err(format!(
								"rerun: feature count changed: {} vs {d_sc}",
								s.mean.len()
							))?;
						}
						let scaled = recipe_infer::zscore_apply_host(
							sl, n, d_sc, &s.mean, &s.std,
						);
						recipe_infer::ZFit {
							mean: s.mean.clone(),
							std: s.std.clone(),
							scaled,
						}
					}
					None => recipe_infer::zscore_fit_host(sl, n, d_sc),
				}
			}
		};
		let mut stage = Stage::new();
		let w_off = stage.push(plan.host());
		let mut mean_off = 0;
		let mut std_off = 0;
		Some(())
			.filter(|_probe| d_sc != 0)
			.map(|_probe| {
				mean_off = stage.push(&zf.mean);
				std_off = stage.push(&zf.std);
			})
			.unwrap_or(());
		let ring_off = stage.reserve(n_rows * epochs);
		let end_off = stage.reserve(1);
		let w_len = plan.host().len();
		let prefix_len = stage.len_floats();
		let consts_off = stage.push(&SCRATCH_CONSTS);
		let sc_off = stage.push(&[-self.lr, 1.0 / n as f64, 2.0 / n as f64, 0.0]);
		let scaled_off = Some(())
			.filter(|_probe| d_sc > 0)
			.map(|_probe| stage.push(&zf.scaled))
			.unwrap_or(0);
		let x_off = match Some(()).filter(|_probe| embed_first) {
			Some(_ef) => {
				let x_std = xinput.as_standard_layout();
				stage.push(x_std
					.as_slice()
					.ok_or_else(|| anyhow::anyhow!("xinput contiguous"))?)
			}
			None => 0,
		};
		let y_off = stage.push(&yp.y_flat);
		let image = stage.into_host();
		let image_floats = image.len();
		let ac_pre = recipe_infer::concat_layer_dims(&plan.dims())
			.map(|d| crate::ooc::ConcatAc { a: d.a, c: d.c });
		let need = image_floats * 8 + crate::ooc::Ooc::min_bytes(&plan.dims(), n, ac_pre);
		let slab = gpu_core::memory::adopt_run_backing_with_image(need, &image)
			.or_else(|| gpu_core::memory::claim_device_arena_with_image(&image))
			.ok_or_else(|| {
				anyhow::anyhow!(
					"arena: claim failed — image {} claimable {} free {}",
					crate::data::human_bytes(need),
					crate::data::human_bytes(gpu_core::memory::claimable_bytes()),
					crate::data::human_bytes(gpu_core::memory::vram_free_base()),
				)
			})?;
		let base = slab.view(0, image_floats);
		let params = plan.materialize(&base, w_off);
		let last = params.len() - 1;
		let concat_fit_raw = concat_layer(&params);
		let ac_fit = concat_fit_raw
			.as_ref()
			.map(|d| crate::ooc::ConcatAc { a: d.a, c: d.c });
		let concat_fit = concat_fit_raw.as_ref().map(|d| crate::ooc::ConcatFit {
			pf: d.pf,
			a: d.a,
			c: d.c,
		});
		let consts_view = base.view(consts_off, 12);
		let sc = {
			let _t = gpu_core::memory::tag_scope("scratch");
			Scratch::carve(&params, n, &consts_view, n_timed)?
		};
		let ss = StepScalars {
			neg_lr: base.view(sc_off, 1),
			inv_n: base.view(sc_off + 1, 1),
			two_inv_n: base.view(sc_off + 2, 1),
			zero: base.view(sc_off + 3, 1),
		};
		Some(())
			.filter(|_probe| d_sc == 0 && !rerun)
			.map(|_probe| {
				*self.scaler.borrow_mut() = Some(Scaler {
					mean: vec![],
					std: vec![],
				});
			})
			.unwrap_or(());
		let x_cat = Some(())
			.filter(|_probe| d_sc > 0 && embed_first)
			.map(|_probe| base.view(scaled_off, n * d_sc));
		let xbuf = Some(())
			.filter(|_probe| d_sc > 0)
			.map(|_probe| {
				Some(())
					.filter(|_p| embed_first)
					.map(|_p| base.view(x_off, n * d))
					.unwrap_or_else(|| base.view(scaled_off, n * d_sc))
			})
			.unwrap_or_else(|| base.view(x_off, xinput.len()));
		let ybuf = base.view(y_off, n * k);
		let summary_graph = Some(())
			.filter(|_probe| !cfg.metrics.is_empty())
			.map(|_probe| {
				let neurons: usize = params.iter().map(|p| p.out_dim).sum();
				let out = params[last].out_dim;
				let kv = |name: &str, val: usize| ogdl::Node {
					name: name.to_string(),
					children: vec![ogdl::Node {
						name: val.to_string(),
						children: Vec::new(),
					}],
				};
				let sect = |name: &str, kids: Vec<ogdl::Node>| ogdl::Node {
					name: name.to_string(),
					children: kids,
				};
				ogdl::Node {
					name: String::new(),
					children: vec![
						sect(
							"arch",
							vec![
								kv("neurons", neurons),
								kv("layers", params.len()),
								kv("samples", n),
								kv("features", d),
								kv("input_dim", d),
								kv("output_dim", out),
							],
						),
						sect(
							"data",
							vec![
								kv("rows", n + 1),
								kv("columns", d + 1),
								kv("predictors", d),
								kv("targets", out),
							],
						),
					],
				}
			});
		Some(())
			.filter(|_probe| !plotting && !rerun)
			.map(|_probe| {
				Some(())
					.filter(|_probe| matches!(did_resume, Resumed::Yes))
					.map(|_probe| {
						resume.map(|path| {
							let full = std::fs::canonicalize(path)
								.unwrap_or_else(|_err| path.into());
							Write::line(save, &format!("resumed: {}", full.display()));
						})
						.unwrap_or(())
					})
					.unwrap_or(());
				Some(())
					.filter(|_probe| summary_graph.is_some())
					.map(|_probe| {
						for g in summary_graph.iter() {
							Write::block(data, &g.serialize());
						}
						Write::line(gpu, &format!(
							"roofline  gemm {} GF/s  vram {} GB/s",
							recipe_infer::GEMM_GFLOPS,
							recipe_infer::VRAM_GBS
						));
					})
					.unwrap_or(());
			})
			.unwrap_or(());
		let mut ooc = {
			let _t = gpu_core::memory::tag_scope("waterfall");
			let o = crate::ooc::Ooc::build(&params, n, ac_fit, net.clone())?;
			o.report();
			o
		};
		let _guard = gpu_core::memory::AllocGuard::freeze();
		gpu_core::hw::arm_saturation_crash();
		INTERRUPTED.store(0, Ordering::SeqCst);
		unsafe {
			libc::signal(libc::SIGINT, on_sigint as *const () as libc::sighandler_t);
		}
		gpu_core::callspy::mark_loop_start();
		let hip_init = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_init = hip_snap.map(|_snap| gpu_core::memory::xfer_calls());
		let mut fit_score = f64::NAN;
		let mut epoch_meta: Vec<EpochMeta> = Vec::new();
		let ring_every = checkpointing || plotting;
		let mut ring_scale: Vec<recipe_infer::LossScale> = vec![
			recipe_infer::LossScale {
				sign: 1.0,
				div: 1.0
			};
			n_rows
		];
		for e in 0..cfg.epochs {
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			let log_now = cfg.log_every > 0
				&& !cfg.metrics.is_empty()
				&& (e % cfg.log_every == 0 || e + 1 == cfg.epochs);
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			ooc.backward(&params, &xbuf, &ybuf, &sc, &ss, self.loss, concat_fit)?;
			let Some(_do) = Some(()).filter(|_probe| log_now || ring_every) else {
				continue;
			};
			ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
			let Some(_go) = Some(()).filter(|_probe| INTERRUPTED.load(Ordering::SeqCst) == 0)
			else {
				break;
			};
			let out = &sc.acts[last];
			for mi in 0..ring_row.len() {
				let m = ring_row[mi];
				let slot = base.view(ring_off + mi * epochs + e, 1);
				ring_scale[mi] =
					metric_gpu_into(self.loss, m, out, &ybuf, &sc, n, k, ss_tot, &slot)?;
			}
			epoch_meta.push(EpochMeta {
				epoch: e,
				elapsed: start.elapsed().as_secs_f64(),
				logged: Some(())
					.filter(|_probe| log_now)
					.map(|_probe| Logged::Yes)
					.unwrap_or(Logged::No),
			});
		}
		let was_interrupted = INTERRUPTED.load(Ordering::SeqCst) != 0;
		Some(())
			.filter(|_probe| !was_interrupted)
			.map(|_probe| -> anyhow::Result<()> {
				ooc.forward(&params, &xbuf, x_cat.as_ref(), &sc, concat_fit)?;
				let end_slot = base.view(end_off, 1);
				match Some(()).filter(|_p| classify) {
					Some(_cls) => match Some(()).filter(|_p| k == 1) {
						Some(_bin) => kernels::gpu_accuracy_into(
							&sc.acts[last],
							&ybuf,
							n,
							&end_slot,
						)
						.context("accuracy")?,
						None => kernels::gpu_argmax_accuracy_into(
							&sc.acts[last],
							&ybuf,
							n,
							k,
							&end_slot,
						)
						.context("argmax accuracy")?,
					},
					None => kernels::gpu_ss_res_into(
						&sc.acts[last],
						&ybuf,
						n * k,
						&end_slot,
					)
					.context("ss_res")?,
				};
				Ok(())
			})
			.transpose()?;
		gpu_core::hw::disarm_saturation_crash();
		gpu_core::callspy::mark_loop_end();
		let hip_loop = hip_snap.map(|_snap| gpu_core::callspy::snapshot());
		let led_loop = hip_snap.map(|_snap| gpu_core::memory::xfer_calls());
		drop(_guard);
		unsafe {
			libc::signal(libc::SIGINT, libc::SIG_DFL);
		}
		drop(ooc);
		let src = base.as_ptr_offset(0);
		let host = self.park(slab, src, prefix_len, sc)?;
		Some(())
			.filter(|_probe| d_sc > 0 && !rerun)
			.map(|_probe| {
				*self.scaler.borrow_mut() = Some(Scaler {
					mean: host[mean_off..mean_off + d_sc].to_vec(),
					std: host[std_off..std_off + d_sc].to_vec(),
				});
			})
			.unwrap_or(());
		let val_of = |m: Metric, e: usize| -> f64 {
			let Some(mi) = ring_row.iter().position(|&d| d == m) else {
				return f64::NAN;
			};
			let raw = host[ring_off + mi * epochs + e];
			match m {
				Metric::R2 => 1.0 - raw / ss_tot,
				Metric::Accuracy => raw,
				Metric::Loss => {
					let sc = ring_scale[mi];
					sc.sign * raw / sc.div
				}
				_other => f64::NAN,
			}
		};
		let key = self.loss.score_key();
		let mut loss_prev = f64::INFINITY;
		let mut ckpt_saved: Option<()> = None;
		for meta in &epoch_meta {
			let e = meta.epoch;
			let score = val_of(stop_metric, e);
			fit_score = Some(())
				.filter(|_probe| score.is_finite())
				.map(|_probe| score)
				.unwrap_or(fit_score);
			let mut checkpointed: Option<()> = None;
			let nan_stop =
				match Some(()).filter(|_probe| checkpointing) {
					Some(_ckpt) => {
						let lossv = val_of(Metric::Loss, e);
						match Some(()).filter(|_probe| lossv.is_nan()) {
							Some(_nan) => {
								drop(Write::err(&format!(
									"NaN loss at epoch {e} — stopping (weights diverged)"
								)));
								Halt::Stop
							}
							None => {
								let carve = ckpt_saved.is_none()
									&& e > 0 && lossv > loss_prev;
								Some(())
									.filter(|_probe| carve)
									.map(|_probe| -> anyhow::Result<()> {
										ckpt_saved = Some(());
										let path = checkpoint_path
											.as_ref()
											.ok_or_else(|| {
												anyhow::anyhow!(
													"checkpoint path"
												)
											})?;
										let better = recipe_infer::saved_score(
											path, key,
										)
										.is_none_or(|best| score > best);
										Some(())
											.filter(|_p| better)
											.map(|_p| -> anyhow::Result<()> {
												recipe_infer::write_ogdl(
												path,
												&plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, score),
											)?;
												checkpointed = Some(());
												Ok(())
											})
											.transpose()?;
										Ok(())
									})
									.transpose()?;
								loss_prev = Some(())
									.filter(|_probe| lossv.is_finite())
									.map(|_probe| lossv)
									.unwrap_or(loss_prev);
								Halt::Continue
							}
						}
					}
					None => Halt::Continue,
				};
			let Halt::Continue = nan_stop else {
				break;
			};
			let logged_flag = matches!(meta.logged, Logged::Yes);
			let Some(_go) = Some(())
				.filter(|_probe| (logged_flag || checkpointed.is_some()) && !plotting)
			else {
				continue;
			};
			for m in cfg.metrics.iter().filter(|m| **m != Metric::Hip) {
				let v = match m {
					Metric::Epoch => e as f64,
					Metric::Lr => self.lr,
					Metric::Time => meta.elapsed,
					_other => val_of(*m, e),
				};
				let flag = match m {
					Metric::Loss => loss,
					Metric::Accuracy => acc,
					Metric::Epoch => epoch,
					Metric::Lr => lr,
					Metric::Time => time,
					Metric::R2 => r2,
					Metric::Hip => device,
				};
				Write::line(flag, &metrics_line(&[*m], &[v]));
			}
			for _ck in checkpointed.iter() {
				Write::line(
					save,
					"<- checkpoint",
				);
			}
		}
		Some(())
			.filter(|_probe| plotting && !epoch_meta.is_empty())
			.map(|_probe| {
				let plot_rows: Vec<Vec<f64>> = epoch_meta
					.iter()
					.map(|meta| {
						let mut row = vec![meta.elapsed];
						for &m in &plot_ys {
							row.push(match m {
								Metric::Lr => self.lr,
								Metric::Hip => f64::NAN,
								_other => val_of(m, meta.epoch),
							});
						}
						row
					})
					.collect();
				let summary_text = summary_graph.as_ref().map(|g| g.serialize()).unwrap_or_default();
				crate::tui::show(&summary_text, &plot_rows, &plot_ys);
			})
			.unwrap_or(());
		let end_val = Some(()).filter(|_probe| !was_interrupted).map(|_probe| {
			Some(())
				.filter(|_p| classify)
				.map(|_p| host[end_off])
				.unwrap_or_else(|| 1.0 - host[end_off] / ss_tot)
		});
		end_val
			.into_iter()
			.filter(|s| s.is_finite())
			.for_each(|s| fit_score = s);
		let neurons: usize = params.iter().map(|p| p.out_dim).sum();
		*self.saved_ogdl.borrow_mut() = Some(crate::model::SavedWeights {
			text: plan.dump_ogdl_host(&host[w_off..w_off + w_len], key, fit_score),
			neurons,
			d,
			c_cat,
			vocab,
		});
		checkpoint_path
			.as_deref()
			.map(|path| -> anyhow::Result<()> {
				let sw = self.saved_ogdl.borrow();
				let text = &sw
					.as_ref()
					.ok_or_else(|| anyhow::anyhow!("fit leaves a mirror"))?
					.text;
				match Some(()).filter(|_probe| was_interrupted) {
					Some(_intr) => {
						let better_on_disk = recipe_infer::saved_score(path, key)
							.is_some_and(|best| {
								!(fit_score.is_finite() && fit_score > best)
							});
						match Some(()).filter(|_probe| better_on_disk) {
							Some(_keep) => Write::line(save, &format!(
								"keeping {path} (better prior {key} on disk)"
							)),
							None => {
								recipe_infer::write_ogdl(path, text)?;
								let full = std::fs::canonicalize(path)
									.unwrap_or_else(|_err| path.into());
								Write::line(save, &format!(
									"saved {} ({key} {fit_score:.4})",
									full.display()
								));
							}
						}
					}
					None => {
						end_val
							.filter(|s| s.is_finite())
							.filter(|&s| {
								recipe_infer::saved_score(path, key)
									.is_none_or(|best| s > best)
							})
							.map(|s| -> anyhow::Result<()> {
								recipe_infer::write_ogdl(path, text)?;
								let full = std::fs::canonicalize(path)
									.unwrap_or_else(|_err| path.into());
								Write::line(save, &format!(
									"saved {} ({neurons} neurons, {key} {s:.4})",
									full.display()
								));
								Ok(())
							})
							.transpose()?;
					}
				}
				Ok(())
			})
			.transpose()?;
		*self.params.borrow_mut() = params;
		self.fit_score.set(fit_score);
		let hip_dump = || -> Option<()> {
			let b0 = hip_snap?;
			let init = hip_init?;
			let lp = hip_loop?;
			let exit = gpu_core::callspy::snapshot();
			Write::line(device, "-- hip init --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&b0, &init).serialize(),
			);
			Write::line(device, "-- hip loop --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&init, &lp).serialize(),
			);
			Write::line(device, "-- hip exit --");
			Write::block(
				device,
				&gpu_core::callspy::report_between(&lp, &exit).serialize(),
			);
			Some(())
		};
		hip_dump();
		let led_dump = || -> Option<()> {
			let s0 = led_snap?;
			let i = led_init?;
			let l = led_loop?;
			let ee = gpu_core::memory::xfer_calls();
			Write::line(device, format!(
				"-- ledger init -- H2D calls {}  D2H calls {}",
				i.h2d - s0.h2d,
				i.d2h - s0.d2h
			));
			Write::line(device, format!(
				"-- ledger loop -- H2D calls {}  D2H calls {}",
				l.h2d - i.h2d,
				l.d2h - i.d2h
			));
			Write::line(device, format!(
				"-- ledger exit -- H2D calls {}  D2H calls {}",
				ee.h2d - l.h2d,
				ee.d2h - l.d2h
			));
			Some(())
		};
		led_dump();
		Ok(())
	}
	fn park(
		&self,
		slab: GpuBuffer,
		src: *mut std::ffi::c_void,
		prefix_len: usize,
		sc: Scratch,
	) -> anyhow::Result<Vec<f64>> {
		let mut host = vec![0.0f64; prefix_len];
		let prefix_bytes = prefix_len * 8;
		let inflight = unsafe {
			gpu_core::memory::exit_d2h_enqueue(src, prefix_bytes)
				.context("exit prefix d2h enqueue")?
		};
		gpu_core::hip::device_synchronize().context("exit drain")?;
		drop(sc);
		inflight.finish(&mut host);
		gpu_core::memory::park_run_backing(slab);
		Ok(host)
	}
	pub(crate) fn prep_eval_input(&self, ds: &Dataset) -> EvalInput {
		let n = ds.x.nrows();
		let embed_first = matches!(self.specs.first(), Some(LayerSpec::Embed(..)));
		let embed_cats = embed_first && ds.text_cols.is_empty() && !ds.onehot_groups.is_empty();
		let coll = Some(())
			.filter(|_probe| embed_cats)
			.map(|_probe| collapse_onehot(ds));
		let eff_x = coll.as_ref().map_or(&ds.x, |c| &c.x);
		let eff_text = match &coll {
			Some(c) => &c.embed_cols,
			None => &ds.text_cols,
		};
		let cat_cols: Vec<usize> = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| {
				Vec::from_iter((0..eff_x.ncols()).filter(|c| !eff_text.contains(c)))
			})
			.unwrap_or_default();
		let xinput = Some(())
			.filter(|_probe| embed_first)
			.map(|_probe| eff_x.select(ndarray::Axis(1), eff_text))
			.unwrap_or_else(|| eff_x.clone());
		let d = xinput.ncols();
		let scaler = self.scaler.borrow();
		let scaler_ref =
			crate::some_or_die(scaler.as_ref(), "eval: missing scaler; train first");
		let up = |m: &ndarray::Array2<f64>| -> GpuBuffer {
			let s = m.as_standard_layout();
			let sl = crate::some_or_die(s.as_slice(), "eval upload: non-contiguous");
			let b = crate::ok_or_die(GpuBuffer::alloc(sl.len()), "eval upload");
			crate::ok_or_die(b.load(sl), "eval upload");
			b
		};
		let apply = |xraw: &GpuBuffer, rows: usize, cols: usize, sc: &Scaler| -> GpuBuffer {
			if sc.mean.len() != cols {
				drop(Write::err(format!(
					"eval: feature count changed: {} vs {cols}",
					sc.mean.len()
				)));
				std::process::abort();
			}
			if sc.std.len() != cols {
				drop(Write::err(format!(
					"eval: feature count changed: {} vs {cols}",
					sc.std.len()
				)));
				std::process::abort();
			}
			let mut st = Stage::new();
			let m_off = st.push(&sc.mean);
			let s_off = st.push(&sc.std);
			let host = st.into_host();
			let img =
				crate::ok_or_die(GpuBuffer::alloc(host.len().max(1)), "eval scaler stage");
			crate::ok_or_die(img.load(&host), "eval scaler stage");
			crate::ok_or_die(
				zscore_apply_views(
					xraw,
					rows,
					cols,
					&img.view(m_off, cols),
					&img.view(s_off, cols),
				),
				"eval zscore",
			)
		};
		match Some(()).filter(|_probe| embed_first) {
			Some(_ef) => {
				let xraw = up(&xinput);
				match Some(()).filter(|_probe| cat_cols.is_empty()) {
					Some(_nocat) => EvalInput {
						x: xraw,
						x_cat: None,
						n,
					},
					None => {
						let cat = eff_x.select(ndarray::Axis(1), &cat_cols);
						let craw = up(&cat);
						let c = cat.ncols();
						EvalInput {
							x: xraw,
							x_cat: Some(apply(&craw, n, c, scaler_ref)),
							n,
						}
					}
				}
			}
			None => {
				let xraw = up(&xinput);
				EvalInput {
					x: apply(&xraw, n, d, scaler_ref),
					x_cat: None,
					n,
				}
			}
		}
	}
}

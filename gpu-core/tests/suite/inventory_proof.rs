use crate::common;

use gpu_core::HipError;
use gpu_core::memory::GpuBuffer;
use std::cmp;
use std::collections::HashMap;
use std::f64::consts;
use std::fs;
use std::mem;
use std::ptr;

type UnaryGpu = fn(&GpuBuffer, usize, &GpuBuffer) -> Result<(), HipError>;
type BinaryGpu = fn(&GpuBuffer, &GpuBuffer, usize, &GpuBuffer) -> Result<(), HipError>;

struct UnaryOp {
	gpu: UnaryGpu,
	oracle: fn(f64) -> f64,
	lo: f64,
	hi: f64,
	tol: f64,
}
struct BinaryOp {
	gpu: BinaryGpu,
	oracle: fn(f64, f64) -> f64,
	lo: f64,
	hi: f64,
	tol: f64,
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
	(0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64)
		.collect()
}

fn run_unary(f: UnaryGpu, x: &[f64]) -> Vec<f64> {
	let b = {
		let __up = x;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(x.len()).unwrap();
	f(&b, x.len(), &o).unwrap();
	let mut out = vec![0.0; x.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}
fn run_binary(f: BinaryGpu, a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = {
		let __up = a;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let bb = {
		let __up = b;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let o = GpuBuffer::alloc(a.len()).unwrap();
	f(&ba, &bb, a.len(), &o).unwrap();
	let mut out = vec![0.0; a.len()];
	unsafe { o.download_async(&mut out, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	out
}

fn unary_registry() -> HashMap<&'static str, UnaryOp> {
	use gpu_core::kernels::{
		gpu_abs_into, gpu_exp, gpu_log_into, gpu_neg, gpu_relu_into, gpu_sigmoid_into,
		gpu_sign_into, gpu_silu_into, gpu_sqrt, gpu_tanh_into,
	};
	use gpu_core::math_ops::{
		gpu_ceil, gpu_cos, gpu_expm1, gpu_floor, gpu_log1p, gpu_reciprocal, gpu_round,
		gpu_rsqrt, gpu_sin, gpu_tan, gpu_trunc,
	};
	let mut m: HashMap<&'static str, UnaryOp> = HashMap::new();
	macro_rules! u {
		($k:literal, $g:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				UnaryOp {
					gpu: $g,
					oracle: $o,
					lo: $lo,
					hi: $hi,
					tol: 1e-9,
				},
			);
		};
	}
	u!("abs", gpu_abs_into, |x| x.abs(), -3.0, 3.0);
	u!("exp", gpu_exp, |x| x.exp(), -3.0, 3.0);
	u!("log", gpu_log_into, |x| x.ln(), 0.05, 5.0);
	u!("sqrt", gpu_sqrt, |x| x.sqrt(), 0.0, 9.0);
	u!("neg", gpu_neg, |x| -x, -3.0, 3.0);
	u!(
		"sign",
		gpu_sign_into,
		|x| if x > 0.0 {
			1.0
		} else if x < 0.0 {
			-1.0
		} else {
			0.0
		},
		-3.0,
		3.0
	);
	u!("relu", gpu_relu_into, |x| x.max(0.0), -3.0, 3.0);
	u!(
		"sigmoid",
		gpu_sigmoid_into,
		|x| 1.0 / (1.0 + (-x).exp()),
		-6.0,
		6.0
	);
	u!("tanh", gpu_tanh_into, |x| x.tanh(), -3.0, 3.0);
	u!("silu", gpu_silu_into, |x| x / (1.0 + (-x).exp()), -6.0, 6.0);
	u!("reciprocal", gpu_reciprocal, |x| 1.0 / x, 0.5, 5.0);
	u!("rsqrt", gpu_rsqrt, |x| 1.0 / x.sqrt(), 0.5, 9.0);
	u!("log1p", gpu_log1p, |x| x.ln_1p(), -0.9, 5.0);
	u!("expm1", gpu_expm1, |x| x.exp_m1(), -3.0, 3.0);
	u!("floor", gpu_floor, |x| x.floor(), -5.0, 5.0);
	u!("ceil", gpu_ceil, |x| x.ceil(), -5.0, 5.0);
	u!("round", gpu_round, |x| x.round(), -5.0, 5.0);
	u!("trunc", gpu_trunc, |x| x.trunc(), -5.0, 5.0);
	u!("sin", gpu_sin, |x| x.sin(), -3.0, 3.0);
	u!("cos", gpu_cos, |x| x.cos(), -3.0, 3.0);
	u!("tan", gpu_tan, |x| x.tan(), -1.2, 1.2);
	use gpu_core::k_mathx::*;
	u!("square", gpu_square, |x| x * x, -3.0, 3.0);
	u!("exp2", gpu_exp2, |x| x.exp2(), -3.0, 3.0);
	u!("log2", gpu_log2, |x| x.log2(), 0.05, 5.0);
	u!("log10", gpu_log10, |x| x.log10(), 0.05, 5.0);
	u!("cbrt", gpu_cbrt, |x| x.cbrt(), -3.0, 3.0);
	u!("sinh", gpu_sinh, |x| x.sinh(), -3.0, 3.0);
	u!("cosh", gpu_cosh, |x| x.cosh(), -3.0, 3.0);
	u!("asin", gpu_asin, |x| x.asin(), -0.99, 0.99);
	u!("acos", gpu_acos, |x| x.acos(), -0.99, 0.99);
	u!("atan", gpu_atan, |x| x.atan(), -3.0, 3.0);
	u!("asinh", gpu_asinh, |x| x.asinh(), -3.0, 3.0);
	u!("acosh", gpu_acosh, |x| x.acosh(), 1.01, 5.0);
	u!("atanh", gpu_atanh, |x| x.atanh(), -0.95, 0.95);
	u!("erf", gpu_erf, |x| libm::erf(x), -3.0, 3.0);
	u!("erfc", gpu_erfc, |x| libm::erfc(x), -3.0, 3.0);
	u!("tgamma", gpu_tgamma, |x| libm::tgamma(x), 0.2, 4.0);
	u!("lgamma", gpu_lgamma, |x| libm::lgamma(x), 0.2, 5.0);
	u!("deg2rad", gpu_deg2rad, |x| x.to_radians(), -180.0, 180.0);
	u!("rad2deg", gpu_rad2deg, |x| x.to_degrees(), -3.0, 3.0);
	use gpu_core::k_gapact::{gpu_hardswish, gpu_mish, gpu_selu, gpu_softplus};
	fn elu1(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
		let a = {
			let __up = &[1.0];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		gpu_core::k_gapact::gpu_elu(x, &a, n, out)
	}
	fn selu_w(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
		let a = {
			let __up = &[1.6732632423543772];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		let l = {
			let __up = &[1.0507009873554805];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		gpu_selu(x, &a, &l, n, out)
	}
	u!(
		"elu",
		elu1,
		|x| if x > 0.0 { x } else { x.exp() - 1.0 },
		-3.0,
		3.0
	);
	u!(
		"selu",
		selu_w,
		|x| {
			let (a, l) = (1.6732632423543772, 1.0507009873554805);
			l * if x > 0.0 { x } else { a * (x.exp() - 1.0) }
		},
		-3.0,
		3.0
	);
	u!(
		"mish",
		gpu_mish,
		|x| x * ((x.max(0.0) + (-x.abs()).exp().ln_1p()).tanh()),
		-3.0,
		3.0
	);
	u!(
		"softplus",
		gpu_softplus,
		|x| x.max(0.0) + (-x.abs()).exp().ln_1p(),
		-3.0,
		3.0
	);
	u!(
		"hardswish",
		gpu_hardswish,
		|x| x * ((x + 3.0).clamp(0.0, 6.0)) / 6.0,
		-4.0,
		4.0
	);
	use gpu_core::k_actx::*;
	fn celu1(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
		let p = {
			let __up = &[1.0];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		gpu_celu(x, &p, n, out)
	}
	fn hardshrink05(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
		let p = {
			let __up = &[0.5];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		gpu_hardshrink(x, &p, n, out)
	}
	fn thresh1(x: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
		let p = {
			let __up = &[1.0];
			let __ub = GpuBuffer::alloc(__up.len())?;
			__ub.load(__up)?;
			__ub
		};
		gpu_thresholdedrelu(x, &p, n, out)
	}
	u!("relu6", gpu_relu6, |x| x.clamp(0.0, 6.0), -4.0, 8.0);
	u!(
		"hardsigmoid",
		gpu_hardsigmoid,
		|x| (x / 6.0 + 0.5).clamp(0.0, 1.0),
		-5.0,
		5.0
	);
	u!("hardtanh", gpu_hardtanh, |x| x.clamp(-1.0, 1.0), -3.0, 3.0);
	u!("softsign", gpu_softsign, |x| x / (1.0 + x.abs()), -3.0, 3.0);
	u!("tanhshrink", gpu_tanhshrink, |x| x - x.tanh(), -3.0, 3.0);
	u!(
		"logsigmoid",
		gpu_logsigmoid,
		|x| -((-x).max(0.0) + (-x.abs()).exp().ln_1p()),
		-3.0,
		3.0
	);
	u!(
		"gelu",
		gpu_gelu_exact,
		|x| 0.5 * x * (1.0 + libm::erf(x * consts::FRAC_1_SQRT_2)),
		-3.0,
		3.0
	);
	u!(
		"softshrink",
		gpu_softshrink,
		|x| if x > 0.5 {
			x - 0.5
		} else if x < -0.5 {
			x + 0.5
		} else {
			0.0
		},
		-3.0,
		3.0
	);
	u!(
		"celu",
		celu1,
		|x| x.max(0.0) + (1.0 * ((x).exp() - 1.0)).min(0.0),
		-3.0,
		3.0
	);
	u!(
		"hardshrink",
		hardshrink05,
		|x| if x.abs() > 0.5 { x } else { 0.0 },
		-3.0,
		3.0
	);
	u!(
		"thresholdedrelu",
		thresh1,
		|x| if x > 1.0 { x } else { 0.0 },
		-3.0,
		3.0
	);
	m
}

fn binary_registry() -> HashMap<&'static str, BinaryOp> {
	use gpu_core::k_gapact::{gpu_geglu, gpu_swiglu};
	use gpu_core::kernels::{gpu_add_into, gpu_div_into, gpu_mul, gpu_sub};
	use gpu_core::math_ops::{gpu_atan2, gpu_fmod, gpu_max, gpu_min};
	let mut m: HashMap<&'static str, BinaryOp> = HashMap::new();
	macro_rules! b {
		($k:literal, $g:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				BinaryOp {
					gpu: $g,
					oracle: $o,
					lo: $lo,
					hi: $hi,
					tol: 1e-9,
				},
			);
		};
	}
	b!("add", gpu_add_into, |a, b| a + b, -3.0, 3.0);
	b!("mul", gpu_mul, |a, b| a * b, -3.0, 3.0);
	b!("sub", gpu_sub, |a, b| a - b, -3.0, 3.0);
	b!("div", gpu_div_into, |a, b| a / b, 0.5, 4.0);
	b!("atan2", gpu_atan2, |a, b| a.atan2(b), -3.0, 3.0);
	b!("fmod", gpu_fmod, |a, b| a % b, 1.0, 4.0);
	b!("maximum", gpu_max, |a, b| a.max(b), -3.0, 3.0);
	b!("minimum", gpu_min, |a, b| a.min(b), -3.0, 3.0);
	b!(
		"swiglu",
		gpu_swiglu,
		|a, b| a * (b / (1.0 + (-b).exp())),
		-3.0,
		3.0
	);
	b!(
		"geglu",
		gpu_geglu,
		|a, b| a * 0.5 * b * (1.0 + libm::erf(b * consts::FRAC_1_SQRT_2)),
		-3.0,
		3.0
	);
	use gpu_core::kernels::{gpu_eq, gpu_gt, gpu_lt};
	b!(
		"gt",
		gpu_gt,
		|a, b| if a > b { 1.0 } else { 0.0 },
		-3.0,
		3.0
	);
	b!(
		"lt",
		gpu_lt,
		|a, b| if a < b { 1.0 } else { 0.0 },
		-3.0,
		3.0
	);
	b!(
		"eq",
		gpu_eq,
		|a, b| if a == b { 1.0 } else { 0.0 },
		-3.0,
		3.0
	);
	m
}

type ReduceGpu = fn(&GpuBuffer, usize) -> Result<f64, HipError>;
struct ReduceOp {
	gpu: ReduceGpu,
	oracle: fn(&[f64]) -> f64,
	lo: f64,
	hi: f64,
	tol: f64,
}
type ScanGpu = fn(&GpuBuffer, usize) -> Result<GpuBuffer, HipError>;
struct ScanOp {
	gpu: ScanGpu,
	oracle: fn(&[f64]) -> Vec<f64>,
	lo: f64,
	hi: f64,
	tol: f64,
}

fn reduce_registry() -> HashMap<&'static str, ReduceOp> {
	use gpu_core::linalg::gpu_dasum;
	use gpu_core::reductions::{
		gpu_l2_norm, gpu_l2_norm_workspace_bytes, gpu_max_all, gpu_max_all_workspace_bytes,
		gpu_mean_all, gpu_mean_all_workspace_bytes, gpu_min_all, gpu_min_all_workspace_bytes,
		gpu_sum_all, gpu_sum_all_workspace_bytes,
	};
	fn red_scalar(x: &GpuBuffer) -> Result<f64, HipError> {
		let mut o = [0.0];
		unsafe { x.download_async(&mut o, ptr::null_mut()) }?;
		gpu_core::hip::device_synchronize()?;
		Ok(o[0])
	}
	fn red_sum(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_sum_all_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(1)?;
		gpu_sum_all(x, &ws, n, &out)?;
		red_scalar(&out)
	}
	fn red_mean(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_mean_all_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(1)?;
		gpu_mean_all(x, &ws, n, &out)?;
		red_scalar(&out)
	}
	fn red_max(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_max_all_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(1)?;
		gpu_max_all(x, &ws, n, &out)?;
		red_scalar(&out)
	}
	fn red_min(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_min_all_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(1)?;
		gpu_min_all(x, &ws, n, &out)?;
		red_scalar(&out)
	}
	fn red_norm(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_l2_norm_workspace_bytes(n))?;
		let sq = GpuBuffer::alloc(n)?;
		let out = GpuBuffer::alloc(1)?;
		gpu_l2_norm(x, &ws, &sq, n, &out)?;
		red_scalar(&out)
	}
	fn red_asum(x: &GpuBuffer, n: usize) -> Result<f64, HipError> {
		let out = GpuBuffer::alloc(1)?;
		gpu_dasum(x, n, &out)?;
		red_scalar(&out)
	}
	let mut m: HashMap<&'static str, ReduceOp> = HashMap::new();
	macro_rules! r {
		($k:literal, $g:expr, $o:expr) => {
			m.insert(
				$k,
				ReduceOp {
					gpu: $g,
					oracle: $o,
					lo: -3.0,
					hi: 3.0,
					tol: 1e-9,
				},
			);
		};
	}
	r!("redsum", red_sum, |x: &[f64]| x.iter().sum());
	r!("redmean", red_mean, |x: &[f64]| x.iter().sum::<f64>()
		/ x.len() as f64);
	r!("redmax", red_max, |x: &[f64]| x
		.iter()
		.cloned()
		.fold(f64::NEG_INFINITY, f64::max));
	r!("redmin", red_min, |x: &[f64]| x
		.iter()
		.cloned()
		.fold(f64::INFINITY, f64::min));
	r!("rednorm", red_norm, |x: &[f64]| x
		.iter()
		.map(|v| v * v)
		.sum::<f64>()
		.sqrt());
	r!("asum", red_asum, |x: &[f64]| x
		.iter()
		.map(|v| v.abs())
		.sum());
	m
}

fn complex_proofs() -> (HashMap<&'static str, bool>, Vec<String>) {
	use gpu_core::kernels::{
		gpu_gemm, gpu_log_softmax_rows, gpu_scale_inplace, gpu_softmax_rows_into,
	};
	use gpu_core::reductions::{gpu_dot, gpu_dot_workspace_bytes};
	let mut m: HashMap<&'static str, bool> = HashMap::new();
	let mut fails: Vec<String> = Vec::new();
	let close = |a: &[f64], b: &[f64]| {
		a.iter()
			.zip(b)
			.all(|(x, y)| (x - y).abs() <= 1e-7 * (1.0 + y.abs()))
	};

	let (mm, kk, nn) = (3usize, 4usize, 2usize);
	let a: Vec<f64> = (0..mm * kk).map(|i| i as f64 * 0.1 - 0.5).collect();
	let b: Vec<f64> = (0..kk * nn).map(|i| i as f64 * 0.2 - 0.3).collect();
	let run_gemm = || {
		let ga = {
			let __up = &a;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gb = {
			let __up = &b;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gc = GpuBuffer::alloc(mm * nn).unwrap();
		gpu_gemm(&ga, &gb, mm, nn, kk, &gc).unwrap();
		let mut o = vec![0.0; mm * nn];
		unsafe { gc.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let c = run_gemm();
	let c2 = run_gemm();
	let mut want = vec![0.0; mm * nn];
	for i in 0..mm {
		for j in 0..nn {
			let mut s = 0.0;
			for p in 0..kk {
				s += a[i * kk + p] * b[p * nn + j];
			}
			want[i * nn + j] = s;
		}
	}
	if !close(&c, &c2) {
		eprintln!("GEMM RETRY DIVERGED: first={c:?} second={c2:?} want={want:?}");
	}
	let ok = close(&c, &want);
	if !ok {
		eprintln!("GEMM WRONG: got={c:?} retry={c2:?} want={want:?}");
	}
	for k in ["gemm", "matmul"] {
		m.insert(k, ok);
	}
	if !ok {
		fails.push("gemm".into());
	}

	let av: Vec<f64> = (0..16).map(|i| i as f64 * 0.1 - 0.5).collect();
	let bv: Vec<f64> = (0..16).map(|i| i as f64 * 0.05 + 0.2).collect();
	let d = {
		let ga = {
			let __up = &av;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gb = {
			let __up = &bv;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let ws = GpuBuffer::alloc_bytes(gpu_dot_workspace_bytes(16)).unwrap();
		let prod = GpuBuffer::alloc(16).unwrap();
		let out = GpuBuffer::alloc(1).unwrap();
		gpu_dot(&ga, &gb, &ws, &prod, 16, &out).unwrap();
		let mut o = [0.0];
		unsafe { out.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o[0]
	};
	let wd: f64 = av.iter().zip(&bv).map(|(x, y)| x * y).sum();
	let ok = (d - wd).abs() <= 1e-9 * (1.0 + wd.abs());
	m.insert("dot", ok);
	if !ok {
		fails.push("dot".into());
	}

	let xs: Vec<f64> = (0..16).map(|i| i as f64 * 0.1 - 0.5).collect();
	let y = {
		let gx = {
			let __up = &xs;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let scalar = {
			let __up = &[2.5];
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		gpu_scale_inplace(&scalar, 16, &gx).unwrap();
		let mut o = vec![0.0; 16];
		unsafe { gx.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let ws: Vec<f64> = xs.iter().map(|v| v * 2.5).collect();
	let ok = close(&y, &ws);
	m.insert("scal", ok);
	if !ok {
		fails.push("scal".into());
	}

	let sx = vec![0.5, -1.0, 2.0, 0.3, 1.1];
	let mx = sx.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
	let ex: Vec<f64> = sx.iter().map(|v| (v - mx).exp()).collect();
	let ssum: f64 = ex.iter().sum();
	let sm = {
		let gx = {
			let __up = &sx;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gy = GpuBuffer::alloc(5).unwrap();
		gpu_softmax_rows_into(&gx, 1, 5, &gy).unwrap();
		let mut o = vec![0.0; 5];
		unsafe { gy.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let ok = close(&sm, &ex.iter().map(|e| e / ssum).collect::<Vec<_>>());
	m.insert("softmax", ok);
	if !ok {
		fails.push("softmax".into());
	}
	let lsm = {
		let gx = {
			let __up = &sx;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gy = GpuBuffer::alloc(5).unwrap();
		gpu_log_softmax_rows(&gx, 1, 5, &gy).unwrap();
		let mut o = vec![0.0; 5];
		unsafe { gy.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let lse = mx + ssum.ln();
	let ok = close(&lsm, &sx.iter().map(|v| v - lse).collect::<Vec<_>>());
	m.insert("log_softmax", ok);
	if !ok {
		fails.push("log_softmax".into());
	}

	use gpu_core::kernels::{gpu_cholesky, gpu_cholesky_workspace_bytes};
	use gpu_core::kernels::{gpu_dgemv_into, gpu_dger_into};
	use gpu_core::linalg::gpu_dsyrk;
	let (gm, gn) = (3usize, 4usize);
	let amat: Vec<f64> = (0..gm * gn).map(|i| i as f64 * 0.1 - 0.6).collect();
	let xv: Vec<f64> = (0..gn).map(|i| i as f64 * 0.3 + 0.2).collect();
	let yv = {
		let ga = {
			let __up = &amat;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gx = {
			let __up = &xv;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gy = GpuBuffer::alloc(gm).unwrap();
		gpu_dgemv_into(&ga, &gx, gm, gn, 0, &gy).unwrap();
		let mut o = vec![0.0; gm];
		unsafe { gy.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let wy: Vec<f64> = (0..gm)
		.map(|i| (0..gn).map(|j| amat[i * gn + j] * xv[j]).sum())
		.collect();
	let ok = close(&yv, &wy);
	m.insert("gemv", ok);
	if !ok {
		fails.push("gemv".into());
	}

	let xo: Vec<f64> = (0..gm).map(|i| i as f64 * 0.2 - 0.1).collect();
	let yo: Vec<f64> = (0..gn).map(|i| i as f64 * 0.15 + 0.3).collect();
	let ao = {
		let gx = {
			let __up = &xo;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gy = {
			let __up = &yo;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let ga = {
			let __zb = GpuBuffer::alloc_bytes(gm * gn * mem::size_of::<f64>()).unwrap();
			__zb.memset_zero(gm * gn * mem::size_of::<f64>()).unwrap();
			__zb
		};
		gpu_dger_into(&gx, &gy, gm, gn, &ga).unwrap();
		let mut o = vec![0.0; gm * gn];
		unsafe { ga.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let mut wo = vec![0.0; gm * gn];
	for i in 0..gm {
		for j in 0..gn {
			wo[i * gn + j] = xo[i] * yo[j];
		}
	}
	let ok = close(&ao, &wo);
	m.insert("ger", ok);
	if !ok {
		fails.push("ger".into());
	}

	let _ = gpu_dsyrk; // syrk convention deferred (ambiguous lda/shape) — task #3

	let cn = 3usize;
	let bm: Vec<f64> = (0..cn * cn)
		.map(|i| ((i * 7 + 1) % 5) as f64 * 0.3 + 0.1)
		.collect();
	let mut ca = vec![0.0; cn * cn];
	for i in 0..cn {
		for j in 0..cn {
			let mut s = 0.0;
			for p in 0..cn {
				s += bm[i * cn + p] * bm[j * cn + p];
			}
			ca[i * cn + j] = s + if i == j { cn as f64 } else { 0.0 };
		}
	}
	let lmat = {
		let ga = {
			let __up = &ca;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let work = GpuBuffer::alloc_bytes(gpu_cholesky_workspace_bytes(cn)).unwrap();
		let info = GpuBuffer::alloc_bytes(mem::size_of::<i32>()).unwrap();
		let gl = GpuBuffer::alloc(cn * cn).unwrap();
		gpu_cholesky(&ga, cn, &work, &info, &gl).unwrap();
		let mut o = vec![0.0; cn * cn];
		unsafe { gl.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let mut rec = vec![0.0; cn * cn];
	for i in 0..cn {
		for j in 0..cn {
			let mut s = 0.0;
			for p in 0..=i.min(j) {
				s += lmat[i * cn + p] * lmat[j * cn + p];
			}
			rec[i * cn + j] = s;
		}
	}
	let ok = close(&rec, &ca);
	m.insert("potrf", ok);
	if !ok {
		fails.push("potrf".into());
	}

	use gpu_core::kernels::gpu_transpose;
	let (tm, tn) = (3usize, 4usize);
	let ta: Vec<f64> = (0..tm * tn).map(|i| i as f64 * 0.25 - 1.0).collect();
	let tb = {
		let ga = {
			let __up = &ta;
			let __ub = GpuBuffer::alloc(__up.len()).unwrap();
			__ub.load(__up).unwrap();
			__ub
		};
		let gt = GpuBuffer::alloc(tm * tn).unwrap();
		gpu_transpose(&ga, tm, tn, &gt).unwrap();
		let mut o = vec![0.0; tm * tn];
		unsafe { gt.download_async(&mut o, ptr::null_mut()) }.unwrap();
		gpu_core::hip::device_synchronize().unwrap();
		o
	};
	let mut wt = vec![0.0; tm * tn];
	for i in 0..tm {
		for j in 0..tn {
			wt[j * tm + i] = ta[i * tn + j];
		}
	}
	let ok = close(&tb, &wt);
	m.insert("transpose", ok);
	if !ok {
		fails.push("transpose".into());
	}

	(m, fails)
}

fn scan_registry() -> HashMap<&'static str, ScanOp> {
	use gpu_core::reductions::{
		gpu_cummax, gpu_cummax_workspace_bytes, gpu_cumprod, gpu_cumprod_workspace_bytes,
		gpu_cumsum_rows,
	};
	fn scan_cumsum(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		let out = GpuBuffer::alloc(n)?;
		gpu_cumsum_rows(x, 1, n, &out)?;
		Ok(out)
	}
	fn scan_cumprod(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_cumprod_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(n)?;
		gpu_cumprod(x, &ws, n, &out)?;
		Ok(out)
	}
	fn scan_cummax(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		let ws = GpuBuffer::alloc_bytes(gpu_cummax_workspace_bytes(n))?;
		let out = GpuBuffer::alloc(n)?;
		gpu_cummax(x, &ws, n, &out)?;
		Ok(out)
	}
	let mut m: HashMap<&'static str, ScanOp> = HashMap::new();
	macro_rules! s {
		($k:literal, $g:expr, $o:expr, $lo:expr, $hi:expr) => {
			m.insert(
				$k,
				ScanOp {
					gpu: $g,
					oracle: $o,
					lo: $lo,
					hi: $hi,
					tol: 1e-7,
				},
			);
		};
	}
	s!(
		"cumsum",
		scan_cumsum,
		|x: &[f64]| {
			let mut a = 0.0;
			x.iter()
				.map(|v| {
					a += v;
					a
				})
				.collect()
		},
		-3.0,
		3.0
	);
	s!(
		"cumprod",
		scan_cumprod,
		|x: &[f64]| {
			let mut a = 1.0;
			x.iter()
				.map(|v| {
					a *= v;
					a
				})
				.collect()
		},
		0.5,
		1.5
	);
	s!(
		"cummax",
		scan_cummax,
		|x: &[f64]| {
			let mut a = f64::NEG_INFINITY;
			x.iter()
				.map(|v| {
					a = a.max(*v);
					a
				})
				.collect()
		},
		-3.0,
		3.0
	);
	m
}

fn canon(name: &str) -> String {
	let mut base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	for p in [
		"rocsolver_",
		"cusolver_",
		"cusparse_",
		"hipsparse_",
		"rocblas_",
		"hipblaslt_",
		"hipblas_",
		"cublaslt_",
		"cublas_",
		"rocblas",
		"hipblas",
		"cublas",
		"mkl_",
		"clblast_",
	] {
		if let Some(s) = base.strip_prefix(p) {
			base = s.to_string();
			break;
		}
	}
	let blas_ops = [
		"gemm", "gemv", "gbmv", "symv", "syrk", "syr2k", "trsm", "trmm", "trsv", "potrf",
		"potrs", "getrf", "getrs", "gesvd", "gesdd", "gesv", "geqrf", "syev", "syevd", "heevd",
		"axpy", "scal", "dotu", "dotc", "dot", "nrm2", "asum", "iamax", "ger", "gerc", "geru",
	];
	let remap = |op: &str| -> String {
		match op {
			"nrm2" => "rednorm",
			"dotu" | "dotc" => "dot",
			"geru" | "gerc" => "ger",
			_ => op,
		}
		.to_string()
	};
	if base.len() >= 3 {
		let (f, rest) = base.split_at(1);
		if matches!(f, "s" | "d" | "c" | "z" | "h") && blas_ops.contains(&rest) {
			return remap(rest);
		}
	}
	if blas_ops.contains(&base.as_str()) {
		return remap(&base);
	}
	let alias: &[(&str, &str)] = &[
		("absolute", "abs"),
		("negative", "neg"),
		("negate", "neg"),
		("multiply", "mul"),
		("subtract", "sub"),
		("divide", "div"),
		("true_divide", "div"),
		("truediv", "div"),
		("sgn", "sign"),
		("swish", "silu"),
		("clip", "clamp"),
		("rint", "round"),
		("nearbyint", "round"),
		("arcsin", "asin"),
		("arccos", "acos"),
		("arctan", "atan"),
		("arcsinh", "asinh"),
		("arccosh", "acosh"),
		("arctanh", "atanh"),
		("arctan2", "atan2"),
		("gamma", "tgamma"),
		("gammaln", "lgamma"),
		("lngamma", "lgamma"),
		("radians", "deg2rad"),
		("degrees", "rad2deg"),
		("hard_swish", "hardswish"),
		("hardsigmoid", "hardswish_NOPE"),
		("exp_2", "exp2"),
		("log_2", "log2"),
		("log_10", "log10"),
		("sum", "redsum"),
		("reduce_sum", "redsum"),
		("nansum", "redsum"),
		("mean", "redmean"),
		("reduce_mean", "redmean"),
		("nanmean", "redmean"),
		("max", "redmax"),
		("amax", "redmax"),
		("reduce_max", "redmax"),
		("min", "redmin"),
		("amin", "redmin"),
		("reduce_min", "redmin"),
		("norm", "rednorm"),
		("l2_norm", "rednorm"),
		("frobenius_norm", "rednorm"),
		("cumulative_sum", "cumsum"),
		("cumulative_prod", "cumprod"),
		("cumulative_max", "cummax"),
		("hard_sigmoid", "hardsigmoid"),
		("hard_tanh", "hardtanh"),
		("log_sigmoid", "logsigmoid"),
		("logsigmoid", "logsigmoid"),
		("tanh_shrink", "tanhshrink"),
		("soft_sign", "softsign"),
		("soft_shrink", "softshrink"),
		("hard_shrink", "hardshrink"),
		("relu_6", "relu6"),
		("thresholded_relu", "thresholdedrelu"),
		("thresholdedrelu", "thresholdedrelu"),
		("matmul", "gemm"),
		("mm", "gemm"),
		("bmm", "gemm"),
		("baddbmm", "gemm_NOPE"),
		("inner", "dot"),
		("vdot", "dot"),
		("dotproduct", "dot"),
		("logsoftmax", "log_softmax"),
		("log_softmax", "log_softmax"),
		("cholesky", "potrf"),
		("cholesky_decomposition", "potrf"),
		("outer", "ger"),
		("greater", "gt"),
		("less", "lt"),
		("equal", "eq"),
		("transpose", "transpose"),
		("swapaxes", "transpose"),
		("matrix_transpose", "transpose"),
	];
	for (a, c) in alias {
		if base == *a {
			return c.to_string();
		}
	}
	base
}

fn load_inventory() -> Vec<(String, String)> {
	let dir = common::inventory_dir();
	let mut items = Vec::new();
	let Ok(rd) = fs::read_dir(&dir) else {
		panic!("no kernel_inventory at {dir}");
	};
	for e in rd.flatten() {
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = fs::read_to_string(&p) else {
				continue;
			};
			let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) else {
				continue;
			};
			if let Some(ks) = v.get("kernels").and_then(|k| k.as_array()) {
				for k in ks {
					let name = k
						.get("name")
						.and_then(|n| n.as_str())
						.unwrap_or("")
						.to_string();
					let cat = k
						.get("category")
						.and_then(|c| c.as_str())
						.unwrap_or("?")
						.to_string();
					if !name.is_empty() {
						items.push((name, cat));
					}
				}
			}
		}
	}
	items
}

#[test]
fn prove_inventory() {
	let items = load_inventory();
	assert!(!items.is_empty(), "inventory empty");
	let uni = unary_registry();
	let bin = binary_registry();
	let red = reduce_registry();
	let scan = scan_registry();
	let (complex, complex_fails) = complex_proofs();

	let n = 24usize;
	let mut proven = 0usize;
	let mut failures: Vec<String> = Vec::new();
	let mut per_cat: HashMap<String, (usize, usize)> = HashMap::new(); // cat -> (proven, total)

	for (name, cat) in &items {
		let entry = per_cat.entry(cat.clone()).or_insert((0, 0));
		entry.1 += 1;
		let key = canon(name);
		if let Some(op) = uni.get(key.as_str()) {
			let xs = probes(op.lo, op.hi, n);
			let got = run_unary(op.gpu, &xs);
			let ok = xs.iter().zip(&got).all(|(x, g)| {
				let want = (op.oracle)(*x);
				want.is_finite() && (g - want).abs() <= op.tol * (1.0 + want.abs())
			});
			if ok {
				proven += 1;
				entry.0 += 1;
			} else {
				failures.push(format!("{name} (unary {key})"));
			}
		} else if let Some(op) = bin.get(key.as_str()) {
			let a = probes(op.lo, op.hi, n);
			let b: Vec<f64> = probes(op.lo + 0.7, op.hi + 0.7, n);
			let got = run_binary(op.gpu, &a, &b);
			let ok = a.iter().zip(&b).zip(&got).all(|((x, y), g)| {
				let want = (op.oracle)(*x, *y);
				want.is_finite() && (g - want).abs() <= op.tol * (1.0 + want.abs())
			});
			if ok {
				proven += 1;
				entry.0 += 1;
			} else {
				failures.push(format!("{name} (binary {key})"));
			}
		} else if let Some(op) = red.get(key.as_str()) {
			let xs = probes(op.lo, op.hi, n);
			let buf = {
				let __up = &xs;
				let __ub = GpuBuffer::alloc(__up.len()).unwrap();
				__ub.load(__up).unwrap();
				__ub
			};
			let got = (op.gpu)(&buf, xs.len()).unwrap();
			let want = (op.oracle)(&xs);
			if want.is_finite() && (got - want).abs() <= op.tol * (1.0 + want.abs()) {
				proven += 1;
				entry.0 += 1;
			} else {
				failures.push(format!("{name} (reduce {key}): got {got} want {want}"));
			}
		} else if let Some(op) = scan.get(key.as_str()) {
			let xs = probes(op.lo, op.hi, n);
			let buf = {
				let __up = &xs;
				let __ub = GpuBuffer::alloc(__up.len()).unwrap();
				__ub.load(__up).unwrap();
				__ub
			};
			let g = (op.gpu)(&buf, xs.len()).unwrap();
			let mut got = vec![0.0; xs.len()];
			unsafe { g.download_async(&mut got, ptr::null_mut()) }.unwrap();
			gpu_core::hip::device_synchronize().unwrap();
			let want = (op.oracle)(&xs);
			let ok = got
				.iter()
				.zip(&want)
				.all(|(a, b)| b.is_finite() && (a - b).abs() <= op.tol * (1.0 + b.abs()));
			if ok {
				proven += 1;
				entry.0 += 1;
			} else {
				failures.push(format!("{name} (scan {key})"));
			}
		} else if let Some(&ok) = complex.get(key.as_str())
			&& ok
		{
			proven += 1;
			entry.0 += 1;
		}
	}
	failures.extend(complex_fails);

	let total = items.len();
	let mut cats: Vec<_> = per_cat.iter().collect();
	cats.sort_by_key(|(_, (p, _))| cmp::Reverse(*p));
	eprintln!("\n=== inventory proof coverage ===");
	for (c, (p, t)) in cats.iter().take(20) {
		if *p > 0 {
			eprintln!("  {:<22} {:>5}/{:<6} proven", c, p, t);
		}
	}
	eprintln!("  --------------------------------");
	eprintln!(
		"  TOTAL PROVEN: {proven} / {total} inventory items ({:.2}%)",
		100.0 * proven as f64 / total as f64
	);
	eprintln!(
		"  registered ops: {} unary, {} binary, {} reduce, {} scan",
		uni.len(),
		bin.len(),
		red.len(),
		scan.len()
	);

	assert!(
		failures.is_empty(),
		"{} registered op(s) FAILED to match oracle: {:?}",
		failures.len(),
		&failures[..failures.len().min(20)]
	);
	assert!(proven > 0, "zero items proven — registry/canon broken");
}

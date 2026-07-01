mod common;
// Data-driven proof harness: for every item in kernel_inventory/*.json, if its
// canonical op name is registered here, run the gpu-core op on the LIVE GPU and
// assert it matches a CPU oracle. Prints proven/total coverage per category.
//
// "Proving an item operates the same" == its semantics (per the JSON description)
// reproduced on-device within tolerance. Coverage grows as ops are registered and
// gaps implemented. The test FAILS only on a registered-op mismatch (a real bug);
// unmapped items are reported as the remaining backlog, not failures.

use gpu_core::hip::HipError;
use gpu_core::memory::GpuBuffer;
use std::collections::HashMap;

type UnaryGpu = fn(&GpuBuffer, usize) -> Result<GpuBuffer, HipError>;
type BinaryGpu = fn(&GpuBuffer, &GpuBuffer, usize) -> Result<GpuBuffer, HipError>;

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
	let b = GpuBuffer::upload(x).unwrap();
	let o = f(&b, x.len()).unwrap();
	let mut out = vec![0.0; x.len()];
	o.download(&mut out).unwrap();
	out
}
fn run_binary(f: BinaryGpu, a: &[f64], b: &[f64]) -> Vec<f64> {
	let ba = GpuBuffer::upload(a).unwrap();
	let bb = GpuBuffer::upload(b).unwrap();
	let o = f(&ba, &bb, a.len()).unwrap();
	let mut out = vec![0.0; a.len()];
	o.download(&mut out).unwrap();
	out
}

fn unary_registry() -> HashMap<&'static str, UnaryOp> {
	use gpu_core::kernels::{
		gpu_abs, gpu_exp, gpu_log, gpu_neg, gpu_relu, gpu_sigmoid, gpu_sign, gpu_silu,
		gpu_sqrt, gpu_tanh,
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
	u!("abs", gpu_abs, |x| x.abs(), -3.0, 3.0);
	u!("exp", gpu_exp, |x| x.exp(), -3.0, 3.0);
	u!("log", gpu_log, |x| x.ln(), 0.05, 5.0);
	u!("sqrt", gpu_sqrt, |x| x.sqrt(), 0.0, 9.0);
	u!("neg", gpu_neg, |x| -x, -3.0, 3.0);
	u!(
		"sign",
		gpu_sign,
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
	u!("relu", gpu_relu, |x| x.max(0.0), -3.0, 3.0);
	u!(
		"sigmoid",
		gpu_sigmoid,
		|x| 1.0 / (1.0 + (-x).exp()),
		-6.0,
		6.0
	);
	u!("tanh", gpu_tanh, |x| x.tanh(), -3.0, 3.0);
	u!("silu", gpu_silu, |x| x / (1.0 + (-x).exp()), -6.0, 6.0);
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
	// mathx gaps
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
	// gap activations
	use gpu_core::k_gapact::{gpu_hardswish, gpu_mish, gpu_selu, gpu_softplus};
	fn elu1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		gpu_core::k_gapact::gpu_elu(x, n, 1.0)
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
		gpu_selu,
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
	// actx gap activations
	use gpu_core::k_actx::*;
	fn celu1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		gpu_celu(x, n, 1.0)
	}
	fn hardshrink05(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		gpu_hardshrink(x, n, 0.5)
	}
	fn thresh1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		gpu_thresholdedrelu(x, n, 1.0)
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
		|x| 0.5 * x * (1.0 + libm::erf(x * std::f64::consts::FRAC_1_SQRT_2)),
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
	use gpu_core::kernels::{gpu_add, gpu_div, gpu_mul, gpu_sub};
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
	b!("add", gpu_add, |a, b| a + b, -3.0, 3.0);
	b!("mul", gpu_mul, |a, b| a * b, -3.0, 3.0);
	b!("sub", gpu_sub, |a, b| a - b, -3.0, 3.0);
	b!("div", gpu_div, |a, b| a / b, 0.5, 4.0);
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
		|a, b| a * 0.5 * b * (1.0 + libm::erf(b * std::f64::consts::FRAC_1_SQRT_2)),
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
		gpu_l2_norm, gpu_max_all, gpu_mean_all, gpu_min_all, gpu_sum_all,
	};
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
	r!("redsum", gpu_sum_all, |x: &[f64]| x.iter().sum());
	r!("redmean", gpu_mean_all, |x: &[f64]| x.iter().sum::<f64>()
		/ x.len() as f64);
	r!("redmax", gpu_max_all, |x: &[f64]| x
		.iter()
		.cloned()
		.fold(f64::NEG_INFINITY, f64::max));
	r!("redmin", gpu_min_all, |x: &[f64]| x
		.iter()
		.cloned()
		.fold(f64::INFINITY, f64::min));
	r!("rednorm", gpu_l2_norm, |x: &[f64]| x
		.iter()
		.map(|v| v * v)
		.sum::<f64>()
		.sqrt());
	r!("asum", gpu_dasum, |x: &[f64]| x
		.iter()
		.map(|v| v.abs())
		.sum());
	m
}

// Complex proofs (matmul/dot/scal/softmax): run ONCE, map all alias keys to the pass/fail bool.
fn complex_proofs() -> (HashMap<&'static str, bool>, Vec<String>) {
	use gpu_core::kernels::{gpu_gemm, gpu_log_softmax_rows, gpu_scale, gpu_softmax_rows};
	use gpu_core::linalg::gpu_ddot;
	let mut m: HashMap<&'static str, bool> = HashMap::new();
	let mut fails: Vec<String> = Vec::new();
	let close = |a: &[f64], b: &[f64]| {
		a.iter()
			.zip(b)
			.all(|(x, y)| (x - y).abs() <= 1e-7 * (1.0 + y.abs()))
	};

	// gemm: C(m,n) = A(m,k)·B(k,n)
	let (mm, kk, nn) = (3usize, 4usize, 2usize);
	let a: Vec<f64> = (0..mm * kk).map(|i| i as f64 * 0.1 - 0.5).collect();
	let b: Vec<f64> = (0..kk * nn).map(|i| i as f64 * 0.2 - 0.3).collect();
	let run_gemm = || {
		let ga = GpuBuffer::upload(&a).unwrap();
		let gb = GpuBuffer::upload(&b).unwrap();
		let gc = gpu_gemm(&ga, &gb, mm, nn, kk).unwrap();
		let mut o = vec![0.0; mm * nn];
		gc.download(&mut o).unwrap();
		o
	};
	// Fault probe: the first Dgemm of a process intermittently returns garbage
	// when another GPU process ran moments before. Run it twice — a transient
	// (clock/init) glitch passes on retry; persistent corruption repeats.
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

	// dot
	let av: Vec<f64> = (0..16).map(|i| i as f64 * 0.1 - 0.5).collect();
	let bv: Vec<f64> = (0..16).map(|i| i as f64 * 0.05 + 0.2).collect();
	let d = {
		let ga = GpuBuffer::upload(&av).unwrap();
		let gb = GpuBuffer::upload(&bv).unwrap();
		gpu_ddot(&ga, &gb, 16).unwrap()
	};
	let wd: f64 = av.iter().zip(&bv).map(|(x, y)| x * y).sum();
	let ok = (d - wd).abs() <= 1e-9 * (1.0 + wd.abs());
	m.insert("dot", ok);
	if !ok {
		fails.push("dot".into());
	}

	// scal: y = alpha·x
	let xs: Vec<f64> = (0..16).map(|i| i as f64 * 0.1 - 0.5).collect();
	let y = {
		let gx = GpuBuffer::upload(&xs).unwrap();
		let gy = gpu_scale(&gx, 2.5, 16).unwrap();
		let mut o = vec![0.0; 16];
		gy.download(&mut o).unwrap();
		o
	};
	let ws: Vec<f64> = xs.iter().map(|v| v * 2.5).collect();
	let ok = close(&y, &ws);
	m.insert("scal", ok);
	if !ok {
		fails.push("scal".into());
	}

	// softmax + log_softmax (1 row)
	let sx = vec![0.5, -1.0, 2.0, 0.3, 1.1];
	let mx = sx.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
	let ex: Vec<f64> = sx.iter().map(|v| (v - mx).exp()).collect();
	let ssum: f64 = ex.iter().sum();
	let sm = {
		let gx = GpuBuffer::upload(&sx).unwrap();
		let gy = gpu_softmax_rows(&gx, 1, 5).unwrap();
		let mut o = vec![0.0; 5];
		gy.download(&mut o).unwrap();
		o
	};
	let ok = close(&sm, &ex.iter().map(|e| e / ssum).collect::<Vec<_>>());
	m.insert("softmax", ok);
	if !ok {
		fails.push("softmax".into());
	}
	let lsm = {
		let gx = GpuBuffer::upload(&sx).unwrap();
		let gy = gpu_log_softmax_rows(&gx, 1, 5).unwrap();
		let mut o = vec![0.0; 5];
		gy.download(&mut o).unwrap();
		o
	};
	let lse = mx + ssum.ln();
	let ok = close(&lsm, &sx.iter().map(|v| v - lse).collect::<Vec<_>>());
	m.insert("log_softmax", ok);
	if !ok {
		fails.push("log_softmax".into());
	}

	// gemv: y(m) = A(m,n)·x(n)
	use gpu_core::kernels::gpu_cholesky;
	use gpu_core::linalg::{gpu_dgemv, gpu_dger, gpu_dsyrk};
	let (gm, gn) = (3usize, 4usize);
	let amat: Vec<f64> = (0..gm * gn).map(|i| i as f64 * 0.1 - 0.6).collect();
	let xv: Vec<f64> = (0..gn).map(|i| i as f64 * 0.3 + 0.2).collect();
	let yv = {
		let ga = GpuBuffer::upload(&amat).unwrap();
		let gx = GpuBuffer::upload(&xv).unwrap();
		let gy = gpu_dgemv(&ga, &gx, gm, gn, false).unwrap();
		let mut o = vec![0.0; gm];
		gy.download(&mut o).unwrap();
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

	// ger: A(m,n) = x(m)·y(n)ᵀ
	let xo: Vec<f64> = (0..gm).map(|i| i as f64 * 0.2 - 0.1).collect();
	let yo: Vec<f64> = (0..gn).map(|i| i as f64 * 0.15 + 0.3).collect();
	let ao = {
		let gx = GpuBuffer::upload(&xo).unwrap();
		let gy = GpuBuffer::upload(&yo).unwrap();
		let ga = gpu_dger(&gx, &gy, gm, gn).unwrap();
		let mut o = vec![0.0; gm * gn];
		ga.download(&mut o).unwrap();
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

	// potrf (cholesky): A SPD, L lower s.t. L·Lᵀ = A
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
		let ga = GpuBuffer::upload(&ca).unwrap();
		let gl = gpu_cholesky(&ga, cn).unwrap();
		let mut o = vec![0.0; cn * cn];
		gl.download(&mut o).unwrap();
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

	// transpose: B(n,m) = A(m,n)ᵀ
	use gpu_core::kernels::gpu_transpose;
	let (tm, tn) = (3usize, 4usize);
	let ta: Vec<f64> = (0..tm * tn).map(|i| i as f64 * 0.25 - 1.0).collect();
	let tb = {
		let ga = GpuBuffer::upload(&ta).unwrap();
		let gt = gpu_transpose(&ga, tm, tn).unwrap();
		let mut o = vec![0.0; tm * tn];
		gt.download(&mut o).unwrap();
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
	use gpu_core::reductions::{gpu_cummax, gpu_cumprod};
	fn cumsum1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> {
		gpu_core::reductions::gpu_cumsum_rows(x, 1, n)
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
		cumsum1,
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
		gpu_cumprod,
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
		gpu_cummax,
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

// JSON name -> canonical registry key.
fn canon(name: &str) -> String {
	let mut base = name
		.rsplit(['.', ':', '$'])
		.next()
		.unwrap_or(name)
		.to_lowercase();
	// strip vendor library prefixes (longest first)
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
	// BLAS/LAPACK ops carry a dtype letter (s/d/c/z/h). Strip it for an EXACT op match
	// (so fused variants like gemm_bias / gemm_ex stay distinct and aren't over-claimed).
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
		// reductions
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
		// scans
		("cumulative_sum", "cumsum"),
		("cumulative_prod", "cumprod"),
		("cumulative_max", "cummax"),
		// more activations
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
		// linalg
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
	let Ok(rd) = std::fs::read_dir(&dir) else {
		panic!("no kernel_inventory at {dir}");
	};
	for e in rd.flatten() {
		let p = e.path();
		if p.extension().is_some_and(|x| x == "json") {
			let Ok(txt) = std::fs::read_to_string(&p) else {
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
			let buf = GpuBuffer::upload(&xs).unwrap();
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
			let buf = GpuBuffer::upload(&xs).unwrap();
			let g = (op.gpu)(&buf, xs.len()).unwrap();
			let mut got = vec![0.0; xs.len()];
			g.download(&mut got).unwrap();
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

	// Report
	let total = items.len();
	let mut cats: Vec<_> = per_cat.iter().collect();
	cats.sort_by_key(|(_, (p, _))| std::cmp::Reverse(*p));
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

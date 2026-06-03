// Data-driven proof for the elementwise_unary category on a LIVE gfx1101 GPU.
// For every canonical op we register a GPU fn + an AUTHORITATIVE CPU oracle
// (std f64 / libm / scipy-textbook formula) and assert byte-for-byte agreement
// within tol over a per-op input domain. Predicate ops (isnan/isinf/isfinite/
// signbit) are fed pathological inputs (+/-inf, NaN, -0.0) so the assertion is
// not vacuously true. New ops live in src/kernels/elementwise_unaryx.hip and are
// reached through the auto-wired launch_elementwise_unaryx_* C symbols.

use gpu_core::memory::GpuBuffer;
use gpu_core::hip::HipError;
use std::collections::BTreeMap;
use std::ffi::c_void;

// ── FFI for the NEW tail-gap kernels (compiled from elementwise_unaryx.hip) ──
unsafe extern "C" {
    fn launch_elementwise_unaryx_sinc(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_frac(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_heaviside(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_signbit(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_isnan(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_isinf(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_isfinite(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_logit(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_expit(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_positive(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
    fn launch_elementwise_unaryx_exp10(x: *const c_void, out: *mut c_void, n: i32, s: *mut c_void);
}

type Launch = unsafe extern "C" fn(*const c_void, *mut c_void, i32, *mut c_void);

fn run_new(f: Launch, x: &[f64]) -> Vec<f64> {
    let b = GpuBuffer::upload(x).unwrap();
    let o = GpuBuffer::alloc(x.len()).unwrap();
    unsafe { f(b.ptr_raw() as *const c_void, o.ptr_raw(), x.len() as i32, std::ptr::null_mut()); }
    gpu_core::hip::check(unsafe { gpu_core::hip::hipGetLastError() }).unwrap();
    let mut out = vec![0.0; x.len()];
    o.download(&mut out).unwrap();
    out
}

type UnaryGpu = fn(&GpuBuffer, usize) -> Result<GpuBuffer, HipError>;
fn run_existing(f: UnaryGpu, x: &[f64]) -> Vec<f64> {
    let b = GpuBuffer::upload(x).unwrap();
    let o = f(&b, x.len()).unwrap();
    let mut out = vec![0.0; x.len()];
    o.download(&mut out).unwrap();
    out
}

fn probes(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    (0..n).map(|i| lo + (hi - lo) * (i as f64 + 0.5) / n as f64).collect()
}

enum Gpu { Existing(UnaryGpu), New(Launch) }
struct Op { gpu: Gpu, oracle: fn(f64) -> f64, input: Vec<f64>, tol: f64 }

fn registry() -> BTreeMap<&'static str, Op> {
    use gpu_core::kernels::{gpu_abs, gpu_exp, gpu_log, gpu_sqrt, gpu_neg, gpu_sign, gpu_relu, gpu_sigmoid, gpu_tanh, gpu_silu, gpu_leaky_relu};
    use gpu_core::k_actx::gpu_gelu_exact;
    use gpu_core::math_ops::{gpu_reciprocal, gpu_rsqrt, gpu_log1p, gpu_expm1, gpu_floor, gpu_ceil, gpu_round, gpu_trunc, gpu_sin, gpu_cos, gpu_tan};
    use gpu_core::k_mathx::{gpu_square, gpu_exp2, gpu_log2, gpu_log10, gpu_cbrt, gpu_sinh, gpu_cosh, gpu_asin, gpu_acos, gpu_atan, gpu_asinh, gpu_acosh, gpu_atanh, gpu_erf, gpu_erfc, gpu_lgamma, gpu_deg2rad, gpu_rad2deg};
    use gpu_core::k_gapact::{gpu_selu, gpu_mish, gpu_softplus, gpu_hardswish};
    use gpu_core::k_actx::{gpu_relu6, gpu_hardsigmoid, gpu_softsign};

    let std_probe = probes(-3.0, 3.0, 64);
    let pos_probe = probes(0.05, 5.0, 64);
    let dom1 = probes(-0.95, 0.95, 64);          // (-1,1) for asin/acos/atanh
    let gt1 = probes(1.01, 5.0, 64);             // (1,inf) for acosh
    let prob01 = probes(0.02, 0.98, 64);         // (0,1) for logit
    // pathological inputs for predicate ops — without these the assert is vacuous
    let patho: Vec<f64> = vec![f64::NEG_INFINITY, -3.5, -1.0, -0.0, 0.0, 1.0, 2.5, f64::INFINITY, f64::NAN];

    let mut m: BTreeMap<&'static str, Op> = BTreeMap::new();
    macro_rules! e { ($k:literal, $g:expr, $o:expr, $in:expr) => { m.insert($k, Op{gpu:Gpu::Existing($g), oracle:$o, input:$in.clone(), tol:1e-7}); }; }
    macro_rules! nw { ($k:literal, $g:expr, $o:expr, $in:expr) => { m.insert($k, Op{gpu:Gpu::New($g), oracle:$o, input:$in.clone(), tol:1e-7}); }; }

    // ── existing ops (49) ──
    e!("abs",        gpu_abs,        |x| x.abs(),        std_probe);
    e!("acos",       gpu_acos,       |x| x.acos(),       dom1);
    e!("acosh",      gpu_acosh,      |x| x.acosh(),      gt1);
    e!("asin",       gpu_asin,       |x| x.asin(),       dom1);
    e!("asinh",      gpu_asinh,      |x| x.asinh(),      std_probe);
    e!("atan",       gpu_atan,       |x| x.atan(),       std_probe);
    e!("atanh",      gpu_atanh,      |x| x.atanh(),      dom1);
    e!("cbrt",       gpu_cbrt,       |x| x.cbrt(),       std_probe);
    e!("ceil",       gpu_ceil,       |x| x.ceil(),       std_probe);
    e!("cos",        gpu_cos,        |x| x.cos(),        std_probe);
    e!("cosh",       gpu_cosh,       |x| x.cosh(),       std_probe);
    e!("deg2rad",    gpu_deg2rad,    |x| x.to_radians(), probes(-180.0,180.0,64));
    e!("erf",        gpu_erf,        |x| libm::erf(x),   std_probe);
    e!("erfc",       gpu_erfc,       |x| libm::erfc(x),  std_probe);
    e!("exp",        gpu_exp,        |x| x.exp(),        std_probe);
    e!("exp2",       gpu_exp2,       |x| x.exp2(),       std_probe);
    e!("expm1",      gpu_expm1,      |x| x.exp_m1(),     std_probe);
    e!("floor",      gpu_floor,      |x| x.floor(),      std_probe);
    e!("gelu",       gpu_gelu_exact, |x| 0.5*x*(1.0+libm::erf(x*0.7071067811865476)), std_probe);
    e!("hardsigmoid",gpu_hardsigmoid,|x| (x/6.0+0.5).clamp(0.0,1.0), probes(-5.0,5.0,64));
    e!("hardswish",  gpu_hardswish,  |x| x*((x+3.0).clamp(0.0,6.0))/6.0, probes(-4.0,4.0,64));
    e!("lgamma",     gpu_lgamma,     |x| libm::lgamma(x), pos_probe);
    e!("log",        gpu_log,        |x| x.ln(),         pos_probe);
    e!("log10",      gpu_log10,      |x| x.log10(),      pos_probe);
    e!("log1p",      gpu_log1p,      |x| x.ln_1p(),      probes(-0.9,5.0,64));
    e!("log2",       gpu_log2,       |x| x.log2(),       pos_probe);
    e!("mish",       gpu_mish,       |x| x*((x.max(0.0)+(-x.abs()).exp().ln_1p()).tanh()), std_probe);
    e!("neg",        gpu_neg,        |x| -x,             std_probe);
    e!("rad2deg",    gpu_rad2deg,    |x| x.to_degrees(), std_probe);
    e!("reciprocal", gpu_reciprocal, |x| 1.0/x,          pos_probe);
    e!("relu",       gpu_relu,       |x| x.max(0.0),     std_probe);
    e!("relu6",      gpu_relu6,      |x| x.clamp(0.0,6.0), probes(-4.0,8.0,64));
    e!("round",      gpu_round,      |x| x.round(),      std_probe);
    e!("rsqrt",      gpu_rsqrt,      |x| 1.0/x.sqrt(),   pos_probe);
    e!("selu",       gpu_selu,       |x| { let (a,l)=(1.6732632423543772,1.0507009873554805); l*if x>0.0 {x} else {a*(x.exp()-1.0)} }, std_probe);
    e!("sigmoid",    gpu_sigmoid,    |x| 1.0/(1.0+(-x).exp()), probes(-6.0,6.0,64));
    e!("sign",       gpu_sign,       |x| if x>0.0 {1.0} else if x<0.0 {-1.0} else {0.0}, std_probe);
    e!("silu",       gpu_silu,       |x| x/(1.0+(-x).exp()), probes(-6.0,6.0,64));
    e!("sin",        gpu_sin,        |x| x.sin(),        std_probe);
    e!("sinh",       gpu_sinh,       |x| x.sinh(),       std_probe);
    e!("softplus",   gpu_softplus,   |x| x.max(0.0)+(-x.abs()).exp().ln_1p(), std_probe);
    e!("softsign",   gpu_softsign,   |x| x/(1.0+x.abs()), std_probe);
    e!("sqrt",       gpu_sqrt,       |x| x.sqrt(),       probes(0.0,9.0,64));
    e!("square",     gpu_square,     |x| x*x,            std_probe);
    e!("tan",        gpu_tan,        |x| x.tan(),        probes(-1.2,1.2,64));
    e!("tanh",       gpu_tanh,       |x| x.tanh(),       std_probe);
    e!("trunc",      gpu_trunc,      |x| x.trunc(),      std_probe);
    {
        // elu/leaky_relu take a param; wrap to the standard unary form
        fn elu1(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_core::k_gapact::gpu_elu(x, n, 1.0) }
        fn lrelu(x: &GpuBuffer, n: usize) -> Result<GpuBuffer, HipError> { gpu_leaky_relu(x, n, 0.01) }
        e!("elu",        elu1,  |x| if x>0.0 {x} else {x.exp()-1.0}, std_probe);
        e!("leaky_relu", lrelu, |x| if x>0.0 {x} else {0.01*x},      std_probe);
    }

    // ── new tail-gap ops (8 from inventory + 3 task extras) ──
    nw!("frac",      launch_elementwise_unaryx_frac,      |x| x - x.trunc(), std_probe);
    nw!("heaviside", launch_elementwise_unaryx_heaviside, |x| if x<0.0 {0.0} else if x>0.0 {1.0} else {0.5}, std_probe);
    nw!("sinc",      launch_elementwise_unaryx_sinc,      |x| { if x==0.0 {1.0} else { let p=std::f64::consts::PI*x; p.sin()/p } }, std_probe);
    nw!("positive",  launch_elementwise_unaryx_positive,  |x| x, std_probe);
    nw!("logit",     launch_elementwise_unaryx_logit,     |x| (x/(1.0-x)).ln(), prob01);
    nw!("expit",     launch_elementwise_unaryx_expit,     |x| 1.0/(1.0+(-x).exp()), probes(-6.0,6.0,64));
    nw!("exp10",     launch_elementwise_unaryx_exp10,     |x| 10f64.powf(x), std_probe);
    // predicate ops: pathological inputs make the assertion non-vacuous
    nw!("isnan",     launch_elementwise_unaryx_isnan,     |x| if x.is_nan() {1.0} else {0.0}, patho);
    nw!("isinf",     launch_elementwise_unaryx_isinf,     |x| if x.is_infinite() {1.0} else {0.0}, patho);
    nw!("isfinite",  launch_elementwise_unaryx_isfinite,  |x| if x.is_finite() {1.0} else {0.0}, patho);
    nw!("signbit",   launch_elementwise_unaryx_signbit,   |x| if x.is_sign_negative() {1.0} else {0.0}, patho);
    m
}

fn close(a: f64, b: f64, tol: f64) -> bool {
    if a.is_nan() && b.is_nan() { return true; }
    if a == b { return true; }                       // exact (incl inf == inf)
    if a.is_infinite() || b.is_infinite() { return false; }
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

#[test]
fn prove_elementwise_unary() {
    let reg = registry();
    let total = reg.len();
    let mut proven = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (name, op) in &reg {
        let got = match &op.gpu {
            Gpu::Existing(f) => run_existing(*f, &op.input),
            Gpu::New(f)      => run_new(*f, &op.input),
        };
        let mut ok = true;
        for (i, &xv) in op.input.iter().enumerate() {
            let want = (op.oracle)(xv);
            if !close(got[i], want, op.tol) {
                ok = false;
                failures.push(format!("{name}: f({xv})=GPU {} vs oracle {} (Δ={:e})", got[i], want, (got[i]-want).abs()));
                break;
            }
        }
        if ok { proven += 1; }
    }

    let implemented = "sinc,frac,heaviside,signbit,isnan,isinf,isfinite,logit,expit,positive,exp10";
    eprintln!("PROVE elementwise_unary: {proven} / {total}");
    for f in &failures { eprintln!("  FAIL {f}"); }
    assert!(failures.is_empty(), "{} op(s) mismatched their oracle: {:?}", failures.len(), failures);
    assert_eq!(proven, total, "every registered op must match its oracle");
    eprintln!("RESULT elementwise_unary: proven={proven} total={total} green=true implemented={implemented}");
}

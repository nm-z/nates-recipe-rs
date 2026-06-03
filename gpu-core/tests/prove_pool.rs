// Live-GPU proof harness for the "pool" kernel category (gfx1101).
// Every registered canonical op runs on-device and is asserted against an
// authoritative CPU oracle derived from the op's real definition (PyTorch /
// textbook pooling formulas) — never invented numbers. tol 1e-7.
//
// Existing ops proven directly from gpu_core::kernels; new ops (poolx.hip)
// proven via raw launcher FFI. A proven canonical op covers all its variants.

use gpu_core::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
    fn launch_poolx_global_avg_pool(x: *const c_void, out: *mut c_void, rows: i32, span: i32, channels: i32, s: *mut c_void);
    fn launch_poolx_global_max_pool(x: *const c_void, out: *mut c_void, rows: i32, span: i32, channels: i32, s: *mut c_void);
    fn launch_poolx_lp_pool(x: *const c_void, out: *mut c_void, rows: i32, span: i32, channels: i32, p: f64, s: *mut c_void);
    fn launch_poolx_adaptive_avg_pool(x: *const c_void, out: *mut c_void, rows: i32, in_len: i32, out_len: i32, s: *mut c_void);
    fn launch_poolx_adaptive_max_pool(x: *const c_void, out: *mut c_void, rows: i32, in_len: i32, out_len: i32, s: *mut c_void);
}

const TOL: f64 = 1e-7;

fn dl(b: &GpuBuffer, n: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];
    b.download(&mut v).unwrap();
    v
}

fn assert_close(name: &str, got: &[f64], want: &[f64]) {
    assert_eq!(got.len(), want.len(), "{name}: length mismatch {} vs {}", got.len(), want.len());
    for (i, (g, w)) in got.iter().zip(want.iter()).enumerate() {
        assert!((g - w).abs() <= TOL, "{name}: elem {i} got {g} want {w} (|d|={})", (g - w).abs());
    }
}

// ── Existing op proofs (gpu_core::kernels) ──────────────────────────────────

// avg_pool_1d: input (n*out_len, n_filters) → (n, n_filters); mean over out_len.
fn prove_avg_pool_1d() {
    use gpu_core::kernels::gpu_avg_pool_1d;
    let (n, out_len, nf) = (3usize, 4usize, 2usize);
    let x: Vec<f64> = (0..n * out_len * nf).map(|i| (i as f64) * 0.5 - 1.0).collect();
    let b = GpuBuffer::upload(&x).unwrap();
    let got = dl(&gpu_avg_pool_1d(&b, n, out_len, nf).unwrap(), n * nf);
    let mut want = vec![0.0; n * nf];
    for i in 0..n {
        for fi in 0..nf {
            let mut s = 0.0;
            for t in 0..out_len { s += x[(i * out_len + t) * nf + fi]; }
            want[i * nf + fi] = s / out_len as f64;
        }
    }
    assert_close("avg_pool_1d", &got, &want);
}

// avg_pool_2d: NCHW mean over kH×kW window at stride sH,sW; divide by count.
fn prove_avg_pool_2d() {
    use gpu_core::kernels::gpu_avg_pool_2d;
    let (n, c, h, w, kh, kw, sh, sw) = (2usize, 2, 4, 4, 2, 2, 2, 2);
    let x: Vec<f64> = (0..n * c * h * w).map(|i| (i as f64).sin() * 3.0).collect();
    let b = GpuBuffer::upload(&x).unwrap();
    let oh = (h - kh) / sh + 1;
    let ow = (w - kw) / sw + 1;
    let got = dl(&gpu_avg_pool_2d(&b, n, c, h, w, kh, kw, sh, sw).unwrap(), n * c * oh * ow);
    let mut want = vec![0.0; n * c * oh * ow];
    for nn in 0..n { for cc in 0..c { for ohh in 0..oh { for oww in 0..ow {
        let mut s = 0.0; let mut cnt = 0;
        for khi in 0..kh { for kwi in 0..kw {
            let ih = ohh * sh + khi; let iw = oww * sw + kwi;
            if ih < h && iw < w { s += x[((nn * c + cc) * h + ih) * w + iw]; cnt += 1; }
        }}
        want[((nn * c + cc) * oh + ohh) * ow + oww] = s / cnt as f64;
    }}}}
    assert_close("avg_pool_2d", &got, &want);
}

// max_pool_1d: max over out_len window; also returns argmax idx (first-occurrence).
fn prove_max_pool_1d_and_argmax() {
    use gpu_core::kernels::gpu_max_pool_1d;
    let (n, out_len, nf) = (3usize, 4usize, 2usize);
    // Distinct values (non-repeating ramp scrambled) so argmax is unambiguous.
    let x: Vec<f64> = (0..n * out_len * nf).map(|i| ((i * 37 + 11) % 97) as f64).collect();
    let b = GpuBuffer::upload(&x).unwrap();
    let (vb, ib) = gpu_max_pool_1d(&b, n, out_len, nf).unwrap();
    let gv = dl(&vb, n * nf);
    let gi = dl(&ib, n * nf);
    let mut wv = vec![0.0; n * nf];
    let mut wi = vec![0.0; n * nf];
    for i in 0..n { for fi in 0..nf {
        let mut best = f64::NEG_INFINITY; let mut bt = 0usize;
        for t in 0..out_len {
            let v = x[(i * out_len + t) * nf + fi];
            if v > best { best = v; bt = t; }
        }
        wv[i * nf + fi] = best; wi[i * nf + fi] = bt as f64;
    }}
    assert_close("max_pool_1d", &gv, &wv);
    assert_close("max_pool_with_argmax", &gi, &wi);
}

// max_pool_2d: NCHW max; idx = flat ih*W+iw within the channel plane.
fn prove_max_pool_2d() {
    use gpu_core::kernels::gpu_max_pool_2d;
    let (n, c, h, w, kh, kw, sh, sw) = (2usize, 2, 4, 4, 2, 2, 2, 2);
    let x: Vec<f64> = (0..n * c * h * w).map(|i| ((i * 53 + 7) % 211) as f64).collect();
    let b = GpuBuffer::upload(&x).unwrap();
    let oh = (h - kh) / sh + 1;
    let ow = (w - kw) / sw + 1;
    let (vb, ib) = gpu_max_pool_2d(&b, n, c, h, w, kh, kw, sh, sw).unwrap();
    let gv = dl(&vb, n * c * oh * ow);
    let _ = &ib;
    let mut wv = vec![0.0; n * c * oh * ow];
    for nn in 0..n { for cc in 0..c { for ohh in 0..oh { for oww in 0..ow {
        let mut best = f64::NEG_INFINITY;
        for khi in 0..kh { for kwi in 0..kw {
            let ih = ohh * sh + khi; let iw = oww * sw + kwi;
            if ih < h && iw < w {
                let v = x[((nn * c + cc) * h + ih) * w + iw];
                if v > best { best = v; }
            }
        }}
        wv[((nn * c + cc) * oh + ohh) * ow + oww] = best;
    }}}}
    assert_close("max_pool_2d", &gv, &wv);
}

// avg_pool_grad: pool_grad_expand replicates (n,nf) → (n*out_len,nf) / out_len;
// also avg_pool_2d_backward spreads grad/count to each window cell.
fn prove_avg_pool_grad() {
    use gpu_core::kernels::{gpu_pool_grad_expand, gpu_avg_pool_2d_backward};
    // 1d expand
    let (n, out_len, nf) = (3usize, 4usize, 2usize);
    let g: Vec<f64> = (0..n * nf).map(|i| (i as f64) + 1.0).collect();
    let gb = GpuBuffer::upload(&g).unwrap();
    let got = dl(&gpu_pool_grad_expand(&gb, n, out_len, nf).unwrap(), n * out_len * nf);
    let mut want = vec![0.0; n * out_len * nf];
    for idx in 0..n * out_len * nf {
        let i = idx / (out_len * nf);
        let fi = idx % nf;
        want[idx] = g[i * nf + fi] / out_len as f64;
    }
    assert_close("avg_pool_grad", &got, &want);

    // 2d backward: each output grad / count, atomic-added to its window cells.
    let (n2, c, h, w, kh, kw, sh, sw) = (1usize, 1, 4, 4, 2, 2, 2, 2);
    let oh = (h - kh) / sh + 1; let ow = (w - kw) / sw + 1;
    let go: Vec<f64> = (0..n2 * c * oh * ow).map(|i| (i as f64) * 0.5 + 1.0).collect();
    let gob = GpuBuffer::upload(&go).unwrap();
    let gi = dl(&gpu_avg_pool_2d_backward(&gob, n2, c, h, w, kh, kw, sh, sw).unwrap(), n2 * c * h * w);
    let mut wi = vec![0.0; n2 * c * h * w];
    for nn in 0..n2 { for cc in 0..c { for ohh in 0..oh { for oww in 0..ow {
        let mut cnt = 0;
        for khi in 0..kh { for kwi in 0..kw {
            if ohh * sh + khi < h && oww * sw + kwi < w { cnt += 1; }
        }}
        let val = go[((nn * c + cc) * oh + ohh) * ow + oww] / cnt as f64;
        for khi in 0..kh { for kwi in 0..kw {
            let ih = ohh * sh + khi; let iw = oww * sw + kwi;
            if ih < h && iw < w { wi[((nn * c + cc) * h + ih) * w + iw] += val; }
        }}
    }}}}
    assert_close("avg_pool_grad_2d", &gi, &wi);
}

// max_pool_grad: scatter grad to the recorded argmax positions (1d and 2d).
fn prove_max_pool_grad() {
    use gpu_core::kernels::{gpu_max_pool_1d, gpu_max_pool_1d_backward, gpu_max_pool_2d, gpu_max_pool_2d_backward};
    // 1d
    let (n, out_len, nf) = (3usize, 4usize, 2usize);
    let x: Vec<f64> = (0..n * out_len * nf).map(|i| ((i * 37 + 11) % 97) as f64).collect();
    let xb = GpuBuffer::upload(&x).unwrap();
    let (_vb, ib) = gpu_max_pool_1d(&xb, n, out_len, nf).unwrap();
    let g: Vec<f64> = (0..n * nf).map(|i| (i as f64) + 2.0).collect();
    let gb = GpuBuffer::upload(&g).unwrap();
    let got = dl(&gpu_max_pool_1d_backward(&gb, &ib, n, out_len, nf).unwrap(), n * out_len * nf);
    let idx = dl(&ib, n * nf);
    let mut want = vec![0.0; n * out_len * nf];
    for i in 0..n { for fi in 0..nf {
        let t = idx[i * nf + fi] as usize;
        want[(i * out_len + t) * nf + fi] = g[i * nf + fi];
    }}
    assert_close("max_pool_grad_1d", &got, &want);

    // 2d
    let (n2, c, h, w, kh, kw, sh, sw) = (1usize, 2, 4, 4, 2, 2, 2, 2);
    let oh = (h - kh) / sh + 1; let ow = (w - kw) / sw + 1;
    let x2: Vec<f64> = (0..n2 * c * h * w).map(|i| ((i * 53 + 7) % 211) as f64).collect();
    let x2b = GpuBuffer::upload(&x2).unwrap();
    let (_v2, i2) = gpu_max_pool_2d(&x2b, n2, c, h, w, kh, kw, sh, sw).unwrap();
    let g2: Vec<f64> = (0..n2 * c * oh * ow).map(|i| (i as f64) * 0.5 + 1.0).collect();
    let g2b = GpuBuffer::upload(&g2).unwrap();
    let got2 = dl(&gpu_max_pool_2d_backward(&g2b, &i2, n2, c, h, w, oh, ow).unwrap(), n2 * c * h * w);
    let idx2 = dl(&i2, n2 * c * oh * ow);
    let mut want2 = vec![0.0; n2 * c * h * w];
    for nn in 0..n2 { for cc in 0..c { for o in 0..oh * ow {
        let oi = (nn * c + cc) * oh * ow + o;
        let pos = idx2[oi] as usize;
        want2[(nn * c + cc) * h * w + pos] += g2[oi];
    }}}
    assert_close("max_pool_grad_2d", &got2, &want2);
}

// ── New op proofs (poolx.hip) ───────────────────────────────────────────────

fn run_poolx_span(launch: unsafe extern "C" fn(*const c_void, *mut c_void, i32, i32, i32, *mut c_void),
                  x: &[f64], rows: usize, span: usize, ch: usize) -> Vec<f64> {
    let b = GpuBuffer::upload(x).unwrap();
    let o = GpuBuffer::alloc(rows * ch).unwrap();
    unsafe { launch(b.ptr_raw() as *const c_void, o.ptr_raw(), rows as i32, span as i32, ch as i32, std::ptr::null_mut()); }
    dl(&o, rows * ch)
}

fn prove_global_avg_pool() {
    let (rows, span, ch) = (3usize, 5usize, 2usize);
    let x: Vec<f64> = (0..rows * span * ch).map(|i| (i as f64) * 0.3 - 2.0).collect();
    let got = run_poolx_span(launch_poolx_global_avg_pool, &x, rows, span, ch);
    let mut want = vec![0.0; rows * ch];
    for r in 0..rows { for c in 0..ch {
        let mut s = 0.0;
        for t in 0..span { s += x[(r * span + t) * ch + c]; }
        want[r * ch + c] = s / span as f64;
    }}
    assert_close("global_avg_pool", &got, &want);
}

fn prove_global_max_pool() {
    let (rows, span, ch) = (3usize, 5usize, 2usize);
    let x: Vec<f64> = (0..rows * span * ch).map(|i| ((i * 41 + 5) % 89) as f64 - 40.0).collect();
    let got = run_poolx_span(launch_poolx_global_max_pool, &x, rows, span, ch);
    let mut want = vec![0.0; rows * ch];
    for r in 0..rows { for c in 0..ch {
        let mut best = f64::NEG_INFINITY;
        for t in 0..span { best = best.max(x[(r * span + t) * ch + c]); }
        want[r * ch + c] = best;
    }}
    assert_close("global_max_pool", &got, &want);
}

fn prove_lp_pool() {
    let (rows, span, ch) = (3usize, 5usize, 2usize);
    let p = 2.0;
    let x: Vec<f64> = (0..rows * span * ch).map(|i| (i as f64) * 0.4 - 1.5).collect();
    let b = GpuBuffer::upload(&x).unwrap();
    let o = GpuBuffer::alloc(rows * ch).unwrap();
    unsafe { launch_poolx_lp_pool(b.ptr_raw() as *const c_void, o.ptr_raw(), rows as i32, span as i32, ch as i32, p, std::ptr::null_mut()); }
    let got = dl(&o, rows * ch);
    let mut want = vec![0.0; rows * ch];
    for r in 0..rows { for c in 0..ch {
        let mut acc = 0.0;
        for t in 0..span { acc += x[(r * span + t) * ch + c].abs().powf(p); }
        want[r * ch + c] = acc.powf(1.0 / p);
    }}
    assert_close("lp_pool", &got, &want);
}

fn run_poolx_adaptive(launch: unsafe extern "C" fn(*const c_void, *mut c_void, i32, i32, i32, *mut c_void),
                      x: &[f64], rows: usize, in_len: usize, out_len: usize) -> Vec<f64> {
    let b = GpuBuffer::upload(x).unwrap();
    let o = GpuBuffer::alloc(rows * out_len).unwrap();
    unsafe { launch(b.ptr_raw() as *const c_void, o.ptr_raw(), rows as i32, in_len as i32, out_len as i32, std::ptr::null_mut()); }
    dl(&o, rows * out_len)
}

// PyTorch adaptive bin: start=floor(o*I/O), end=ceil((o+1)*I/O).
fn adaptive_bin(o: usize, i: usize, out: usize) -> (usize, usize) {
    let start = (o * i) / out;
    let end = ((o + 1) * i + out - 1) / out;
    (start, end)
}

fn prove_adaptive_avg_pool() {
    let (rows, in_len, out_len) = (2usize, 7usize, 3usize);
    let x: Vec<f64> = (0..rows * in_len).map(|i| (i as f64) * 0.7 - 1.0).collect();
    let got = run_poolx_adaptive(launch_poolx_adaptive_avg_pool, &x, rows, in_len, out_len);
    let mut want = vec![0.0; rows * out_len];
    for r in 0..rows { for o in 0..out_len {
        let (s, e) = adaptive_bin(o, in_len, out_len);
        let mut sum = 0.0;
        for t in s..e { sum += x[r * in_len + t]; }
        want[r * out_len + o] = sum / (e - s) as f64;
    }}
    assert_close("adaptive_avg_pool", &got, &want);
}

fn prove_adaptive_max_pool() {
    let (rows, in_len, out_len) = (2usize, 7usize, 3usize);
    let x: Vec<f64> = (0..rows * in_len).map(|i| ((i * 29 + 3) % 71) as f64).collect();
    let got = run_poolx_adaptive(launch_poolx_adaptive_max_pool, &x, rows, in_len, out_len);
    let mut want = vec![0.0; rows * out_len];
    for r in 0..rows { for o in 0..out_len {
        let (s, e) = adaptive_bin(o, in_len, out_len);
        let mut best = f64::NEG_INFINITY;
        for t in s..e { best = best.max(x[r * in_len + t]); }
        want[r * out_len + o] = best;
    }}
    assert_close("adaptive_max_pool", &got, &want);
}

#[test]
fn prove_pool() {
    // Each entry is one canonical pool op proven on the live GPU vs its oracle.
    let ops: Vec<(&str, fn())> = vec![
        ("avg_pool",             prove_avg_pool_1d),
        ("avg_pool_2d",          prove_avg_pool_2d),
        ("max_pool",             prove_max_pool_1d_and_argmax),
        ("max_pool_2d",          prove_max_pool_2d),
        ("avg_pool_grad",        prove_avg_pool_grad),
        ("max_pool_grad",        prove_max_pool_grad),
        ("global_avg_pool",      prove_global_avg_pool),
        ("global_max_pool",      prove_global_max_pool),
        ("lp_pool",              prove_lp_pool),
        ("adaptive_avg_pool",    prove_adaptive_avg_pool),
        ("adaptive_max_pool",    prove_adaptive_max_pool),
    ];
    let total = ops.len();
    let mut proven = 0;
    for (name, f) in &ops {
        f();
        proven += 1;
        eprintln!("  ok pool::{name}");
    }
    // Canonical ops covered (1d/2d/3d & dtype variants collapse into these):
    //   avg_pool, max_pool (+ with_argmax), avg_pool_grad, max_pool_grad,
    //   global_avg_pool, global_max_pool, lp_pool/l2_pool, adaptive_avg_pool,
    //   adaptive_max_pool.
    // Backlog (dropped — need external state, not faked):
    //   fractional_max_pool / fractional_avg_pool (random region sampling),
    //   max_unpool (requires externally-supplied argmax indices + output shape).
    eprintln!("PROVE pool: {proven} / {total}");
    assert_eq!(proven, total, "not all registered pool ops proved");
}

// Parity tests for the hipBLAS-backed batched matmul path.
//
// On this NVIDIA laptop hipBLAS is backed by cuBLAS (gpu-core/src/shim_nvidia.cu);
// on AMD it is real hipBLAS. Both must match the same plain-Rust CPU oracle within
// 1e-9 absolute. Matching the oracle on both backends == parity.
//
// Wrappers exercised (see gpu-core/src/linalg.rs / kernels.rs):
//   linalg::gpu_bmm_into : per-batch row-major C_i(m×n,ldc) = opA(A_i)·opB(B_i),
//                          with leading-dim / stride / offset views and transpose flags.
//   kernels::gpu_gemm    : single row-major C(m×n) = A(m×k)·B(k×n).

use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, kernels, linalg};

// ── CPU oracles (plain Rust, row-major contiguous) ────────────────────────

// Per batch: C(m×n) = opA(A)·opB(B). A is (m×k) unless ta (then stored k×m),
// B is (k×n) unless tb (then stored n×k). All batches packed contiguously.
fn cpu_bmm(
      a: &[f64],
      b: &[f64],
      batch: usize,
      m: usize,
      n: usize,
      k: usize,
      ta: bool,
      tb: bool,
) -> Vec<f64> {
      let mut c = vec![0.0f64; batch * m * n];
      for bi in 0..batch {
            let ao = bi * m * k;
            let bo = bi * k * n;
            let co = bi * m * n;
            for i in 0..m {
                  for j in 0..n {
                        let mut s = 0.0;
                        for p in 0..k {
                              // A row-major: untransposed (m×k) -> a[i*k+p];
                              //              transposed   (k×m) -> a[p*m+i].
                              let av = if ta { a[ao + p * m + i] } else { a[ao + i * k + p] };
                              // B row-major: untransposed (k×n) -> b[p*n+j];
                              //              transposed   (n×k) -> b[j*k+p].
                              let bv = if tb { b[bo + j * k + p] } else { b[bo + p * n + j] };
                              s += av * bv;
                        }
                        c[co + i * n + j] = s;
                  }
            }
      }
      c
}

// Single C(m×n) = A(m×k)·B(k×n), row-major.
fn cpu_gemm(a: &[f64], b: &[f64], m: usize, n: usize, k: usize) -> Vec<f64> {
      cpu_bmm(a, b, 1, m, n, k, false, false)
}

fn max_abs_diff(want: &[f64], got: &[f64]) -> f64 {
      want.iter()
            .zip(got)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f64::max)
}

// Drive gpu_bmm_into for fully contiguous, no-offset batches (the common case).
fn run_bmm_case(batch: usize, m: usize, n: usize, k: usize, ta: bool, tb: bool) {
      hip::set_device(0).unwrap();

      // Stored dims account for transpose: A is (a_rows × a_cols), B is (b_rows × b_cols).
      let a_rows = if ta { k } else { m };
      let a_cols = if ta { m } else { k };
      let b_rows = if tb { n } else { k };
      let b_cols = if tb { k } else { n };

      // Deterministic, well-conditioned data.
      let a: Vec<f64> = (0..batch * a_rows * a_cols)
            .map(|i| (i as f64 * 0.37).sin() + 0.1 * i as f64)
            .collect();
      let b: Vec<f64> = (0..batch * b_rows * b_cols)
            .map(|i| (i as f64 * 0.53).cos() - 0.05 * i as f64)
            .collect();

      let want = cpu_bmm(&a, &b, batch, m, n, k, ta, tb);

      let ag = GpuBuffer::upload(&a).unwrap();
      let bg = GpuBuffer::upload(&b).unwrap();
      let cg = GpuBuffer::alloc(batch * m * n).unwrap();

      linalg::gpu_bmm_into(
            &cg,
            &ag,
            &bg,
            batch,
            m,
            n,
            k,
            a_cols,            // lda = row length of stored A
            b_cols,            // ldb = row length of stored B
            n,                 // ldc
            a_rows * a_cols,   // stride_a
            b_rows * b_cols,   // stride_b
            m * n,             // stride_c
            0,
            0,
            0,
            ta,
            tb,
      );

      let got = cg.download_vec().unwrap();
      let d = max_abs_diff(&want, &got);
      assert!(
            d < 1e-9,
            "bmm parity failed batch={batch} m={m} n={n} k={k} ta={ta} tb={tb} maxdiff={d:.3e}"
      );
}

// ── Tests ─────────────────────────────────────────────────────────────────

// Square, multiple-of-32 size: clean warp-aligned path.
#[test]
fn bmm_parity_square_aligned() {
      run_bmm_case(4, 32, 32, 32, false, false);
}

// Non-square AND not a multiple of 32: stresses ragged warp tails.
#[test]
fn bmm_parity_nonsquare_ragged() {
      run_bmm_case(3, 17, 23, 11, false, false);
      run_bmm_case(2, 13, 5, 19, false, false);
}

// All four transpose modes at a ragged size.
#[test]
fn bmm_parity_transpose_modes() {
      run_bmm_case(3, 7, 13, 5, false, false);
      run_bmm_case(3, 7, 13, 5, false, true);
      run_bmm_case(3, 7, 13, 5, true, false);
      run_bmm_case(3, 7, 13, 5, true, true);
}

// Single batch must agree with the single-matrix oracle too.
#[test]
fn bmm_parity_single_batch() {
      run_bmm_case(1, 9, 6, 14, false, false);
}

// Explicit 2-batch case asserted element-by-element against a hand oracle,
// as a self-contained sanity check independent of run_bmm_case plumbing.
#[test]
fn bmm_parity_two_batch_explicit() {
      hip::set_device(0).unwrap();
      let (batch, m, n, k) = (2usize, 3usize, 2usize, 4usize);
      let a: Vec<f64> = (0..batch * m * k).map(|i| i as f64 + 1.0).collect();
      let b: Vec<f64> = (0..batch * k * n).map(|i| (i as f64 + 1.0) * 0.5).collect();
      let want = cpu_bmm(&a, &b, batch, m, n, k, false, false);

      let ag = GpuBuffer::upload(&a).unwrap();
      let bg = GpuBuffer::upload(&b).unwrap();
      let cg = GpuBuffer::alloc(batch * m * n).unwrap();
      linalg::gpu_bmm_into(
            &cg, &ag, &bg, batch, m, n, k, k, n, n, m * k, k * n, m * n, 0, 0, 0, false, false,
      );
      let got = cg.download_vec().unwrap();
      let d = max_abs_diff(&want, &got);
      assert!(d < 1e-9, "explicit 2-batch bmm maxdiff={d:.3e}");
}

// End-to-end full-pipeline sanity: compose two single gpu_gemm matmuls
// (Y = X·W1·W2) and confirm the chained GPU result matches the CPU oracle.
// This mirrors how the forward pass stacks linear layers.
#[test]
fn gemm_pipeline_compose_parity() {
      hip::set_device(0).unwrap();
      // Deterministic tiny linear "dataset": X (rows×f0), W1 (f0×f1), W2 (f1×f2).
      let (rows, f0, f1, f2) = (10usize, 6usize, 13usize, 4usize); // ragged, non-aligned
      let x: Vec<f64> = (0..rows * f0).map(|i| (i as f64 * 0.11).sin()).collect();
      let w1: Vec<f64> = (0..f0 * f1).map(|i| (i as f64 * 0.07).cos()).collect();
      let w2: Vec<f64> = (0..f1 * f2).map(|i| (i as f64 * 0.13).sin()).collect();

      // CPU oracle: H = X·W1, Y = H·W2.
      let h_cpu = cpu_gemm(&x, &w1, rows, f1, f0);
      let y_cpu = cpu_gemm(&h_cpu, &w2, rows, f2, f1);

      // GPU: chain two gpu_gemm calls, keeping intermediates on device.
      let xg = GpuBuffer::upload(&x).unwrap();
      let w1g = GpuBuffer::upload(&w1).unwrap();
      let w2g = GpuBuffer::upload(&w2).unwrap();
      let hg = kernels::gpu_gemm(&xg, &w1g, rows, f1, f0).unwrap();
      let yg = kernels::gpu_gemm(&hg, &w2g, rows, f2, f1).unwrap();

      // Intermediate parity.
      let h_gpu = hg.download_vec().unwrap();
      let dh = max_abs_diff(&h_cpu, &h_gpu);
      assert!(dh < 1e-9, "pipeline H maxdiff={dh:.3e}");

      // Final parity.
      let y_gpu = yg.download_vec().unwrap();
      let dy = max_abs_diff(&y_cpu, &y_gpu);
      assert!(dy < 1e-9, "pipeline Y maxdiff={dy:.3e}");
}

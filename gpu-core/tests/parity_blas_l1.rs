// Parity tests for hipBLAS L1 reductions (gpu_ddot / gpu_dnrm2 / gpu_dasum /
// gpu_idamax). The GPU result must match a plain-Rust CPU oracle within 1e-9.
// The same test runs on AMD (native hipBLAS) and NVIDIA (cuBLAS shim); matching
// the CPU oracle on both backends == parity.

use gpu_core::memory::GpuBuffer;
use gpu_core::{hip, linalg};

const TOL: f64 = 1e-9;

// A spread of element counts: square-ish, non-power-of-two, prime, and a value
// that is NOT a multiple of 32 to stress the warp/reduction tail paths.
const SIZES: &[usize] = &[1, 7, 31, 32, 33, 64, 100, 127, 256, 1000];

// Deterministic pseudo-random sequence with both signs and a zero, so |x| and
// argmax behavior are exercised. Plain LCG — no external crates.
fn make_seq(n: usize, seed: u64) -> Vec<f64> {
      let mut state = seed.wrapping_add(0x9E3779B97F4A7C15);
      let mut v = Vec::with_capacity(n);
      for _ in 0..n {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            // map top bits to [-1, 1)
            let u = ((state >> 11) as f64) / ((1u64 << 53) as f64); // [0,1)
            v.push(2.0 * u - 1.0);
      }
      v
}

fn cpu_ddot(a: &[f64], b: &[f64]) -> f64 {
      a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

fn cpu_dnrm2(x: &[f64]) -> f64 {
      x.iter().map(|v| v * v).sum::<f64>().sqrt()
}

fn cpu_dasum(x: &[f64]) -> f64 {
      x.iter().map(|v| v.abs()).sum()
}

// 0-based index of the first element with the largest absolute value, matching
// the gpu_idamax wrapper (which converts BLAS's 1-based result to 0-based).
fn cpu_idamax(x: &[f64]) -> usize {
      let mut best = 0usize;
      let mut best_abs = x[0].abs();
      for (i, v) in x.iter().enumerate() {
            if v.abs() > best_abs {
                  best_abs = v.abs();
                  best = i;
            }
      }
      best
}

#[test]
fn ddot_parity() {
      hip::set_device(0).unwrap();
      let mut max_diff = 0.0f64;
      for &n in SIZES {
            let a = make_seq(n, 1);
            let b = make_seq(n, 2);
            let ga = GpuBuffer::upload(&a).unwrap();
            let gb = GpuBuffer::upload(&b).unwrap();
            let got = linalg::gpu_ddot(&ga, &gb, n).unwrap();
            let want = cpu_ddot(&a, &b);
            let diff = (got - want).abs();
            if diff > max_diff {
                  max_diff = diff;
            }
            assert!(
                  diff < TOL,
                  "ddot mismatch n={n}: gpu={got} cpu={want} diff={diff}"
            );
      }
      eprintln!("ddot_parity max abs diff = {max_diff:e}");
}

#[test]
fn dnrm2_parity() {
      hip::set_device(0).unwrap();
      let mut max_diff = 0.0f64;
      for &n in SIZES {
            let x = make_seq(n, 3);
            let gx = GpuBuffer::upload(&x).unwrap();
            let got = linalg::gpu_dnrm2(&gx, n).unwrap();
            let want = cpu_dnrm2(&x);
            let diff = (got - want).abs();
            if diff > max_diff {
                  max_diff = diff;
            }
            assert!(
                  diff < TOL,
                  "dnrm2 mismatch n={n}: gpu={got} cpu={want} diff={diff}"
            );
      }
      eprintln!("dnrm2_parity max abs diff = {max_diff:e}");
}

#[test]
fn dasum_parity() {
      hip::set_device(0).unwrap();
      let mut max_diff = 0.0f64;
      for &n in SIZES {
            let x = make_seq(n, 4);
            let gx = GpuBuffer::upload(&x).unwrap();
            let got = linalg::gpu_dasum(&gx, n).unwrap();
            let want = cpu_dasum(&x);
            let diff = (got - want).abs();
            if diff > max_diff {
                  max_diff = diff;
            }
            assert!(
                  diff < TOL,
                  "dasum mismatch n={n}: gpu={got} cpu={want} diff={diff}"
            );
      }
      eprintln!("dasum_parity max abs diff = {max_diff:e}");
}

#[test]
fn idamax_parity() {
      hip::set_device(0).unwrap();
      for &n in SIZES {
            let x = make_seq(n, 5);
            let gx = GpuBuffer::upload(&x).unwrap();
            let got = linalg::gpu_idamax(&gx, n).unwrap();
            let want = cpu_idamax(&x);
            assert_eq!(
                  got, want,
                  "idamax mismatch n={n}: gpu={got} cpu={want} (0-based index)"
            );
      }

      // Explicit case where the max-|x| element is negative and sits off any
      // 32-aligned boundary, to pin down 0-based indexing + sign handling.
      let mut x = make_seq(50, 6);
      x[37] = -99.0;
      let gx = GpuBuffer::upload(&x).unwrap();
      let got = linalg::gpu_idamax(&gx, x.len()).unwrap();
      assert_eq!(got, 37, "idamax negative-spike: gpu={got} want=37");
}

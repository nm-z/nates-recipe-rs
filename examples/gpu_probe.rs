// Proof-of-life: Rust drives the attached AMD GPU through gpu-core's hipblasSgemm.
// Small case verifies correctness vs a CPU reference; large case measures f32 TFLOP/s.
use gpu_core::memory::GpuBuffer;
use gpu_core::nn_f32::gpu_linear_f32;
use std::time::Instant;

fn cpu_linear(x: &[f32], w: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
      let mut out = vec![0.0f32; m * n];
      for i in 0..m {
            for j in 0..n {
                  let mut acc = 0.0f32;
                  for p in 0..k {
                        acc += x[i * k + p] * w[p * n + j];
                  }
                  out[i * n + j] = acc;
            }
      }
      out
}

fn gpu_linear(x: &[f32], w: &[f32], m: usize, n: usize, k: usize) -> Vec<f32> {
      let xb = GpuBuffer::upload_f32(x).expect("upload x");
      let wb = GpuBuffer::upload_f32(w).expect("upload w");
      let bb = GpuBuffer::zeros_f32(n).expect("zero bias");
      let out = gpu_linear_f32(&xb, &wb, &bb, m, n, k).expect("sgemm");
      out.download_vec_f32().expect("download")
}

fn main() {
      recipe_infer::init().expect("gpu init (set_device 0)");
      eprintln!("GPU device 0 selected.");

      // ── correctness: 2x3 = (2x4) @ (4x3) ───────────────────────────────
      let (m, n, k) = (2usize, 3usize, 4usize);
      let x: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.1 - 0.3).collect();
      let w: Vec<f32> = (0..k * n).map(|i| (i as f32) * 0.05 + 0.2).collect();
      let g = gpu_linear(&x, &w, m, n, k);
      let c = cpu_linear(&x, &w, m, n, k);
      let max_err = g
            .iter()
            .zip(&c)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
      eprintln!("correctness: gpu={g:?}");
      eprintln!("             cpu={c:?}");
      eprintln!("             max abs err = {max_err:.3e}");
      assert!(max_err < 1e-3, "GPU sgemm disagrees with CPU");

      // ── throughput: 4096^3 f32 GEMM ────────────────────────────────────
      let d = 4096usize;
      let xl: Vec<f32> = (0..d * d).map(|i| ((i % 17) as f32) * 0.01).collect();
      let wl: Vec<f32> = (0..d * d).map(|i| ((i % 13) as f32) * 0.01).collect();
      // warm up (allocs, hipblas handle, kernel load)
      let _ = gpu_linear(&xl, &wl, d, d, d);
      let t = Instant::now();
      let reps = 5;
      for _ in 0..reps {
            let _ = gpu_linear(&xl, &wl, d, d, d);
      }
      let secs = t.elapsed().as_secs_f64() / reps as f64;
      let flops = 2.0 * d as f64 * d as f64 * d as f64;
      eprintln!(
            "throughput: {d}^3 sgemm = {:.1} ms/call, {:.2} TFLOP/s f32",
            secs * 1e3,
            flops / secs / 1e12
      );
      eprintln!("PROOF: hand-written Rust ran {reps}+2 real GEMMs on the AMD card.");
      recipe_infer::shutdown();
}

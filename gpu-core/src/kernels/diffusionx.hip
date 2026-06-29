#include <hip/hip_runtime.h>

// ── Entropy-gated discrete diffusion sampler STEP (collision-safe diffusionx_) ─
//
// diffusionx_entropy_gated_step : one fused commit/keep decision per canvas slot.
//   logits  (n_positions, vocab) f64 row-major
//   canvas  (n_positions)        current token ids stored as f64
//   bound   f64 entropy threshold
// For each position p: softmax over logits[p,:], Shannon entropy
//   H_p = -sum_v sm_v * ln(sm_v)  (natural log; ln(0) guarded by skipping sm_v<=0).
//   H_p <  bound -> confident: accepted[p] = argmax_v logits[p,:], renoise[p] = 0
//   H_p >= bound -> uncertain: accepted[p] = canvas[p],            renoise[p] = 1

extern "C" __global__ void diffusionx_entropy_gated_step_kernel(
    const double* logits, const double* canvas, double* accepted, double* renoise,
    double bound, int n_positions, int vocab
) {
    int p = blockIdx.x * blockDim.x + threadIdx.x;
    if (p >= n_positions) return;
    const double* row = logits + (long)p * vocab;

    // pass 1: max logit (stable softmax) + argmax
    double m = row[0];
    int arg = 0;
    for (int v = 1; v < vocab; ++v) {
        double l = row[v];
        if (l > m) { m = l; arg = v; }
    }

    // pass 2: normalizer Z = sum exp(l - m)
    double z = 0.0;
    for (int v = 0; v < vocab; ++v) {
        z += exp(row[v] - m);
    }

    // pass 3: Shannon entropy of the softmax
    double h = 0.0;
    for (int v = 0; v < vocab; ++v) {
        double sm = exp(row[v] - m) / z;
        if (sm > 0.0) h -= sm * log(sm);
    }

    if (h < bound) {
        accepted[p] = (double)arg;
        renoise[p]  = 0.0;
    } else {
        accepted[p] = canvas[p];
        renoise[p]  = 1.0;
    }
}
extern "C" void launch_diffusionx_entropy_gated_step(
    const double* logits, const double* canvas, double* accepted, double* renoise,
    double bound, int n_positions, int vocab, hipStream_t s
) {
    dim3 block(256); dim3 grid((n_positions + 255) / 256);
    hipLaunchKernelGGL(diffusionx_entropy_gated_step_kernel, grid, block, 0, s,
                       logits, canvas, accepted, renoise, bound, n_positions, vocab);
}

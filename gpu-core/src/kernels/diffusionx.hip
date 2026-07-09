#include <hip/hip_runtime.h>


extern "C" __global__ void diffusionx_entropy_gated_step_kernel(
    const double* logits, const double* canvas, double* accepted, double* renoise,
    const double* bound_, int n_positions, int vocab
) {
    double bound = *bound_;
    int p = blockIdx.x * blockDim.x + threadIdx.x;
    if (p >= n_positions) return;
    const double* row = logits + (long)p * vocab;

    double m = row[0];
    int arg = 0;
    for (int v = 1; v < vocab; ++v) {
        double l = row[v];
        if (l > m) { m = l; arg = v; }
    }

    double z = 0.0;
    for (int v = 0; v < vocab; ++v) {
        z += exp(row[v] - m);
    }

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
    const double* bound, int n_positions, int vocab, hipStream_t s
) {
    dim3 block(256); dim3 grid((n_positions + 255) / 256);
    hipLaunchKernelGGL(diffusionx_entropy_gated_step_kernel, grid, block, 0, s,
                       logits, canvas, accepted, renoise, bound, n_positions, vocab);
}

extern "C" __global__ void diffusionx_commit_kernel(
    double* canvas, const double* accepted, const double* renoise, double* committed, int n
) {
    int p = blockIdx.x * blockDim.x + threadIdx.x;
    if (p >= n) return;
    if (committed[p] != 0.0) return;
    canvas[p] = accepted[p];
    if (renoise[p] == 0.0) committed[p] = 1.0;
}
extern "C" void launch_diffusionx_commit(
    double* canvas, const double* accepted, const double* renoise, double* committed, int n, hipStream_t s
) {
    dim3 block(256); dim3 grid((n + 255) / 256);
    hipLaunchKernelGGL(diffusionx_commit_kernel, grid, block, 0, s, canvas, accepted, renoise, committed, n);
}

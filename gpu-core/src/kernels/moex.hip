#include <hip/hip_runtime.h>

// ── MoE routing GPU kernel (collision-safe moex_ / launch_moex_) ─────────────
//
// moex_weighted_accumulate : out[i,j] += gate[i, e] * ye[i,j] for one expert e.
//   Dense combine step of mixture-of-experts: every token's expert-e output Ye
//   is scaled by that token's gate probability for expert e and summed into out.
//   Caller zeros `out` once, then calls this once per expert e in [0, n_experts).

// ── weighted per-token accumulate ───────────────────────────────────────────
// ye:(n,d) row-major expert-e output. gate:(n,n_experts) row-major gate probs.
// out:(n,d) row-major accumulator. e: expert column index into gate.
extern "C" __global__ void moex_weighted_accumulate_kernel(
    const double* ye, const double* gate, double* out,
    int n, int d, int n_experts, int e
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * d) return;
    int i = idx / d;
    double g = gate[(long)i * n_experts + e];
    out[idx] += g * ye[idx];
}
extern "C" void launch_moex_weighted_accumulate(
    const double* ye, const double* gate, double* out,
    int n, int d, int n_experts, int e, hipStream_t s
) {
    int total = n * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(moex_weighted_accumulate_kernel, grid, block, 0, s, ye, gate, out, n, d, n_experts, e);
}

// ── backward of the weighted accumulate (out[i,j] += gate[i,e]*ye[i,j]) ──────
// Given d_out, for expert e:
//   d_ye[i,j]   = gate[i,e] * d_out[i,j]           (n*d threads)
//   d_gate[i,e] = Σ_j ye[i,j] * d_out[i,j]         (n threads, reduce over d)
extern "C" __global__ void moex_dye_kernel(
    const double* d_out, const double* gate, double* d_ye,
    int n, int d, int n_experts, int e
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n * d) return;
    int i = idx / d;
    d_ye[idx] = gate[(long)i * n_experts + e] * d_out[idx];
}
extern "C" __global__ void moex_dgate_kernel(
    const double* d_out, const double* ye, double* d_gate,
    int n, int d, int n_experts, int e
) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    double acc = 0.0;
    long base = (long)i * d;
    for (int j = 0; j < d; j++) acc += ye[base + j] * d_out[base + j];
    d_gate[(long)i * n_experts + e] = acc;
}
extern "C" void launch_moex_weighted_accumulate_backward(
    const double* d_out, const double* gate, const double* ye,
    double* d_ye, double* d_gate, int n, int d, int n_experts, int e, hipStream_t s
) {
    int total = n * d;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(moex_dye_kernel, grid, block, 0, s, d_out, gate, d_ye, n, d, n_experts, e);
    dim3 grid_n((n + 255) / 256);
    hipLaunchKernelGGL(moex_dgate_kernel, grid_n, block, 0, s, d_out, ye, d_gate, n, d, n_experts, e);
}

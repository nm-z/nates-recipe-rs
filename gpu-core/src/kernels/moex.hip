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

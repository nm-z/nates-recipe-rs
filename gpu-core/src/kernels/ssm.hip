#include <hip/hip_runtime.h>

#define SSM_MAX_STATE 256

// Causal depthwise conv1d + per-channel bias + SiLU, token-major in/out.
// x and out are [T, d_inner] token-major (feature-contiguous per token); the
// conv is per-channel over the time axis with an implicit (d_conv-1) left
// zero-pad (the j>=0 guard), matching ggml_ssm_conv on a zeroed state prefix.
// conv_w is [d_inner, d_conv] channel-major (tap-contiguous): w[c*d_conv + k].
extern "C" __global__ void ssm_conv_causal_silu_kernel(
    const double* x, const double* conv_w, const double* bias, double* out,
    int T, int d_inner, int d_conv
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)T * d_inner;
    if (idx >= total) return;
    int c = idx % d_inner;
    int t = idx / d_inner;
    const double* wc = conv_w + (long)c * d_conv;
    double acc = bias ? bias[c] : 0.0;
    for (int k = 0; k < d_conv; ++k) {
        int j = t - (d_conv - 1) + k;
        if (j >= 0) acc += wc[k] * x[(long)j * d_inner + c];
    }
    out[idx] = acc / (1.0 + exp(-acc));
}

extern "C" void launch_ssm_conv_causal_silu(
    const double* x, const double* conv_w, const double* bias, double* out,
    int T, int d_inner, int d_conv, hipStream_t stream
) {
    long total = (long)T * d_inner;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(ssm_conv_causal_silu_kernel, grid, block, 0, stream,
        x, conv_w, bias, out, T, d_inner, d_conv);
}

// Fused Mamba-1 selective scan (ground truth ggml-cuda/ssm-scan.cu:83-113).
// One thread per inner channel c holds an N=d_state private state vector across
// the whole t loop, folding softplus(dt), exp(dt*A), the B/x accumulate, the C
// contraction and the D skip in one pass. f64, no intra-loop allocation.
//   xc  [T, d_inner]  conv+silu output (token-major)
//   dt  [T, d_inner]  time-step after ssm_dt projection + bias (token-major)
//   A   [d_inner, d_state]  channel-major, state-contiguous (used directly)
//   B   [T, d_state]  single-group selective input (token-major)
//   C   [T, d_state]  single-group readout   (token-major)
//   D   [d_inner]     per-channel skip
//   y   [T, d_inner]  output (token-major), y[t,c] = sum_n h*C + D[c]*xc[t,c]
extern "C" __global__ void ssm_scan_mamba1_kernel(
    const double* xc, const double* dt, const double* A,
    const double* B, const double* C, const double* D,
    double* y, int T, int d_inner, int d_state
) {
    int c = blockIdx.x * blockDim.x + threadIdx.x;
    if (c >= d_inner) return;
    double state[SSM_MAX_STATE];
    for (int n = 0; n < d_state; ++n) state[n] = 0.0;
    const double* Ac = A + (long)c * d_state;
    double Dc = D[c];
    for (int t = 0; t < T; ++t) {
        double dtc = dt[(long)t * d_inner + c];
        double dt_sp = (dtc <= 20.0) ? log1p(exp(dtc)) : dtc;
        double xct = xc[(long)t * d_inner + c];
        double x_dt = xct * dt_sp;
        const double* Bt = B + (long)t * d_state;
        const double* Ct = C + (long)t * d_state;
        double yacc = 0.0;
        for (int n = 0; n < d_state; ++n) {
            double a = exp(dt_sp * Ac[n]);
            state[n] = state[n] * a + Bt[n] * x_dt;
            yacc += state[n] * Ct[n];
        }
        y[(long)t * d_inner + c] = yacc + Dc * xct;
    }
}

extern "C" void launch_ssm_scan_mamba1(
    const double* xc, const double* dt, const double* A,
    const double* B, const double* C, const double* D,
    double* y, int T, int d_inner, int d_state, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((d_inner + 255) / 256);
    hipLaunchKernelGGL(ssm_scan_mamba1_kernel, grid, block, 0, stream,
        xc, dt, A, B, C, D, y, T, d_inner, d_state);
}

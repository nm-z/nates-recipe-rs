#include <hip/hip_runtime.h>

#define SSM_MAX_STATE 256

// Causal depthwise conv1d + per-channel bias + SiLU, token-major in/out.
// x and out are [T, d_inner] token-major (feature-contiguous per token); the
// conv is per-channel over the time axis with an implicit (d_conv-1) left
// zero-pad (the j>=0 guard), matching ggml_ssm_conv on a zeroed state prefix.
// conv_w is [d_inner, d_conv] channel-major (tap-contiguous): w[c*d_conv + k].
// apply_silu==0 leaves the raw conv output (lfm2 shortconv gates it downstream);
// nonzero fuses SiLU (mamba family). Same channel-major tap-contiguous weight and
// (d_conv-1) causal left zero-pad in both cases.
extern "C" __global__ void ssm_conv_causal_silu_kernel(
    const double* x, const double* conv_w, const double* bias, double* out,
    int T, int d_inner, int d_conv, int apply_silu
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
    out[idx] = apply_silu ? (acc / (1.0 + exp(-acc))) : acc;
}

extern "C" void launch_ssm_conv_causal_silu(
    const double* x, const double* conv_w, const double* bias, double* out,
    int T, int d_inner, int d_conv, int apply_silu, hipStream_t stream
) {
    long total = (long)T * d_inner;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(ssm_conv_causal_silu_kernel, grid, block, 0, stream,
        x, conv_w, bias, out, T, d_inner, d_conv, apply_silu);
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

// Fused Mamba-2 grouped selective scan (ggml ssm-scan.cu mamba2 branch,
// A->ne[0]==1 per-head decay, ggml.c:5616). One thread per inner channel c holds
// an N=d_state private state vector across the whole t loop. Differences from
// mamba-1: A and D are per-HEAD scalars (ssm_a/ssm_d [1,n_head], broadcast over
// head_dim and state); dt is per-head, shared across the head's head_dim
// channels; B/C are per-GROUP (group g = h / (n_head/n_group)) and read directly
// out of the conv output xbc by column offset, so no B/C slice buffers exist.
//   xbc  [T, conv_dim]  conv+silu output: x=cols[0,d_inner),
//        B=cols[d_inner + g*d_state, +d_state), C=cols[d_inner + n_group*d_state
//        + g*d_state, +d_state)
//   dt   [T, n_head]    post-bias time-step (token-major, per-head)
//   A    [n_head]       per-head scalar decay, used directly (already -exp(A_log))
//   D    [n_head]       per-head scalar skip
//   y    [T, d_inner]   output, y[t,c] = sum_n h*C + D[h]*x[t,c]
extern "C" __global__ void ssm_scan_mamba2_kernel(
    const double* xbc, const double* dt, const double* A, const double* D,
    double* y, int T, int d_inner, int d_state, int n_head, int n_group, int conv_dim
) {
    int c = blockIdx.x * blockDim.x + threadIdx.x;
    if (c >= d_inner) return;
    int head_dim = d_inner / n_head;
    int h = c / head_dim;
    int g = h / (n_head / n_group);
    double state[SSM_MAX_STATE];
    for (int n = 0; n < d_state; ++n) state[n] = 0.0;
    double Ah = A[h];
    double Dh = D[h];
    long b_off = (long)d_inner + (long)g * d_state;
    long c_off = (long)d_inner + (long)n_group * d_state + (long)g * d_state;
    for (int t = 0; t < T; ++t) {
        double dth = dt[(long)t * n_head + h];
        double dt_sp = (dth <= 20.0) ? log1p(exp(dth)) : dth;
        double xct = xbc[(long)t * conv_dim + c];
        double x_dt = xct * dt_sp;
        double a = exp(dt_sp * Ah);
        const double* Bt = xbc + (long)t * conv_dim + b_off;
        const double* Ct = xbc + (long)t * conv_dim + c_off;
        double yacc = 0.0;
        for (int n = 0; n < d_state; ++n) {
            state[n] = state[n] * a + Bt[n] * x_dt;
            yacc += state[n] * Ct[n];
        }
        y[(long)t * d_inner + c] = yacc + Dh * xct;
    }
}

extern "C" void launch_ssm_scan_mamba2(
    const double* xbc, const double* dt, const double* A, const double* D,
    double* y, int T, int d_inner, int d_state, int n_head, int n_group,
    int conv_dim, hipStream_t stream
) {
    dim3 block(256);
    dim3 grid((d_inner + 255) / 256);
    hipLaunchKernelGGL(ssm_scan_mamba2_kernel, grid, block, 0, stream,
        xbc, dt, A, D, y, T, d_inner, d_state, n_head, n_group, conv_dim);
}

__device__ __forceinline__ double ssm_block_sum_bcast(double val, double* smem) {
    int lane = threadIdx.x % warpSize;
    int wid  = threadIdx.x / warpSize;
    for (int off = warpSize / 2; off > 0; off /= 2) val += __shfl_down(val, off);
    if (lane == 0) smem[wid] = val;
    __syncthreads();
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        double v = (lane < nwarps) ? smem[lane] : 0.0;
        for (int off = warpSize / 2; off > 0; off /= 2) v += __shfl_down(v, off);
        if (lane == 0) smem[0] = v;
    }
    __syncthreads();
    double total = smem[0];
    __syncthreads();
    return total;
}

// Grouped RMSNorm with per-group gamma (Mamba-2 gated-norm, mamba-base.cpp:272).
// The already-gated y buffer is [T, d_inner] token-major; each token's d_inner
// channels split into n_group contiguous blocks of dpg=d_inner/n_group. Each
// (token,group) row RMS-normalizes over its dpg channels then scales by
// gamma[g*dpg + j] (ssm_norm is [dpg, n_group], group g contiguous at g*dpg).
// Flattened as rows = T*n_group so a row's group is row % n_group. In-place safe.
extern "C" __global__ void ssm_group_rmsnorm_kernel(
    const double* x, const double* gamma, const double* eps_p, double* out,
    int rows, int dpg, int n_group
) {
    double eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    int g = row % n_group;
    const double* r = x + (long)row * dpg;
    double* o = out + (long)row * dpg;
    const double* gg = gamma + (long)g * dpg;
    __shared__ double smem[32];
    double ss = 0.0;
    for (int j = threadIdx.x; j < dpg; j += blockDim.x) { double v = r[j]; ss += v * v; }
    double ms = ssm_block_sum_bcast(ss, smem) / dpg;
    double inv = 1.0 / sqrt(ms + eps);
    for (int j = threadIdx.x; j < dpg; j += blockDim.x)
        o[j] = r[j] * inv * gg[j];
}

extern "C" void launch_ssm_group_rmsnorm(
    const double* x, const double* gamma, const double* eps, double* out,
    int rows, int dpg, int n_group, hipStream_t stream
) {
    int threads = min(256, max(32, dpg));
    hipLaunchKernelGGL(ssm_group_rmsnorm_kernel, dim3(rows), dim3(threads), 0, stream,
        x, gamma, eps, out, rows, dpg, n_group);
}

// Per-row L2 normalize (ggml_l2_norm, ops.cpp:4174): out[row,j] = x[row,j] /
// max(sqrt(sum_j x[row,j]^2), eps). Distinct from RMSNorm (no 1/cols factor, no
// gamma; the divisor floors at eps, it is not added under the sqrt). rows = T*H,
// cols = head_dim; one block per row. In-place safe.
extern "C" __global__ void l2norm_rows_kernel(
    const double* x, const double* eps_p, double* out, int rows, int cols
) {
    double eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    const double* r = x + (long)row * cols;
    double* o = out + (long)row * cols;
    __shared__ double smem[32];
    double ss = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { double v = r[j]; ss += v * v; }
    double tot = ssm_block_sum_bcast(ss, smem);
    double inv = 1.0 / fmax(sqrt(tot), eps);
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        o[j] = r[j] * inv;
}

extern "C" void launch_l2norm_rows(
    const double* x, const double* eps, double* out, int rows, int cols, hipStream_t stream
) {
    int threads = min(256, max(32, cols));
    hipLaunchKernelGGL(l2norm_rows_kernel, dim3(rows), dim3(threads), 0, stream,
        x, eps, out, rows, cols);
}

#define GDS_MAX_D 256

// Fused gated delta-rule scan (delta-net-base.cpp build_delta_net_autoregressive
// :289-371, applied sequentially over the T tokens of a full-batch prompt; state
// starts at zero, no cross-call cache). Ground-truth per-token recurrence, per
// head h, state S[i,j] (i = key/query axis, j = value/output axis):
//   (1) decay  S[i,j] *= exp(g * a[h])   (g is a per-head scalar for GDA, a
//              per-j vector for KDA; either way one factor per column j)
//   (2) sk[j]  = sum_i S[i,j] * k[i]
//   (3) d[j]   = beta[h] * (v[j] - sk[j])
//   (4) update S[i,j] += k[i] * d[j]
//   (5) read   o[j]  = sum_i S[i,j] * q[i]   (q pre-scaled by `scale` = 1/sqrt(d))
// Geometry: grid = Hv workgroups (one head), blockDim = d threads. Thread j owns
// state COLUMN S[:,j] as a private length-d register array, resident across the
// whole T loop (zero global state traffic). k and q are staged to LDS each step
// (every thread reads all i). v[j]/g/beta are per-thread direct reads. f64, no
// intra-loop allocation. q/k/v are [T, Hv*d] token-major head-major; g is [T,Hv]
// (GDA) or [T,Hv*d] (KDA); beta is [T,Hv]; a is [Hv]; out is [T, Hv*d].
extern "C" __global__ void gated_delta_scan_kernel(
    const double* q, const double* k, const double* v,
    const double* g, const double* beta, const double* a,
    double* out, int T, int Hv, int d, int per_channel, double scale
) {
    int h = blockIdx.x;
    int j = threadIdx.x;
    if (j >= d) return;
    __shared__ double kl[GDS_MAX_D];
    __shared__ double ql[GDS_MAX_D];
    double state[GDS_MAX_D];
    for (int i = 0; i < d; ++i) state[i] = 0.0;
    double ah = a[h];
    long stride = (long)Hv * d;
    long hd = (long)h * d;
    for (int t = 0; t < T; ++t) {
        long base = (long)t * stride + hd;
        kl[j] = k[base + j];
        ql[j] = q[base + j] * scale;
        __syncthreads();
        double gval = per_channel ? g[base + j] : g[(long)t * Hv + h];
        double dec = exp(gval * ah);
        double sk = 0.0;
        for (int i = 0; i < d; ++i) { state[i] *= dec; sk += state[i] * kl[i]; }
        double dj = beta[(long)t * Hv + h] * (v[base + j] - sk);
        double o = 0.0;
        for (int i = 0; i < d; ++i) { state[i] += kl[i] * dj; o += state[i] * ql[i]; }
        out[base + j] = o;
        __syncthreads();
    }
}

extern "C" void launch_gated_delta_scan(
    const double* q, const double* k, const double* v,
    const double* g, const double* beta, const double* a,
    double* out, int T, int Hv, int d, int per_channel, double scale, hipStream_t stream
) {
    hipLaunchKernelGGL(gated_delta_scan_kernel, dim3(Hv), dim3(d), 0, stream,
        q, k, v, g, beta, a, out, T, Hv, d, per_channel, scale);
}

// Per-row scalar multiply: out[r*cols + c] = x[r*cols + c] * s[r]. s is one scalar
// per row (rows entries). The qwen3.5/next shared-expert token gate multiplies the
// [T, n_embd] expert output by a per-token sigmoid scalar. In-place safe.
extern "C" __global__ void row_scale_kernel(
    const double* x, const double* s, double* out, int rows, int cols
) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * cols;
    if (i >= total) return;
    out[i] = x[i] * s[i / cols];
}

extern "C" void launch_row_scale(
    const double* x, const double* s, double* out, int rows, int cols, hipStream_t stream
) {
    long total = (long)rows * cols;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(row_scale_kernel, grid, block, 0, stream, x, s, out, rows, cols);
}

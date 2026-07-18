#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"

#define SSM_MAX_STATE 256

// Causal depthwise conv1d + per-channel bias + SiLU, token-major in/out.
// x and out are [T, d_inner] token-major (feature-contiguous per token); the
// conv is per-channel over the time axis with an implicit (d_conv-1) left
// zero-pad (the j>=0 guard), matching ggml_ssm_conv on a zeroed state prefix.
// conv_w is [d_inner, d_conv] channel-major (tap-contiguous): w[c*d_conv + k].
// apply_silu==0 leaves the raw conv output (lfm2 shortconv gates it downstream);
// nonzero fuses SiLU (mamba family). Same channel-major tap-contiguous weight and
// (d_conv-1) causal left zero-pad in both cases.
// state_in (nullable, [(d_conv-1), d_inner] token-major): the previous d_conv-1
// input rows. When present, a negative tap (combined index m<0 into the virtual
// sequence [state_in | x]) reads state_in[(d_conv-1+m)*d_inner + c] instead of
// contributing zero, so an incremental conv over only the NEW rows matches a
// full-sequence conv on the zero-padded prefix. Null keeps the (d_conv-1) left
// zero-pad.
template<typename DT> __global__ void ssm_conv_causal_silu_kernel(
    const DT* x, const DT* conv_w, const DT* bias, DT* out,
    int T, int d_inner, int d_conv, int apply_silu, const DT* state_in
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)T * d_inner;
    if (idx >= total) return;
    int c = idx % d_inner;
    int t = idx / d_inner;
    const DT* wc = conv_w + (long)c * d_conv;
    DT acc = bias ? bias[c] : 0.0;
    for (int k = 0; k < d_conv; ++k) {
        int m = t - (d_conv - 1) + k;
        if (m >= 0) acc += wc[k] * x[(long)m * d_inner + c];
        else if (state_in) acc += wc[k] * state_in[(long)(d_conv - 1 + m) * d_inner + c];
    }
    out[idx] = apply_silu ? (acc / (1.0 + exp(-acc))) : acc;
}

// Writes the trailing d_conv-1 input rows of the virtual sequence [state_in | x]
// into state_out (distinct buffer, no in-place hazard): row r takes combined
// index T+r, sourced from x when T+r>=d_conv-1 else from state_in.
template<typename DT> __global__ void ssm_conv_state_save_kernel(
    const DT* state_in, const DT* x, DT* state_out, int T, int d_inner, int d_conv
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)(d_conv - 1) * d_inner;
    if (idx >= total) return;
    int c = idx % d_inner;
    int r = idx / d_inner;
    int m = T + r;
    if (m >= d_conv - 1) state_out[idx] = x[(long)(m - (d_conv - 1)) * d_inner + c];
    else state_out[idx] = state_in ? state_in[(long)m * d_inner + c] : (DT)0.0;
}

extern "C" void launch_ssm_conv_causal_silu(
    const double* x, const double* conv_w, const double* bias, double* out,
    int T, int d_inner, int d_conv, int apply_silu,
    const double* state_in, double* state_out, hipStream_t stream, int dtype
) {
    long total = (long)T * d_inner;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    int Tn = T;
    DT_DISPATCH(dtype, ssm_conv_causal_silu_kernel<T><<<grid, block, 0, stream>>>(
        (const T*)x, (const T*)conv_w, (const T*)bias, (T*)out, Tn, d_inner, d_conv, apply_silu, (const T*)state_in));
    if (state_out) {
        long stotal = (long)(d_conv - 1) * d_inner;
        dim3 sgrid((stotal + 255) / 256);
        DT_DISPATCH(dtype, ssm_conv_state_save_kernel<T><<<sgrid, block, 0, stream>>>(
            (const T*)state_in, (const T*)x, (T*)state_out, Tn, d_inner, d_conv));
    }
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
// state_g (nullable, [d_inner, d_state] channel-major, thread c owns row
// state_g[c*d_state + n]): when non-null the carried recurrent state is loaded at
// entry and written back at exit, so a call over only the NEW rows produces output
// bit-identical to a full-sequence call. Null keeps the zero-init, no-write-back
// behavior (fresh prefill).
template<typename DT> __global__ void ssm_scan_mamba1_kernel(
    const DT* xc, const DT* dt, const DT* A,
    const DT* B, const DT* C, const DT* D,
    DT* y, int T, int d_inner, int d_state, DT* state_g
) {
    int c = blockIdx.x * blockDim.x + threadIdx.x;
    if (c >= d_inner) return;
    DT state[SSM_MAX_STATE];
    for (int n = 0; n < d_state; ++n) state[n] = state_g ? state_g[(long)c * d_state + n] : 0.0;
    const DT* Ac = A + (long)c * d_state;
    DT Dc = D[c];
    for (int t = 0; t < T; ++t) {
        DT dtc = dt[(long)t * d_inner + c];
        DT dt_sp = (dtc <= 20.0) ? log1p(exp(dtc)) : dtc;
        DT xct = xc[(long)t * d_inner + c];
        DT x_dt = xct * dt_sp;
        const DT* Bt = B + (long)t * d_state;
        const DT* Ct = C + (long)t * d_state;
        DT yacc = 0.0;
        for (int n = 0; n < d_state; ++n) {
            DT a = exp(dt_sp * Ac[n]);
            state[n] = state[n] * a + Bt[n] * x_dt;
            yacc += state[n] * Ct[n];
        }
        y[(long)t * d_inner + c] = yacc + Dc * xct;
    }
    if (state_g) for (int n = 0; n < d_state; ++n) state_g[(long)c * d_state + n] = state[n];
}

extern "C" void launch_ssm_scan_mamba1(
    const double* xc, const double* dt, const double* A,
    const double* B, const double* C, const double* D,
    double* y, int T, int d_inner, int d_state, double* state, hipStream_t stream, int dtype
) {
    dim3 block(256);
    dim3 grid((d_inner + 255) / 256);
    int Tn = T;
    DT_DISPATCH(dtype, ssm_scan_mamba1_kernel<T><<<grid, block, 0, stream>>>(
        (const T*)xc, (const T*)dt, (const T*)A, (const T*)B, (const T*)C, (const T*)D,
        (T*)y, Tn, d_inner, d_state, (T*)state));
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
// state_g (nullable, [d_inner, d_state] channel-major, thread c owns row
// state_g[c*d_state + n]): same carry contract as ssm_scan_mamba1_kernel.
template<typename DT> __global__ void ssm_scan_mamba2_kernel(
    const DT* xbc, const DT* dt, const DT* A, const DT* D,
    DT* y, int T, int d_inner, int d_state, int n_head, int n_group, int conv_dim, DT* state_g
) {
    int c = blockIdx.x * blockDim.x + threadIdx.x;
    if (c >= d_inner) return;
    int head_dim = d_inner / n_head;
    int h = c / head_dim;
    int g = h / (n_head / n_group);
    DT state[SSM_MAX_STATE];
    for (int n = 0; n < d_state; ++n) state[n] = state_g ? state_g[(long)c * d_state + n] : 0.0;
    DT Ah = A[h];
    DT Dh = D[h];
    long b_off = (long)d_inner + (long)g * d_state;
    long c_off = (long)d_inner + (long)n_group * d_state + (long)g * d_state;
    for (int t = 0; t < T; ++t) {
        DT dth = dt[(long)t * n_head + h];
        DT dt_sp = (dth <= 20.0) ? log1p(exp(dth)) : dth;
        DT xct = xbc[(long)t * conv_dim + c];
        DT x_dt = xct * dt_sp;
        DT a = exp(dt_sp * Ah);
        const DT* Bt = xbc + (long)t * conv_dim + b_off;
        const DT* Ct = xbc + (long)t * conv_dim + c_off;
        DT yacc = 0.0;
        for (int n = 0; n < d_state; ++n) {
            state[n] = state[n] * a + Bt[n] * x_dt;
            yacc += state[n] * Ct[n];
        }
        y[(long)t * d_inner + c] = yacc + Dh * xct;
    }
    if (state_g) for (int n = 0; n < d_state; ++n) state_g[(long)c * d_state + n] = state[n];
}

extern "C" void launch_ssm_scan_mamba2(
    const double* xbc, const double* dt, const double* A, const double* D,
    double* y, int T, int d_inner, int d_state, int n_head, int n_group,
    int conv_dim, double* state, hipStream_t stream, int dtype
) {
    dim3 block(256);
    dim3 grid((d_inner + 255) / 256);
    int Tn = T;
    DT_DISPATCH(dtype, ssm_scan_mamba2_kernel<T><<<grid, block, 0, stream>>>(
        (const T*)xbc, (const T*)dt, (const T*)A, (const T*)D,
        (T*)y, Tn, d_inner, d_state, n_head, n_group, conv_dim, (T*)state));
}

template<typename DT> __device__ __forceinline__ DT ssm_block_sum_bcast(DT val, DT* smem) {
    int lane = threadIdx.x % warpSize;
    int wid  = threadIdx.x / warpSize;
    for (int off = warpSize / 2; off > 0; off /= 2) val += __shfl_down(val, off);
    if (lane == 0) smem[wid] = val;
    __syncthreads();
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        DT v = (lane < nwarps) ? smem[lane] : 0.0;
        for (int off = warpSize / 2; off > 0; off /= 2) v += __shfl_down(v, off);
        if (lane == 0) smem[0] = v;
    }
    __syncthreads();
    DT total = smem[0];
    __syncthreads();
    return total;
}

// Grouped RMSNorm with per-group gamma (Mamba-2 gated-norm, mamba-base.cpp:272).
// The already-gated y buffer is [T, d_inner] token-major; each token's d_inner
// channels split into n_group contiguous blocks of dpg=d_inner/n_group. Each
// (token,group) row RMS-normalizes over its dpg channels then scales by
// gamma[g*dpg + j] (ssm_norm is [dpg, n_group], group g contiguous at g*dpg).
// Flattened as rows = T*n_group so a row's group is row % n_group. In-place safe.
template<typename DT> __global__ void ssm_group_rmsnorm_kernel(
    const DT* x, const DT* gamma, const DT* eps_p, DT* out,
    int rows, int dpg, int n_group
) {
    DT eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    int g = row % n_group;
    const DT* r = x + (long)row * dpg;
    DT* o = out + (long)row * dpg;
    const DT* gg = gamma + (long)g * dpg;
    __shared__ DT smem[32];
    DT ss = 0.0;
    for (int j = threadIdx.x; j < dpg; j += blockDim.x) { DT v = r[j]; ss += v * v; }
    DT ms = ssm_block_sum_bcast(ss, smem) / dpg;
    DT inv = 1.0 / sqrt(ms + eps);
    for (int j = threadIdx.x; j < dpg; j += blockDim.x)
        o[j] = r[j] * inv * gg[j];
}

extern "C" void launch_ssm_group_rmsnorm(
    const double* x, const double* gamma, const double* eps, double* out,
    int rows, int dpg, int n_group, hipStream_t stream, int dtype
) {
    int threads = min(256, max(32, dpg));
    DT_DISPATCH(dtype, ssm_group_rmsnorm_kernel<T><<<dim3(rows), dim3(threads), 0, stream>>>(
        (const T*)x, (const T*)gamma, (const T*)eps, (T*)out, rows, dpg, n_group));
}

// Per-row L2 normalize (ggml_l2_norm, ops.cpp:4174): out[row,j] = x[row,j] /
// max(sqrt(sum_j x[row,j]^2), eps). Distinct from RMSNorm (no 1/cols factor, no
// gamma; the divisor floors at eps, it is not added under the sqrt). rows = T*H,
// cols = head_dim; one block per row. In-place safe.
template<typename DT> __global__ void l2norm_rows_kernel(
    const DT* x, const DT* eps_p, DT* out, int rows, int cols
) {
    DT eps = *eps_p;
    int row = blockIdx.x;
    if (row >= rows) return;
    const DT* r = x + (long)row * cols;
    DT* o = out + (long)row * cols;
    __shared__ DT smem[32];
    DT ss = 0.0;
    for (int j = threadIdx.x; j < cols; j += blockDim.x) { DT v = r[j]; ss += v * v; }
    DT tot = ssm_block_sum_bcast(ss, smem);
    DT inv = 1.0 / fmax(sqrt(tot), eps);
    for (int j = threadIdx.x; j < cols; j += blockDim.x)
        o[j] = r[j] * inv;
}

extern "C" void launch_l2norm_rows(
    const double* x, const double* eps, double* out, int rows, int cols, hipStream_t stream, int dtype
) {
    int threads = min(256, max(32, cols));
    DT_DISPATCH(dtype, l2norm_rows_kernel<T><<<dim3(rows), dim3(threads), 0, stream>>>(
        (const T*)x, (const T*)eps, (T*)out, rows, cols));
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
// state_g (nullable, [Hv, d, d] with layout state_g[h*d*d + i*d + j], thread
// (block h, thread j) owns column S[:,j]): same carry contract — loaded at entry,
// written back at exit — so an incremental call over only the NEW rows is
// bit-identical to a full-sequence call. Null keeps the zero-init behavior.
template<typename DT> __global__ void gated_delta_scan_kernel(
    const DT* q, const DT* k, const DT* v,
    const DT* g, const DT* beta, const DT* a,
    DT* out, int T, int Hv, int d, int per_channel, DT scale, DT* state_g
) {
    int h = blockIdx.x;
    int j = threadIdx.x;
    if (j >= d) return;
    __shared__ DT kl[GDS_MAX_D];
    __shared__ DT ql[GDS_MAX_D];
    DT state[GDS_MAX_D];
    for (int i = 0; i < d; ++i) state[i] = state_g ? state_g[(long)h * d * d + (long)i * d + j] : 0.0;
    DT ah = a[h];
    long stride = (long)Hv * d;
    long hd = (long)h * d;
    for (int t = 0; t < T; ++t) {
        long base = (long)t * stride + hd;
        kl[j] = k[base + j];
        ql[j] = q[base + j] * scale;
        __syncthreads();
        DT gval = per_channel ? g[base + j] : g[(long)t * Hv + h];
        DT dec = exp(gval * ah);
        DT sk = 0.0;
        for (int i = 0; i < d; ++i) { state[i] *= dec; sk += state[i] * kl[i]; }
        DT dj = beta[(long)t * Hv + h] * (v[base + j] - sk);
        DT o = 0.0;
        for (int i = 0; i < d; ++i) { state[i] += kl[i] * dj; o += state[i] * ql[i]; }
        out[base + j] = o;
        __syncthreads();
    }
    if (state_g) for (int i = 0; i < d; ++i) state_g[(long)h * d * d + (long)i * d + j] = state[i];
}

extern "C" void launch_gated_delta_scan(
    const double* q, const double* k, const double* v,
    const double* g, const double* beta, const double* a,
    double* out, int T, int Hv, int d, int per_channel, double scale, double* state, hipStream_t stream, int dtype
) {
    int Tn = T;
    DT_DISPATCH(dtype, gated_delta_scan_kernel<T><<<dim3(Hv), dim3(d), 0, stream>>>(
        (const T*)q, (const T*)k, (const T*)v, (const T*)g, (const T*)beta, (const T*)a,
        (T*)out, Tn, Hv, d, per_channel, (T)scale, (T*)state));
}

// Per-row scalar multiply: out[r*cols + c] = x[r*cols + c] * s[r]. s is one scalar
// per row (rows entries). The qwen3.5/next shared-expert token gate multiplies the
// [T, n_embd] expert output by a per-token sigmoid scalar. In-place safe.
template<typename DT> __global__ void row_scale_kernel(
    const DT* x, const DT* s, DT* out, int rows, int cols
) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * cols;
    if (i >= total) return;
    out[i] = x[i] * s[i / cols];
}

extern "C" void launch_row_scale(
    const double* x, const double* s, double* out, int rows, int cols, hipStream_t stream, int dtype
) {
    long total = (long)rows * cols;
    dim3 block(256);
    dim3 grid((total + 255) / 256);
    DT_DISPATCH(dtype, row_scale_kernel<T><<<grid, block, 0, stream>>>(
        (const T*)x, (const T*)s, (T*)out, rows, cols));
}

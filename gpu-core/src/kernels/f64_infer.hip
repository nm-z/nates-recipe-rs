#include <hip/hip_runtime.h>

// ── f64 streaming-inference kernels ──────────────────────────────────────────
// General, model-agnostic ops for running a bf16-weight transformer entirely in
// f64: widen bf16 bytes to f64 in VRAM, a GQA attention with a mixed
// causal/bidirectional prefix mask (kq_scale = 1.0), and two gated-GELU fusions
// (tanh-approx GELU, matching gelu_pytorch_tanh). Weights arrive as raw bf16
// bytes and are widened just before each GEMM into a reusable VRAM window.

// bf16 (upper 16 bits of an f32) → f64. `in` is the raw little-endian bf16 half
// reinterpreted as u16; the pad to f32 is exact (lossless), then f32 → f64.
extern "C" __global__ void widen_bf16_f64(const unsigned short* in, double* out, long n) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        unsigned int bits = ((unsigned int)in[i]) << 16;
        out[i] = (double)__uint_as_float(bits);
    }
}
extern "C" void launch_widen_bf16_f64(const unsigned short* in, double* out, long n, hipStream_t s) {
    int t = 256;
    long blocks = (n + t - 1) / t;
    hipLaunchKernelGGL(widen_bf16_f64, dim3((unsigned int)blocks), dim3(t), 0, s, in, out, n);
}

// Block sum-reduce (warp shuffle + shared), broadcast to all threads via smem[0].
__device__ __forceinline__ double fi_block_sum(double v, double* smem) {
    int lane = threadIdx.x % warpSize;
    int wid  = threadIdx.x / warpSize;
    for (int o = warpSize / 2; o > 0; o /= 2) v += __shfl_down(v, o);
    if (lane == 0) smem[wid] = v;
    __syncthreads();
    int nw = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        double t = (lane < nw) ? smem[lane] : 0.0;
        for (int o = warpSize / 2; o > 0; o /= 2) t += __shfl_down(t, o);
        if (lane == 0) smem[0] = t;
    }
    __syncthreads();
    double tot = smem[0];
    __syncthreads();
    return tot;
}

// GQA self-attention, one block per (query position p, query head h). q is
// (t, nqh*hd), k/v are (t, nkv*hd), out is (t, nqh*hd), all row-major f64. Each
// q-head maps to kv-head h/(nqh/nkv). kq_scale = 1.0 (NOT 1/sqrt(hd)). Mask:
// prompt positions (p < prefix) are causal (attend sp <= p); canvas positions
// (p >= prefix) are fully bidirectional. Dynamic shared holds the t scores.
extern "C" __global__ void gqa_masked_attn(
    const double* q, const double* k, const double* v, double* out,
    int t, int nqh, int nkv, int hd, int prefix
) {
    int blk = blockIdx.x;
    int p = blk / nqh;
    int h = blk % nqh;
    int g2 = nqh / nkv;
    int kv = h / g2;
    int qd = nqh * hd;
    int kd = nkv * hd;
    const double* qrow = q + (long)p * qd + (long)h * hd;

    __shared__ double red[64];
    extern __shared__ double sc[];   // t doubles

    for (int sp = 0; sp < t; sp++) {
        const double* krow = k + (long)sp * kd + (long)kv * hd;
        double part = 0.0;
        for (int x = threadIdx.x; x < hd; x += blockDim.x) part += qrow[x] * krow[x];
        double dot = fi_block_sum(part, red);
        if (threadIdx.x == 0) {
            bool masked = (p < prefix) && (sp > p);
            sc[sp] = masked ? -1e30 : dot;
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) {
        double m = -1e300;
        for (int sp = 0; sp < t; sp++) if (sc[sp] > m) m = sc[sp];
        double s = 0.0;
        for (int sp = 0; sp < t; sp++) { double e = exp(sc[sp] - m); sc[sp] = e; s += e; }
        for (int sp = 0; sp < t; sp++) sc[sp] /= s;
    }
    __syncthreads();

    double* orow = out + (long)p * qd + (long)h * hd;
    for (int x = threadIdx.x; x < hd; x += blockDim.x) {
        double acc = 0.0;
        for (int sp = 0; sp < t; sp++) acc += sc[sp] * v[(long)sp * kd + (long)kv * hd + x];
        orow[x] = acc;
    }
}
extern "C" void launch_gqa_masked_attn(
    const double* q, const double* k, const double* v, double* out,
    int t, int nqh, int nkv, int hd, int prefix, hipStream_t s
) {
    int threads = hd < 256 ? hd : 256;
    size_t shmem = (size_t)t * sizeof(double);
    hipLaunchKernelGGL(gqa_masked_attn, dim3((unsigned)(t * nqh)), dim3(threads), shmem, s,
        q, k, v, out, t, nqh, nkv, hd, prefix);
}

// NeoX partial rotary embedding, in-place on `buf` (rows, head_dim). Only the
// first `rotary_dim` dims of each head are rotated (rotate-half over that block:
// pair (i, i+rotary_dim/2)); the remaining head_dim-rotary_dim dims pass
// through. Each row's position is row/heads_per_tok. rotary_dim == head_dim is
// full rotary. angle = pos * theta^(-2i/rotary_dim).
extern "C" __global__ void rope_partial(
    double* buf, int rows, int head_dim, int rotary_dim, int heads_per_tok, double theta
) {
    int half = rotary_dim / 2;
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx >= total) return;
    int row = idx / half;
    int i = idx % half;
    int pos = row / heads_per_tok;
    double freq = (double)pos * pow(theta, -2.0 * (double)i / (double)rotary_dim);
    double c = cos(freq), s = sin(freq);
    double* b = buf + (long)row * head_dim;
    double a0 = b[i], a1 = b[i + half];
    b[i] = a0 * c - a1 * s;
    b[i + half] = a0 * s + a1 * c;
}
extern "C" void launch_rope_partial(
    double* buf, int rows, int head_dim, int rotary_dim, int heads_per_tok, double theta, hipStream_t s
) {
    int t = 256;
    long total = (long)rows * (rotary_dim / 2);
    long blk = (total + t - 1) / t;
    hipLaunchKernelGGL(rope_partial, dim3((unsigned)blk), dim3(t), 0, s,
        buf, rows, head_dim, rotary_dim, heads_per_tok, theta);
}

__device__ __forceinline__ double fi_gelu_tanh(double x) {
    return 0.5 * x * (1.0 + tanh(0.7978845608028654 * (x + 0.044715 * x * x * x)));
}

// out = gelu(a) * b, elementwise (n). Separate gate/up buffers.
extern "C" __global__ void gelu_mul(const double* a, const double* b, double* out, long n) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = fi_gelu_tanh(a[i]) * b[i];
}
extern "C" void launch_gelu_mul(const double* a, const double* b, double* out, long n, hipStream_t s) {
    int t = 256;
    long blk = (n + t - 1) / t;
    hipLaunchKernelGGL(gelu_mul, dim3((unsigned)blk), dim3(t), 0, s, a, b, out, n);
}

// Fused gate|up row layout: in is (rows, 2*half), out is (rows, half) with
// out[r,i] = gelu(in[r,i]) * in[r,half+i]. For MoE experts' gate_up_proj.
extern "C" __global__ void glu_gelu(const double* in, double* out, int rows, int half) {
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx < total) {
        int r = idx / half;
        int i = idx % half;
        long b = (long)r * 2 * half;
        out[idx] = fi_gelu_tanh(in[b + i]) * in[b + half + i];
    }
}
extern "C" void launch_glu_gelu(const double* in, double* out, int rows, int half, hipStream_t s) {
    int t = 256;
    long total = (long)rows * half;
    long blk = (total + t - 1) / t;
    hipLaunchKernelGGL(glu_gelu, dim3((unsigned)blk), dim3(t), 0, s, in, out, rows, half);
}

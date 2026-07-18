#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"


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

extern "C" __global__ void widen_bf16_f32(const unsigned short* in, float* out, long n) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        unsigned int bits = ((unsigned int)in[i]) << 16;
        out[i] = __uint_as_float(bits);
    }
}
extern "C" void launch_widen_bf16_f32(const unsigned short* in, float* out, long n, hipStream_t s) {
    int t = 256;
    long blocks = (n + t - 1) / t;
    hipLaunchKernelGGL(widen_bf16_f32, dim3((unsigned int)blocks), dim3(t), 0, s, in, out, n);
}

template<typename T> __device__ __forceinline__ T fi_block_sum(T v, T* smem) {
    int lane = threadIdx.x % warpSize;
    int wid  = threadIdx.x / warpSize;
    for (int o = warpSize / 2; o > 0; o /= 2) v += __shfl_down(v, o);
    if (lane == 0) smem[wid] = v;
    __syncthreads();
    int nw = (blockDim.x + warpSize - 1) / warpSize;
    if (wid == 0) {
        T t = (lane < nw) ? smem[lane] : 0.0;
        for (int o = warpSize / 2; o > 0; o /= 2) t += __shfl_down(t, o);
        if (lane == 0) smem[0] = t;
    }
    __syncthreads();
    T tot = smem[0];
    __syncthreads();
    return tot;
}

template<typename T> __global__ void gqa_masked_attn(
    const T* q, const T* k, const T* v, T* out,
    int t, int nqh, int nkv, int hd, int prefix, double max_bias
) {
    int blk = blockIdx.x;
    int p = blk / nqh;
    int h = blk % nqh;
    int g2 = nqh / nkv;
    int kv = h / g2;
    int qd = nqh * hd;
    int kd = nkv * hd;
    const T* qrow = q + (long)p * qd + (long)h * hd;

    double slope = 0.0;
    if (max_bias > 0.0) {
        int nhl2 = 1 << (int)floor(log2((double)nqh));
        double m0 = exp2(-max_bias / (double)nhl2);
        double m1 = exp2(-(max_bias * 0.5) / (double)nhl2);
        slope = (h < nhl2) ? pow(m0, (double)(h + 1)) : pow(m1, (double)(2 * (h - nhl2) + 1));
    }

    __shared__ T red[64];
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* sc = reinterpret_cast<T*>(smem_raw);

    for (int sp = 0; sp < t; sp++) {
        const T* krow = k + (long)sp * kd + (long)kv * hd;
        T part = 0.0;
        for (int x = threadIdx.x; x < hd; x += blockDim.x) part += qrow[x] * krow[x];
        T dot = fi_block_sum(part, red);
        if (threadIdx.x == 0) {
            bool masked = (p < prefix) && (sp > p);
            sc[sp] = masked ? -1e30 : (dot - slope * (double)(p - sp));
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) {
        T m = -1e300;
        for (int sp = 0; sp < t; sp++) if (sc[sp] > m) m = sc[sp];
        T s = 0.0;
        for (int sp = 0; sp < t; sp++) { T e = exp(sc[sp] - m); sc[sp] = e; s += e; }
        for (int sp = 0; sp < t; sp++) sc[sp] /= s;
    }
    __syncthreads();

    T* orow = out + (long)p * qd + (long)h * hd;
    for (int x = threadIdx.x; x < hd; x += blockDim.x) {
        T acc = 0.0;
        for (int sp = 0; sp < t; sp++) acc += sc[sp] * v[(long)sp * kd + (long)kv * hd + x];
        orow[x] = acc;
    }
}
extern "C" void launch_gqa_masked_attn(
    const void* q, const void* k, const void* v, void* out,
    int t, int nqh, int nkv, int hd, int prefix, double max_bias, hipStream_t s, int dtype
) {
    int threads = hd < 256 ? hd : 256;
    size_t shmem = (size_t)t * dt_elem_size(dtype);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(gqa_masked_attn<T>, dim3((unsigned)(t * nqh)), dim3(threads), shmem, s,
        (const T*)q, (const T*)k, (const T*)v, (T*)out, t, nqh, nkv, hd, prefix, max_bias));
}

// Multi-head Latent Attention scores/gather with distinct key and value head
// dims. Identical causal softmax to gqa_masked_attn, but the query-key dot runs
// over hdk (kv_lora + rope for MLA) while the value gather runs over hdv
// (kv_lora), because MLA's compressed K carries the shared RoPE key that V omits.
// q:[t,nqh,hdk] k:[t,nkv,hdk] v:[t,nkv,hdv] out:[t,nqh,hdv]; Q is pre-scaled.
template<typename T> __global__ void mla_masked_attn(
    const T* q, const T* k, const T* v, T* out,
    int t, int nqh, int nkv, int hdk, int hdv, int prefix
) {
    int blk = blockIdx.x;
    int p = blk / nqh;
    int h = blk % nqh;
    int g2 = nqh / nkv;
    int kv = h / g2;
    int qdk = nqh * hdk;
    int kdk = nkv * hdk;
    int qdv = nqh * hdv;
    int kdv = nkv * hdv;
    const T* qrow = q + (long)p * qdk + (long)h * hdk;

    __shared__ T red[64];
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* sc = reinterpret_cast<T*>(smem_raw);

    for (int sp = 0; sp < t; sp++) {
        const T* krow = k + (long)sp * kdk + (long)kv * hdk;
        T part = 0.0;
        for (int x = threadIdx.x; x < hdk; x += blockDim.x) part += qrow[x] * krow[x];
        T dot = fi_block_sum(part, red);
        if (threadIdx.x == 0) {
            bool masked = (p < prefix) && (sp > p);
            sc[sp] = masked ? -1e30 : dot;
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) {
        T m = -1e300;
        for (int sp = 0; sp < t; sp++) if (sc[sp] > m) m = sc[sp];
        T s = 0.0;
        for (int sp = 0; sp < t; sp++) { T e = exp(sc[sp] - m); sc[sp] = e; s += e; }
        for (int sp = 0; sp < t; sp++) sc[sp] /= s;
    }
    __syncthreads();

    T* orow = out + (long)p * qdv + (long)h * hdv;
    for (int x = threadIdx.x; x < hdv; x += blockDim.x) {
        T acc = 0.0;
        for (int sp = 0; sp < t; sp++) acc += sc[sp] * v[(long)sp * kdv + (long)kv * hdv + x];
        orow[x] = acc;
    }
}
extern "C" void launch_mla_masked_attn(
    const void* q, const void* k, const void* v, void* out,
    int t, int nqh, int nkv, int hdk, int hdv, int prefix, hipStream_t s, int dtype
) {
    int wide = hdk > hdv ? hdk : hdv;
    int threads = wide < 256 ? wide : 256;
    size_t shmem = (size_t)t * dt_elem_size(dtype);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(mla_masked_attn<T>, dim3((unsigned)(t * nqh)), dim3(threads), shmem, s,
        (const T*)q, (const T*)k, (const T*)v, (T*)out, t, nqh, nkv, hdk, hdv, prefix));
}

// factors (nullable): LongRoPE per-pair frequency factors [rotary_dim/2]. When
// present the pair-i angle is divided by factors[i] (ggml rope_ext freq_factors,
// minicpm3); null reproduces plain NeoX RoPE.
template<typename T> __global__ void rope_partial(
    T* buf, int rows, int head_dim, int rotary_dim, int heads_per_tok,
    const T* theta_p, const T* factors
) {
    T theta = *theta_p;
    int half = rotary_dim / 2;
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx >= total) return;
    int row = idx / half;
    int i = idx % half;
    int pos = row / heads_per_tok;
    T ff = factors ? factors[i] : 1.0;
    T freq = (T)pos * pow(theta, (T)(-2.0 * (double)i / (double)rotary_dim)) / ff;
    T c = cos(freq), s = sin(freq);
    T* b = buf + (long)row * head_dim;
    T a0 = b[i], a1 = b[i + half];
    b[i] = a0 * c - a1 * s;
    b[i + half] = a0 * s + a1 * c;
}
extern "C" void launch_rope_partial(
    void* buf, int rows, int head_dim, int rotary_dim, int heads_per_tok,
    const void* theta, const void* factors, hipStream_t s, int dtype
) {
    int t = 256;
    long total = (long)rows * (rotary_dim / 2);
    long blk = (total + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(rope_partial<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (T*)buf, rows, head_dim, rotary_dim, heads_per_tok, (const T*)theta, (const T*)factors));
}

template<typename T> __device__ __forceinline__ T fi_gelu_tanh(T x) {
    return 0.5 * x * (1.0 + tanh(0.7978845608028654 * (x + 0.044715 * x * x * x)));
}

template<typename T> __global__ void gelu_mul(const T* a, const T* b, T* out, long n) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) out[i] = fi_gelu_tanh(a[i]) * b[i];
}
extern "C" void launch_gelu_mul(const void* a, const void* b, void* out, long n, hipStream_t s, int dtype) {
    int t = 256;
    long blk = (n + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(gelu_mul<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (const T*)a, (const T*)b, (T*)out, n));
}

template<typename T> __global__ void glu_gelu(const T* in, T* out, int rows, int half) {
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx < total) {
        int r = idx / half;
        int i = idx % half;
        long b = (long)r * 2 * half;
        out[idx] = fi_gelu_tanh(in[b + i]) * in[b + half + i];
    }
}
extern "C" void launch_glu_gelu(const void* in, void* out, int rows, int half, hipStream_t s, int dtype) {
    int t = 256;
    long total = (long)rows * half;
    long blk = (total + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(glu_gelu<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (const T*)in, (T*)out, rows, half));
}

template<typename T> __global__ void glu_silu(const T* in, T* out, int rows, int half) {
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx < total) {
        int r = idx / half;
        int i = idx % half;
        long b = (long)r * 2 * half;
        T g = in[b + i];
        out[idx] = (g / (1.0 + exp(-g))) * in[b + half + i];
    }
}
extern "C" void launch_glu_silu(const void* in, void* out, int rows, int half, hipStream_t s, int dtype) {
    int t = 256;
    long total = (long)rows * half;
    long blk = (total + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(glu_silu<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (const T*)in, (T*)out, rows, half));
}

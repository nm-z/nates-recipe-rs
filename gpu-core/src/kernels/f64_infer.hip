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

// Batched flash attention (GQA/MQA) over a K/V cache. One block per
// (new-query-row i, head h); the query at absolute position p_base+i attends the
// cache rows [0, kmax) where kmax = p_base+i+1 (causal) or t_kv (bidirectional).
// Online (running max/sum) softmax: O(1) score memory, the [t_q,t_kv] score
// matrix is NEVER materialized, so the prefill (t_q>1) and the decode step
// (t_q==1) run the identical algorithm. Q for the block's new row and the value
// accumulator live in LDS; K and V are read with plain (L2-allocating) loads:
// plain vs nontemporal measured 1.61 vs 1.45 tok/s (qwen3-0.6b, 2k ctx, 2026-07-18).
//
// L2 tiling of the KV sequence: the cache is walked in tiles of `tile` rows with
// the tile loop OUTERMOST, and `tile` is sized so the K and V tiles for ALL nkv
// KV heads at one sequence offset are resident together in the 4 MiB L2 (with a
// query row and its accumulator). Every query block sharing a KV head then reads
// that offset's K/V from L2 instead of refetching MALL/VRAM.
//   K tiles (all heads) = nkv * tile * hd * elem
//   V tiles (all heads) = nkv * tile * hd * elem
//   Q row + accumulator = 2 * hd * elem
//   inequality: 2*nkv*tile*hd*elem + 2*hd*elem <= L2 (4 MiB = 4194304 B)
//   solved:     tile = (L2 - 2*hd*elem) / (2*nkv*hd*elem), clamped to [1,t_kv]
//   with elem = sizeof(T): hd=128 nkv=8 f64 -> 255; hd=256 nkv=4 f64 -> 255;
//                          hd=64 nkv=4 f64 -> 1023; hd=128 nkv=8 f32 -> 511
// The sp sweep stays ascending, so the running (max,sum,acc) reproduces the
// untiled online softmax to T rounding: sp=0 is never causally masked, so the
// running max is a real score before any masked (-1e30) row arrives.
//
// Optional cross-launch carry (m_io,l_io,acc_io indexed by query block = i*nqh+h;
// [t_q*nqh] for m/l, [t_q*nqh*hd] for acc). NULL m_io => today's kernel exactly:
// fresh (-inf,0,0), in-launch normalize to out, kv_off and finalize inert. Non-null
// => resume the prior segment's (m,l,acc), fold this segment's keys at absolute
// position kv_off+sp, then finalize=1 normalizes acc/l to out while finalize=0
// stores (m,l,acc) back and writes no out. Segments in ascending kv_off make the
// chunked output BIT-IDENTICAL to one launch (state crosses global as exact T bits,
// accumulation order and absolute positions match). Carry lives per query block,
// not in the K/V tile budget, so the L2 tile solve above is unchanged.
// q:[t_q,nqh,hd] kc:[t_kv,nkv,hd] vc:[t_kv,nkv,hd] out:[t_q,nqh,hd]; q pre-scaled.
template<typename T> __global__ void flash_gqa(
    const T* q, const T* kc, const T* vc, T* out,
    int t_q, int t_kv, int nqh, int nkv, int hd, double max_bias, int p_base, int causal_below,
    T* m_io, T* l_io, T* acc_io, int kv_off, int finalize
) {
    int blk = blockIdx.x;
    int i = blk / nqh;
    int h = blk % nqh;
    int g2 = nqh / nkv;
    int kv = h / g2;
    int qd = nqh * hd;
    int kd = nkv * hd;
    int p_abs = p_base + i;
    const T* qsrc = q + (long)i * qd + (long)h * hd;

    double slope = 0.0;
    if (max_bias > 0.0) {
        int nhl2 = 1 << (int)floor(log2((double)nqh));
        double m0 = exp2(-max_bias / (double)nhl2);
        double m1 = exp2(-(max_bias * 0.5) / (double)nhl2);
        slope = (h < nhl2) ? pow(m0, (double)(h + 1)) : pow(m1, (double)(2 * (h - nhl2) + 1));
    }

    __shared__ T red[64];
    __shared__ T sh_m, sh_s, sh_corr, sh_e;
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* acc = reinterpret_cast<T*>(smem_raw);
    T* qsh = acc + hd;
    bool carry = m_io != nullptr;
    for (int x = threadIdx.x; x < hd; x += blockDim.x) { acc[x] = carry ? acc_io[(long)blk * hd + x] : (T)0.0; qsh[x] = qsrc[x]; }
    if (threadIdx.x == 0) { sh_m = carry ? m_io[blk] : (T)(-1e300); sh_s = carry ? l_io[blk] : (T)0.0; }
    __syncthreads();

    long tl = (4194304L - 2L * hd * (long)sizeof(T)) / (2L * nkv * hd * (long)sizeof(T));
    int tile = tl < 1 ? 1 : (tl > t_kv ? t_kv : (int)tl);
    for (int t0 = 0; t0 < t_kv; t0 += tile) {
        int t1 = t0 + tile < t_kv ? t0 + tile : t_kv;
        for (int sp = t0; sp < t1; sp++) {
            const T* krow = kc + (long)sp * kd + (long)kv * hd;
            T part = 0.0;
            for (int x = threadIdx.x; x < hd; x += blockDim.x) part += qsh[x] * krow[x];
            T dot = fi_block_sum(part, red);
            if (threadIdx.x == 0) {
                bool masked = (p_abs < causal_below) && (kv_off + sp > p_abs);
                T d = masked ? (T)(-1e30) : (dot - slope * (double)(p_abs - (kv_off + sp)));
                T new_m = d > sh_m ? d : sh_m;
                T corr = exp(sh_m - new_m);
                T e = exp(d - new_m);
                sh_s = sh_s * corr + e;
                sh_m = new_m; sh_corr = corr; sh_e = e;
            }
            __syncthreads();
            const T* vrow = vc + (long)sp * kd + (long)kv * hd;
            for (int x = threadIdx.x; x < hd; x += blockDim.x) acc[x] = acc[x] * sh_corr + sh_e * vrow[x];
            __syncthreads();
        }
    }
    if (finalize || !carry) {
        T* orow = out + (long)i * qd + (long)h * hd;
        for (int x = threadIdx.x; x < hd; x += blockDim.x) orow[x] = acc[x] / sh_s;
    } else {
        for (int x = threadIdx.x; x < hd; x += blockDim.x) acc_io[(long)blk * hd + x] = acc[x];
        if (threadIdx.x == 0) { m_io[blk] = sh_m; l_io[blk] = sh_s; }
    }
}
extern "C" void launch_flash_gqa(
    const void* q, const void* kc, const void* vc, void* out,
    int t_q, int t_kv, int nqh, int nkv, int hd, double max_bias, int p_base, int causal_below,
    void* m_io, void* l_io, void* acc_io, int kv_off, int finalize, hipStream_t s, int dtype
) {
    int threads = hd < 256 ? hd : 256;
    size_t shmem = (size_t)hd * 2 * dt_elem_size(dtype);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(flash_gqa<T>, dim3((unsigned)(t_q * nqh)), dim3(threads), shmem, s,
        (const T*)q, (const T*)kc, (const T*)vc, (T*)out, t_q, t_kv, nqh, nkv, hd, max_bias, p_base, causal_below,
        (T*)m_io, (T*)l_io, (T*)acc_io, kv_off, finalize));
}

// Batched flash MLA: distinct key/value head dims (hdk dot, hdv gather), MQA
// (nkv=1). Same online-softmax + LDS-Q algorithm and same L2 tiling as flash_gqa,
// but K is hdk wide and V is hdv wide, so the resident tile pairs the two:
//   K tiles (all heads) = nkv * tile * hdk * elem
//   V tiles (all heads) = nkv * tile * hdv * elem
//   Q row + accumulator = (hdk + hdv) * elem
//   inequality: nkv*tile*(hdk+hdv)*elem + (hdk+hdv)*elem <= L2 (4 MiB)
//   solved:     tile = (L2 - (hdk+hdv)*elem) / (nkv*(hdk+hdv)*elem), clamped [1,t_kv]
//   with elem = sizeof(T): hdk=192 hdv=128 nkv=1 f64 -> 1637
// K and V are read with plain (L2-allocating) loads; the sp sweep stays ascending.
// q:[t_q,nqh,hdk] kc:[t_kv,nkv,hdk] vc:[t_kv,nkv,hdv] out:[t_q,nqh,hdv]; q pre-scaled.
template<typename T> __global__ void flash_mla(
    const T* q, const T* kc, const T* vc, T* out,
    int t_q, int t_kv, int nqh, int nkv, int hdk, int hdv, int p_base, int causal_below,
    T* m_io, T* l_io, T* acc_io, int kv_off, int finalize
) {
    int blk = blockIdx.x;
    int i = blk / nqh;
    int h = blk % nqh;
    int g2 = nqh / nkv;
    int kv = h / g2;
    int qdk = nqh * hdk;
    int kdk = nkv * hdk;
    int qdv = nqh * hdv;
    int kdv = nkv * hdv;
    int p_abs = p_base + i;
    const T* qsrc = q + (long)i * qdk + (long)h * hdk;

    __shared__ T red[64];
    __shared__ T sh_m, sh_s, sh_corr, sh_e;
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* acc = reinterpret_cast<T*>(smem_raw);
    T* qsh = acc + hdv;
    bool carry = m_io != nullptr;
    for (int x = threadIdx.x; x < hdv; x += blockDim.x) acc[x] = carry ? acc_io[(long)blk * hdv + x] : (T)0.0;
    for (int x = threadIdx.x; x < hdk; x += blockDim.x) qsh[x] = qsrc[x];
    if (threadIdx.x == 0) { sh_m = carry ? m_io[blk] : (T)(-1e300); sh_s = carry ? l_io[blk] : (T)0.0; }
    __syncthreads();

    long tl = (4194304L - (long)(hdk + hdv) * (long)sizeof(T)) / ((long)nkv * (hdk + hdv) * (long)sizeof(T));
    int tile = tl < 1 ? 1 : (tl > t_kv ? t_kv : (int)tl);
    for (int t0 = 0; t0 < t_kv; t0 += tile) {
        int t1 = t0 + tile < t_kv ? t0 + tile : t_kv;
        for (int sp = t0; sp < t1; sp++) {
            const T* krow = kc + (long)sp * kdk + (long)kv * hdk;
            T part = 0.0;
            for (int x = threadIdx.x; x < hdk; x += blockDim.x) part += qsh[x] * krow[x];
            T dot = fi_block_sum(part, red);
            if (threadIdx.x == 0) {
                bool masked = (p_abs < causal_below) && (kv_off + sp > p_abs);
                T dm = masked ? (T)(-1e30) : dot;
                T new_m = dm > sh_m ? dm : sh_m;
                T corr = exp(sh_m - new_m);
                T e = exp(dm - new_m);
                sh_s = sh_s * corr + e;
                sh_m = new_m; sh_corr = corr; sh_e = e;
            }
            __syncthreads();
            const T* vrow = vc + (long)sp * kdv + (long)kv * hdv;
            for (int x = threadIdx.x; x < hdv; x += blockDim.x) acc[x] = acc[x] * sh_corr + sh_e * vrow[x];
            __syncthreads();
        }
    }
    if (finalize || !carry) {
        T* orow = out + (long)i * qdv + (long)h * hdv;
        for (int x = threadIdx.x; x < hdv; x += blockDim.x) orow[x] = acc[x] / sh_s;
    } else {
        for (int x = threadIdx.x; x < hdv; x += blockDim.x) acc_io[(long)blk * hdv + x] = acc[x];
        if (threadIdx.x == 0) { m_io[blk] = sh_m; l_io[blk] = sh_s; }
    }
}
extern "C" void launch_flash_mla(
    const void* q, const void* kc, const void* vc, void* out,
    int t_q, int t_kv, int nqh, int nkv, int hdk, int hdv, int p_base, int causal_below,
    void* m_io, void* l_io, void* acc_io, int kv_off, int finalize, hipStream_t s, int dtype
) {
    int wide = hdk > hdv ? hdk : hdv;
    int threads = wide < 256 ? wide : 256;
    size_t shmem = (size_t)(hdk + hdv) * dt_elem_size(dtype);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(flash_mla<T>, dim3((unsigned)(t_q * nqh)), dim3(threads), shmem, s,
        (const T*)q, (const T*)kc, (const T*)vc, (T*)out, t_q, t_kv, nqh, nkv, hdk, hdv, p_base, causal_below,
        (T*)m_io, (T*)l_io, (T*)acc_io, kv_off, finalize));
}

// factors (nullable): LongRoPE per-pair frequency factors [rotary_dim/2]. When
// present the pair-i angle is divided by factors[i] (ggml rope_ext freq_factors,
// minicpm3); null reproduces plain NeoX RoPE.
template<typename T> __global__ void rope_partial(
    T* buf, int rows, int head_dim, int rotary_dim, int heads_per_tok,
    const T* theta_p, const T* factors, int pos_base
) {
    T theta = *theta_p;
    int half = rotary_dim / 2;
    long idx = (long)blockIdx.x * blockDim.x + threadIdx.x;
    long total = (long)rows * half;
    if (idx >= total) return;
    int row = idx / half;
    int i = idx % half;
    int pos = pos_base + row / heads_per_tok;
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
    const void* theta, const void* factors, int pos_base, hipStream_t s, int dtype
) {
    int t = 256;
    long total = (long)rows * (rotary_dim / 2);
    long blk = (total + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(rope_partial<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (T*)buf, rows, head_dim, rotary_dim, heads_per_tok, (const T*)theta, (const T*)factors, pos_base));
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

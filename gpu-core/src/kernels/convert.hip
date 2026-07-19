#include <hip/hip_runtime.h>
#include "convert.h"

/// f16 bit decode, sourced from a byte pair so no 2-byte alignment is assumed
/// of a quant block field. Bit-identical to the host codec it replaces.
__device__ __forceinline__ float cv_h2f(unsigned short b) {
    unsigned int s = (unsigned int)((b >> 15) & 1);
    int e = (int)((b >> 10) & 0x1f);
    unsigned int m = (unsigned int)(b & 0x3ff);
    unsigned int bits;
    if (e == 0) {
        if (m == 0) {
            bits = s << 31;
        } else {
            int e2 = -14;
            unsigned int m2 = m;
            while ((m2 & 0x400u) == 0u) { m2 <<= 1; e2 -= 1; }
            m2 &= 0x3ffu;
            bits = (s << 31) | ((unsigned int)(e2 + 127) << 23) | (m2 << 13);
        }
    } else if (e == 31) {
        bits = (s << 31) | (0xffu << 23) | (m << 13);
    } else {
        bits = (s << 31) | ((unsigned int)(e - 15 + 127) << 23) | (m << 13);
    }
    return __uint_as_float(bits);
}

__device__ __forceinline__ unsigned short cv_f2h(float x) {
    unsigned int bits = __float_as_uint(x);
    unsigned short sign = (unsigned short)((bits >> 16) & 0x8000u);
    int exp = (int)((bits >> 23) & 0xffu);
    unsigned int mant = bits & 0x7fffffu;
    if (exp == 0xff) {
        return (unsigned short)(sign | 0x7c00u | (mant != 0u ? 0x200u : 0u));
    }
    int e = exp - 127 + 15;
    if (e >= 0x1f) return (unsigned short)(sign | 0x7c00u);
    if (e <= 0) {
        if (e < -10) return sign;
        return (unsigned short)(sign | (unsigned short)((mant | 0x800000u) >> (14 - e)));
    }
    return (unsigned short)(sign | ((unsigned short)e << 10) | (unsigned short)(mant >> 13));
}

__device__ __forceinline__ unsigned short cv_f2bf(float x) {
    unsigned int bits = __float_as_uint(x);
    if ((bits & 0x7fffffffu) > 0x7f800000u) return (unsigned short)((bits >> 16) | 0x0040u);
    return (unsigned short)((bits + 0x7fffu + ((bits >> 16) & 1u)) >> 16);
}

__device__ __forceinline__ unsigned short cv_u16(const unsigned char* p) {
    return (unsigned short)((unsigned int)p[0] | ((unsigned int)p[1] << 8));
}

__device__ __forceinline__ unsigned int cv_u32(const unsigned char* p) {
    return (unsigned int)p[0] | ((unsigned int)p[1] << 8) | ((unsigned int)p[2] << 16)
         | ((unsigned int)p[3] << 24);
}

/// The 6-bit scale/min pair packed across 12 bytes (ggml `get_scale_min_k4`).
__device__ __forceinline__ void cv_scale_min(int j, const unsigned char* q,
                                             unsigned char* s, unsigned char* m) {
    if (j < 4) {
        *s = (unsigned char)(q[j] & 63);
        *m = (unsigned char)(q[j + 4] & 63);
    } else {
        *s = (unsigned char)((q[j + 4] & 0xF) | ((q[j - 4] >> 6) << 4));
        *m = (unsigned char)((q[j + 4] >> 4) | ((q[j] >> 6) << 4));
    }
}

/// Reads element `i` of a buffer holding dtype `dt` as a double. Quant dtypes
/// resolve their own block header per element: the walk is the exact inverse of
/// the reference row loops, so element `i` here equals element `i` of the row
/// the reference decoder would emit.
__device__ __forceinline__ double cv_load(const unsigned char* p, long i, int dt) {
    switch (dt) {
        case CV_F64: return ((const double*)p)[i];
        case CV_F32: return (double)((const float*)p)[i];
        case CV_F16: return (double)cv_h2f(cv_u16(p + i * 2));
        case CV_BF16: return (double)__uint_as_float((unsigned int)cv_u16(p + i * 2) << 16);
        case CV_Q4_0: {
            const unsigned char* b = p + (i >> 5) * 18;
            int j = (int)(i & 31);
            float d = cv_h2f(cv_u16(b));
            int q = j < 16 ? (b[2 + j] & 0xF) : (b[2 + j - 16] >> 4);
            return (double)((float)(q - 8) * d);
        }
        case CV_Q4_1: {
            const unsigned char* b = p + (i >> 5) * 20;
            int j = (int)(i & 31);
            float d = cv_h2f(cv_u16(b));
            float m = cv_h2f(cv_u16(b + 2));
            int q = j < 16 ? (b[4 + j] & 0xF) : (b[4 + j - 16] >> 4);
            return (double)((float)q * d + m);
        }
        case CV_Q5_0: {
            const unsigned char* b = p + (i >> 5) * 22;
            int j = (int)(i & 31);
            float d = cv_h2f(cv_u16(b));
            unsigned int qh = cv_u32(b + 2);
            unsigned int x = ((qh >> j) & 1u) << 4;
            unsigned int q = j < 16 ? ((unsigned int)(b[6 + j] & 0xF) | x)
                                    : ((unsigned int)(b[6 + j - 16] >> 4) | x);
            return (double)(d * ((float)q - 16.0f));
        }
        case CV_Q5_1: {
            const unsigned char* b = p + (i >> 5) * 24;
            int j = (int)(i & 31);
            float d = cv_h2f(cv_u16(b));
            float m = cv_h2f(cv_u16(b + 2));
            unsigned int qh = cv_u32(b + 4);
            unsigned int q;
            if (j < 16) {
                q = (unsigned int)(b[8 + j] & 0xF) | ((qh >> j) << 4 & 0x10u);
            } else {
                int l = j - 16;
                q = (unsigned int)(b[8 + l] >> 4) | ((qh >> (l + 12)) & 0x10u);
            }
            return (double)((float)q * d + m);
        }
        case CV_Q8_0: {
            const unsigned char* b = p + (i >> 5) * 34;
            int j = (int)(i & 31);
            float d = cv_h2f(cv_u16(b));
            return (double)(d * (float)((signed char)b[2 + j]));
        }
        case CV_Q2_K: {
            const unsigned char* b = p + (i >> 8) * 84;
            int e = (int)(i & 255);
            int nb = e >> 7, iter = (e >> 5) & 3, half = (e >> 4) & 1, l = e & 15;
            unsigned char sc = b[nb * 8 + iter * 2 + half];
            float d = cv_h2f(cv_u16(b + 80));
            float mn = cv_h2f(cv_u16(b + 82));
            float dl = d * (float)(sc & 0xF);
            float ml = mn * (float)(sc >> 4);
            unsigned char q = b[16 + nb * 32 + half * 16 + l];
            return (double)(dl * (float)((q >> (iter * 2)) & 3) - ml);
        }
        case CV_Q3_K: {
            const unsigned char* b = p + (i >> 8) * 110;
            int e = (int)(i & 255);
            int nb = e >> 7, iter = (e >> 5) & 3, half = (e >> 4) & 1, l = e & 15;
            unsigned int kmask1 = 0x03030303u, kmask2 = 0x0f0f0f0fu;
            unsigned int a0 = cv_u32(b + 96), a1 = cv_u32(b + 100), tmp = cv_u32(b + 104);
            int is = nb * 8 + iter * 2 + half;
            unsigned int word;
            switch (is >> 2) {
                case 0: word = (a0 & kmask2) | (((tmp >> 0) & kmask1) << 4); break;
                case 1: word = (a1 & kmask2) | (((tmp >> 2) & kmask1) << 4); break;
                case 2: word = ((a0 >> 4) & kmask2) | (((tmp >> 4) & kmask1) << 4); break;
                default: word = ((a1 >> 4) & kmask2) | (((tmp >> 6) & kmask1) << 4); break;
            }
            signed char scv = (signed char)((word >> ((is & 3) * 8)) & 0xffu);
            float dl = cv_h2f(cv_u16(b + 108)) * (float)((int)scv - 32);
            unsigned char m = (unsigned char)(1u << (nb * 4 + iter));
            unsigned char hm = b[half * 16 + l];
            int hbit = (hm & m) != 0 ? 0 : 4;
            int v = (int)((b[32 + nb * 32 + half * 16 + l] >> (iter * 2)) & 3) - hbit;
            return (double)(dl * (float)v);
        }
        case CV_Q4_K: {
            const unsigned char* b = p + (i >> 8) * 144;
            int e = (int)(i & 255);
            int jb = e >> 6, half = (e >> 5) & 1, l = e & 31;
            float d = cv_h2f(cv_u16(b));
            float dmin = cv_h2f(cv_u16(b + 2));
            unsigned char s, m;
            cv_scale_min(jb * 2 + half, b + 4, &s, &m);
            unsigned char q = b[16 + jb * 32 + l];
            float nib = (float)(half == 0 ? (q & 0xF) : (q >> 4));
            return (double)(d * (float)s * nib - dmin * (float)m);
        }
        case CV_Q5_K: {
            const unsigned char* b = p + (i >> 8) * 176;
            int e = (int)(i & 255);
            int jb = e >> 6, half = (e >> 5) & 1, l = e & 31;
            float d = cv_h2f(cv_u16(b));
            float dmin = cv_h2f(cv_u16(b + 2));
            unsigned char s, m;
            cv_scale_min(jb * 2 + half, b + 4, &s, &m);
            unsigned char u = (unsigned char)((half == 0 ? 1u : 2u) << (2 * jb));
            int hi = (b[16 + l] & u) != 0 ? 16 : 0;
            unsigned char q = b[48 + jb * 32 + l];
            int nib = half == 0 ? (q & 0xF) : (q >> 4);
            return (double)(d * (float)s * (float)(nib + hi) - dmin * (float)m);
        }
        case CV_Q6_K: {
            const unsigned char* b = p + (i >> 8) * 210;
            int e = (int)(i & 255);
            int nn = e >> 7, grp = (e >> 5) & 3, l = e & 31;
            unsigned char ql = b[nn * 64 + ((grp & 1) ? l + 32 : l)];
            unsigned char qh = b[128 + nn * 32 + l];
            int nib = (grp >= 2) ? (ql >> 4) : (ql & 0xF);
            int q = (nib | (int)(((qh >> (grp * 2)) & 3) << 4)) - 32;
            signed char sc = (signed char)b[192 + nn * 8 + (l >> 4) + grp * 2];
            float d = cv_h2f(cv_u16(b + 208));
            return (double)(d * (float)sc * (float)q);
        }
        default: return 0.0;
    }
}

__device__ __forceinline__ void cv_store(unsigned char* p, long i, double v, int dt) {
    switch (dt) {
        case CV_F64: ((double*)p)[i] = v; break;
        case CV_F32: ((float*)p)[i] = (float)v; break;
        case CV_F16: ((unsigned short*)p)[i] = cv_f2h((float)v); break;
        case CV_BF16: ((unsigned short*)p)[i] = cv_f2bf((float)v); break;
        default: break;
    }
}

/// One thread per element: the destination is element-granular, the source is
/// anything `cv_load` reads. Every value is scaled in double before it narrows.
__global__ void convert_elem_kernel(const unsigned char* __restrict__ src,
                                    unsigned char* __restrict__ dst, long n, double scale,
                                    int sdt, int ddt) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) cv_store(dst, i, cv_load(src, i, sdt) * scale, ddt);
}

/// One thread per destination block, `quantize_row_q8_0_ref`: a block's scale
/// needs every one of its elements before a single byte lands, which is why
/// encoding owns a block where decoding owns an element. The source is anything
/// `cv_load` reads, so requant (quant in, quant out) is the same launch as
/// encode. Q8_0 is the only encoder wired; `gpu_convert` rejects every other
/// quant destination by name before reaching here.
__global__ void convert_q8_out_kernel(const unsigned char* __restrict__ src,
                                      unsigned char* __restrict__ dst, long nblocks, double scale,
                                      int sdt) {
    long blk = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (blk >= nblocks) return;
    long base = blk * 32;
    float amax = 0.0f;
    for (int j = 0; j < 32; ++j) {
        float v = (float)(cv_load(src, base + j, sdt) * scale);
        amax = fmaxf(amax, fabsf(v));
    }
    unsigned char* b = dst + blk * 34;
    float d = amax / 127.0f;
    float id = d != 0.0f ? 1.0f / d : 0.0f;
    unsigned short dh = cv_f2h(d);
    b[0] = (unsigned char)(dh & 0xff);
    b[1] = (unsigned char)(dh >> 8);
    for (int j = 0; j < 32; ++j) {
        float v = (float)(cv_load(src, base + j, sdt) * scale);
        b[2 + j] = (unsigned char)(signed char)roundf(v * id);
    }
}

/// THE embedding read: row `r` of `out` is the `scale`-weighted sum of `k`
/// staged source rows, converted from `sdt` on the way in. A plain token
/// embedding is this with `k = 1` and weight `1.0`; a diffusion canvas blend is
/// the same call with `k` candidates and their probabilities. One kernel, so
/// there is no CPU-side blend to drift from it.
__global__ void embed_blend_kernel(const unsigned char* __restrict__ rowsrc,
                                   const double* __restrict__ w, unsigned char* __restrict__ out,
                                   long rows, int k, int ne, double scale, int sdt, int ddt) {
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= rows * (long)ne) return;
    long r = i / ne;
    int x = (int)(i % ne);
    double acc = 0.0;
    for (int j = 0; j < k; ++j) {
        long src = (r * k + j) * (long)ne + x;
        acc += w[r * k + j] * cv_load(rowsrc, src, sdt);
    }
    cv_store(out, i, acc * scale, ddt);
}

extern "C" void launch_embed_blend(const void* rowsrc, const void* w, void* out, long rows, int k,
                                   int ne, double scale, int sdt, int ddt, hipStream_t s) {
    int t = 256;
    long n = rows * (long)ne;
    long blocks = (n + t - 1) / t;
    hipLaunchKernelGGL(embed_blend_kernel, dim3((unsigned int)blocks), dim3(t), 0, s,
        (const unsigned char*)rowsrc, (const double*)w, (unsigned char*)out, rows, k, ne, scale,
        sdt, ddt);
}

extern "C" void launch_convert(const void* src, void* dst, long n, double scale, int sdt,
                               int ddt, hipStream_t s) {
    int t = 256;
    if (ddt == CV_Q8_0) {
        long nblocks = n / 32;
        long blocks = (nblocks + t - 1) / t;
        hipLaunchKernelGGL(convert_q8_out_kernel, dim3((unsigned int)blocks), dim3(t), 0, s,
            (const unsigned char*)src, (unsigned char*)dst, nblocks, scale, sdt);
        return;
    }
    long blocks = (n + t - 1) / t;
    hipLaunchKernelGGL(convert_elem_kernel, dim3((unsigned int)blocks), dim3(t), 0, s,
        (const unsigned char*)src, (unsigned char*)dst, n, scale, sdt, ddt);
}

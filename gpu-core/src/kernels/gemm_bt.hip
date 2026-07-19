#include <hip/hip_runtime.h>
#include <hip/hip_fp16.h>
#include <hip/hip_bf16.h>
#include <cstdio>
#include <cstdlib>
#include "dtype_dispatch.h"

template<typename T> struct acc_of { using type = T; };
template<> struct acc_of<__half> { using type = float; };
template<> struct acc_of<__hip_bfloat16> { using type = float; };

template<typename T> __global__ void gemm_bt(
    const T* __restrict__ A,
    const T* __restrict__ B,
    T* __restrict__ C,
    int m, int n, int k,
    int tile_m, int tile_n, int tile_k
) {
    using A_ = typename acc_of<T>::type;
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* bs = reinterpret_cast<T*>(smem_raw);
    int j0 = blockIdx.x * tile_n;
    int jt = (n - j0 < tile_n) ? (n - j0) : tile_n;
    int i0 = blockIdx.y * tile_m;
    int iend = (m - i0 < tile_m) ? m : i0 + tile_m;

    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;

    for (int c0 = 0; c0 < k; c0 += tile_k) {
        int clen = (k - c0 < tile_k) ? (k - c0) : tile_k;
        for (int jj = 0; jj < tile_n; ++jj) {
            const T* brow = B + (long)(j0 + (jj < jt ? jj : 0)) * k + c0;
            bool live = jj < jt;
            for (int x = threadIdx.x; x < clen; x += blockDim.x)
                bs[(size_t)jj * clen + x] = live ? brow[x] : (T)0.0f;
        }
        __syncthreads();

        for (int i = i0 + wid; i < iend; i += nwarps) {
            const T* arow = A + (long)i * k + c0;
            T* crow = C + (long)i * n + j0;
            for (int jb = 0; jb < tile_n; jb += 2) {
                const T* b0 = bs + (size_t)jb * clen;
                const T* b1 = b0 + clen;
                A_ p0 = 0.0f;
                A_ p1 = 0.0f;
                for (int x = lane; x < clen; x += warpSize) {
                    A_ av = (A_)arow[x];
                    p0 += av * (A_)b0[x];
                    p1 += av * (A_)b1[x];
                }
                for (int offset = warpSize / 2; offset > 0; offset /= 2) {
                    p0 += __shfl_down(p0, offset);
                    p1 += __shfl_down(p1, offset);
                }
                if (lane == 0) {
                    if (jb < jt) {
                        if (c0 == 0) crow[jb] = (T)p0;
                        else crow[jb] = (T)((A_)crow[jb] + p0);
                    }
                    if (jb + 1 < jt) {
                        if (c0 == 0) crow[jb + 1] = (T)p1;
                        else crow[jb + 1] = (T)((A_)crow[jb + 1] + p1);
                    }
                }
            }
        }
        __syncthreads();
    }
}

static int gemm_bt_cus(void) {
    static int cus = [] {
        int dev = 0;
        hipError_t e = hipGetDevice(&dev);
        if (e != hipSuccess) {
            fprintf(stderr, "gemm_bt: hipGetDevice: %s\n", hipGetErrorString(e));
            abort();
        }
        hipDeviceProp_t p;
        e = hipGetDeviceProperties(&p, dev);
        if (e != hipSuccess) {
            fprintf(stderr, "gemm_bt: hipGetDeviceProperties: %s\n", hipGetErrorString(e));
            abort();
        }
        return p.multiProcessorCount;
    }();
    return cus;
}

extern "C" void launch_gemm_bt_tiles(
    const void* a, const void* b, void* c, int m, int n, int k,
    int tile_m, int tile_n, int tile_k, hipStream_t s, int dtype
) {
    int threads = 256;
    tile_n &= ~1;
    if (tile_n < 2) tile_n = 2;
    if (tile_k < 1) tile_k = 1;
    if (k > 0 && tile_k > k) tile_k = k;
    if (tile_m < 1) tile_m = 1;
    int min_tm = (m + 65534) / 65535;
    if (tile_m < min_tm) tile_m = min_tm;
    size_t shmem = (size_t)tile_n * (size_t)tile_k * dt_elem_size(dtype);
    int bx = (n + tile_n - 1) / tile_n;
    int by = (m + tile_m - 1) / tile_m;
    DT_DISPATCH_F(dtype, hipLaunchKernelGGL(gemm_bt<T>, dim3((unsigned)bx, (unsigned)by), dim3(threads), shmem, s,
        (const T*)a, (const T*)b, (T*)c, m, n, k, tile_m, tile_n, tile_k));
}

extern "C" void launch_gemm_bt(
    const void* a, const void* b, void* c, int m, int n, int k, hipStream_t s, int dtype
) {
    long esz = (long)dt_elem_size(dtype);
    int tile_k = (k >= 1 && k < 1024) ? k : 1024;
    long lds_cols = 65536L / ((long)tile_k * esz);
    long fill = (long)n / (2L * gemm_bt_cus());
    long tn = lds_cols < fill ? lds_cols : fill;
    tn &= ~1L;
    if (tn < 2) tn = 2;
    long bx = ((long)n + tn - 1) / tn;
    long ny = (2L * gemm_bt_cus()) / (bx > 0 ? bx : 1);
    if (ny < 1) ny = 1;
    long tm = ((long)m + ny - 1) / ny;
    if (tm < 1) tm = 1;
    launch_gemm_bt_tiles(a, b, c, m, n, k, (int)tm, (int)tn, tile_k, s, dtype);
}

template<typename T> __global__ void scale_f64(T* x, const T* scalar_p, long n) {
    T scalar = *scalar_p;
    long i = (long)blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) x[i] *= scalar;
}
extern "C" void launch_scale_f64(void* x, const void* scalar, long n, hipStream_t s, int dtype) {
    int t = 256;
    long blk = (n + t - 1) / t;
    DT_DISPATCH(dtype, hipLaunchKernelGGL(scale_f64<T>, dim3((unsigned)blk), dim3(t), 0, s,
        (T*)x, (const T*)scalar, n));
}

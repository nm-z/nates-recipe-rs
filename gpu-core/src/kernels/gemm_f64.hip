#include <hip/hip_runtime.h>
#include "dtype_dispatch.h"


// LDS chunk width in doubles. bounds shared-memory use at GEMM_BT_CHUNK*8 bytes
// (32 KiB) so k may exceed the per-block LDS limit; k is tiled in chunks and each
// column's partial dot products accumulate into C across chunks.
#define GEMM_BT_CHUNK 4096

template<typename T> __global__ void gemm_bt_f64(
    const T* __restrict__ A,
    const T* __restrict__ B,
    T* __restrict__ C,
    int m, int n, int k
) {
    HIP_DYNAMIC_SHARED(char, smem_raw);
    T* bs = reinterpret_cast<T*>(smem_raw);
    int j = blockIdx.x;
    const T* brow = B + (long)j * k;

    int lane = threadIdx.x % warpSize;
    int wid = threadIdx.x / warpSize;
    int nwarps = (blockDim.x + warpSize - 1) / warpSize;

    for (int c0 = 0; c0 < k; c0 += GEMM_BT_CHUNK) {
        int clen = (k - c0 < GEMM_BT_CHUNK) ? (k - c0) : GEMM_BT_CHUNK;
        for (int x = threadIdx.x; x < clen; x += blockDim.x) bs[x] = brow[c0 + x];
        __syncthreads();

        for (int i = wid; i < m; i += nwarps) {
            const T* arow = A + (long)i * k + c0;
            T part = 0.0;
            for (int x = lane; x < clen; x += warpSize) part += arow[x] * bs[x];
            for (int offset = warpSize / 2; offset > 0; offset /= 2)
                part += __shfl_down(part, offset);
            if (lane == 0) {
                if (c0 == 0) C[(long)i * n + j] = part;
                else C[(long)i * n + j] += part;
            }
        }
        __syncthreads();
    }
}

extern "C" void launch_gemm_bt_f64(
    const void* a, const void* b, void* c, int m, int n, int k, hipStream_t s, int dtype
) {
    int threads = 256;
    int chunk = k < GEMM_BT_CHUNK ? k : GEMM_BT_CHUNK;
    size_t shmem = (size_t)chunk * dt_elem_size(dtype);
    DT_DISPATCH(dtype, hipLaunchKernelGGL(gemm_bt_f64<T>, dim3((unsigned)n), dim3(threads), shmem, s,
        (const T*)a, (const T*)b, (T*)c, m, n, k));
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

// ───────────────────────────────────────────────────────────────────────────
// HIP host-runtime → CUDA backend for NVIDIA.
//
// hipBLAS/hipSOLVER/hipFFT are NOT here: on NVIDIA the framework links the real
// from-source libhipblas/libhipsolver/libhipfft built for HIP_PLATFORM=nvidia,
// which dispatch to cuBLAS/cuSOLVER/cuFFT internally — same as on AMD where they
// dispatch to rocBLAS/rocSOLVER/rocFFT. This file provides only what no pre-built
// library can on NVIDIA: the HIP host runtime (hipMalloc/hipMemcpy/streams/events
// …) — header-only inline on NVIDIA, so it exports no linkable symbols; the Rust
// FFI needs real ones, materialised here over the CUDA runtime.
//
// Compiled with nvcc, linked ONLY on the NVIDIA build.
// ───────────────────────────────────────────────────────────────────────────
#include <cuda_runtime.h>
#include <cublas_v2.h>
#include <cstdio>
#include <cstdlib>
#include <cstddef>
typedef long long hipblasStride; // i64 in the Rust decls

extern "C" {

// ── HIP host runtime ───────────────────────────────────────────────────────
// HIP and CUDA enum integers agree for the values this crate passes
// (hipMemcpyKind H2D=1/D2H=2/D2D=3 == cudaMemcpyKind; host-alloc flags; success 0).

int hipMalloc(void** ptr, size_t size) { return (int)cudaMalloc(ptr, size); }
int hipFree(void* ptr) { return (int)cudaFree(ptr); }
int hipMemcpy(void* dst, const void* src, size_t size, int kind) {
      return (int)cudaMemcpy(dst, src, size, (cudaMemcpyKind)kind);
}
int hipMemset(void* dst, int value, size_t size) { return (int)cudaMemset(dst, value, size); }
int hipGetLastError() { return (int)cudaGetLastError(); }
int hipPeekAtLastError() { return (int)cudaPeekAtLastError(); }
int hipDeviceSynchronize() { return (int)cudaDeviceSynchronize(); }

int hipEventCreate(void** event) { return (int)cudaEventCreate((cudaEvent_t*)event); }
int hipEventDestroy(void* event) { return (int)cudaEventDestroy((cudaEvent_t)event); }
int hipEventRecord(void* event, void* stream) {
      return (int)cudaEventRecord((cudaEvent_t)event, (cudaStream_t)stream);
}
int hipEventSynchronize(void* event) { return (int)cudaEventSynchronize((cudaEvent_t)event); }
int hipEventElapsedTime(float* ms, void* start, void* stop) {
      return (int)cudaEventElapsedTime(ms, (cudaEvent_t)start, (cudaEvent_t)stop);
}

int hipSetDevice(int device) { return (int)cudaSetDevice(device); }
int hipGetDeviceCount(int* count) { return (int)cudaGetDeviceCount(count); }

int hipStreamCreate(void** stream) { return (int)cudaStreamCreate((cudaStream_t*)stream); }
int hipStreamSynchronize(void* stream) { return (int)cudaStreamSynchronize((cudaStream_t)stream); }
int hipStreamDestroy(void* stream) { return (int)cudaStreamDestroy((cudaStream_t)stream); }

int hipMemGetInfo(size_t* freeMem, size_t* totalMem) { return (int)cudaMemGetInfo(freeMem, totalMem); }

const char* hipGetErrorName(int error) { return cudaGetErrorName((cudaError_t)error); }
const char* hipGetErrorString(int error) { return cudaGetErrorString((cudaError_t)error); }

int hipMemcpyAsync(void* dst, const void* src, size_t size, int kind, void* stream) {
      return (int)cudaMemcpyAsync(dst, src, size, (cudaMemcpyKind)kind, (cudaStream_t)stream);
}
int hipMemsetAsync(void* dst, int value, size_t size, void* stream) {
      return (int)cudaMemsetAsync(dst, value, size, (cudaStream_t)stream);
}

int hipHostMalloc(void** ptr, size_t size, unsigned int flags) {
      return (int)cudaHostAlloc(ptr, size, flags);
}
int hipHostFree(void* ptr) { return (int)cudaFreeHost(ptr); }
int hipHostRegister(void* ptr, size_t size, unsigned int flags) {
      return (int)cudaHostRegister(ptr, size, flags);
}
int hipHostUnregister(void* ptr) { return (int)cudaHostUnregister(ptr); }

int hipDeviceGetAttribute(int* pi, int attr, int device) {
      return (int)cudaDeviceGetAttribute(pi, (cudaDeviceAttr)attr, device);
}
int hipDeviceCanAccessPeer(int* canAccess, int device, int peer) {
      return (int)cudaDeviceCanAccessPeer(canAccess, device, peer);
}
int hipDeviceEnablePeerAccess(int peer, unsigned int flags) {
      return (int)cudaDeviceEnablePeerAccess(peer, flags);
}
int hipMemcpyPeer(void* dst, int dstDev, const void* src, int srcDev, size_t size) {
      return (int)cudaMemcpyPeer(dst, dstDev, src, srcDev, size);
}

int hipMallocAsync(void** ptr, size_t size, void* stream) {
      return (int)cudaMallocAsync(ptr, size, (cudaStream_t)stream);
}
int hipFreeAsync(void* ptr, void* stream) {
      return (int)cudaFreeAsync(ptr, (cudaStream_t)stream);
}
int hipMallocManaged(void** ptr, size_t size, unsigned int flags) {
      return (int)cudaMallocManaged(ptr, size, flags);
}

} // extern "C"

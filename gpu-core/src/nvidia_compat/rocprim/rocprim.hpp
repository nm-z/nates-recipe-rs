// rocPRIM → CUDA CUB/Thrust shim for the NVIDIA backend.
//
// rocPRIM is AMD-only (no CUDA backend), so the device-wide primitives the
// kernels use are re-expressed on top of CUDA's CUB and Thrust. rocPRIM and CUB
// share lineage and semantics; only argument orders differ, so each entry point
// is a thin forwarding wrapper. Reduction functors and the transform/counting
// iterator helpers are provided in the rocprim:: namespace to match call sites.
#pragma once
#ifndef ROCPRIM_NV_SHIM_HPP
#define ROCPRIM_NV_SHIM_HPP

#include <cub/cub.cuh>
#include <thrust/transform.h>
#include <thrust/execution_policy.h>
#include <thrust/iterator/transform_iterator.h>
#include <thrust/iterator/counting_iterator.h>
#include <hip/hip_runtime.h>

namespace rocprim {

// ── reduction functors ─────────────────────────────────────────────────────
template <class T>
struct plus {
      __host__ __device__ T operator()(const T& a, const T& b) const { return a + b; }
};
template <class T>
struct multiplies {
      __host__ __device__ T operator()(const T& a, const T& b) const { return a * b; }
};
template <class T>
struct maximum {
      __host__ __device__ T operator()(const T& a, const T& b) const { return a < b ? b : a; }
};
template <class T>
struct minimum {
      __host__ __device__ T operator()(const T& a, const T& b) const { return b < a ? b : a; }
};

// ── iterator helpers ────────────────────────────────────────────────────────
template <class Iter, class UnaryOp>
__host__ __device__ auto make_transform_iterator(Iter it, UnaryOp op) {
      return thrust::make_transform_iterator(it, op);
}
template <class T>
__host__ __device__ auto make_counting_iterator(T v) {
      return thrust::make_counting_iterator(v);
}

// ── device reductions / scans / sort ────────────────────────────────────────
// rocprim::reduce(temp, bytes, in, out, init, n, op, stream)
template <class InputIt, class OutputIt, class InitT, class BinaryOp>
hipError_t reduce(void* temp, size_t& bytes, InputIt in, OutputIt out, InitT init, size_t n,
                  BinaryOp op, hipStream_t stream = 0) {
      return (hipError_t)(int)cub::DeviceReduce::Reduce(temp, bytes, in, out, (int)n, op, (InitT)init,
                                                        (cudaStream_t)stream);
}

// rocprim::segmented_reduce(temp, bytes, in, out, num_segments,
//                           begin_offsets, end_offsets, op, init, stream)
template <class InputIt, class OutputIt, class OffsetIt, class BinaryOp, class InitT>
hipError_t segmented_reduce(void* temp, size_t& bytes, InputIt in, OutputIt out,
                            unsigned int num_segments, OffsetIt begin_offsets, OffsetIt end_offsets,
                            BinaryOp op, InitT init, hipStream_t stream = 0) {
      return (hipError_t)(int)cub::DeviceSegmentedReduce::Reduce(
          temp, bytes, in, out, (int)num_segments, begin_offsets, end_offsets, op, (InitT)init,
          (cudaStream_t)stream);
}

// rocprim::inclusive_scan(temp, bytes, in, out, n, op, stream)
template <class InputIt, class OutputIt, class BinaryOp>
hipError_t inclusive_scan(void* temp, size_t& bytes, InputIt in, OutputIt out, size_t n, BinaryOp op,
                          hipStream_t stream = 0) {
      return (hipError_t)(int)cub::DeviceScan::InclusiveScan(temp, bytes, in, out, op, (int)n,
                                                             (cudaStream_t)stream);
}

// rocprim::exclusive_scan(temp, bytes, in, out, init, n, op, stream)
template <class InputIt, class OutputIt, class InitT, class BinaryOp>
hipError_t exclusive_scan(void* temp, size_t& bytes, InputIt in, OutputIt out, InitT init, size_t n,
                          BinaryOp op, hipStream_t stream = 0) {
      return (hipError_t)(int)cub::DeviceScan::ExclusiveScan(temp, bytes, in, out, op, (InitT)init,
                                                             (int)n, (cudaStream_t)stream);
}

// rocprim::radix_sort_pairs(temp, bytes, kin, kout, vin, vout, n, begin_bit, end_bit, stream)
template <class Key, class Value>
hipError_t radix_sort_pairs(void* temp, size_t& bytes, const Key* kin, Key* kout, const Value* vin,
                            Value* vout, size_t n, int begin_bit = 0, int end_bit = sizeof(Key) * 8,
                            hipStream_t stream = 0) {
      return (hipError_t)(int)cub::DeviceRadixSort::SortPairs(temp, bytes, kin, kout, vin, vout,
                                                             (int)n, begin_bit, end_bit,
                                                             (cudaStream_t)stream);
}

// rocprim::transform(in, out, n, op, stream)  — elementwise unary transform
template <class InputIt, class OutputIt, class UnaryOp>
hipError_t transform(InputIt in, OutputIt out, size_t n, UnaryOp op, hipStream_t stream = 0) {
      thrust::transform(thrust::cuda::par.on((cudaStream_t)stream), in, in + n, out, op);
      return hipSuccess;
}

} // namespace rocprim

#endif

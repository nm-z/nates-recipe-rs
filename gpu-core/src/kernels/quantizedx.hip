#include <hip/hip_runtime.h>

// ── Quantization primitives (affine per-tensor) ──────────────────────────────
// Convention (PyTorch / TFLite / ONNX affine quant):
//   quantize   : q = clamp(round(x/scale) + zero_point, qmin, qmax)
//   dequantize : x = (q - zero_point) * scale
//   fake_quant : x' = (clamp(round(x/scale)+zp, qmin, qmax) - zp) * scale
// round() is half-away-from-zero (HIP device round == C round == f64::round).
// int8 storage uses signed char (i8); int4 uses one value per byte in [-8,7].

// ── quantize_per_tensor → i8 (qmin=-128, qmax=127) ───────────────────────────
extern "C" __global__ void quantizedx_quantize_i8_kernel(
      const double* x, signed char* q, double scale, int zero_point,
      int qmin, int qmax, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double r = round(x[i] / scale) + (double)zero_point;
            int v = (int)r;
            if (v < qmin) v = qmin;
            if (v > qmax) v = qmax;
            q[i] = (signed char)v;
      }
}
extern "C" void launch_quantizedx_quantize_i8(
      const double* x, signed char* q, double scale, int zero_point,
      int qmin, int qmax, int n, hipStream_t s
) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(quantizedx_quantize_i8_kernel, grid, block, 0, s,
                         x, q, scale, zero_point, qmin, qmax, n);
}

// ── dequantize from i8 ───────────────────────────────────────────────────────
extern "C" __global__ void quantizedx_dequantize_i8_kernel(
      const signed char* q, double* x, double scale, int zero_point, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            x[i] = ((double)(int)q[i] - (double)zero_point) * scale;
      }
}
extern "C" void launch_quantizedx_dequantize_i8(
      const signed char* q, double* x, double scale, int zero_point, int n, hipStream_t s
) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(quantizedx_dequantize_i8_kernel, grid, block, 0, s,
                         q, x, scale, zero_point, n);
}

// ── fake_quant i8 (dequant of quant, stays in f64) ───────────────────────────
extern "C" __global__ void quantizedx_fake_quant_i8_kernel(
      const double* x, double* out, double scale, int zero_point,
      int qmin, int qmax, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double r = round(x[i] / scale) + (double)zero_point;
            int v = (int)r;
            if (v < qmin) v = qmin;
            if (v > qmax) v = qmax;
            out[i] = ((double)v - (double)zero_point) * scale;
      }
}
extern "C" void launch_quantizedx_fake_quant_i8(
      const double* x, double* out, double scale, int zero_point,
      int qmin, int qmax, int n, hipStream_t s
) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(quantizedx_fake_quant_i8_kernel, grid, block, 0, s,
                         x, out, scale, zero_point, qmin, qmax, n);
}

// ── int4 variants (qmin=-8, qmax=7), one value per signed-char byte ──────────
extern "C" __global__ void quantizedx_quantize_i4_kernel(
      const double* x, signed char* q, double scale, int zero_point,
      int qmin, int qmax, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double r = round(x[i] / scale) + (double)zero_point;
            int v = (int)r;
            if (v < qmin) v = qmin;
            if (v > qmax) v = qmax;
            q[i] = (signed char)v;
      }
}
extern "C" void launch_quantizedx_quantize_i4(
      const double* x, signed char* q, double scale, int zero_point,
      int qmin, int qmax, int n, hipStream_t s
) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(quantizedx_quantize_i4_kernel, grid, block, 0, s,
                         x, q, scale, zero_point, qmin, qmax, n);
}

extern "C" __global__ void quantizedx_fake_quant_i4_kernel(
      const double* x, double* out, double scale, int zero_point,
      int qmin, int qmax, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double r = round(x[i] / scale) + (double)zero_point;
            int v = (int)r;
            if (v < qmin) v = qmin;
            if (v > qmax) v = qmax;
            out[i] = ((double)v - (double)zero_point) * scale;
      }
}
extern "C" void launch_quantizedx_fake_quant_i4(
      const double* x, double* out, double scale, int zero_point,
      int qmin, int qmax, int n, hipStream_t s
) {
      dim3 block(256); dim3 grid((n + 255) / 256);
      hipLaunchKernelGGL(quantizedx_fake_quant_i4_kernel, grid, block, 0, s,
                         x, out, scale, zero_point, qmin, qmax, n);
}

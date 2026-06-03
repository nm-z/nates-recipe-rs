#include <hip/hip_runtime.h>
#include <math.h>

// ── New optimizer weight-update kernels (collision-safe optimizerx_ names) ───
// Each kernel performs ONE in-place step over per-element weights/state.

// Nesterov accelerated SGD (PyTorch form):
//   buf = momentum*buf + g
//   d   = g + momentum*buf
//   w  -= lr * d
extern "C" __global__ void optimizerx_nesterov_kernel(
      double* w, double* buf, const double* g,
      double lr, double momentum, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double bi = momentum * buf[i] + g[i];
            buf[i] = bi;
            double d = g[i] + momentum * bi;
            w[i] -= lr * d;
      }
}
extern "C" void launch_optimizerx_nesterov(
      double* w, double* buf, const double* g,
      double lr, double momentum, int n, hipStream_t stream)
{
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(optimizerx_nesterov_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, buf, g, lr, momentum, n);
}

// Adadelta (Zeiler 2012):
//   Eg = rho*Eg + (1-rho)*g^2
//   dx = -sqrt(Edx + eps)/sqrt(Eg + eps) * g
//   Edx = rho*Edx + (1-rho)*dx^2
//   w += lr * dx     (lr usually 1.0)
extern "C" __global__ void optimizerx_adadelta_kernel(
      double* w, double* eg, double* edx, const double* g,
      double lr, double rho, double eps, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double egi = rho * eg[i] + (1.0 - rho) * gi * gi;
            eg[i] = egi;
            double dx = -sqrt(edx[i] + eps) / sqrt(egi + eps) * gi;
            edx[i] = rho * edx[i] + (1.0 - rho) * dx * dx;
            w[i] += lr * dx;
      }
}
extern "C" void launch_optimizerx_adadelta(
      double* w, double* eg, double* edx, const double* g,
      double lr, double rho, double eps, int n, hipStream_t stream)
{
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(optimizerx_adadelta_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, eg, edx, g, lr, rho, eps, n);
}

// RAdam (Liu et al. 2019), rectified Adam:
//   m = b1*m + (1-b1)*g ;  v = b2*v + (1-b2)*g^2
//   m_hat = m/(1-b1^t)
//   rho_inf = 2/(1-b2) - 1
//   rho_t   = rho_inf - 2*t*b2^t/(1-b2^t)
//   if rho_t > 4:
//       l = sqrt((1-b2^t)/v)             (adaptive lr term, v not v_hat)
//       r = sqrt(((rho_t-4)*(rho_t-2)*rho_inf)/((rho_inf-4)*(rho_inf-2)*rho_t))
//       w -= lr * r * m_hat * l
//   else:
//       w -= lr * m_hat
extern "C" __global__ void optimizerx_radam_kernel(
      double* w, double* m, double* v, const double* g,
      double lr, double b1, double b2, double eps, int t, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double mi = b1 * m[i] + (1.0 - b1) * gi;
            double vi = b2 * v[i] + (1.0 - b2) * gi * gi;
            m[i] = mi; v[i] = vi;

            double b1t = pow(b1, (double)t);
            double b2t = pow(b2, (double)t);
            double m_hat = mi / (1.0 - b1t);
            double rho_inf = 2.0 / (1.0 - b2) - 1.0;
            double rho_t = rho_inf - 2.0 * (double)t * b2t / (1.0 - b2t);

            if (rho_t > 4.0) {
                  double l = sqrt((1.0 - b2t) / (vi + eps));
                  double r = sqrt(((rho_t - 4.0) * (rho_t - 2.0) * rho_inf) /
                                  ((rho_inf - 4.0) * (rho_inf - 2.0) * rho_t));
                  w[i] -= lr * r * m_hat * l;
            } else {
                  w[i] -= lr * m_hat;
            }
      }
}
extern "C" void launch_optimizerx_radam(
      double* w, double* m, double* v, const double* g,
      double lr, double b1, double b2, double eps, int t, int n, hipStream_t stream)
{
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(optimizerx_radam_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, m, v, g, lr, b1, b2, eps, t, n);
}

// LARS (You et al. 2017), Layer-wise Adaptive Rate Scaling. Single vector = one layer.
// Two-phase: phase1 accumulates ||w||^2 and ||g||^2; phase2 applies the update.
//   trust = (||w||>0 && ||g||>0) ? lambda*||w|| / (||g|| + wd*||w|| + eps) : 1.0
//   buf = momentum*buf + trust*(g + wd*w)
//   w  -= lr * buf
extern "C" __global__ void optimizerx_lars_phase1_kernel(
      const double* w, const double* g, int n,
      double* w_norm_sq, double* g_norm_sq)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double lw = 0.0, lg = 0.0;
      if (i < n) { lw = w[i] * w[i]; lg = g[i] * g[i]; }
      for (int off = warpSize / 2; off > 0; off /= 2) {
            lw += __shfl_down(lw, off);
            lg += __shfl_down(lg, off);
      }
      if ((threadIdx.x % warpSize) == 0) {
            atomicAdd(w_norm_sq, lw);
            atomicAdd(g_norm_sq, lg);
      }
}
extern "C" void launch_optimizerx_lars_phase1(
      const double* w, const double* g, int n,
      double* w_norm_sq, double* g_norm_sq, hipStream_t stream)
{
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(optimizerx_lars_phase1_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, g, n, w_norm_sq, g_norm_sq);
}

extern "C" __global__ void optimizerx_lars_phase2_kernel(
      double* w, double* buf, const double* g,
      double lr, double momentum, double wd, double lambda,
      double w_norm_sq, double g_norm_sq, double eps, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double w_norm = sqrt(w_norm_sq);
            double g_norm = sqrt(g_norm_sq);
            double trust = (w_norm > 0.0 && g_norm > 0.0)
                  ? lambda * w_norm / (g_norm + wd * w_norm + eps)
                  : 1.0;
            double bi = momentum * buf[i] + trust * (g[i] + wd * w[i]);
            buf[i] = bi;
            w[i] -= lr * bi;
      }
}
extern "C" void launch_optimizerx_lars_phase2(
      double* w, double* buf, const double* g,
      double lr, double momentum, double wd, double lambda,
      double w_norm_sq, double g_norm_sq, double eps, int n, hipStream_t stream)
{
      int threads = 256; int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(optimizerx_lars_phase2_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, buf, g, lr, momentum, wd, lambda, w_norm_sq, g_norm_sq, eps, n);
}

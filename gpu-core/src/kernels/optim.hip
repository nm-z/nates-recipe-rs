#include <hip/hip_runtime.h>
#include <math.h>

// Momentum SGD: v = momentum*v - lr*g; w += v  (in-place, updates both w and v)
extern "C" __global__ void momentum_update_kernel(
      double* w, double* v, const double* g,
      double lr, double momentum, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double vi = momentum * v[i] - lr * g[i];
            v[i] = vi;
            w[i] += vi;
      }
}

extern "C" void launch_momentum_update(
      double* w, double* v, const double* g,
      double lr, double momentum, int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(momentum_update_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, v, g, lr, momentum, n);
}

// RMSProp: cache = decay*cache + (1-decay)*g^2; w -= lr*g/(sqrt(cache)+eps)
extern "C" __global__ void rmsprop_update_kernel(
      double* w, double* cache, const double* g,
      double lr, double decay, double eps, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double ci = decay * cache[i] + (1.0 - decay) * gi * gi;
            cache[i] = ci;
            w[i] -= lr * gi / (sqrt(ci) + eps);
      }
}

extern "C" void launch_rmsprop_update(
      double* w, double* cache, const double* g,
      double lr, double decay, double eps, int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(rmsprop_update_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, cache, g, lr, decay, eps, n);
}

// Adagrad: accum += g^2; w -= lr*g/(sqrt(accum)+eps)
extern "C" __global__ void adagrad_update_kernel(
      double* w, double* accum, const double* g,
      double lr, double eps, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double ai = accum[i] + gi * gi;
            accum[i] = ai;
            w[i] -= lr * gi / (sqrt(ai) + eps);
      }
}

extern "C" void launch_adagrad_update(
      double* w, double* accum, const double* g,
      double lr, double eps, int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(adagrad_update_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, accum, g, lr, eps, n);
}

// LAMB phase 1: compute Adam moments, write per-element update to tmp_upd,
// and accumulate ||w||^2 and ||update||^2 into single-element buffers via atomicAdd.
extern "C" __global__ void lamb_phase1_kernel(
      double* w, double* m, double* v, const double* g,
      double b1, double b2, double eps, double wd, int t, int n,
      double* tmp_upd, double* w_norm_sq, double* u_norm_sq)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      double local_w2 = 0.0;
      double local_u2 = 0.0;

      if (i < n) {
            double gi = g[i];
            double mi = b1 * m[i] + (1.0 - b1) * gi;
            double vi = b2 * v[i] + (1.0 - b2) * gi * gi;
            m[i] = mi;
            v[i] = vi;

            double bc1 = 1.0 - pow(b1, (double)t);
            double bc2 = 1.0 - pow(b2, (double)t);
            double m_hat = mi / bc1;
            double v_hat = vi / bc2;
            double upd = m_hat / (sqrt(v_hat) + eps) + wd * w[i];
            tmp_upd[i] = upd;

            local_w2 = w[i] * w[i];
            local_u2 = upd * upd;
      }

      for (int offset = warpSize / 2; offset > 0; offset /= 2) {
            local_w2 += __shfl_down(local_w2, offset);
            local_u2 += __shfl_down(local_u2, offset);
      }
      if ((threadIdx.x % warpSize) == 0) {
            atomicAdd(w_norm_sq, local_w2);
            atomicAdd(u_norm_sq, local_u2);
      }
}

// LAMB phase 2: w -= lr * trust_ratio * tmp_upd
// trust_ratio = ||w|| / ||update||, clamped to 1.0 if either norm is 0.
extern "C" __global__ void lamb_phase2_kernel(
      double* w, const double* tmp_upd,
      double lr, double w_norm_sq, double u_norm_sq, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double w_norm = sqrt(w_norm_sq);
            double u_norm = sqrt(u_norm_sq);
            double ratio = (w_norm == 0.0 || u_norm == 0.0) ? 1.0 : w_norm / u_norm;
            w[i] -= lr * ratio * tmp_upd[i];
      }
}

extern "C" void launch_lamb_phase1(
      double* w, double* m, double* v, const double* g,
      double b1, double b2, double eps, double wd, int t, int n,
      double* tmp_upd, double* w_norm_sq, double* u_norm_sq,
      hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(lamb_phase1_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, m, v, g, b1, b2, eps, wd, t, n, tmp_upd, w_norm_sq, u_norm_sq);
}

extern "C" void launch_lamb_phase2(
      double* w, const double* tmp_upd,
      double lr, double w_norm_sq, double u_norm_sq,
      int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(lamb_phase2_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, tmp_upd, lr, w_norm_sq, u_norm_sq, n);
}

// Lion: update = sign(b1*m + (1-b1)*g); w -= lr*(update + wd*w); m = b2*m + (1-b2)*g
extern "C" __global__ void lion_update_kernel(
      double* w, double* m, const double* g,
      double lr, double b1, double b2, double wd, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double mi = m[i];
            double interp = b1 * mi + (1.0 - b1) * gi;
            double upd = (interp > 0.0) ? 1.0 : ((interp < 0.0) ? -1.0 : 0.0);
            w[i] -= lr * (upd + wd * w[i]);
            m[i] = b2 * mi + (1.0 - b2) * gi;
      }
}

extern "C" void launch_lion_update(
      double* w, double* m, const double* g,
      double lr, double b1, double b2, double wd, int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(lion_update_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, m, g, lr, b1, b2, wd, n);
}

// Nadam: Nesterov-accelerated Adam
// m = b1*m + (1-b1)*g
// v = b2*v + (1-b2)*g^2
// m_hat_nesterov = b1*m/(1-b1^(t+1)) + (1-b1)*g/(1-b1^t)
// v_hat = v / (1 - b2^t)
// w -= lr * m_hat_nesterov / (sqrt(v_hat) + eps)
extern "C" __global__ void nadam_update_kernel(
      double* w, double* m, double* v, const double* g,
      double lr, double b1, double b2, double eps, int t, int n)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double gi = g[i];
            double mi = b1 * m[i] + (1.0 - b1) * gi;
            double vi = b2 * v[i] + (1.0 - b2) * gi * gi;
            m[i] = mi;
            v[i] = vi;

            double bc1_t  = 1.0 - pow(b1, (double)t);
            double bc1_t1 = 1.0 - pow(b1, (double)(t + 1));
            double bc2    = 1.0 - pow(b2, (double)t);

            double m_hat = b1 * mi / bc1_t1 + (1.0 - b1) * gi / bc1_t;
            double v_hat = vi / bc2;
            w[i] -= lr * m_hat / (sqrt(v_hat) + eps);
      }
}

extern "C" void launch_nadam_update(
      double* w, double* m, double* v, const double* g,
      double lr, double b1, double b2, double eps, int t, int n, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(nadam_update_kernel, dim3(blocks), dim3(threads), 0, stream,
            w, m, v, g, lr, b1, b2, eps, t, n);
}

// Clip value in-place: x[i] = clamp(x[i], lo, hi)
extern "C" __global__ void clip_value_kernel(double* x, double lo, double hi, int n) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < n) {
            double v = x[i];
            if (v < lo) v = lo;
            if (v > hi) v = hi;
            x[i] = v;
      }
}

extern "C" void launch_clip_value(double* x, double lo, double hi, int n, hipStream_t stream) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(clip_value_kernel, dim3(blocks), dim3(threads), 0, stream,
            x, lo, hi, n);
}

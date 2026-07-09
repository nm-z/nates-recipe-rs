#include <hip/hip_runtime.h>
#include <math.h>

__global__ void discounted_returns_kernel(
      const double* rewards, double* returns, const double* gamma_p, int t_len
) {
      double gamma = *gamma_p;
      if (threadIdx.x != 0 || blockIdx.x != 0) return;
      double g = 0.0;
      for (int t = t_len - 1; t >= 0; t--) {
            g = rewards[t] + gamma * g;
            returns[t] = g;
      }
}

extern "C" void launch_discounted_returns(
      const double* rewards, double* returns, const double* gamma, int t_len, hipStream_t stream
) {
      hipLaunchKernelGGL(discounted_returns_kernel, dim3(1), dim3(1), 0, stream,
            rewards, returns, gamma, t_len);
}

__global__ void gae_kernel(
      const double* rewards, const double* values,
      double* advantages, const double* gamma_p, const double* lam_p, int t_len
) {
      double gamma = *gamma_p;
      double lam   = *lam_p;
      if (threadIdx.x != 0 || blockIdx.x != 0) return;
      double a = 0.0;
      for (int t = t_len - 1; t >= 0; t--) {
            double v_next = (t + 1 < t_len) ? values[t + 1] : 0.0;
            double delta  = rewards[t] + gamma * v_next - values[t];
            a = delta + gamma * lam * a;
            advantages[t] = a;
      }
}

extern "C" void launch_gae(
      const double* rewards, const double* values,
      double* advantages, const double* gamma, const double* lam, int t_len, hipStream_t stream
) {
      hipLaunchKernelGGL(gae_kernel, dim3(1), dim3(1), 0, stream,
            rewards, values, advantages, gamma, lam, t_len);
}

__global__ void td_targets_kernel(
      const double* rewards, const double* values_next, const double* done_mask,
      double* targets, const double* gamma_p, int n
) {
      double gamma = *gamma_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      targets[i] = rewards[i] + gamma * values_next[i] * (1.0 - done_mask[i]);
}

extern "C" void launch_td_targets(
      const double* rewards, const double* values_next, const double* done_mask,
      double* targets, const double* gamma, int n, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(td_targets_kernel, dim3(blocks), dim3(threads), 0, stream,
            rewards, values_next, done_mask, targets, gamma, n);
}

__global__ void categorical_logprob_kernel(
      const double* logits, const int* actions,
      double* logp, int n, int n_actions
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      const double* row = logits + i * n_actions;

      double max_val = row[0];
      for (int k = 1; k < n_actions; k++) {
            if (row[k] > max_val) max_val = row[k];
      }
      double sum_exp = 0.0;
      for (int k = 0; k < n_actions; k++) {
            sum_exp += exp(row[k] - max_val);
      }
      double log_sum = max_val + log(sum_exp);
      int a = actions[i];
      logp[i] = row[a] - log_sum;
}

extern "C" void launch_categorical_logprob(
      const double* logits, const int* actions,
      double* logp, int n, int n_actions, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(categorical_logprob_kernel, dim3(blocks), dim3(threads), 0, stream,
            logits, actions, logp, n, n_actions);
}

__global__ void gaussian_logprob_kernel(
      const double* mu, const double* log_std, const double* actions,
      double* logp, int n, int dim
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      const double* mu_row  = mu       + i * dim;
      const double* ls_row  = log_std  + i * dim;
      const double* a_row   = actions  + i * dim;
      const double half_log2pi = 0.9189385332046727;
      double acc = 0.0;
      for (int d = 0; d < dim; d++) {
            double sigma = exp(ls_row[d]);
            double z     = (a_row[d] - mu_row[d]) / sigma;
            acc += -0.5 * z * z - ls_row[d] - half_log2pi;
      }
      logp[i] = acc;
}

extern "C" void launch_gaussian_logprob(
      const double* mu, const double* log_std, const double* actions,
      double* logp, int n, int dim, hipStream_t stream
) {
      int threads = 256;
      int blocks  = (n + threads - 1) / threads;
      hipLaunchKernelGGL(gaussian_logprob_kernel, dim3(blocks), dim3(threads), 0, stream,
            mu, log_std, actions, logp, n, dim);
}

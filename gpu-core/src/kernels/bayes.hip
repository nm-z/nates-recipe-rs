#include <hip/hip_runtime.h>


extern "C" __global__ void nb_count_table_kernel(
      const double* x_counts, const int* y, double* out,
      int n, int n_features, int n_classes)
{
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      if (idx >= n * n_features) return;
      int i = idx / n_features;
      int f = idx % n_features;
      int cls = y[i];
      if (cls < 0 || cls >= n_classes) return;
      atomicAdd(&out[cls * n_features + f], x_counts[i * n_features + f]);
}

extern "C" void launch_nb_count_table(
      const double* x_counts, const int* y, double* out,
      int n, int n_features, int n_classes, hipStream_t stream)
{
      int total = n * n_features;
      int threads = 256;
      int blocks = (total + threads - 1) / threads;
      hipLaunchKernelGGL(nb_count_table_kernel, dim3(blocks), dim3(threads), 0, stream,
            x_counts, y, out, n, n_features, n_classes);
}


extern "C" __global__ void nb_feature_log_prob_kernel(
      const double* count_table, double* out,
      int n_classes, int n_features, const double* alpha_p)
{
      double alpha = *alpha_p;
      int cls = blockIdx.x * blockDim.x + threadIdx.x;
      if (cls >= n_classes) return;
      double row_sum = 0.0;
      for (int f = 0; f < n_features; f++) {
            row_sum += count_table[cls * n_features + f] + alpha;
      }
      double log_denom = log(row_sum);
      for (int f = 0; f < n_features; f++) {
            out[cls * n_features + f] = log(count_table[cls * n_features + f] + alpha) - log_denom;
      }
}

extern "C" void launch_nb_feature_log_prob(
      const double* count_table, double* out,
      int n_classes, int n_features, const double* alpha, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n_classes + threads - 1) / threads;
      hipLaunchKernelGGL(nb_feature_log_prob_kernel, dim3(blocks), dim3(threads), 0, stream,
            count_table, out, n_classes, n_features, alpha);
}


extern "C" __global__ void multinomial_nb_logprob_kernel(
      const double* log_class_prior, const double* feature_log_prob,
      const double* x, double* out,
      int n, int n_features, int n_classes)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      int c = blockIdx.y * blockDim.y + threadIdx.y;
      if (i >= n || c >= n_classes) return;
      double acc = log_class_prior[c];
      for (int f = 0; f < n_features; f++) {
            acc += x[i * n_features + f] * feature_log_prob[c * n_features + f];
      }
      out[i * n_classes + c] = acc;
}

extern "C" void launch_multinomial_nb_logprob(
      const double* log_class_prior, const double* feature_log_prob,
      const double* x, double* out,
      int n, int n_features, int n_classes, hipStream_t stream)
{
      dim3 threads(16, 16);
      dim3 blocks((n + 15) / 16, (n_classes + 15) / 16);
      hipLaunchKernelGGL(multinomial_nb_logprob_kernel, blocks, threads, 0, stream,
            log_class_prior, feature_log_prob, x, out, n, n_features, n_classes);
}


extern "C" __global__ void bernoulli_nb_logprob_kernel(
      const double* log_class_prior,
      const double* feature_log_prob, const double* feature_log_neg,
      const double* x_binary, double* out,
      int n, int n_features, int n_classes)
{
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      int c = blockIdx.y * blockDim.y + threadIdx.y;
      if (i >= n || c >= n_classes) return;
      double acc = log_class_prior[c];
      for (int f = 0; f < n_features; f++) {
            double xv = x_binary[i * n_features + f];
            acc += xv * feature_log_prob[c * n_features + f]
                 + (1.0 - xv) * feature_log_neg[c * n_features + f];
      }
      out[i * n_classes + c] = acc;
}

extern "C" void launch_bernoulli_nb_logprob(
      const double* log_class_prior,
      const double* feature_log_prob, const double* feature_log_neg,
      const double* x_binary, double* out,
      int n, int n_features, int n_classes, hipStream_t stream)
{
      dim3 threads(16, 16);
      dim3 blocks((n + 15) / 16, (n_classes + 15) / 16);
      hipLaunchKernelGGL(bernoulli_nb_logprob_kernel, blocks, threads, 0, stream,
            log_class_prior, feature_log_prob, feature_log_neg, x_binary, out,
            n, n_features, n_classes);
}

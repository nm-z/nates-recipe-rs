#include <hip/hip_runtime.h>

// ── Multinomial NB count table ─────────────────────────────────────────────
// x_counts: [n * n_features] feature counts (f64)
// y:        [n] class labels (i32)
// out:      [n_classes * n_features] count table, zeroed by caller
// Each thread owns one (sample, feature) pair and atomically adds x_counts[i,f]
// to out[class, f].  HIP provides atomicAdd for double via __longlong_as_double.

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

// ── Feature log-prob (Laplace smoothing) ──────────────────────────────────
// count_table: [n_classes * n_features]
// out:         [n_classes * n_features]  log P(feature|class)
// Each row (class) is smoothed with alpha and normalised.

extern "C" __global__ void nb_feature_log_prob_kernel(
      const double* count_table, double* out,
      int n_classes, int n_features, double alpha)
{
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
      int n_classes, int n_features, double alpha, hipStream_t stream)
{
      int threads = 256;
      int blocks = (n_classes + threads - 1) / threads;
      hipLaunchKernelGGL(nb_feature_log_prob_kernel, dim3(blocks), dim3(threads), 0, stream,
            count_table, out, n_classes, n_features, alpha);
}

// ── Multinomial NB log-posterior ──────────────────────────────────────────
// log_class_prior:  [n_classes]
// feature_log_prob: [n_classes * n_features]  (row = class)
// x:                [n * n_features]
// out:              [n * n_classes]  log P(class|x) (unnormalised)
// out[i, c] = log_prior[c] + sum_f x[i,f] * feature_log_prob[c,f]

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

// ── Bernoulli NB log-posterior ────────────────────────────────────────────
// log_class_prior:  [n_classes]
// feature_log_prob: [n_classes * n_features]  log P(f=1|c)
// feature_log_neg:  [n_classes * n_features]  log P(f=0|c) = log(1 - P(f=1|c))
// x_binary:         [n * n_features]  (0.0 or 1.0)
// out:              [n * n_classes]
// out[i,c] = log_prior[c] + sum_f [ x[i,f]*log_p[c,f] + (1-x[i,f])*log_neg[c,f] ]

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

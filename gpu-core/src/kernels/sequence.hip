#include <hip/hip_runtime.h>
#include <math.h>

// ── log-space logsumexp helpers ────────────────────────────────────────────

// logsumexp over n_states values stored in smem; result written to smem[0].
// Call with exactly n_states active threads in one block.
// Each thread contributes smem[tid] before calling this.
// After the call only thread 0 holds the valid result.
__device__ double block_logsumexp(double* smem, int n_states, int tid) {
      // find max
      __syncthreads();
      for (int stride = n_states / 2; stride >= 1; stride >>= 1) {
            if (tid < stride && tid + stride < n_states) {
                  if (smem[tid + stride] > smem[tid]) smem[tid] = smem[tid + stride];
            }
            __syncthreads();
      }
      double mx = smem[0];
      __syncthreads();

      // subtract max, exp, write back
      if (tid < n_states) {
            smem[tid] = exp(smem[tid] - mx);  // smem was clobbered; caller must restore before
      }
      __syncthreads();

      // sum
      for (int stride = n_states / 2; stride >= 1; stride >>= 1) {
            if (tid < stride && tid + stride < n_states) {
                  smem[tid] += smem[tid + stride];
            }
            __syncthreads();
      }
      // result: log(sum) + mx in smem[0] (thread 0 computes final)
      if (tid == 0) smem[0] = log(smem[0]) + mx;
      __syncthreads();
      return smem[0];
}

// ── Forward pass kernel ────────────────────────────────────────────────────
// One block per time step t (launched sequentially t=0..T-1 from host loop).
// blockDim.x = n_states (must be power-of-2 or <= 1024).
// log_trans [S*S]: log_trans[s*S + s2] = log P(s2 | s).
// log_emit  [T*S]: log_emit[t*S + s]   = log P(obs_t | s).
// log_alpha [T*S]: output (row-major).
// t: current time step.
__global__ void forward_step_kernel(
      const double* __restrict__ log_trans,
      const double* __restrict__ log_emit,
      double* __restrict__ log_alpha,
      int n_states, int t
) {
      int s2 = threadIdx.x;  // destination state
      if (s2 >= n_states) return;

      extern __shared__ double smem[];

      double val;
      if (t == 0) {
            // uniform prior in log space: log(1/S)
            val = log(1.0 / (double)n_states) + log_emit[s2];
      } else {
            // logsumexp_s1 { log_alpha[t-1, s1] + log_trans[s1*S + s2] }
            // Each thread computes its own logsumexp in registers — no smem needed.
            double mx = -1e300;
            for (int s1 = 0; s1 < n_states; s1++) {
                  double v = log_alpha[(t - 1) * n_states + s1] + log_trans[s1 * n_states + s2];
                  if (v > mx) mx = v;
            }
            double sum = 0.0;
            for (int s1 = 0; s1 < n_states; s1++) {
                  double v = log_alpha[(t - 1) * n_states + s1] + log_trans[s1 * n_states + s2];
                  sum += exp(v - mx);
            }
            val = log(sum) + mx + log_emit[t * n_states + s2];
      }
      log_alpha[t * n_states + s2] = val;
}

// ── Backward pass kernel ───────────────────────────────────────────────────
// log_beta [T*S]: output.
// t: current time step (launched T-1 down to 0 from host loop).
__global__ void backward_step_kernel(
      const double* __restrict__ log_trans,
      const double* __restrict__ log_emit,
      double* __restrict__ log_beta,
      int n_states, int t_len, int t
) {
      int s1 = threadIdx.x;  // source state
      if (s1 >= n_states) return;

      extern __shared__ double smem[];

      if (t == t_len - 1) {
            log_beta[t * n_states + s1] = 0.0;  // log(1)
      } else {
            // logsumexp_s2 { log_trans[s1*S + s2] + log_emit[t+1, s2] + log_beta[t+1, s2] }
            double mx = -1e300;
            for (int s2 = 0; s2 < n_states; s2++) {
                  double v = log_trans[s1 * n_states + s2]
                           + log_emit[(t + 1) * n_states + s2]
                           + log_beta[(t + 1) * n_states + s2];
                  if (v > mx) mx = v;
            }
            double sum = 0.0;
            for (int s2 = 0; s2 < n_states; s2++) {
                  double v = log_trans[s1 * n_states + s2]
                           + log_emit[(t + 1) * n_states + s2]
                           + log_beta[(t + 1) * n_states + s2];
                  sum += exp(v - mx);
            }
            log_beta[t * n_states + s1] = log(sum) + mx;
      }
}

// ── Gamma (posterior) kernel ───────────────────────────────────────────────
// log_gamma[t*S+s] = log_alpha[t*S+s] + log_beta[t*S+s] - log_norm_t
// Launched with (T*S) threads total.
__global__ void gamma_kernel(
      const double* __restrict__ log_alpha,
      const double* __restrict__ log_beta,
      double* __restrict__ log_gamma,
      int n_states, int t_len
) {
      int idx = blockIdx.x * blockDim.x + threadIdx.x;
      int total = t_len * n_states;
      if (idx >= total) return;

      int t = idx / n_states;
      int s = idx % n_states;

      // normalization: logsumexp over states at this t
      // Each thread independently computes norm for its t (redundant but simple)
      double mx = -1e300;
      for (int i = 0; i < n_states; i++) {
            double v = log_alpha[t * n_states + i] + log_beta[t * n_states + i];
            if (v > mx) mx = v;
      }
      double sum = 0.0;
      for (int i = 0; i < n_states; i++) {
            double v = log_alpha[t * n_states + i] + log_beta[t * n_states + i];
            sum += exp(v - mx);
      }
      double log_norm = log(sum) + mx;

      log_gamma[idx] = log_alpha[t * n_states + s] + log_beta[t * n_states + s] - log_norm;
}

// ── Viterbi forward kernel ─────────────────────────────────────────────────
// viterbi_delta [T*S]: max log-prob paths; backptr [T*S]: argmax state.
// t: current time step.
__global__ void viterbi_step_kernel(
      const double* __restrict__ log_trans,
      const double* __restrict__ log_emit,
      double* __restrict__ delta,
      int* __restrict__ backptr,
      int n_states, int t
) {
      int s2 = threadIdx.x;
      if (s2 >= n_states) return;

      if (t == 0) {
            delta[s2] = log(1.0 / (double)n_states) + log_emit[s2];
            backptr[s2] = -1;
      } else {
            double best_val = -1e300;
            int best_s = 0;
            for (int s1 = 0; s1 < n_states; s1++) {
                  double v = delta[(t - 1) * n_states + s1] + log_trans[s1 * n_states + s2];
                  if (v > best_val) { best_val = v; best_s = s1; }
            }
            delta[t * n_states + s2] = best_val + log_emit[t * n_states + s2];
            backptr[t * n_states + s2] = best_s;
      }
}

// ── Viterbi backtrace kernel ───────────────────────────────────────────────
// Single-thread kernel: walks backptr from best final state.
__global__ void viterbi_backtrace_kernel(
      const double* __restrict__ delta,
      const int* __restrict__ backptr,
      int* __restrict__ best_path,
      int n_states, int t_len
) {
      // find argmax of delta at t = t_len-1
      int best = 0;
      double best_val = delta[(t_len - 1) * n_states + 0];
      for (int s = 1; s < n_states; s++) {
            double v = delta[(t_len - 1) * n_states + s];
            if (v > best_val) { best_val = v; best = s; }
      }
      best_path[t_len - 1] = best;
      for (int t = t_len - 2; t >= 0; t--) {
            best = backptr[(t + 1) * n_states + best];
            best_path[t] = best;
      }
}

// ── Host launchers ─────────────────────────────────────────────────────────

extern "C" void launch_forward_backward(
      const double* log_trans,
      const double* log_emit,
      double* log_alpha,
      double* log_beta,
      double* log_gamma,
      int n_states, int t_len,
      hipStream_t stream
) {
      // Shared memory per block: n_states doubles (for logsumexp scratch)
      size_t smem = (size_t)n_states * sizeof(double);

      // Forward pass: sequential over t
      int threads = n_states;
      for (int t = 0; t < t_len; t++) {
            hipLaunchKernelGGL(forward_step_kernel, dim3(1), dim3(threads), smem, stream,
                  log_trans, log_emit, log_alpha, n_states, t);
            hipStreamSynchronize(stream);
      }

      // Backward pass: sequential over t (reverse)
      for (int t = t_len - 1; t >= 0; t--) {
            hipLaunchKernelGGL(backward_step_kernel, dim3(1), dim3(threads), smem, stream,
                  log_trans, log_emit, log_beta, n_states, t_len, t);
            hipStreamSynchronize(stream);
      }

      // Gamma: parallelized over all T*S
      int total = t_len * n_states;
      int blk = 256;
      int grid = (total + blk - 1) / blk;
      hipLaunchKernelGGL(gamma_kernel, dim3(grid), dim3(blk), 0, stream,
            log_alpha, log_beta, log_gamma, n_states, t_len);
}

extern "C" void launch_viterbi(
      const double* log_trans,
      const double* log_emit,
      double* delta,
      int* backptr,
      int* best_path,
      int n_states, int t_len,
      hipStream_t stream
) {
      int threads = n_states;
      for (int t = 0; t < t_len; t++) {
            hipLaunchKernelGGL(viterbi_step_kernel, dim3(1), dim3(threads), 0, stream,
                  log_trans, log_emit, delta, backptr, n_states, t);
            hipStreamSynchronize(stream);
      }
      // Backtrace: single thread
      hipLaunchKernelGGL(viterbi_backtrace_kernel, dim3(1), dim3(1), 0, stream,
            delta, backptr, best_path, n_states, t_len);
}

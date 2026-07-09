#include <hip/hip_runtime.h>


extern "C" __global__ void distancex_manhattan_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < nq * nt) {
            int q = i / nt, t = i % nt;
            const double* qp = query + (long)q * dim;
            const double* tp = train + (long)t * dim;
            double sum = 0.0;
            for (int d = 0; d < dim; d++) {
                  double diff = qp[d] - tp[d];
                  sum += fabs(diff);
            }
            out[i] = sum;
      }
}
extern "C" void launch_distancex_manhattan(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t s
) {
      int n = nq * nt;
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(distancex_manhattan_kernel, g, b, 0, s, query, train, out, nq, nt, dim);
}

extern "C" __global__ void distancex_chebyshev_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < nq * nt) {
            int q = i / nt, t = i % nt;
            const double* qp = query + (long)q * dim;
            const double* tp = train + (long)t * dim;
            double mx = 0.0;
            for (int d = 0; d < dim; d++) {
                  double a = fabs(qp[d] - tp[d]);
                  if (a > mx) mx = a;
            }
            out[i] = mx;
      }
}
extern "C" void launch_distancex_chebyshev(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t s
) {
      int n = nq * nt;
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(distancex_chebyshev_kernel, g, b, 0, s, query, train, out, nq, nt, dim);
}

extern "C" __global__ void distancex_minkowski_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, double p
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < nq * nt) {
            int q = i / nt, t = i % nt;
            const double* qp = query + (long)q * dim;
            const double* tp = train + (long)t * dim;
            double sum = 0.0;
            for (int d = 0; d < dim; d++) {
                  double a = fabs(qp[d] - tp[d]);
                  sum += pow(a, p);
            }
            out[i] = pow(sum, 1.0 / p);
      }
}
extern "C" void launch_distancex_minkowski(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, double p, hipStream_t s
) {
      int n = nq * nt;
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(distancex_minkowski_kernel, g, b, 0, s, query, train, out, nq, nt, dim, p);
}

extern "C" __global__ void distancex_braycurtis_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < nq * nt) {
            int q = i / nt, t = i % nt;
            const double* qp = query + (long)q * dim;
            const double* tp = train + (long)t * dim;
            double num = 0.0, den = 0.0;
            for (int d = 0; d < dim; d++) {
                  num += fabs(qp[d] - tp[d]);
                  den += fabs(qp[d] + tp[d]);
            }
            out[i] = (den > 0.0) ? num / den : 0.0;
      }
}
extern "C" void launch_distancex_braycurtis(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t s
) {
      int n = nq * nt;
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(distancex_braycurtis_kernel, g, b, 0, s, query, train, out, nq, nt, dim);
}

extern "C" __global__ void distancex_canberra_kernel(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i < nq * nt) {
            int q = i / nt, t = i % nt;
            const double* qp = query + (long)q * dim;
            const double* tp = train + (long)t * dim;
            double sum = 0.0;
            for (int d = 0; d < dim; d++) {
                  double den = fabs(qp[d]) + fabs(tp[d]);
                  if (den > 0.0) sum += fabs(qp[d] - tp[d]) / den;
            }
            out[i] = sum;
      }
}
extern "C" void launch_distancex_canberra(
      const double* query, const double* train, double* out,
      int nq, int nt, int dim, hipStream_t s
) {
      int n = nq * nt;
      dim3 b(256); dim3 g((n + 255) / 256);
      hipLaunchKernelGGL(distancex_canberra_kernel, g, b, 0, s, query, train, out, nq, nt, dim);
}

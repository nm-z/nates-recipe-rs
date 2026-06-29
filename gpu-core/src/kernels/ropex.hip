#include <hip/hip_runtime.h>

// ── ropex_: NeoX half-split RoPE, EXPLICIT per-row positions, applied to Q & K ─
//
// q,k:(n_rows,d) row-major f64. pos:(n_rows) f64, cast to int per row.
// theta: rotation base (e.g. 10000.0). Pair element i in [0,d/2) with i+d/2;
// angle = pos / theta^(2i/d):
//   out[i]      = x[i]*cos(angle) - x[i+d/2]*sin(angle)
//   out[i+d/2]  = x[i+d/2]*cos(angle) + x[i]*sin(angle)
// Identical rotation applied to both Q and K. Fresh outputs qo, ko.
extern "C" __global__ void ropex_qk_kernel(
    const double* q, const double* k, const double* pos,
    double* qo, double* ko, int n_rows, int d, double theta
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int half = d / 2;
    if (idx >= n_rows * half) return;
    int row = idx / half;          // position row
    int i = idx % half;            // pair index
    double p = (double)(int)pos[row];
    double angle = p / pow(theta, (2.0 * (double)i) / (double)d);
    double c = cos(angle);
    double s = sin(angle);
    long base = (long)row * d;
    double qa = q[base + i];
    double qb = q[base + i + half];
    qo[base + i]        = qa * c - qb * s;
    qo[base + i + half] = qb * c + qa * s;
    double ka = k[base + i];
    double kb = k[base + i + half];
    ko[base + i]        = ka * c - kb * s;
    ko[base + i + half] = kb * c + ka * s;
}
extern "C" void launch_ropex_qk(
    const double* q, const double* k, const double* pos,
    double* qo, double* ko, int n_rows, int d, double theta, hipStream_t s
) {
    int total = n_rows * (d / 2);
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(ropex_qk_kernel, grid, block, 0, s, q, k, pos, qo, ko, n_rows, d, theta);
}

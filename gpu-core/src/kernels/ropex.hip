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

// ── Multi-head RoPE, IN-PLACE, implicit position = token's sequence index ──────
//
// q,k:(m,d) row-major f64, d = heads*hd. Rotate WITHIN each head block [h*hd,
// h*hd+hd): pair (j, j+hd/2) by angle = (row % seq) / theta^(2j/hd). Position is
// the token's index within its sequence (row r = sample*seq + pos → pos = r%seq).
// `sgn` selects direction: +1 forward (Q_rot = R(angle)·Q), -1 backward — a
// rotation is orthogonal so dQ = R(-angle)·dQ_rot, the SAME kernel with sgn=-1.
extern "C" __global__ void ropex_qk_heads_kernel(
    double* q, double* k, int m, int d, int heads, int seq, double theta, double sgn
) {
    int hd = d / heads;
    int half = hd / 2;
    int total = m * heads * half;
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= total) return;
    int j = idx % half;            // pair index within the head
    int t = idx / half;
    int h = t % heads;             // head
    int row = t / heads;           // token row
    int pos = row % seq;           // sequence position
    double angle = (double)pos / pow(theta, (2.0 * (double)j) / (double)hd);
    double c = cos(angle);
    double s = sgn * sin(angle);
    long base = (long)row * d + (long)h * hd;
    double qa = q[base + j], qb = q[base + j + half];
    q[base + j]        = qa * c - qb * s;
    q[base + j + half] = qb * c + qa * s;
    double ka = k[base + j], kb = k[base + j + half];
    k[base + j]        = ka * c - kb * s;
    k[base + j + half] = kb * c + ka * s;
}
extern "C" void launch_ropex_qk_heads(
    double* q, double* k, int m, int d, int heads, int seq, double theta, double sgn, hipStream_t st
) {
    int half = (d / heads) / 2;
    if (half == 0) return;
    int total = m * heads * half;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(ropex_qk_heads_kernel, grid, block, 0, st, q, k, m, d, heads, seq, theta, sgn);
}

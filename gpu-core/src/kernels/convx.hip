#include <hip/hip_runtime.h>

// ── Direct convolution kernels (cross-correlation convention, NO kernel flip) ──
// Matches PyTorch / TensorFlow / Keras / cuDNN / MIOpen forward convolution.
// All "valid" padding (out = in - k + 1). f64 throughout. Unique convx_ names.

// conv1d: x (N, Cin, L)  w (Cout, Cin, K)  bias (Cout, may be null) → y (N, Cout, Lout)
// Lout = (L - K) / S + 1. Cross-correlation with stride S.
extern "C" __global__ void convx_conv1d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int L, int Cout, int K, int Lout, int S
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cout * Lout;
    if (idx >= total) return;
    int t  = idx % Lout;
    int r  = idx / Lout;
    int co = r % Cout;
    int n  = r / Cout;
    double acc = bias ? bias[co] : 0.0;
    for (int ci = 0; ci < Cin; ++ci) {
        const double* xp = x + ((n * Cin + ci) * L);
        const double* wp = w + ((co * Cin + ci) * K);
        for (int k = 0; k < K; ++k) acc += xp[t * S + k] * wp[k];
    }
    y[idx] = acc;
}
extern "C" void launch_convx_conv1d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int L, int Cout, int K, int Lout, int S, hipStream_t s
) {
    int total = N * Cout * Lout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_conv1d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, L, Cout, K, Lout, S);
}

// conv1d backward: dL/dX. dy (N, Cout, Lout), w (Cout, Cin, K) → dx (N, Cin, L).
// dx[n,ci,l] = sum_co sum_k w[co,ci,k] * dy[n,co,t] where t = (l-k)/S, if (l-k)%S==0 and 0<=t<Lout.
extern "C" __global__ void convx_conv1d_backward_data_kernel(
    const double* dy, const double* w, double* dx,
    int N, int Cin, int L, int Cout, int K, int Lout, int S
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cin * L;
    if (idx >= total) return;
    int l  = idx % L;
    int r  = idx / L;
    int ci = r % Cin;
    int n  = r / Cin;
    double acc = 0.0;
    for (int co = 0; co < Cout; ++co) {
        const double* dyp = dy + ((n * Cout + co) * Lout);
        const double* wp  = w + ((co * Cin + ci) * K);
        for (int k = 0; k < K; ++k) {
            int lk = l - k;
            if (lk >= 0 && lk % S == 0) {
                int t = lk / S;
                if (t < Lout) acc += wp[k] * dyp[t];
            }
        }
    }
    dx[idx] = acc;
}
extern "C" void launch_convx_conv1d_backward_data(
    const double* dy, const double* w, double* dx,
    int N, int Cin, int L, int Cout, int K, int Lout, int S, hipStream_t s
) {
    int total = N * Cin * L;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_conv1d_backward_data_kernel, grid, block, 0, s, dy, w, dx, N, Cin, L, Cout, K, Lout, S);
}

// One workgroup per (filter_weight, chunk). Each thread accumulates in a register
// over its slice of N*Lout, warp shuffle reduces, one write to global. Zero atomics.
// Rust-side reduce_sum_cols on temp[fsz*chunks] → dw[fsz].
extern "C" __global__ void convx_conv1d_backward_filter_kernel(
    const double* dy, const double* x, double* out,
    int N, int Cin, int L, int Cout, int K, int Lout, int S, int chunks
) {
    int fsz = Cout * Cin * K;
    int filter_idx = blockIdx.x / chunks;
    int chunk_id   = blockIdx.x % chunks;
    if (filter_idx >= fsz) return;
    int cik = Cin * K;
    int co = filter_idx / cik;
    int ci = (filter_idx % cik) / K;
    int kk = filter_idx % K;
    int nl = N * Lout;
    int chunk_size = (nl + chunks - 1) / chunks;
    int start = chunk_id * chunk_size;
    int end   = start + chunk_size;
    if (end > nl) end = nl;
    __shared__ double smem[256];
    double sum = 0.0;
    for (int nt = start + threadIdx.x; nt < end; nt += blockDim.x) {
        int t = nt % Lout;
        int n = nt / Lout;
        sum += dy[n * Cout * Lout + co * Lout + t]
             * x[(n * Cin + ci) * L + t * S + kk];
    }
    smem[threadIdx.x] = sum;
    __syncthreads();
    for (int s = 128; s > 0; s >>= 1) {
        if (threadIdx.x < s) smem[threadIdx.x] += smem[threadIdx.x + s];
        __syncthreads();
    }
    if (threadIdx.x == 0) out[chunk_id * fsz + filter_idx] = smem[0];
}
extern "C" void launch_convx_conv1d_backward_filter(
    const double* dy, const double* x, double* temp,
    int N, int Cin, int L, int Cout, int K, int Lout, int S, int chunks, hipStream_t s
) {
    int fsz = Cout * Cin * K;
    int grid = fsz * chunks;
    hipLaunchKernelGGL(convx_conv1d_backward_filter_kernel, dim3(grid), dim3(256),
        0, s, dy, x, temp, N, Cin, L, Cout, K, Lout, S, chunks);
}

// Single-pass backward_bias. Each block handles one (co, chunk) pair. Tree reduction
// in smem, one atomicAdd to db[co]. Grid = Cout * blocks_per_co, data-driven.
extern "C" __global__ void convx_conv1d_backward_bias_kernel(
    const double* dy, double* db, int N, int Cout, int Lout
) {
    __shared__ double smem[256];
    int blocks_per_co = gridDim.x / Cout;
    int co = blockIdx.x % Cout;
    int chunk = blockIdx.x / Cout;
    int tid = threadIdx.x;
    int nl = N * Lout;
    double acc = 0.0;
    for (int nt = chunk * 256 + tid; nt < nl; nt += blocks_per_co * 256) {
        int t = nt % Lout;
        int n = nt / Lout;
        acc += dy[n * Cout * Lout + co * Lout + t];
    }
    smem[tid] = acc;
    __syncthreads();
    for (int s = 128; s > 0; s >>= 1) {
        if (tid < s) smem[tid] += smem[tid + s];
        __syncthreads();
    }
    if (tid == 0) atomicAdd(&db[co], smem[0]);
}
extern "C" void launch_convx_conv1d_backward_bias(
    const double* dy, double* db, int N, int Cout, int Lout, hipStream_t s
) {
    int nl = N * Lout;
    int blocks_per_co = (nl + 255) / 256;
    int grid = Cout * blocks_per_co;
    hipLaunchKernelGGL(convx_conv1d_backward_bias_kernel, dim3(grid), dim3(256),
        256 * sizeof(double), s, dy, db, N, Cout, Lout);
}

// conv2d: x (N,Cin,H,W) w (Cout,Cin,kH,kW) bias (Cout|null) → y (N,Cout,Hout,Wout)
// Hout = H-kH+1, Wout = W-kW+1. Cross-correlation.
extern "C" __global__ void convx_conv2d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int kH, int kW, int Hout, int Wout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cout * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout;
    int r  = idx / Wout;
    int oh = r % Hout;
    r /= Hout;
    int co = r % Cout;
    int n  = r / Cout;
    double acc = bias ? bias[co] : 0.0;
    for (int ci = 0; ci < Cin; ++ci) {
        const double* xp = x + (((n * Cin + ci) * H) * W);
        const double* wp = w + (((co * Cin + ci) * kH) * kW);
        for (int kh = 0; kh < kH; ++kh)
            for (int kw = 0; kw < kW; ++kw)
                acc += xp[(oh + kh) * W + (ow + kw)] * wp[kh * kW + kw];
    }
    y[idx] = acc;
}
extern "C" void launch_convx_conv2d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int kH, int kW, int Hout, int Wout, hipStream_t s
) {
    int total = N * Cout * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_conv2d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, H, W, Cout, kH, kW, Hout, Wout);
}

// conv3d: x (N,Cin,D,H,W) w (Cout,Cin,kD,kH,kW) bias (Cout|null) → y (N,Cout,Dout,Hout,Wout)
extern "C" __global__ void convx_conv3d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int D, int H, int W, int Cout, int kD, int kH, int kW,
    int Dout, int Hout, int Wout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cout * Dout * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout; int r = idx / Wout;
    int oh = r % Hout;   r /= Hout;
    int od = r % Dout;   r /= Dout;
    int co = r % Cout;   int n = r / Cout;
    double acc = bias ? bias[co] : 0.0;
    for (int ci = 0; ci < Cin; ++ci) {
        const double* xp = x + ((((n * Cin + ci) * D) * H) * W);
        const double* wp = w + ((((co * Cin + ci) * kD) * kH) * kW);
        for (int kd = 0; kd < kD; ++kd)
            for (int kh = 0; kh < kH; ++kh)
                for (int kw = 0; kw < kW; ++kw)
                    acc += xp[((od + kd) * H + (oh + kh)) * W + (ow + kw)]
                         * wp[(kd * kH + kh) * kW + kw];
    }
    y[idx] = acc;
}
extern "C" void launch_convx_conv3d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int D, int H, int W, int Cout, int kD, int kH, int kW,
    int Dout, int Hout, int Wout, hipStream_t s
) {
    int total = N * Cout * Dout * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_conv3d_kernel, grid, block, 0, s, x, w, bias, y,
        N, Cin, D, H, W, Cout, kD, kH, kW, Dout, Hout, Wout);
}

// depthwise_conv1d: each input channel convolved by its own filter (groups = Cin),
// channel_multiplier m → Cout = Cin*m. x (N,Cin,L) w (Cin,m,K) → y (N,Cin*m,Lout)
extern "C" __global__ void convx_dwconv1d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int L, int M, int K, int Lout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int Cout = Cin * M;
    int total = N * Cout * Lout;
    if (idx >= total) return;
    int t  = idx % Lout;
    int r  = idx / Lout;
    int co = r % Cout;
    int n  = r / Cout;
    int ci = co / M;
    int m  = co % M;
    double acc = bias ? bias[co] : 0.0;
    const double* xp = x + ((n * Cin + ci) * L);
    const double* wp = w + ((ci * M + m) * K);
    for (int k = 0; k < K; ++k) acc += xp[t + k] * wp[k];
    y[idx] = acc;
}
extern "C" void launch_convx_dwconv1d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int L, int M, int K, int Lout, hipStream_t s
) {
    int total = N * (Cin * M) * Lout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_dwconv1d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, L, M, K, Lout);
}

// depthwise_conv2d: x (N,Cin,H,W) w (Cin,m,kH,kW) → y (N,Cin*m,Hout,Wout)
extern "C" __global__ void convx_dwconv2d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int M, int kH, int kW, int Hout, int Wout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int Cout = Cin * M;
    int total = N * Cout * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout; int r = idx / Wout;
    int oh = r % Hout;   r /= Hout;
    int co = r % Cout;   int n = r / Cout;
    int ci = co / M;
    int m  = co % M;
    double acc = bias ? bias[co] : 0.0;
    const double* xp = x + (((n * Cin + ci) * H) * W);
    const double* wp = w + (((ci * M + m) * kH) * kW);
    for (int kh = 0; kh < kH; ++kh)
        for (int kw = 0; kw < kW; ++kw)
            acc += xp[(oh + kh) * W + (ow + kw)] * wp[kh * kW + kw];
    y[idx] = acc;
}
extern "C" void launch_convx_dwconv2d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int M, int kH, int kW, int Hout, int Wout, hipStream_t s
) {
    int total = N * (Cin * M) * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_dwconv2d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, H, W, M, kH, kW, Hout, Wout);
}

// grouped_conv2d: G groups, Cin split into G chunks, Cout split into G chunks.
// group g maps input channels [g*Cin/G, (g+1)*Cin/G) → output channels [g*Cout/G,...).
// w (Cout, Cin/G, kH, kW). Cross-correlation. (depthwise is the G==Cin special case.)
extern "C" __global__ void convx_gconv2d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int G, int kH, int kW, int Hout, int Wout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cout * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout; int r = idx / Wout;
    int oh = r % Hout;   r /= Hout;
    int co = r % Cout;   int n = r / Cout;
    int cin_g = Cin / G;
    int cout_g = Cout / G;
    int g = co / cout_g;          // which group this output channel belongs to
    int ci_base = g * cin_g;      // first input channel of the group
    double acc = bias ? bias[co] : 0.0;
    for (int ciw = 0; ciw < cin_g; ++ciw) {
        int ci = ci_base + ciw;
        const double* xp = x + (((n * Cin + ci) * H) * W);
        const double* wp = w + (((co * cin_g + ciw) * kH) * kW);
        for (int kh = 0; kh < kH; ++kh)
            for (int kw = 0; kw < kW; ++kw)
                acc += xp[(oh + kh) * W + (ow + kw)] * wp[kh * kW + kw];
    }
    y[idx] = acc;
}
extern "C" void launch_convx_gconv2d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int G, int kH, int kW, int Hout, int Wout, hipStream_t s
) {
    int total = N * Cout * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_gconv2d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, H, W, Cout, G, kH, kW, Hout, Wout);
}

// conv_transpose2d (deconvolution / fractionally-strided), stride 1, no padding.
// x (N,Cin,H,W) w (Cin,Cout,kH,kW) → y (N,Cout,Hout,Wout), Hout=H+kH-1, Wout=W+kW-1.
// Transpose-conv convention (PyTorch): weight indexed [cin,cout,kh,kw], NO flip in the
// scatter form below — it is the exact transpose (adjoint) of the cross-correlation
// forward, which is what conv_transpose computes. Gather form per output element:
//   y[n,co,oh,ow] = sum_ci sum_{kh,kw : 0<=oh-kh<H, 0<=ow-kw<W} x[n,ci,oh-kh,ow-kw]*w[ci,co,kh,kw]
extern "C" __global__ void convx_convtranspose2d_kernel(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int kH, int kW, int Hout, int Wout
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * Cout * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout; int r = idx / Wout;
    int oh = r % Hout;   r /= Hout;
    int co = r % Cout;   int n = r / Cout;
    double acc = bias ? bias[co] : 0.0;
    for (int ci = 0; ci < Cin; ++ci) {
        const double* xp = x + (((n * Cin + ci) * H) * W);
        const double* wp = w + (((ci * Cout + co) * kH) * kW);
        for (int kh = 0; kh < kH; ++kh) {
            int ih = oh - kh;
            if (ih < 0 || ih >= H) continue;
            for (int kw = 0; kw < kW; ++kw) {
                int iw = ow - kw;
                if (iw < 0 || iw >= W) continue;
                acc += xp[ih * W + iw] * wp[kh * kW + kw];
            }
        }
    }
    y[idx] = acc;
}
extern "C" void launch_convx_convtranspose2d(
    const double* x, const double* w, const double* bias, double* y,
    int N, int Cin, int H, int W, int Cout, int kH, int kW, int Hout, int Wout, hipStream_t s
) {
    int total = N * Cout * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_convtranspose2d_kernel, grid, block, 0, s, x, w, bias, y, N, Cin, H, W, Cout, kH, kW, Hout, Wout);
}

// dilation2d (grayscale morphological dilation, TF convention): per channel,
//   y[n,c,oh,ow] = max_{kh,kw} ( x[n,c,oh+kh,ow+kw] + w[c,kh,kw] )
// erosion2d (mode=1): y = min_{kh,kw} ( x[...] - w[c,kh,kw] ). Max-plus / min-plus.
extern "C" __global__ void convx_dilation2d_kernel(
    const double* x, const double* w, double* y,
    int N, int C, int H, int W, int kH, int kW, int Hout, int Wout, int erode
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    int total = N * C * Hout * Wout;
    if (idx >= total) return;
    int ow = idx % Wout; int r = idx / Wout;
    int oh = r % Hout;   r /= Hout;
    int c  = r % C;      int n = r / C;
    const double* xp = x + (((n * C + c) * H) * W);
    const double* wp = w + ((c * kH) * kW);
    double best = erode ? 1e308 : -1e308;
    for (int kh = 0; kh < kH; ++kh)
        for (int kw = 0; kw < kW; ++kw) {
            double v = erode ? (xp[(oh + kh) * W + (ow + kw)] - wp[kh * kW + kw])
                             : (xp[(oh + kh) * W + (ow + kw)] + wp[kh * kW + kw]);
            best = erode ? (v < best ? v : best) : (v > best ? v : best);
        }
    y[idx] = best;
}
extern "C" void launch_convx_dilation2d(
    const double* x, const double* w, double* y,
    int N, int C, int H, int W, int kH, int kW, int Hout, int Wout, int erode, hipStream_t s
) {
    int total = N * C * Hout * Wout;
    dim3 block(256); dim3 grid((total + 255) / 256);
    hipLaunchKernelGGL(convx_dilation2d_kernel, grid, block, 0, s, x, w, y, N, C, H, W, kH, kW, Hout, Wout, erode);
}

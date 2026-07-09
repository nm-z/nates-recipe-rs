#include <hip/hip_runtime.h>
#include <math.h>


extern "C" __global__ void mae_grad_kernel(
      const double* pred, const double* target, double* out, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double d = pred[i] - target[i];
      out[i] = (d > 0.0 ? 1.0 : (d < 0.0 ? -1.0 : 0.0)) / (double)n;
}

extern "C" void launch_mae_grad(
      const double* pred, const double* target, double* out, int n, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(mae_grad_kernel, dim3(blocks), dim3(threads), 0, stream, pred, target, out, n);
}


extern "C" __global__ void huber_grad_kernel(
      const double* pred, const double* target, double* out, int n, const double* delta_p
) {
      double delta = *delta_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double d = pred[i] - target[i];
      double inv_n = 1.0 / (double)n;
      if (d > delta)       out[i] =  delta * inv_n;
      else if (d < -delta) out[i] = -delta * inv_n;
      else                 out[i] =  d * inv_n;
}

extern "C" void launch_huber_grad(
      const double* pred, const double* target, double* out, int n, const double* delta, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(huber_grad_kernel, dim3(blocks), dim3(threads), 0, stream, pred, target, out, n, delta);
}


extern "C" __global__ void bce_logits_kernel(
      const double* z, const double* y, double* loss_out, double* grad_out, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double zi = z[i];
      double yi = y[i];
      double abs_z = zi >= 0.0 ? zi : -zi;
      loss_out[i] = (zi > 0.0 ? zi : 0.0) - zi * yi + log1p(exp(-abs_z));
      double sig = 1.0 / (1.0 + exp(-zi));
      grad_out[i] = sig - yi;
}

extern "C" void launch_bce_logits(
      const double* z, const double* y, double* loss_out, double* grad_out, int n, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(bce_logits_kernel, dim3(blocks), dim3(threads), 0, stream, z, y, loss_out, grad_out, n);
}


extern "C" __global__ void focal_loss_kernel(
      const double* prob, const double* target, double* loss_out, double* grad_out,
      int n, const double* gamma_p, const double* alpha_p
) {
      double gamma = *gamma_p;
      double alpha = *alpha_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double p = prob[i];
      double t = target[i];
      double eps = 1e-12;
      p = p < eps ? eps : (p > 1.0 - eps ? 1.0 - eps : p);

      double p_t = t > 0.5 ? p : 1.0 - p;
      double log_pt = log(p_t);
      double wt = (1.0 - p_t);
      double focal_wt = alpha * pow(wt, gamma);
      loss_out[i] = -focal_wt * log_pt;

      double sign_pt = t > 0.5 ? 1.0 : -1.0;
      double g = -alpha * (
            gamma * pow(wt, gamma - 1.0) * (-sign_pt) * log_pt
            + pow(wt, gamma) * sign_pt / p_t
      );
      grad_out[i] = g;
}

extern "C" void launch_focal_loss(
      const double* prob, const double* target, double* loss_out, double* grad_out,
      int n, const double* gamma, const double* alpha, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(focal_loss_kernel, dim3(blocks), dim3(threads), 0, stream,
            prob, target, loss_out, grad_out, n, gamma, alpha);
}

extern "C" __global__ void focal_grad_kernel(
      const double* prob, const double* target, double* grad_out,
      int n, const double* gamma_p, const double* alpha_p, const double* inv_n_p
) {
      double gamma = *gamma_p;
      double alpha = *alpha_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double inv_n = *inv_n_p;
      double p = prob[i];
      double t = target[i];
      double eps = 1e-12;
      p = p < eps ? eps : (p > 1.0 - eps ? 1.0 - eps : p);

      double p_t = t > 0.5 ? p : 1.0 - p;
      double log_pt = log(p_t);
      double wt = 1.0 - p_t;
      double sign_pt = t > 0.5 ? 1.0 : -1.0;
      double g = -alpha * (
            gamma * pow(wt, gamma - 1.0) * (-sign_pt) * log_pt
            + pow(wt, gamma) * sign_pt / p_t
      );
      grad_out[i] = g * inv_n;
}

extern "C" void launch_focal_grad(
      const double* prob, const double* target, double* grad_out,
      int n, const double* gamma, const double* alpha, const double* inv_n, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(focal_grad_kernel, dim3(blocks), dim3(threads), 0, stream,
            prob, target, grad_out, n, gamma, alpha, inv_n);
}


extern "C" __global__ void kl_div_loss_kernel(
      const double* log_p, const double* target, double* out, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double t = target[i];
      out[i] = (t > 0.0) ? t * (log(t) - log_p[i]) : 0.0;
}

extern "C" void launch_kl_div_loss(
      const double* log_p, const double* target, double* out, int n, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(kl_div_loss_kernel, dim3(blocks), dim3(threads), 0, stream, log_p, target, out, n);
}


extern "C" __global__ void hinge_kernel(
      const double* scores, const double* labels, double* loss_out, double* grad_out, int n
) {
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      double y = labels[i];
      double s = scores[i];
      double margin = 1.0 - y * s;
      loss_out[i] = margin > 0.0 ? margin : 0.0;
      grad_out[i] = margin > 0.0 ? -y : 0.0;
}

extern "C" void launch_hinge(
      const double* scores, const double* labels, double* loss_out, double* grad_out,
      int n, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(hinge_kernel, dim3(blocks), dim3(threads), 0, stream, scores, labels, loss_out, grad_out, n);
}


extern "C" __global__ void cosine_emb_kernel(
      const double* a, const double* b, const double* label, double* out,
      int n, int dim, const double* margin_p
) {
      double margin = *margin_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      const double* ai = a + i * dim;
      const double* bi = b + i * dim;
      double dot = 0.0, na2 = 0.0, nb2 = 0.0;
      for (int d = 0; d < dim; d++) {
            dot += ai[d] * bi[d];
            na2 += ai[d] * ai[d];
            nb2 += bi[d] * bi[d];
      }
      double eps = 1e-12;
      double cos_sim = dot / (sqrt(na2) * sqrt(nb2) + eps);
      double y = label[i];
      if (y > 0.0) {
            out[i] = 1.0 - cos_sim;
      } else {
            double v = cos_sim - margin;
            out[i] = v > 0.0 ? v : 0.0;
      }
}

extern "C" void launch_cosine_emb(
      const double* a, const double* b, const double* label, double* out,
      int n, int dim, const double* margin, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(cosine_emb_kernel, dim3(blocks), dim3(threads), 0, stream,
            a, b, label, out, n, dim, margin);
}


extern "C" __global__ void triplet_kernel(
      const double* anchor, const double* pos, const double* neg, double* out,
      int n, int dim, const double* margin_p
) {
      double margin = *margin_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      const double* a = anchor + i * dim;
      const double* p = pos + i * dim;
      const double* nv = neg + i * dim;
      double d_ap = 0.0, d_an = 0.0;
      for (int d = 0; d < dim; d++) {
            double dap = a[d] - p[d];
            double dan = a[d] - nv[d];
            d_ap += dap * dap;
            d_an += dan * dan;
      }
      double v = d_ap - d_an + margin;
      out[i] = v > 0.0 ? v : 0.0;
}

extern "C" void launch_triplet(
      const double* anchor, const double* pos, const double* neg, double* out,
      int n, int dim, const double* margin, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(triplet_kernel, dim3(blocks), dim3(threads), 0, stream,
            anchor, pos, neg, out, n, dim, margin);
}


extern "C" __global__ void contrastive_kernel(
      const double* a, const double* b, const double* label, double* out,
      int n, int dim, const double* margin_p
) {
      double margin = *margin_p;
      int i = blockIdx.x * blockDim.x + threadIdx.x;
      if (i >= n) return;
      const double* ai = a + i * dim;
      const double* bi = b + i * dim;
      double dist2 = 0.0;
      for (int d = 0; d < dim; d++) {
            double diff = ai[d] - bi[d];
            dist2 += diff * diff;
      }
      double y = label[i];
      double dist = sqrt(dist2);
      double neg_margin = margin - dist;
      double neg_term = neg_margin > 0.0 ? neg_margin * neg_margin : 0.0;
      out[i] = y * dist2 + (1.0 - y) * neg_term;
}

extern "C" void launch_contrastive(
      const double* a, const double* b, const double* label, double* out,
      int n, int dim, const double* margin, hipStream_t stream
) {
      int threads = 256;
      int blocks = (n + threads - 1) / threads;
      hipLaunchKernelGGL(contrastive_kernel, dim3(blocks), dim3(threads), 0, stream,
            a, b, label, out, n, dim, margin);
}

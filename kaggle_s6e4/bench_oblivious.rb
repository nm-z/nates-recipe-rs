#!/usr/bin/env ruby
$LOAD_PATH.unshift File.expand_path("../nates-gpu-ruby/target/release", __dir__)
require "nates_gpu"
$stdout.reopen($stderr); $stdout.sync = true; $stderr.sync = true

# ── Load Abalone ──────────────────────────────────────────────────────────
lines = File.readlines("/tmp/abalone.data").map(&:chomp).reject(&:empty?)
sex_map = { "M" => 0, "F" => 1, "I" => 2 }
rows = lines.map { |l| f = l.split(","); [sex_map[f[0]]] + f[1..].map(&:to_f) }
n = rows.size; nf = 8
x = rows.map { |r| r[0...nf] }.flatten
y = rows.map { |r| r[-1] }
$stderr.puts "Abalone: n=#{n} features=#{nf}"

# Train/test split — exact sklearn train_test_split(test_size=0.2, random_state=0)
tr_idx = File.readlines("/tmp/abalone_train_idx.txt").map { |l| l.strip.to_i }
te_idx = File.readlines("/tmp/abalone_test_idx.txt").map { |l| l.strip.to_i }
n_tr = tr_idx.size; n_te = te_idx.size

x_tr = tr_idx.flat_map { |i| x[i * nf, nf] }
x_te = te_idx.flat_map { |i| x[i * nf, nf] }
y_tr = tr_idx.map { |i| y[i] }
y_te = te_idx.map { |i| y[i] }

# Quantize to u8 bins (256 bins)
NB = 256
def quantize(flat, n, nf, nb)
      bins = Array.new(n * nf)
      bounds = Array.new(nf)
      nf.times do |j|
            col = n.times.map { |i| flat[i * nf + j] }.sort
            bord = (1...nb).map { |q| col[(q * n / nb).clamp(0, n - 1)] }.uniq
            bounds[j] = bord
            n.times { |i| bins[i * nf + j] = (bord.bsearch_index { |t| t > flat[i * nf + j] } || bord.size).clamp(0, 255) }
      end
      [bins, bounds]
end

bins_tr, bounds = quantize(x_tr, n_tr, nf, NB)
bins_te, _ = bounds.each_with_index.map { |bord, j|
      n_te.times.map { |i| (bord.bsearch_index { |t| t > x_te[i * nf + j] } || bord.size).clamp(0, 255) }
}.then { |cols| [n_te.times.flat_map { |i| cols.map { |c| c[i] } }, nil] }

# Upload u8 bins: row-major and feature-major
bins_rm = upload_u8(bins_tr, n_tr, nf)
bins_fm_data = Array.new(nf * n_tr)
nf.times { |f| n_tr.times { |i| bins_fm_data[f * n_tr + i] = bins_tr[i * nf + f] } }
bins_fm = upload_u8(bins_fm_data, nf, n_tr)
te_bins_rm = upload_u8(bins_te, n_te, nf)

# Config
DEPTH = 6; LR = 0.03; LAM = 1.0; ITERS = 1000
n_leaves = 1 << DEPTH

# Scratch buffers (all f32)
pred = zeros_f32(n_tr, 1)
te_pred = zeros_f32(n_te, 1)
grad = zeros_f32(n_tr, 1)
hess = zeros_f32(n_tr, 1)
node_a = zeros_u8(n_tr); node_b = zeros_u8(n_tr)
leaf_idx = zeros_u8(n_tr); te_leaf = zeros_u8(n_te)
max_nodes = n_leaves
grad_hist = zeros_f32(max_nodes * nf, NB)
hess_hist = zeros_f32(max_nodes * nf, NB)
gain_buf = zeros_f32(nf, NB)
leaf_g = zeros_f32(n_leaves, 1); leaf_h = zeros_f32(n_leaves, 1); leaf_v = zeros_f32(n_leaves, 1)

# Upload targets as f32, initialize predictions to y_mean
y_mean = y_tr.sum / n_tr.to_f
tgt_f32 = upload_f32(y_tr, n_tr, 1)
te_tgt = y_te
fill_f32!(pred, y_mean)
fill_f32!(te_pred, y_mean)
fill_f32!(hess, 1.0)

# Persistent device split arrays
sf_dev = upload_i32(Array.new(DEPTH, 0), DEPTH, 1)
sb_dev = zeros_u8(DEPTH)
best_idx_dev = upload_i32([0], 1, 1)

$stderr.puts "Config: depth=#{DEPTH} lr=#{LR} lambda=#{LAM} iters=#{ITERS}"
$stderr.puts "Scratch allocated. Training..."

times_wall = []; times_kernel = []
t_total = Time.now

for t in 0...ITERS
      t_wall = Time.now
      gpu_sync
      t_kern = Time.now

      mse_grad_into!(pred, tgt_f32, grad)
      zero!(node_a)

      DEPTH.times do |d|
            n_nodes = 1 << d
            zero!(grad_hist); zero!(hess_hist)
            oblivious_histogram_into!(bins_fm, node_a, grad, hess, grad_hist, hess_hist, NB, n_nodes)
            zero!(gain_buf)
            oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, NB, LAM)
            argmax_write_split_into!(gain_buf, sf_dev, sb_dev, best_idx_dev, NB, d)
            oblivious_route_step_dev_into!(bins_rm, node_a, node_b, sf_dev, sb_dev, d)
            node_a, node_b = node_b, node_a
      end

      oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, DEPTH)
      zero!(leaf_g); zero!(leaf_h)
      leaf_reduce_into!(leaf_idx, grad, hess, leaf_g, leaf_h)
      leaf_finalize_into!(leaf_g, leaf_h, leaf_v, LAM)
      scatter_add_by_leaf!(pred, leaf_idx, leaf_v, LR)

      oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, DEPTH)
      scatter_add_by_leaf!(te_pred, te_leaf, leaf_v, LR)

      gpu_sync
      dt_kern = Time.now - t_kern
      dt_wall = Time.now - t_wall

      times_wall << dt_wall; times_kernel << dt_kern

      if (t + 1) % 100 == 0
            te_p = download_f32(te_pred)
            rmse = Math.sqrt(n_te.times.sum { |i| (te_p[i] - te_tgt[i]) ** 2 } / n_te)
            $stderr.puts "  iter=%d  rmse=%.4f  wall=%.2fms  kernel=%.2fms" % [t + 1, rmse, dt_wall * 1000, dt_kern * 1000]
      end
end

total = Time.now - t_total
med_wall = times_wall.sort[times_wall.size / 2]
med_kern = times_kernel.sort[times_kernel.size / 2]

te_p = download_f32(te_pred)
rmse = Math.sqrt(n_te.times.sum { |i| (te_p[i] - te_tgt[i]) ** 2 } / n_te)

$stderr.puts "\n# ── results ──────────────────────────────────────────────────────"
$stderr.puts "  dataset:          Abalone (n=#{n}, features=#{nf})"
$stderr.puts "  split:            sklearn train_test_split(test_size=0.2, random_state=0)"
$stderr.puts "  depth=#{DEPTH}  lr=#{LR}  lambda=#{LAM}  iters=#{ITERS}"
$stderr.puts "  RMSE:             %.4f" % rmse
$stderr.puts "  total wall:       %.2fs" % total
$stderr.puts "  median wall:      %.3fms" % (med_wall * 1000)
$stderr.puts "  median kernel:    %.3fms  (gpu_sync bookended)" % (med_kern * 1000)
$stderr.puts ""
$stderr.puts "  CatBoost GPU ref: RMSE=2.154  total=5.763s  iter=0.006s"
$stderr.puts "  XGBoost GPU ref:  RMSE=2.154  total=5.764s  iter=0.005s"
$stderr.puts "  LightGBM GPU ref: RMSE=2.143  total=23.109s iter=0.004s"

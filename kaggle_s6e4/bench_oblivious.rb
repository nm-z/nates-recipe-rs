#!/usr/bin/env ruby
$LOAD_PATH.unshift File.expand_path("../nates-gpu-ruby/target/release", __dir__)
require "nates_gpu"
require_relative "lib/split"
$stdout.reopen($stderr); $stdout.sync = true; $stderr.sync = true

DATASET = ARGV[0] || "abalone"

SPECS = {
      "abalone"       => { task: :regression,   nf: 8,   format: :csv_abalone },
      "letters"       => { task: :multiclass,   nf: 16,  format: :csv_letters,  nc: 26 },
      "epsilon"       => { task: :binary,       nf: 2000, format: :libsvm },
      "higgs"         => { task: :binary,       nf: 28,  format: :csv_higgs },
      "msrank"        => { task: :regression,   nf: 137, format: :libsvm_qid },
      "synthetic"     => { task: :regression,   nf: 100, format: :synthetic,   n_rows: 10_000_000 },
      "synthetic-5k"  => { task: :regression,   nf: 5000, format: :synthetic,  n_rows: 100_000 },
}

raise "Unknown dataset: #{DATASET}. Valid: #{SPECS.keys.join(', ')}" unless SPECS.key?(DATASET)

spec = SPECS[DATASET]
task = spec[:task]
nf   = spec[:nf]

def load_csv_abalone(path)
      sex_map = { "M" => 0.0, "F" => 1.0, "I" => 2.0 }
      rows = File.readlines(path).map(&:chomp).reject(&:empty?).map do |l|
            f = l.split(",")
            [sex_map[f[0]]] + f[1..].map(&:to_f)
      end
      n  = rows.size
      nf = rows[0].size - 1
      x  = rows.flat_map { |r| r[0...nf] }
      y  = rows.map { |r| r[-1] }
      [x, y, n, nf, nil]
end

def load_csv_letters(path)
      label_map = {}
      rows = File.readlines(path).map(&:chomp).reject(&:empty?).map do |l|
            f = l.split(",")
            label_map[f[0]] ||= label_map.size.to_f
            [label_map[f[0]]] + f[1..].map(&:to_f)
      end
      n  = rows.size
      nf = rows[0].size - 1
      x  = rows.flat_map { |r| r[1..] }
      y  = rows.map { |r| r[0] }
      [x, y, n, nf, label_map.size]
end

def load_csv_higgs(path)
      rows = File.readlines(path).map(&:chomp).reject(&:empty?).map { |l| l.split(",").map(&:to_f) }
      n  = rows.size
      nf = rows[0].size - 1
      x  = rows.flat_map { |r| r[1..] }
      y  = rows.map { |r| r[0] }
      [x, y, n, nf, nil]
end

def load_libsvm(path, expected_nf, has_qid)
      x_rows = []
      y      = []
      File.foreach(path) do |line|
            line.chomp!
            next if line.empty?
            parts = line.split(" ")
            y << parts[0].to_f
            row = Array.new(expected_nf, 0.0)
            parts[1..].each do |tok|
                  next if has_qid && tok.start_with?("qid:")
                  idx, val = tok.split(":")
                  i = idx.to_i - 1
                  row[i] = val.to_f if i >= 0 && i < expected_nf
            end
            x_rows << row
      end
      n = x_rows.size
      x = x_rows.flatten
      [x, y, n, expected_nf, nil]
end

def generate_synthetic(n_rows, nf, seed)
      rng = Random.new(seed)
      weights = Array.new(nf) { rng.rand * 2.0 - 1.0 }
      x = Array.new(n_rows * nf) { rng.rand * 2.0 - 1.0 }
      y = Array.new(n_rows) do |i|
            nf.times.sum { |j| x[i * nf + j] * weights[j] }
      end
      [x, y, n_rows, nf, nil]
end

def load_split(_name, n)
      deterministic_split(n, test_size: 0.2, seed: 0)
end

def quantize(flat, n, nf, nb)
      bins   = Array.new(n * nf)
      bounds = Array.new(nf)
      nf.times do |j|
            col   = n.times.map { |i| flat[i * nf + j] }.sort
            bord  = (1...nb).map { |q| col[(q * n / nb).clamp(0, n - 1)] }.uniq
            bounds[j] = bord
            n.times { |i| bins[i * nf + j] = (bord.bsearch_index { |t| t > flat[i * nf + j] } || bord.size).clamp(0, 255) }
      end
      [bins, bounds]
end

$stderr.puts "Loading #{DATASET}..."
t_load = Time.now

x, y, n, nf, nc = case spec[:format]
      when :csv_abalone  then load_csv_abalone("/tmp/abalone.data")
      when :csv_letters  then load_csv_letters("/tmp/letter.data")
      when :csv_higgs    then load_csv_higgs("/tmp/HIGGS.csv")
      when :libsvm       then load_libsvm("/tmp/epsilon_normalized", nf, false)
      when :libsvm_qid   then load_libsvm("/tmp/msrank.txt", nf, true)
      when :synthetic    then generate_synthetic(spec[:n_rows], nf, 0)
      end

nc ||= spec[:nc]
y.map! { |v| v < 0 ? 0.0 : 1.0 } if task == :binary
$stderr.puts "  loaded n=#{n} features=#{nf}#{nc ? " classes=#{nc}" : ""} in %.2fs" % (Time.now - t_load)

tr_idx, te_idx = load_split(DATASET, n)
n_tr = tr_idx.size
n_te = te_idx.size
$stderr.puts "  split: train=#{n_tr} test=#{n_te}"

x_tr = tr_idx.flat_map { |i| x[i * nf, nf] }
x_te = te_idx.flat_map { |i| x[i * nf, nf] }
y_tr = tr_idx.map { |i| y[i] }
y_te = te_idx.map { |i| y[i] }

$stderr.puts "Quantizing..."
t_q = Time.now
NB = 256
bins_tr, bounds = quantize(x_tr, n_tr, nf, NB)

bins_te = bounds.each_with_index.flat_map { |bord, j|
      n_te.times.map { |i| (bord.bsearch_index { |t| t > x_te[i * nf + j] } || bord.size).clamp(0, 255) }
}.then { |cols_flat|
      cols = cols_flat.each_slice(n_te).to_a
      n_te.times.flat_map { |i| cols.map { |c| c[i] } }
}
$stderr.puts "  quantized in %.2fs" % (Time.now - t_q)

bins_rm    = upload_u8(bins_tr, n_tr, nf)
bins_fm_d  = Array.new(nf * n_tr)
nf.times { |f| n_tr.times { |i| bins_fm_d[f * n_tr + i] = bins_tr[i * nf + f] } }
bins_fm    = upload_u8(bins_fm_d, nf, n_tr)
te_bins_rm = upload_u8(bins_te, n_te, nf)

DEPTH    = 6
LR       = 0.03
LAM      = 1.0
ITERS    = 1000
n_leaves = 1 << DEPTH

sf_dev       = upload_i32(Array.new(DEPTH, 0), DEPTH, 1)
sb_dev       = zeros_u8(DEPTH)
best_idx_dev = upload_i32([0], 1, 1)
node_a       = zeros_u8(n_tr)
node_b       = zeros_u8(n_tr)
leaf_idx     = zeros_u8(n_tr)
te_leaf      = zeros_u8(n_te)
grad_hist    = zeros_f32(n_leaves * nf, NB)
hess_hist    = zeros_f32(n_leaves * nf, NB)
gain_buf     = zeros_f32(nf, NB)
leaf_g       = zeros_f32(n_leaves, 1)
leaf_h       = zeros_f32(n_leaves, 1)
leaf_v       = zeros_f32(n_leaves, 1)
tgt_f32      = upload_f32(y_tr, n_tr, 1)

case task
when :regression
      pred    = zeros_f32(n_tr, 1)
      te_pred = zeros_f32(n_te, 1)
      grad    = zeros_f32(n_tr, 1)
      hess    = zeros_f32(n_tr, 1)
      y_mean  = y_tr.sum / n_tr.to_f
      fill_f32!(pred, y_mean)
      fill_f32!(te_pred, y_mean)
      fill_f32!(hess, 1.0)
when :binary
      pred    = zeros_f32(n_tr, 1)
      te_pred = zeros_f32(n_te, 1)
      grad    = zeros_f32(n_tr, 1)
      hess    = zeros_f32(n_tr, 1)
when :multiclass
      pred_k    = nc.times.map { zeros_f32(n_tr, 1) }
      te_pred_k = nc.times.map { zeros_f32(n_te, 1) }
      grad      = zeros_f32(n_tr, 1)
      hess      = zeros_f32(n_tr, 1)
end

_, vram_total = gpu_stats
vram_free_min = gpu_stats[0]

$stderr.puts "Config: dataset=#{DATASET} task=#{task} depth=#{DEPTH} lr=#{LR} lambda=#{LAM} iters=#{ITERS}"
$stderr.puts "Training..."

times_wall   = []
times_kernel = []
t_total      = Time.now

ITERS.times do |t|
      t_wall = Time.now
      gpu_sync
      t_kern = Time.now

      case task
      when :regression
            mse_grad_into!(pred, tgt_f32, grad)
      when :binary
            logloss_grad_f32!(pred, tgt_f32, grad, hess)
      when :multiclass
            nc.times do |k|
                  softmax_ce_class_grad_f32!(pred_k, tgt_f32, grad, hess, k, n_tr)
                  zero!(node_a)
                  DEPTH.times do |d|
                        n_nodes = 1 << d
                        zero!(grad_hist); zero!(hess_hist)
                        oblivious_histogram_into!(bins_fm, node_a, grad, hess, grad_hist, hess_hist, NB, n_nodes)
                        zero!(gain_buf)
                        oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, NB, LAM)
                        argmax_write_split_into!(gain_buf, sf_dev, sb_dev, best_idx_dev, NB, d)
                        best_idx = download_i32_scalar(best_idx_dev)
                        bf = best_idx / NB; bb = best_idx % NB
                        oblivious_route_step_into!(bins_rm, node_a, node_b, bf, bb, d)
                        node_a, node_b = node_b, node_a
                  end
                  oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, DEPTH)
                  zero!(leaf_g); zero!(leaf_h)
                  leaf_reduce_into!(leaf_idx, grad, hess, leaf_g, leaf_h)
                  leaf_finalize_into!(leaf_g, leaf_h, leaf_v, LAM)
                  scatter_add_by_leaf!(pred_k[k], leaf_idx, leaf_v, LR)
                  oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, DEPTH)
                  scatter_add_by_leaf!(te_pred_k[k], te_leaf, leaf_v, LR)
            end
      end

      if task != :multiclass
            zero!(node_a)
            DEPTH.times do |d|
                  n_nodes = 1 << d
                  zero!(grad_hist); zero!(hess_hist)
                  oblivious_histogram_into!(bins_fm, node_a, grad, hess, grad_hist, hess_hist, NB, n_nodes)
                  zero!(gain_buf)
                  oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, NB, LAM)
                  argmax_write_split_into!(gain_buf, sf_dev, sb_dev, best_idx_dev, NB, d)
                  best_idx = download_i32_scalar(best_idx_dev)
                  bf = best_idx / NB; bb = best_idx % NB
                  oblivious_route_step_into!(bins_rm, node_a, node_b, bf, bb, d)
                  node_a, node_b = node_b, node_a
            end
            oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, DEPTH)
            zero!(leaf_g); zero!(leaf_h)
            leaf_reduce_into!(leaf_idx, grad, hess, leaf_g, leaf_h)
            leaf_finalize_into!(leaf_g, leaf_h, leaf_v, LAM)
            scatter_add_by_leaf!(pred, leaf_idx, leaf_v, LR)
            oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, DEPTH)
            scatter_add_by_leaf!(te_pred, te_leaf, leaf_v, LR)
      end

      gpu_sync
      dt_kern = Time.now - t_kern
      dt_wall = Time.now - t_wall
      times_wall   << dt_wall
      times_kernel << dt_kern

      cur_free = gpu_stats[0]
      vram_free_min = cur_free if cur_free < vram_free_min

      if (t + 1) % 100 == 0
            metric_str = case task
            when :regression
                  te_p = download_f32(te_pred)
                  rmse = Math.sqrt(n_te.times.sum { |i| (te_p[i] - y_te[i]) ** 2 } / n_te)
                  "rmse=%.4f" % rmse
            when :binary
                  te_p = download_f32(te_pred)
                  acc  = n_te.times.count { |i| (te_p[i] >= 0.0) == (y_te[i] >= 0.5) }.to_f / n_te
                  "acc=%.4f" % acc
            when :multiclass
                  te_ps = te_pred_k.map { |b| download_f32(b) }
                  correct = n_te.times.count { |i| te_ps.each_with_index.max_by { |p, _| p[i] }[1] == y_te[i].to_i }
                  "acc=%.4f" % (correct.to_f / n_te)
            end
            $stderr.puts "  iter=%d  #{metric_str}  wall=%.2fms  kernel=%.2fms" % [t + 1, dt_wall * 1000, dt_kern * 1000]
      end
end

total    = Time.now - t_total
med_wall = times_wall.sort[times_wall.size / 2]
med_kern = times_kernel.sort[times_kernel.size / 2]
peak_vram = (vram_total - vram_free_min).round(1)

final_metric = case task
when :regression
      te_p = download_f32(te_pred)
      rmse = Math.sqrt(n_te.times.sum { |i| (te_p[i] - y_te[i]) ** 2 } / n_te)
      "RMSE: %.4f" % rmse
when :binary
      te_p = download_f32(te_pred)
      acc  = n_te.times.count { |i| (te_p[i] >= 0.0) == (y_te[i] >= 0.5) }.to_f / n_te
      "Accuracy: %.4f" % acc
when :multiclass
      te_ps = te_pred_k.map { |b| download_f32(b) }
      correct = n_te.times.count { |i| te_ps.each_with_index.max_by { |p, _| p[i] }[1] == y_te[i].to_i }
      "Accuracy: %.4f" % (correct.to_f / n_te)
end

$stderr.puts ""
$stderr.puts "# ── results ──────────────────────────────────────────────────────"
$stderr.puts "  dataset:          #{DATASET} (n=#{n}, features=#{nf}#{nc ? ", classes=#{nc}" : ""})"
$stderr.puts "  split:            test_size=0.2 seed=0"
$stderr.puts "  task:             #{task}"
$stderr.puts "  depth=#{DEPTH}  lr=#{LR}  lambda=#{LAM}  iters=#{ITERS}"
$stderr.puts "  #{final_metric}"
$stderr.puts "  total wall:       %.2fs" % total
$stderr.puts "  median wall:      %.3fms" % (med_wall * 1000)
$stderr.puts "  median kernel:    %.3fms  (gpu_sync bookended)" % (med_kern * 1000)
$stderr.puts "  peak VRAM:        %.1f MB / %.1f MB" % [peak_vram, vram_total]

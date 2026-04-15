#!/usr/bin/env ruby
$LOAD_PATH.unshift File.expand_path("../nates-gpu-ruby/target/release", __dir__)
require "nates_gpu"
require "json"
$stdout.reopen($stderr); $stdout.sync = true; $stderr.sync = true

BENCH_DIR = "/tmp/bench_data"
DATASETS = %w[abalone letters epsilon higgs msrank synthetic synthetic-5k]

NB = 256
DEPTH = 6; LR = 0.03; LAM = 1.0; ITERS = 500

def load_dataset(name)
      meta = JSON.parse(File.read(File.join(BENCH_DIR, "#{name}_meta.json")), symbolize_names: true)
      n = meta[:n_rows]; nf = meta[:n_features]; n_tr = meta[:n_train]; n_te = n - n_tr
      x_all = File.binread(File.join(BENCH_DIR, "#{name}_x.bin")).unpack("e*")
      y_all = File.binread(File.join(BENCH_DIR, "#{name}_y.bin")).unpack("e*")
      [x_all[0, n_tr * nf], y_all[0, n_tr], x_all[n_tr * nf, n_te * nf], y_all[n_tr, n_te],
       n_tr, n_te, nf, meta[:n_classes].to_i, meta[:task]]
end

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

def apply_bounds(x_te, bounds, n_te, nf)
      n_te.times.flat_map { |i|
            nf.times.map { |j|
                  bord = bounds[j]
                  (bord.bsearch_index { |t| t > x_te[i * nf + j] } || bord.size).clamp(0, 255)
            }
      }
end

def run_regression(bins_fm, bins_rm, te_bins_rm, y_tr, y_te, n_tr, n_te, nf, n_leaves, depth, lr, lam, iters)
      pred       = zeros_f32(n_tr, 1); te_pred    = zeros_f32(n_te, 1)
      grad       = zeros_f32(n_tr, 1); hess       = zeros_f32(n_tr, 1)
      node_a     = zeros_u8(n_tr); node_b = zeros_u8(n_tr)
      leaf_idx   = zeros_u8(n_tr); te_leaf = zeros_u8(n_te)
      grad_hist  = zeros_f32(n_leaves * nf, NB); hess_hist = zeros_f32(n_leaves * nf, NB)
      gain_buf   = zeros_f32(nf, NB)
      leaf_g     = zeros_f32(n_leaves, 1); leaf_h = zeros_f32(n_leaves, 1); leaf_v = zeros_f32(n_leaves, 1)
      sf_dev     = upload_i32(Array.new(depth, 0), depth, 1)
      sb_dev     = zeros_u8(depth); best_idx_dev = upload_i32([0], 1, 1)

      y_mean = y_tr.sum / n_tr.to_f
      tgt_f32 = upload_f32(y_tr, n_tr, 1)
      fill_f32!(pred, y_mean); fill_f32!(te_pred, y_mean); fill_f32!(hess, 1.0)

      times_kernel = []
      t_total = Time.now
      iters.times do |t|
            gpu_sync; t_kern = Time.now
            mse_grad_into!(pred, tgt_f32, grad)
            zero!(node_a)
            depth.times do |d|
                  n_nodes = 1 << d
                  zero!(grad_hist); zero!(hess_hist)
                  oblivious_histogram_into!(bins_fm, node_a, grad, hess, grad_hist, hess_hist, NB, n_nodes)
                  zero!(gain_buf)
                  oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, NB, lam)
                  argmax_write_split_into!(gain_buf, sf_dev, sb_dev, best_idx_dev, NB, d)
                  oblivious_route_step_dev_into!(bins_rm, node_a, node_b, sf_dev, sb_dev, d)
                  node_a, node_b = node_b, node_a
            end
            oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, depth)
            zero!(leaf_g); zero!(leaf_h)
            leaf_reduce_into!(leaf_idx, grad, hess, leaf_g, leaf_h)
            leaf_finalize_into!(leaf_g, leaf_h, leaf_v, lam)
            scatter_add_by_leaf!(pred, leaf_idx, leaf_v, lr)
            oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, depth)
            scatter_add_by_leaf!(te_pred, te_leaf, leaf_v, lr)
            gpu_sync; times_kernel << (Time.now - t_kern)
      end
      total = Time.now - t_total
      te_p = download_f32(te_pred)
      rmse = Math.sqrt(n_te.times.sum { |i| (te_p[i] - y_te[i]) ** 2 } / n_te)
      med_kern = times_kernel.sort[times_kernel.size / 2]
      [rmse, "RMSE", total, med_kern]
end

def run_multiclass(bins_fm, bins_rm, te_bins_rm, y_tr, y_te, n_tr, n_te, nf, nc, n_leaves, depth, lr, lam, iters)
      pred      = zeros_f32(n_tr, nc); te_pred   = zeros_f32(n_te, nc)
      grad_all  = zeros_f32(n_tr, nc); hess_all  = zeros_f32(n_tr, nc)
      node_a    = zeros_u8(n_tr); node_b = zeros_u8(n_tr)
      leaf_idx  = zeros_u8(n_tr); te_leaf = zeros_u8(n_te)
      grad_hist = zeros_f32(n_leaves * nf, NB); hess_hist = zeros_f32(n_leaves * nf, NB)
      gain_buf  = zeros_f32(nf, NB)
      leaf_g    = zeros_f32(n_leaves, 1); leaf_h = zeros_f32(n_leaves, 1); leaf_v = zeros_f32(n_leaves, 1)
      sf_dev    = upload_i32(Array.new(depth, 0), depth, 1)
      sb_dev    = zeros_u8(depth); best_idx_dev = upload_i32([0], 1, 1)

      tgt_f32    = upload_f32(y_tr, n_tr, 1)
      te_tgt_f32 = upload_f32(y_te, n_te, 1)

      times_kernel = []
      t_total = Time.now
      iters.times do |t|
            gpu_sync; t_kern = Time.now

            prob = copy(pred)
            softmax_inplace_mc!(prob, nc)
            logloss_grad_mc_into!(prob, tgt_f32, grad_all, hess_all, nc)

            grad_t = transpose(reshape(grad_all, n_tr, nc))
            hess_t = transpose(reshape(hess_all, n_tr, nc))

            nc.times do |c|
                  grad_c = reshape(slice_rows(grad_t, c, 1), n_tr, 1)
                  hess_c = reshape(slice_rows(hess_t, c, 1), n_tr, 1)

                  zero!(node_a)
                  depth.times do |d|
                        n_nodes = 1 << d
                        zero!(grad_hist); zero!(hess_hist)
                        oblivious_histogram_into!(bins_fm, node_a, grad_c, hess_c, grad_hist, hess_hist, NB, n_nodes)
                        zero!(gain_buf)
                        oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, NB, lam)
                        argmax_write_split_into!(gain_buf, sf_dev, sb_dev, best_idx_dev, NB, d)
                        best_idx = download_i32_scalar(best_idx_dev)
                        bf = best_idx / NB; bb = best_idx % NB
                        oblivious_route_step_into!(bins_rm, node_a, node_b, bf, bb, d)
                        node_a, node_b = node_b, node_a
                  end

                  oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, depth)
                  zero!(leaf_g); zero!(leaf_h)
                  leaf_reduce_into!(leaf_idx, grad_c, hess_c, leaf_g, leaf_h)
                  leaf_finalize_into!(leaf_g, leaf_h, leaf_v, lam)
                  scatter_add_by_leaf_col!(pred, leaf_idx, leaf_v, lr, nc, c)

                  oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, depth)
                  scatter_add_by_leaf_col!(te_pred, te_leaf, leaf_v, lr, nc, c)
            end

            gpu_sync; times_kernel << (Time.now - t_kern)
      end
      total = Time.now - t_total
      acc = accuracy(te_pred, te_tgt_f32, nc)
      med_kern = times_kernel.sort[times_kernel.size / 2]
      [acc, "Acc", total, med_kern]
end

summary_rows = []
DATASETS.each do |ds|
      meta_f = File.join(BENCH_DIR, "#{ds}_meta.json")
      unless File.exist?(meta_f)
            $stderr.puts "#{ds}: missing #{meta_f}, skipping (run prep_bench_data.rb first)"
            next
      end
      meta = JSON.parse(File.read(meta_f), symbolize_names: true)
      n_total = meta[:n_rows]; nf = meta[:n_features]; task = meta[:task]
      nc = task == "binary" ? 2 : meta[:n_classes].to_i
      $stderr.puts "\n=== #{ds} (n=#{n_total} feat=#{nf} task=#{task}) ==="

      x_tr, y_tr, x_te, y_te, n_tr, n_te, _nf, _nc, _task = load_dataset(ds)
      n_leaves = 1 << DEPTH

      t_prep = Time.now
      bins_tr, bounds = quantize(x_tr, n_tr, nf, NB)
      bins_te = apply_bounds(x_te, bounds, n_te, nf)
      $stderr.puts "  prep: %.2fs" % (Time.now - t_prep)

      bins_rm = upload_u8(bins_tr, n_tr, nf)
      bins_fm_data = Array.new(nf * n_tr)
      nf.times { |f| n_tr.times { |i| bins_fm_data[f * n_tr + i] = bins_tr[i * nf + f] } }
      bins_fm = upload_u8(bins_fm_data, nf, n_tr)
      te_bins_rm = upload_u8(bins_te, n_te, nf)

      gs_before = gpu_stats
      vram_before_free = gs_before[0]

      score, metric, total, med_kern = if task == "regression"
            run_regression(bins_fm, bins_rm, te_bins_rm, y_tr, y_te, n_tr, n_te, nf, n_leaves, DEPTH, LR, LAM, ITERS)
      else
            run_multiclass(bins_fm, bins_rm, te_bins_rm, y_tr, y_te, n_tr, n_te, nf, nc, n_leaves, DEPTH, LR, LAM, ITERS)
      end

      gs_after = gpu_stats
      peak_mb = gs_after[1] - [vram_before_free, gs_after[0]].min

      task_label = case task
      when "regression" then "regression"
      when "binary"     then "binary"
      else "multi (#{nc})"
      end

      summary_rows << {
            ds: ds, n: n_total, feat: nf, task: task_label,
            metric: metric, score: score.round(4),
            med_kern_ms: (med_kern * 1000).round(3),
            total_s: total.round(2),
            peak_mb: peak_mb
      }
      $stderr.puts "  #{metric}=#{score.round(4)}  total=#{total.round(2)}s  kernel=#{(med_kern*1000).round(3)}ms  VRAM~#{peak_mb}MB"
end

hdr = "%-14s | %9s | %5s | %-12s | %6s | %8s | %12s | %12s | %9s" %
      %w[Dataset Rows Feat Task Metric Score Med_Kernel Total_Wall Peak_VRAM]
sep = "-" * hdr.size

$stderr.puts "\n#{sep}"
$stderr.puts hdr
$stderr.puts sep
summary_rows.each do |r|
      $stderr.puts "%-14s | %9s | %5d | %-12s | %6s | %8s | %12s | %12s | %9s" % [
            r[:ds], r[:n].to_s, r[:feat], r[:task], r[:metric],
            r[:score].to_s, "#{r[:med_kern_ms]}ms", "#{r[:total_s]}s", "#{r[:peak_mb]}MB"
      ]
end
$stderr.puts sep

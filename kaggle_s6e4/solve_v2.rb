#!/usr/bin/env ruby
$LOAD_PATH.unshift File.expand_path("../nates-gpu-ruby/target/release", __dir__)
require "nates_gpu"
require "json"
require "digest"
$stdout.sync = true; $stderr.sync = true
$stdout.reopen($stderr)
require "xgb"
require "lightgbm"

class XGBoost::DMatrix
      def self.from_flat(flat, nrow, ncol, label: nil, weight: nil, missing: Float::NAN)
            c_data = ::FFI::MemoryPointer.new(:float, nrow * ncol)
            c_data.write_array_of_float(flat)
            out = ::FFI::MemoryPointer.new(:pointer)
            err = XGBoost::FFI.XGDMatrixCreateFromMat(c_data, nrow, ncol, missing, out)
            raise XGBoost::Error, XGBoost::FFI.XGBGetLastError if err != 0
            dm = allocate
            dm.instance_variable_set(:@handle, ::FFI::AutoPointer.new(out.read_pointer, XGBoost::FFI.method(:XGDMatrixFree)))
            dm.label = label if label
            dm.weight = weight if weight
            dm
      end
end

class LightGBM::Dataset
      def self.from_flat(flat, nrow, ncol, label: nil, params: nil, reference: nil)
            c_data = ::FFI::MemoryPointer.new(:double, nrow * ncol)
            c_data.write_array_of_double(flat)
            handle = ::FFI::MemoryPointer.new(:pointer)
            prms = (params || {}).map { |k, v| "#{k}=#{v}" }.join(" ")
            ref_handle = reference ? reference.handle : nil
            err = LightGBM::FFI.LGBM_DatasetCreateFromMat(c_data, LightGBM::FFI::C_API_DTYPE_FLOAT64, nrow, ncol, 1, prms, ref_handle, handle)
            raise LightGBM::Error, LightGBM::FFI.LGBM_GetLastError if err != 0
            ds = allocate
            ds.instance_variable_set(:@handle, ::FFI::AutoPointer.new(handle.read_pointer, LightGBM::FFI.method(:LGBM_DatasetFree)))
            ds.instance_variable_set(:@data, nil)
            ds.instance_variable_set(:@params, params)
            ds.label = label if label
            ds
      end
end

class LightGBM::Booster
      def predict_flat(flat, nrow, ncol, num_iteration: nil, start_iteration: 0)
            num_iteration = best_iteration if num_iteration.nil? && start_iteration <= 0
            num_iteration ||= -1
            predict_type = LightGBM::FFI::C_API_PREDICT_NORMAL
            out_n = ::FFI::MemoryPointer.new(:int64)
            LightGBM::FFI.LGBM_BoosterCalcNumPredict(@handle, nrow, predict_type, start_iteration, num_iteration, out_n)
            n_preds = out_n.read_int64
            c_data = ::FFI::MemoryPointer.new(:double, nrow * ncol)
            c_data.write_array_of_double(flat)
            out_num = ::FFI::MemoryPointer.new(:int64)
            out_res = ::FFI::MemoryPointer.new(:double, n_preds)
            err = LightGBM::FFI.LGBM_BoosterPredictForMat(@handle, c_data, LightGBM::FFI::C_API_DTYPE_FLOAT64, nrow, ncol, 1, predict_type, start_iteration, num_iteration, "", out_num, out_res)
            raise LightGBM::Error, LightGBM::FFI.LGBM_GetLastError if err != 0
            out_res.read_array_of_double(out_num.read_int64)
      end
end

def mem_report(label)
      rss = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      free, total = gpu_stats
      $stderr.puts "  %-25s RAM=%6dMB  VRAM=%5dMB used" % [label, rss, total - free]
end

DIR = File.dirname(__FILE__)
ART = "#{DIR}/artifacts"
N_BINS = 256; NC = 3; K_FOLDS = 5; PATIENCE = 50
CLASSES = %w[High Low Medium]; SMOOTH = 10.0
$model_stats = []
CACHE = "#{DIR}/cache"
Dir.mkdir(CACHE) unless Dir.exist?(CACHE)
Dir.mkdir(ART) unless Dir.exist?(ART)
DATA_HASH = Digest::MD5.hexdigest(File.read("#{DIR}/train.csv")[0..4096] + "seed42_folds#{K_FOLDS}_bins#{N_BINS}")[0..7]

GBM_CFGS = [
      { depth: 6, lr: 0.03, rounds: 1200, lam: 2.0, ss: 0.70, mcw: 5.0, seed: 42 },
      #{ depth: 5, lr: 0.02, rounds: 1500, lam: 1.5, ss: 0.65, mcw: 5.0, seed: 42 },
      #{ depth: 7, lr: 0.05, rounds:  800, lam: 3.0, ss: 0.75, mcw: 5.0, seed: 42 },
      #{ depth: 8, lr: 0.05, rounds:  600, lam: 1.0, ss: 0.80, mcw: 10.0, seed: 42 },
      #{ depth: 5, lr: 0.03, rounds: 1000, lam: 2.5, ss: 0.70, mcw: 5.0, seed: 123 },
      #{ depth: 6, lr: 0.04, rounds: 1000, lam: 2.0, ss: 0.75, mcw: 5.0, seed: 77 },
      #{ depth: 6, lr: 0.03, rounds: 1200, lam: 1.0, ss: 0.85, mcw: 3.0, seed: 99 },
]
NN_CFGS = [
      { h: 64, blocks: 3, drop: 0.3, lr: 1e-3, wd: 1e-3, epochs: 300, k_best: 40, seed: 42 },
      #{ h: 64,  blocks: 2, drop: 0.4, lr: 5e-4, wd: 1e-3, epochs: 300, k_best: 50, seed: 77 },
]
NN_PATIENCE = 50; LN_EPS = 1e-5

CAT = %w[Soil_Type Crop_Type Crop_Growth_Stage Season Irrigation_Type Water_Source Mulching_Used Region]
NUM = %w[Soil_pH Soil_Moisture Organic_Carbon Electrical_Conductivity Temperature_C Humidity Rainfall_mm Sunlight_Hours Wind_Speed_kmh Field_Area_hectare Previous_Irrigation_mm]
CROSS_PAIRS_CI = []

# ── Load ───────────────────────────────────────────────────────────────────
$stderr.puts "Loading data..."
# Load original dataset for NN augmentation
orig_lines = File.readlines("#{DIR}/irrigation_prediction.csv")
orig_headers = orig_lines.shift.chomp.split(',')
orig_f = orig_lines.map { |l| l.chomp.split(',') }; orig_lines = nil
n_orig = orig_f.size
$stderr.puts "Original dataset: #{n_orig} rows"
tr_lines = File.readlines("#{DIR}/train.csv")
te_lines = File.readlines("#{DIR}/test.csv")
headers = tr_lines.shift.chomp.split(',')
te_lines.shift
tr_f = tr_lines.map { |l| l.chomp.split(',') }; tr_lines = nil
te_f = te_lines.map { |l| l.chomp.split(',') }; te_lines = nil
n_tr = tr_f.size; n_te = te_f.size
test_ids = te_f.map { |f| f[headers.index("id")] }
tgt_enc = CLASSES.each_with_index.to_h
targets_f = tr_f.map { |f| v = tgt_enc[f[headers.index("Irrigation_Need")]]; raise "unknown label: #{f[headers.index("Irrigation_Need")].inspect}" if v.nil?; v.to_f }
targets_i = targets_f.map(&:to_i)

cat_ci = CAT.map { |c| headers.index(c) }
num_ci = NUM.map { |f| headers.index(f) }
CROSS_PAIRS_CI.replace([[headers.index("Soil_Type"), headers.index("Crop_Type")],
                        [headers.index("Soil_Type"), headers.index("Season")],
                        [headers.index("Crop_Type"), headers.index("Season")]])

# ── Precompute numeric + interaction features (shared, no target info) ────
$stderr.puts "Precomputing features..."
SM = headers.index("Soil_Moisture"); TC = headers.index("Temperature_C")
HU = headers.index("Humidity");     RF = headers.index("Rainfall_mm")
WS = headers.index("Wind_Speed_kmh"); SH = headers.index("Sunlight_Hours")
PI = headers.index("Previous_Irrigation_mm")

def eng(f)
      m = f[SM].to_f; t = [f[TC].to_f, 1.0].max; hu = [f[HU].to_f, 1.0].max
      r = f[RF].to_f; w = f[WS].to_f; s = f[SH].to_f; p = f[PI].to_f
      [m / t, w * t, r - p, t * w / hu, r / t, m * r / 1000.0,
       t / hu, s * t, m * p / 100.0, w * s / hu, m * hu / 100.0, (t - 25.0).abs]
end

num_tr = tr_f.map { |f| num_ci.map { |ci| f[ci].to_f } }
num_te = te_f.map { |f| num_ci.map { |ci| f[ci].to_f } }
eng_tr = tr_f.map { |f| eng(f) }
eng_te = te_f.map { |f| eng(f) }

# Precompute original dataset features (for NN augmentation)
orig_num_ci = NUM.map { |c| orig_headers.index(c) }
orig_cat_ci = CAT.map { |c| orig_headers.index(c) }
orig_sm = orig_headers.index("Soil_Moisture"); orig_tc = orig_headers.index("Temperature_C")
orig_hu = orig_headers.index("Humidity"); orig_rf = orig_headers.index("Rainfall_mm")
orig_ws = orig_headers.index("Wind_Speed_kmh"); orig_sh = orig_headers.index("Sunlight_Hours")
orig_pi = orig_headers.index("Previous_Irrigation_mm")
def eng_orig(f, sm, tc, hu, rf, ws, sh, pi)
      m = f[sm].to_f; t = [f[tc].to_f, 1.0].max; hu_ = [f[hu].to_f, 1.0].max
      r = f[rf].to_f; w = f[ws].to_f; s = f[sh].to_f; p = f[pi].to_f
      [m / t, w * t, r - p, t * w / hu_, r / t, m * r / 1000.0,
       t / hu_, s * t, m * p / 100.0, w * s / hu_, m * hu_ / 100.0, (t - 25.0).abs]
end
orig_num = orig_f.map { |f| orig_num_ci.map { |ci| f[ci].to_f } }
orig_eng = orig_f.map { |f| eng_orig(f, orig_sm, orig_tc, orig_hu, orig_rf, orig_ws, orig_sh, orig_pi) }
orig_tgt_i = orig_f.map { |f| v = tgt_enc[f[orig_headers.index("Irrigation_Need")]]; raise "unknown orig label: #{f[orig_headers.index("Irrigation_Need")].inspect}" if v.nil?; v }

N_NUM = num_ci.size; N_CAT = cat_ci.size; N_ENG = 12
CB_CAT_CI = (N_NUM...(N_NUM + N_CAT)).to_a
N_TE_FEAT = (CAT.size + CROSS_PAIRS_CI.size) * NC
NF = N_NUM + N_CAT + N_TE_FEAT + N_ENG
$stderr.puts "#{NF} features"

# ── Folds ──────────────────────────────────────────────────────────────────
srand(42)
folds = Array.new(K_FOLDS) { [] }
per_class = Array.new(NC) { [] }
targets_i.each_with_index { |t, i| per_class[t] << i }
per_class.each { |idxs| idxs.shuffle!; idxs.each_with_index { |idx, i| folds[i % K_FOLDS] << idx } }

# ── Data prep helpers (per-fold TE, label encoding, binning) ──────────────
def compute_te_stats(train_idx, targets_i, tr_f, cat_ci, cross_pairs, priors)
      single = {}
      cat_ci.each do |ci|
            cc = Hash.new { |h, k| h[k] = Array.new(NC, 0) }; cn = Hash.new(0)
            train_idx.each { |i| v = tr_f[i][ci]; cn[v] += 1; cc[v][targets_i[i]] += 1 }
            single[ci] = [cc, cn]
      end
      cross = {}
      cross_pairs.each do |a, b|
            cc = Hash.new { |h, k| h[k] = Array.new(NC, 0) }; cn = Hash.new(0)
            train_idx.each { |i| v = "#{tr_f[i][a]}_#{tr_f[i][b]}"; cn[v] += 1; cc[v][targets_i[i]] += 1 }
            cross[[a, b]] = [cc, cn]
      end
      [single, cross]
end

def apply_te(fields, single, cross, cat_ci, cross_pairs, priors)
      te = []
      cat_ci.each { |ci| cc, cn = single[ci]; v = fields[ci]; NC.times { |k| te << (cn[v] > 0 ? (cc[v][k] + SMOOTH * priors[k]) / (cn[v] + SMOOTH) : priors[k]) } }
      cross_pairs.each { |a, b| cc, cn = cross[[a, b]]; v = "#{fields[a]}_#{fields[b]}"; NC.times { |k| te << (cn[v] > 0 ? (cc[v][k] + SMOOTH * priors[k]) / (cn[v] + SMOOTH) : priors[k]) } }
      te
end

def build_and_bin(row_idx, num_rows, eng_rows, fields, te_stats, cat_ci, cross_pairs, priors, bounds, cat_enc)
      single, cross = te_stats; n = row_idx.size
      cat_enc ||= {}.tap { |e| cat_ci.each { |ci| vals = {}; row_idx.each { |i| vals[fields[i][ci]] = true }; e[ci] = vals.keys.sort.each_with_index.to_h } }
      flat = Array.new(n * NF)
      n.times do |li|
            i = row_idx[li]
            te = apply_te(fields[i], single, cross, cat_ci, cross_pairs, priors)
            cv = cat_ci.map { |ci| (cat_enc[ci][fields[i][ci]] || cat_enc[ci].size).to_f }
            row = num_rows[i] + cv + te + eng_rows[i]
            NF.times { |j| flat[li * NF + j] = row[j] }
      end
      if bounds
            bins = Array.new(n * NF)
            NF.times { |j| b = bounds[j]; n.times { |i| bins[i * NF + j] = (b.bsearch_index { |t| t > flat[i * NF + j] } || b.size).to_f } }
            [bins, bounds, flat, cat_enc]
      else
            bds = Array.new(NF); bins = Array.new(n * NF)
            NF.times { |j| col = Array.new(n) { |i| flat[i * NF + j] }.sort; b = (1...N_BINS).map { |q| col[(q * n / N_BINS).clamp(0, n-1)] }.uniq; bds[j] = b; n.times { |i| bins[i * NF + j] = (b.bsearch_index { |t| t > flat[i * NF + j] } || b.size).to_f } }
            [bins, bds, flat, cat_enc]
      end
end

# ── GBM training ──────────────────────────────────────────────────────────
def train_gbm(g_tr, g_te, g_tgt, g_sw, cfg, label, val_tgt, max_rounds: nil)
      $stderr.puts "    #{label}..."
      rounds = max_rounds || cfg[:rounds]
      ntr = g_tr.rows; nte = g_te.rows
      l = zeros(ntr, NC); tl = zeros(nte, NC)
      p = zeros(ntr, NC)
      g = zeros(ntr, 1); h = zeros(ntr, 1); m = zeros(ntr, 1)
      tr_p = zeros(ntr, 1); te_p = zeros(nte, 1)
      best_ba = 0; stale = 0; best_r = 0; best_tl = nil
      t0 = Time.now

      prev_tr_ptr = nil; prev_te_ptr = nil
      for r in 0...rounds
            alloc_count_reset
            vf0, vt0 = gpu_stats; v0 = vt0 - vf0
            rss0 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
            t_r = Time.now

            softmax_into!(l, p)
            for k in 0...NC
                  bernoulli_into!(m, cfg[:ss], cfg[:seed] + r * NC + k)
                  grad_hess_into!(p, g_tgt, g_sw, m, g, h, k)
                  alloc_count_reset
                  vfb, vtb = gpu_stats; vb = vtb - vfb
                  t_tree = Time.now
                  tree_build_into!(g_tr, g_te, g, h, N_BINS, cfg[:depth], cfg[:lam], cfg[:mcw], tr_p, te_p)
                  t_tree_ms = (Time.now - t_tree) * 1000
                  allocs = alloc_count_reset
                  vfa, vta = gpu_stats; va = vta - vfa
                  cur_tr = tr_p.ptr_addr; cur_te = te_p.ptr_addr
                  ptr_changed = cur_tr != prev_tr_ptr || cur_te != prev_te_ptr
                  prev_tr_ptr = cur_tr; prev_te_ptr = cur_te
                  if allocs > 0 || va - vb != 0 || ptr_changed
                        $stderr.puts "      r=%d k=%d tree_build  ms=%.1f  vram %d->%d (%+d)  allocs=%d  ptrs tr=%s te=%s" % [r, k, t_tree_ms, vb, va, va - vb, allocs, cur_tr, cur_te]
                  end
                  add_col!(l,  k, tr_p, cfg[:lr])
                  add_col!(tl, k, te_p, cfg[:lr])
            end

            round_allocs = alloc_count_reset
            vf1, vt1 = gpu_stats; v1 = vt1 - vf1
            rss1 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
            round_ms = (Time.now - t_r) * 1000
            if val_tgt
                  ba = report(tl, val_tgt, r)
                  if round_allocs > 0 || v1 - v0 != 0
                        $stderr.puts "      r=%d total %.0fms  vram %d->%d (%+d)  ram %d->%d  allocs=%d" % [r, round_ms, v0, v1, v1 - v0, rss0, rss1, round_allocs]
                  end
                  if ba > best_ba
                        best_ba = ba; stale = 0; best_r = r + 1
                        alloc_count_reset
                        best_tl = copy(tl)
                        ca = alloc_count_reset
                        $stderr.puts "      r=%d copy(tl) allocs=%d ptr=%s" % [r, ca, best_tl.ptr_addr]
                  else stale += 1; break if stale >= PATIENCE; end
            end
      end
      best_r = rounds if best_r == 0
      $stderr.puts "    #{label} (d=#{cfg[:depth]} lr=#{cfg[:lr]} stopped=#{best_r}): %.1fs" % [Time.now - t0]
      [download(softmax(best_tl || tl)), best_r]
end

# ── NN helpers ────────────────────────────────────────────────────────────
P_ = Struct.new(:w, :m, :v)
$nn_seed = 1000
def nn_p(fi, fo) $nn_seed += 1; P_.new(randn(fi, fo, $nn_seed) * Math.sqrt(2.0 / fi), zeros(fi, fo), zeros(fi, fo)) end
def nn_b(d) P_.new(zeros(1, d), zeros(1, d), zeros(1, d)) end
def nn_g(d) P_.new(ones(1, d), zeros(1, d), zeros(1, d)) end
def nn_up(p, g, lr, wd, t, tmp) grad_clip_norm_scratch(g, 1.0, tmp); adamw_update(p.w, p.m, p.v, g, lr, 0.9, 0.999, 1e-8, wd, t) end

def nn_alloc_cache(n, h, nc, nb)
      blk_caches = nb.times.map {
            { ln1: zeros(n, h), a1: zeros(n, h), h1: zeros(n, h),
              ln2: zeros(n, h), a2: zeros(n, h), mk: zeros_u8(n * h),
              gw1: zeros(h, h), gb1: zeros(1, h), gw2: zeros(h, h), gb2: zeros(1, h),
              gg1: zeros(1, h), gn1: zeros(1, h), gg2: zeros(1, h), gn2: zeros(1, h) }
      }
      chain = (nb + 1).times.map { zeros(n, h) }
      { blks: blk_caches, chain: chain, gr_a: zeros(n, h), gr_b: zeros(n, h), ce_grad: zeros(n, nc) }
end

def nn_fwd_into(inp, b, n, h, ds, sc, out)
      sc[:inp] = inp
      layernorm_into!(inp, b[:g1].w, b[:n1].w, sc[:ln1])
      gelu_into!(sc[:ln1], sc[:a1])
      linear_into!(sc[:a1], b[:w1].w, b[:b1].w, sc[:h1])
      layernorm_into!(sc[:h1], b[:g2].w, b[:n2].w, sc[:ln2])
      gelu_into!(sc[:ln2], sc[:a2])
      bernoulli_u8_into!(sc[:mk], ds, b[:drop])
      dropout_u8_into!(sc[:a2], sc[:mk], b[:drop], sc[:a2])
      linear_into!(sc[:a2], b[:w2].w, b[:b2].w, out)
      add_inplace!(out, inp)
end

def nn_bwd_into(g, b, sc, dst, src)
      sk = g
      linear_backward_full_into!(g, sc[:a2], b[:w2].w, dst, sc[:gw2], sc[:gb2])
      dropout_u8_into!(dst, sc[:mk], b[:drop], dst)
      gelu_backward_into!(dst, sc[:ln2], src); g = src
      layernorm_backward_full_into!(g, sc[:h1], b[:g2].w, LN_EPS, dst, sc[:gg2], sc[:gn2])
      linear_backward_full_into!(dst, sc[:a1], b[:w1].w, src, sc[:gw1], sc[:gb1])
      gelu_backward_into!(src, sc[:ln1], dst); g = dst
      layernorm_backward_full_into!(g, sc[:inp], b[:g1].w, LN_EPS, src, sc[:gg1], sc[:gn1])
      add_inplace!(src, sk)
      [src, {w1: sc[:gw1], b1: sc[:gb1], w2: sc[:gw2], b2: sc[:gb2], g1: sc[:gg1], n1: sc[:gn1], g2: sc[:gg2], n2: sc[:gn2]}]
end

def nn_alloc_eval(n, h, nc, nb)
      { chain: (nb + 1).times.map { zeros(n, h) }, s1: zeros(n, h), s2: zeros(n, h), out: zeros(n, nc) }
end

def nn_predict_into(g_x, pW, pB, hW, hB, blks, n, h, ev)
      linear_into!(g_x, pW.w, pB.w, ev[:chain][0])
      blks.each_with_index do |b, i|
            layernorm_into!(ev[:chain][i], b[:g1].w, b[:n1].w, ev[:s1])
            gelu_into!(ev[:s1], ev[:s1])
            linear_into!(ev[:s1], b[:w1].w, b[:b1].w, ev[:s2])
            layernorm_into!(ev[:s2], b[:g2].w, b[:n2].w, ev[:s1])
            gelu_into!(ev[:s1], ev[:s1])
            linear_into!(ev[:s1], b[:w2].w, b[:b2].w, ev[:chain][i+1])
            add_inplace!(ev[:chain][i+1], ev[:chain][i])
      end
      linear_into!(ev[:chain][blks.size], hW.w, hB.w, ev[:out])
      ev[:out]
end

BK = %i[w1 b1 w2 b2 g1 n1 g2 n2]

# ── NN preprocessing ──────────────────────────────────────────────────────
def variance_threshold(flat, n, nf, thr)
      m = Array.new(nf, 0.0); n.times { |i| nf.times { |j| m[j] += flat[i * nf + j] } }; m.map! { |s| s / n }
      v = Array.new(nf, 0.0); n.times { |i| nf.times { |j| v[j] += (flat[i * nf + j] - m[j])**2 } }; v.map! { |s| s / n }
      (0...nf).select { |j| v[j] > thr }
end

def select_k_best(flat, n, nf, tgt, k)
      ov = Array.new(nf, 0.0); n.times { |i| nf.times { |j| ov[j] += flat[i * nf + j] } }; ov.map! { |s| s / n }
      cm = Array.new(NC) { Array.new(nf, 0.0) }; cc = Array.new(NC, 0)
      n.times { |i| c = tgt[i]; cc[c] += 1; nf.times { |j| cm[c][j] += flat[i * nf + j] } }
      NC.times { |c| nf.times { |j| cm[c][j] /= [cc[c], 1].max } }
      fs = Array.new(nf) { |j| ssb = NC.times.sum { |c| cc[c] * (cm[c][j] - ov[j])**2 }; ssw = 0.0; n.times { |i| ssw += (flat[i * nf + j] - cm[tgt[i]][j])**2 }; ssb / [NC - 1, 1].max / ([ssw, 1e-30].max / [n - NC, 1].max) }
      fs.each_with_index.sort_by { |f, _| -f }.first(k).map { |_, i| i }.sort
end

def sel_cols(flat, n, nf, cols)
      nc = cols.size; out = Array.new(n * nc)
      n.times { |i| cols.each_with_index { |j, ci| out[i * nc + ci] = flat[i * nf + j] } }
      [out, nc]
end

# ── NN training ───────────────────────────────────────────────────────────
def train_nn(raw_tr, raw_val, n_tr, n_val, nf_raw, tgt_cpu, val_tgt, fold_cw, cfg, label, max_epochs: nil)
      h = cfg[:h]; nb = cfg[:blocks]; t0 = Time.now

      fs_cache = "#{CACHE}/nn_feat_sel_#{label.tr(' #', '_').downcase}_#{DATA_HASH}.json"
      if File.exist?(fs_cache)
            cached = JSON.parse(File.read(fs_cache), symbolize_names: true)
            vt = cached[:vt]; fk = cached[:fk]; nv = cached[:nv]
      else
            vt = variance_threshold(raw_tr, n_tr, nf_raw, 1e-6)
            rt, nv = sel_cols(raw_tr, n_tr, nf_raw, vt)
            fk = select_k_best(rt, n_tr, nv, tgt_cpu, [cfg[:k_best], nv].min); rt = nil
            File.write(fs_cache, JSON.generate({ vt: vt, fk: fk, nv: nv }))
      end
      str, ns = sel_cols(sel_cols(raw_tr, n_tr, nf_raw, vt)[0], n_tr, vt.size, fk)
      rvt, _ = sel_cols(raw_val, n_val, nf_raw, vt)
      sval, _ = sel_cols(rvt, n_val, vt.size, fk); rvt = nil
      $stderr.puts "    #{label}: #{ns} features"

      g_tr = upload(str, n_tr, ns); str = nil
      g_val = upload(sval, n_val, ns); sval = nil
      mu = mean(g_tr); sd = sqrt(clamp(var(g_tr), 1e-12, 1e30))
      g_tr = (g_tr - mu) / sd
      g_val = (g_val - mu) / sd

      g_tgt = upload(tgt_cpu.map(&:to_f), n_tr, 1)
      g_sw = upload(tgt_cpu.map { |t| fold_cw[t] }, n_tr, 1)

      $nn_seed = cfg[:seed] * 100
      pW = nn_p(ns, h); pB = nn_b(h); hW = nn_p(h, NC); hB = nn_b(NC)
      blks = nb.times.map { {w1: nn_p(h,h), b1: nn_b(h), w2: nn_p(h,h), b2: nn_b(h),
                              g1: nn_g(h), n1: nn_b(h), g2: nn_g(h), n2: nn_b(h), drop: cfg[:drop]} }

      best_ba = 0; stale = 0; best_ep = 0; best_vl = nil
      sc = nn_alloc_cache(n_tr, h, NC, nb)
      logits_buf = zeros(n_tr, NC)
      ghw_buf = zeros(h, NC); ghb_buf = zeros(1, NC)
      gpw_buf = zeros(ns, h); gpb_buf = zeros(1, h)
      clip_tmp = zeros(1, 1)
      ev = nn_alloc_eval(n_val, h, NC, nb)

      epochs = max_epochs || cfg[:epochs]
      for ep in 0...epochs
            alloc_count_reset
            vf0, vt0 = gpu_stats; v0 = vt0 - vf0
            rss0 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
            t_ep = Time.now

            linear_into!(g_tr, pW.w, pB.w, sc[:chain][0])
            blks.each_with_index { |b, i| nn_fwd_into(sc[:chain][i], b, n_tr, h, cfg[:seed] * 10000 + ep * 100 + i, sc[:blks][i], sc[:chain][i+1]) }
            hpre = sc[:chain][nb]
            linear_into!(hpre, hW.w, hB.w, logits_buf)

            softmax_ce_grad_into!(logits_buf, g_tgt, g_sw, sc[:ce_grad], 1.0 / n_tr)

            linear_backward_full_into!(sc[:ce_grad], hpre, hW.w, sc[:gr_a], ghw_buf, ghb_buf)
            gr = sc[:gr_a]
            bg = []; (nb-1).downto(0) { |i| gr, bgs = nn_bwd_into(gr, blks[i], sc[:blks][i], sc[:gr_b], sc[:gr_a]); bg.unshift(bgs) }
            linear_backward_weights_only_into!(gr, g_tr, gpw_buf, gpb_buf)

            nn_up(pW, gpw_buf, cfg[:lr], cfg[:wd], ep+1, clip_tmp); nn_up(pB, gpb_buf, cfg[:lr], cfg[:wd], ep+1, clip_tmp)
            nn_up(hW, ghw_buf, cfg[:lr], cfg[:wd], ep+1, clip_tmp); nn_up(hB, ghb_buf, cfg[:lr], cfg[:wd], ep+1, clip_tmp)
            blks.each_with_index { |b, i| BK.each { |k| nn_up(b[k], bg[i][k], cfg[:lr], cfg[:wd], ep+1, clip_tmp) } }

            ep_allocs = alloc_count_reset
            vf1, vt1 = gpu_stats; v1 = vt1 - vf1
            rss1 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
            ep_ms = (Time.now - t_ep) * 1000

            if val_tgt
                  vl = nn_predict_into(g_val, pW, pB, hW, hB, blks, n_val, h, ev)
                  ba = report(vl, val_tgt, ep)
                  if ep_allocs > 0 || v1 - v0 != 0
                        $stderr.puts "      ep=%d total %.0fms  vram %d->%d (%+d)  ram %d->%d  allocs=%d" % [ep, ep_ms, v0, v1, v1 - v0, rss0, rss1, ep_allocs]
                  end
                  if ba > best_ba; best_ba = ba; stale = 0; best_ep = ep + 1; best_vl = copy(vl)
                  else stale += 1; break if stale >= NN_PATIENCE; end
            end
      end
      ep_used = val_tgt ? best_ep : epochs
      $stderr.puts "    #{label} (h=#{h} blk=#{nb} ep=#{ep_used}): %.1fs" % [Time.now - t0]

      probs_out = if val_tgt
            download(softmax(best_vl || vl))
      else
            vl = nn_predict_into(g_val, pW, pB, hW, hB, blks, n_val, h, ev)
            download(softmax(vl))
      end
      [probs_out, ep_used]
end

# ── XGBoost / LightGBM helpers ───────────────────────────────────────────
XGB_CFGS = [
      { max_depth: 6, eta: 0.03, num_round: 1200, lambda: 2.0, subsample: 0.7, colsample_bytree: 0.7, seed: 42 },
      #{ max_depth: 7, eta: 0.05, num_round: 800, lambda: 3.0, subsample: 0.75, colsample_bytree: 0.8, seed: 77 },
]
LGBM_CFGS = [
      { num_leaves: 63, learning_rate: 0.03, n_estimators: 1200, lambda_l2: 2.0, subsample: 0.7, colsample_bytree: 0.7, seed: 42 },
      #{ num_leaves: 127, learning_rate: 0.05, n_estimators: 800, lambda_l2: 3.0, subsample: 0.75, colsample_bytree: 0.8, seed: 99 },
]

def train_xgb(x_tr, x_te, n_tr, n_te, nf, tgt_cpu, val_tgt, cfg, label, max_rounds: nil)
      $stderr.puts "    #{label}..."
      alloc_count_reset
      vf0, vt0 = gpu_stats; v0 = vt0 - vf0
      rss0 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      t0 = Time.now; rounds = max_rounds || cfg[:num_round]
      dtrain = XGBoost::DMatrix.from_flat(x_tr, n_tr, nf, label: tgt_cpu)
      params = {
            objective: "multi:softprob", num_class: NC, eval_metric: "mlogloss",
            max_depth: cfg[:max_depth], eta: cfg[:eta], lambda: cfg[:lambda],
            subsample: cfg[:subsample], colsample_bytree: cfg[:colsample_bytree],
            seed: cfg[:seed], verbosity: 0,
      }
      if val_tgt
            dval = XGBoost::DMatrix.from_flat(x_te, n_te, nf, label: val_tgt)
            bst = XGBoost.train(params, dtrain, num_boost_round: rounds,
                  evals: [[dval, "val"]], early_stopping_rounds: PATIENCE, verbose_eval: 1)
      else
            bst = XGBoost.train(params, dtrain, num_boost_round: rounds)
      end
      best_r = bst.best_iteration rescue rounds
      dtest = val_tgt ? dval : XGBoost::DMatrix.from_flat(x_te, n_te, nf, label: Array.new(n_te, 0))
      probs = bst.predict(dtest)
      allocs = alloc_count_reset
      vf1, vt1 = gpu_stats; v1 = vt1 - vf1
      rss1 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      ms = (Time.now - t0) * 1000
      $stderr.puts "    #{label} (stopped=#{best_r}): %.1fs  vram %d->%d (%+d)  ram %d->%d  allocs=%d" % [ms / 1000, v0, v1, v1 - v0, rss0, rss1, allocs]
      [probs.flatten, best_r]
end

def train_lgbm(x_tr, x_te, n_tr, n_te, nf, tgt_cpu, val_tgt, cfg, label, max_rounds: nil)
      $stderr.puts "    #{label}..."
      alloc_count_reset
      vf0, vt0 = gpu_stats; v0 = vt0 - vf0
      rss0 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      t0 = Time.now; rounds = max_rounds || cfg[:n_estimators]
      train_set = LightGBM::Dataset.from_flat(x_tr, n_tr, nf, label: tgt_cpu)
      params = {
            objective: "multiclass", num_class: NC, metric: "multi_logloss",
            num_leaves: cfg[:num_leaves], learning_rate: cfg[:learning_rate],
            lambda_l2: cfg[:lambda_l2], subsample: cfg[:subsample],
            colsample_bytree: cfg[:colsample_bytree], seed: cfg[:seed], verbose: -1,
      }
      if val_tgt
            val_set = LightGBM::Dataset.from_flat(x_te, n_te, nf, label: val_tgt)
            bst = LightGBM.train(params, train_set, num_boost_round: rounds,
                  valid_sets: [val_set], early_stopping_rounds: PATIENCE, verbose_eval: 1)
      else
            bst = LightGBM.train(params, train_set, num_boost_round: rounds)
      end
      best_r = bst.best_iteration rescue rounds
      probs = bst.predict_flat(x_te, n_te, nf)
      allocs = alloc_count_reset
      vf1, vt1 = gpu_stats; v1 = vt1 - vf1
      rss1 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      ms = (Time.now - t0) * 1000
      $stderr.puts "    #{label} (stopped=#{best_r}): %.1fs  vram %d->%d (%+d)  ram %d->%d  allocs=%d" % [ms / 1000, v0, v1, v1 - v0, rss0, rss1, allocs]
      [probs.flatten, best_r]
end

# ── CatBoost (via catboost-rs) ────────────────────────────────────────────
CB_CFGS = [
      { iterations: 1000, depth: 6, lr: 0.03, l2_reg: 3.0, n_permutations: 4, seed: 42 },
      #{ iterations: 800, depth: 8, lr: 0.05, l2_reg: 1.0, n_permutations: 2, seed: 77 },
]

def train_cb_gpu(bins_tr, bins_te, n_tr, n_te, nf, tgt_cpu, val_tgt, cfg, label, max_rounds: nil)
      $stderr.puts "    #{label}..."
      alloc_count_reset
      vf0, vt0 = gpu_stats; v0 = vt0 - vf0
      rss0 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      t0 = Time.now
      rounds = max_rounds || cfg[:iterations]
      depth = cfg[:depth]; lr = cfg[:lr]; lam = cfg[:l2_reg].to_f

      bins_u8_tr = bins_tr.map { |v| v.to_i.clamp(0, 255) }
      bins_u8_te = bins_te.map { |v| v.to_i.clamp(0, 255) }
      bins_rm = upload_u8(bins_u8_tr, n_tr, nf)
      bins_fm_data = Array.new(nf * n_tr)
      nf.times { |f| n_tr.times { |i| bins_fm_data[f * n_tr + i] = bins_u8_tr[i * nf + f] } }
      bins_fm = upload_u8(bins_fm_data, nf, n_tr)
      te_bins_rm = upload_u8(bins_u8_te, n_te, nf)

      n_leaves = 1 << depth; max_nodes = n_leaves
      preds = Array.new(NC) { zeros_f32(n_tr, 1) }
      te_preds = Array.new(NC) { zeros_f32(n_te, 1) }
      grad = zeros_f32(n_tr, 1); hess = zeros_f32(n_tr, 1)
      node_a = zeros_u8(n_tr); node_b = zeros_u8(n_tr)
      leaf_idx = zeros_u8(n_tr); te_leaf = zeros_u8(n_te)
      grad_hist = zeros_f32(max_nodes * nf, N_BINS)
      hess_hist = zeros_f32(max_nodes * nf, N_BINS)
      gain_buf = zeros_f32(nf, N_BINS)
      leaf_g = zeros_f32(n_leaves, 1); leaf_h = zeros_f32(n_leaves, 1); leaf_v = zeros_f32(n_leaves, 1)
      tgt_f32 = upload_f32(tgt_cpu.map(&:to_f), n_tr, 1)

      trees = []
      for t in 0...rounds
            NC.times do |k|
                  softmax_ce_class_grad_f32!(preds, tgt_f32, grad, hess, k, n_tr)

                  zero!(node_a)
                  sf_host = []; sb_host = []

                  depth.times do |d|
                        n_nodes = 1 << d
                        zero!(grad_hist); zero!(hess_hist)
                        oblivious_histogram_into!(bins_fm, node_a, grad, hess, grad_hist, hess_hist, N_BINS, n_nodes)
                        zero!(gain_buf)
                        oblivious_split_eval_into!(grad_hist, hess_hist, gain_buf, n_nodes, N_BINS, lam)
                        gains = download_f32(gain_buf)
                        best_idx = gains.each_with_index.max_by { |v, _| v }[1]
                        best_feat = best_idx / N_BINS; best_bin = best_idx % N_BINS
                        sf_host << best_feat; sb_host << best_bin
                        oblivious_route_step_into!(bins_rm, node_a, node_b, best_feat, best_bin, d)
                        node_a, node_b = node_b, node_a
                  end

                  sf_dev = upload_i32(sf_host, depth, 1)
                  sb_dev = upload_u8(sb_host.map { |v| v & 0xff }, depth, 1)

                  oblivious_route_full_into!(bins_rm, sf_dev, sb_dev, leaf_idx, depth)
                  zero!(leaf_g); zero!(leaf_h)
                  leaf_reduce_into!(leaf_idx, grad, hess, leaf_g, leaf_h)
                  leaf_finalize_into!(leaf_g, leaf_h, leaf_v, lam)
                  scatter_add_by_leaf!(preds[k], leaf_idx, leaf_v, lr)

                  oblivious_route_full_into!(te_bins_rm, sf_dev, sb_dev, te_leaf, depth)
                  scatter_add_by_leaf!(te_preds[k], te_leaf, leaf_v, lr)

                  trees << { sf: sf_host.dup, sb: sb_host.dup, lv: download_f32(leaf_v), k: k }
            end

            if val_tgt && (t + 1) % 10 == 0
                  te_probs = n_te.times.map { |i|
                        ls = NC.times.map { |c| download_f32(te_preds[c])[i] }
                        mx = ls.max; es = ls.map { |v| Math.exp(v - mx) }; s = es.sum
                        es.map { |v| v / s }
                  }
                  preds_i = te_probs.map { |p| p.each_with_index.max_by { |v, _| v }[1] }
                  ba = bal_acc(preds_i, val_tgt)
                  $stderr.puts "      cb_gpu t=%d val=%.4f" % [t + 1, ba]
            end
            $stderr.puts "      cb_gpu iter=%d/%d" % [t + 1, rounds] if (t + 1) % 50 == 0
      end

      te_probs_flat = Array.new(n_te * NC, 0.0)
      n_te.times do |i|
            ls = NC.times.map { |c| download_f32(te_preds[c])[i] }
            mx = ls.max; es = ls.map { |v| Math.exp(v - mx) }; s = es.sum
            NC.times { |c| te_probs_flat[i * NC + c] = es[c] / s }
      end

      allocs = alloc_count_reset
      vf1, vt1 = gpu_stats; v1 = vt1 - vf1
      rss1 = File.read("/proc/self/status")[/VmRSS:\s+(\d+)/, 1].to_i / 1024
      ms = (Time.now - t0) * 1000
      $stderr.puts "    #{label} (iters=#{rounds}): %.1fs  vram %d->%d (%+d)  ram %d->%d  allocs=%d" % [ms / 1000, v0, v1, v1 - v0, rss0, rss1, allocs]
      [te_probs_flat, rounds]
end

# ── CPU balanced accuracy (for OOF/threshold, not training) ───────────────
def bal_acc(preds, tgt)
      c = Array.new(NC, 0.0); t = Array.new(NC, 0.0)
      preds.each_with_index { |p, i| cl = tgt[i]; t[cl] += 1; c[cl] += 1 if p.to_i == cl }
      (0...NC).sum { |k| t[k] > 0 ? c[k] / t[k] : 0.0 } / NC
end

def cache_path(label, fold, cfg = nil)
      tag = cfg ? "_" + Digest::MD5.hexdigest(cfg.sort.to_s + DATA_HASH)[0..7] : ""
      "#{CACHE}/#{label.tr(' #', '_').downcase}_f#{fold}#{tag}"
end

def cache_save(path, probs, round, val)
      File.open("#{path}.bin", "wb") { |f| f.write(probs.pack("E*")) }
      File.write("#{path}.json", JSON.generate({ round: round, val: val, n: probs.size / NC, nc: NC, len: probs.size }))
end

def cache_load(path, expected_len)
      return nil unless File.exist?("#{path}.bin") && File.exist?("#{path}.json")
      meta = JSON.parse(File.read("#{path}.json"), symbolize_names: true)
      return nil unless meta[:len] == expected_len && meta[:nc] == NC
      probs = File.binread("#{path}.bin").unpack("E*")
      return nil unless probs.size == expected_len
      [probs, meta[:round], meta[:val]]
end

def with_cache(label, fold, fvc, cfg: nil)
      cp = cache_path(label, fold, cfg)
      cached = cache_load(cp, fvc.size * NC)
      if cached
            probs, br, cv = cached
            $stderr.puts "    #{label} (cached val=%.4f round=%d)" % [cv, br]
            $model_stats << { label: label, fold: fold, runtime: 0.0, val: cv, round: br, cached: true }
            return [probs, br]
      end
      t_m = Time.now
      probs, br = yield
      pred_m = fvc.size.times.map { |i| NC.times.max_by { |k| probs[i * NC + k] } }
      cv = bal_acc(pred_m, fvc)
      $model_stats << { label: label, fold: fold, runtime: Time.now - t_m, val: cv, round: br, cached: false }
      cache_save(cp, probs, br, cv)
      [probs, br]
end

def print_summary(stats, oof_raw, oof_tuned, scales, phase_time)
      by_label = stats.group_by { |s| s[:label] }
      rows = by_label.map do |label, entries|
            n = entries.size
            computed = entries.reject { |e| e[:cached] }
            { label: label,
              rt: computed.empty? ? 0.0 : computed.sum { |e| e[:runtime] } / computed.size,
              val: entries.sum { |e| e[:val] } / n,
              rnd: entries.sum { |e| e[:round].to_f } / n,
              cached: entries.all? { |e| e[:cached] } }
      end.sort_by { |r| -r[:val] }

      families = stats.group_by { |s| s[:label].sub(/ #\d+$/, "") }
      fam_rows = families.map do |fam, entries|
            { fam: fam, n: entries.size,
              total: entries.sum { |e| e[:runtime] },
              mean: entries.sum { |e| e[:runtime] } / entries.size }
      end.sort_by { |r| -r[:total] }

      $stderr.puts "\n# ── model performance (mean across #{K_FOLDS} folds) #{"─" * 30}"
      $stderr.puts "  %-18s %10s %10s %12s" % %w[label runtime best_val best_round]
      $stderr.puts "  #{"─" * 52}"
      rows.each { |r| $stderr.puts "  %-18s %9s %10.4f %12.1f" % [r[:label], r[:cached] ? "cached" : "%.1fs" % r[:rt], r[:val], r[:rnd]] }
      $stderr.puts "\n# ── family runtime #{"─" * 48}"
      $stderr.puts "  %-12s %8s %13s %11s" % %w[family models total_rt mean_rt]
      $stderr.puts "  #{"─" * 46}"
      fam_rows.each { |r| $stderr.puts "  %-12s %8d %12.1fs %10.1fs" % [r[:fam], r[:n], r[:total], r[:mean]] }
      $stderr.puts "\n# ── OOF #{"─" * 59}"
      $stderr.puts "  raw=%.5f  tuned=%.5f  scales=[%.2f, %.2f, %.2f]" % [oof_raw, oof_tuned, *scales]
      $stderr.puts "  phase 1 total: %.1fs" % phase_time
end

# ══════════════════════════════════════════════════════════════════════════
# Phase 1: K-fold OOF
# ══════════════════════════════════════════════════════════════════════════
zero_tail = Array.new(N_CAT + N_TE_FEAT, 0.0).freeze
orig_raw = Array.new(n_orig * NF, 0.0)
n_orig.times do |oi|
      row = orig_num[oi] + zero_tail + orig_eng[oi]
      NF.times { |j| orig_raw[oi * NF + j] = row[j] }
end
orig_raw.freeze

$stderr.puts "\n# ── phase 1: #{K_FOLDS}-fold OOF #{"─" * 43}"
oof = Array.new(n_tr * NC, 0.0)
best_rounds = Array.new(GBM_CFGS.size) { [] }
nn_best_epochs = Array.new(NN_CFGS.size) { [] }
xgb_best_rounds = Array.new(XGB_CFGS.size) { [] }
lgbm_best_rounds = Array.new(LGBM_CFGS.size) { [] }
cb_best_rounds = Array.new(CB_CFGS.size) { [] }
t_phase1 = Time.now

K_FOLDS.times do |fold|
      vi = folds[fold]; ti = (0...K_FOLDS).reject { |k| k == fold }.flat_map { |k| folds[k] }
      nft = ti.size; nfv = vi.size

      fc = Array.new(NC, 0); ti.each { |i| fc[targets_i[i]] += 1 }
      fp = fc.map { |c| c.to_f / nft }; fw = fc.map { |c| nft.to_f / (NC * c) }

      $stderr.puts "  Fold #{fold+1}/#{K_FOLDS}: prep..."
      ts = compute_te_stats(ti, targets_i, tr_f, cat_ci, CROSS_PAIRS_CI, fp)
      tb, bd, raw_t, ce = build_and_bin(ti, num_tr, eng_tr, tr_f, ts, cat_ci, CROSS_PAIRS_CI, fp, nil, nil)
      vb, _, raw_v, _   = build_and_bin(vi, num_tr, eng_tr, tr_f, ts, cat_ci, CROSS_PAIRS_CI, fp, bd, ce)

      ft = ti.map { |i| targets_f[i] }; fsw = ti.map { |i| fw[targets_i[i]] }
      ftc = ti.map { |i| targets_i[i] }; fvc = vi.map { |i| targets_i[i] }

      mem_report("F#{fold+1} before upload")
      g_tr = upload(tb, nft, NF); tb = nil
      g_te = upload(vb, nfv, NF); vb = nil
      g_tgt = upload(ft, nft, 1); ft = nil
      g_sw = upload(fsw, nft, 1); fsw = nil
      mem_report("F#{fold+1} after upload")

      nm = GBM_CFGS.size + NN_CFGS.size + XGB_CFGS.size + LGBM_CFGS.size + CB_CFGS.size
      $stderr.puts "  Fold #{fold+1}/#{K_FOLDS} (#{nft}/#{nfv}, #{nm} models)"
      fp_acc = Array.new(nfv * NC, 0.0)

      GBM_CFGS.each_with_index do |cfg, bag|
            probs, br = with_cache("GPU-GBM ##{bag+1}", fold+1, fvc, cfg: cfg) {
                  train_gbm(g_tr, g_te, g_tgt, g_sw, cfg, "F#{fold+1} GPU-GBM ##{bag+1}", fvc)
            }
            best_rounds[bag] << br
            (nfv * NC).times { |i| fp_acc[i] += probs[i] }
      end
      mem_report("F#{fold+1} after GBM")
      nn_raw_t = orig_raw + raw_t.to_a
      nn_ftc = orig_tgt_i + ftc
      nn_nft = n_orig + nft

      NN_CFGS.each_with_index do |cfg, ni|
            probs, ep = with_cache("ResNet ##{ni+1}", fold+1, fvc, cfg: cfg) {
                  train_nn(nn_raw_t, raw_v, nn_nft, nfv, NF, nn_ftc, fvc, fw, cfg, "F#{fold+1} ResNet ##{ni+1}")
            }
            nn_best_epochs[ni] << ep
            (nfv * NC).times { |i| fp_acc[i] += probs[i] }
      end
      mem_report("F#{fold+1} after NN")
      g_tr = nil; g_te = nil; g_tgt = nil; g_sw = nil
      GC.start

      XGB_CFGS.each_with_index do |cfg, xi|
            probs, br = with_cache("XGBoost ##{xi+1}", fold+1, fvc, cfg: cfg) {
                  train_xgb(raw_t, raw_v, nft, nfv, NF, ftc, fvc, cfg, "F#{fold+1} XGBoost ##{xi+1}")
            }
            xgb_best_rounds[xi] << br
            (nfv * NC).times { |i| fp_acc[i] += probs[i] }
      end

      LGBM_CFGS.each_with_index do |cfg, li|
            probs, br = with_cache("LightGBM ##{li+1}", fold+1, fvc, cfg: cfg) {
                  train_lgbm(raw_t, raw_v, nft, nfv, NF, ftc, fvc, cfg, "F#{fold+1} LightGBM ##{li+1}")
            }
            lgbm_best_rounds[li] << br
            (nfv * NC).times { |i| fp_acc[i] += probs[i] }
      end

      CB_CFGS.each_with_index do |cfg, ci|
            probs, br = with_cache("CatBoost ##{ci+1}", fold+1, fvc, cfg: cfg) {
                  train_cb_gpu(raw_t, raw_v, nft, nfv, NF, ftc, fvc, cfg, "F#{fold+1} CatBoost ##{ci+1}")
            }
            cb_best_rounds[ci] << br
            (nfv * NC).times { |i| fp_acc[i] += probs[i] }
      end

      mem_report("F#{fold+1} after CB")
      raw_t = nil; raw_v = nil
      fp_acc.map! { |v| v / nm }

      vi.each_with_index { |orig, li| NC.times { |k| oof[orig * NC + k] = fp_acc[li * NC + k] } }
      preds = nfv.times.map { |i| NC.times.max_by { |k| fp_acc[i * NC + k] } }
      mem_report("F#{fold+1} end")
      $stderr.puts "  Fold #{fold+1} val=%.5f\n" % bal_acc(preds, fvc)
end
$stderr.puts "Phase 1: %.1fs" % [Time.now - t_phase1]

# ══════════════════════════════════════════════════════════════════════════
# OOF metrics + threshold tuning
# ══════════════════════════════════════════════════════════════════════════
oof_preds = n_tr.times.map { |i| NC.times.max_by { |k| oof[i * NC + k] } }
oof_raw_ba = bal_acc(oof_preds, targets_i)
$stderr.puts "\nOOF raw bal_acc=%.5f" % oof_raw_ba

best_ba = 0.0; best_s = [1.0, 1.0, 1.0]
(5..60).each { |sh| (3..25).each { |sm| s = [sh/10.0, 1.0, sm/10.0]; preds = n_tr.times.map { |i| NC.times.max_by { |k| oof[i*NC+k]*s[k] } }; ba = bal_acc(preds, targets_i); if ba > best_ba; best_ba = ba; best_s = s.dup; end } }
$stderr.puts "OOF tuned=%.5f (optimistic)  scales=%.2f,%.2f,%.2f" % [best_ba, *best_s]
print_summary($model_stats, oof_raw_ba, best_ba, best_s, Time.now - t_phase1)

# ══════════════════════════════════════════════════════════════════════════
# Phase 2: Full retrain
# ══════════════════════════════════════════════════════════════════════════
p2r = best_rounds.map { |rs| rs.sort[rs.size / 2] }
p2r_nn = nn_best_epochs.map { |es| es.empty? ? 300 : es.sort[es.size / 2] }
p2r_xgb = xgb_best_rounds.map { |rs| rs.sort[rs.size / 2] }
p2r_lgbm = lgbm_best_rounds.map { |rs| rs.sort[rs.size / 2] }
p2r_cb = cb_best_rounds.map { |rs| rs.sort[rs.size / 2] }
$stderr.puts "\nPhase 2 caps:"
$stderr.puts "  GPU-GBM: #{GBM_CFGS.zip(p2r).map { |c, r| "d#{c[:depth]}=#{r}" }.join(', ')}"
$stderr.puts "  ResNet: #{NN_CFGS.zip(p2r_nn).map { |c, r| "h#{c[:h]}=#{r}" }.join(', ')}"
$stderr.puts "  XGBoost: #{XGB_CFGS.zip(p2r_xgb).map { |c, r| "d#{c[:max_depth]}=#{r}" }.join(', ')}"
$stderr.puts "  LightGBM: #{LGBM_CFGS.zip(p2r_lgbm).map { |c, r| "l#{c[:num_leaves]}=#{r}" }.join(', ')}"
$stderr.puts "  CatBoost: #{CB_CFGS.zip(p2r_cb).map { |c, r| "d#{c[:depth]}=#{r}" }.join(', ')}"
$stderr.puts "# ── phase 2: full retrain #{"─" * 42}"
t_p2 = Time.now

ac = Array.new(NC, 0); targets_i.each { |t| ac[t] += 1 }
ap = ac.map { |c| c.to_f / n_tr }; aw = ac.map { |c| n_tr.to_f / (NC * c) }

ts = compute_te_stats((0...n_tr).to_a, targets_i, tr_f, cat_ci, CROSS_PAIRS_CI, ap)
atb, abd, raw_ft, ace = build_and_bin((0...n_tr).to_a, num_tr, eng_tr, tr_f, ts, cat_ci, CROSS_PAIRS_CI, ap, nil, nil)

sf, cf = ts
ateb = Array.new(n_te * NF); raw_fte = Array.new(n_te * NF)
n_te.times do |i|
      te = apply_te(te_f[i], sf, cf, cat_ci, CROSS_PAIRS_CI, ap)
      cv = cat_ci.map { |ci| (ace[ci][te_f[i][ci]] || ace[ci].size).to_f }
      row = num_te[i] + cv + te + eng_te[i]
      NF.times { |j| raw_fte[i*NF+j] = row[j]; b = abd[j]; ateb[i*NF+j] = (b.bsearch_index { |t| t > row[j] } || b.size).to_f }
end

fsw = targets_i.map { |t| aw[t] }
g_tr = upload(atb, n_tr, NF); atb = nil
g_te = upload(ateb, n_te, NF); ateb = nil
g_tgt = upload(targets_f, n_tr, 1)
g_sw = upload(fsw, n_tr, 1); fsw = nil

nm = GBM_CFGS.size + NN_CFGS.size + XGB_CFGS.size + LGBM_CFGS.size + CB_CFGS.size
tp = Array.new(n_te * NC, 0.0)

GBM_CFGS.each_with_index do |cfg, bag|
      probs, _ = train_gbm(g_tr, g_te, g_tgt, g_sw, cfg, "Full GPU-GBM ##{bag+1}", nil, max_rounds: p2r[bag])
      (n_te * NC).times { |i| tp[i] += probs[i] }
end

nn_raw_ft = orig_raw + raw_ft.to_a
nn_targets_i = orig_tgt_i + targets_i.to_a
nn_n_tr = n_orig + n_tr

NN_CFGS.each_with_index do |cfg, ni|
      probs, _ = train_nn(nn_raw_ft, raw_fte, nn_n_tr, n_te, NF, nn_targets_i, nil, aw, cfg, "Full ResNet ##{ni+1}", max_epochs: p2r_nn[ni])
      (n_te * NC).times { |i| tp[i] += probs[i] }
end

XGB_CFGS.each_with_index do |cfg, xi|
      probs, _ = train_xgb(raw_ft, raw_fte, n_tr, n_te, NF, targets_i, nil, cfg, "Full XGBoost ##{xi+1}", max_rounds: p2r_xgb[xi])
      (n_te * NC).times { |i| tp[i] += probs[i] }
end

LGBM_CFGS.each_with_index do |cfg, li|
      probs, _ = train_lgbm(raw_ft, raw_fte, n_tr, n_te, NF, targets_i, nil, cfg, "Full LightGBM ##{li+1}", max_rounds: p2r_lgbm[li])
      (n_te * NC).times { |i| tp[i] += probs[i] }
end

CB_CFGS.each_with_index do |cfg, ci|
      probs, _ = train_cb_gpu(raw_ft, raw_fte, n_tr, n_te, NF, targets_i, nil, cfg, "Full CatBoost ##{ci+1}", max_rounds: p2r_cb[ci])
      (n_te * NC).times { |i| tp[i] += probs[i] }
end

raw_ft = nil; raw_fte = nil
tp.map! { |v| v / nm }
$stderr.puts "Phase 2: %.1fs" % [Time.now - t_p2]

# ══════════════════════════════════════════════════════════════════════════
# Submission + artifacts
# ══════════════════════════════════════════════════════════════════════════
File.open("#{DIR}/submission.csv", "w") do |f|
      f.puts "id,Irrigation_Need"
      n_te.times { |i| pred = NC.times.max_by { |k| tp[i*NC+k] * best_s[k] }; f.puts "#{test_ids[i]},#{CLASSES[pred]}" }
end

dist = Hash.new(0); File.readlines("#{DIR}/submission.csv").drop(1).each { |l| dist[l.chomp.split(',')[1]] += 1 }
$stderr.puts "Submission: #{dist.sort.map { |k,v| "#{k}=#{v}" }.join("  ")}"

ts = Time.now.strftime("%Y%m%d_%H%M%S")
File.open("#{ART}/oof_probs_#{ts}.csv", "w") { |f| f.puts "idx,p_High,p_Low,p_Medium,true"; n_tr.times { |i| f.puts "%d,%.6f,%.6f,%.6f,%s" % [i, oof[i*NC], oof[i*NC+1], oof[i*NC+2], CLASSES[targets_i[i]]] } }
File.open("#{ART}/test_probs_#{ts}.csv", "w") { |f| f.puts "id,p_High,p_Low,p_Medium"; n_te.times { |i| f.puts "%s,%.6f,%.6f,%.6f" % [test_ids[i], tp[i*NC], tp[i*NC+1], tp[i*NC+2]] } }
File.write("#{ART}/run_#{ts}.json", JSON.pretty_generate({
      timestamp: ts, n_train: n_tr, n_test: n_te, n_features: NF, k_folds: K_FOLDS,
      gbm_configs: GBM_CFGS, nn_configs: NN_CFGS, gbm_best_rounds: best_rounds,
      p2_round_caps: p2r, oof_raw_ba: bal_acc(oof_preds, targets_i),
      oof_tuned_ba: best_ba, threshold_scales: best_s, submission_dist: dist,
}))
$stderr.puts "Done!"

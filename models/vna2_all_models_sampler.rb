require 'csv'
require 'set'
require 'nates_gpu'

SEED = 20_260_413
PHASE1_SAMPLES = 50
PHASE2_SAMPLES = 100
TOPK_PRINT = 10
MIN_COMPOSE = 3
MAX_COMPOSE = 6
EPS = 1e-8

Param = Struct.new(:w, :m, :v)
TreeNode = Struct.new(:feature, :bin, :left_value, :right_value)

MODEL_SPECS = [
  ["Linear Regression", :linear, :native],
  ["Ridge Regression", :ridge, :native],
  ["Lasso Regression", :lasso, :native],
  ["Elastic Net Regression", :elastic_net, :native],
  ["Logistic Regression", :logistic_proxy, :proxy],
  ["Decision Trees", :decision_tree, :native],
  ["Random Forest", :random_forest, :native],
  ["Gradient Boosting Machines (GBM)", :gbm, :native],
  ["XGBoost", :xgb, :native],
  ["LightGBM", :lgbm, :native],
  ["CatBoost", :catboost, :native],
  ["Support Vector Machines (SVM)", :kernel_proxy, :proxy],
  ["K-Nearest Neighbors (KNN)", :knn, :native],
  ["Principal Component Analysis (PCA)", :factor_proxy, :proxy],
  ["Independent Component Analysis (ICA)", :ica_proxy, :proxy],
  ["Non-Negative Matrix Factorization (NMF)", :nmf_proxy, :proxy],
  ["Gaussian Mixture Models (GMM)", :gmm_proxy, :proxy],
  ["Hidden Markov Models (HMM)", :hmm_proxy, :proxy],
  ["Neural Networks (Feedforward, Convolutional, Recurrent)", :mlp, :native],
  ["Long Short-Term Memory (LSTM)", :lstm_proxy, :proxy],
  ["Gated Recurrent Units (GRU)", :gru_proxy, :proxy],
  ["Autoencoders", :autoencoder_proxy, :proxy],
  ["Variational Autoencoders (VAE)", :vae_proxy, :proxy],
  ["Generative Adversarial Networks (GAN)", :gan_proxy, :proxy],
  ["Deep Q-Networks (DQN)", :rl_proxy, :proxy],
  ["Actor-Critic Models", :rl_proxy, :proxy],
  ["Temporal Difference Learning", :rl_proxy, :proxy],
  ["Gaussian Process Models", :kernel_proxy, :proxy],
  ["Kernel Methods", :kernel_proxy, :proxy],
  ["Multilayer Perceptrons (MLP)", :mlp, :native],
  ["Word Embeddings (Word2Vec, GloVe)", :embedding_proxy, :proxy],
  ["Transformer Models (BERT, GPT, T5)", :transformer_proxy, :proxy],
  ["Sequence-to-Sequence Models", :seq_proxy, :proxy],
  ["Hierarchical Models", :hierarchical_proxy, :proxy],
  ["Dynamic Time Warping (DTW)", :dtw_proxy, :proxy],
  ["Hierarchical Clustering", :cluster_proxy, :proxy],
  ["Mean Shift Clustering", :cluster_proxy, :proxy],
  ["DBSCAN (Density-Based Spatial Clustering of Applications with Noise)", :cluster_proxy, :proxy],
  ["Agglomerative Clustering", :cluster_proxy, :proxy],
  ["Self-Organizing Maps (SOM)", :cluster_proxy, :proxy],
  ["Isolation Forest", :anomaly_proxy, :proxy],
  ["One-Class SVM", :anomaly_proxy, :proxy],
  ["Anomaly Detection Models", :anomaly_proxy, :proxy],
  ["Time Series Models (ARIMA, SARIMA, Exponential Smoothing)", :seq_proxy, :proxy],
  ["Hidden Markov Models (HMM) for Time Series", :hmm_proxy, :proxy],
  ["Gaussian Process Regression", :kernel_proxy, :proxy],
  ["Bayesian Networks", :hierarchical_proxy, :proxy],
  ["Association Rule Learning (Apriori, FP-Growth)", :rule_proxy, :proxy],
  ["Markov Chains", :seq_proxy, :proxy],
  ["Reinforcement Learning Models (Q-Learning, SARSA, Policy Gradient)", :rl_proxy, :proxy]
].freeze

MODEL_INDEX = MODEL_SPECS.map { |name, family, adapter| [name, { family: family, adapter: adapter }] }.to_h

class SamplerRng
  def initialize(seed)
    @rng = Random.new(seed)
  end

  def int(lo, hi)
    @rng.rand(lo..hi)
  end

  def float(lo, hi)
    lo + @rng.rand * (hi - lo)
  end

  def log_float(lo, hi)
    Math.exp(float(Math.log(lo), Math.log(hi)))
  end

  def pick(arr)
    arr[@rng.rand(arr.length)]
  end

  def sample(arr, k)
    arr.shuffle(random: @rng).take(k)
  end

  def dirichlet(k)
    xs = Array.new(k) { -Math.log([@rng.rand, 1e-12].max) }
    s = xs.sum
    xs.map { |x| x / s }
  end

  def bool(p = 0.5)
    @rng.rand < p
  end

  def seed
    @rng.rand(1..2_000_000_000)
  end
end

def p_he(fi, fo, seed)
  scale(randn(fi, fo, seed), Math.sqrt(2.0 / [fi, 1].max))
end

def p_zero(rows, cols)
  Param.new(zeros(rows, cols), zeros(rows, cols), zeros(rows, cols))
end

def p_weight(fi, fo, seed)
  Param.new(p_he(fi, fo, seed), zeros(fi, fo), zeros(fi, fo))
end

def p_bias(dim)
  Param.new(zeros(1, dim), zeros(1, dim), zeros(1, dim))
end

def p_gamma(dim)
  Param.new(ones(1, dim), zeros(1, dim), zeros(1, dim))
end

def adamw_step!(param, grad, t, lr, wd)
  adamw_update(param.w, param.m, param.v, grad, lr, 0.9, 0.999, 1e-8, wd, t)
end

def scalar(buf)
  download(buf)[0].to_f
end

def ones_col(n)
  ones(n, 1)
end

def bias_fill(n, value)
  fill(n, 1, value)
end

def safe_var(buf)
  clamp(reduce_var_cols(buf), 1e-12, 1e30)
end

def mse(pred, actual)
  scalar(reduce_mean_cols(mul(sub(pred, actual), sub(pred, actual))))
end

def r2(pred, actual)
  ss_res = scalar(reduce_sum_cols(mul(sub(pred, actual), sub(pred, actual))))
  mu = scalar(reduce_mean_cols(actual))
  centered = sub(actual, bias_fill(actual.rows, mu))
  ss_tot = scalar(reduce_sum_cols(mul(centered, centered)))
  return 0.0 if ss_tot < 1e-30
  1.0 - (ss_res / ss_tot)
end

def weighted_sum(preds, weights)
  acc = scale(preds[0], weights[0])
  (1...preds.length).each { |i| acc = add(acc, scale(preds[i], weights[i])) }
  acc
end

def unscale_y(buf, y_mu, y_sd)
  add(scale(buf, y_sd), bias_fill(buf.rows, y_mu))
end

def numeric_headers(rows)
  rows.headers.select do |h|
    rows.all? do |r|
      v = r[h]
      v.nil? || v == '' || v.match?(/\A[+\-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+\-]?\d+)?\z/)
    end
  end
end

def fisher_yates!(arr, rng)
  (arr.length - 1).downto(1) do |i|
    j = rng.int(0, i)
    arr[i], arr[j] = arr[j], arr[i]
  end
end

def load_vna2
  x_path = File.join(__dir__, '../data/vna2/X_vna2.csv')
  y_path = File.join(__dir__, '../data/vna2/y_vna2.csv')
  x_rows = CSV.read(x_path, headers: true)
  y_rows = CSV.read(y_path, headers: true)
  cols = numeric_headers(x_rows)
  idx = (0...x_rows.size).to_a
  rng = SamplerRng.new(SEED)
  fisher_yates!(idx, rng)

  x_data = []
  y_data = []
  idx.each do |i|
    row = x_rows[i]
    x_data.concat(cols.map { |h| (row[h] || '0').to_f })
    y_data << y_rows[i][0].to_f
  end

  x = upload(x_data, x_rows.size, cols.size)
  y = upload(y_data, x_rows.size, 1)

  n = x_rows.size
  n_train = (n * 0.6).to_i
  n_val = (n * 0.2).to_i
  n_test = n - n_train - n_val

  x_train = slice_rows(x, 0, n_train)
  x_val = slice_rows(x, n_train, n_val)
  x_test = slice_rows(x, n_train + n_val, n_test)
  y_train_raw = slice_rows(y, 0, n_train)
  y_val_raw = slice_rows(y, n_train, n_val)
  y_test_raw = slice_rows(y, n_train + n_val, n_test)

  x_mu = reduce_mean_cols(x_train)
  x_sd = sqrt(safe_var(x_train))
  x_train = broadcast_div(broadcast_sub(x_train, x_mu), x_sd)
  x_val = broadcast_div(broadcast_sub(x_val, x_mu), x_sd)
  x_test = broadcast_div(broadcast_sub(x_test, x_mu), x_sd)

  y_mu = scalar(reduce_mean_cols(y_train_raw))
  y_sd = Math.sqrt([scalar(reduce_var_cols(y_train_raw)), 1e-30].max)
  y_train = scale(sub(y_train_raw, bias_fill(n_train, y_mu)), 1.0 / y_sd)
  y_val = scale(sub(y_val_raw, bias_fill(n_val, y_mu)), 1.0 / y_sd)
  y_test = scale(sub(y_test_raw, bias_fill(n_test, y_mu)), 1.0 / y_sd)

  {
    x_train: x_train,
    y_train: y_train,
    x_val: x_val,
    y_val: y_val,
    x_test: x_test,
    y_test: y_test,
    y_val_raw: y_val_raw,
    y_test_raw: y_test_raw,
    y_mu: y_mu,
    y_sd: y_sd,
    n: n,
    p: cols.size,
    headers: cols
  }
end

def activation_forward(kind, x)
  case kind
  when :relu then relu(x)
  when :tanh then tanh_act(x)
  when :silu then silu(x)
  else gelu(x)
  end
end

def activation_backward(kind, grad, cache)
  case kind
  when :relu then relu_backward(grad, cache)
  when :tanh then tanh_backward(grad, cache)
  when :silu then silu_backward(grad, cache)
  else gelu_backward(grad, cache)
  end
end

def ridge_closed_form(x, y, lambda_l2)
  xb = concat(x, ones_col(x.rows))
  xtx = gemm(xb, xb, 'T', 'N')
  diag_add(xtx, lambda_l2)
  xty = gemm(xb, y, 'T', 'N')
  wb = solve(xtx, xty)
  w = slice_rows(wb, 0, x.cols)
  b = transpose(slice_rows(wb, x.cols, 1))
  { w: w, b: b }
end

def fit_linear_family(kind, hp)
  lambda_l2 = hp.fetch(:lambda_l2, 0.0)
  lambda_l1 = hp.fetch(:lambda_l1, 0.0)
  lr = hp.fetch(:lr, 1e-2)
  epochs = hp.fetch(:epochs, 800)

  state = {}
  fit = lambda do |x_train, y_train|
    if kind == :linear || (kind == :ridge && lambda_l1 <= 0.0)
      state.replace(ridge_closed_form(x_train, y_train, lambda_l2))
    else
      w = zeros(x_train.cols, 1)
      b = zeros(1, 1)
      epochs.times do
        pred = linear(x_train, w, b)
        grad = scale(sub(pred, y_train), 2.0 / x_train.rows)
        g_w = gemm(x_train, grad, 'T', 'N')
        g_b = reduce_mean_cols(grad)
        w = sub_scale(w, g_w, lr)
        b = sub_scale(b, g_b, lr)
        if lambda_l2 > 0.0
          w = scale(w, 1.0 / (1.0 + lr * lambda_l2))
        end
        if lambda_l1 > 0.0
          th = fill(w.rows, w.cols, lr * lambda_l1)
          w = mul(sign(w), clamp(sub(abs(w), th), 0.0, 1e30))
        end
      end
      state[:w] = w
      state[:b] = b
    end
    state
  end

  predict = lambda do |xq|
    linear(xq, state[:w], state[:b])
  end

  { fit: fit, predict: predict }
end

def fit_knn(hp)
  k = hp.fetch(:k, 7)
  state = {}
  fit = lambda do |x_train, y_train|
    state[:x_train_cpu] = download(x_train)
    state[:y_train_cpu] = download(y_train)
    state[:train_rows] = x_train.rows
    state[:train_cols] = x_train.cols
    state[:k] = [[k, 1].max, x_train.rows].min
    state
  end

  predict = lambda do |xq|
    xq_cpu = download(xq)
    ys = state[:y_train_cpu]
    xtr = state[:x_train_cpu]
    rows = xq.rows
    cols = state[:train_cols]
    kk = state[:k]
    out = Array.new(rows, 0.0)

    rows.times do |i|
      dists = Array.new(state[:train_rows])
      base_q = i * cols
      state[:train_rows].times do |j|
        base_t = j * cols
        s = 0.0
        cols.times do |c|
          dv = xq_cpu[base_q + c] - xtr[base_t + c]
          s += dv * dv
        end
        dists[j] = [s, j]
      end
      best = dists.min_by(kk) { |pair| pair[0] }
      acc = 0.0
      best.each { |_, j| acc += ys[j] }
      out[i] = acc / kk
    end

    upload(out, rows, 1)
  end

  { fit: fit, predict: predict }
end

def quantize_cpu(x_cpu, rows, cols, n_bins)
  mins = Array.new(cols, Float::INFINITY)
  maxs = Array.new(cols, -Float::INFINITY)
  rows.times do |i|
    cols.times do |j|
      v = x_cpu[i * cols + j]
      mins[j] = v if v < mins[j]
      maxs[j] = v if v > maxs[j]
    end
  end
  bins = Array.new(rows * cols, 0.0)
  rows.times do |i|
    cols.times do |j|
      idx = i * cols + j
      lo = mins[j]
      hi = maxs[j]
      if (hi - lo).abs < 1e-12
        bins[idx] = 0.0
      else
        raw = ((x_cpu[idx] - lo) / (hi - lo + 1e-12) * (n_bins - 1)).round
        bins[idx] = [[raw, 0].max, n_bins - 1].min.to_f
      end
    end
  end
  [bins, mins, maxs]
end

def apply_quantization_cpu(x_cpu, rows, cols, n_bins, mins, maxs)
  out = Array.new(rows * cols, 0.0)
  rows.times do |i|
    cols.times do |j|
      idx = i * cols + j
      lo = mins[j]
      hi = maxs[j]
      if (hi - lo).abs < 1e-12
        out[idx] = 0.0
      else
        raw = ((x_cpu[idx] - lo) / (hi - lo + 1e-12) * (n_bins - 1)).round
        out[idx] = [[raw, 0].max, n_bins - 1].min.to_f
      end
    end
  end
  out
end

def fit_hist_ensemble(kind, hp)
  n_estimators = hp.fetch(:n_estimators, 16)
  n_bins = hp.fetch(:n_bins, 32)
  lambda_l2 = hp.fetch(:lambda_l2, 1.0)
  learning_rate = hp.fetch(:learning_rate, 0.05)
  colsample = hp.fetch(:colsample, 1.0)
  subsample = hp.fetch(:subsample, 1.0)
  rng = Random.new(hp.fetch(:seed, SEED))

  case kind
  when :decision_tree
    n_estimators = 1
    colsample = 1.0
    subsample = 1.0
    learning_rate = 1.0
  when :random_forest
    n_estimators = [[n_estimators, 16].max, 128].min
    learning_rate = 1.0
  when :gbm
    n_estimators = [[n_estimators, 12].max, 128].min
  when :xgb
    n_estimators = [[n_estimators, 12].max, 160].min
    colsample = [colsample, 0.55].max
    subsample = [subsample, 0.55].max
  when :lgbm
    n_estimators = [[n_estimators, 16].max, 192].min
  when :catboost
    n_estimators = [[n_estimators, 16].max, 192].min
    subsample = [subsample, 0.75].max
  end

  state = {}
  fit = lambda do |x_train, y_train|
    x_cpu = download(x_train)
    y_cpu = download(y_train)
    rows = x_train.rows
    cols = x_train.cols
    bin_cpu, mins, maxs = quantize_cpu(x_cpu, rows, cols, n_bins)
    bins_gpu = upload(bin_cpu, rows, cols)
    pred_cpu = Array.new(rows, 0.0)
    trees = []

    n_estimators.times do
      active_cols = (0...cols).to_a.select { |_j| rng.rand < colsample }
      active_cols = (0...cols).to_a.first(1) if active_cols.empty?
      active_rows = Array.new(rows, 0.0)
      rows.times { |i| active_rows[i] = rng.rand < subsample ? 1.0 : 0.0 }
      active_gpu = upload(active_rows, rows, 1)
      grad_cpu = Array.new(rows) { |i| pred_cpu[i] - y_cpu[i] }
      grad_gpu = upload(grad_cpu, rows, 1)
      hess_gpu = ones(rows, 1)
      gh, hh, = histogram_build(bins_gpu, grad_gpu, hess_gpu, active_gpu, n_bins)
      gain_buf, split_buf = split_eval(gh, hh, lambda_l2, 1.0)
      gains = download(gain_buf)
      split_bins = download(split_buf).map(&:to_i)

      feature = active_cols.max_by { |j| gains[j] } || 0
      bin = split_bins[feature] || (n_bins / 2)
      g_l = 0.0
      h_l = 0.0
      g_r = 0.0
      h_r = 0.0
      rows.times do |i|
        next if active_rows[i] <= 0.0
        g = grad_cpu[i]
        if bin_cpu[i * cols + feature].to_i <= bin
          g_l += g
          h_l += 1.0
        else
          g_r += g
          h_r += 1.0
        end
      end
      left_value = -(g_l / (h_l + lambda_l2))
      right_value = -(g_r / (h_r + lambda_l2))
      stump = TreeNode.new(feature, bin, left_value, right_value)
      trees << stump
      rows.times do |i|
        addend = bin_cpu[i * cols + feature].to_i <= bin ? left_value : right_value
        pred_cpu[i] += learning_rate * addend
      end
    end

    state[:trees] = trees
    state[:learning_rate] = learning_rate
    state[:n_bins] = n_bins
    state[:mins] = mins
    state[:maxs] = maxs
    state
  end

  predict = lambda do |xq|
    x_cpu = download(xq)
    qbins = apply_quantization_cpu(x_cpu, xq.rows, xq.cols, state[:n_bins], state[:mins], state[:maxs])
    out = Array.new(xq.rows, 0.0)
    state[:trees].each do |tree|
      xq.rows.times do |i|
        addend = qbins[i * xq.cols + tree.feature].to_i <= tree.bin ? tree.left_value : tree.right_value
        out[i] += state[:learning_rate] * addend
      end
    end
    upload(out, xq.rows, 1)
  end

  { fit: fit, predict: predict }
end

def fit_random_feature_proxy(hp)
  width = hp.fetch(:width, 32)
  ridge = hp.fetch(:lambda_l2, 1e-2)
  seed = hp.fetch(:seed, SEED)
  nonneg = hp.fetch(:nonneg, false)
  activation = hp.fetch(:activation, :gelu)
  kernel_mix = hp.fetch(:kernel_mix, false)

  state = {}
  fit = lambda do |x_train, y_train|
    proj = scale(randn(x_train.cols, width, seed), 1.0 / Math.sqrt([x_train.cols, 1].max))
    z = gemm(x_train, proj, 'N', 'N')
    z = activation_forward(activation, z)
    z = clamp(z, 0.0, 1e30) if nonneg
    if kernel_mix
      z2 = mul(z, z)
      z = concat(z, z2)
    end
    head = ridge_closed_form(z, y_train, ridge)
    state[:proj] = proj
    state[:head] = head
    state[:activation] = activation
    state[:nonneg] = nonneg
    state[:kernel_mix] = kernel_mix
    state
  end

  predict = lambda do |xq|
    z = gemm(xq, state[:proj], 'N', 'N')
    z = activation_forward(state[:activation], z)
    z = clamp(z, 0.0, 1e30) if state[:nonneg]
    z = concat(z, mul(z, z)) if state[:kernel_mix]
    linear(z, state[:head][:w], state[:head][:b])
  end

  { fit: fit, predict: predict }
end

def fit_mlp(hp)
  hidden = hp.fetch(:hidden, 64)
  epochs = hp.fetch(:epochs, 1200)
  lr = hp.fetch(:lr, 8e-4)
  wd = hp.fetch(:wd, 1e-3)
  drop = hp.fetch(:dropout, 0.0)
  act = hp.fetch(:activation, :gelu)
  seed = hp.fetch(:seed, SEED)

  state = {}
  fit = lambda do |x_train, y_train|
    w1 = p_weight(x_train.cols, hidden, seed + 1)
    b1 = p_bias(hidden)
    w2 = p_weight(hidden, 1, seed + 2)
    b2 = p_bias(1)

    epochs.times do |ep|
      z1 = linear(x_train, w1.w, b1.w)
      a1 = activation_forward(act, z1)
      if drop > 0.0
        mask = bernoulli(a1.rows, a1.cols, 1.0 - drop, seed + 1000 + ep)
        a1 = dropout(a1, mask, drop)
      end
      pred = linear(a1, w2.w, b2.w)
      grad = scale(sub(pred, y_train), 2.0 / x_train.rows)
      g_h, g_w2, g_b2 = linear_backward(grad, a1, w2.w)
      g_h = activation_backward(act, g_h, z1)
      _, g_w1, g_b1 = linear_backward(g_h, x_train, w1.w)
      t = ep + 1
      adamw_step!(w1, g_w1, t, lr, wd)
      adamw_step!(b1, g_b1, t, lr, wd)
      adamw_step!(w2, g_w2, t, lr, wd)
      adamw_step!(b2, g_b2, t, lr, wd)
    end

    state[:w1] = w1
    state[:b1] = b1
    state[:w2] = w2
    state[:b2] = b2
    state[:act] = act
    state[:drop] = drop
    state
  end

  predict = lambda do |xq|
    z1 = linear(xq, state[:w1].w, state[:b1].w)
    a1 = activation_forward(state[:act], z1)
    linear(a1, state[:w2].w, state[:b2].w)
  end

  { fit: fit, predict: predict }
end

def fit_tabresnet(hp)
  hidden = hp.fetch(:hidden, 32)
  blocks = hp.fetch(:blocks, 2)
  epochs = hp.fetch(:epochs, 1800)
  lr = hp.fetch(:lr, 5e-4)
  wd = hp.fetch(:wd, 1e-3)
  drop = hp.fetch(:dropout, 0.3)
  seed = hp.fetch(:seed, SEED)

  state = {}
  fit = lambda do |x_train, y_train|
    lift_w = p_weight(x_train.cols, hidden, seed + 1)
    lift_b = p_bias(hidden)
    head_w = p_weight(hidden, 1, seed + 2)
    head_b = p_bias(1)
    block_params = Array.new(blocks) do |i|
      {
        w1: p_weight(hidden, hidden, seed + 100 + i * 10 + 1),
        b1: p_bias(hidden),
        w2: p_weight(hidden, hidden, seed + 100 + i * 10 + 2),
        b2: p_bias(hidden),
        g1: p_gamma(hidden),
        n1: p_bias(hidden),
        g2: p_gamma(hidden),
        n2: p_bias(hidden)
      }
    end

    fwd = lambda do |h, train_flag, step_seed|
      caches = []
      block_params.each_with_index do |b, i|
        c = { inp: h }
        o1, m1, iv1 = batchnorm_forward(h, b[:g1].w, b[:n1].w, EPS)
        a1 = gelu(o1)
        x2 = linear(a1, b[:w1].w, b[:b1].w)
        o2, m2, iv2 = batchnorm_forward(x2, b[:g2].w, b[:n2].w, EPS)
        a2 = gelu(o2)
        if train_flag && drop > 0.0
          mk = bernoulli(a2.rows, a2.cols, 1.0 - drop, step_seed + i)
          a2 = dropout(a2, mk, drop)
          c[:mk] = mk
        end
        out = linear(a2, b[:w2].w, b[:b2].w)
        c[:o1] = o1
        c[:a1] = a1
        c[:x2] = x2
        c[:o2] = o2
        c[:a2] = a2
        c[:m1] = m1
        c[:iv1] = iv1
        c[:m2] = m2
        c[:iv2] = iv2
        h = add(out, c[:inp])
        caches << c
      end
      [h, caches]
    end

    bwd = lambda do |g, caches, t|
      (blocks - 1).downto(0) do |i|
        b = block_params[i]
        c = caches[i]
        skip = g
        g, g_w2, g_b2 = linear_backward(g, c[:a2], b[:w2].w)
        g = dropout(g, c[:mk], drop) if c[:mk]
        g = gelu_backward(g, c[:o2])
        g, g_g2, g_n2 = batchnorm_backward(g, c[:x2], c[:m2], c[:iv2], b[:g2].w)
        g, g_w1, g_b1 = linear_backward(g, c[:a1], b[:w1].w)
        g = gelu_backward(g, c[:o1])
        g, g_g1, g_n1 = batchnorm_backward(g, c[:inp], c[:m1], c[:iv1], b[:g1].w)
        g = add(g, skip)
        adamw_step!(b[:w2], g_w2, t, lr, wd)
        adamw_step!(b[:b2], g_b2, t, lr, wd)
        adamw_step!(b[:g2], g_g2, t, lr, wd)
        adamw_step!(b[:n2], g_n2, t, lr, wd)
        adamw_step!(b[:w1], g_w1, t, lr, wd)
        adamw_step!(b[:b1], g_b1, t, lr, wd)
        adamw_step!(b[:g1], g_g1, t, lr, wd)
        adamw_step!(b[:n1], g_n1, t, lr, wd)
      end
      g
    end

    epochs.times do |ep|
      h0 = linear(x_train, lift_w.w, lift_b.w)
      h, caches = fwd.call(h0, true, seed + ep * 100)
      pred = linear(h, head_w.w, head_b.w)
      grad = scale(sub(pred, y_train), 2.0 / x_train.rows)
      g, g_hw, g_hb = linear_backward(grad, h, head_w.w)
      g = bwd.call(g, caches, ep + 1)
      _, g_lw, g_lb = linear_backward(g, x_train, lift_w.w)
      t = ep + 1
      adamw_step!(lift_w, g_lw, t, lr, wd)
      adamw_step!(lift_b, g_lb, t, lr, wd)
      adamw_step!(head_w, g_hw, t, lr, wd)
      adamw_step!(head_b, g_hb, t, lr, wd)
    end

    state[:lift_w] = lift_w
    state[:lift_b] = lift_b
    state[:head_w] = head_w
    state[:head_b] = head_b
    state[:blocks] = block_params
    state[:drop] = drop
    state
  end

  predict = lambda do |xq|
    h = linear(xq, state[:lift_w].w, state[:lift_b].w)
    state[:blocks].each do |b|
      o1, = batchnorm_forward(h, b[:g1].w, b[:n1].w, EPS)
      a1 = gelu(o1)
      x2 = linear(a1, b[:w1].w, b[:b1].w)
      o2, = batchnorm_forward(x2, b[:g2].w, b[:n2].w, EPS)
      a2 = gelu(o2)
      h = add(linear(a2, b[:w2].w, b[:b2].w), h)
    end
    linear(h, state[:head_w].w, state[:head_b].w)
  end

  { fit: fit, predict: predict }
end

def builder_for(name, family, adapter, hp)
  base = case family
         when :linear then fit_linear_family(:linear, hp)
         when :ridge then fit_linear_family(:ridge, hp)
         when :lasso then fit_linear_family(:lasso, hp)
         when :elastic_net then fit_linear_family(:elastic_net, hp)
         when :knn then fit_knn(hp)
         when :decision_tree, :random_forest, :gbm, :xgb, :lgbm, :catboost then fit_hist_ensemble(family, hp)
         when :mlp then fit_mlp(hp)
         when :logistic_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 32), activation: :silu))
         when :kernel_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 48), activation: :gelu, kernel_mix: true))
         when :factor_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 24), activation: :gelu))
         when :ica_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 24), activation: :tanh))
         when :nmf_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 24), activation: :relu, nonneg: true))
         when :gmm_proxy then fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 32), activation: :tanh, kernel_mix: true))
         when :hmm_proxy, :lstm_proxy, :gru_proxy, :seq_proxy, :dtw_proxy, :hierarchical_proxy, :embedding_proxy, :transformer_proxy, :cluster_proxy, :anomaly_proxy, :rule_proxy, :rl_proxy, :autoencoder_proxy, :vae_proxy, :gan_proxy
           if hp.fetch(:use_residual, false)
             fit_tabresnet(hp)
           else
             fit_random_feature_proxy(hp.merge(width: hp.fetch(:width, 40), activation: hp.fetch(:activation, :gelu), kernel_mix: hp.fetch(:kernel_mix, false), nonneg: hp.fetch(:nonneg, false)))
           end
         else
           fit_random_feature_proxy(hp)
         end

  {
    name: name,
    family: family,
    adapter: adapter,
    fit: base[:fit],
    predict: base[:predict],
    hp: hp
  }
end

def sample_hp(family, rng)
  case family
  when :linear
    {}
  when :ridge
    { lambda_l2: rng.log_float(1e-5, 10.0) }
  when :lasso
    { lambda_l1: rng.log_float(1e-5, 1.0), lambda_l2: 0.0, lr: rng.log_float(5e-4, 5e-2), epochs: rng.int(300, 1400) }
  when :elastic_net
    { lambda_l1: rng.log_float(1e-5, 1.0), lambda_l2: rng.log_float(1e-5, 10.0), lr: rng.log_float(5e-4, 5e-2), epochs: rng.int(300, 1400) }
  when :knn
    { k: rng.pick([3, 5, 7, 9, 11, 15]) }
  when :decision_tree, :random_forest, :gbm, :xgb, :lgbm, :catboost
    {
      n_estimators: rng.int(8, 96),
      n_bins: rng.pick([16, 24, 32, 48, 64]),
      lambda_l2: rng.log_float(1e-3, 10.0),
      learning_rate: rng.log_float(0.01, 0.2),
      colsample: rng.float(0.5, 1.0),
      subsample: rng.float(0.5, 1.0),
      seed: rng.seed
    }
  when :mlp
    {
      hidden: rng.pick([16, 24, 32, 48, 64, 96, 128]),
      epochs: rng.int(400, 2200),
      lr: rng.log_float(1e-4, 5e-3),
      wd: rng.log_float(1e-5, 5e-3),
      dropout: rng.float(0.0, 0.45),
      activation: rng.pick([:gelu, :relu, :silu, :tanh]),
      seed: rng.seed
    }
  else
    {
      width: rng.pick([16, 24, 32, 48, 64]),
      lambda_l2: rng.log_float(1e-5, 5.0),
      activation: rng.pick([:gelu, :relu, :silu, :tanh]),
      kernel_mix: rng.bool(0.35),
      nonneg: rng.bool(0.2),
      use_residual: rng.bool(0.2),
      hidden: rng.pick([16, 24, 32, 48]),
      blocks: rng.pick([1, 2, 3]),
      epochs: rng.int(300, 1400),
      lr: rng.log_float(1e-4, 2e-3),
      wd: rng.log_float(1e-5, 2e-3),
      dropout: rng.float(0.0, 0.35),
      seed: rng.seed
    }
  end
end

def sample_candidate(rng)
  count = rng.int(MIN_COMPOSE, MAX_COMPOSE)
  chosen = rng.sample(MODEL_SPECS, count)
  members = chosen.map do |name, family, adapter|
    { name: name, family: family, adapter: adapter, hp: sample_hp(family, rng) }
  end
  { members: members, weights: rng.dirichlet(count), phase: 1 }
end

def refine_candidate(best, rng)
  weights = best[:weights].map { |w| [w + rng.float(-0.15, 0.15), 0.01].max }
  sw = weights.sum
  weights.map! { |w| w / sw }
  members = best[:members].map do |m|
    hp = m[:hp].dup
    hp.each do |k, v|
      hp[k] = case v
              when Integer then [1, (v * rng.float(0.75, 1.25)).round].max
              when Float then v > 0 ? v * rng.float(0.75, 1.25) : v
              else v
              end
    end
    { name: m[:name], family: m[:family], adapter: m[:adapter], hp: hp }
  end
  { members: members, weights: weights, phase: 2 }
end

def compose_closure(candidate)
  lambda do |_hp_unused = nil|
    trained = []
    fit = lambda do |x_train, y_train|
      trained.clear
      candidate[:members].each do |m|
        tm = builder_for(m[:name], m[:family], m[:adapter], m[:hp])
        tm[:fit].call(x_train, y_train)
        trained << tm
      end
      trained
    end
    predict = lambda do |xq|
      preds = trained.map { |tm| tm[:predict].call(xq) }
      weighted_sum(preds, candidate[:weights])
    end
    { fit: fit, predict: predict }
  end
end

def describe_candidate(candidate)
  parts = candidate[:members].each_with_index.map do |m, i|
    w = format('%.3f', candidate[:weights][i])
    "#{m[:name]}[#{m[:adapter]}|#{w}]"
  end
  parts.join(' + ')
end

def evaluate_candidate(candidate, data)
  composed = compose_closure(candidate).call
  composed[:fit].call(data[:x_train], data[:y_train])
  val_pred = composed[:predict].call(data[:x_val])
  test_pred = composed[:predict].call(data[:x_test])

  val_pred_raw = unscale_y(val_pred, data[:y_mu], data[:y_sd])
  test_pred_raw = unscale_y(test_pred, data[:y_mu], data[:y_sd])

  candidate.merge(
    val_r2: r2(val_pred_raw, data[:y_val_raw]),
    val_mse: mse(val_pred_raw, data[:y_val_raw]),
    test_r2: r2(test_pred_raw, data[:y_test_raw]),
    test_mse: mse(test_pred_raw, data[:y_test_raw]),
    error: nil
  )
rescue StandardError => e
  candidate.merge(
    val_r2: -Float::INFINITY,
    val_mse: Float::INFINITY,
    test_r2: -Float::INFINITY,
    test_mse: Float::INFINITY,
    error: "#{e.class}: #{e.message}"
  )
end

def refit_best(candidate, data)
  x_fit = vconcat(data[:x_train], data[:x_val])
  y_fit = vconcat(data[:y_train], data[:y_val])
  composed = compose_closure(candidate).call
  composed[:fit].call(x_fit, y_fit)
  pred = composed[:predict].call(data[:x_test])
  pred_raw = unscale_y(pred, data[:y_mu], data[:y_sd])
  { test_r2: r2(pred_raw, data[:y_test_raw]), test_mse: mse(pred_raw, data[:y_test_raw]) }
end

def leaderboard(title, rows)
  $stderr.puts "\n#{title}"
  rows.each_with_index do |r, i|
    line = format('%3d  phase=%d  val_r2=% .6f  test_r2=% .6f  val_mse=% .6f  %s', i + 1, r[:phase], r[:val_r2], r[:test_r2], r[:val_mse], describe_candidate(r))
    line += "  ERROR=#{r[:error]}" if r[:error]
    $stderr.puts line
  end
end

data = load_vna2
$stderr.puts "Loaded vna2: n=#{data[:n]} p=#{data[:p]} train=#{data[:x_train].rows} val=#{data[:x_val].rows} test=#{data[:x_test].rows}"
$stderr.puts "Model registry size: #{MODEL_SPECS.size}"

rng = SamplerRng.new(SEED)
phase1 = []
PHASE1_SAMPLES.times do |i|
  cand = sample_candidate(rng)
  res = evaluate_candidate(cand, data)
  phase1 << res
  msg = format('phase1 %3d/%3d  val_r2=% .6f  test_r2=% .6f  %s', i + 1, PHASE1_SAMPLES, res[:val_r2], res[:test_r2], describe_candidate(res))
  msg += "  ERROR=#{res[:error]}" if res[:error]
  $stderr.puts msg
end
phase1.sort_by! { |r| -r[:val_r2] }
leaderboard('Phase 1 leaderboard', phase1.first(TOPK_PRINT))

phase1_ok = phase1.reject { |r| r[:error] }
abort('No valid phase-1 candidates completed successfully.') if phase1_ok.empty?
best_seed = phase1_ok.first
phase2 = []
PHASE2_SAMPLES.times do |i|
  cand = refine_candidate(best_seed, rng)
  res = evaluate_candidate(cand, data)
  phase2 << res
  msg = format('phase2 %3d/%3d  val_r2=% .6f  test_r2=% .6f  %s', i + 1, PHASE2_SAMPLES, res[:val_r2], res[:test_r2], describe_candidate(res))
  msg += "  ERROR=#{res[:error]}" if res[:error]
  $stderr.puts msg
end
phase2.sort_by! { |r| -r[:val_r2] }
leaderboard('Phase 2 leaderboard', phase2.first(TOPK_PRINT))

phase2_ok = phase2.reject { |r| r[:error] }
best = ([best_seed] + phase2_ok).max_by { |r| r[:val_r2] }
final = refit_best(best, data)

puts "BEST_SAMPLE"
puts "phase=#{best[:phase]}"
puts format('val_r2=%.8f', best[:val_r2])
puts format('val_mse=%.8f', best[:val_mse])
puts format('test_r2_holdout=%.8f', best[:test_r2])
puts format('test_mse_holdout=%.8f', best[:test_mse])
puts format('test_r2_refit=%.8f', final[:test_r2])
puts format('test_mse_refit=%.8f', final[:test_mse])
puts "composition=#{describe_candidate(best)}"
puts 'members='
best[:members].each do |m|
  puts "- #{m[:name]} | family=#{m[:family]} | adapter=#{m[:adapter]} | hp=#{m[:hp]}"
end

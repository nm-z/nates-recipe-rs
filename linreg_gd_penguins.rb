require "datasets"
require File.expand_path("nates-gpu-ruby/target/release/nates_gpu.so", __dir__)
include NatesGpu

LR     = 0.01
EPOCHS = 100

rows = Datasets::Penguins.new.each.select do |r|
      r.flipper_length_mm && r.bill_length_mm && r.body_mass_g
end
m = rows.size

def zscore(a)
      (a - reduce_mean_cols(a)) / sqrt(reduce_var_cols(a))
end

x = zscore(upload(rows.flat_map { |r| [r.flipper_length_mm.to_f, r.bill_length_mm.to_f] }, m, 2))
y = zscore(upload(rows.map { |r| r.body_mass_g.to_f }, m, 1))

w = zeros(2, 1)
b = zeros(1, 1)

EPOCHS.times do |epoch|
      z = linear(x, w, b)
      d = z - y
      cost = download(reduce_mean_cols(d * d))[0]

      grad = d * (2.0 / m)
      g1 = gemm(x, grad, "T", "N")
      g2 = reduce_sum_cols(grad)

      sgd_update(w, g1, LR)
      sgd_update(b, g2, LR)

      wv = download(w)
      bv = download(b)
      puts format("epoch %3d  cost %.6f  w=[% .4f, % .4f]  b=% .4f", epoch, cost, wv[0], wv[1], bv[0])
end

resid = download(linear(x, w, b) - y)
yv = download(y)
ybar = yv.sum / m
ss_res = resid.sum { |e| e * e }
ss_tot = yv.sum { |t| (t - ybar)**2 }
puts format("R2 = %.4f   (lr=%g, epochs=%d, n=%d)", 1 - ss_res / ss_tot, LR, EPOCHS, m)

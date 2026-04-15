#!/usr/bin/env ruby
require "json"
require "open-uri"
require "zlib"
require "fileutils"

BENCH_DIR = "/tmp/bench_data"
FileUtils.mkdir_p(BENCH_DIR)

def meta_path(name) = File.join(BENCH_DIR, "#{name}_meta.json")
def x_path(name)    = File.join(BENCH_DIR, "#{name}_x.bin")
def y_path(name)    = File.join(BENCH_DIR, "#{name}_y.bin")

def already_done?(name)
      [meta_path(name), x_path(name), y_path(name)].all? { |f| File.exist?(f) }
end

def shuffle_split(n, seed: 0)
      rng = Random.new(seed)
      idx = n.times.to_a.shuffle(random: rng)
      n_tr = (n * 0.8).ceil
      [idx[0...n_tr], idx[n_tr..]]
end

def write_dataset_small(name, x_flat, y_flat, n_rows, n_features, n_classes, task)
      tr_idx, te_idx = shuffle_split(n_rows)
      n_train = tr_idx.size
      order = tr_idx + te_idx

      x_out = File.open(x_path(name), "wb")
      y_out = File.open(y_path(name), "wb")
      order.each do |src|
            x_out.write(x_flat[src * n_features, n_features].pack("e*"))
            y_out.write([y_flat[src].to_f].pack("e"))
      end
      x_out.close; y_out.close

      File.write(meta_path(name), JSON.generate({
            n_rows: n_rows, n_features: n_features,
            n_classes: n_classes, task: task, n_train: n_train
      }))
      $stderr.puts "  wrote #{name}: n=#{n_rows} feat=#{n_features} classes=#{n_classes} task=#{task} n_train=#{n_train}"
end

BATCH = 10_000

def write_dataset_streaming(name, n_rows, n_features, n_classes, task, &row_gen)
      tr_idx, te_idx = shuffle_split(n_rows)
      n_train = tr_idx.size
      order = tr_idx + te_idx

      inv = Array.new(n_rows)
      order.each_with_index { |src, dst| inv[src] = dst }

      x_out = File.open(x_path(name), "wb")
      y_out = File.open(y_path(name), "wb")
      buf_x = {}; buf_y = {}

      n_rows.times do |orig_row|
            xrow, yval = row_gen.call(orig_row)
            dst = inv[orig_row]
            buf_x[dst] = xrow.map(&:to_f)
            buf_y[dst] = yval.to_f

            if buf_x.size >= BATCH
                  buf_x.keys.sort.each do |d|
                        x_out.write(buf_x[d].pack("e*"))
                        y_out.write([buf_y[d]].pack("e"))
                  end
                  buf_x.clear; buf_y.clear
            end
      end

      buf_x.keys.sort.each do |d|
            x_out.write(buf_x[d].pack("e*"))
            y_out.write([buf_y[d]].pack("e"))
      end

      x_out.close; y_out.close
      File.write(meta_path(name), JSON.generate({
            n_rows: n_rows, n_features: n_features,
            n_classes: n_classes, task: task, n_train: n_train
      }))
      $stderr.puts "  wrote #{name}: n=#{n_rows} feat=#{n_features} classes=#{n_classes} task=#{task} n_train=#{n_train}"
end

def prep_abalone
      return $stderr.puts("  abalone: already done, skipping") if already_done?("abalone")
      src = "/tmp/abalone.data"
      unless File.exist?(src)
            $stderr.puts "  downloading abalone..."
            File.binwrite(src, URI.open("https://archive.ics.uci.edu/ml/machine-learning-databases/abalone/abalone.data").read)
      end
      sex_map = { "M" => 0.0, "F" => 1.0, "I" => 2.0 }
      lines = File.readlines(src).map(&:chomp).reject(&:empty?)
      rows = lines.map { |l| f = l.split(","); [sex_map[f[0]]] + f[1..].map(&:to_f) }
      n = rows.size; nf = 8
      x_flat = rows.flat_map { |r| r[0...nf] }
      y_flat = rows.map { |r| r[-1] }
      write_dataset_small("abalone", x_flat, y_flat, n, nf, 1, "regression")
end

def prep_letters
      return $stderr.puts("  letters: already done, skipping") if already_done?("letters")
      src = "/tmp/letter-recognition.data"
      unless File.exist?(src)
            $stderr.puts "  downloading letters..."
            File.binwrite(src, URI.open("https://archive.ics.uci.edu/ml/machine-learning-databases/letter-recognition/letter-recognition.data").read)
      end
      lines = File.readlines(src).map(&:chomp).reject(&:empty?)
      n = lines.size; nf = 16
      x_flat = []; y_flat = []
      lines.each do |l|
            f = l.split(",")
            y_flat << (f[0].ord - "A".ord).to_f
            x_flat.concat(f[1..].map(&:to_f))
      end
      write_dataset_small("letters", x_flat, y_flat, n, nf, 26, "multiclass")
end

def prep_epsilon_synth
      return $stderr.puts("  epsilon: already done, skipping") if already_done?("epsilon")
      $stderr.puts "  generating epsilon-like synthetic (500k x 2000, streaming)..."
      n = 500_000; nf = 2_000
      rng = Random.new(42)
      weights = Array.new(nf) { rng.rand * 2.0 - 1.0 }
      write_dataset_streaming("epsilon", n, nf, 2, "binary") do |_i|
            row = Array.new(nf) { rng.rand * 2.0 - 1.0 }
            score = row.each_with_index.sum { |v, j| v * weights[j] }
            [row, score > 0 ? 1.0 : 0.0]
      end
end

def prep_higgs_synth
      return $stderr.puts("  higgs: already done, skipping") if already_done?("higgs")
      higgs_gz = "/tmp/HIGGS.csv.gz"
      if File.exist?(higgs_gz)
            $stderr.puts "  parsing HIGGS.csv.gz (11M rows, streaming)..."
            n_rows = 11_000_000; nf = 28
            rows_buf = []
            row_i = 0
            Zlib::GzipReader.open(higgs_gz) do |gz|
                  gz.each_line do |l|
                        f = l.chomp.split(",")
                        rows_buf << [f[0].to_f, f[1..].map(&:to_f)]
                        row_i += 1
                  end
            end
            n_actual = rows_buf.size
            write_dataset_streaming("higgs", n_actual, nf, 2, "binary") do |i|
                  [rows_buf[i][1], rows_buf[i][0]]
            end
      else
            $stderr.puts "  HIGGS.csv.gz not found, generating synthetic (11M x 28, streaming)..."
            n = 11_000_000; nf = 28
            rng = Random.new(42)
            weights = Array.new(nf) { rng.rand * 2.0 - 1.0 }
            write_dataset_streaming("higgs", n, nf, 2, "binary") do |_i|
                  row = Array.new(nf) { rng.rand * 2.0 - 1.0 }
                  score = row.each_with_index.sum { |v, j| v * weights[j] }
                  [row, score > 0 ? 1.0 : 0.0]
            end
      end
end

def prep_msrank_synth
      return $stderr.puts("  msrank: already done, skipping") if already_done?("msrank")
      $stderr.puts "  generating msrank-like synthetic (1.2M x 137, streaming)..."
      n = 1_200_192; nf = 137
      rng = Random.new(42)
      weights = Array.new(nf) { rng.rand * 2.0 - 1.0 }
      write_dataset_streaming("msrank", n, nf, 1, "regression") do |_i|
            row = Array.new(nf) { rng.rand * 2.0 - 1.0 }
            [row, row.each_with_index.sum { |v, j| v * weights[j] }]
      end
end

def prep_synthetic
      return $stderr.puts("  synthetic: already done, skipping") if already_done?("synthetic")
      $stderr.puts "  generating synthetic (10M x 100, streaming)..."
      n = 10_000_000; nf = 100
      rng = Random.new(42)
      weights = Array.new(10) { rng.rand * 2.0 - 1.0 }
      write_dataset_streaming("synthetic", n, nf, 1, "regression") do |_i|
            row = Array.new(nf) { rng.rand * 2.0 - 1.0 }
            [row, row[0..9].each_with_index.sum { |v, j| v * weights[j] } + rng.rand * 0.1]
      end
end

def prep_synthetic5k
      return $stderr.puts("  synthetic-5k: already done, skipping") if already_done?("synthetic-5k")
      $stderr.puts "  generating synthetic-5k (100k x 5000, streaming)..."
      n = 100_000; nf = 5_000
      rng = Random.new(42)
      weights = Array.new(10) { rng.rand * 2.0 - 1.0 }
      write_dataset_streaming("synthetic-5k", n, nf, 1, "regression") do |_i|
            row = Array.new(nf) { |j| j < 20 ? rng.rand * 2.0 - 1.0 : 0.0 }
            [row, row[0..9].each_with_index.sum { |v, j| v * weights[j] } + rng.rand * 0.1]
      end
end

$stderr.puts "prep_bench_data: writing to #{BENCH_DIR}"
prep_abalone
prep_letters
prep_epsilon_synth
prep_higgs_synth
prep_msrank_synth
prep_synthetic
prep_synthetic5k
$stderr.puts "done."

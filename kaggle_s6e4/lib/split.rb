def deterministic_split(n, test_size: 0.2, seed: 0)
      rng = Random.new(seed)
      indices = (0...n).to_a
      (n - 1).downto(1) { |i| j = rng.rand(i + 1); indices[i], indices[j] = indices[j], indices[i] }
      n_test = (n * test_size).round
      n_train = n - n_test
      [indices[0...n_train], indices[n_train..]]
end

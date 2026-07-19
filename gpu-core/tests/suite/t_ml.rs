use gpu_core::bayes::{
	gpu_bernoulli_nb_logprob, gpu_multinomial_nb_logprob, gpu_nb_count_table,
	gpu_nb_feature_log_prob,
};
use gpu_core::catboost::{
	gpu_iota, gpu_ordered_target_stats, gpu_random_permutation,
	gpu_random_permutation_workspace_bytes,
};
use gpu_core::forest::{
	gpu_bootstrap_sample, gpu_feature_subset, gpu_oob_mask, gpu_random_threshold_split,
};
use gpu_core::kernels::gpu_rand_uniform_into;
use gpu_core::memory::GpuBuffer;
use gpu_core::rl::{
	gpu_categorical_logprob, gpu_discounted_returns, gpu_gae, gpu_gaussian_logprob,
	gpu_td_targets,
};
use std::collections::HashSet;
use std::f64::consts;
use std::mem;
use std::ptr;

const EPS: f64 = 1e-9;

// ── RL ────────────────────────────────────────────────────────────────────────

#[test]
fn test_rl_discounted_returns() {
	let rewards = {
		let __up = &[1.0_f64, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let gamma = {
		let __up = &[0.5_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_discounted_returns(&rewards, &gamma, 3, &out).unwrap();
	let mut result = [0.0_f64; 3];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	assert!(
		(result[0] - 1.75).abs() < EPS,
		"G[0]={} expected 1.75",
		result[0]
	);
	assert!(
		(result[1] - 1.50).abs() < EPS,
		"G[1]={} expected 1.5",
		result[1]
	);
	assert!(
		(result[2] - 1.00).abs() < EPS,
		"G[2]={} expected 1.0",
		result[2]
	);
	eprintln!("discounted_returns OK: {:?}", result);
}

#[test]
fn test_rl_gae() {
	let rewards = {
		let __up = &[1.0_f64, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let values = {
		let __up = &[1.0_f64, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let gamma = 0.9_f64;
	let lam = 0.95_f64;
	let gamma_buf = {
		let __up = &[gamma];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let lam_buf = {
		let __up = &[lam];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_gae(&rewards, &values, &gamma_buf, &lam_buf, 3, &out).unwrap();
	let mut result = [0.0_f64; 3];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let exp_a2 = 0.0_f64;
	let exp_a1 = 0.9_f64;
	let exp_a0 = 0.9 + gamma * lam * exp_a1;
	assert!((result[2] - exp_a2).abs() < 1e-9, "GAE A[2]={}", result[2]);
	assert!((result[1] - exp_a1).abs() < 1e-9, "GAE A[1]={}", result[1]);
	assert!(
		(result[0] - exp_a0).abs() < 1e-9,
		"GAE A[0]={} expected {}",
		result[0],
		exp_a0
	);
	eprintln!("gae OK: {:?}", result);
}

#[test]
fn test_rl_td_targets() {
	let rewards = {
		let __up = &[1.0_f64, 2.0, 3.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let values_next = {
		let __up = &[4.0_f64, 5.0, 6.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let done = {
		let __up = &[0.0_f64, 1.0, 0.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let gamma = {
		let __up = &[0.99_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(3).unwrap();
	gpu_td_targets(&rewards, &values_next, &done, &gamma, 3, &out).unwrap();
	let mut result = [0.0_f64; 3];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let exp = [4.96_f64, 2.0, 8.94];
	for i in 0..3 {
		assert!(
			(result[i] - exp[i]).abs() < 1e-9,
			"td_targets[{}]={} expected {}",
			i,
			result[i],
			exp[i]
		);
	}
	eprintln!("td_targets OK: {:?}", result);
}

#[test]
fn test_rl_categorical_logprob() {
	let logits = {
		let __up = &[0.0_f64, 1.0, 2.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let actions = GpuBuffer::upload_i32(&[2_i32]).unwrap();
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_categorical_logprob(&logits, &actions, 1, 3, &out).unwrap();
	let mut result = [0.0_f64; 1];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let log_z = (1.0_f64.exp() + 2.0_f64.exp() + 1.0_f64).ln();
	let expected = 2.0 - log_z;
	assert!(
		result[0].is_finite(),
		"categorical_logprob is not finite: {}",
		result[0]
	);
	assert!(
		(result[0] - expected).abs() < 1e-9,
		"categorical_logprob={} expected={}",
		result[0],
		expected
	);
	eprintln!(
		"categorical_logprob OK: {} (expected {})",
		result[0], expected
	);
}

#[test]
fn test_rl_gaussian_logprob() {
	let mu = {
		let __up = &[0.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let log_std = {
		let __up = &[0.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let actions = {
		let __up = &[1.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(1).unwrap();
	gpu_gaussian_logprob(&mu, &log_std, &actions, 1, 1, &out).unwrap();
	let mut result = [0.0_f64; 1];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let half_log2pi = 0.5 * (2.0 * consts::PI).ln();
	let expected = -0.5 - 0.0 - half_log2pi;
	assert!(
		result[0].is_finite(),
		"gaussian_logprob not finite: {}",
		result[0]
	);
	assert!(
		(result[0] - expected).abs() < 1e-9,
		"gaussian_logprob={} expected={}",
		result[0],
		expected
	);
	eprintln!("gaussian_logprob OK: {} (expected {})", result[0], expected);
}

// ── Bayes ─────────────────────────────────────────────────────────────────────

#[test]
fn test_bayes_nb_feature_log_prob() {
	let counts = {
		let __up = &[
			1.0_f64, 2.0, 3.0, // class 0
			4.0, 5.0, 6.0, // class 1
		];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let alpha = {
		let __up = &[1.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(6).unwrap();
	gpu_nb_feature_log_prob(&counts, &alpha, 2, 3, &out).unwrap();
	let mut result = [0.0_f64; 6];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let log9 = 9.0_f64.ln();
	let exp_c0 = [
		2.0_f64.ln() - log9,
		3.0_f64.ln() - log9,
		4.0_f64.ln() - log9,
	];
	let log18 = 18.0_f64.ln();
	let exp_c1 = [
		5.0_f64.ln() - log18,
		6.0_f64.ln() - log18,
		7.0_f64.ln() - log18,
	];

	for f in 0..3 {
		assert!(result[f].is_finite(), "class0 feat{} not finite", f);
		assert!(
			(result[f] - exp_c0[f]).abs() < 1e-9,
			"class0 feat{}: got={} expected={}",
			f,
			result[f],
			exp_c0[f]
		);
	}
	for f in 0..3 {
		assert!(result[3 + f].is_finite(), "class1 feat{} not finite", f);
		assert!(
			(result[3 + f] - exp_c1[f]).abs() < 1e-9,
			"class1 feat{}: got={} expected={}",
			f,
			result[3 + f],
			exp_c1[f]
		);
	}
	eprintln!("nb_feature_log_prob OK: {:?}", result);
}

#[test]
fn test_bayes_nb_count_table() {
	let x = {
		let __up = &[1.0_f64, 0.0, 0.0, 1.0, 1.0, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let y = GpuBuffer::upload_i32(&[0_i32, 1, 0]).unwrap();
	let out = {
		let __zb = GpuBuffer::alloc_bytes(4 * mem::size_of::<f64>()).unwrap();
		__zb.memset_zero(4 * mem::size_of::<f64>()).unwrap();
		__zb
	};
	gpu_nb_count_table(&x, &y, 3, 2, 2, &out).unwrap();
	let mut result = [0.0_f64; 4];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	assert!((result[0] - 2.0).abs() < 1e-9, "c0f0={}", result[0]);
	assert!((result[1] - 1.0).abs() < 1e-9, "c0f1={}", result[1]);
	assert!((result[2] - 0.0).abs() < 1e-9, "c1f0={}", result[2]);
	assert!((result[3] - 1.0).abs() < 1e-9, "c1f1={}", result[3]);
	eprintln!("nb_count_table OK: {:?}", result);
}

#[test]
fn test_bayes_multinomial_nb_logprob() {
	let log_prior = {
		let __up = &[0.5_f64.ln(), 0.5_f64.ln()];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let flp = {
		let __up = &[
			0.3_f64.ln(),
			0.7_f64.ln(), // class 0
			0.6_f64.ln(),
			0.4_f64.ln(), // class 1
		];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let x = {
		let __up = &[1.0_f64, 1.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(2).unwrap();
	gpu_multinomial_nb_logprob(&log_prior, &flp, &x, 1, 2, 2, &out).unwrap();
	let mut result = [0.0_f64; 2];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let exp0 = 0.5_f64.ln() + 0.3_f64.ln() + 0.7_f64.ln();
	let exp1 = 0.5_f64.ln() + 0.6_f64.ln() + 0.4_f64.ln();
	assert!(result[0].is_finite(), "multinomial logprob[0] not finite");
	assert!(result[1].is_finite(), "multinomial logprob[1] not finite");
	assert!(
		(result[0] - exp0).abs() < 1e-9,
		"logprob[0]={} expected={}",
		result[0],
		exp0
	);
	assert!(
		(result[1] - exp1).abs() < 1e-9,
		"logprob[1]={} expected={}",
		result[1],
		exp1
	);
	eprintln!("multinomial_nb_logprob OK: {:?}", result);
}

#[test]
fn test_bayes_bernoulli_nb_logprob() {
	let log_prior = {
		let __up = &[0.5_f64.ln(), 0.5_f64.ln()];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let log_p = {
		let __up = &[0.3_f64.ln(), 0.7_f64.ln(), 0.6_f64.ln(), 0.4_f64.ln()];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let log_neg = {
		let __up = &[0.7_f64.ln(), 0.3_f64.ln(), 0.4_f64.ln(), 0.6_f64.ln()];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let x = {
		let __up = &[1.0_f64, 0.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let out = GpuBuffer::alloc(2).unwrap();
	gpu_bernoulli_nb_logprob(&log_prior, &log_p, &log_neg, &x, 1, 2, 2, &out).unwrap();
	let mut result = [0.0_f64; 2];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let exp0 = 0.5_f64.ln() + 0.3_f64.ln() + 0.3_f64.ln();
	let exp1 = 0.5_f64.ln() + 0.6_f64.ln() + 0.6_f64.ln();
	assert!(result[0].is_finite(), "bernoulli logprob[0] not finite");
	assert!(result[1].is_finite(), "bernoulli logprob[1] not finite");
	assert!(
		(result[0] - exp0).abs() < 1e-9,
		"bernoulli logprob[0]={} expected={}",
		result[0],
		exp0
	);
	assert!(
		(result[1] - exp1).abs() < 1e-9,
		"bernoulli logprob[1]={} expected={}",
		result[1],
		exp1
	);
	eprintln!("bernoulli_nb_logprob OK: {:?}", result);
}

// ── Forest ────────────────────────────────────────────────────────────────────

#[test]
fn test_forest_bootstrap_sample() {
	let n = 10_usize;
	let n_samples = 20_usize;
	let uniform_ws = GpuBuffer::alloc(n_samples).unwrap();
	gpu_rand_uniform_into(42, n_samples, &uniform_ws).unwrap();
	let buf = GpuBuffer::alloc_bytes(n_samples * mem::size_of::<i32>()).unwrap();
	gpu_bootstrap_sample(&uniform_ws, n, n_samples, 42, &buf).unwrap();
	assert_eq!(buf.len(), n_samples * 4, "wrong byte length");
	let mut idx = vec![0_i32; n_samples];
	buf.download_i32(&mut idx).unwrap();
	for &v in &idx {
		assert!(v >= 0 && v < n as i32, "index {} out of [0,{})", v, n);
	}
	eprintln!("bootstrap_sample OK: {:?}", idx);
}

#[test]
fn test_forest_feature_subset() {
	let n_features = 10_usize;
	let k = 4_usize;
	let keys_ws = GpuBuffer::alloc(n_features).unwrap();
	let buf = GpuBuffer::alloc_bytes(n_features * mem::size_of::<i32>()).unwrap();
	gpu_feature_subset(&keys_ws, n_features, k, 7, &buf).unwrap();
	let mut all_idx = vec![0_i32; n_features];
	buf.download_i32(&mut all_idx).unwrap();
	for i in 0..k {
		assert!(
			all_idx[i] >= 0 && all_idx[i] < n_features as i32,
			"feature_subset[{}]={} out of range",
			i,
			all_idx[i]
		);
	}
	let subset: HashSet<i32> = all_idx[..k].iter().cloned().collect();
	assert_eq!(
		subset.len(),
		k,
		"feature_subset has duplicates in first {}: {:?}",
		k,
		&all_idx[..k]
	);
	eprintln!("feature_subset OK (first {}): {:?}", k, &all_idx[..k]);
}

#[test]
fn test_forest_random_threshold_split() {
	let col_data = vec![1.0_f64, 3.0, 2.0, 5.0, 4.0];
	let col = {
		let __up = &col_data;
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let d_min_ws = GpuBuffer::alloc(1).unwrap();
	let d_max_ws = GpuBuffer::alloc(1).unwrap();
	let thr_buf = GpuBuffer::alloc(1).unwrap();
	gpu_random_threshold_split(&col, &d_min_ws, &d_max_ws, col_data.len(), 99, &thr_buf).unwrap();
	let mut thr = [0.0_f64; 1];
	unsafe { thr_buf.download_async(&mut thr, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();
	let threshold = thr[0];
	assert!(threshold.is_finite(), "threshold not finite: {}", threshold);
	let col_min = 1.0_f64;
	let col_max = 5.0_f64;
	assert!(
		threshold >= col_min && threshold <= col_max,
		"threshold {} not in [{}, {}]",
		threshold,
		col_min,
		col_max
	);
	eprintln!("random_threshold_split OK: {}", threshold);
}

#[test]
fn test_forest_oob_mask() {
	let bootstrap = [0_i32, 1, 2, 3];
	let n = 6_usize;
	let bs_buf = GpuBuffer::upload_i32(&bootstrap).unwrap();
	let used_ws = GpuBuffer::alloc_bytes(n).unwrap();
	let mask_buf = GpuBuffer::alloc_bytes(n).unwrap();
	gpu_oob_mask(&bs_buf, &used_ws, bootstrap.len(), n, &mask_buf).unwrap();
	let mut mask = vec![0_u8; n];
	mask_buf.download_u8(&mut mask).unwrap();

	for i in 0..4 {
		assert_eq!(mask[i], 0, "oob[{}] should be 0 (was sampled)", i);
	}
	for i in 4..6 {
		assert_eq!(mask[i], 1, "oob[{}] should be 1 (not sampled)", i);
	}
	eprintln!("oob_mask OK: {:?}", mask);
}

// ── CatBoost ──────────────────────────────────────────────────────────────────

#[test]
fn test_catboost_iota() {
	let n = 8_usize;
	let buf = GpuBuffer::alloc_bytes(n * mem::size_of::<i32>()).unwrap();
	gpu_iota(n, &buf).unwrap();
	let mut out = vec![0_i32; n];
	buf.download_i32(&mut out).unwrap();
	for (i, &v) in out.iter().enumerate() {
		assert_eq!(v, i as i32, "iota[{}]={} expected {}", i, v, i);
	}
	eprintln!("iota OK: {:?}", out);
}

#[test]
fn test_catboost_random_permutation() {
	let n = 16_usize;
	let tmp_bytes = gpu_random_permutation_workspace_bytes(n);
	let keys = GpuBuffer::alloc(n).unwrap();
	let keys_out = GpuBuffer::alloc(n).unwrap();
	let iota_scratch = GpuBuffer::alloc_bytes(n * mem::size_of::<i32>()).unwrap();
	let tmp = GpuBuffer::alloc_bytes(tmp_bytes).unwrap();
	let buf = GpuBuffer::alloc_bytes(n * mem::size_of::<i32>()).unwrap();
	gpu_random_permutation(
		&keys,
		&keys_out,
		&iota_scratch,
		&tmp,
		n,
		42,
		tmp_bytes,
		&buf,
	)
	.unwrap();
	let mut perm = vec![0_i32; n];
	buf.download_i32(&mut perm).unwrap();

	let mut sorted = perm.clone();
	sorted.sort();
	for (i, &v) in sorted.iter().enumerate() {
		assert_eq!(
			v, i as i32,
			"permutation is not a valid permutation at position {}",
			i
		);
	}
	eprintln!("random_permutation OK: {:?}", perm);
}

#[test]
fn test_catboost_ordered_target_stats() {
	let cat_col = GpuBuffer::upload_i32(&[0_i32, 1, 0, 1]).unwrap();
	let target = {
		let __up = &[10.0_f64, 20.0, 30.0, 40.0];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let perm = GpuBuffer::upload_i32(&[0_i32, 1, 2, 3]).unwrap();
	let prior = {
		let __up = &[0.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let smoothing = {
		let __up = &[1.0_f64];
		let __ub = GpuBuffer::alloc(__up.len()).unwrap();
		__ub.load(__up).unwrap();
		__ub
	};
	let cat_sum = {
		let __zb = GpuBuffer::alloc_bytes(2 * mem::size_of::<f64>()).unwrap();
		__zb.memset_zero(2 * mem::size_of::<f64>()).unwrap();
		__zb
	};
	let cat_cnt = {
		let __zb = GpuBuffer::alloc_bytes(2 * mem::size_of::<f64>()).unwrap();
		__zb.memset_zero(2 * mem::size_of::<f64>()).unwrap();
		__zb
	};
	let out = GpuBuffer::alloc(4).unwrap();
	gpu_ordered_target_stats(
		&cat_col, &target, &perm, &prior, &smoothing, &cat_sum, &cat_cnt, 4, 2, &out,
	)
	.unwrap();
	let mut result = [0.0_f64; 4];
	unsafe { out.download_async(&mut result, ptr::null_mut()) }.unwrap();
	gpu_core::hip::device_synchronize().unwrap();

	let expected = [0.0_f64, 0.0, 5.0, 10.0];
	for i in 0..4 {
		assert!(
			result[i].is_finite(),
			"ordered_target_stats[{}] not finite",
			i
		);
		assert!(
			(result[i] - expected[i]).abs() < 1e-9,
			"ordered_target_stats[{}]={} expected {}",
			i,
			result[i],
			expected[i]
		);
	}
	eprintln!("ordered_target_stats OK: {:?}", result);
}

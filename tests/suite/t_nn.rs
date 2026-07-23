use gpu_core::attention::{
	gpu_bn_update_running, gpu_causal_softmax_rows, gpu_embedding_backward, gpu_im2col_2d_ext, gpu_mha_merge,
	gpu_mha_split, gpu_positional_encoding, gpu_rmsnorm, gpu_rmsnorm_backward, gpu_rope, gpu_scaled_dot_product_attn,
};
use gpu_core::memory::GpuBuffer;
use gpu_core::nn_f32::{
	gpu_add_f16, gpu_avg_pool_2d_f32, gpu_bias_add_f32, gpu_gelu_backward_f32, gpu_gelu_f16, gpu_gelu_f32,
	gpu_gru_cell_f32, gpu_layernorm_backward_f32, gpu_layernorm_f32, gpu_linear_f32, gpu_lstm_cell_f32,
	gpu_max_pool_2d_f32, gpu_mul_f16, gpu_relu_backward_f32, gpu_relu_f16, gpu_relu_f32, gpu_sgd_update_f32,
};
use half::f16;

fn sync() {
	unsafe {
		gpu_core::hip::hipDeviceSynchronize();
	}
}

fn abs_err(a: f32, b: f32) -> f32 {
	(a - b).abs()
}

fn tol(expected: f32) -> f32 {
	1e-4 * (1.0 + expected.abs())
}

fn assert_close(got: &[f32], expected: &[f32], label: &str) {
	assert_eq!(got.len(), expected.len(), "{}: length mismatch", label);
	for (i, (&g, &e)) in got.iter().zip(expected.iter()).enumerate() {
		assert!(
			g.is_finite(),
			"{}: got[{}] = {} is not finite (expected {})",
			label,
			i,
			g,
			e
		);
		let err = abs_err(g, e);
		let t = tol(e);
		assert!(
			err <= t,
			"{}: got[{}]={} expected={} abs_err={} tol={}",
			label,
			i,
			g,
			e,
			err,
			t
		);
	}
}

// ── nn_f32 tests ──────────────────────────────────────────────────────────────

#[test]
fn test_linear_f32() {
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
	let w_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
	let bias_data: Vec<f32> = vec![0.1, 0.2];
	let expected: Vec<f32> = vec![4.1, 5.2, 10.1, 11.2];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let w = GpuBuffer::upload_f32(&w_data).unwrap();
	let bias = GpuBuffer::upload_f32(&bias_data).unwrap();

	let out = GpuBuffer::zeros_f32(4).unwrap();
	gpu_linear_f32(&x, &w, &bias, 2, 2, 3, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();

	eprintln!("linear_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_linear_f32");
}

#[test]
fn test_relu_f32() {
	let x_data: Vec<f32> = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
	let expected: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0, 2.0];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let out = GpuBuffer::zeros_f32(5).unwrap();
	gpu_relu_f32(&x, 5, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("relu_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_relu_f32");
}

#[test]
fn test_relu_backward_f32() {
	let grad_data: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0, 1.0];
	let act_data: Vec<f32> = vec![-1.0, 0.0, 0.5, 1.0, 2.0];
	let expected: Vec<f32> = vec![0.0, 0.0, 1.0, 1.0, 1.0];

	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	let act = GpuBuffer::upload_f32(&act_data).unwrap();
	let out = GpuBuffer::zeros_f32(5).unwrap();
	gpu_relu_backward_f32(&grad, &act, 5, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("relu_backward_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_relu_backward_f32");
}

#[test]
fn test_gelu_f32() {
	let x_data: Vec<f32> = vec![0.0, 1.0, -1.0, 2.0, -2.0];
	let expected: Vec<f32> = {
		fn gelu(x: f32) -> f32 {
			const C: f32 = 0.797_884_6;
			const CUB: f32 = 0.044715;
			0.5 * x * (1.0 + ((C * (x + CUB * x * x * x)).tanh()))
		}
		vec![gelu(0.0), gelu(1.0), gelu(-1.0), gelu(2.0), gelu(-2.0)]
	};

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let out = GpuBuffer::zeros_f32(5).unwrap();
	gpu_gelu_f32(&x, 5, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("gelu_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_gelu_f32");
}

#[test]
fn test_gelu_backward_f32() {
	let x_data: Vec<f32> = vec![0.0, 1.0, -1.0];
	let grad_data: Vec<f32> = vec![1.0, 1.0, 1.0];
	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	let out = GpuBuffer::zeros_f32(3).unwrap();
	gpu_gelu_backward_f32(&grad, &x, 3, &out).unwrap();
	sync();
	let mut got = vec![0.0f32; 3];
	out.download_f32(&mut got).unwrap();
	eprintln!("gelu_backward_f32 got: {:?}", got);
	for &v in &got {
		assert!(v.is_finite(), "gelu_backward non-finite: {}", v);
	}
}

#[test]
fn test_layernorm_f32() {
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 2.0, 4.0, 6.0, 8.0];
	let gamma_data: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0];
	let beta_data: Vec<f32> = vec![0.0, 0.0, 0.0, 0.0];
	let eps = 1e-5_f32;

	fn layernorm_cpu(row: &[f32], eps: f32) -> Vec<f32> {
		let mean = row.iter().sum::<f32>() / row.len() as f32;
		let var = row.iter().map(|&x| (x - mean) * (x - mean)).sum::<f32>() / row.len() as f32;
		let inv_std = 1.0 / (var + eps).sqrt();
		row.iter().map(|&x| (x - mean) * inv_std).collect()
	}

	let mut expected = layernorm_cpu(&x_data[0..4], eps);
	expected.extend(layernorm_cpu(&x_data[4..8], eps));

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let beta = GpuBuffer::upload_f32(&beta_data).unwrap();
	let eps_buf = GpuBuffer::upload_f32(&[eps]).unwrap();
	let out = GpuBuffer::zeros_f32(8).unwrap();
	gpu_layernorm_f32(&x, &gamma, &beta, &eps_buf, 2, 4, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 8];
	out.download_f32(&mut got).unwrap();
	eprintln!("layernorm_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_layernorm_f32");
}

#[test]
fn test_layernorm_backward_f32() {
	let rows = 2usize;
	let cols = 4usize;
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, -1.0, 0.0, 1.0, 2.0];
	let gamma_data: Vec<f32> = vec![1.0; cols];
	let grad_y_data: Vec<f32> = vec![1.0; rows * cols];
	let eps = 1e-5_f32;

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let grad_y = GpuBuffer::upload_f32(&grad_y_data).unwrap();

	let eps_buf = GpuBuffer::upload_f32(&[eps]).unwrap();
	let grad_x = GpuBuffer::zeros_f32(rows * cols).unwrap();
	let grad_gamma = GpuBuffer::zeros_f32(cols).unwrap();
	let grad_beta = GpuBuffer::zeros_f32(cols).unwrap();
	gpu_layernorm_backward_f32(
		&grad_y,
		&x,
		&gamma,
		&eps_buf,
		rows,
		cols,
		&grad_x,
		&grad_gamma,
		&grad_beta,
	)
	.unwrap();
	sync();

	let mut gx = vec![0.0f32; rows * cols];
	let mut gg = vec![0.0f32; cols];
	let mut gb = vec![0.0f32; cols];
	grad_x.download_f32(&mut gx).unwrap();
	grad_gamma.download_f32(&mut gg).unwrap();
	grad_beta.download_f32(&mut gb).unwrap();

	eprintln!("layernorm_backward grad_x: {:?}", gx);
	eprintln!("layernorm_backward grad_gamma: {:?}", gg);
	eprintln!("layernorm_backward grad_beta: {:?}", gb);

	for row in 0..rows {
		let sum: f32 = gx[row * cols..(row + 1) * cols].iter().sum();
		assert!(
			sum.abs() < 1e-3,
			"layernorm_backward grad_x row {} sum = {} (should be ~0)",
			row,
			sum
		);
	}
	for &v in &gb {
		assert!(v.is_finite(), "grad_beta non-finite");
	}
	for &v in &gg {
		assert!(v.is_finite(), "grad_gamma non-finite");
	}
}

#[test]
fn test_bias_add_f32() {
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
	let bias_data: Vec<f32> = vec![10.0, 20.0, 30.0];
	let expected: Vec<f32> = vec![11.0, 22.0, 33.0, 14.0, 25.0, 36.0];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let bias = GpuBuffer::upload_f32(&bias_data).unwrap();
	let out = GpuBuffer::zeros_f32(6).unwrap();
	gpu_bias_add_f32(&x, &bias, 2, 3, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 6];
	out.download_f32(&mut got).unwrap();
	eprintln!("bias_add_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_bias_add_f32");
}

#[test]
fn test_avg_pool_2d_f32() {
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	let expected: Vec<f32> = vec![3.5, 5.5, 11.5, 13.5];

	let x = GpuBuffer::upload_f32(&input).unwrap();
	let out = GpuBuffer::zeros_f32(4).unwrap();
	gpu_avg_pool_2d_f32(&x, 1, 1, 4, 4, 2, 2, 2, 2, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("avg_pool_2d got: {:?}", got);
	assert_close(&got, &expected, "gpu_avg_pool_2d_f32");
}

#[test]
fn test_max_pool_2d_f32() {
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	let expected: Vec<f32> = vec![6.0, 8.0, 14.0, 16.0];

	let x = GpuBuffer::upload_f32(&input).unwrap();
	let out_vals = GpuBuffer::zeros_f32(4).unwrap();
	let out_idx = GpuBuffer::zeros_f32(4).unwrap();
	gpu_max_pool_2d_f32(&x, 1, 1, 4, 4, 2, 2, 2, 2, &out_vals, &out_idx).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out_vals.download_f32(&mut got).unwrap();
	eprintln!("max_pool_2d got: {:?}", got);
	assert_close(&got, &expected, "gpu_max_pool_2d_f32");
}

#[test]
fn test_lstm_cell_f32() {
	let gates_data: Vec<f32> = vec![0.0, 0.0, 1.0, 1.0, 0.5, -0.5, 0.0, 0.0];
	let c_data: Vec<f32> = vec![0.0, 0.0];
	let h_data: Vec<f32> = vec![0.0, 0.0];

	let gates = GpuBuffer::upload_f32(&gates_data).unwrap();
	let c = GpuBuffer::upload_f32(&c_data).unwrap();
	let h = GpuBuffer::upload_f32(&h_data).unwrap();

	gpu_lstm_cell_f32(&gates, 1, 2, &c, &h).unwrap();
	sync();

	let mut c_got = vec![0.0f32; 2];
	let mut h_got = vec![0.0f32; 2];
	c.download_f32(&mut c_got).unwrap();
	h.download_f32(&mut h_got).unwrap();

	eprintln!("lstm c_got: {:?}", c_got);
	eprintln!("lstm h_got: {:?}", h_got);

	let expected_c: Vec<f32> = vec![0.33783475, -0.33783475];
	let expected_h: Vec<f32> = vec![0.1627715, -0.1627715];
	assert_close(&c_got, &expected_c, "lstm_cell c");
	assert_close(&h_got, &expected_h, "lstm_cell h");
}

#[test]
fn test_gru_cell_f32() {
	let gates_data: Vec<f32> = vec![0.0, 0.0, 0.5, 0.5, 1.0, -1.0, 0.2, -0.2];
	let h_data: Vec<f32> = vec![1.0, -1.0];

	let gates = GpuBuffer::upload_f32(&gates_data).unwrap();
	let h = GpuBuffer::upload_f32(&h_data).unwrap();

	let h_new = GpuBuffer::zeros_f32(2).unwrap();
	gpu_gru_cell_f32(&gates, &h, 1, 2, &h_new).unwrap();
	sync();

	let mut got = vec![0.0f32; 2];
	h_new.download_f32(&mut got).unwrap();
	eprintln!("gru h_new: {:?}", got);

	let expected: Vec<f32> = vec![0.90456283, -0.90456283];
	assert_close(&got, &expected, "gpu_gru_cell_f32");
}

#[test]
fn test_relu_f16() {
	let x_data: Vec<f16> = vec![f16::from_f32(-1.0), f16::from_f32(0.0), f16::from_f32(2.0)];
	let expected: Vec<f32> = vec![0.0, 0.0, 2.0];

	let x = GpuBuffer::upload_f16(&x_data).unwrap();
	let out = GpuBuffer::alloc_bytes(3 * 2).unwrap();
	gpu_relu_f16(&x, 3, &out).unwrap();
	sync();

	let mut got_f16 = vec![f16::ZERO; 3];
	out.download_f16(&mut got_f16).unwrap();
	let got: Vec<f32> = got_f16.iter().map(|h| h.to_f32()).collect();
	eprintln!("relu_f16 got: {:?}", got);
	assert_close(&got, &expected, "gpu_relu_f16");
}

#[test]
fn test_gelu_f16() {
	let x_data: Vec<f16> = vec![f16::from_f32(0.0), f16::from_f32(1.0), f16::from_f32(-1.0)];
	fn gelu(x: f32) -> f32 {
		const C: f32 = 0.797_884_6;
		const CUB: f32 = 0.044715;
		0.5 * x * (1.0 + ((C * (x + CUB * x * x * x)).tanh()))
	}
	let expected: Vec<f32> = vec![gelu(0.0), gelu(1.0), gelu(-1.0)];

	let x = GpuBuffer::upload_f16(&x_data).unwrap();
	let out = GpuBuffer::alloc_bytes(3 * 2).unwrap();
	gpu_gelu_f16(&x, 3, &out).unwrap();
	sync();

	let mut got_f16 = vec![f16::ZERO; 3];
	out.download_f16(&mut got_f16).unwrap();
	let got: Vec<f32> = got_f16.iter().map(|h| h.to_f32()).collect();
	eprintln!("gelu_f16 got: {:?}", got);
	for (i, (&g, &e)) in got.iter().zip(expected.iter()).enumerate() {
		assert!(g.is_finite(), "gelu_f16 got[{}] not finite", i);
		assert!(
			(g - e).abs() <= 1e-2 * (1.0 + e.abs()),
			"gelu_f16[{}]: got={} expected={}",
			i,
			g,
			e
		);
	}
}

#[test]
fn test_add_f16() {
	let a: Vec<f16> = vec![f16::from_f32(1.0), f16::from_f32(2.0), f16::from_f32(3.0)];
	let b: Vec<f16> = vec![f16::from_f32(0.5), f16::from_f32(1.5), f16::from_f32(-1.0)];
	let expected: Vec<f32> = vec![1.5, 3.5, 2.0];

	let ga = GpuBuffer::upload_f16(&a).unwrap();
	let gb = GpuBuffer::upload_f16(&b).unwrap();
	let out = GpuBuffer::alloc_bytes(3 * 2).unwrap();
	gpu_add_f16(&ga, &gb, 3, &out).unwrap();
	sync();

	let mut got_f16 = vec![f16::ZERO; 3];
	out.download_f16(&mut got_f16).unwrap();
	let got: Vec<f32> = got_f16.iter().map(|h| h.to_f32()).collect();
	eprintln!("add_f16 got: {:?}", got);
	for (i, (&g, &e)) in got.iter().zip(expected.iter()).enumerate() {
		assert!(g.is_finite(), "add_f16 got[{}] not finite", i);
		assert!(
			(g - e).abs() <= 1e-2 * (1.0 + e.abs()),
			"add_f16[{}]: got={} expected={}",
			i,
			g,
			e
		);
	}
}

#[test]
fn test_mul_f16() {
	let a: Vec<f16> = vec![f16::from_f32(2.0), f16::from_f32(3.0), f16::from_f32(-1.0)];
	let b: Vec<f16> = vec![f16::from_f32(3.0), f16::from_f32(0.5), f16::from_f32(4.0)];
	let expected: Vec<f32> = vec![6.0, 1.5, -4.0];

	let ga = GpuBuffer::upload_f16(&a).unwrap();
	let gb = GpuBuffer::upload_f16(&b).unwrap();
	let out = GpuBuffer::alloc_bytes(3 * 2).unwrap();
	gpu_mul_f16(&ga, &gb, 3, &out).unwrap();
	sync();

	let mut got_f16 = vec![f16::ZERO; 3];
	out.download_f16(&mut got_f16).unwrap();
	let got: Vec<f32> = got_f16.iter().map(|h| h.to_f32()).collect();
	eprintln!("mul_f16 got: {:?}", got);
	for (i, (&g, &e)) in got.iter().zip(expected.iter()).enumerate() {
		assert!(g.is_finite(), "mul_f16 got[{}] not finite", i);
		assert!(
			(g - e).abs() <= 1e-2 * (1.0 + e.abs()),
			"mul_f16[{}]: got={} expected={}",
			i,
			g,
			e
		);
	}
}

#[test]
fn test_sgd_update_f32() {
	let w_data: Vec<f32> = vec![1.0, 2.0, 3.0];
	let grad_data: Vec<f32> = vec![0.1, 0.2, 0.3];
	let lr = 0.1_f32;
	let expected: Vec<f32> = vec![0.99, 1.98, 2.97];

	let w = GpuBuffer::upload_f32(&w_data).unwrap();
	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	let lr_buf = GpuBuffer::upload_f32(&[lr]).unwrap();
	gpu_sgd_update_f32(&grad, &lr_buf, 3, &w).unwrap();
	sync();

	let mut got = vec![0.0f32; 3];
	w.download_f32(&mut got).unwrap();
	eprintln!("sgd_update got: {:?}", got);
	assert_close(&got, &expected, "gpu_sgd_update_f32");
}

// ── attention tests ───────────────────────────────────────────────────────────

#[test]
fn test_sdpa_noncausal() {
	let q_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let k_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let v_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];

	let q = GpuBuffer::upload_f32(&q_data).unwrap();
	let k = GpuBuffer::upload_f32(&k_data).unwrap();
	let v = GpuBuffer::upload_f32(&v_data).unwrap();

	let out = GpuBuffer::zeros_f32(4).unwrap();
	gpu_scaled_dot_product_attn(&q, &k, &v, 1, 2, 2, 0, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("sdpa non-causal got: {:?}", got);

	let expected: Vec<f32> = vec![1.6604769, 2.660_477, 2.339_523, 3.339_523];
	assert_close(&got, &expected, "sdpa_noncausal");
}

#[test]
fn test_sdpa_causal() {
	let q_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let k_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let v_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];

	let q = GpuBuffer::upload_f32(&q_data).unwrap();
	let k = GpuBuffer::upload_f32(&k_data).unwrap();
	let v = GpuBuffer::upload_f32(&v_data).unwrap();

	let out = GpuBuffer::zeros_f32(4).unwrap();
	gpu_scaled_dot_product_attn(&q, &k, &v, 1, 2, 2, 1, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("sdpa causal got: {:?}", got);

	let expected: Vec<f32> = vec![1.0, 2.0, 2.339_523, 3.339_523];
	assert_close(&got, &expected, "sdpa_causal");
}

#[test]
fn test_causal_softmax_rows() {
	let x_data: Vec<f32> = vec![1.0; 9];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	gpu_causal_softmax_rows(3, 3, &x).unwrap();
	sync();

	let mut got = vec![0.0f32; 9];
	x.download_f32(&mut got).unwrap();
	eprintln!("causal_softmax got: {:?}", got);

	assert!(
		(got[0] - 1.0).abs() < 1e-4,
		"row0[0] should be 1.0 got {}",
		got[0]
	);
	assert!(
		(got[1]).abs() < 1e-4,
		"row0[1] should be 0.0 got {}",
		got[1]
	);
	assert!(
		(got[2]).abs() < 1e-4,
		"row0[2] should be 0.0 got {}",
		got[2]
	);
	assert!(
		(got[3] - 0.5).abs() < 1e-4,
		"row1[0] should be 0.5 got {}",
		got[3]
	);
	assert!(
		(got[4] - 0.5).abs() < 1e-4,
		"row1[1] should be 0.5 got {}",
		got[4]
	);
	assert!(
		(got[5]).abs() < 1e-4,
		"row1[2] should be 0.0 got {}",
		got[5]
	);
	for j in 6..9 {
		assert!(
			(got[j] - 1.0 / 3.0).abs() < 1e-4,
			"row2[{}] should be 1/3 got {}",
			j - 6,
			got[j]
		);
	}
}

#[test]
fn test_mha_split_merge_roundtrip() {
	let n_heads = 2usize;
	let head_dim = 3usize;
	let seq = 4usize;
	let data: Vec<f32> = (0..(seq * n_heads * head_dim)).map(|i| i as f32).collect();

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let split = GpuBuffer::zeros_f32(seq * n_heads * head_dim).unwrap();
	gpu_mha_split(&x, seq, n_heads, head_dim, &split).unwrap();
	sync();

	let merged = GpuBuffer::zeros_f32(seq * n_heads * head_dim).unwrap();
	gpu_mha_merge(&split, seq, n_heads, head_dim, &merged).unwrap();
	sync();

	let mut got = vec![0.0f32; data.len()];
	merged.download_f32(&mut got).unwrap();
	eprintln!("mha round-trip got: {:?}", got);
	assert_close(&got, &data, "mha_split_merge_roundtrip");
}

#[test]
fn test_mha_split_layout() {
	let data: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
	let expected: Vec<f32> = vec![0.0, 1.0, 4.0, 5.0, 2.0, 3.0, 6.0, 7.0];

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let split = GpuBuffer::zeros_f32(8).unwrap();
	gpu_mha_split(&x, 2, 2, 2, &split).unwrap();
	sync();

	let mut got = vec![0.0f32; 8];
	split.download_f32(&mut got).unwrap();
	eprintln!("mha_split layout got: {:?}", got);
	assert_close(&got, &expected, "mha_split_layout");
}

#[test]
fn test_rope_norm_preserved() {
	let seq = 3usize;
	let dim = 4usize;
	let data: Vec<f32> = vec![
		1.0, 0.0, 0.0, 1.0, // s=0
		2.0, 1.0, -1.0, 2.0, // s=1
		0.0, 3.0, 1.0, 0.0,
	]; // s=2

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let base = GpuBuffer::upload_f32(&[10000.0]).unwrap();
	let out = GpuBuffer::zeros_f32(seq * dim).unwrap();
	gpu_rope(&x, seq, dim, &base, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; seq * dim];
	out.download_f32(&mut got).unwrap();
	eprintln!("rope got: {:?}", got);

	let half = dim / 2;
	for s in 0..seq {
		for d in 0..half {
			let x0 = data[s * dim + d];
			let x1 = data[s * dim + d + half];
			let y0 = got[s * dim + d];
			let y1 = got[s * dim + d + half];
			let norm_x = (x0 * x0 + x1 * x1).sqrt();
			let norm_y = (y0 * y0 + y1 * y1).sqrt();
			assert!(got[s * dim + d].is_finite(), "rope output non-finite");
			assert!(
				(norm_x - norm_y).abs() < 1e-4 * (1.0 + norm_x),
				"rope s={} d={}: norm_x={} norm_y={}",
				s,
				d,
				norm_x,
				norm_y
			);
		}
	}
}

#[test]
fn test_positional_encoding() {
	let seq = 4usize;
	let dim = 8usize;
	let out = GpuBuffer::zeros_f32(seq * dim).unwrap();
	gpu_positional_encoding(seq, dim, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; seq * dim];
	out.download_f32(&mut got).unwrap();
	eprintln!("pos_enc (4,8): {:?}", got);

	for (i, &v) in got.iter().enumerate() {
		assert!(v.is_finite(), "pos_enc[{}] not finite", i);
		assert!(v.abs() <= 1.0 + 1e-5, "pos_enc[{}]={} out of [-1,1]", i, v);
	}

	for d in (0..dim).step_by(2) {
		assert!(
			(got[d]).abs() < 1e-5,
			"pos_enc[0,{}] (even, should be sin(0)=0) got {}",
			d,
			got[d]
		);
	}
	for d in (1..dim).step_by(2) {
		assert!(
			(got[d] - 1.0).abs() < 1e-5,
			"pos_enc[0,{}] (odd, should be cos(0)=1) got {}",
			d,
			got[d]
		);
	}
}

#[test]
fn test_rmsnorm() {
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
	let gamma_data: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0];
	let eps_buf = GpuBuffer::upload_f32(&[1e-5_f32]).unwrap();

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let out = GpuBuffer::zeros_f32(4).unwrap();
	gpu_rmsnorm(&x, &gamma, &eps_buf, 1, 4, &out).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("rmsnorm got: {:?}", got);

	let expected: Vec<f32> = vec![0.36514813, 0.73029625, 1.0954444, 1.4605925];
	assert_close(&got, &expected, "gpu_rmsnorm");
}

#[test]
fn test_rmsnorm_backward() {
	let rows = 2usize;
	let cols = 4usize;
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 1.0, 1.0, 1.0, 1.0];
	let gamma_data: Vec<f32> = vec![1.0; cols];
	let grad_out_data: Vec<f32> = vec![1.0; rows * cols];
	let eps_buf = GpuBuffer::upload_f32(&[1e-5_f32]).unwrap();

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let grad_out = GpuBuffer::upload_f32(&grad_out_data).unwrap();

	let grad_x = GpuBuffer::zeros_f32(rows * cols).unwrap();
	let grad_gamma = GpuBuffer::zeros_f32(cols).unwrap();
	gpu_rmsnorm_backward(
		&grad_out,
		&x,
		&gamma,
		&eps_buf,
		rows,
		cols,
		&grad_x,
		&grad_gamma,
	)
	.unwrap();
	sync();

	let mut gx = vec![0.0f32; rows * cols];
	let mut gg = vec![0.0f32; cols];
	grad_x.download_f32(&mut gx).unwrap();
	grad_gamma.download_f32(&mut gg).unwrap();

	eprintln!("rmsnorm_backward grad_x: {:?}", gx);
	eprintln!("rmsnorm_backward grad_gamma: {:?}", gg);

	for &v in &gx {
		assert!(v.is_finite(), "rmsnorm grad_x non-finite");
	}
	for &v in &gg {
		assert!(v.is_finite(), "rmsnorm grad_gamma non-finite");
	}

	let gx_sum: f32 = gx.iter().sum();
	assert!(gx_sum.is_finite(), "rmsnorm grad_x sum non-finite");
}

#[test]
fn test_im2col_2d_ext() {
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	let n = 1usize;
	let c = 1usize;
	let h = 4usize;
	let w = 4usize;
	let kh = 2usize;
	let kw = 2usize;
	let sh = 2usize;
	let sw = 2usize;
	let pad_h = 1usize;
	let pad_w = 1usize;
	let dil_h = 1usize;
	let dil_w = 1usize;

	let out_h = (h + 2 * pad_h - dil_h * (kh - 1) - 1) / sh + 1;
	let out_w = (w + 2 * pad_w - dil_w * (kw - 1) - 1) / sw + 1;
	assert_eq!(out_h, 3);
	assert_eq!(out_w, 3);

	let x = GpuBuffer::upload_f32(&input).unwrap();
	let patches = GpuBuffer::zeros_f32(n * out_h * out_w * c * kh * kw).unwrap();
	gpu_im2col_2d_ext(
		&x, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w, &patches,
	)
	.unwrap();
	sync();

	let patch_count = n * out_h * out_w;
	let patch_size = c * kh * kw;
	let mut got = vec![0.0f32; patch_count * patch_size];
	patches.download_f32(&mut got).unwrap();
	eprintln!("im2col patches (9x4):");
	for r in 0..9 {
		eprintln!("  patch[{}]: {:?}", r, &got[r * 4..(r + 1) * 4]);
	}

	let expected_patch0: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0];
	let expected_patch4: Vec<f32> = vec![6.0, 7.0, 10.0, 11.0];
	let expected_patch8: Vec<f32> = vec![16.0, 0.0, 0.0, 0.0];
	assert_close(&got[0..4], &expected_patch0, "im2col patch[0]");
	assert_close(&got[16..20], &expected_patch4, "im2col patch[4]");
	assert_close(&got[32..36], &expected_patch8, "im2col patch[8]");

	assert_eq!(got.len(), 9 * 4, "im2col output size wrong");
}

#[test]
fn test_embedding_backward() {
	let grad_out_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
	let indices_i32: Vec<i32> = vec![0, 1, 0];
	let n = 3usize;
	let cols = 2usize;
	let vocab = 2usize;

	let grad_out = GpuBuffer::upload_f32(&grad_out_data).unwrap();
	let indices = GpuBuffer::upload_i32(&indices_i32).unwrap();

	let grad_table = GpuBuffer::zeros_f32(vocab * cols).unwrap();
	gpu_embedding_backward(&grad_out, &indices, n, cols, vocab, &grad_table).unwrap();
	sync();

	let mut got = vec![0.0f32; vocab * cols];
	grad_table.download_f32(&mut got).unwrap();
	eprintln!("embedding_backward got: {:?}", got);

	let expected: Vec<f32> = vec![6.0, 8.0, 3.0, 4.0];
	assert_close(&got, &expected, "gpu_embedding_backward");
}

#[test]
fn test_bn_update_running() {
	let run_mean_data: Vec<f32> = vec![0.0, 0.0];
	let run_var_data: Vec<f32> = vec![1.0, 1.0];
	let save_mean_data: Vec<f32> = vec![2.0, -2.0];
	let save_var_data: Vec<f32> = vec![0.5, 0.5];
	let momentum = GpuBuffer::upload_f32(&[0.1_f32]).unwrap();

	let run_mean = GpuBuffer::upload_f32(&run_mean_data).unwrap();
	let run_var = GpuBuffer::upload_f32(&run_var_data).unwrap();
	let save_mean = GpuBuffer::upload_f32(&save_mean_data).unwrap();
	let save_var = GpuBuffer::upload_f32(&save_var_data).unwrap();

	gpu_bn_update_running(&save_mean, &save_var, &momentum, 2, &run_mean, &run_var).unwrap();
	sync();

	let mut got_mean = vec![0.0f32; 2];
	let mut got_var = vec![0.0f32; 2];
	run_mean.download_f32(&mut got_mean).unwrap();
	run_var.download_f32(&mut got_var).unwrap();
	eprintln!("bn_update_running mean: {:?}", got_mean);
	eprintln!("bn_update_running var: {:?}", got_var);

	let expected_mean: Vec<f32> = vec![0.2, -0.2];
	let expected_var: Vec<f32> = vec![0.95, 0.95];
	assert_close(&got_mean, &expected_mean, "bn_update_running mean");
	assert_close(&got_var, &expected_var, "bn_update_running var");
}

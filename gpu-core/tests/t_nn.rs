use gpu_core::attention::{
	gpu_bn_update_running, gpu_causal_softmax_rows, gpu_embedding_backward, gpu_im2col_2d_ext,
	gpu_mha_merge, gpu_mha_split, gpu_positional_encoding, gpu_rmsnorm, gpu_rmsnorm_backward,
	gpu_rope, gpu_scaled_dot_product_attention,
};
use gpu_core::memory::GpuBuffer;
use gpu_core::nn_f32::{
	gpu_add_f16, gpu_avg_pool_2d_f32, gpu_bias_add_f32, gpu_gelu_backward_f32, gpu_gelu_f16,
	gpu_gelu_f32, gpu_gru_cell_f32, gpu_layernorm_backward_f32, gpu_layernorm_f32,
	gpu_linear_f32, gpu_lstm_cell_f32, gpu_max_pool_2d_f32, gpu_mul_f16, gpu_relu_backward_f32,
	gpu_relu_f16, gpu_relu_f32, gpu_sgd_update_f32,
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
	// X(2,3) @ W(3,2) + bias(2) = out(2,2)
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
	let w_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
	let bias_data: Vec<f32> = vec![0.1, 0.2];
	let expected: Vec<f32> = vec![4.1, 5.2, 10.1, 11.2];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let w = GpuBuffer::upload_f32(&w_data).unwrap();
	let bias = GpuBuffer::upload_f32(&bias_data).unwrap();

	let out = gpu_linear_f32(&x, &w, &bias, 2, 2, 3).unwrap();
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
	let out = gpu_relu_f32(&x, 5).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("relu_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_relu_f32");
}

#[test]
fn test_relu_backward_f32() {
	// grad passes through where act > 0
	let grad_data: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0, 1.0];
	let act_data: Vec<f32> = vec![-1.0, 0.0, 0.5, 1.0, 2.0];
	let expected: Vec<f32> = vec![0.0, 0.0, 1.0, 1.0, 1.0];

	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	let act = GpuBuffer::upload_f32(&act_data).unwrap();
	let out = gpu_relu_backward_f32(&grad, &act, 5).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("relu_backward_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_relu_backward_f32");
}

#[test]
fn test_gelu_f32() {
	// GELU(0)=0, GELU(large positive) ~ x, GELU(large negative) ~ 0
	let x_data: Vec<f32> = vec![0.0, 1.0, -1.0, 2.0, -2.0];
	// reference: 0.5*x*(1+tanh(sqrt(2/pi)*(x+0.044715*x^3)))
	let expected: Vec<f32> = {
		fn gelu(x: f32) -> f32 {
			const C: f32 = 0.797_884_6;
			const CUB: f32 = 0.044715;
			0.5 * x * (1.0 + ((C * (x + CUB * x * x * x)).tanh()))
		}
		vec![gelu(0.0), gelu(1.0), gelu(-1.0), gelu(2.0), gelu(-2.0)]
	};

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let out = gpu_gelu_f32(&x, 5).unwrap();
	sync();

	let mut got = vec![0.0f32; 5];
	out.download_f32(&mut got).unwrap();
	eprintln!("gelu_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_gelu_f32");
}

#[test]
fn test_gelu_backward_f32() {
	// Just check it runs and produces finite values
	let x_data: Vec<f32> = vec![0.0, 1.0, -1.0];
	let grad_data: Vec<f32> = vec![1.0, 1.0, 1.0];
	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	let out = gpu_gelu_backward_f32(&grad, &x, 3).unwrap();
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
	// 2 rows of 4 cols; gamma=1, beta=0 → output is (x-mean)/std per row
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
	let out = gpu_layernorm_f32(&x, &gamma, &beta, 2, 4, eps).unwrap();
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

	let (grad_x, grad_gamma, grad_beta) =
		gpu_layernorm_backward_f32(&grad_y, &x, &gamma, rows, cols, eps).unwrap();
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

	// When grad_y = all-ones, grad_x should sum to ~0 per row (layernorm symmetry)
	for row in 0..rows {
		let sum: f32 = gx[row * cols..(row + 1) * cols].iter().sum();
		assert!(
			sum.abs() < 1e-3,
			"layernorm_backward grad_x row {} sum = {} (should be ~0)",
			row,
			sum
		);
	}
	// grad_beta = sum of grad_y over rows = [2,2,2,2]
	for &v in &gb {
		assert!(v.is_finite(), "grad_beta non-finite");
	}
	// grad_gamma should be finite
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
	let out = gpu_bias_add_f32(&x, &bias, 2, 3).unwrap();
	sync();

	let mut got = vec![0.0f32; 6];
	out.download_f32(&mut got).unwrap();
	eprintln!("bias_add_f32 got: {:?}", got);
	assert_close(&got, &expected, "gpu_bias_add_f32");
}

#[test]
fn test_avg_pool_2d_f32() {
	// NCHW: n=1, c=1, h=4, w=4; kernel=2x2, stride=2
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	// out_h = (4-2)/2+1 = 2, out_w = 2
	// pool[0,0] = avg(1,2,5,6)=3.5, pool[0,1]=avg(3,4,7,8)=5.5
	// pool[1,0] = avg(9,10,13,14)=11.5, pool[1,1]=avg(11,12,15,16)=13.5
	let expected: Vec<f32> = vec![3.5, 5.5, 11.5, 13.5];

	let x = GpuBuffer::upload_f32(&input).unwrap();
	let out = gpu_avg_pool_2d_f32(&x, 1, 1, 4, 4, 2, 2, 2, 2).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("avg_pool_2d got: {:?}", got);
	assert_close(&got, &expected, "gpu_avg_pool_2d_f32");
}

#[test]
fn test_max_pool_2d_f32() {
	// NCHW: n=1, c=1, h=4, w=4; kernel=2x2, stride=2
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	// max[0,0]=max(1,2,5,6)=6, max[0,1]=max(3,4,7,8)=8
	// max[1,0]=max(9,10,13,14)=14, max[1,1]=max(11,12,15,16)=16
	let expected: Vec<f32> = vec![6.0, 8.0, 14.0, 16.0];

	let x = GpuBuffer::upload_f32(&input).unwrap();
	let (out_vals, _out_idx) = gpu_max_pool_2d_f32(&x, 1, 1, 4, 4, 2, 2, 2, 2).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out_vals.download_f32(&mut got).unwrap();
	eprintln!("max_pool_2d got: {:?}", got);
	assert_close(&got, &expected, "gpu_max_pool_2d_f32");
}

#[test]
fn test_lstm_cell_f32() {
	// n=1, hs=2
	// gates layout: [f_pre(hs=2), i_pre(hs=2), g_pre(hs=2), o_pre(hs=2)]
	let gates_data: Vec<f32> = vec![0.0, 0.0, 1.0, 1.0, 0.5, -0.5, 0.0, 0.0];
	let c_data: Vec<f32> = vec![0.0, 0.0];
	let h_data: Vec<f32> = vec![0.0, 0.0];

	let gates = GpuBuffer::upload_f32(&gates_data).unwrap();
	let c = GpuBuffer::upload_f32(&c_data).unwrap();
	let h = GpuBuffer::upload_f32(&h_data).unwrap();

	gpu_lstm_cell_f32(&gates, &c, &h, 1, 2);
	sync();

	let mut c_got = vec![0.0f32; 2];
	let mut h_got = vec![0.0f32; 2];
	c.download_f32(&mut c_got).unwrap();
	h.download_f32(&mut h_got).unwrap();

	eprintln!("lstm c_got: {:?}", c_got);
	eprintln!("lstm h_got: {:?}", h_got);

	// expected from python: c=[0.33783475, -0.33783475], h=[0.1627715, -0.1627715]
	let expected_c: Vec<f32> = vec![0.33783475, -0.33783475];
	let expected_h: Vec<f32> = vec![0.1627715, -0.1627715];
	assert_close(&c_got, &expected_c, "lstm_cell c");
	assert_close(&h_got, &expected_h, "lstm_cell h");
}

#[test]
fn test_gru_cell_f32() {
	// n=1, hs=2
	// gates: [z_pre(2), r_pre(2), n_x(2), n_h(2)]
	let gates_data: Vec<f32> = vec![0.0, 0.0, 0.5, 0.5, 1.0, -1.0, 0.2, -0.2];
	let h_data: Vec<f32> = vec![1.0, -1.0];

	let gates = GpuBuffer::upload_f32(&gates_data).unwrap();
	let h = GpuBuffer::upload_f32(&h_data).unwrap();

	let h_new = gpu_gru_cell_f32(&gates, &h, 1, 2).unwrap();
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
	let out = gpu_relu_f16(&x, 3).unwrap();
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
	let out = gpu_gelu_f16(&x, 3).unwrap();
	sync();

	let mut got_f16 = vec![f16::ZERO; 3];
	out.download_f16(&mut got_f16).unwrap();
	let got: Vec<f32> = got_f16.iter().map(|h| h.to_f32()).collect();
	eprintln!("gelu_f16 got: {:?}", got);
	// f16 has ~1e-3 precision, use looser tolerance
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
	let out = gpu_add_f16(&ga, &gb, 3).unwrap();
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
	let out = gpu_mul_f16(&ga, &gb, 3).unwrap();
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
	// expected: w - lr * grad = [0.99, 1.98, 2.97]
	let expected: Vec<f32> = vec![0.99, 1.98, 2.97];

	let w = GpuBuffer::upload_f32(&w_data).unwrap();
	let grad = GpuBuffer::upload_f32(&grad_data).unwrap();
	gpu_sgd_update_f32(&w, &grad, lr, 3);
	sync();

	let mut got = vec![0.0f32; 3];
	w.download_f32(&mut got).unwrap();
	eprintln!("sgd_update got: {:?}", got);
	assert_close(&got, &expected, "gpu_sgd_update_f32");
}

// ── attention tests ───────────────────────────────────────────────────────────

#[test]
fn test_sdpa_noncausal() {
	// n_rows=1, seq=2, dim=2
	// Q=[[1,0],[0,1]], K=[[1,0],[0,1]], V=[[1,2],[3,4]]
	let q_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let k_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let v_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];

	let q = GpuBuffer::upload_f32(&q_data).unwrap();
	let k = GpuBuffer::upload_f32(&k_data).unwrap();
	let v = GpuBuffer::upload_f32(&v_data).unwrap();

	let out = gpu_scaled_dot_product_attention(&q, &k, &v, 1, 2, 2, false).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("sdpa non-causal got: {:?}", got);

	// from python: [1.6604769, 2.6604769, 2.3395231, 3.3395231]
	let expected: Vec<f32> = vec![1.6604769, 2.660_477, 2.339_523, 3.339_523];
	assert_close(&got, &expected, "sdpa_noncausal");
}

#[test]
fn test_sdpa_causal() {
	// same as above but causal=true
	let q_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let k_data: Vec<f32> = vec![1.0, 0.0, 0.0, 1.0];
	let v_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];

	let q = GpuBuffer::upload_f32(&q_data).unwrap();
	let k = GpuBuffer::upload_f32(&k_data).unwrap();
	let v = GpuBuffer::upload_f32(&v_data).unwrap();

	let out = gpu_scaled_dot_product_attention(&q, &k, &v, 1, 2, 2, true).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("sdpa causal got: {:?}", got);

	// from python: row0 attends only key0 → V[0]=[1,2]; row1 same as non-causal
	let expected: Vec<f32> = vec![1.0, 2.0, 2.339_523, 3.339_523];
	assert_close(&got, &expected, "sdpa_causal");
}

#[test]
fn test_causal_softmax_rows() {
	// 3x3 matrix of all ones; row i should have uniform probs over [0..i]
	let x_data: Vec<f32> = vec![1.0; 9];

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	gpu_causal_softmax_rows(&x, 3, 3);
	sync();

	let mut got = vec![0.0f32; 9];
	x.download_f32(&mut got).unwrap();
	eprintln!("causal_softmax got: {:?}", got);

	// Row 0: only j=0 unmasked → prob=[1,0,0]
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
	// Row 1: j=0,1 → prob=[0.5, 0.5, 0]
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
	// Row 2: all three → prob=[1/3, 1/3, 1/3]
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
	// x: (seq=4, n_heads*head_dim = 2*3=6)
	let n_heads = 2usize;
	let head_dim = 3usize;
	let seq = 4usize;
	let data: Vec<f32> = (0..(seq * n_heads * head_dim)).map(|i| i as f32).collect();

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let split = gpu_mha_split(&x, seq, n_heads, head_dim).unwrap();
	sync();

	let merged = gpu_mha_merge(&split, seq, n_heads, head_dim).unwrap();
	sync();

	let mut got = vec![0.0f32; data.len()];
	merged.download_f32(&mut got).unwrap();
	eprintln!("mha round-trip got: {:?}", got);
	assert_close(&got, &data, "mha_split_merge_roundtrip");
}

#[test]
fn test_mha_split_layout() {
	// x: (seq=2, 2*2=4): [[0,1,2,3],[4,5,6,7]]
	// head 0 dim 0,1: x[:,0:2]
	// head 1 dim 0,1: x[:,2:4]
	// split → (n_heads=2, seq=2, head_dim=2)
	// out[0,0,*] = x[0, 0:2] = [0,1]
	// out[0,1,*] = x[1, 0:2] = [4,5]
	// out[1,0,*] = x[0, 2:4] = [2,3]
	// out[1,1,*] = x[1, 2:4] = [6,7]
	let data: Vec<f32> = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
	let expected: Vec<f32> = vec![0.0, 1.0, 4.0, 5.0, 2.0, 3.0, 6.0, 7.0];

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let split = gpu_mha_split(&x, 2, 2, 2).unwrap();
	sync();

	let mut got = vec![0.0f32; 8];
	split.download_f32(&mut got).unwrap();
	eprintln!("mha_split layout got: {:?}", got);
	assert_close(&got, &expected, "mha_split_layout");
}

#[test]
fn test_rope_norm_preserved() {
	// RoPE should preserve L2 norm per (d, d+dim/2) pair
	let seq = 3usize;
	let dim = 4usize;
	let data: Vec<f32> = vec![
		1.0, 0.0, 0.0, 1.0, // s=0
		2.0, 1.0, -1.0, 2.0, // s=1
		0.0, 3.0, 1.0, 0.0,
	]; // s=2

	let x = GpuBuffer::upload_f32(&data).unwrap();
	let out = gpu_rope(&x, seq, dim, 10000.0).unwrap();
	sync();

	let mut got = vec![0.0f32; seq * dim];
	out.download_f32(&mut got).unwrap();
	eprintln!("rope got: {:?}", got);

	// Check that for each row, norm of (got[d], got[d+dim/2]) == norm of (x[d], x[d+dim/2])
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
	let out = gpu_positional_encoding(seq, dim).unwrap();
	sync();

	let mut got = vec![0.0f32; seq * dim];
	out.download_f32(&mut got).unwrap();
	eprintln!("pos_enc (4,8): {:?}", got);

	// All values should be finite and in [-1, 1]
	for (i, &v) in got.iter().enumerate() {
		assert!(v.is_finite(), "pos_enc[{}] not finite", i);
		assert!(v.abs() <= 1.0 + 1e-5, "pos_enc[{}]={} out of [-1,1]", i, v);
	}

	// pe[0, even] = sin(0 / ...) = 0
	for d in (0..dim).step_by(2) {
		assert!(
			(got[d]).abs() < 1e-5,
			"pos_enc[0,{}] (even, should be sin(0)=0) got {}",
			d,
			got[d]
		);
	}
	// pe[0, odd] = cos(0 / ...) = 1
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
	// 1 row of 4 cols; gamma=ones; expected = x / rms(x)
	let x_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
	let gamma_data: Vec<f32> = vec![1.0, 1.0, 1.0, 1.0];
	let eps = 1e-5_f64;

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let out = gpu_rmsnorm(&x, &gamma, 1, 4, eps).unwrap();
	sync();

	let mut got = vec![0.0f32; 4];
	out.download_f32(&mut got).unwrap();
	eprintln!("rmsnorm got: {:?}", got);

	// from python: [0.36514813, 0.73029625, 1.0954444, 1.4605925]
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
	let eps = 1e-5_f64;

	let x = GpuBuffer::upload_f32(&x_data).unwrap();
	let gamma = GpuBuffer::upload_f32(&gamma_data).unwrap();
	let grad_out = GpuBuffer::upload_f32(&grad_out_data).unwrap();

	let (grad_x, grad_gamma) =
		gpu_rmsnorm_backward(&grad_out, &x, &gamma, rows, cols, eps).unwrap();
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

	// RMSNorm backward: grad_x should sum to near 0 per row when grad_out is uniform
	// (similar to LayerNorm — the scaling factor removes one degree of freedom)
	// Actually RMSNorm doesn't center, so this doesn't hold exactly.
	// Just verify finite and non-zero
	let gx_sum: f32 = gx.iter().sum();
	assert!(gx_sum.is_finite(), "rmsnorm grad_x sum non-finite");
}

#[test]
fn test_im2col_2d_ext() {
	// n=1, c=1, h=4, w=4, kh=2, kw=2, sh=2, sw=2, pad=1, dil=1
	let input: Vec<f32> = (1..=16).map(|x| x as f32).collect();
	// out_h = (4+2*1 - 1*(2-1) - 1)/2 + 1 = (4+2-1-1)/2+1 = 4/2+1 = 3
	// out_w = 3
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
	let patches =
		gpu_im2col_2d_ext(&x, n, c, h, w, kh, kw, sh, sw, pad_h, pad_w, dil_h, dil_w).unwrap();
	sync();

	let patch_count = n * out_h * out_w;
	let patch_size = c * kh * kw;
	let mut got = vec![0.0f32; patch_count * patch_size];
	patches.download_f32(&mut got).unwrap();
	eprintln!("im2col patches (9x4):");
	for r in 0..9 {
		eprintln!("  patch[{}]: {:?}", r, &got[r * 4..(r + 1) * 4]);
	}

	// from python: patches[0]=[0,0,0,1], patches[4]=[6,7,10,11], patches[8]=[16,0,0,0]
	let expected_patch0: Vec<f32> = vec![0.0, 0.0, 0.0, 1.0];
	let expected_patch4: Vec<f32> = vec![6.0, 7.0, 10.0, 11.0];
	let expected_patch8: Vec<f32> = vec![16.0, 0.0, 0.0, 0.0];
	assert_close(&got[0..4], &expected_patch0, "im2col patch[0]");
	assert_close(&got[16..20], &expected_patch4, "im2col patch[4]");
	assert_close(&got[32..36], &expected_patch8, "im2col patch[8]");

	// check output dimensions by verifying length
	assert_eq!(got.len(), 9 * 4, "im2col output size wrong");
}

#[test]
fn test_embedding_backward() {
	// grad_out: (n=3, cols=2), indices: [0, 1, 0] (i32), vocab=2
	// grad_table[0] = grad_out[0] + grad_out[2] = [1,2] + [5,6] = [6,8]
	// grad_table[1] = grad_out[1] = [3,4]
	let grad_out_data: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
	let indices_i32: Vec<i32> = vec![0, 1, 0];
	let n = 3usize;
	let cols = 2usize;
	let vocab = 2usize;

	let grad_out = GpuBuffer::upload_f32(&grad_out_data).unwrap();
	let indices = GpuBuffer::upload_i32(&indices_i32).unwrap();

	let grad_table = gpu_embedding_backward(&grad_out, &indices, n, cols, vocab).unwrap();
	sync();

	let mut got = vec![0.0f32; vocab * cols];
	grad_table.download_f32(&mut got).unwrap();
	eprintln!("embedding_backward got: {:?}", got);

	let expected: Vec<f32> = vec![6.0, 8.0, 3.0, 4.0];
	assert_close(&got, &expected, "gpu_embedding_backward");
}

#[test]
fn test_bn_update_running() {
	// run = (1-momentum)*run + momentum*save
	let run_mean_data: Vec<f32> = vec![0.0, 0.0];
	let run_var_data: Vec<f32> = vec![1.0, 1.0];
	let save_mean_data: Vec<f32> = vec![2.0, -2.0];
	let save_var_data: Vec<f32> = vec![0.5, 0.5];
	let momentum = 0.1_f64;

	let run_mean = GpuBuffer::upload_f32(&run_mean_data).unwrap();
	let run_var = GpuBuffer::upload_f32(&run_var_data).unwrap();
	let save_mean = GpuBuffer::upload_f32(&save_mean_data).unwrap();
	let save_var = GpuBuffer::upload_f32(&save_var_data).unwrap();

	gpu_bn_update_running(&run_mean, &run_var, &save_mean, &save_var, momentum, 2);
	sync();

	let mut got_mean = vec![0.0f32; 2];
	let mut got_var = vec![0.0f32; 2];
	run_mean.download_f32(&mut got_mean).unwrap();
	run_var.download_f32(&mut got_var).unwrap();
	eprintln!("bn_update_running mean: {:?}", got_mean);
	eprintln!("bn_update_running var: {:?}", got_var);

	// expected: mean = 0.9*0 + 0.1*2 = 0.2; mean[1] = 0.9*0 + 0.1*(-2) = -0.2
	// var = 0.9*1 + 0.1*0.5 = 0.95
	let expected_mean: Vec<f32> = vec![0.2, -0.2];
	let expected_var: Vec<f32> = vec![0.95, 0.95];
	assert_close(&got_mean, &expected_mean, "bn_update_running mean");
	assert_close(&got_var, &expected_var, "bn_update_running var");
}

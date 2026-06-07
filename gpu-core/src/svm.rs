use crate::hip::HipError;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
	// launch_kernel_matrix(x, k_out, n, dim, kind, gamma, coef0, degree, stream)
	fn launch_kernel_matrix(
		x: *const c_void,
		k_out: *mut c_void,
		n: i32,
		dim: i32,
		kind: i32,
		gamma: f64,
		coef0: f64,
		degree: f64,
		stream: *mut c_void,
	);

	// launch_smo_kkt_score(grad, alpha, y, score_i, score_j, n, C, stream)
	fn launch_smo_kkt_score(
		grad: *const c_void,
		alpha: *const c_void,
		y: *const c_void,
		score_i: *mut c_void,
		score_j: *mut c_void,
		n: i32,
		c: f64,
		stream: *mut c_void,
	);

	// launch_smo_kernel_row(x, krow, n, dim, row, kind, gamma, coef0, degree, stream)
	fn launch_smo_kernel_row(
		x: *const c_void,
		krow: *mut c_void,
		n: i32,
		dim: i32,
		row: i32,
		kind: i32,
		gamma: f64,
		coef0: f64,
		degree: f64,
		stream: *mut c_void,
	);

	// launch_smo_argmax(s, out, n, stream) — out[0]=max value, out[1]=index (as f64)
	fn launch_smo_argmax(s: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

	// launch_smo_update_gradient_rows(grad, ki, kj, n, di, dj, stream)
	fn launch_smo_update_gradient_rows(
		grad: *mut c_void,
		ki: *const c_void,
		kj: *const c_void,
		n: i32,
		di: f64,
		dj: f64,
		stream: *mut c_void,
	);
}

// Compute the n×n kernel matrix for the n training samples in x (n×dim, row-major).
// kind: 0=linear, 1=rbf, 2=poly, 3=sigmoid.
// gamma, coef0, degree are kernel hyperparameters (full real-line valid; unused params ignored).
// Returns K[n*n] on GPU. NOTE: O(n²) memory — for SVM training prefer the matrix-free
// `gpu_smo_train`, which never materializes this. Kept for callers that genuinely need
// the dense Gram matrix.
pub fn gpu_kernel_matrix(
	x: &GpuBuffer,
	n: usize,
	dim: usize,
	kind: i32,
	gamma: f64,
	coef0: f64,
	degree: f64,
) -> Result<GpuBuffer, HipError> {
	let k_out = GpuBuffer::alloc(n * n)?;
	unsafe {
		launch_kernel_matrix(
			x.ptr_raw() as *const c_void,
			k_out.ptr_raw(),
			n as i32,
			dim as i32,
			kind,
			gamma,
			coef0,
			degree,
			std::ptr::null_mut(),
		);
	}
	crate::kernels::check_launch();
	Ok(k_out)
}

// Read a single f64 element out of a GPU buffer at element index `idx` (one tiny D2H
// copy). Used for the handful of scalars SMO needs per iteration (K[i,i], K[i,j],
// K[j,j], G[i], G[j]) instead of downloading whole vectors.
fn read_at(buf: &GpuBuffer, idx: usize) -> Result<f64, HipError> {
	let mut v = [0.0f64];
	unsafe {
		let src = (buf.ptr_raw() as *const u8).add(idx * std::mem::size_of::<f64>())
			as *const c_void;
		crate::hip::check(crate::hip::hipMemcpy(
			v.as_mut_ptr() as *mut c_void,
			src,
			std::mem::size_of::<f64>(),
			crate::hip::HIP_MEMCPY_D2H,
		))?;
	}
	Ok(v[0])
}

// Matrix-free working-set SMO training for binary SVM.
//
// Nothing O(n²) is ever allocated or downloaded: the kernel matrix is never
// materialized — each iteration recomputes only the two needed rows K[i,:], K[j,:]
// on the GPU (item 5). Working-set selection (argmax of the two KKT-violation score
// vectors) runs on the GPU and returns just (value,index) pairs, and the per-iteration
// scalars (K[i,i], K[i,j], K[j,j], G[i], G[j]) are single-element reads — no whole
// vectors cross the bus (item 3).
//
// Each iteration:
//   1. GPU: smo_kkt_score → per-sample I_up / I_down violation scores.
//   2. GPU: argmax of each score vector → (val,idx); download 2 doubles each.
//      Stop when score_i[i] - score_j[j] < tol.
//   3. GPU: recompute rows K[i,:] and K[j,:]; read K[i,i], K[i,j], K[j,j] (3 scalars).
//   4. Host: standard SMO closed-form alpha pair update, clipped to [0,C].
//   5. GPU: gradient update from the two rows. Read G[i], G[j] (2 scalars) for bias.
//
// x: n×dim row-major samples. y must be in {-1.0,+1.0}. C, tol, max_iter: full valid
// ranges (C>0, tol>0, max_iter>0). Returns (alpha[n], b).
pub fn gpu_smo_train(
	x: &GpuBuffer, // n×dim row-major training samples
	y_pm1: &[f64], // labels in {-1,+1}, length n
	n: usize,
	dim: usize,
	kind: i32,
	gamma: f64,
	coef0: f64,
	degree: f64,
	c: f64,
	tol: f64,
	max_iter: i32,
) -> Result<(Vec<f64>, f64), HipError> {
	let y_buf = GpuBuffer::upload(y_pm1)?;
	let alpha_buf = GpuBuffer::alloc(n)?;
	alpha_buf.memset_zero(n * std::mem::size_of::<f64>())?;

	// Gradient G[t] = -1 initially (all alphas = 0).
	let grad_buf = GpuBuffer::upload(&vec![-1.0_f64; n])?;

	let score_i_buf = GpuBuffer::alloc(n)?;
	let score_j_buf = GpuBuffer::alloc(n)?;
	let krow_i = GpuBuffer::alloc(n)?;
	let krow_j = GpuBuffer::alloc(n)?;
	let argmax_out = GpuBuffer::alloc(2)?;

	let mut alpha_host = vec![0.0_f64; n];
	let mut b = 0.0_f64;
	let mut b_count = 0_usize;

	let argmax = |scores: &GpuBuffer| -> Result<(f64, usize), HipError> {
		unsafe {
			launch_smo_argmax(
				scores.ptr_raw() as *const c_void,
				argmax_out.ptr_raw(),
				n as i32,
				std::ptr::null_mut(),
			);
		}
		crate::kernels::check_launch();
		let mut o = [0.0_f64; 2];
		argmax_out.download(&mut o)?;
		Ok((o[0], o[1] as usize))
	};
	let kernel_row = |row: usize, out: &GpuBuffer| {
		unsafe {
			launch_smo_kernel_row(
				x.ptr_raw() as *const c_void,
				out.ptr_raw(),
				n as i32,
				dim as i32,
				row as i32,
				kind,
				gamma,
				coef0,
				degree,
				std::ptr::null_mut(),
			);
		}
		crate::kernels::check_launch();
	};

	for _iter in 0..max_iter {
		unsafe {
			launch_smo_kkt_score(
				grad_buf.ptr_raw() as *const c_void,
				alpha_buf.ptr_raw() as *const c_void,
				y_buf.ptr_raw() as *const c_void,
				score_i_buf.ptr_raw(),
				score_j_buf.ptr_raw(),
				n as i32,
				c,
				std::ptr::null_mut(),
			);
		}
		crate::kernels::check_launch();

		// Working-set selection on the GPU: i = argmax(score_i), j = argmax(score_j).
		let (val_i, i) = argmax(&score_i_buf)?;
		let (val_j, j) = argmax(&score_j_buf)?;
		if val_i - val_j < tol {
			break;
		}

		// Recompute only rows i and j of the kernel matrix (matrix-free).
		kernel_row(i, &krow_i);
		kernel_row(j, &krow_j);
		let kii = read_at(&krow_i, i)?;
		let kij = read_at(&krow_i, j)?;
		let kjj = read_at(&krow_j, j)?;

		let yi = y_pm1[i];
		let yj = y_pm1[j];
		let eta = kii + kjj - 2.0 * kij;

		let old_ai = alpha_host[i];
		let old_aj = alpha_host[j];

		// Unconstrained step in the j direction (val_i - val_j is the optimality gap).
		let grad_diff = -(val_i - val_j);
		let new_aj_raw = if eta.abs() > 1e-12 {
			old_aj + yj * grad_diff / eta
		} else {
			old_aj
		};

		// Box constraints [L,H] for alpha_j.
		let (lo, hi) = if (yi - yj).abs() < 1e-9 {
			let s = old_ai + old_aj;
			(f64::max(0.0, s - c), f64::min(c, s))
		} else {
			let s = old_ai - old_aj;
			(f64::max(0.0, -s), f64::min(c, c - s))
		};

		let new_aj = new_aj_raw.clamp(lo, hi);
		let new_ai = (old_ai + yi * yj * (old_aj - new_aj)).clamp(0.0, c);

		let delta_ai = new_ai - old_ai;
		let delta_aj = new_aj - old_aj;
		if delta_ai.abs() < 1e-12 && delta_aj.abs() < 1e-12 {
			break;
		}

		// GPU gradient update from the two kernel rows:
		//   G[t] += yi*delta_ai*K[i,t] + yj*delta_aj*K[j,t]
		unsafe {
			launch_smo_update_gradient_rows(
				grad_buf.ptr_raw(),
				krow_i.ptr_raw() as *const c_void,
				krow_j.ptr_raw() as *const c_void,
				n as i32,
				yi * delta_ai,
				yj * delta_aj,
				std::ptr::null_mut(),
			);
		}
		crate::kernels::check_launch();

		alpha_host[i] = new_ai;
		alpha_host[j] = new_aj;

		// Bias from free support vectors (0 < alpha < C): b = -G[t]/y[t]. Read just
		// the two updated gradient entries (no full-vector download).
		if new_ai > 0.0 && new_ai < c {
			b += -read_at(&grad_buf, i)? / yi;
			b_count += 1;
		}
		if new_aj > 0.0 && new_aj < c {
			b += -read_at(&grad_buf, j)? / yj;
			b_count += 1;
		}
	}

	let b_final = if b_count > 0 { b / b_count as f64 } else { 0.0 };
	Ok((alpha_host, b_final))
}

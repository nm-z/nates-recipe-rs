use crate::hip::HipError;
use crate::kernels::check_launch;
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
		gamma: *const c_void,
		coef0: *const c_void,
		degree: *const c_void,
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
		c: *const c_void,
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
		gamma: *const c_void,
		coef0: *const c_void,
		degree: *const c_void,
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
		di: *const c_void,
		dj: *const c_void,
		stream: *mut c_void,
	);
}

// Compute the n×n kernel matrix for the n training samples in x (n×dim, row-major).
// kind: 0=linear, 1=rbf, 2=poly, 3=sigmoid.
// gamma, coef0, degree are 1-elem device buffers (kernel hyperparameters; unused
// params ignored). Writes K[n*n] into `k_out`. NOTE: O(n²) memory — for SVM training
// prefer the matrix-free `gpu_smo_train`, which never materializes this. Kept for
// callers that genuinely need the dense Gram matrix.
pub fn gpu_kernel_matrix(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	coef0: &GpuBuffer,
	degree: &GpuBuffer,
	n: usize,
	dim: usize,
	kind: usize,
	k_out: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_kernel_matrix(
			x.ptr_raw() as *const c_void,
			k_out.ptr_raw(),
			n as i32,
			dim as i32,
			kind as i32,
			gamma.ptr_raw() as *const c_void,
			coef0.ptr_raw() as *const c_void,
			degree.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// ── Schedulable SMO primitives ──────────────────────────────────────────────
// The matrix-free SMO iteration (gpu_smo_train, below) is a host-side driver that
// composes these four conforming ops.

// KKT-violation scoring for working-set selection. c is a 1-elem device buffer (box
// bound C). Writes per-sample I_up / I_down violation scores into score_i / score_j.
pub fn gpu_smo_kkt_score(
	grad: &GpuBuffer,
	alpha: &GpuBuffer,
	y: &GpuBuffer,
	c: &GpuBuffer,
	n: usize,
	score_i: &GpuBuffer,
	score_j: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_smo_kkt_score(
			grad.ptr_raw() as *const c_void,
			alpha.ptr_raw() as *const c_void,
			y.ptr_raw() as *const c_void,
			score_i.ptr_raw(),
			score_j.ptr_raw(),
			n as i32,
			c.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Recompute one kernel-matrix row on demand (matrix-free): krow[t] = K(x_row, x_t).
// gamma/coef0/degree are 1-elem device buffers; kind and row are plan-time dims.
pub fn gpu_smo_kernel_row(
	x: &GpuBuffer,
	gamma: &GpuBuffer,
	coef0: &GpuBuffer,
	degree: &GpuBuffer,
	n: usize,
	dim: usize,
	kind: usize,
	row: usize,
	krow: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_smo_kernel_row(
			x.ptr_raw() as *const c_void,
			krow.ptr_raw(),
			n as i32,
			dim as i32,
			row as i32,
			kind as i32,
			gamma.ptr_raw() as *const c_void,
			coef0.ptr_raw() as *const c_void,
			degree.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Single-block argmax over s[n]. Writes out[0]=max value, out[1]=index (as f64).
pub fn gpu_smo_argmax(s: &GpuBuffer, n: usize, out: &GpuBuffer) -> Result<(), HipError> {
	unsafe {
		launch_smo_argmax(
			s.ptr_raw() as *const c_void,
			out.ptr_raw(),
			n as i32,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// Gradient update from two precomputed kernel rows: grad[t] += di*ki[t] + dj*kj[t].
// di/dj are 1-elem device buffers; grad is the in/out accumulator.
pub fn gpu_smo_update_gradient_rows(
	ki: &GpuBuffer,
	kj: &GpuBuffer,
	di: &GpuBuffer,
	dj: &GpuBuffer,
	n: usize,
	grad: &GpuBuffer,
) -> Result<(), HipError> {
	unsafe {
		launch_smo_update_gradient_rows(
			grad.ptr_raw(),
			ki.ptr_raw() as *const c_void,
			kj.ptr_raw() as *const c_void,
			n as i32,
			di.ptr_raw() as *const c_void,
			dj.ptr_raw() as *const c_void,
			std::ptr::null_mut(),
		);
	}
	check_launch();
	Ok(())
}

// not-an-op: plumbing — D2H readback of a single f64 element from a GPU buffer.
// Used for the handful of scalars SMO needs per iteration (K[i,i], K[i,j], K[j,j],
// G[i], G[j]) instead of downloading whole vectors.
fn read_at(buf: &GpuBuffer, idx: usize) -> Result<f64, HipError> {
	let mut v = [0.0f64];
	unsafe {
		let src = (buf.ptr_raw() as *const u8).add(idx * std::mem::size_of::<f64>())
			as *const c_void;
		crate::memory::xfer_sync(
			v.as_mut_ptr() as *mut c_void,
			src,
			std::mem::size_of::<f64>(),
			crate::hip::HIP_MEMCPY_D2H,
		)?;
	}
	Ok(v[0])
}

// not-an-op: plumbing — H2D write of a single f64 into a 1-elem device buffer.
fn write1(buf: &GpuBuffer, val: f64) -> Result<(), HipError> {
	let v = [val];
	unsafe {
		crate::memory::xfer_sync(
			buf.ptr_raw(),
			v.as_ptr() as *const c_void,
			std::mem::size_of::<f64>(),
			crate::hip::HIP_MEMCPY_H2D,
		)?;
	}
	Ok(())
}

// not-an-op: driver — matrix-free working-set SMO training for binary SVM. The host
// loop, closed-form alpha pair update, box clip, convergence break, and per-iter D2H
// scalar reads (read_at) that steer control flow all live here; the driver composes
// the conforming ops gpu_smo_kkt_score / gpu_smo_argmax / gpu_smo_kernel_row /
// gpu_smo_update_gradient_rows plus memory plumbing.
//
// Nothing O(n²) is ever allocated or downloaded: each iteration recomputes only the
// two needed rows K[i,:], K[j,:] on the GPU; working-set selection runs on the GPU
// (returns just (value,index) pairs); the per-iteration scalars (K[i,i], K[i,j],
// K[j,j], G[i], G[j]) are single-element reads.
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

	// 1-elem device scalars the conforming ops consume.
	let gamma_buf = GpuBuffer::upload(&[gamma])?;
	let coef0_buf = GpuBuffer::upload(&[coef0])?;
	let degree_buf = GpuBuffer::upload(&[degree])?;
	let c_buf = GpuBuffer::upload(&[c])?;
	let di_buf = GpuBuffer::alloc(1)?;
	let dj_buf = GpuBuffer::alloc(1)?;
	let kind_u = kind as usize;

	let mut alpha_host = vec![0.0_f64; n];
	let mut b = 0.0_f64;
	let mut b_count = 0_usize;

	for _iter in 0..max_iter {
		gpu_smo_kkt_score(&grad_buf, &alpha_buf, &y_buf, &c_buf, n, &score_i_buf, &score_j_buf)?;

		// Working-set selection on the GPU: i = argmax(score_i), j = argmax(score_j).
		let mut o = [0.0_f64; 2];
		gpu_smo_argmax(&score_i_buf, n, &argmax_out)?;
		argmax_out.download(&mut o)?;
		let (val_i, i) = (o[0], o[1] as usize);
		gpu_smo_argmax(&score_j_buf, n, &argmax_out)?;
		argmax_out.download(&mut o)?;
		let (val_j, j) = (o[0], o[1] as usize);
		if val_i - val_j < tol {
			break;
		}

		// Recompute only rows i and j of the kernel matrix (matrix-free).
		gpu_smo_kernel_row(x, &gamma_buf, &coef0_buf, &degree_buf, n, dim, kind_u, i, &krow_i)?;
		gpu_smo_kernel_row(x, &gamma_buf, &coef0_buf, &degree_buf, n, dim, kind_u, j, &krow_j)?;
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
		write1(&di_buf, yi * delta_ai)?;
		write1(&dj_buf, yj * delta_aj)?;
		gpu_smo_update_gradient_rows(&krow_i, &krow_j, &di_buf, &dj_buf, n, &grad_buf)?;

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

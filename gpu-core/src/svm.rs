use crate::hip::HipError;
use crate::kernels::check_launch;
use crate::memory::GpuBuffer;
use std::ffi::c_void;

unsafe extern "C" {
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

	fn launch_smo_argmax(s: *const c_void, out: *mut c_void, n: i32, stream: *mut c_void);

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

fn read_at(buf: &GpuBuffer, idx: usize) -> Result<f64, HipError> {
	let mut v = [0.0f64];
	unsafe {
		let src = (buf.ptr_raw() as *const u8).add(idx * std::mem::size_of::<f64>())
			as *const c_void;
		crate::memory::xfer(
			v.as_mut_ptr() as *mut c_void,
			src,
			std::mem::size_of::<f64>(),
			crate::hip::HIP_MEMCPY_D2H,
			std::ptr::null_mut(),
		)?;
	}
	crate::hip::device_synchronize()?;
	Ok(v[0])
}

fn write1(buf: &GpuBuffer, val: f64) -> Result<(), HipError> {
	let v = [val];
	unsafe {
		crate::memory::xfer(
			buf.ptr_raw(),
			v.as_ptr() as *const c_void,
			std::mem::size_of::<f64>(),
			crate::hip::HIP_MEMCPY_H2D,
			std::ptr::null_mut(),
		)?;
	}
	Ok(())
}

pub fn gpu_smo_train(
	x: &GpuBuffer,
	y_pm1: &[f64],
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
	let y_buf = GpuBuffer::alloc(n)?;
	y_buf.load(y_pm1)?;
	let alpha_buf = GpuBuffer::alloc(n)?;
	alpha_buf.memset_zero(n * std::mem::size_of::<f64>())?;

	let grad_buf = GpuBuffer::alloc(n)?;
	grad_buf.load(&vec![-1.0_f64; n])?;

	let score_i_buf = GpuBuffer::alloc(n)?;
	let score_j_buf = GpuBuffer::alloc(n)?;
	let krow_i = GpuBuffer::alloc(n)?;
	let krow_j = GpuBuffer::alloc(n)?;
	let argmax_out = GpuBuffer::alloc(2)?;

	let gamma_buf = GpuBuffer::alloc(1)?;
	gamma_buf.load(&[gamma])?;
	let coef0_buf = GpuBuffer::alloc(1)?;
	coef0_buf.load(&[coef0])?;
	let degree_buf = GpuBuffer::alloc(1)?;
	degree_buf.load(&[degree])?;
	let c_buf = GpuBuffer::alloc(1)?;
	c_buf.load(&[c])?;
	let di_buf = GpuBuffer::alloc(1)?;
	let dj_buf = GpuBuffer::alloc(1)?;
	let kind_u = kind as usize;

	let mut alpha_host = vec![0.0_f64; n];
	let mut b = 0.0_f64;
	let mut b_count = 0_usize;

	for _iter in 0..max_iter {
		gpu_smo_kkt_score(&grad_buf, &alpha_buf, &y_buf, &c_buf, n, &score_i_buf, &score_j_buf)?;

		let mut o = [0.0_f64; 2];
		gpu_smo_argmax(&score_i_buf, n, &argmax_out)?;
		unsafe { argmax_out.download_async(&mut o, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()?;
		let (val_i, i) = (o[0], o[1] as usize);
		gpu_smo_argmax(&score_j_buf, n, &argmax_out)?;
		unsafe { argmax_out.download_async(&mut o, std::ptr::null_mut()) }?;
		crate::hip::device_synchronize()?;
		let (val_j, j) = (o[0], o[1] as usize);
		if val_i - val_j < tol {
			break;
		}

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

		let grad_diff = -(val_i - val_j);
		let new_aj_raw = if eta.abs() > 1e-12 {
			old_aj + yj * grad_diff / eta
		} else {
			old_aj
		};

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

		write1(&di_buf, yi * delta_ai)?;
		write1(&dj_buf, yj * delta_aj)?;
		gpu_smo_update_gradient_rows(&krow_i, &krow_j, &di_buf, &dj_buf, n, &grad_buf)?;

		alpha_host[i] = new_ai;
		alpha_host[j] = new_aj;

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

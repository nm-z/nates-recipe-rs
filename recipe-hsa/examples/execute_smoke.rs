use recipe_hsa::{DeviceType, DispatchGeometry, QueueConfig, QueueKind, Runtime, WaitOutcome};
use std::io;
use std::path::Path;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let runtime = Runtime::open_default()?;
	let discovery = runtime.discover()?;
	let mut cpu = None;
	let mut gpu = None;
	for agent in discovery.into_agents() {
		match agent.description().device_type {
			DeviceType::Cpu if cpu.is_none() => cpu = Some(agent),
			DeviceType::Gpu if gpu.is_none() => gpu = Some(agent),
			_ => {}
		}
	}
	let cpu = cpu.ok_or_else(|| io::Error::other("no ROCr CPU agent"))?;
	let gpu = gpu.ok_or_else(|| io::Error::other("no ROCr GPU agent"))?;
	let session = gpu.into_session()?;

	let bytes: Vec<u8> = (0..=255).collect();
	let source = cpu.allocate_fine(bytes.len())?;
	let device = session.allocate_coarse(bytes.len())?;
	let destination = cpu.allocate_fine(bytes.len())?;

	session.grant_access(&source)?;
	cpu.grant_access(&device)?;
	session.grant_access(&destination)?;

	// SAFETY: the allocations came from the CPU agent's fine-grained pool and
	// no device operation is using either range yet.
	unsafe {
		source.copy_from_host(0, &bytes)?;
	}

	let mut upload = session.copy_async(&device, 0, &source, 0, bytes.len())?;
	if upload.wait(Duration::from_secs(5))? != WaitOutcome::Complete {
		return Err(io::Error::new(io::ErrorKind::TimedOut, "upload timed out").into());
	}
	drop(upload);

	let mut download = session.copy_async(&destination, 0, &device, 0, bytes.len())?;
	if download.wait(Duration::from_secs(5))? != WaitOutcome::Complete {
		return Err(io::Error::new(io::ErrorKind::TimedOut, "download timed out").into());
	}
	drop(download);

	let mut observed = vec![0_u8; bytes.len()];
	// SAFETY: this is a CPU fine-grained allocation and the system-scope copy
	// completed before the host read.
	unsafe {
		destination.copy_to_host(0, &mut observed)?;
	}
	if observed != bytes {
		return Err(io::Error::other("round-trip copy mismatch").into());
	}

	if let Some(path) = std::env::var_os("RECIPE_HSA_SMOKE_COPY_HSACO") {
		let symbol = std::env::var("RECIPE_HSA_SMOKE_SYMBOL")
			.map_err(|_| io::Error::other("RECIPE_HSA_SMOKE_SYMBOL is required"))?;
		let hsaco = std::fs::read(Path::new(&path))?;
		let executable = session.load_hsaco(&hsaco)?;
		let kernel = executable.kernel(&symbol)?;
		let metadata = kernel.metadata().clone();
		if metadata.kernarg_segment_size < 16 {
			return Err(io::Error::other("copy smoke kernel must accept destination and source pointers").into());
		}
		let output = session.allocate_coarse(bytes.len())?;
		cpu.grant_access(&output)?;
		let kernarg = cpu.allocate_kernarg(metadata.kernarg_segment_size as usize)?;
		session.grant_access(&kernarg)?;
		let mut arguments = vec![0_u8; metadata.kernarg_segment_size as usize];
		let output_address = output.as_ptr() as usize as u64;
		let input_address = device.as_ptr() as usize as u64;
		arguments[..8].copy_from_slice(&output_address.to_ne_bytes());
		arguments[8..16].copy_from_slice(&input_address.to_ne_bytes());
		// SAFETY: this is a CPU kernarg pool allocation with no concurrent
		// reader before dispatch publication.
		unsafe {
			kernarg.copy_from_host(0, &arguments)?;
		}
		let minimum_queue = session
			.description()
			.queue
			.ok_or_else(|| io::Error::other("GPU has no queue capability"))?
			.minimum_packets;
		let queue = session.create_queue(QueueConfig::new(minimum_queue, QueueKind::SingleProducer))?;
		let mut prerequisite = session.copy_async(&device, 0, &source, 0, bytes.len())?;
		let dependency = prerequisite.dependency();
		let mut dispatch = queue.dispatch_after(
			&kernel,
			Some(&kernarg),
			DispatchGeometry::one_dimensional((bytes.len() / 4) as u32, 64),
			std::slice::from_ref(&dependency),
		)?;
		drop(dependency);
		if dispatch.wait(Duration::from_secs(5))? != WaitOutcome::Complete {
			return Err(io::Error::new(io::ErrorKind::TimedOut, "dispatch timed out").into());
		}
		drop(dispatch);
		if prerequisite.wait(Duration::from_secs(5))? != WaitOutcome::Complete {
			return Err(io::Error::new(io::ErrorKind::TimedOut, "dispatch prerequisite timed out").into());
		}
		drop(prerequisite);
		queue.close()?;

		let mut verify = session.copy_async(&destination, 0, &output, 0, bytes.len())?;
		if verify.wait(Duration::from_secs(5))? != WaitOutcome::Complete {
			return Err(io::Error::new(io::ErrorKind::TimedOut, "verification copy timed out").into());
		}
		drop(verify);
		// SAFETY: the CPU fine-grained destination was updated by a completed
		// system-scope copy.
		unsafe {
			destination.copy_to_host(0, &mut observed)?;
		}
		if observed != bytes {
			return Err(io::Error::other("kernel copy mismatch").into());
		}

		drop(kernel);
		executable.close()?;
		kernarg.close()?;
		output.close()?;
		println!("optional HSACO kernel dispatch smoke passed");
	}

	destination.close()?;
	device.close()?;
	source.close()?;
	drop(session);
	drop(cpu);
	runtime.close()?;
	println!("ROCr fine→coarse→fine asynchronous copy smoke passed");
	Ok(())
}

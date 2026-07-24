use core::fmt;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use recipe_core::{ByteCount, DeviceId};
use rustix::fs::{FallocateFlags, fallocate};

use crate::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaKind {
	Ram,
	Disk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskFileSpec {
	path: PathBuf,
}

impl DiskFileSpec {
	pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
		let path = path.into();
		if path.file_name().is_none() {
			return Err(Error::InvalidPath);
		}
		Ok(Self { path })
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}
}

pub(crate) enum Backing {
	Ram {
		storage: Mutex<Box<[u8]>>,
		bytes: u64,
	},
	Disk {
		file: File,
		path: PathBuf,
		bytes: u64,
	},
}

impl fmt::Debug for Backing {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Ram { bytes, .. } => formatter
				.debug_struct("RamBacking")
				.field("bytes", bytes)
				.finish(),
			Self::Disk { path, bytes, .. } => formatter
				.debug_struct("DiskBacking")
				.field("path", path)
				.field("bytes", bytes)
				.finish_non_exhaustive(),
		}
	}
}

impl Backing {
	pub(crate) fn len(&self) -> u64 {
		match self {
			Self::Ram { bytes, .. } => *bytes,
			Self::Disk { bytes, .. } => *bytes,
		}
	}
}

impl Drop for Backing {
	fn drop(&mut self) {
		if let Self::Disk { file, path, .. } = self {
			let _ = file.sync_data();
			let _ = std::fs::remove_file(path);
		}
	}
}

#[derive(Clone)]
pub struct Arena {
	device: DeviceId,
	kind: ArenaKind,
	pub(crate) backing: Arc<Backing>,
}

impl Arena {
	pub fn ram(device: DeviceId, bytes: ByteCount) -> Result<Self> {
		if bytes == ByteCount::ZERO {
			return Err(Error::InvalidConfiguration(
				"RAM allocation must be nonzero",
			));
		}
		let length = usize::try_from(bytes.get())
			.map_err(|_| Error::InvalidConfiguration("RAM arena size does not fit the host address space"))?;
		let mut storage = Vec::new();
		storage
			.try_reserve_exact(length)
			.map_err(|_| Error::InvalidConfiguration("RAM arena allocation failed"))?;
		storage.resize(length, 0);
		Ok(Self {
			device,
			kind: ArenaKind::Ram,
			backing: Arc::new(Backing::Ram {
				storage: Mutex::new(storage.into_boxed_slice()),
				bytes: bytes.get(),
			}),
		})
	}

	pub fn disk(device: DeviceId, spec: DiskFileSpec, bytes: ByteCount) -> Result<Self> {
		let (file, path) = create_allocated_file(spec, bytes, "create disk arena")?;
		Ok(Self {
			device,
			kind: ArenaKind::Disk,
			backing: Arc::new(Backing::Disk {
				file,
				path,
				bytes: bytes.get(),
			}),
		})
	}

	#[must_use]
	pub const fn device(&self) -> DeviceId {
		self.device
	}

	#[must_use]
	pub const fn kind(&self) -> ArenaKind {
		self.kind
	}

	#[must_use]
	pub fn bytes(&self) -> ByteCount {
		ByteCount::new(self.backing.len())
	}

	pub fn close(self) -> Result<()> {
		let backing = Arc::try_unwrap(self.backing).map_err(|_| Error::SlotBusy)?;
		close_backing(backing)
	}

	/// Read one exact range for a pre-realized heterogeneous transfer bridge.
	#[doc(hidden)]
	pub fn bridge_read_exact(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
		self.read_exact(offset, destination)
	}

	/// Write one exact range for a pre-realized heterogeneous transfer bridge.
	#[doc(hidden)]
	pub fn bridge_write_exact(&self, offset: u64, source: &[u8]) -> Result<()> {
		self.write_exact(offset, source)
	}

	pub(crate) fn write_exact(&self, offset: u64, source: &[u8]) -> Result<()> {
		let count = u64::try_from(source.len()).map_err(|_| Error::RangeOverflow)?;
		validate_backing_range(&self.backing, offset, count)?;
		match &*self.backing {
			Backing::Ram { storage, .. } => {
				let mut storage = storage.lock().map_err(|_| Error::Poisoned)?;
				let range = host_range(offset, count)?;
				storage[range].copy_from_slice(source);
				Ok(())
			}
			Backing::Disk { file, .. } => write_all_at(file, source, offset),
		}
	}

	pub(crate) fn read_exact(&self, offset: u64, destination: &mut [u8]) -> Result<()> {
		let count = u64::try_from(destination.len()).map_err(|_| Error::RangeOverflow)?;
		validate_backing_range(&self.backing, offset, count)?;
		match &*self.backing {
			Backing::Ram { storage, .. } => {
				let storage = storage.lock().map_err(|_| Error::Poisoned)?;
				let range = host_range(offset, count)?;
				destination.copy_from_slice(&storage[range]);
				Ok(())
			}
			Backing::Disk { file, .. } => read_exact_at(file, destination, offset),
		}
	}
}

impl fmt::Debug for Arena {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter
			.debug_struct("Arena")
			.field("device", &self.device)
			.field("kind", &self.kind)
			.field("bytes", &self.bytes())
			.finish_non_exhaustive()
	}
}

fn create_allocated_file(spec: DiskFileSpec, bytes: ByteCount, operation: &'static str) -> Result<(File, PathBuf)> {
	if bytes == ByteCount::ZERO {
		return Err(Error::InvalidConfiguration(
			"disk allocation must be nonzero",
		));
	}
	let path = spec.path;
	let file = OpenOptions::new()
		.read(true)
		.write(true)
		.create_new(true)
		.open(&path)
		.map_err(|error| crate::error::io(operation, &error))?;
	if let Err(error) = fallocate(&file, FallocateFlags::empty(), 0, bytes.get()) {
		drop(file);
		let _ = std::fs::remove_file(&path);
		return Err(Error::Io {
			operation: "allocate disk extent",
			kind: error.kind(),
		});
	}
	if let Err(error) = file.sync_data() {
		drop(file);
		let _ = std::fs::remove_file(&path);
		return Err(crate::error::io("sync disk allocation", &error));
	}
	Ok((file, path))
}

fn close_backing(backing: Backing) -> Result<()> {
	match &backing {
		Backing::Ram { .. } => Ok(()),
		Backing::Disk { file, path, .. } => {
			file.sync_data()
				.map_err(|error| crate::error::io("sync disk file", &error))?;
			std::fs::remove_file(path).map_err(|error| crate::error::io("remove disk file", &error))
		}
	}
}

fn validate_backing_range(backing: &Backing, offset: u64, bytes: u64) -> Result<()> {
	offset.checked_add(bytes)
		.filter(|end| *end <= backing.len())
		.ok_or(Error::RangeOverflow)
		.map(|_| ())
}

fn host_range(offset: u64, bytes: u64) -> Result<std::ops::Range<usize>> {
	let start = usize::try_from(offset).map_err(|_| Error::RangeOverflow)?;
	let length = usize::try_from(bytes).map_err(|_| Error::RangeOverflow)?;
	let end = start.checked_add(length).ok_or(Error::RangeOverflow)?;
	Ok(start..end)
}

fn read_exact_at(file: &File, mut destination: &mut [u8], mut offset: u64) -> Result<()> {
	while !destination.is_empty() {
		let count = file
			.read_at(destination, offset)
			.map_err(|error| Error::Io {
				operation: "read host arena",
				kind: error.kind(),
			})?;
		if count == 0 {
			return Err(Error::Io {
				operation: "read host arena",
				kind: std::io::ErrorKind::UnexpectedEof,
			});
		}
		offset = offset
			.checked_add(count as u64)
			.ok_or(Error::RangeOverflow)?;
		destination = &mut destination[count..];
	}
	Ok(())
}

fn write_all_at(file: &File, mut source: &[u8], mut offset: u64) -> Result<()> {
	while !source.is_empty() {
		let count = file.write_at(source, offset).map_err(|error| Error::Io {
			operation: "write host arena",
			kind: error.kind(),
		})?;
		if count == 0 {
			return Err(Error::Io {
				operation: "write host arena",
				kind: std::io::ErrorKind::WriteZero,
			});
		}
		offset = offset
			.checked_add(count as u64)
			.ok_or(Error::RangeOverflow)?;
		source = &source[count..];
	}
	Ok(())
}

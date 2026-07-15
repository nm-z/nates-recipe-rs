use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::mpsc;

pub struct Chan<T> {
	pub tx: mpsc::SyncSender<T>,
	pub rx: mpsc::Receiver<T>,
}

#[must_use]
#[inline]
pub fn sync_chan<T>(depth: usize) -> Chan<T> {
	let (tx, rx) = mpsc::sync_channel::<T>(depth);
	return Chan { tx, rx };
}

/// # Errors
/// Returns an error if the spill file at `path` cannot be opened for read/write.
#[inline]
pub fn open_spill(path: &Path) -> io::Result<File> {
	return OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(path);
}

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::mpsc;

pub struct Chan<T> {
	pub tx: mpsc::SyncSender<T>,
	pub rx: mpsc::Receiver<T>,
}

pub fn sync_chan<T>(depth: usize) -> Chan<T> {
	let (tx, rx) = mpsc::sync_channel::<T>(depth);
	Chan { tx, rx }
}

pub fn open_spill(path: &Path) -> io::Result<File> {
	OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(path)
}

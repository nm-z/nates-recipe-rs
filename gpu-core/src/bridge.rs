use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[inline]
pub fn open_spill(path: &Path) -> io::Result<File> {
	return OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(path);
}

pub struct Chan<T> {
	pub tx: std::sync::mpsc::SyncSender<T>,
	pub rx: std::sync::mpsc::Receiver<T>,
}

pub fn sync_chan<T>(depth: usize) -> Chan<T> {
	let (tx, rx) = std::sync::mpsc::sync_channel::<T>(depth);
	Chan { tx, rx }
}

pub fn open_spill(path: &std::path::Path) -> std::io::Result<std::fs::File> {
	std::fs::OpenOptions::new()
		.read(true)
		.write(true)
		.create(true)
		.truncate(true)
		.open(path)
}

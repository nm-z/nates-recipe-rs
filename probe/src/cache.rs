use std::{
	fs::{self, File, OpenOptions},
	io::{Read, Write},
	os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
};

use rustix::{
	fs::{CWD, RenameFlags, renameat_with},
	process::geteuid,
};

use crate::{
	codec::{MAXIMUM_PROFILE_BYTES, MeasuredProfileCodec},
	error::{ProbeError, ProbeResult},
	model::{CacheIdentity, MeasuredProfile, ProfileCache},
};

const PRIVATE_FILE_MODE: u32 = 0o600;
const PRIVATE_DIRECTORY_MASK: u32 = 0o077;
const O_CLOEXEC: i32 = 0o2_000_000;
const O_NOFOLLOW: i32 = 0o400_000;

/// Opt-in single-file cache at one caller-selected absolute path.
///
/// This type has no default path and never overwrites an existing, different
/// cache file. Callers can use identity-derived filenames when retaining
/// multiple hardware profiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplicitPathProfileCache {
	path: PathBuf,
}

impl ExplicitPathProfileCache {
	pub fn new(path: impl Into<PathBuf>) -> ProbeResult<Self> {
		let path = path.into();
		if !path.is_absolute() {
			return Err(cache_error("cache path must be absolute"));
		}
		if path.file_name().is_none() {
			return Err(cache_error("cache path must name a file"));
		}
		Ok(Self { path })
	}

	#[must_use]
	pub fn path(&self) -> &Path { &self.path }

	fn parent(&self) -> ProbeResult<&Path> {
		let parent = self
			.path
			.parent()
			.ok_or_else(|| cache_error("cache path has no parent directory"))?;
		let canonical = fs::canonicalize(parent)
			.map_err(|error| ProbeError::io("canonicalize cache directory", parent, error))?;
		if canonical != parent {
			return Err(cache_error(format!(
				"cache directory {} is not canonical or contains a symlink",
				parent.display()
			)));
		}
		let metadata = fs::symlink_metadata(parent)
			.map_err(|error| ProbeError::io("inspect cache directory", parent, error))?;
		if !metadata.is_dir() || metadata.file_type().is_symlink() {
			return Err(cache_error("cache parent must be a real directory"));
		}
		if metadata.mode() & PRIVATE_DIRECTORY_MASK != 0 {
			return Err(cache_error(format!(
				"cache directory {} must not grant group or other permissions",
				parent.display()
			)));
		}
		if metadata.uid() != geteuid().as_raw() {
			return Err(cache_error(format!(
				"cache directory {} is not owned by the effective user",
				parent.display()
			)));
		}
		Ok(parent)
	}

	fn load_existing(&self, identity: CacheIdentity) -> ProbeResult<Option<MeasuredProfile>> {
		self.parent()?;
		let metadata = match fs::symlink_metadata(&self.path) {
			Ok(metadata) => metadata,
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
			Err(error) => return Err(ProbeError::io("inspect cache", &self.path, error)),
		};
		if metadata.file_type().is_symlink() || !metadata.is_file() {
			return Err(cache_error(format!(
				"cache target {} must be a regular non-symlink file",
				self.path.display()
			)));
		}
		if metadata.mode() & PRIVATE_DIRECTORY_MASK != 0 {
			return Err(cache_error(format!(
				"cache file {} has insecure permissions {:o}",
				self.path.display(),
				metadata.mode() & 0o777
			)));
		}
		if metadata.uid() != geteuid().as_raw() {
			return Err(cache_error(format!(
				"cache file {} is not owned by the effective user",
				self.path.display()
			)));
		}
		let mut file = OpenOptions::new()
			.read(true)
			.custom_flags(O_NOFOLLOW | O_CLOEXEC)
			.open(&self.path)
			.map_err(|error| ProbeError::io("open cache", &self.path, error))?;
		let opened = file
			.metadata()
			.map_err(|error| ProbeError::io("inspect opened cache", &self.path, error))?;
		if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
			return Err(cache_error("cache target changed while it was opened"));
		}
		if opened.len() > MAXIMUM_PROFILE_BYTES as u64 {
			return Err(cache_error(format!(
				"cache file is {} bytes; limit is {MAXIMUM_PROFILE_BYTES}",
				opened.len()
			)));
		}
		let mut bytes = Vec::with_capacity(
			usize::try_from(opened.len()).map_err(|_| cache_error("cache length does not fit usize"))?,
		);
		file.read_to_end(&mut bytes)
			.map_err(|error| ProbeError::io("read cache", &self.path, error))?;
		let profile = MeasuredProfileCodec::decode(&bytes)?;
		if profile.cache_identity != identity {
			return Err(cache_error(format!(
				"stale cache identity: file has {:?}, requested {:?}",
				profile.cache_identity, identity
			)));
		}
		Ok(Some(profile))
	}
}

impl ProfileCache for ExplicitPathProfileCache {
	fn load(&self, identity: CacheIdentity) -> ProbeResult<Option<MeasuredProfile>> { self.load_existing(identity) }

	fn store(&self, profile: &MeasuredProfile) -> ProbeResult<()> {
		let bytes = MeasuredProfileCodec::encode(profile)?;
		let parent = self.parent()?;
		if let Some(existing) = self.load_existing(profile.cache_identity)? {
			if existing == *profile {
				return Ok(());
			}
			return Err(cache_error(format!(
				"cache target {} already contains a different profile",
				self.path.display()
			)));
		}

		let mut temporary = PendingFile::create(parent, &self.path)?;
		temporary
			.file
			.as_mut()
			.expect("temporary file is open")
			.write_all(&bytes)
			.map_err(|error| ProbeError::io("write cache temporary", &temporary.path, error))?;
		let file = temporary.file.take().expect("temporary file is open");
		file.sync_all()
			.map_err(|error| ProbeError::io("sync cache temporary", &temporary.path, error))?;
		drop(file);

		match renameat_with(
			CWD,
			&temporary.path,
			CWD,
			&self.path,
			RenameFlags::NOREPLACE,
		) {
			Ok(()) => temporary.committed = true,
			Err(error) => {
				if let Some(existing) = self.load_existing(profile.cache_identity)?
					&& existing == *profile
				{
					return Ok(());
				}
				return Err(ProbeError::io(
					"atomically install cache without replacement",
					&self.path,
					error,
				));
			}
		}
		File::open(parent)
			.and_then(|directory| directory.sync_all())
			.map_err(|error| ProbeError::io("sync cache directory", parent, error))?;
		Ok(())
	}
}

#[derive(Debug)]
struct PendingFile {
	path: PathBuf,
	file: Option<File>,
	committed: bool,
}

impl PendingFile {
	fn create(parent: &Path, target: &Path) -> ProbeResult<Self> {
		static NEXT: AtomicU64 = AtomicU64::new(1);
		let target_name = target
			.file_name()
			.and_then(|value| value.to_str())
			.ok_or_else(|| cache_error("cache filename must be UTF-8"))?;
		for _ in 0..64 {
			let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
			let path = parent.join(format!(".{target_name}.tmp.{}.{nonce}", std::process::id()));
			match OpenOptions::new()
				.read(true)
				.write(true)
				.create_new(true)
				.mode(PRIVATE_FILE_MODE)
				.custom_flags(O_NOFOLLOW | O_CLOEXEC)
				.open(&path)
			{
				Ok(file) => {
					file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
						.map_err(|error| ProbeError::io("set cache temporary permissions", &path, error))?;
					return Ok(Self {
						path,
						file: Some(file),
						committed: false,
					});
				}
				Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
				Err(error) => {
					return Err(ProbeError::io("create cache temporary", &path, error));
				}
			}
		}
		Err(cache_error(
			"could not allocate a unique same-directory cache temporary file",
		))
	}
}

impl Drop for PendingFile {
	fn drop(&mut self) {
		if !self.committed {
			let _ = fs::remove_file(&self.path);
		}
	}
}

fn cache_error(message: impl Into<String>) -> ProbeError { ProbeError::Cache(message.into()) }

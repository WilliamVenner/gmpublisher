use byteorder::{LittleEndian, WriteBytesExt};
use lazy_static::lazy_static;
use rayon::ThreadPool;
use std::{
	collections::LinkedList,
	fs::{self, File},
	io::{BufWriter, Write},
	path::Path,
	sync::{atomic::AtomicBool, Arc},
	time::SystemTime,
};

use path_slash::PathExt;
use walkdir::WalkDir;

use crate::{transactions::Transaction, GMAFile, NTStringWriter};

use super::{whitelist, GMAError, GMAMetadata};

use super::GMA_HEADER;

lazy_static! {
	static ref THREAD_POOL: ThreadPool = thread_pool!();
}

impl NTStringWriter for BufWriter<File> {}

impl GMAFile {
	pub fn write(&self) -> Result<BufWriter<File>, GMAError> {
		Ok(BufWriter::new(File::create(&self.path)?))
	}

	pub fn create<P: AsRef<Path>>(&self, src_path: P, transaction: Transaction) -> Result<(), GMAError> {
		let mut f = self.write()?;

		let src_path = src_path.as_ref();

		let metadata = self.metadata.as_ref().expect("Expected metadata to be set");
		let ignore = metadata.ignore().map(|ignore| {
			ignore.into_iter().map(|ignore| {
				let mut ignore = ignore.to_owned();
				ignore.push('\0');
				ignore
			}).collect::<Vec<_>>().into_boxed_slice()
		});

		let (title, addon_json) = match metadata {
			GMAMetadata::Legacy { title, .. } => (title.as_str(), None),
			GMAMetadata::Standard { title, .. } => (title.as_str(), Some(metadata)),
		};

		f.write_all(GMA_HEADER)?;

		f.write_u8(3)?; // gma version

		// steamid [unused]
		f.write_u64::<LittleEndian>(0)?;

		// timestamp [unused]
		f.write_u64::<LittleEndian>(match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
			Ok(unix) => unix.as_secs(),
			Err(_) => 0,
		})?;

		// required content [unused]
		f.write_u8(0)?;

		// addon name
		f.write_nt_string(title)?;

		// addon description
		match addon_json {
			Some(addon_json) => f.write_nt_string(serde_json::ser::to_string(addon_json).as_deref().unwrap())?,
			None => f.write_nt_string("Description")?,
		};

		// addon author [unused]
		f.write_nt_string("Author Name")?;

		// addon version [unused]
		f.write_i32::<LittleEndian>(1)?;

		// file list
		let (error, rx, total) = {
			let error = Arc::new(AtomicBool::new(false));

			let (tx, rx) = crossbeam::channel::unbounded();

			let root_path_strip_len = src_path.to_string_lossy().len();

			let mut total = 0.;
			for (path, relative_path) in WalkDir::new(src_path).follow_links(true).into_iter().filter_map(|entry| {
				entry.ok().and_then(|entry| {
					if entry.file_type().is_file() {
						let path = entry.into_path();

						let relative_path = path.to_slash_lossy()[root_path_strip_len..].trim_matches('/').to_lowercase();

						if whitelist::check(&relative_path) {
							if let Some(ref ignore) = ignore {
								if whitelist::is_ignored(&relative_path, ignore) {
									return None;
								}
							}
							return Some((path, relative_path));
						} else {
							transaction.data(("ERR_WHITELIST", relative_path));
						}
					}
					None
				})
			}) {
				if error.load(std::sync::atomic::Ordering::Acquire) {
					break;
				}

				let tx = tx.clone();
				let transaction = transaction.clone();
				let error = error.clone();
				THREAD_POOL.spawn(move || {
					let contents = match fs::read(&path) {
						Ok(contents) => contents,
						Err(_) => {
							return {
								error.store(true, std::sync::atomic::Ordering::Release);
								transaction.error("ERR_PATH_IO_ERROR", path);
							}
						}
					};

					let mut crc32 = crc32fast::Hasher::new();
					crc32.reset();
					crc32.update(&contents);
					let crc32 = crc32.finalize();

					tx.send((relative_path.into_bytes().into_boxed_slice(), contents.into_boxed_slice(), crc32))
						.unwrap();
				});

				total += 1.;
			}

			(error, rx, total)
		};

		let mut entries_buf = LinkedList::new();

		let mut i = 0;
		let mut i_f: f64 = 0.;
		while let Ok((path, contents, crc32)) = rx.recv() {
			i += 1;

			f.write_u32::<LittleEndian>(i as u32)?;
			f.write_all(&path)?;
			f.write_u8(0)?;
			f.write_i64::<LittleEndian>(contents.len() as i64)?;
			f.write_u32::<LittleEndian>(crc32)?;

			entries_buf.push_back(contents);

			i_f += 1.;
			transaction.progress(i_f / total);
		}

		f.write_u32::<LittleEndian>(0)?;

		for contents in entries_buf.into_iter() {
			f.write_all(&contents)?;
		}

		let written = f.buffer();

		let mut crc32 = crc32fast::Hasher::new();
		crc32.reset();
		crc32.update(written);
		let crc32 = crc32.finalize();

		f.write_u32::<LittleEndian>(crc32)?;

		if Arc::try_unwrap(error).unwrap().into_inner() {
			return Err(GMAError::IOError);
		}

		Ok(())
	}
}

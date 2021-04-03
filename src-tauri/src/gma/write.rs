use std::{collections::LinkedList, fs::{self, File}, io::{BufWriter, Seek, Write}, path::{Path, PathBuf}, sync::{atomic::AtomicBool, mpsc}, time::SystemTime};
use byteorder::{LittleEndian, WriteBytesExt};
use lazy_static::lazy_static;
use rayon::{ThreadBuilder, ThreadPool, ThreadPoolBuilder, iter::{IntoParallelIterator, ParallelBridge, ParallelIterator}};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use path_slash::PathExt;

use crate::{GMAError, octopus::steamworks::publishing::ContentPath, transactions::Transaction, whitelist, GMAMetadata};

use super::GMA_HEADER;

lazy_static! {
	static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

pub struct GMAWriteHandle<W: Write + Seek> {
	pub inner: BufWriter<W>
}
impl<W: Write + Seek + Send> GMAWriteHandle<W> {
	pub fn write_nt_string(&mut self, str: &str) -> Result<(), std::io::Error> {
		self.write(str.as_bytes())?;
		self.write_u8(0)?;
		Ok(())
	}

	pub(crate) fn create<P: AsRef<Path>>(mut self, src_path: P, data: &GMAMetadata, transaction: Transaction) -> Result<(), GMAError> {
		let src_path = src_path.as_ref();
		
		let (title, addon_json) = match data {
			GMAMetadata::Legacy(data) => (data.title.as_str(), None),
			GMAMetadata::Standard(data) => (data.title.as_str(), Some(data))
		};

		self.write(GMA_HEADER)?;

		// steamid [unused]
		self.write_u64::<LittleEndian>(0)?;

		// timestamp [unused]
		self.write_u64::<LittleEndian>(match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
			Ok(unix) => unix.as_secs(),
			Err(_) => 0
		})?;

		// required content [unused]
		self.write_u8(0)?;

		// addon name
		self.write_nt_string(title)?;

		// addon description
		match addon_json {
			Some(addon_json) => self.write_nt_string(serde_json::ser::to_string(addon_json).as_deref().unwrap())?,
			None => self.write_nt_string("Description")?
		};

		// addon author [unused]
		self.write_nt_string("Author Name")?;

		// addon version [unused]
		self.write_i32::<LittleEndian>(1)?;

		// file list
		// FIXME handle IO errors, don't unwrap
		let (rx, total) = {
			let (tx, rx) = mpsc::channel();

			let root_path_strip_len = src_path.to_string_lossy().len() + 1;

			let mut total = 0;
			for (path, relative_path) in WalkDir::new(src_path).into_iter().filter_map(|entry| {
				entry.ok().and_then(|entry| {
					if entry.file_type().is_file() {
						let path = entry.into_path();

						let mut relative_path = path.to_slash_lossy().to_lowercase();
						{ relative_path.drain(0..root_path_strip_len); }

						if whitelist::check(&relative_path) {
							return Some((path, relative_path));
						} else {
							transaction.data(("ERR_WHITELIST", relative_path));
						}
					}
					None
				})
			}) {
				let tx = tx.clone();
				THREAD_POOL.spawn(move || {
					let contents = fs::read(&path).unwrap();

					let mut crc32 = crc32fast::Hasher::new();
					crc32.reset();
					crc32.update(&contents);
					let crc32 = crc32.finalize();

					tx.send((
						relative_path.into_bytes().into_boxed_slice(),
						contents.into_boxed_slice(),
						crc32
					)).unwrap();
				});

				total += 1;
			}

			(rx, total as f64)
		};

		let mut entries_list_buf = LinkedList::new();
		let mut entries_buf = LinkedList::new();

		let mut i: f64 = 0.;
		while let Ok((path, contents, crc32)) = rx.recv() {
			entries_list_buf.push_back((path, contents.len(), crc32));
			entries_buf.push_back(contents);

			i += 1.;
			transaction.progress(i / total);
		}

		for (i, (path, size, crc32)) in entries_list_buf.into_iter().enumerate() {
			self.write_u32::<LittleEndian>((i + 1) as u32)?;
			self.write(&path)?;
			self.write_u8(0)?;
			self.write_i64::<LittleEndian>(size as i64)?;
			self.write_u32::<LittleEndian>(crc32)?;
		}

		self.write_u32::<LittleEndian>(0)?;

		for contents in entries_buf.into_iter() {
			self.write(&contents)?;
			self.write_u8(0)?;
		}
		
		let written = self.buffer();

		let mut crc32 = crc32fast::Hasher::new();
		crc32.reset();
		crc32.update(written);
		let crc32 = crc32.finalize();

		self.write_u32::<LittleEndian>(crc32)?;

		transaction.finished(turbonone!());

		Ok(())
	}
}
impl<W: Write + Seek> std::ops::Deref for GMAWriteHandle<W> {
    type Target = BufWriter<W>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<W: Write + Seek> std::ops::DerefMut for GMAWriteHandle<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

use std::{collections::LinkedList, fs::{self, File}, io::{BufWriter, Seek, Write}, path::PathBuf, sync::{atomic::AtomicBool, mpsc}, time::SystemTime};
use byteorder::{LittleEndian, WriteBytesExt};
use lazy_static::lazy_static;
use rayon::{ThreadBuilder, ThreadPool, ThreadPoolBuilder, iter::{IntoParallelIterator, ParallelBridge, ParallelIterator}};
use serde::{Serialize, Deserialize};
use walkdir::WalkDir;
use path_slash::PathExt;

use crate::{GMAError, octopus::steamworks::publishing::ContentPath};

use super::GMA_HEADER;

lazy_static! {
	static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GMACreationData {
	#[serde(skip)]
	pub title: String,

	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
	pub ignore: Vec<String>,
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

	pub(crate) fn create(mut self, src_path: PathBuf, data: GMACreationData) -> Result<(), GMAError> {
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
		self.write_nt_string(data.title.as_str())?;

		// addon description
		self.write_nt_string(&serde_json::ser::to_string(&data).unwrap())?;

		// addon author [unused]
		self.write_nt_string("Author Name")?;

		// addon version [unused]
		self.write_i32::<LittleEndian>(1)?;

		// file list
		// FIXME handle IO errors, don't unwrap
		// TODO whitelist
		THREAD_POOL.install(move || {
			let (tx, rx) = mpsc::channel();

			let root_path_strip_len = src_path.to_string_lossy().len() + 1;

			let paths: Vec<PathBuf> = WalkDir::new(src_path).into_iter().filter_map(|entry| entry.ok().and_then(|entry| if entry.file_type().is_file() { Some(entry.into_path()) } else { None })).collect();

			paths.into_par_iter().for_each_with(tx, |tx, path| {
				let contents = fs::read(&path).unwrap();

				let mut crc32 = crc32fast::Hasher::new();
				crc32.reset();
				crc32.update(&contents);
				let crc32 = crc32.finalize();

				let mut path = path.to_slash_lossy().to_lowercase().into_bytes();
				{ path.drain(0..root_path_strip_len); }

				tx.send((
					path.into_boxed_slice(),
					contents.into_boxed_slice(),
					crc32
				)).unwrap();
			});
	
			let mut entries_list_buf = LinkedList::new();
			let mut entries_buf = LinkedList::new();

			while let Ok((path, contents, crc32)) = rx.recv() {
				entries_list_buf.push_back((path, contents.len(), crc32));
				entries_buf.push_back(contents);
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

			Ok(())
		})
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

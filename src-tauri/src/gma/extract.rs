use std::{fs::{self, File}, io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom}, path::{Path, PathBuf}, sync::atomic::{AtomicUsize, Ordering}};

use crate::{app_data, transactions::Transaction};

use super::{whitelist, GMAEntry, GMAError, GMAFile, GMAMetadata};

use lazy_static::lazy_static;
use rayon::{
	iter::{IntoParallelRefIterator, ParallelIterator},
	ThreadPool, ThreadPoolBuilder,
};
use serde::{Serialize, Deserialize};
use sysinfo::SystemExt;

lazy_static! {
	pub static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractDestination {
	Temp,
	Downloads,
	Addons,
	/// path/to/addon/*
	Directory(PathBuf),
	/// path/to/addon/addon_name_123456790/*
	NamedDirectory(PathBuf),
}
impl ExtractDestination {
	fn into<S: AsRef<str>>(self, extracted_name: S) -> PathBuf {
		use ExtractDestination::*;

		let push_extracted_name = |mut path: PathBuf| {
			path.push(extracted_name.as_ref());
			Some(path)
		};

		match self {

			Temp => None,

			Directory(path) => Some(path),

			Addons => app_data!().gmod_dir().and_then(|mut path| {
				path.push("GarrysMod");
				path.push("addons");
				Some(path)
			}),

			Downloads => app_data!().downloads_dir().to_owned().and_then(push_extracted_name),

			NamedDirectory(path) => push_extracted_name(path),

		}
		.unwrap_or_else(|| push_extracted_name(app_data!().temp_dir().to_owned()).unwrap())
	}
}
impl Default for ExtractDestination {
    fn default() -> Self {
        ExtractDestination::Temp
    }
}

impl GMAFile {
	pub fn decompress<P: AsRef<Path>>(path: P) -> Result<GMAFile, GMAError> {
		main_thread_forbidden!();

		let available_memory = ({
			let mut sys = sysinfo::System::new();
			sys.refresh_memory();
			sys.get_available_memory()
		} * 1000) - 1000000000;

		// TODO somehow, in some really unsafe and stupid way, monitor the progress of decompression

		let input = std::fs::read(path.as_ref()).map_err(|_| GMAError::IOError)?;
		let mut output = Vec::with_capacity(input.len());
		let status = xz2::stream::Stream::new_lzma_decoder(available_memory).map_err(|_| GMAError::LZMA)?.process_vec(&input, &mut output, xz2::stream::Action::Run).map_err(|_| GMAError::LZMA)?;

		if let xz2::stream::Status::Ok = status {
			Ok(GMAFile::read_header(Cursor::new(output), path)?)
		} else {
			Err(GMAError::LZMA)
		}
	}

	pub fn extract(&mut self, dest: ExtractDestination, transaction: Transaction, open: bool) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		THREAD_POOL.install(move || {
			self.entries()?;

			println!("{:#?}", self.entries);

			let mut dest_path = dest.into(&self.extracted_name);
			let entries_start = self.pointers.entries;

			let entries = self.entries.as_ref().unwrap();
			let entries_len = entries.len() as f64;

			let i = AtomicUsize::new(0);
			entries.par_iter().for_each_init(
				|| File::open(&self.path).map(|f| BufReader::new(f)),
				|handle, (entry_path, entry)| match handle {
					Ok(handle) => {
						if whitelist::check(entry_path) {
							// FIXME count errors, check if errors == number of entries, return an error instead of finished
							ignore! { GMAFile::stream_entry_bytes(handle, entries_start, &dest_path.join(entry_path), entry) };
							transaction.progress(((i.fetch_add(1, Ordering::AcqRel) + 1) as f64) / entries_len);
						} else {
							transaction.error("ERR_WHITELIST", entry_path.clone());
						}
					}
					Err(_) => transaction.error("ERR_GMA_IO_ERROR", entry_path.clone()),
				},
			);

			if transaction.aborted() {
				Err(GMAError::IOError)
			} else {
				let metadata = self.metadata.as_ref().unwrap();
				if let GMAMetadata::Standard { .. } = metadata {
					if let Ok(json) = serde_json::ser::to_string_pretty(metadata) {
						dest_path.push("addon.json");
						ignore! { fs::write(&dest_path, json.as_bytes()) };
						dest_path.pop();
					}
				}

				if open {
					ignore! { crate::path::open(&dest_path) };
				}

				transaction.finished(Some(dest_path.to_owned()));

				Ok(dest_path)
			}
		})
	}

	pub fn extract_entry(&mut self, entry_path: String) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		let mut handle = match self.entries()? {
			Some(handle) => handle,
			None => self.read()?,
		};

		let entry = self.entries.as_ref().unwrap().get(&entry_path).ok_or(GMAError::EntryNotFound)?;
		debug_assert_ne!(entry.index, 0);

		let mut path = std::env::temp_dir();
		path.push("gmpublisher");
		path.push(&self.extracted_name);
		path.push(&entry_path);

		GMAFile::stream_entry_bytes(&mut handle, self.pointers.entries, &path, entry)?;

		Ok(path)
	}

	fn stream_entry_bytes<R: Read + Seek>(
		handle: &mut BufReader<R>,
		entries_start: u64,
		entry_path: &PathBuf,
		entry: &GMAEntry,
	) -> Result<(), GMAError> {
		fs::create_dir_all(&entry_path.with_file_name(""))?;
		let f = File::create(entry_path)?;

		handle.seek(SeekFrom::Start(entries_start + entry.index))?;

		crate::stream_bytes(handle, &mut BufWriter::new(f), entry.size as usize)?;

		Ok(())
	}
}

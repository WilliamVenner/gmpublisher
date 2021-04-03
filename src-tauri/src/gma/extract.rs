use std::{fs::{self, File}, io::{BufReader, BufWriter, Read, Seek, SeekFrom}, path::PathBuf, sync::atomic::{AtomicUsize, Ordering}};

use crate::{whitelist, app_data, transactions::Transaction};

use super::{GMAError, GMAFile, GMAEntry, GMAMetadata};

use lazy_static::lazy_static;
use rayon::{ThreadPool, ThreadPoolBuilder, iter::{IntoParallelRefIterator, ParallelIterator}};
use serde::Deserialize;

lazy_static! {
	static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

#[derive(Debug, Clone, Deserialize)]
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
    fn into(self, extracted_name: &String) -> PathBuf {
        use ExtractDestination::*;

		let push_extracted_name = |mut path: PathBuf| {
			path.push(extracted_name);
			Some(path)
		};

		match self {

			Temp => None,

		    Directory(path) => Some(path),
			
			Addons => app_data!().gmod().and_then(|mut path| {
				path.push("garrysmod");
				path.push("addons");
				Some(path)
			}),

			Downloads => dirs::download_dir().and_then(push_extracted_name),

			NamedDirectory(path) => push_extracted_name(path),

		}.unwrap_or_else(|| {
			let mut path = std::env::temp_dir();
			path.push("gmpublisher");
			push_extracted_name(path).unwrap()
		})
    }
}

impl GMAFile {
	pub fn extract(&mut self, dest: ExtractDestination, transaction: Transaction) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		self.entries()?;

		// FIXME: unwrap
		THREAD_POOL.install(move || {
			let mut dest_path = dest.into(&self.extracted_name);
			let entries_start = self.pointers.entries;

			let entries = self.entries.as_ref().unwrap();
			let entries_len = entries.len() as f64;

			let i = AtomicUsize::new(0);
			entries.par_iter().for_each_init(
				|| BufReader::new(File::open(&self.path).unwrap()),
				|handle, (entry_path, entry)| {
					if whitelist::check(entry_path) {
						ignore! { GMAFile::stream_entry_bytes(handle, entries_start, &dest_path.join(entry_path), entry) };
						transaction.progress(((i.fetch_add(1, Ordering::AcqRel) + 1) as f64) / entries_len);
					} else {
						transaction.data(("ERR_WHITELIST", entry_path.clone()));
					}
				}
			);

			if let GMAMetadata::Standard(metadata) = self.metadata.clone().unwrap() {
				dest_path.push("addon.json");
				ignore! { fs::write(&dest_path, serde_json::ser::to_string_pretty(&metadata).unwrap().as_bytes()) };
				dest_path.pop();
			}

			transaction.finished(turbonone!());

			Ok(dest_path)
		})
	}

	pub fn extract_entry(&mut self, entry_path: String) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		let mut handle = match self.entries()? {
			Some(handle) => handle,
			None => self.read()?
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

	fn stream_entry_bytes<R: Read + Seek>(handle: &mut BufReader<R>, entries_start: u64, entry_path: &PathBuf, entry: &GMAEntry) -> Result<(), GMAError> {
		fs::create_dir_all(&entry_path.with_file_name(""))?;
		let f = File::create(entry_path)?;

		handle.seek(SeekFrom::Start(entries_start + entry.index))?;

		crate::stream_bytes(handle, &mut BufWriter::new(f), entry.size as usize)?;

		Ok(())
	}
}
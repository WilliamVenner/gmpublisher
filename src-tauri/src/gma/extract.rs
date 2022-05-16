use std::{fs::{self, File}, io::{BufWriter, Cursor, Read, SeekFrom}, path::{Path, PathBuf}, sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}}};

use crate::{app_data, transactions::Transaction};

use super::{whitelist, GMAEntry, GMAError, GMAFile, GMAMetadata, GMAReader};

use lazy_static::lazy_static;
use rayon::{
	iter::{IntoParallelRefIterator, ParallelIterator},
	ThreadPool,
};
use serde::{Deserialize, Serialize};

lazy_static! {
	pub static ref THREAD_POOL: ThreadPool = thread_pool!();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionOverwriteMode {
	Overwrite,
	Recycle,
	Delete
}
impl Default for ExtractionOverwriteMode {
    fn default() -> Self {
        ExtractionOverwriteMode::Recycle
    }
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
	fn prepare<S: AsRef<str>>(self, extracted_name: S) -> PathBuf {
		use ExtractDestination::*;

		let push_extracted_name = |mut path: PathBuf| {
			path.push(extracted_name.as_ref());
			Some(path)
		};

		let recycle_existing = !matches!(self, Directory(_));

		let mut path = match self {
			Temp => None,

			Directory(path) => Some(path),

			Addons => app_data!().gmod_dir().and_then(|mut path| {
				path.push("GarrysMod");
				path.push("addons");
				path.push(extracted_name.as_ref());
				Some(path)
			}),

			Downloads => app_data!().downloads_dir().to_owned().and_then(push_extracted_name),

			NamedDirectory(path) => push_extracted_name(path),
		}
		.unwrap_or_else(|| push_extracted_name(app_data!().temp_dir().to_owned()).unwrap());

		if recycle_existing && path.exists() {
			let success = match &app_data!().settings.read().extract_overwrite_mode {
			    ExtractionOverwriteMode::Overwrite => true,
			    ExtractionOverwriteMode::Recycle => trash::delete(&path).is_ok(),
			    ExtractionOverwriteMode::Delete => fs::remove_dir_all(&path).is_ok(),
			};
			if !success {
				let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
				path.pop();

				let mut i: u8 = 0;
				while i < 255 {
					i += 1;

					path.push(format!("{} ({})", dir_name, i));

					if !path.exists() {
						break;
					} else {
						path.pop();
					}
				}
			}
		}

		path
	}
}
impl Default for ExtractDestination {
	fn default() -> Self {
		ExtractDestination::Temp
	}
}

impl GMAFile {
	pub fn decompress<P: AsRef<Path>>(path: P, transaction: Transaction) -> Result<GMAFile, GMAError> {
		main_thread_forbidden!();

		let input = File::open(path.as_ref())?;
		let mut bytes_total = input.metadata().map(|metadata| metadata.len()).ok();

		let lzma_decoder = xz2::stream::Stream::new_lzma_decoder(u64::MAX).map_err(|_| GMAError::LZMA)?;
		let mut xz_decoder = xz2::read::XzDecoder::new_stream(input, lzma_decoder);

		let mut output = if let Some(ref bytes_total) = bytes_total {
			Vec::with_capacity((*bytes_total * 5) as usize)
		} else {
			Vec::new()
		};

		let result = if let Some(bytes_total) = bytes_total.take() {
			transaction.data((turbonone!(), bytes_total));

			let bytes_total_f = bytes_total as f64;

			let complete = Arc::new(AtomicBool::new(false));
			let complete_ref = complete.clone();

			struct StupidlyUnsafeProgressMonitorPtr(*const xz2::read::XzDecoder<File>);
			unsafe impl Sync for StupidlyUnsafeProgressMonitorPtr {}
			unsafe impl Send for StupidlyUnsafeProgressMonitorPtr {}

			let xz_decoder_ptr = StupidlyUnsafeProgressMonitorPtr(&xz_decoder as *const _);
			rayon::spawn(move || while !complete_ref.load(Ordering::Acquire) {
				let xz_decoder = unsafe { &*xz_decoder_ptr.0 };

				let bytes_read = xz_decoder.total_in() as f64;
				transaction.progress(bytes_read / bytes_total_f);

				let decompressed_bytes = xz_decoder.total_out();
				if decompressed_bytes > bytes_total {
					transaction.data((turbonone!(), decompressed_bytes));
				}

				sleep_ms!(25);
			});

			let result = xz_decoder.read_to_end(&mut output);
			complete.store(true, Ordering::Release);
			result
		} else {
			xz_decoder.read_to_end(&mut output)
		};

		output.shrink_to_fit();

		if let Err(err) = result {
			if !matches!(err.kind(), std::io::ErrorKind::Other) {
				return Err(GMAError::LZMA);
			}
		}

		let decompressed_size = output.len() as u64;

		let mut gma = GMAFile::read_header(GMAReader::MemBuffer(Cursor::new(output.into())), path)?;
		gma.size = decompressed_size;

		Ok(gma)
	}

	fn stream_entry_bytes_with_transaction(
		handle: &mut GMAReader,
		entries_start: u64,
		entry_path: &PathBuf,
		entry: &GMAEntry,
		transaction: &Transaction,
	) -> Result<(), GMAError> {
		use std::io::Write;

		fs::create_dir_all(&entry_path.with_file_name(""))?;
		let f = File::create(&entry_path)?;

		handle.seek(SeekFrom::Start(entries_start + entry.index))?;

		let mut w = BufWriter::new(f);
		crate::stream_bytes_with_transaction(&mut **handle, &mut w, entry.size as usize, transaction)?;

		w.flush()?;

		Ok(())
	}

	fn stream_entry_bytes(handle: &mut GMAReader, entries_start: u64, entry_path: &PathBuf, entry: &GMAEntry) -> Result<(), GMAError> {
		use std::io::Write;

		fs::create_dir_all(&entry_path.with_file_name(""))?;
		let f = File::create(&entry_path)?;

		handle.seek(SeekFrom::Start(entries_start + entry.index))?;

		let mut w = BufWriter::new(f);
		crate::stream_bytes(&mut **handle, &mut w, entry.size as usize)?;

		w.flush()?;

		Ok(())
	}
}

pub trait ExtractGMAImmut {
	fn extract(&self, dest: ExtractDestination, transaction: &Transaction, open_after_extract: bool, ignore_whitelist: bool) -> Result<PathBuf, GMAError>;
	fn extract_entry(&self, entry_path: String, transaction: &Transaction, open_after_extract: bool) -> Result<PathBuf, GMAError>;
	fn extract_entry_with_handle(
		&self,
		entry_path: String,
		transaction: &Transaction,
		open_after_extract: bool,
		handle: Option<GMAReader>,
	) -> Result<PathBuf, GMAError>;
}
pub trait ExtractGMAMut {
	fn extract(&mut self, dest: ExtractDestination, transaction: &Transaction, open_after_extract: bool, ignore_whitelist: bool) -> Result<PathBuf, GMAError>;
	fn extract_entry(&mut self, entry_path: String, transaction: &Transaction, open_after_extract: bool) -> Result<PathBuf, GMAError>;
}
impl ExtractGMAImmut for GMAFile {
	fn extract(&self, dest: ExtractDestination, transaction: &Transaction, open_after_extract: bool, ignore_whitelist: bool) -> Result<PathBuf, GMAError> {
		let result = THREAD_POOL.install(move || {
			let dest_path = dest.prepare(&self.extracted_name);
			let entries_start = self.pointers.entries;

			let entries = self.entries.as_ref().unwrap();
			let entries_len_f = entries.len() as f64;
			let entries_len_i = entries.len();

			self.read()?; // Don't waste time with the threads if the file fails to open

			let i = AtomicUsize::new(0);

			let finished = |mut dest_path: PathBuf| {
				if i.fetch_add(1, Ordering::AcqRel) > entries_len_i || transaction.aborted() {
					return;
				}

				transaction.finished(dest_path.to_owned());

				if open_after_extract {
					crate::path::open(&dest_path);
				}

				let metadata = self.metadata.as_ref().unwrap();
				if let GMAMetadata::Standard { .. } = metadata {
					if let Ok(json) = serde_json::ser::to_string_pretty(metadata) {
						dest_path.push("addon.json");
						if let Some(parent) = dest_path.parent() {
							ignore! { fs::create_dir_all(parent) };
						}
						ignore! { fs::write(dest_path, json.as_bytes()) };
					}
				}
			};

			entries
				.par_iter()
				.try_for_each(|(entry_path, entry)| -> Result<(), GMAError> {
					let mut handle = self.read()?;

					if ignore_whitelist || whitelist::check(entry_path) {
						if transaction.aborted() {
							return Err(GMAError::Cancelled);
						}

						// FIXME count errors, check if errors == number of entries, return an error instead of finished
						ignore! { GMAFile::stream_entry_bytes(&mut handle, entries_start, &dest_path.join(entry_path), entry) };

						let i = i.fetch_add(1, Ordering::AcqRel) + 1;
						transaction.progress((i as f64) / entries_len_f);

						if i == entries_len_i {
							(finished)(dest_path.to_owned());
						}
					} else {
						transaction.error("ERR_WHITELIST", entry_path.clone()); // TODO
					}

					Ok(())
				})
				.map(|_| {
					(finished)(dest_path.to_owned());
					dest_path
				})
		});

		if !transaction.aborted() {
			if let Err(ref error) = result {
				transaction.error(error.to_string(), turbonone!());
			}
		}

		result
	}

	fn extract_entry_with_handle(
		&self,
		entry_path: String,
		transaction: &Transaction,
		open_after_extract: bool,
		handle: Option<GMAReader>,
	) -> Result<PathBuf, GMAError> {
		let mut path = app_data!().temp_dir().to_owned();
		path.push("gmpublisher");
		path.push(&self.extracted_name);
		path.push(&entry_path);

		let mut handle = match handle {
			Some(handle) => handle,
			None => self.read()?,
		};

		let entry = self
			.entries
			.as_ref()
			.expect("Expected entries to be read by this point")
			.get(&entry_path)
			.ok_or(GMAError::EntryNotFound)?;

		let result =
			GMAFile::stream_entry_bytes_with_transaction(&mut handle, self.pointers.entries, &path, entry, transaction).map(|_| path.to_owned());

		if let Err(ref error) = result {
			if !transaction.aborted() {
				transaction.error(error.to_string(), turbonone!());
			}
		} else if !transaction.aborted() {
			if open_after_extract {
				transaction.finished(path.to_owned());
				crate::path::open(path);
			} else {
				transaction.finished(path);
			}
		}

		result
	}

	fn extract_entry(&self, entry_path: String, transaction: &Transaction, open_after_extract: bool) -> Result<PathBuf, GMAError> {
		ExtractGMAImmut::extract_entry_with_handle(self, entry_path, transaction, open_after_extract, None)
	}
}
impl ExtractGMAMut for GMAFile {
	fn extract(&mut self, dest: ExtractDestination, transaction: &Transaction, open_after_extract: bool, ignore_whitelist: bool) -> Result<PathBuf, GMAError> {
		THREAD_POOL.install(move || {
			self.entries()?;
			(&*self).extract(dest, transaction, open_after_extract, ignore_whitelist)
		})
	}
	fn extract_entry(&mut self, entry_path: String, transaction: &Transaction, open_after_extract: bool) -> Result<PathBuf, GMAError> {
		THREAD_POOL.install(move || {
			let handle = self.entries()?;
			(&*self).extract_entry_with_handle(entry_path, transaction, open_after_extract, handle)
		})
	}
}

#[tauri::command]
pub fn extract_gma(gma_path: PathBuf, dest: ExtractDestination) -> Option<u32> {
	let mut gma = GMAFile::open(gma_path).ok()?;
	gma.entries().ok()?;

	let transaction = transaction!();
	let id = transaction.id;

	rayon::spawn(move || {
		ignore! { gma.extract(dest, &transaction, true, true) };
	});

	Some(id)
}

use std::{fs::{self, File, create_dir_all}, io::{self, Read, Seek, SeekFrom, Write}, path::PathBuf, sync::{Arc, atomic::AtomicU16}};

use byteorder::{LittleEndian, ReadBytesExt};
use sysinfo::SystemExt;

use super::{GMAEntry, GMAFile, GMAReadError, ProgressCallback};

#[derive(Debug, Clone)]
pub enum ExtractDestination {
	Temp,
	/// path/to/addon/*
	Directory(PathBuf),
	/// path/to/addon/addon_name_123456790/*
	NamedDirectory(PathBuf),
}
impl ExtractDestination {
    fn resolve(self, gma: &GMAFile) -> PathBuf {
		use ExtractDestination::*;

        match self {
			Directory(ref extract_to) => extract_to.clone(),
			_ => {
				let dir_name = gma.metadata.as_ref().unwrap().name.clone().replace(|char: char| !char.is_alphanumeric(), "_");

				match self {
					NamedDirectory(ref extract_to) => {
						let mut extract_to = extract_to.clone();
						extract_to.push(dir_name);
						extract_to
					},
					Temp => {
						let mut dir = std::env::temp_dir();
						dir.push("gmpublisher");
						dir.push(dir_name);
						dir
					}
					_ => { unreachable!() }
				}
			}
		}
    }
}

impl GMAFile {
	pub fn extract(&mut self, to: ExtractDestination, progress_callback: Option<ProgressCallback>) -> Result<(), GMAReadError> {
		use ExtractDestination::*;

		let extract_to = match to {
			Directory(ref extract_to) => extract_to.clone(),
			_ => {
				let dir_name = self.metadata.as_ref().unwrap().name.clone().replace(|char: char| !char.is_alphanumeric(), "_");

				match to {
					NamedDirectory(ref extract_to) => {
						let mut extract_to = extract_to.clone();
						extract_to.push(dir_name);
						extract_to
					},
					Temp => {
						let mut dir = std::env::temp_dir();
						dir.push("gmpublisher");
						dir.push(dir_name);
						dir
					}
					_ => { unreachable!() }
				}
			}
		};

		let available_memory = ({
			let mut sys = sysinfo::System::new();
			sys.refresh_memory();
			sys.get_available_memory()
		} * 1000) - 1000000000;

		let (entries, _) = self.entries().unwrap();
		let total_entries = entries.len();

		// We should only multithread file i/o if we have enough available memory to actually store the GMA entry data
		let mut threads: Option<Vec<std::thread::JoinHandle<()>>> = if available_memory > self.size {
			Some(Vec::with_capacity(total_entries))
		} else { None };

		let progress_callback = Arc::new(progress_callback);

		let entries_start = self.entries_start.unwrap();

		if let Some(threads) = &mut threads {
			let i = Arc::new(AtomicU16::new(0));
			for entry in entries.iter() {

				let fs_path = extract_to.join(&entry.path);
				let mut handle_r = self.spawn_handle().unwrap();
				let size = entry.size;
				let index = entry.index;
				let i = i.clone();
				let progress_callback = progress_callback.clone();
				
				threads.push(std::thread::spawn(move || {
					let mut handle_w = File::create(fs_path).unwrap();

					let mut buf = vec![0; size as usize];
					handle_r.seek(SeekFrom::Start(entries_start + index)).unwrap();
					handle_r.read_exact(&mut buf).unwrap();
					handle_w.write(&buf).unwrap();
					handle_w.flush().unwrap();

					drop(handle_r);
					drop(handle_w);
	
					match *progress_callback {
						Some(ref progress_callback) => {
							let progress = i.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
							(progress_callback)(((progress as f32) / (total_entries as f32)) * 100.);
						},
						None => {}
					}
				}));

			}
		} else {
			let mut i: usize = 0;
			for entry in entries.iter() {
				let mut buf = vec![0; entry.size as usize];
				self.handle.read_exact(&mut buf).unwrap();

				let fs_path = extract_to.join(&entry.path);
				match fs::write(fs_path, buf) {
					Ok(_) => {},
					Err(_) => { return Err(GMAReadError::IOError) }
				}

				match *progress_callback {
					Some(ref progress_callback) => (progress_callback)(((i as f32) / (total_entries as f32)) * 100.),
					None => {}
				}

				i = i + 1;
			}
		}

		// Apparently some gma just completely omit the addon CRC from the end
		// Hence, we shouldn't unwrap the following since it may fail
		self.handle.read_u32::<LittleEndian>().ok(); // crc [unused]

		match *progress_callback {
			Some(ref progress_callback) => (progress_callback)(100.),
			None => {}
		}

		if let Some(threads) = threads {
			for thread in threads {
				match thread.join() {
					Ok(_) => {},
					Err(_) => return Err(GMAReadError::IOError)
				}
			}
		}

		Ok(())
	}

	pub fn extract_entry(&mut self, entry_path: String, to: ExtractDestination) -> Result<PathBuf, GMAReadError> {
		let extract_to = to.resolve(self).join(PathBuf::from(entry_path.clone()));
		
		fs::create_dir_all(extract_to.with_file_name("")).map_err(|_| GMAReadError::IOError)?;
		let mut handle_w = File::create(&extract_to).map_err(|_| GMAReadError::IOError)?;

		self.open()?;
		self.entries()?;

		let entry = self.entries.as_ref().unwrap().get(*self.entries_map.as_ref().unwrap().get(&entry_path).ok_or(GMAReadError::EntryNotFound)?).ok_or(GMAReadError::EntryNotFound)?;

		let handle = &mut self.handle;
		handle.seek(SeekFrom::Start(self.entries_start.unwrap() + entry.index)).unwrap();

		let mut buf = vec![0; entry.size as usize];
		handle.read_exact(&mut buf).unwrap();
		handle_w.write(&buf).map_err(|_| GMAReadError::IOError)?;

		drop(buf);
		handle_w.flush().map_err(|_| GMAReadError::IOError)?;

		Ok(extract_to)
	}
}

impl GMAEntry {
	// TODO
	/*pub fn extract(&mut self, mut handle: GMAFileHandle, to: ExtractDestination, progress_callback: Option<ProgressCallback>) -> Result<(), GMAReadError> {
		handle.seek(SeekFrom::Start(self.index)).unwrap();

		let mut buf = vec![0; self.size as usize];
		handle.read_exact(&mut buf).unwrap();

		Some(buf)
	}*/
}
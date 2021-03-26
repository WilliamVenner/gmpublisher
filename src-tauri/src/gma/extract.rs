use std::{fs::{self, File}, io::{Read, Seek, SeekFrom, Write}, mem::MaybeUninit, path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicU16}}};

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
    pub(crate) fn resolve(self, gma: &GMAFile) -> PathBuf {
		use ExtractDestination::*;

        match self {
			Directory(ref extract_to) => extract_to.clone(),
			_ => {
				match self {
					NamedDirectory(ref extract_to) => {
						let mut extract_to = extract_to.clone();
						extract_to.push(gma.extracted_name());
						extract_to
					},
					Temp => {
						let mut dir = std::env::temp_dir();
						dir.push("gmpublisher");
						dir.push(gma.extracted_name());
						dir
					}
					_ => { unreachable!() }
				}
			}
		}
    }

	pub(crate) fn build(tmp: bool, path: Option<PathBuf>, named_dir: bool, downloads: bool, addons: bool) -> Result<ExtractDestination, ()> {
		Ok(match tmp {
			true => ExtractDestination::Temp,
			false => {
				let mut check_exists = true;
				let mut discriminated_path = MaybeUninit::<PathBuf>::uninit();
				unsafe {
					if addons {
						*discriminated_path.as_mut_ptr() = crate::APP_DATA.read().unwrap().gmod.as_ref().unwrap().join("garrysmod/addons");
					} else if downloads {
						*discriminated_path.as_mut_ptr() = dirs::download_dir().unwrap();
					} else {
						check_exists = false;
						*discriminated_path.as_mut_ptr() = path.unwrap();
					}
				}

				let discriminated_path = unsafe { discriminated_path.assume_init() };
				if discriminated_path.is_absolute() && (!check_exists || discriminated_path.exists()) {
					match !addons && !downloads && named_dir {
						true => ExtractDestination::NamedDirectory(discriminated_path),
						false => ExtractDestination::Directory(discriminated_path)
					}
				} else {
					return Err(());
				}
			}
		})
	}
}

impl GMAFile {
	pub fn extract(&self, to: ExtractDestination, progress_callback: Option<ProgressCallback>) -> Result<PathBuf, GMAReadError> {
		use ExtractDestination::*;

		let extract_to = match to {
			Directory(ref extract_to) => extract_to.clone(),
			_ => {
				match to {
					NamedDirectory(ref extract_to) => {
						let mut extract_to = extract_to.clone();
						extract_to.push(self.extracted_name());
						extract_to
					},
					Temp => {
						let mut dir = std::env::temp_dir();
						dir.push("gmpublisher");
						dir.push(self.extracted_name());
						dir
					}
					_ => { unreachable!() }
				}
			}
		};

		fs::create_dir_all(&extract_to).unwrap();

		let available_memory = ({
			let mut sys = sysinfo::System::new();
			sys.refresh_memory();
			sys.get_available_memory()
		} * 1000) - 1000000000;

		let entries = self.entries.as_ref().expect("Expected entries to be read this point"); // TODO go through and add .expect() instead of .unwrap()
		let total_entries = entries.len();

		// We should only multithread file i/o if we have enough available memory to actually store the GMA entry data
		// TODO use some kind of reserved memory pool instead so even memory-strapped systems can multithread this
		let mut threads: Option<Vec<std::thread::JoinHandle<()>>> = if available_memory > self.size {
			Some(Vec::with_capacity(total_entries))
		} else { None };

		let progress_callback = Arc::new(progress_callback);

		let entries_start = self.entries_start.unwrap();

		if let Some(threads) = &mut threads {
			let failed = Arc::new(AtomicBool::new(false));
			let i = Arc::new(AtomicU16::new(0));
			for entry in entries.iter() {

				let fs_path = extract_to.join(&entry.path);
				fs::create_dir_all(fs_path.with_file_name("")).map_err(|_| GMAReadError::IOError)?;

				let mut handle_r = self.handle().unwrap();
				let size = entry.size;
				let index = entry.index;

				let i = i.clone();
				let progress_callback = progress_callback.clone();
				let failed = failed.clone();
				
				threads.push(std::thread::spawn(move || {
					if let Err(_) = (|| -> Result<(), std::io::Error> {

						let mut handle_w = File::create(fs_path)?;

						let mut buf = vec![0; size as usize];
						handle_r.seek(SeekFrom::Start(entries_start + index))?;
						handle_r.read_exact(&mut buf)?;
						drop(handle_r);

						handle_w.write(&buf)?;
						handle_w.flush()?;
						drop(handle_w);
						
						if !failed.load(std::sync::atomic::Ordering::Acquire) {
							match *progress_callback {
								Some(ref progress_callback) => {
									let progress = i.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
									(progress_callback)((progress as f64) / (total_entries as f64));
								},
								None => {}
							}
						}

						Ok(())

					})() {
						failed.store(true, std::sync::atomic::Ordering::Release);
					}
				}));

			}
		} else {
			let mut handle = self.handle().map_err(|_| GMAReadError::IOError)?;

			let mut i: usize = 0;
			for entry in entries.iter() {
				let mut buf = vec![0; entry.size as usize];
				handle.read_exact(&mut buf).unwrap();

				let fs_path = extract_to.join(&entry.path);
				match fs::write(fs_path, buf) {
					Ok(_) => {},
					Err(_) => { return Err(GMAReadError::IOError) }
				}

				match *progress_callback {
					Some(ref progress_callback) => (progress_callback)((i as f64) / (total_entries as f64)),
					None => {}
				}

				i = i + 1;
			}
		}

		if let Some(threads) = threads {
			for thread in threads {
				match thread.join() {
					Ok(_) => {},
					Err(_) => return Err(GMAReadError::IOError)
				}
			}
		}

		match *progress_callback {
			Some(ref progress_callback) => (progress_callback)(1.),
			None => {}
		}

		Ok(extract_to)
	}

	pub fn extract_entry(&self, entry_path: String, to: ExtractDestination) -> Result<PathBuf, GMAReadError> {
		let extract_to = to.resolve(self).join(PathBuf::from(entry_path.clone()));
		
		fs::create_dir_all(extract_to.with_file_name("")).map_err(|_| GMAReadError::IOError)?;
		let mut handle_w = File::create(&extract_to).map_err(|_| GMAReadError::IOError)?;

		let entry = self.entries.as_ref().expect("Expected entries to be read at this point")
			.get(
				*self.entries_map.as_ref().unwrap()
				.get(&entry_path).ok_or(GMAReadError::EntryNotFound)?
			).ok_or(GMAReadError::EntryNotFound)?;

		let mut handle = self.handle().map_err(|_| GMAReadError::IOError)?;
		handle.seek(SeekFrom::Start(self.entries_start.unwrap() + entry.index)).unwrap();

		let mut buf = vec![0; entry.size as usize];
		handle.read_exact(&mut buf).unwrap();
		handle_w.write(&buf).map_err(|_| GMAReadError::IOError)?;

		drop(buf);
		handle_w.flush().map_err(|_| GMAReadError::IOError)?;

		Ok(extract_to)
	}

	pub fn extracted_name(&self) -> String {
		let mut dir_name = String::new();
		let mut underscored = false;
		for char in self.metadata.as_ref().expect("Expected GMA metadata to be read at this point").name.chars() {
			if char.is_alphanumeric() {
				underscored = false;
				dir_name.push_str(&char.to_lowercase().to_string());
			} else {
				if !underscored {
					underscored = true;
					dir_name.push('_');
				}
			}
		}
		if let Some(id) = self.id {
			if !underscored {
				dir_name.push('_');
			}
			dir_name.push_str(&id.0.to_string());
		}
		dir_name
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
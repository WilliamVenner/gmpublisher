use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use sysinfo::SystemExt;
use std::{fmt::Display, fs::{self, File}, io::{self, BufRead, BufReader, Read}, path::PathBuf, time::SystemTime};

use super::{AddonJson, GMAEntry, GMAFile, GMA_HEADER, ProgressCallback, SUPPORTED_GMA_VERSION};

#[derive(Debug, Clone)]
pub enum ExtractDestination {
	Memory,
	Temp,
	/// path/to/addon/*
	Directory(PathBuf),
	/// path/to/addon/addon_name_123456790/*
	NamedDirectory(PathBuf),
}

#[derive(Debug, Clone, Serialize)]
pub enum GMAReadError {
	IOError,
	InvalidHeader,
	UnsupportedVersion,
	OutOfMemory,
	FormatError
}
impl Display for GMAReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use GMAReadError::*;
		match self {
			IOError => write!(f, "An error occured opening or writing to a file. Make sure you have the appropriate permissions for this file and it is not in use by another process."),
			InvalidHeader => write!(f, "This doesn't appear to be a valid GMA file."),
			UnsupportedVersion => write!(f, "This GMA file uses an unsupported version of the format - sorry!"),
			OutOfMemory => write!(f, "This GMA file is too large to be loaded into memory. Please extract it instead."),
			FormatError => write!(f, "This GMA file appears to be corrupted or of an unrecognised format or version of the format.")
		}
    }
}

fn safe_read<D, E>(res: Result<D, E>) -> Result<D, GMAReadError>
where
	D: std::any::Any,
	E: std::error::Error
{
	match res {
		Ok(data) => Ok(data),
		Err(_) => Err(GMAReadError::FormatError)
	}
}

fn read_nt_string<R: Read + BufRead>(handle: &mut R) -> Result<String, GMAReadError> {
	let mut buf = Vec::new();
	safe_read(handle.read_until(0, &mut buf))?;

	let nt_string = &buf[0..buf.len() - 1];

	Ok(match std::str::from_utf8(nt_string) {
		Ok(str) => str.to_owned(),
		Err(_) => {
			// Some file paths aren't UTF-8 encoded, usually due to Windows NTFS
			// This will simply guess the text encoding and decode it with that instead
			let mut decoder = chardetng::EncodingDetector::new();
			decoder.feed(nt_string, true);
			let encoding = decoder.guess(None, false);
			let (str, _, _) = encoding.decode(nt_string);
			str.to_string()
		}
	})
}

fn skip_nt_string<R: Read + BufRead>(handle: &mut R) -> Result<usize, GMAReadError> {
	let mut buf = Vec::new();
	safe_read(handle.read_until(0, &mut buf))
}

fn read(path: &PathBuf) -> Result<(GMAFile, BufReader<File>), GMAReadError> {
	let mut handle = BufReader::new(match File::open(path) {
		Ok(handle) => handle,
		Err(_) => return Err(GMAReadError::IOError)
	});

	let size = match path.metadata() {
		Ok(metadata) => metadata.len(),
		Err(_) => 0
	};

	let mut magic_buf = [0; 4];
	match handle.read_exact(&mut magic_buf) {
		Ok(_) => {
			if &magic_buf != GMA_HEADER {
				return Err(GMAReadError::InvalidHeader);
			}
		},
		Err(_) => return Err(GMAReadError::InvalidHeader)
	};

	let fmt_version = safe_read(handle.read_u8())?;
	if fmt_version != SUPPORTED_GMA_VERSION { return Err(GMAReadError::UnsupportedVersion); }

	safe_read(handle.read_u64::<LittleEndian>())?; // steamid [unused]
	safe_read(handle.read_u64::<LittleEndian>())?; // timestamp

	skip_nt_string(&mut handle)?; // https://github.com/Facepunch/gmad/blob/master/src/create_gmad.cpp#L74

	let name = read_nt_string(&mut handle)?;
	let mut tags = None;
	let mut ignore = None;
	let mut addon_type = None;

	let addon_json = read_nt_string(&mut handle)?;
	match serde_json::de::from_str(&addon_json) {
		Ok(addon_json) => {
			let addon_json: AddonJson = addon_json;
			ignore = addon_json.ignore;
			tags = Some(addon_json.tags);
			addon_type = Some(addon_json.addon_type);
		},
		Err(_) => {}
	};

	skip_nt_string(&mut handle)?; // author [unused]
	safe_read(handle.read_u32::<LittleEndian>())?; // addon version [unused]

	Ok((
		GMAFile {
			id: None,
			
			addon_type,

			#[cfg(target_os = "windows")]
			path: Some(
				match dunce::canonicalize(path) {
					Ok(path) => path,
					Err(_) => path.to_owned()
				}
			),
			
			#[cfg(not(target_os = "windows"))]
			path: Some(path.canonicalize().unwrap_or_else(|_| path.to_owned())),

			size,
			
			name,
			tags,
			ignore,
		
			entries: None,
			entries_read: false,
		},
		handle
	))
}

pub fn metadata(path: &PathBuf) -> Result<(GMAFile, BufReader<File>), GMAReadError> {
	Ok(read(path)?)
}

pub fn entries<'a>(gma: &'a mut GMAFile, handle: &'a mut BufReader<File>) -> Result<&'a Vec<GMAEntry>, GMAReadError> {
	if let None = gma.entries {
		let mut entries = Vec::new();
		while safe_read(handle.read_u32::<LittleEndian>())? != 0 {
			entries.push(GMAEntry {
				path: PathBuf::from(read_nt_string(handle)?),
				size: safe_read(handle.read_i64::<LittleEndian>())? as u64,
				crc: safe_read(handle.read_u32::<LittleEndian>())?,
				data: None
			});
		}
		gma.entries = Some(entries);
	}
	Ok(gma.entries.as_ref().unwrap())
}

pub struct ExtractableGMA {
	gma: Result<GMAFile, GMAReadError>,
	handle: Option<BufReader<File>>,
	path: PathBuf,
}
impl From<PathBuf> for ExtractableGMA {
    fn from(path: PathBuf) -> Self {
		match read(&path) {
			Ok((gma, handle)) => ExtractableGMA {
				gma: Ok(gma),
				handle: Some(handle),
				path,
			},
			Err(err) => ExtractableGMA {
				gma: Err(err),
				handle: None,
				path
			}
		}
    }
}
impl From<(GMAFile, BufReader<File>)> for ExtractableGMA {
    fn from((gma, handle): (GMAFile, BufReader<File>)) -> Self {
        ExtractableGMA {
			handle: Some(handle),
			path: gma.path.as_ref().unwrap().clone(),
			gma: Ok(gma),
		}
    }
}

pub fn extract<E: Into<ExtractableGMA>>(extractable: E, to: ExtractDestination, progress_callback: Option<ProgressCallback>) -> Result<GMAFile, GMAReadError> {
	use ExtractDestination::*;

	let extractable: ExtractableGMA = extractable.into();
	let mut gma = extractable.gma?;
	let mut handle = extractable.handle.unwrap();
	let path = extractable.path;

	let mut extract_to = match to {
		Memory => PathBuf::from(""),
		Directory(ref extract_to) => extract_to.clone(),
		_ => {
			let dir_name = gma.name.clone().replace(|char: char| !char.is_alphanumeric(), "_");

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

	let mut entries = entries(&mut gma, &mut handle)?.clone();
	let total_entries = entries.len();
	let mut i: usize = 0;

	// If we're trying to extract this GMA to memory, and there isn't enough, throw an error
	if let Memory = to {
		if available_memory < gma.size {
			return Err(GMAReadError::OutOfMemory);
		}
	}

	// We should only multithread if we have enough available RAM to actually store the GMA entries in memory
	let mut threads: Option<Vec<std::thread::JoinHandle<()>>> = if available_memory > gma.size {
		Some(Vec::with_capacity(total_entries))
	} else { None };

	if !gma.entries_read {
		// Read file contents
		for mut entry in &mut entries {
			let mut buf = vec![0; entry.size as usize];
			handle.read_exact(&mut buf).unwrap();

			match to {
				Memory => entry.data = Some(buf),
				_ => {
					extract_to.push(&entry.path);
					let fs_path = extract_to.clone();
					extract_to.pop();

					match threads {
						Some(ref mut threads) => {
							threads.push(std::thread::spawn(move || {
								fs::write(fs_path, buf).unwrap();
							}));
						},
						None => {
							match fs::write(fs_path, buf) {
								Ok(_) => {},
								Err(_) => { return Err(GMAReadError::IOError) }
							}
						}
					};
				},
			}

			match progress_callback {
				Some(ref progress_callback) => (progress_callback)(((i as f32) / (total_entries as f32)) * 100.),
				None => {}
			}

			i = i + 1;
		}
	} else if let Memory = to {} else {
		// Write file contents from memory
		for entry in entries {
			extract_to.push(&entry.path);
			let fs_path = extract_to.clone();
			extract_to.pop();

			match threads {
				Some(ref mut threads) => {
					threads.push(std::thread::spawn(move || {
						fs::write(fs_path, entry.data.unwrap()).unwrap();
					}));
				},
				None => {
					match fs::write(fs_path, entry.data.unwrap()) {
						Ok(_) => {},
						Err(_) => { return Err(GMAReadError::IOError) }
					}
				}
			};

			match progress_callback {
				Some(ref progress_callback) => (progress_callback)(((i as f32) / (total_entries as f32)) * 100.),
				None => {}
			}

			i = i + 1;
		}
	}

	// Apparently some gma just completely omit the addon CRC from the end
	// Hence, we shouldn't unwrap the following since it may fail
	handle.read_u32::<LittleEndian>().ok(); // crc [unused]
	
	#[cfg(debug_assertions)]
	{
		let remaining = io::copy(&mut handle, &mut io::sink()).unwrap();
		if remaining != 0 {
			eprintln!("Warning: GMA file had {} bytes of extra _after_ the entries (total: {})\nFrom {:?}", remaining, total_entries, path);
		}
	}

	match progress_callback {
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

	gma.entries_read = true;
	Ok(gma)
}
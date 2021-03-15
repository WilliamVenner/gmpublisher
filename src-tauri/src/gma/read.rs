use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use std::{fs::{self, File}, io::{self, BufRead, BufReader, Read}, path::PathBuf, time::SystemTime};

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
	UnsupportedVersion
}

fn read_nt_string<R: Read + BufRead>(handle: &mut R) -> String {
	let mut buf = Vec::new();
	handle.read_until(0, &mut buf).unwrap();

	let nt_string = &buf[0..buf.len() - 1];

	match std::str::from_utf8(nt_string) {
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
	}
}

fn skip_nt_string<R: Read + BufRead>(handle: &mut R) {
	let mut buf = Vec::new();
	handle.read_until(0, &mut buf).unwrap();
}

fn read(path: &PathBuf) -> Result<(GMAFile, BufReader<File>), GMAReadError> {
	let handle = BufReader::new(match File::open(path) {
		Ok(handle) => handle,
		Err(_) => return Err(GMAReadError::IOError)
	});

	let size = match path.metadata() {
		Ok(metadata) => metadata.len(),
		Err(_) => 0
	};

	let mut magic_buf = [0; 4];
	handle.read_exact(&mut magic_buf).unwrap();
	if &magic_buf != GMA_HEADER { return Err(GMAReadError::InvalidHeader); }

	let fmt_version = handle.read_u8().unwrap();
	if fmt_version != SUPPORTED_GMA_VERSION { return Err(GMAReadError::UnsupportedVersion); }

	handle.read_u64::<LittleEndian>(); // steamid [unused]
	handle.read_u64::<LittleEndian>(); // timestamp

	skip_nt_string(&mut handle); // https://github.com/Facepunch/gmad/blob/master/src/create_gmad.cpp#L74

	let title = read_nt_string(&mut handle);

	let mut name = None;
	let mut description = None;
	let mut tags = None;
	let mut ignore = None;
	let mut addon_type = None;
	match serde_json::de::from_str(&read_nt_string(&mut handle)) {
		Ok(addon_json) => {
			let addon_json: AddonJson = addon_json;
			name = Some(addon_json.title);
			//description = addon_json.;
			tags = Some(addon_json.tags);
			ignore = Some(addon_json.ignore);
			addon_type = Some(addon_json.addon_type);
		},
		Err(_) => {}
	};

	skip_nt_string(&mut handle); // author [unused]
	skip_nt_string(&mut handle); // addon version [unused]

	Ok((
		GMAFile {
			id: None,
			
			addon_type,
			path: Some(path.to_owned()),
			size,
			
			name,
			description,
			tags,
			ignore,
		
			entries: None,
		},
		handle
	))
}

pub fn metadata(path: &PathBuf) -> Result<GMAFile, GMAReadError> {
	Ok(read(path)?.0)
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
			gma: Ok(gma),
			handle: Some(handle),
			path: gma.path.unwrap(),
		}
    }
}

pub fn extract<E: Into<ExtractableGMA>>(extractable: E, to: ExtractDestination, progress_callback: Option<ProgressCallback>) -> Result<GMAFile, GMAReadError> {
	use ExtractDestination::*;

	let extractable: ExtractableGMA = extractable.into();
	let gma = extractable.gma?;
	let handle = extractable.handle.unwrap();
	let path = extractable.path;

	let mut extract_to = match to {
		Memory => PathBuf::from(""),
		Directory(extract_to) => extract_to,
		_ => {
			let dir_name = path.file_name()
			.and_then(|f| Some(f.to_string_lossy().to_string())).unwrap_or_else(|| gma.name
				.unwrap_or_else(|| format!("extracted_gma_{}", SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or(std::time::Duration::from_secs(0)).as_secs()))
			);

			match to {
				NamedDirectory(mut extract_to) => {
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

	let mut entries = Vec::new();
	while handle.read_u32::<LittleEndian>().unwrap() != 0 {
		entries.push(GMAEntry {
			path: PathBuf::from(read_nt_string(&mut handle)),
			size: handle.read_i64::<LittleEndian>().unwrap() as u64,
			crc: handle.read_u32::<LittleEndian>().unwrap(),
			data: None
		});
	}

	// Read file contents
	let total_entries = entries.len();
	let mut i: usize = 0;
	let mut threads = Vec::with_capacity(total_entries);
	for mut entry in &mut entries {
		let mut buf = vec![0; entry.size as usize];
		handle.read_exact(&mut buf).unwrap();

		match to {
			Memory => entry.data = Some(buf),
			_ => {
				extract_to.push(entry.path);
				let fs_path = extract_to.clone();
				threads.push(std::thread::spawn(move || {
					fs::write(fs_path, buf);
				}));
			},
		}

		match progress_callback {
			Some(ref progress_callback) => (progress_callback)(((i as f32) / (total_entries as f32)) * 100.),
			None => {}
		}

		i = i + 1;
	}

	// Apparently some gma just completely omit the addon CRC from the end
	// Hence, we shouldn't unwrap the following since it may fail
	handle.read_u32::<LittleEndian>(); // crc [unused]
	
	#[cfg(debug_assertions)]
	{
		let remaining = io::copy(&mut handle, &mut io::sink()).unwrap();
		if remaining != 0 {
			eprintln!("Warning: GMA file had {} bytes of extra _after_ the entries (total: {})\nFrom {:?}", remaining, entries.len(), path);
		}
	}

	match progress_callback {
		Some(ref progress_callback) => (progress_callback)(100.),
		None => {}
	}

	match to {
		Memory => gma.entries = Some(entries),
		_ => for thread in threads { thread.join().unwrap(); }
	}

	Ok(gma)
}
use super::{GMAFile, GMAEntry, SUPPORTED_GMA_VERSION, GMA_HEADER};
use byteorder::{LittleEndian, ReadBytesExt};
use std::io;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum GMAReadError {
	InvalidHeader,
	UnsupportedVersion
}

fn read_nt_string<R: Read + BufRead>(handle: &mut R) -> String {
	let mut buf = Vec::new();
	handle.read_until(0, &mut buf).unwrap();

	// don't include null byte
	return std::str::from_utf8(&buf[0..buf.len() - 1])
		.unwrap()
		.to_owned();
}

pub fn read_gma(mut handle: BufReader<File>, read_entry: bool) -> Result<GMAFile, GMAReadError> {
	let mut magic_buf = [0; 4];
	handle.read_exact(&mut magic_buf).unwrap();

	if &magic_buf != GMA_HEADER {
		return Err(GMAReadError::InvalidHeader);
	}

	let fmt_version = handle.read_u8().unwrap();
	if fmt_version != SUPPORTED_GMA_VERSION {
		return Err(GMAReadError::UnsupportedVersion);
	}

	let _steamid = handle.read_u64::<LittleEndian>().unwrap();
	let _timestamp = handle.read_u64::<LittleEndian>().unwrap();

	let mut dumb_string = read_nt_string(&mut handle);
	while dumb_string.len() > 0 {
		dumb_string = read_nt_string(&mut handle);
	}

	let name = read_nt_string(&mut handle);
	let desc = read_nt_string(&mut handle);
	let author = read_nt_string(&mut handle);

	let _addon_version = handle.read_u32::<LittleEndian>().unwrap();

	let mut entries = vec!();

	while handle.read_u32::<LittleEndian>().unwrap() != 0 {
		let entry_name = read_nt_string(&mut handle);
		let entry_size = handle.read_i64::<LittleEndian>().unwrap();
		let entry_crc = handle.read_u32::<LittleEndian>().unwrap();

		let entry = GMAEntry {
			name: entry_name,
			size: entry_size as u64,
			crc: entry_crc,
			contents: None
		};
		entries.push(entry);
	}

	// Read file contents
	for mut e in &mut entries {
		if read_entry {
			let mut buf = vec![0; e.size as usize];
			handle.read_exact(&mut buf).unwrap();
			e.contents = Some(buf);
		} else {
			// Pipe to sink
			let mut_handle = &mut handle;
			io::copy(&mut mut_handle.take(e.size), &mut io::sink()).unwrap();
		}
	}

	// Apparently some gma just completely omit the addon CRC from the end
	// Hence, we shouldn't unwrap the following since it may fail
	let _addon_crc = handle.read_u32::<LittleEndian>();

	let remaining = io::copy(&mut handle, &mut io::sink()).unwrap();
	if remaining != 0 {
		eprintln!("Warning: GMA file had {} bytes of extra _after_ the entries", remaining);
	}

	Ok(GMAFile {
		name: name,
		description: desc,
		author: author,
		entries: entries
	})
}
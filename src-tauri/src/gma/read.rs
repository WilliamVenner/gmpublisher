use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use std::{collections::HashMap, fmt::Display, io::{Read, Seek, SeekFrom}, sync::Arc};
use thiserror::Error;

use super::{AddonJson, GMAEntry, GMAFile, GMAFileHandle, GMAMetadata};

macro_rules! safe_read {
    ( $x:expr ) => {
		match $x {
			Ok(data) => Ok(data),
			Err(_) => Err(GMAReadError::FormatError)
		}
    };
}

#[derive(Debug, Clone, Serialize, Error)]
pub enum GMAReadError {
	IOError,
	InvalidHeader,
	UnsupportedVersion,
	FormatError,
	EntryNotFound
}
impl Display for GMAReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use GMAReadError::*;
		match self {
			IOError => write!(f, "An error occured opening or writing to a file. Make sure you have the appropriate permissions for this file and it is not in use by another process."),
			InvalidHeader => write!(f, "This doesn't appear to be a valid GMA file."),
			UnsupportedVersion => write!(f, "This GMA file uses an unsupported version of the format - sorry!"),
			FormatError => write!(f, "This GMA file appears to be corrupted or of an unrecognised format or version of the format."),
			EntryNotFound => write!(f, "The entry could not be found in this GMA file.")
		}
    }
}

impl GMAFile {
	fn update_extractable_name(&mut self) {
		if self.extracted_name.1.is_none() || self.extracted_name.0 != self.id.is_some() {
			self.extracted_name = (self.id.is_some(), Some(self.extracted_name()));
		}
	}

	pub fn metadata(&mut self) -> Result<&GMAMetadata, GMAReadError> {
		if let None = self.metadata {

			let mut handle = self.handle().map_err(|_| GMAReadError::IOError)?;

			handle.seek(SeekFrom::Start(self.metadata_start)).unwrap();

			safe_read!(handle.read_u64::<LittleEndian>())?; // steamid [unused]
			safe_read!(handle.read_u64::<LittleEndian>())?; // timestamp

			handle.skip_nt_string()?; // https://github.com/Facepunch/gmad/blob/master/src/create_gmad.cpp#L74

			let name = handle.read_nt_string()?;
			let mut tags = None;
			let mut ignore = None;
			let mut addon_type = None;

			let addon_json = handle.read_nt_string()?;
			match serde_json::de::from_str(&addon_json) {
				Ok(addon_json) => {
					let addon_json: AddonJson = addon_json;
					ignore = addon_json.ignore;
					tags = Some(addon_json.tags);
					addon_type = Some(addon_json.addon_type);
				},
				Err(_) => {}
			};

			handle.skip_nt_string()?; // author [unused]
			safe_read!(handle.read_u32::<LittleEndian>())?; // addon version [unused]

			self.metadata = Some(GMAMetadata {
				name,
				tags,
				ignore,
				addon_type,
			});

			self.entries_list_start = Some(handle.seek(SeekFrom::Current(0)).unwrap());

			self.update_extractable_name();

		}

		Ok(&self.metadata.as_ref().unwrap())
	}

	pub fn entries(&mut self) -> Result<(Arc<Vec<GMAEntry>>, Arc<HashMap<String, usize>>), GMAReadError> {
		if let None = self.entries {

			if let None = self.entries_list_start { self.metadata().unwrap(); }

			let mut handle = self.handle().map_err(|_| GMAReadError::IOError)?;
			handle.seek(SeekFrom::Start(self.entries_list_start.unwrap())).unwrap();

			let mut entries = Vec::new();
			let mut entries_map = HashMap::new();
			let mut entry_cursor = 0;

			while safe_read!(handle.read_u32::<LittleEndian>())? != 0 {
				let path = handle.read_nt_string()?;
				let size = safe_read!(handle.read_i64::<LittleEndian>())? as u64;
				let crc = safe_read!(handle.read_u32::<LittleEndian>())?;

				let entry = GMAEntry {
					path: path.clone(),
					size,
					crc,
					index: entry_cursor
				};

				entry_cursor = entry_cursor + size;

				entries.push(entry);
				entries_map.insert(path, entries.len()-1);
			}

			self.entries = Some(Arc::new(entries));
			self.entries_map = Some(Arc::new(entries_map)); // TODO does this need to be Arc?
			self.entries_start = Some(handle.seek(SeekFrom::Current(0)).unwrap());

		}

		Ok((self.entries.as_ref().unwrap().clone(), self.entries_map.as_ref().unwrap().clone()))
	}

	pub fn read_entry(&self, path: String) -> Option<Vec<u8>> {
		let entry = self.entries.as_ref().unwrap().get(*self.entries_map.as_ref().unwrap().get(&path)?)?;

		let mut handle = self.handle().ok()?;

		handle.seek(SeekFrom::Start(entry.index)).unwrap();

		let mut buf = vec![0; entry.size as usize];
		handle.read_exact(&mut buf).unwrap();

		Some(buf)
	}
}

impl GMAEntry {
	pub fn read(&self, mut handle: GMAFileHandle) -> Option<Vec<u8>> {
		handle.seek(SeekFrom::Start(self.index)).unwrap();

		let mut buf = vec![0; self.size as usize];
		handle.read_exact(&mut buf).unwrap();

		Some(buf)
	}
}
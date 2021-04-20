use std::{collections::HashMap, fs::File, io::{BufReader, Cursor, SeekFrom}};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{ArcBytes, NTStringReader};

use super::{GMAEntriesMap, GMAEntry, GMAError, GMAFile, GMAMetadata};

macro_rules! safe_read {
	( $x:expr ) => {
		$x.map_err(|_| GMAError::FormatError)
	};
}

pub enum GMAReader {
	MemBuffer(Cursor<ArcBytes>),
	Disk(BufReader<File>)
}
impl std::ops::Deref for GMAReader {
    type Target = dyn NTStringReader;

    fn deref(&self) -> &Self::Target {
        match self {
			Self::MemBuffer(buf) => buf,
			Self::Disk(buf) => buf
		}
    }
}
impl std::ops::DerefMut for GMAReader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
			Self::MemBuffer(buf) => buf,
			Self::Disk(buf) => buf
		}
    }
}
impl NTStringReader for Cursor<ArcBytes> {}
impl NTStringReader for BufReader<File> {}

impl GMAFile {
	pub fn read(&self) -> Result<GMAReader, GMAError> {
		if let Some(ref membuffer) = self.membuffer {
			Ok(GMAReader::MemBuffer(Cursor::new(membuffer.clone())))
		} else {
			Ok(GMAReader::Disk(BufReader::new(File::open(&self.path)?)))
		}
	}

	pub fn metadata(&mut self) -> Result<Option<GMAReader>, GMAError> {
		main_thread_forbidden!();

		if self.metadata.is_some() {
			Ok(None)
		} else {
			let mut handle = self.read()?;
			handle.seek(SeekFrom::Start(self.pointers.metadata))?;

			safe_read!(handle.read_u64::<LittleEndian>())?; // steamid [unused]
			safe_read!(handle.read_u64::<LittleEndian>())?; // timestamp

			if self.version > 1 {
				// required content [unused]
				safe_read!(handle.skip_nt_string())?;
			}

			let embedded_title = safe_read!(handle.read_nt_string())?;
			let embedded_description = safe_read!(handle.read_nt_string())?;

			self.metadata = Some(match serde_json::de::from_str::<GMAMetadata>(&embedded_description) {
				Ok(mut metadata) => {
					match &mut metadata {
						GMAMetadata::Standard { title, .. } => *title = embedded_title,
						GMAMetadata::Legacy { title, description } => {
							*title = embedded_title;
							*description = embedded_description;
						}
					}
					metadata
				}
				Err(_) => GMAMetadata::Legacy {
					title: embedded_title,
					description: embedded_description,
				},
			});

			safe_read!(handle.skip_nt_string())?; // author [unused]
			safe_read!(handle.read_i32::<LittleEndian>())?; // addon version [unused]

			self.pointers.entries_list = handle.seek(SeekFrom::Current(0))?;

			self.compute_extracted_name();

			Ok(Some(handle))
		}
	}

	// https://steamcommunity.com/sharedfiles/filedetails/?id=1727993520

	pub fn entries(&mut self) -> Result<Option<GMAReader>, GMAError> {
		main_thread_forbidden!();

		if self.entries.is_some() {
			Ok(None)
		} else {
			let mut handle = match self.metadata()? {
				Some(handle) => handle,
				None => self.read()?,
			};
			handle.seek(SeekFrom::Start(self.pointers.entries_list))?;

			let mut entries = GMAEntriesMap(HashMap::new());
			let mut entry_cursor = 0;

			while handle.read_u32::<LittleEndian>()? != 0 {
				let path = handle.read_nt_string()?;
				let size = handle.read_i64::<LittleEndian>()? as u64;
				let crc = handle.read_u32::<LittleEndian>()?;

				let entry = GMAEntry {
					path: path.clone(),
					size,
					crc,
					index: entry_cursor,
				};

				entry_cursor = match entry_cursor.checked_add(size) {
					None => return Err(GMAError::FormatError),
					Some(entry_cursor) => entry_cursor,
				};

				entries.insert(path, entry);
			}

			self.entries = Some(entries);
			self.pointers.entries = handle.seek(SeekFrom::Current(0))?;

			Ok(Some(handle))
		}
	}
}

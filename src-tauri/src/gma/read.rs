use std::{collections::HashMap, fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}};

use byteorder::{LittleEndian, ReadBytesExt};

use super::{GMAEntriesMap, GMAEntry, GMAError, GMAFile, GMAMetadata, LegacyGMAMetadata, StandardGMAMetadata};

macro_rules! safe_read {
	( $x:expr ) => {
		$x.map_err(|_| GMAError::FormatError)
	};
}
pub struct GMAReadHandle<R: Read + Seek> {
	pub inner: BufReader<R>,
}
impl<R: Read + Seek> GMAReadHandle<R> {
	pub fn read_nt_string(&mut self) -> Result<String, GMAError> {
		let mut buf = vec![];
		let bytes_read = safe_read!(self.read_until(0, &mut buf))?;
		let nt_string = &buf[0..bytes_read - 1];

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
	
	pub fn skip_nt_string(&mut self) -> Result<usize, GMAError> {
		let mut buf = vec![];
		safe_read!(self.read_until(0, &mut buf))
	}
}
impl<R: Read + Seek> std::ops::Deref for GMAReadHandle<R> {
    type Target = BufReader<R>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<R: Read + Seek> std::ops::DerefMut for GMAReadHandle<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl GMAFile {
	pub fn metadata(&mut self) -> Result<Option<GMAReadHandle<File>>, GMAError> {
		main_thread_forbidden!();

		if let Some(ref metadata) = self.metadata {
			Ok(None)
		} else {
			let mut handle = self.read()?;
			handle.seek(SeekFrom::Start(self.pointers.metadata))?;

			handle.read_u64::<LittleEndian>()?; // steamid [unused]
			handle.read_u64::<LittleEndian>()?; // timestamp

			if self.version > 1 {
				// required content [unused]
				handle.skip_nt_string()?;
			}
			
			let title = handle.read_nt_string()?;
			let description = handle.read_nt_string()?;

			self.metadata = Some(match serde_json::de::from_str::<StandardGMAMetadata>(&description) {
				Ok(addon_json) => GMAMetadata::Standard(addon_json),
				Err(_) => {
					GMAMetadata::Legacy(LegacyGMAMetadata {
						title,
						description
					})
				}
			});

			handle.skip_nt_string()?; // author [unused]
			handle.read_i32::<LittleEndian>()?; // addon version [unused]

			self.pointers.entries_list = handle.seek(SeekFrom::Current(0))?;

			self.compute_extracted_name();

			Ok(Some(handle))
		}
	}

	pub fn entries(&mut self) -> Result<Option<GMAReadHandle<File>>, GMAError> {
		main_thread_forbidden!();
		
		if let Some(ref entries) = self.entries {
			Ok(None)
		} else {
			let mut handle = match self.metadata()? {
				Some(handle) => handle,
				None => self.read()?
			};
			handle.seek(SeekFrom::Start(self.pointers.entries_list))?;

			let mut entries = GMAEntriesMap { inner: HashMap::new() };
			let mut entry_cursor = 0;
			
			while handle.read_u32::<LittleEndian>()? != 0 {
				let path = handle.read_nt_string()?;
				let size = handle.read_i64::<LittleEndian>()? as u64;
				let crc = handle.read_u32::<LittleEndian>()?;

				let entry = GMAEntry {
					path: path.clone(),
					size,
					crc,
					index: entry_cursor
				};

				entry_cursor = entry_cursor + size;

				entries.insert(path, entry);
			}

			self.entries = Some(entries);
			self.pointers.entries = handle.seek(SeekFrom::Current(0))?;

			Ok(Some(handle))
		}
	}
}
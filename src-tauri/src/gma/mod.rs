use std::{collections::HashMap, fs::File, io::{BufRead, BufReader, Read, Seek, SeekFrom}, path::PathBuf, sync::Arc};
use byteorder::ReadBytesExt;
use serde::{Deserialize, Serialize};
use steamworks::PublishedFileId;

pub const GMA_HEADER: &'static [u8; 4] = b"GMAD";
pub const SUPPORTED_GMA_VERSION: u8 = 3;

#[macro_use]
pub mod read;
pub use read::*;

pub mod write;
pub use write::*;

pub mod extract;
pub use extract::*;

use crate::{util::path::NormalizedPathBuf, workshop::WorkshopItem};

pub type ProgressCallback = Box<dyn Fn(f64) -> () + Sync + Send>;

fn serialize_extracted_name<S>(data: &(bool, Option<String>), s: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer
{
	s.serialize_str(&data.1.as_ref().expect("Missing extracted name! Make sure to call metadata()"))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GMAFile {
	pub id: Option<PublishedFileId>,

	pub path: NormalizedPathBuf,
	pub size: u64,

	#[serde(flatten)]
	pub(crate) metadata: Option<GMAMetadata>,
	#[serde(skip)]
	pub(crate) metadata_start: u64,
	
	pub(crate) entries: Option<Arc<Vec<GMAEntry>>>,
	pub(crate) entries_map: Option<Arc<HashMap<String, usize>>>,
	#[serde(skip)]
	pub(crate) entries_list_start: Option<u64>,
	#[serde(skip)]
	pub(crate) entries_start: Option<u64>,

	#[serde(serialize_with = "serialize_extracted_name")]
	#[serde(skip_deserializing)]
	pub(crate) extracted_name: (bool, Option<String>)
}
impl GMAFile {
	pub fn new(path: &PathBuf, id: Option<PublishedFileId>) -> Result<GMAFile, GMAReadError> {
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
	
		let fmt_version = safe_read!(handle.read_u8())?;
		if fmt_version != SUPPORTED_GMA_VERSION { return Err(GMAReadError::UnsupportedVersion); }

		Ok(GMAFile {
			path: path.into(),
			size,

			id,

			metadata_start: handle.seek(SeekFrom::Current(0)).unwrap(),
			metadata: None,

			entries: None,
			entries_map: None,
			entries_start: None,
			entries_list_start: None,

			extracted_name: (id.is_some(), None),
		})
	}

	pub fn handle(&self) -> Result<GMAFileHandle, std::io::Error> {
		Ok(BufReader::new(File::open(&*self.path)?).into())
	}
}

pub struct GMAFileHandle{
	inner: Option<BufReader<File>>,
}
impl Clone for GMAFileHandle {
	/// # This doesn't actually clone it and is merely a hack to derive Clone in GMAFile
    fn clone(&self) -> Self {
        GMAFileHandle::default()
    }
}
impl From<BufReader<File>> for GMAFileHandle {
    fn from(handle: BufReader<File>) -> Self {
		GMAFileHandle { inner: Some(handle)  }
    }
}
impl Default for GMAFileHandle {
    fn default() -> Self {
		GMAFileHandle { inner: None }
    }
}
impl std::ops::Deref for GMAFileHandle {
    type Target = BufReader<File>;
    fn deref(&self) -> &Self::Target {
		debug_assert!(self.inner.is_some(), "Tried to use an invalid GMA file handle!");
		self.inner.as_ref().unwrap()
	}
}
impl std::ops::DerefMut for GMAFileHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
		debug_assert!(self.inner.is_some(), "Tried to use an invalid GMA file handle!");
		self.inner.as_mut().unwrap()
	}
}
impl GMAFileHandle {
	pub(crate) fn is_open(&self) -> bool {
		self.inner.is_some()
	}

	pub(crate) fn read_nt_string(&mut self) -> Result<String, GMAReadError> {
		let mut buf = Vec::new();
		safe_read!(self.read_until(0, &mut buf))?;

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

	pub(crate) fn skip_nt_string(&mut self) -> Result<usize, GMAReadError> {
		let mut buf = Vec::new();
		safe_read!(self.read_until(0, &mut buf))
	}
}
impl std::fmt::Debug for GMAFileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.is_open() {
			true => "Open",
			false => "Closed"
		})
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GMAMetadata {
	pub name: String,
	#[serde(rename = "type")]
	pub addon_type: Option<String>,
	pub tags: Option<Vec<String>>,
	pub ignore: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GMAEntry {
	#[serde(skip)]
	pub(crate) index: u64,
	pub path: String,
	pub size: u64,
	pub crc: u32,
}
impl std::fmt::Debug for GMAEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GMAEntry")
			.field("path", &self.path)
			.field("size", &self.size)
			.field("crc", &self.crc)
			.finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddonJson {
	pub description: String,
	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
	pub ignore: Option<Vec<String>>
}
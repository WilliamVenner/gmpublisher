use std::{collections::HashMap, fmt::Display, fs::File, io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write}, path::PathBuf, time::SystemTime};

use byteorder::ReadBytesExt;
use steamworks::PublishedFileId;
use thiserror::Error;
use serde::{Serialize, Deserialize};

use crate::main_thread_forbidden;

use self::read::GMAReadHandle;

const GMA_HEADER: &'static [u8; 4] = b"GMAD";

#[derive(Debug, Clone, Serialize, Error)]
pub enum GMAError {
	IOError,
	InvalidHeader,
	FormatError,
	EntryNotFound,
}
impl Display for GMAError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use GMAError::*;
		match self {
			IOError => write!(
				f,
				"An error occured opening or writing to a file. Make sure you have the appropriate permissions for this file and it is not in use by another process."
			),
			InvalidHeader => write!(f, "This doesn't appear to be a valid GMA file."),
			UnsupportedVersion => write!(f, "This GMA file uses an unsupported version of the format - sorry!"),
			FormatError => write!(f, "This GMA file appears to be corrupted or of an unrecognised format or version of the format."),
			EntryNotFound => write!(f, "The entry could not be found in this GMA file."),
		}
	}
}
impl From<std::io::Error> for GMAError {
    fn from(_: std::io::Error) -> Self {
        Self::IOError
    }
}

#[derive(Debug, Clone, Default)]
pub struct GMAFilePointers {
	metadata: u64,
	entries: u64,
	entries_list: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegacyGMAMetadata {
	pub title: String,
	pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardGMAMetadata {
	#[serde(default)]
	pub title: String,
	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
	pub ignore: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum GMAMetadata {
	Legacy(LegacyGMAMetadata),
	Standard(StandardGMAMetadata)
}

#[derive(Debug, Clone, Serialize)]
pub struct GMAEntry {
	path: String,
	size: u64,
	crc: u32,

	#[serde(skip)]
	index: u64,
}

#[derive(Clone, Debug)]
pub struct GMAEntriesMap {
	inner: HashMap<String, GMAEntry>,
}
impl std::ops::Deref for GMAEntriesMap {
    type Target = HashMap<String, GMAEntry>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for GMAEntriesMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
impl Serialize for GMAEntriesMap {
	fn serialize<S: serde::Serializer>(&self, serialize: S) -> Result<S::Ok, S::Error> {
		serialize.collect_seq(self.inner.keys())
	}
}

#[derive(Debug, Clone, Serialize)]
#[serde(remote = "GMAFile")]
pub struct GMAFile {
	pub path: PathBuf,
	pub size: u64,

	pub id: Option<PublishedFileId>,

	#[serde(flatten)]
	pub metadata: Option<GMAMetadata>,

	pub entries: Option<GMAEntriesMap>,

	#[serde(skip)]
	pub pointers: GMAFilePointers,

	#[serde(skip)]
	pub version: u8,

	#[serde(getter = "GMAFile::extracted_name")]
	extracted_name: String
}

impl GMAFile {
	pub fn open(path: PathBuf) -> Result<GMAFile, GMAError> {
		main_thread_forbidden!();

		let mut f = BufReader::new(File::open(&path)?);

		let mut gma = GMAFile {
			size: path.metadata().and_then(|metadata| Ok(metadata.len())).unwrap_or(0),
			path,
		    id: None,
		    metadata: None,
		    entries: None,
		    pointers: GMAFilePointers::default(),
			version: 0,
			extracted_name: String::new()
		};

		if gma.size == 0 {
			if let Ok(size) = crate::stream_len(&mut f) {
				gma.size = size;
			}
		}

		let mut header_buf = [0; 4];
		f.read_exact(&mut header_buf).map_err(|_| GMAError::InvalidHeader)?;
		if &header_buf != GMA_HEADER {
			return Err(GMAError::InvalidHeader);
		}

		gma.version = f.read_u8()?;

		gma.pointers.metadata = f.seek(SeekFrom::Current(0))?;

		Ok(gma)
	}

	pub fn set_ws_id(&mut self, id: PublishedFileId) {
		self.id = Some(id);
	}

	pub fn extracted_name(&self) -> String {
		let mut extracted_name = String::new();
		let mut underscored = false;

		{
			let name = match self.metadata() {
				Ok(metadata) => match metadata {
					GMAMetadata::Legacy(LegacyGMAMetadata { title, .. }) | GMAMetadata::Standard(StandardGMAMetadata { title, .. }) => title.clone(),
				},
				Err(_) => {
					match self.path.file_name() {
						Some(file_name) => file_name.to_string_lossy().to_string(),
						None => match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
							Ok(unix) => format!("gmpublisher_extracted_{}", unix.as_secs()),
							Err(_) => "gmpublisher_extracted".into()
						}
					}
				},
			};

			extracted_name.reserve(name.len());

			for char in name.chars() {
				if char.is_alphanumeric() {
					underscored = false;
					extracted_name.push(char);
				} else if !underscored {
					underscored = true;
					extracted_name.push('_');
				}
			}
		}

		if let Some(id) = self.id {
			let id_str = id.0.to_string();
			if !underscored {
				extracted_name.reserve(id_str.len() + 1);
				extracted_name.push('_');
				extracted_name.push_str(&id_str);
			} else {
				extracted_name.reserve(id_str.len());
				extracted_name.push_str(&id_str);
			}
		} else if underscored {
			extracted_name.pop();
		}

		extracted_name
	}

	fn extracted_name_serde(&self, _: String) -> String {
		self.extracted_name()
	}

	pub fn read(&self) -> Result<GMAReadHandle<File>, GMAError> {
		Ok(GMAReadHandle { inner: BufReader::new(File::open(&self.path)?) })
	}

	pub fn write(&self) -> Result<GMAWriteHandle<File>, GMAError> {
		Ok(GMAWriteHandle { inner: BufWriter::new(File::create(&self.path)?) })
	}
}

pub struct GMAWriteHandle<W: Write + Seek> {
	inner: BufWriter<W>
}
impl<W: Write + Seek> std::ops::Deref for GMAWriteHandle<W> {
    type Target = BufWriter<W>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<W: Write + Seek> std::ops::DerefMut for GMAWriteHandle<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub mod read;
pub mod write;
pub mod extract;
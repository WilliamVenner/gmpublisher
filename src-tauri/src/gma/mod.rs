use std::{
	collections::HashMap,
	fmt::Display,
	fs::File,
	io::{BufReader, BufWriter, Read, Seek, SeekFrom},
	path::{Path, PathBuf},
	time::SystemTime,
};

use byteorder::ReadBytesExt;

use serde::{Deserialize, Serialize};
use steamworks::PublishedFileId;
use thiserror::Error;

use crate::{main_thread_forbidden, transaction, transactions::Transaction};

use self::{read::GMAReadHandle, write::GMAWriteHandle};

const GMA_HEADER: &'static [u8; 4] = b"GMAD";

#[derive(Debug, Clone, Serialize, Error)]
pub enum GMAError {
	IOError,
	FormatError,
	InvalidHeader,
	EntryNotFound,
}
impl Display for GMAError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use GMAError::*;
		match self {
			IOError => write!(f, "ERR_GMA_IO_ERROR"),
			FormatError => write!(f, "ERR_GMA_FORMAT_ERROR"),
			InvalidHeader => write!(f, "ERR_GMA_INVALID_HEADER"),
			EntryNotFound => write!(f, "ERR_GMA_ENTRY_NOT_FOUND"),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GMAMetadata {
	Legacy {
		title: String,
		description: String,
	},
	Standard {
		#[serde(default)]
		title: String,
		#[serde(rename = "type")]
		addon_type: String,
		tags: Vec<String>,
		ignore: Vec<String>,
	},
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

	extracted_name: String,
}

impl GMAFile {
	pub fn open<P: AsRef<Path>>(path: P) -> Result<GMAFile, GMAError> {
		main_thread_forbidden!();

		let path = path.as_ref();

		let mut f = BufReader::new(File::open(path)?);

		let mut gma = GMAFile {
			size: path.metadata().and_then(|metadata| Ok(metadata.len())).unwrap_or(0),
			path: path.to_path_buf(),
			id: None,
			metadata: None,
			entries: None,
			pointers: GMAFilePointers::default(),
			version: 0,
			extracted_name: String::new(),
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

		gma.compute_extracted_name();

		Ok(gma)
	}

	pub fn set_ws_id(&mut self, id: PublishedFileId) {
		self.id = Some(id);
		self.compute_extracted_name();
	}

	fn compute_extracted_name(&mut self) {
		let mut extracted_name = String::new();
		let mut underscored = false;

		{
			let name = match self.metadata() {
				Ok(_) => match self.metadata.as_ref().unwrap() {
					GMAMetadata::Legacy { title, .. } | GMAMetadata::Standard { title, .. } => {
						title.to_lowercase()
					}
				},
				Err(_) => match self.path.file_name() {
					Some(file_name) => file_name.to_string_lossy().to_lowercase(),
					None => match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
						Ok(unix) => format!("gmpublisher_extracted_{}", unix.as_secs()),
						Err(_) => "gmpublisher_extracted".into(),
					},
				},
			};

			extracted_name.reserve(name.len());

			let mut first = true;
			for char in name.chars() {
				if char.is_alphanumeric() {
					underscored = false;
					extracted_name.push(char);
				} else if !underscored && !first {
					underscored = true;
					extracted_name.push('_');
				}
				first = false;
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

		self.extracted_name = extracted_name;
	}

	pub fn read(&self) -> Result<GMAReadHandle<File>, GMAError> {
		Ok(GMAReadHandle {
			inner: BufReader::new(File::open(&self.path)?),
		})
	}

	pub fn write<P: AsRef<Path>>(src_path: P, dest_path: P, data: &GMAMetadata) -> Result<Transaction, GMAError> {
		let transaction = transaction!();
		GMAWriteHandle {
			inner: BufWriter::new(File::create(dest_path.as_ref())?),
		}
		.create(src_path, data, transaction.clone())?;
		Ok(transaction)
	}
}

pub mod extract;
pub mod read;
pub mod whitelist;
pub mod write;

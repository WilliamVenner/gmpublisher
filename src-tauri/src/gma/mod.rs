use std::{
	collections::HashMap,
	fmt::Display,
	fs::File,
	io::{BufReader, SeekFrom},
	path::{Path, PathBuf},
	time::SystemTime,
};

use byteorder::ReadBytesExt;

use serde::{Deserialize, Serialize};
use steamworks::PublishedFileId;
use thiserror::Error;

use crate::{main_thread_forbidden, ArcBytes};

const GMA_HEADER: &'static [u8; 4] = b"GMAD";

#[derive(Debug, Clone, Serialize, Error)]
pub enum GMAError {
	IOError,
	FormatError,
	InvalidHeader,
	EntryNotFound,
	LZMA,
	Cancelled,
}
impl Display for GMAError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use GMAError::*;
		match self {
			IOError => write!(f, "ERR_IO_ERROR"),
			FormatError => write!(f, "ERR_GMA_FORMAT_ERROR"),
			InvalidHeader => write!(f, "ERR_GMA_INVALID_HEADER"),
			EntryNotFound => write!(f, "ERR_GMA_ENTRY_NOT_FOUND"),
			LZMA => write!(f, "ERR_LZMA"),
			Cancelled => write!(f, "ERR_CANCELLED"),
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
	Standard {
		#[serde(default)]
		title: String,
		#[serde(default)]
		#[serde(rename = "type")]
		addon_type: String,
		#[serde(default)]
		tags: Vec<String>,
		#[serde(default)]
		ignore: Vec<String>,
	},
	Legacy {
		title: String,
		description: String,
	},
}
impl GMAMetadata {
	pub fn title(&self) -> &str {
		match &self {
			GMAMetadata::Standard { title, .. } => title,
			GMAMetadata::Legacy { title, .. } => title,
		}
		.as_str()
	}

	pub fn addon_type(&self) -> Option<&str> {
		match &self {
			GMAMetadata::Standard { addon_type, .. } => Some(addon_type.as_str()),
			_ => None,
		}
	}

	pub fn tags(&self) -> Option<&Vec<String>> {
		match &self {
			GMAMetadata::Standard { tags, .. } => Some(tags),
			_ => None,
		}
	}

	pub fn ignore(&self) -> Option<&Vec<String>> {
		match &self {
			GMAMetadata::Standard { ignore, .. } => Some(ignore),
			_ => None,
		}
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct GMAEntry {
	pub path: String,
	pub size: u64,
	pub crc: u32,

	#[serde(skip)]
	pub index: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GMAFile {
	#[serde(serialize_with = "serde_canonicalize")]
	pub path: PathBuf,
	pub size: u64,

	pub id: Option<PublishedFileId>,

	#[serde(flatten)]
	pub metadata: Option<GMAMetadata>,

	pub entries: Option<HashMap<String, GMAEntry>>,

	#[serde(skip)]
	pub pointers: GMAFilePointers,

	#[serde(skip)]
	pub version: u8,

	pub extracted_name: String,

	#[serde(skip)]
	pub modified: Option<u64>,

	#[serde(skip)]
	pub membuffer: Option<ArcBytes>,
}
impl std::fmt::Debug for GMAFile {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("GMAFile")
			.field("path", &self.path)
			.field("size", &self.size)
			.field("id", &self.id)
			.field("metadata", &self.metadata)
			.field("entries", &self.entries)
			.field("pointers", &self.pointers)
			.field("version", &self.version)
			.field("extracted_name", &self.extracted_name)
			.field("modified", &self.modified)
			.finish()
	}
}
impl PartialEq for GMAFile {
	fn eq(&self, other: &Self) -> bool {
		self.path == other.path
	}
}
impl Eq for GMAFile {}
impl PartialOrd for GMAFile {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.modified.partial_cmp(&other.modified).map(|x| x.reverse())
	}
}
impl Ord for GMAFile {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.modified.cmp(&other.modified).reverse()
	}
}

impl GMAFile {
	fn read_header<P: AsRef<Path>>(mut f: GMAReader, path: P) -> Result<GMAFile, GMAError> {
		let mut gma = GMAFile {
			size: path.as_ref().metadata().map(|metadata| metadata.len()).unwrap_or(0),
			path: path.as_ref().to_owned(),
			id: None,
			metadata: None,
			entries: None,
			pointers: GMAFilePointers::default(),
			version: 0,
			extracted_name: String::new(),
			modified: None,
			membuffer: None,
		};

		if gma.size == 0 {
			if let Ok(size) = crate::stream_len(&mut *f) {
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

		if let GMAReader::MemBuffer(buf) = f {
			gma.membuffer = Some(buf.into_inner());
		}

		Ok(gma)
	}

	pub fn open<P: AsRef<Path>>(path: P) -> Result<GMAFile, GMAError> {
		main_thread_forbidden!();
		GMAFile::read_header(GMAReader::Disk(BufReader::new(File::open(path.as_ref())?)), path)
	}

	pub fn set_ws_id(&mut self, id: PublishedFileId) {
		let compute = self.id.is_some() || self.metadata.is_some();

		self.id = Some(id);

		if compute {
			self.compute_extracted_name();
		} else {
			self.extracted_name.push('_');
			self.extracted_name.push_str(&id.0.to_string());
		}
	}

	fn compute_extracted_name(&mut self) {
		let mut extracted_name = String::new();
		let mut underscored = false;

		{
			let name = match self.metadata {
				Some(ref metadata) => match metadata {
					GMAMetadata::Legacy { title, .. } | GMAMetadata::Standard { title, .. } => title.to_lowercase(),
				},
				None => match self.path.file_name() {
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
}

fn serde_canonicalize<S>(path: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	match dunce::canonicalize(&path) {
		Ok(path) => path.serialize(serializer),
		Err(_) => path.serialize(serializer),
	}
}

pub mod whitelist;
pub use whitelist::*;

pub mod extract;
pub use extract::*;

pub mod read;
pub use read::*;

pub mod write;
pub use write::*;

pub mod preview;

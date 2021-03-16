use std::path::PathBuf;
use serde::{Serialize, Deserialize};

pub const GMA_HEADER: &'static [u8; 4] = b"GMAD";
pub const SUPPORTED_GMA_VERSION: u8 = 3;

pub mod read;
pub use read::*;

pub mod write;
use steamworks::PublishedFileId;
pub use write::*;

pub type ProgressCallback = Box<dyn Fn(f32) -> ()>;

#[derive(Serialize, Deserialize, Clone)]
pub struct GMAFile {
	pub id: Option<PublishedFileId>,
	
	pub path: Option<PathBuf>,
	pub size: u64,
	
	#[serde(rename = "type")]
	pub addon_type: Option<String>,
	pub name: String,
	pub tags: Option<Vec<String>>,
	pub ignore: Option<Vec<String>>,
	
	pub entries: Option<Vec<GMAEntry>>,

	#[serde(skip)]
	pub entries_read: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GMAEntry {
	#[serde(skip)]
	pub data: Option<Vec<u8>>,
	pub path: PathBuf,
	pub size: u64,
	pub crc: u32,
}
impl std::fmt::Debug for GMAEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GMAEntry")
         .field("path", &self.path)
         .field("size", &self.size)
         .field("crc", &self.crc)
         .field("data", &match &self.data {
			 Some(data) => data.len(),
			 None => 0
		 })
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
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
	pub name: Option<String>,
	pub description: Option<String>,
	pub tags: Option<Vec<String>>,
	pub ignore: Option<Vec<String>>,
	
	#[serde(skip)]
	pub entries: Option<Vec<GMAEntry>>,
}

#[derive(Clone)]
pub struct GMAEntry {
	pub data: Option<Vec<u8>>,
	pub path: PathBuf,
	pub size: u64,
	pub crc: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddonJson {
	pub title: String,
	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
	pub ignore: Vec<String>
}
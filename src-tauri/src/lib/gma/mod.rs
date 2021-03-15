mod read;
pub use read::read_gma;
pub use read::GMAReadError;

mod write;
pub use write::write_gma;

use serde::{Serialize, Deserialize};
use std::{fs::File, path::PathBuf};
use std::path::Path;

pub const GMA_HEADER: &'static [u8; 4] = b"GMAD";
pub const SUPPORTED_GMA_VERSION: u8 = 3;

pub struct GMAFile {
	pub path: PathBuf,
	pub name: String,
	pub description: String,
	pub author: String,
	pub entries: Vec<GMAEntry>,
	pub size: usize
}
pub struct GMAEntry {
	pub name: String,
	pub size: u64,
	pub crc: u32,
	pub contents: Option<Vec<u8>>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddonJson {
	pub title: String,
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
	pub ignore: Vec<String>,
}

impl AddonJson {
	pub fn from_file(p: &Path) -> Option<AddonJson> {
		let f = match File::open(p) {
			Ok(f) => f,
			_ => return None
		};
		let parsed_addon: serde_json::Result<AddonJson> =
			serde_json::from_reader(f);
		
		parsed_addon.ok()
	}
	pub fn from_gma_file(f: &GMAFile) -> Option<AddonJson> {
		let parsed_desc: serde_json::Result<GMADescriptionJson> =
			serde_json::from_str(&f.description);
		
		if let Ok(gma_desc) = parsed_desc {
			Some(
				AddonJson {
					title: f.name.clone(),
					description: gma_desc.description,
					addon_type: gma_desc.addon_type,
					tags: gma_desc.tags,
					ignore: vec!()
				}
			)
		} else {
			None
		}
	}
}

/// Subset of addon.json properties that are stored (by convention) in gma description field
#[derive(Serialize, Deserialize, Debug)]
pub struct GMADescriptionJson {
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub addon_type: String,
	pub tags: Vec<String>,
}

impl GMADescriptionJson {
	pub fn from_addon(a: &AddonJson) -> GMADescriptionJson {
		GMADescriptionJson {
			description: a.description.clone(),
			addon_type: a.addon_type.clone(),
			tags: a.tags.clone()
		}
	}
}
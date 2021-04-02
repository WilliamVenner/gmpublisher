use std::path::PathBuf;

use crate::app_data;

use super::{GMAError, GMAFile};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub enum ExtractDestination {
	Temp,
	Downloads,
	Addons,
	/// path/to/addon/*
	Directory(PathBuf),
	/// path/to/addon/addon_name_123456790/*
	NamedDirectory(PathBuf),
}
impl ExtractDestination {
    fn into(self, extracted_name: String) -> PathBuf {
        use ExtractDestination::*;

		let push_extracted_name = |mut path: PathBuf| {
			path.push(extracted_name);
			Some(path)
		};

		match self {

			Temp => None,

		    Directory(path) => Some(path),
			
			Addons => app_data!().gmod().and_then(|mut path| {
				path.push("garrysmod");
				path.push("addons");
				Some(path)
			}),

			Downloads => dirs::download_dir().and_then(push_extracted_name),

			NamedDirectory(mut path) => push_extracted_name(path),

		}.unwrap_or_else(|| push_extracted_name(std::env::temp_dir()).unwrap())
    }
}

impl GMAFile {
	pub fn extract(&mut self, dest: ExtractDestination) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		self.entries()?;

		let path = dest.into(self.extracted_name());
		
		
	}

	pub fn extract_entry(&mut self) -> Result<PathBuf, GMAError> {
		main_thread_forbidden!();

		self.entries()?;


	}
}
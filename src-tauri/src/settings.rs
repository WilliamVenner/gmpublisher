use anyhow::Error;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};
use tauri::{Webview, WebviewMut};

use crate::show;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub(crate) struct Settings {
	pub(crate) window_size: (i32, i32),
	pub(crate) window_maximized: bool,

	pub(crate) gmod: Option<PathBuf>,
	pub(crate) destinations: Vec<PathBuf>,
	pub(crate) create_folder_on_extract: bool,

	#[serde(skip)]
	pub(crate) file: PathBuf,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			file: PathBuf::from(""),

			window_size: (1280, 720), // TODO make this a pct of the primary monitors
			window_maximized: true,
			gmod: None,
			destinations: Vec::new(),
			create_folder_on_extract: true,
		}
	}
}

impl Settings {
	pub(crate) fn from_json(json: &str) -> Result<Settings, Error> {
		Ok(Settings::from(serde_json::from_str(json)?))
	}

	pub(crate) fn sanitize(&mut self) {
		// TODO replace with drain_filter when it's stable
		let mut i = 0;
		while i != self.destinations.len() {
			if !self.destinations[i].exists() {
				self.destinations.remove(i);
			} else {
				i += 1;
			}
		}

		self.destinations.truncate(20);
	}

	pub(crate) fn load(data_dir: &PathBuf) -> Settings {
		let mut settings_file = data_dir.to_owned();
		settings_file.push("settings.json");
		if Path::is_file(&settings_file) {
			match fs::read_to_string(&settings_file) {
				Ok(settings_str) => match serde_json::de::from_str::<Settings>(&settings_str) {
					Ok(mut data) => {
						data.file = settings_file;
						data.sanitize();
						return data;
					}
					Err(_) => {}
				},
				Err(_) => {}
			}
		}

		let mut data = Settings::default();
		data.file = settings_file.clone();
		data.sanitize();

		match data.save(None) {
			Ok(_) => {}
			Err(error) => show::panic(format!(
				"Failed to save user settings!\nPath: {:?}\nError: {:#?}",
				settings_file, error
			)),
		}

		data
	}

	pub(crate) fn save(&self, location: Option<&PathBuf>) -> Result<(), Error> {
		Ok(fs::write(
			&location.unwrap_or(&self.file),
			serde_json::ser::to_string(&self)?,
		)?)
	}

	pub(crate) fn save_json(json: &str, location: Option<&PathBuf>) -> Result<Settings, Error> {
		let settings = Settings::from_json(json)?;
		settings.save(location)?;
		Ok(settings)
	}

	pub(crate) fn send(&mut self, mut webview: WebviewMut) {
		self.sanitize();
		let success = tauri::event::emit(
			&mut webview,
			"updateAppData",
			Some(&*crate::APP_DATA.read()),
		);
		debug_assert!(success.is_ok(), "Failed to update app data");
	}
}

pub(crate) fn invoke_handler(_webview: &mut Webview<'_>, settings: String) -> Result<(), String> {
	let mut app_data = crate::APP_DATA.write();
	match Settings::save_json(&settings, Some(&app_data.settings.file)) {
		Ok(mut deserialized) => {
			deserialized.file = app_data.settings.file.clone();
			(*app_data).settings = deserialized;
			Ok(())
		}
		Err(error) => Err(format!("Failed to save settings file!\n{:#?}", error)),
	}
}

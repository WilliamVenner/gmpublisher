use std::{fs, path::Path, path::PathBuf, sync::{Arc, Mutex}};
use serde::{Serialize, Deserialize};
use anyhow::Error;
use tauri::Webview;

use crate::{appdata::AppData, show, workshop::Workshop};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Settings {
	pub(crate) window_size: (i32, i32),
	pub(crate) gmod: Option<PathBuf>,

	#[serde(skip)]
	pub(crate) file: PathBuf
}

impl Settings {
	pub(crate) fn from_json(json: &str) -> Result<Settings, Error> {
		Ok(Settings::from(serde_json::from_str(json)?))
	}

	pub(crate) fn load(data_dir: &PathBuf) -> Settings {
		let mut settings_file = data_dir.to_owned();
		settings_file.push("settings.json");
		if Path::is_file(&settings_file) {
			match fs::read_to_string(&settings_file) {
				Ok(settings_str) => {
					match serde_json::de::from_str::<Settings>(&settings_str) {
						Ok(mut data) => {
							data.file = settings_file;
							return data;
						},
						Err(_) => {}
					}
				},
				Err(_) => {}
			}
		}

		let data = Settings {
			window_size: (1300, 750),
			gmod: None,
			file: settings_file.clone()
		};
		
		match data.save(None) {
			Ok(_) => {},
			Err(error) => show::panic(format!("Failed to save user settings!\nPath: {:?}\nError: {:#?}", settings_file, error))
		}

		data
	}

	pub(crate) fn save(&self, location: Option<&PathBuf>) -> Result<(), Error> {
		Ok(fs::write(&location.unwrap_or(&self.file), serde_json::ser::to_string(&self)?)?)
	}

	pub(crate) fn save_json(json: &str, location: Option<&PathBuf>) -> Result<Settings, Error> {
		let settings = Settings::from_json(json)?;
		settings.save(location)?;
		Ok(settings)
	}
}

pub(crate) fn invoke_handler(_webview: &mut Webview<'_>, app_data: Arc<Mutex<AppData>>, settings: String) -> Result<(), String> {
	let mut app_data = app_data.lock().unwrap();
	match Settings::save_json(&settings, Some(&app_data.settings.file)) {
		Ok(mut deserialized) => {
			deserialized.file = app_data.settings.file.clone();
			(*app_data).settings = deserialized;
			Ok(())
		},
		Err(error) => Err(format!("Failed to save settings file!\n{:#?}", error))
	}
}
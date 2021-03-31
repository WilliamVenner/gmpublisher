use std::{
	fs::File,
	io::{BufReader, BufWriter},
	path::PathBuf,
};

use crate::webview;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::WebviewManager;

const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
	name: "gmpublisher",
	author: "WilliamVenner",
};
lazy_static! {
	static ref APP_DATA_DIR: PathBuf = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &APP_INFO).unwrap();
	static ref APP_SETTINGS_PATH: PathBuf = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &APP_INFO).unwrap().join("settings.json");
}

fn settings_path() -> PathBuf {
	APP_SETTINGS_PATH.to_owned()
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Settings {
	pub(crate) gmod: Option<PathBuf>,
	pub(crate) window_size: (f64, f64),
	pub(crate) window_maximized: bool,
	pub(crate) destinations: Vec<PathBuf>,
	pub(crate) create_folder_on_extract: bool,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			gmod: None,
			window_size: (800., 600.),
			window_maximized: true,
			destinations: Vec::new(),
			create_folder_on_extract: true,
		}
	}
}
impl Settings {
	pub(crate) fn init() -> Settings {
		match Settings::load() {
			Ok(settings) => settings,
			Err(_) => Settings::default(),
		}
	}

	fn load() -> Result<Settings, anyhow::Error> {
		Ok(serde_json::de::from_reader(BufReader::new(File::open(&*APP_SETTINGS_PATH)?))?)
	}

	pub(crate) fn save(&self) -> Result<(), anyhow::Error> {
		Ok(serde_json::ser::to_writer(BufWriter::new(File::open(&*APP_SETTINGS_PATH)?), self)?)
	}

	pub(crate) fn sanitize(&mut self) -> bool {
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

		true
	}
}

#[derive(Debug, Serialize)]
pub(crate) struct AppData {
	pub(crate) settings: RwLock<Settings>,
}
impl AppData {
	pub(crate) fn init() -> Self {
		Self {
			settings: RwLock::new(Settings::init()),
		}
	}

	pub(crate) fn send(&self) {
		webview!().emit("UpdateAppData", Some(self)).unwrap();
	}
}

#[tauri::command(with_manager)]
pub(crate) fn update_settings(mgr: WebviewManager, mut settings: Settings) {
	if settings.sanitize() {
		settings.save().ok();
		*crate::APP_DATA.settings.write() = settings;
	}
}

pub(crate) struct Plugin;
impl<Application: tauri::ApplicationExt + 'static> tauri::plugin::Plugin<Application> for Plugin {
	fn initialization_script(&self) -> Option<String> {
		Some(
			include_str!("../../app/plugins/AppData.js")
				.replace("{$_APP_DATA_$}", &serde_json::ser::to_string(&*crate::APP_DATA).unwrap().replace("\\", "\\\\").replace("'", "\\'"))
				.replace(
					"{$_WS_DEAD_$}",
					&serde_json::ser::to_string(&crate::WorkshopItem::from(steamworks::PublishedFileId(0)))
						.unwrap()
						.replace("\\", "\\\\")
						.replace("'", "\\'"),
				),
		)
	}

	fn name(&self) -> &'static str {
		"AppData"
	}
}

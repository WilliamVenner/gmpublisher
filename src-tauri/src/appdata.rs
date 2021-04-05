use std::{fs::File, io::{BufReader, BufWriter}, path::PathBuf};

use crate::{octopus::game_addons, webview_emit};

use crate::GMOD_APP_ID;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

const APP_INFO: app_dirs::AppInfo = app_dirs::AppInfo {
	name: "gmpublisher",
	author: "WilliamVenner",
};
lazy_static! {
	static ref APP_DATA_DIR: PathBuf = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &APP_INFO).unwrap();
	static ref APP_SETTINGS_PATH: PathBuf = app_dirs::app_root(app_dirs::AppDataType::UserConfig, &APP_INFO)
		.unwrap()
		.join("settings.json");
}

fn settings_path() -> PathBuf {
	APP_SETTINGS_PATH.to_owned()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
	pub gmod: Option<PathBuf>,
	pub window_size: (f64, f64),
	pub window_maximized: bool,
	pub destinations: Vec<PathBuf>,
	pub create_folder_on_extract: bool,
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
	pub fn init() -> Settings {
		match Settings::load() {
			Ok(settings) => settings,
			Err(_) => Settings::default(),
		}
	}

	fn load() -> Result<Settings, anyhow::Error> {
		Ok(serde_json::de::from_reader(BufReader::new(File::open(&*APP_SETTINGS_PATH)?))?)
	}

	pub fn save(&self) -> Result<(), anyhow::Error> {
		Ok(serde_json::ser::to_writer(BufWriter::new(File::open(&*APP_SETTINGS_PATH)?), self)?)
	}

	pub fn sanitize(&mut self) -> bool {
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
pub struct AppData {
	pub settings: RwLock<Settings>,
}
impl AppData {
	pub fn init() -> Self {
		Self {
			settings: RwLock::new(Settings::init()),
		}
	}

	pub fn send(&'static self) {
		webview_emit!("UpdateAppData", self);
	}

	pub fn gmod(&self) -> Option<PathBuf> {
		if let Some(ref gmod) = self.settings.read().gmod {
			if gmod.is_dir() && gmod.exists() {
				return Some(gmod.clone());
			}
		}

		if !steamworks!().connected() { return None };

		let gmod: PathBuf = steamworks!().client().apps().app_install_dir(GMOD_APP_ID).into();
		if gmod.is_dir() && gmod.exists() {
			Some(gmod)
		} else {
			None
		}
	}
}

#[tauri::command]
pub fn update_settings(mut settings: Settings) {
	if settings.sanitize() {
		ignore! { settings.save() };
		
		let rediscover_addons = crate::APP_DATA.settings.read().gmod != settings.gmod;

		*crate::APP_DATA.settings.write() = settings;

		if rediscover_addons {
			game_addons!().refresh();
		}
	}
}

#[tauri::command]
pub fn clean_app_data() {
	// TODO
	// clean %temp%/gmpublisher
}

pub struct Plugin;
impl<Application: tauri::ApplicationExt + 'static> tauri::plugin::Plugin<Application> for Plugin {
	fn initialization_script(&self) -> Option<String> {
		Some(
			include_str!("../../app/plugins/AppData.js")
				.replace(
					"{$_APP_DATA_$}",
					&crate::escape_single_quoted_json(serde_json::ser::to_string(&*crate::APP_DATA).unwrap()),
				)
				.replace(
					"{$_WS_DEAD_$}",
					&crate::escape_single_quoted_json(serde_json::ser::to_string(&crate::WorkshopItem::from(steamworks::PublishedFileId(0))).unwrap()),
				),
		)
	}

	fn name(&self) -> &'static str {
		"AppData"
	}
}

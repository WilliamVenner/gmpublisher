use std::{borrow::Cow, fs::File, io::{BufReader, BufWriter}, path::PathBuf};

use crate::webview_emit;

use crate::GMOD_APP_ID;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tauri::Params;

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

fn serde_settings_gmod<S>(_: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	match app_data!().gmod() {
		Some(gmod) => serializer.serialize_some(&*gmod),
		None => serializer.serialize_none()
	}
}

fn serde_settings_temp<S>(_: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	match app_data!().temp() {
		Some(temp) => serializer.serialize_some(&*temp),
		None => serializer.serialize_none()
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
	#[serde(serialize_with = "serde_settings_temp")]
	pub temp: Option<PathBuf>,
	#[serde(serialize_with = "serde_settings_gmod")]
	pub gmod: Option<PathBuf>,

	pub window_size: (f64, f64),
	pub window_maximized: bool,
	pub destinations: Vec<PathBuf>,
	pub create_folder_on_extract: bool,
	pub content_generator_history: Vec<PathBuf>,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			temp: None,
			gmod: None,
			window_size: (800., 600.),
			window_maximized: true,
			destinations: Vec::new(),
			create_folder_on_extract: true,
			content_generator_history: Vec::new(),
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
	pub version: &'static str,
	pub downloads_dir: Option<PathBuf>,
}
impl AppData {
	pub fn init() -> Self {
		Self {
			settings: RwLock::new(Settings::init()),
			version: env!("CARGO_PKG_VERSION"),
			downloads_dir: dirs::download_dir(),
		}
	}

	pub fn send(&'static self) {
		webview_emit!("UpdateAppData", self);
	}

	pub fn gmod(&self) -> Option<PathBuf> {
		if let Some(ref gmod) = self.settings.read().gmod {
			if gmod.is_dir() && gmod.exists() {
				return Some(gmod.to_owned());
			}
		}

		if !steam!().connected() {
			return steamlocate::SteamDir::locate().and_then(|mut steam_dir| {
				steam_dir.app(&GMOD_APP_ID.0).and_then(|steam_app| {
					Some(steam_app.path.to_owned())
				})
			});
		}

		let gmod: PathBuf = steam!().client().apps().app_install_dir(GMOD_APP_ID).into();
		if gmod.is_dir() && gmod.exists() {
			Some(gmod)
		} else {
			None
		}
	}

	pub fn temp(&self) -> Option<PathBuf> {
		if let Some(ref temp) = self.settings.read().temp {
			if temp.is_dir() && temp.exists() {
				return Some(temp.to_owned());
			}
		}

		Some(std::env::temp_dir())
	}
}

#[cfg(target_os = "windows")]
const PATH_SEPARATOR: char = '\\';
#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: char = '/';

pub struct Plugin;
impl<M: Params + 'static> tauri::plugin::Plugin<M> for Plugin {
	fn initialization_script(&self) -> Option<String> {
		Some(
			include_str!("../../app/plugins/AppData.js")
				.replacen(
					"{$_APP_DATA_$}",
					&crate::escape_single_quoted_json(serde_json::ser::to_string(&*crate::APP_DATA).unwrap()),
					1
				)
				.replacen(
					"{$_WS_DEAD_$}",
					&crate::escape_single_quoted_json(
						serde_json::ser::to_string(&crate::WorkshopItem::from(steamworks::PublishedFileId(0))).unwrap(),
					),
					1
				)
				.replacen(
					"{$_PATH_SEPARATOR_$}",
					&serde_json::ser::to_string(&PATH_SEPARATOR).unwrap(),
					1
				),
		)
	}

	fn name(&self) -> &'static str {
		"AppData"
	}
}

#[tauri::command]
pub fn update_settings(mut settings: Settings) {
	if settings.sanitize() {
		ignore! { settings.save() };

		let rediscover_addons = app_data!().settings.read().gmod != settings.gmod;

		*app_data!().settings.write() = settings;

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

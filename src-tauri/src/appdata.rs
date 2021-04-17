use std::{fs::File, io::{BufReader, BufWriter}, path::PathBuf};

use crate::{RwLockCow, webview_emit};

use crate::GMOD_APP_ID;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockReadGuard};
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
	static ref TEMP_DIR: PathBuf = std::env::temp_dir();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
	pub temp: Option<PathBuf>,
	pub gmod: Option<PathBuf>,
	pub user_data: Option<PathBuf>,

	pub window_size: (f64, f64),
	pub window_maximized: bool,
	pub destinations: Vec<PathBuf>,
	pub create_folder_on_extract: bool,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			temp: None,
			gmod: None,
			user_data: None,
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
	pub version: &'static str,

	#[serde(serialize_with = "serde_temp_dir")]
	temp_dir: PathBuf,
	#[serde(serialize_with = "serde_gmod_dir")]
	gmod_dir: Option<PathBuf>,
	#[serde(serialize_with = "serde_user_data_dir")]
	user_data_dir: PathBuf,
	downloads_dir: Option<PathBuf>,
}
impl AppData {
	pub fn init() -> Self {
		let settings = Settings::init();
		Self {
			settings: RwLock::new(settings),
			version: env!("CARGO_PKG_VERSION"),

			downloads_dir: dirs::download_dir(),
			temp_dir: PathBuf::new(),
			gmod_dir: None,
			user_data_dir: PathBuf::new(),
		}
	}

	pub fn send(&'static self) {
		webview_emit!("UpdateAppData", self);
	}

	pub fn gmod_dir(&self) -> Option<PathBuf> {
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

	pub fn temp_dir(&self) -> RwLockCow<'_, PathBuf> {
		let lock = self.settings.read();
		if let Some(ref temp) = lock.temp {
			if temp.is_dir() && temp.exists() {
				return RwLockCow::Locked(RwLockReadGuard::map(lock, |s| s.temp.as_ref().unwrap()));
			}
		}

		RwLockCow::Borrowed(&*TEMP_DIR)
	}

	pub fn user_data_dir(&self) -> RwLockCow<'_, PathBuf> {
		let lock = self.settings.read();
		if let Some(ref user_data) = lock.user_data {
			if user_data.is_dir() && user_data.exists() {
				return RwLockCow::Locked(RwLockReadGuard::map(lock, |s| s.user_data.as_ref().unwrap()));
			}
		}

		RwLockCow::Borrowed(&*APP_DATA_DIR)
	}

	pub fn downloads_dir(&self) -> Option<&PathBuf> {
		self.downloads_dir.as_ref()
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

fn serde_gmod_dir<S>(_: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	app_data!().gmod_dir().serialize(serializer)
}

fn serde_temp_dir<S>(_: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	app_data!().temp_dir().serialize(serializer)
}

fn serde_user_data_dir<S>(_: &PathBuf, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	app_data!().user_data_dir().serialize(serializer)
}

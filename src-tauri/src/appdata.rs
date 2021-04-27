use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
	cell::Cell,
	collections::HashMap,
	fs::{self, File, OpenOptions},
	io::{Seek, SeekFrom},
	path::PathBuf,
};

use crate::{gma::ExtractDestination, webview_emit, RwLockCow};

use crate::GMOD_APP_ID;
use lazy_static::lazy_static;
use parking_lot::{RwLock, RwLockReadGuard};
use serde::{Deserialize, Serialize};
use steamworks::PublishedFileId;
use tauri::Params;

lazy_static! {
	static ref USER_DATA_DIR: PathBuf = dirs_next::data_dir()
		.unwrap_or_else(|| std::env::current_exe().unwrap_or_else(|_| std::env::temp_dir()))
		.join("gmpublisher");
	static ref APP_SETTINGS_PATH: PathBuf = dirs_next::config_dir()
		.unwrap_or_else(|| dirs_next::data_dir().unwrap_or_else(|| std::env::current_exe().unwrap_or_else(|_| std::env::temp_dir())))
		.join("gmpublisher/settings.json");
	static ref TEMP_DIR: PathBuf = std::env::temp_dir().join("gmpublisher");
	static ref DOWNLOADS_DIR: Option<PathBuf> = dirs::download_dir();
}

#[derive(Debug)]
pub struct OpenCount(Cell<u32>);
unsafe impl Send for OpenCount {}
unsafe impl Sync for OpenCount {}
impl OpenCount {
	fn init() -> OpenCount {
		OpenCount(Cell::new(0))
	}

	fn increment(&self) {
		self.0.set(
			(|| -> Result<u32, std::io::Error> {
				let mut count_file = app_data!().user_data_dir().to_owned();
				fs::create_dir_all(&count_file)?;

				count_file.push(".open_count");

				let mut f = OpenOptions::new().read(true).write(true).create(true).open(count_file)?;

				let count = f.read_u32::<LittleEndian>().unwrap_or(0) + 1;

				f.seek(SeekFrom::Start(0))?;
				f.write_u32::<LittleEndian>(count)?;

				Ok(count)
			})()
			.unwrap_or(0),
		);
	}
}
impl serde::Serialize for OpenCount {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_u32(self.0.get())
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
	pub temp: Option<PathBuf>,
	pub gmod: Option<PathBuf>,
	pub user_data: Option<PathBuf>,
	pub downloads: Option<PathBuf>,

	pub sounds: bool,

	pub window_size: (f64, f64),
	pub window_maximized: bool,

	pub extract_destination: ExtractDestination,
	pub destinations: Vec<PathBuf>,
	pub create_folder_on_extract: bool,

	pub ignore_globs: Vec<String>,

	pub my_workshop_local_paths: HashMap<PublishedFileId, PathBuf>,
	pub upscale_addon_icon: bool,

	pub language: Option<String>,
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			temp: None,
			gmod: None,
			user_data: None,
			downloads: None,

			extract_destination: ExtractDestination::default(),
			sounds: true,

			window_size: (800., 600.),
			window_maximized: false,

			destinations: Vec::new(),
			create_folder_on_extract: true,

			ignore_globs: Vec::new(),
			my_workshop_local_paths: HashMap::new(),
			upscale_addon_icon: true,

			language: None,
		}
	}
}
impl Settings {
	pub fn init() -> Settings {
		println!("Initializing Settings...");
		match Settings::load(false) {
			Ok(settings) => settings,
			Err(_) => Settings::default(),
		}
	}

	fn load(sanitize: bool) -> Result<Settings, anyhow::Error> {
		let contents = fs::read_to_string(&*APP_SETTINGS_PATH)?;
		let mut settings: Settings = serde_json::de::from_str(&contents)?;
		if sanitize {
			settings.sanitize();
		}
		Ok(settings)
	}

	pub fn save(&self) -> Result<(), anyhow::Error> {
		Ok(serde_json::ser::to_writer(File::create(&*APP_SETTINGS_PATH)?, self)?)
	}

	pub fn sanitize(&mut self) {
		self.destinations.retain(|dir| dir.is_absolute() && dir.is_dir());
		self.my_workshop_local_paths.retain(|_, dir| dir.is_absolute() && dir.is_dir());

		match &self.extract_destination {
			ExtractDestination::Directory(path) => {
				if self.create_folder_on_extract || !path.is_dir() {
					self.extract_destination = ExtractDestination::NamedDirectory(path.to_owned());
				}
			}
			ExtractDestination::NamedDirectory(path) => {
				if !self.create_folder_on_extract || !path.is_dir() {
					self.extract_destination = ExtractDestination::Directory(path.to_owned());
				}
			}
			ExtractDestination::Downloads => {
				if app_data!().downloads_dir().is_none() {
					self.extract_destination = ExtractDestination::default();
				}
			}
			ExtractDestination::Addons => {
				if app_data!().gmod_dir().is_none() {
					self.extract_destination = ExtractDestination::default();
				}
			}
			_ => {}
		}

		self.destinations.truncate(20);
	}
}

#[derive(Debug, Serialize)]
pub struct AppData {
	pub settings: RwLock<Settings>,
	pub version: &'static str,
	pub open_count: OpenCount,

	#[serde(serialize_with = "serde_temp_dir")]
	temp_dir: PathBuf,
	#[serde(serialize_with = "serde_gmod_dir")]
	gmod_dir: Option<PathBuf>,
	#[serde(serialize_with = "serde_user_data_dir")]
	user_data_dir: PathBuf,
	#[serde(serialize_with = "serde_downloads_dir")]
	downloads_dir: Option<PathBuf>,
}
impl AppData {
	pub fn init() -> Self {
		let settings = Settings::init();
		Self {
			settings: RwLock::new(settings),
			version: env!("CARGO_PKG_VERSION"),
			open_count: OpenCount::init(),

			// Placeholders
			temp_dir: PathBuf::new(),
			user_data_dir: PathBuf::new(),
			gmod_dir: None,
			downloads_dir: None,
		}
	}

	pub fn send(&'static self) {
		webview_emit!("UpdateAppData", self);
	}

	pub fn gmod_dir(&self) -> Option<PathBuf> {
		println!("Locating Garry's Mod...");

		if let Some(ref gmod) = self.settings.read().gmod {
			if gmod.is_dir() {
				println!("Using user-defined path");
				return Some(gmod.to_owned());
			}
		}

		if !steam!().connected() {
			println!("Steam is not connected, parsing Steam library folders...");
			match steamlocate::SteamDir::locate().and_then(|mut steam_dir| steam_dir.app(&GMOD_APP_ID.0).and_then(|steam_app| Some(steam_app.path.to_owned()))) {
				Some(path) => {
					println!("Located!");
					return Some(path)
				},
				None => {
					println!("Failed to parse Steam library folders. Waiting for Steam...");
					for i in 0..3 as u8 {
						sleep!(1);
						if steam!().connected() {
							println!("Steam connected!");
							break;
						} else if i == 2 {
							println!("Gave up.");
							return None;
						}
					}
				}
			}
		}

		println!("Getting Garry's Mod location from Steamworks...");
		let gmod: PathBuf = steam!().client().apps().app_install_dir(GMOD_APP_ID).into();
		if gmod.is_dir() {
			println!("Located!");
			Some(gmod)
		} else {
			println!("Failed.");
			None
		}
	}

	pub fn temp_dir(&self) -> RwLockCow<'_, PathBuf> {
		let lock = self.settings.read();
		if let Some(ref temp) = lock.temp {
			if temp.is_dir() {
				return RwLockCow::Locked(RwLockReadGuard::map(lock, |s| s.temp.as_ref().unwrap()));
			}
		}

		RwLockCow::Borrowed(&*TEMP_DIR)
	}

	pub fn user_data_dir(&self) -> RwLockCow<'_, PathBuf> {
		let lock = self.settings.read();
		if let Some(ref user_data) = lock.user_data {
			if user_data.is_dir() {
				return RwLockCow::Locked(RwLockReadGuard::map(lock, |s| s.user_data.as_ref().unwrap()));
			}
		}

		RwLockCow::Borrowed(&*USER_DATA_DIR)
	}

	pub fn downloads_dir(&self) -> RwLockCow<'_, Option<PathBuf>> {
		let lock = self.settings.read();
		if let Some(ref downloads) = lock.downloads {
			if downloads.is_dir() {
				return RwLockCow::Locked(RwLockReadGuard::map(lock, |s| &s.downloads));
			}
		}

		RwLockCow::Borrowed(&*DOWNLOADS_DIR)
	}
}

#[cfg(target_os = "windows")]
const PATH_SEPARATOR: char = '\\';
#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: char = '/';

pub struct Plugin;
impl<M: Params + 'static> tauri::plugin::Plugin<M> for Plugin {
	fn initialization_script(&self) -> Option<String> {
		let mut sanitized = app_data!().settings.read().clone();
		sanitized.sanitize();
		*app_data!().settings.write() = sanitized;

		let mut default_ignore: Vec<String> = crate::gma::DEFAULT_IGNORE
			.iter()
			.map(|glob| (&glob[0..glob.len() - 1]).to_string())
			.collect::<Vec<String>>();
		default_ignore.sort();

		app_data!().open_count.increment();

		Some(
			include_str!("../../app/plugins/AppData.js")
				.replacen(
					"{$_APP_DATA_$}",
					&crate::escape_single_quoted_json(serde_json::ser::to_string(&*crate::APP_DATA).unwrap()),
					1,
				)
				.replacen(
					"{$_WS_DEAD_$}",
					&crate::escape_single_quoted_json(
						serde_json::ser::to_string(&crate::WorkshopItem::from(steamworks::PublishedFileId(0))).unwrap(),
					),
					1,
				)
				.replacen(
					"{$_DEFAULT_IGNORE_GLOBS_$}",
					&crate::escape_single_quoted_json(serde_json::ser::to_string(&default_ignore).unwrap()),
					1,
				)
				.replacen("{$_PATH_SEPARATOR_$}", &serde_json::ser::to_string(&PATH_SEPARATOR).unwrap(), 1),
		)
	}

	fn name(&self) -> &'static str {
		"AppData"
	}
}

#[tauri::command]
pub fn update_settings(mut settings: Settings) -> bool {
	settings.sanitize();

	ignore! { settings.save() };

	let rediscover_addons = app_data!().settings.read().gmod != settings.gmod;

	*app_data!().settings.write() = settings;

	if rediscover_addons {
		game_addons!().refresh();
		webview_emit!("InstalledAddonsRefreshed");
	}

	webview_emit!("UpdateAppData", &*crate::APP_DATA);

	true
}

#[tauri::command]
pub fn validate_gmod(mut path: PathBuf) -> bool {
	path.push("GarrysMod");
	path.push("addons");
	path.is_absolute() && path.is_dir()
}

#[tauri::command]
pub fn window_resized(width: f64, height: f64) {
	{
		let mut settings = app_data!().settings.write();
		settings.window_size = (width, height);
		settings.window_maximized = webview!().window.borrow().as_ref().unwrap().is_maximized();
	}
	ignore! { app_data!().settings.read().save() };
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

fn serde_downloads_dir<S>(_: &Option<PathBuf>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	app_data!().downloads_dir().serialize(serializer)
}

pub fn write_tauri_settings() -> Option<()> {
	use serde_json::Value as JsonValue;
	use std::io::{BufReader, BufWriter};

	let mut settings_path = dirs_next::config_dir()?;
	settings_path.push("gmpublisher");

	fs::create_dir_all(&settings_path).ok()?;

	settings_path.push(".tauri-settings.json");

	if settings_path.exists() {
		let stored = {
			let mut stored: HashMap<String, JsonValue> = serde_json::from_reader(BufReader::new(File::open(&settings_path).ok()?)).ok()?;
			stored.insert("allow_notification".to_string(), JsonValue::Bool(true));
			stored
		};
		serde_json::to_writer(BufWriter::new(File::create(settings_path).ok()?), &stored).ok()?;
	} else {
		fs::write(settings_path, r#"{"allow_notification":true}"#).ok()?;
	}

	Some(())
}

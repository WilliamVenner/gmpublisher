use std::{env, fs, path::PathBuf};
use anyhow::{Error, anyhow};
use serde::Serialize;

extern crate steamlocate;

use crate::{settings::Settings, workshop::SteamUser};

#[derive(Debug, Serialize, Clone)]
pub(crate) struct AppData {
	pub(crate) version: String,
	pub(crate) settings: Settings,
	pub(crate) data_dir: PathBuf,
	pub(crate) gmad: Option<PathBuf>,
	pub(crate) gmpublish: Option<PathBuf>,
	pub(crate) gmod: Option<PathBuf>,
	pub(crate) user: SteamUser
}

impl AppData {
	pub(crate) fn init(user: SteamUser) -> Result<AppData, Error> {
		let data_dir = AppData::create_data_dir()?;

		let mut app_data = AppData {
			version: env!("CARGO_PKG_VERSION").to_string(),
			settings: Settings::load(&data_dir),
			data_dir,
			gmod: None,
			gmad: None,
			gmpublish: None,
			user,
		};

		app_data.find_gmod();

		Ok(app_data)
	}

	fn create_data_dir() -> Result<PathBuf, anyhow::Error> {
		match dirs::home_dir() {
			Some(mut gmpublisher_dir) => {
				gmpublisher_dir.push(".gmpublisher");
				if gmpublisher_dir.is_file() { return Err(anyhow!(format!("This file is present on your system, and shouldn't be: {:?}", gmpublisher_dir))); }
				if gmpublisher_dir.is_dir() || fs::create_dir(&gmpublisher_dir).is_ok() { return Ok(gmpublisher_dir); }
			},
			None => {}
		}

		let mut gmpublisher_dir = env::current_dir()?;
		gmpublisher_dir.push(".gmpublisher");

		if gmpublisher_dir.is_file() { return Err(anyhow!(format!("This file is present on your system, and shouldn't be: {:?}", gmpublisher_dir))); }
		if gmpublisher_dir.is_dir() { return Ok(gmpublisher_dir); }

		fs::create_dir(&gmpublisher_dir)?;
		Ok(gmpublisher_dir)
	}

	pub(crate) fn find_gmod(&mut self) -> Option<()> {
		let gmod_path_setting = self.settings.gmod.as_ref();
		if gmod_path_setting.is_some() && gmod_path_setting.unwrap().is_dir() {
			self.gmod = Some(gmod_path_setting.unwrap().to_owned());
			if self.find_workshop_binaries().is_some() { return Some(()); }
		}

		let mut steam = steamlocate::SteamDir::locate()?;
		let gmod_app = steam.app(&4000)?;

		self.gmod = Some(gmod_app.path.to_owned());
		self.find_workshop_binaries();

		Some(())
	}

	pub(crate) fn find_workshop_binaries(&mut self) -> Option<()> {
		let gmod_path = self.gmod.as_ref().unwrap();

		#[cfg(target_os="windows")]
		let binaries: (PathBuf, PathBuf) = (gmod_path.join(PathBuf::from("gmad.exe")), gmod_path.join(PathBuf::from("gmpublish.exe")));
		#[cfg(target_os="linux")]
		let binaries: (PathBuf, PathBuf) = (gmod_path.join(PathBuf::from("gmad_linux")), gmod_path.join(PathBuf::from("gmpublish_linux")));
		#[cfg(target_os="macos")]
		let binaries: (PathBuf, PathBuf) = (gmod_path.join(PathBuf::from("gmad_osx")), gmod_path.join(PathBuf::from("gmpublish_osx")));

		if binaries.0.is_file() && binaries.1.is_file() {
			self.gmad = Some(binaries.0.to_owned());
			self.gmpublish = Some(binaries.1.to_owned());
			Some(())
		} else {
			None
		}
	}
}

use tauri::plugin::Plugin;
#[derive(Serialize)]
pub(crate) struct AppDataPlugin {
	app_data: AppData
}
impl AppDataPlugin {
	pub(crate) fn init(app_data: &AppData) -> AppDataPlugin {
		AppDataPlugin { app_data: app_data.clone() }
	}
}
impl Plugin for AppDataPlugin {
	fn init_script(&self) -> Option<String> {
		Some(include_str!("../../app/plugins/AppData.js").replace("{$_SETTINGS_$}", &serde_json::ser::to_string(&self.app_data).unwrap()))
	}
}
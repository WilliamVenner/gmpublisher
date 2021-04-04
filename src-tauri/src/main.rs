#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

pub const GMOD_APP_ID: AppId = AppId(4000);

use gma::extract::ExtractDestination;
use std::path::PathBuf;
use tauri::WebviewBuilderExt;

use lazy_static::lazy_static;
use steamworks::{AppId, PublishedFileId};

#[macro_use]
extern crate turbonone;

pub mod transactions;

#[macro_use]
pub mod util;
pub use util::*;

pub mod gma;
pub use gma::*;

pub mod base64_image;
pub use base64_image::Base64Image;

pub mod addon_size_analyzer;

pub mod octopus;
pub use octopus::steamworks::{users::SteamUser, workshop::WorkshopItem};
lazy_static! {
	pub static ref STEAMWORKS: octopus::Steamworks = octopus::Steamworks::init();
	pub static ref GMA: octopus::GMA = octopus::GMA::init();
	pub static ref GAME_ADDONS: octopus::GameAddons = octopus::GameAddons::init();
}
#[macro_export]
macro_rules! steamworks {
	() => {
		&*crate::STEAMWORKS
	};
}
#[macro_export]
macro_rules! downloads {
	() => {
		&*crate::octopus::steamworks::DOWNLOADS
	};
}
#[macro_export]
macro_rules! gma {
	() => {
		&*crate::GMA
	};
}
#[macro_export]
macro_rules! game_addons {
	() => {
		&*crate::GAME_ADDONS
	};
}

pub mod appdata;
pub use appdata::AppData;
lazy_static! {
	pub static ref APP_DATA: AppData = AppData::init();
}
#[macro_export]
macro_rules! app_data {
	() => {
		&*crate::APP_DATA
	};
}

#[macro_use]
mod webview;
pub use webview::WEBVIEW;
#[macro_export]
macro_rules! webview {
	() => {
		&*crate::WEBVIEW
	};
}
#[macro_export]
macro_rules! webview_emit {
	( $event:expr, $data:expr ) => {
		crate::webview!().emit($event, Some($data)).unwrap()
	};

	( $event:expr ) => {
		crate::webview!().emit($event, turbonone!()).unwrap()
	};
}

fn main() {
	lazy_static::initialize(&STEAMWORKS);

	std::thread::spawn(move || {
		steamworks!().client_wait();
		downloads!().download(vec![
			PublishedFileId(2439258443),
			PublishedFileId(2439806441),
			PublishedFileId(2440241937),
		]);
	});

	std::thread::spawn(move || {
		steamworks!().client_wait();
		game_addons!().discover_addons();
		println!("{:#?}", game_addons!());
	});

	std::thread::spawn(move || {
		std::thread::sleep(std::time::Duration::from_secs(2));
		let now = std::time::Instant::now();

		let src_path = PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\lw_bmw_pack"#);
		let dest_path = PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\lw_bmw_pack_sneed.gma"#);

		println!("=================================================================");

		GMAFile::write(
			src_path,
			dest_path.clone(),
			&GMAMetadata::Standard {
				title: "LW BMW Pack Test".to_string(),
				addon_type: "addon".to_string(),
				tags: vec!["gmpublisher".to_string()],
				ignore: vec!["test".to_string()],
			},
		)
		.unwrap();

		println!("{:?}ms", now.elapsed().as_millis());

		let now = std::time::Instant::now();

		let transaction = transaction!();
		GMAFile::open(dest_path).unwrap().extract(ExtractDestination::Temp, transaction).unwrap();

		println!("{:?}ms", now.elapsed().as_millis());
	});

	tauri::AppBuilder::default()
		.create_webview("gmpublisher".to_string(), tauri::WindowUrl::App, |args| {
			let settings = APP_DATA.settings.read();
			Ok({
				#[cfg(not(debug_assertions))]
				{
					args.title(format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")))
						.resizable(true)
						.width(settings.window_size.0)
						.height(settings.window_size.1)
						.maximized(settings.window_maximized)
				}

				#[cfg(debug_assertions)]
				{
					args.title(format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")))
						.resizable(true)
						.width(settings.window_size.0)
						.height(settings.window_size.1)
				}
			})
		})
		.unwrap()
		.setup(|mgr| crate::WEBVIEW.init(mgr))
		.plugin(appdata::Plugin)
		.invoke_handler(tauri::generate_handler![transactions::cancel_transaction, appdata::update_settings])
		.build(tauri::generate_context!())
		.run();
}

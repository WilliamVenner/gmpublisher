#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use lazy_static::lazy_static;
use tauri::WebviewBuilderExt;

pub(crate) mod util;
pub(crate) use util::*;

pub(crate) mod gma;
pub(crate) use gma::*;

pub(crate) mod base64_image;
pub(crate) use base64_image::Base64Image;

pub(crate) mod octopus;
lazy_static! {
	static ref STEAMWORKS: octopus::Steamworks = octopus::Steamworks::init();
	static ref GMA: octopus::GMA = octopus::GMA::init();
}

pub(crate) mod appdata;
pub(crate) use appdata::AppData;
lazy_static! {
	pub(crate) static ref APP_DATA: AppData = AppData::init();
}

pub(crate) use octopus::steamworks::WorkshopItem;

fn main() {
	lazy_static::initialize(&APP_DATA);
	lazy_static::initialize(&STEAMWORKS);
	lazy_static::initialize(&GMA);

	tauri::AppBuilder::default()
		.create_webview("gmpublisher".to_string(), tauri::WindowUrl::App, |args| {
			let settings = APP_DATA.settings.read();
			Ok(
				args.title(format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")))
				.resizable(true)
				.width(settings.window_size.0)
				.height(settings.window_size.1)
				//.maximized(settings.window_maximized)
			)
		})
		.unwrap()
		.plugin(appdata::Plugin)
		.invoke_handler(tauri::generate_handler![])
		.build(tauri::generate_context!())
		.run();
}

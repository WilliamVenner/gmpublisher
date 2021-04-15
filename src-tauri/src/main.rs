#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use tauri::{Manager, Attributes};

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate turbonone;

#[macro_use]
pub mod globals;
pub use globals::*;

#[macro_use]
pub mod util;
pub use util::*;

pub mod base64_image;
pub use base64_image::Base64Image;

pub mod appdata;
pub use appdata::AppData;

pub mod game_addons;
pub use game_addons::GameAddons;

pub mod addon_size_analyzer;
pub use addon_size_analyzer::AddonSizeAnalyzer;

pub mod gma;
pub use gma::{GMAError, GMAFile, GMAMetadata};

pub mod steam;
pub use steam::workshop::WorkshopItem;

pub mod octopus;
pub use octopus::*;

pub mod transactions;
pub use transactions::Transaction;

pub mod search;
pub mod content_generator;
pub mod webview;

mod commands;

fn main() {
	globals::init_globals();

	tauri::Builder::default()

		.create_window("gmpublisher".to_string(), tauri::WindowUrl::default(), |args| {
			let settings = APP_DATA.settings.read();
			args.title(format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")))
				.maximized(!cfg!(debug_assertions) && settings.window_maximized)
				.resizable(true)
				.width(settings.window_size.0)
				.height(settings.window_size.1)
				.min_width(800.)
				.min_height(600.)
		})

		.setup(|app| {
			let window = app.get_window(&"gmpublisher".to_string()).unwrap();
			webview!().init(window);
			Ok(())
		})

		.plugin(appdata::Plugin)

		.invoke_handler(commands::invoke_handler())

		.run(tauri::generate_context!())
		.unwrap();
}

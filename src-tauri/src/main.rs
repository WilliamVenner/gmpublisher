#![cfg_attr(
	all(not(debug_assertions), target_os = "windows"),
	windows_subsystem = "windows"
)]

use tauri::{AppBuilder, Webview};
extern crate webview_official;

mod show;
mod settings;

mod appdata;
use appdata::AppData;
use appdata::AppDataPlugin;

mod commands;

mod workshop;
use workshop::Workshop;

mod base64_image;
pub(crate) use base64_image::Base64Image;

mod game_addons;
use game_addons::GameAddons;

pub(crate) mod lib;

fn main() {
	let workshop = match Workshop::init() {
		Ok(workshop) => workshop,
		Err(error) => return show::panic(format!("Couldn't initialize the Steam API! Is Steam running?\nError: {:#?}", error))
	};

	// TODO use steam api to get gmod dir instead of steamlocate

	let app_data = match AppData::init(workshop.get_user()) {
		Ok(app_data) => app_data,
		Err(error) => return show::panic(format!("{:#?}", error))
	};

	let game_addons = GameAddons::init();
	
	let window_size = app_data.settings.window_size.clone();
	let mut first_setup = true;
	let setup = move |webview: &mut Webview, _: String| {
		webview.set_title(&format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")));

		if first_setup {
			webview.set_size(500, 500, webview_official::SizeHint::MIN);
			webview.set_size(std::cmp::max(window_size.0, 500), std::cmp::max(window_size.1, 500), webview_official::SizeHint::NONE);

			drop(window_size);
			first_setup = false;
		}
	};

	AppBuilder::new()
		.setup(setup)
		.plugin(AppDataPlugin::init(&app_data))
		.invoke_handler(commands::invoke_handler(app_data, workshop, game_addons))
		.build().run();
}

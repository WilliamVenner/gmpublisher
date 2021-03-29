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

fn main() {
	lazy_static::initialize(&STEAMWORKS);
	lazy_static::initialize(&GMA);

	tauri::AppBuilder::default()
		.create_webview("gmpublisher".to_string(), tauri::WindowUrl::App, |args| {
			Ok(args.title("gmpublisher".to_string()).resizable(true).width(800.).height(600.))
		})
		.unwrap()
		.invoke_handler(tauri::generate_handler![])
		.build(tauri::generate_context!())
		.run();
}

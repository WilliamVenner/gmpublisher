#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::collections::HashMap;

use lazy_static::lazy_static;
use steamworks::PublishedFileId;
use tauri::WebviewBuilderExt;

pub(crate) mod base64_image;
pub(crate) use base64_image::Base64Image;

pub(crate) mod octopus;
use octopus::Steamworks;

lazy_static! {
	static ref STEAMWORKS: Steamworks = Steamworks::init();
}

#[tauri::command]
fn new_cmd_test(_test: bool) -> Result<HashMap<&'static str, &'static str>, &'static str> {
	let mut hashmap = HashMap::new();
	hashmap.insert("don\\\\'t", "don\\'t");
	Ok(hashmap)
}

fn main() {
	lazy_static::initialize(&STEAMWORKS);

	STEAMWORKS.client_wait();
	STEAMWORKS.fetch_workshop_item_async(PublishedFileId(2328355906), |item| println!("{:#?}", item));
	STEAMWORKS.fetch_workshop_items_async(vec![PublishedFileId(2328355906), PublishedFileId(1952750559)], |items| println!("{:#?}", items));

	tauri::AppBuilder::default()
		.create_webview("gmpublisher".to_string(), tauri::WindowUrl::App, |args| {
			Ok(args.title("gmpublisher".to_string()).resizable(true).width(800.).height(600.))
		})
		.unwrap()
		.invoke_handler(tauri::generate_handler![new_cmd_test])
		.build(tauri::generate_context!())
		.run();
}

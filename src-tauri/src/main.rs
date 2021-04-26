use tauri::{Attributes, Manager};

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

#[macro_use]
pub mod transactions;
pub use transactions::Transaction;

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

pub mod bundles;
pub mod search;
pub mod webview;

mod commands;
mod cli;

fn write_tauri_settings() -> Option<()> {
	// silly bypass for the pointless permissions system

	use serde_json::Value as JsonValue;
	use std::{
		collections::HashMap,
		fs::{self, File},
		io::{BufReader, BufWriter},
	};

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

#[cfg(debug_assertions)]
fn deadlock_watchdog() {
	std::thread::spawn(move || loop {
		sleep!(10);

		let deadlocks = parking_lot::deadlock::check_deadlock();
		if deadlocks.is_empty() {
			continue;
		}

		println!("{} deadlocks detected", deadlocks.len());
		for (i, threads) in deadlocks.iter().enumerate() {
			println!("Deadlock #{}", i);
			for t in threads {
				println!("Thread Id {:#?}", t.thread_id());
				println!("{:#?}", t.backtrace());
			}
		}
	});
}

fn main() {
	#[cfg(debug_assertions)]
	if cli::stdin() {
		return;
	}

	println!("gmpublisher v{}", env!("CARGO_PKG_VERSION"));

	#[cfg(debug_assertions)]
	deadlock_watchdog();

	//ignore! { write_tauri_settings() };

	globals::init_globals();

	println!("Starting GUI...");

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

	println!("Goodbye!");
}

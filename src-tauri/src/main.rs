#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use tauri::Manager;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate turbonone;

#[macro_use]
mod logging;
pub use logging::*;

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

pub mod content_generator;
pub mod search;
pub mod webview;

mod cli;
mod commands;

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
	// https://github.com/WilliamVenner/gmpublisher/issues/210
	if cfg!(target_os = "linux") {
		std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
	}

	std::panic::set_hook(Box::new(|panic| logging::panic(panic)));

	rayon::ThreadPoolBuilder::new().num_threads(*crate::NUM_THREADS).build_global().unwrap();

	if cli::stdin() {
		return;
	}

	println!("gmpublisher v{}", env!("CARGO_PKG_VERSION"));

	#[cfg(debug_assertions)]
	deadlock_watchdog();

	//ignore! { app_data::write_tauri_settings() };

	globals::init_globals();

	println!("Starting GUI...");

	tauri::Builder::default()
		.setup(|app| {
			let settings = APP_DATA.settings.read();

			let window = app.get_window("gmpublisher").unwrap();

			window.set_title(&format!("gmpublisher v{}", env!("CARGO_PKG_VERSION"))).ok();

			window.set_size(tauri::Size::Logical(tauri::LogicalSize {
				width: settings.window_size.0.max(800.),
				height: settings.window_size.1.max(600.)
			})).ok();

			if !cfg!(debug_assertions) && settings.window_maximized {
				window.maximize().ok();
			}

			webview!().init(window);

			Ok(())
		})
		.plugin(webview::ErrorReporter)
		.plugin(appdata::Plugin)
		.invoke_handler(commands::invoke_handler())
		.run(tauri::generate_context!())
		.unwrap();

	println!("Goodbye!");
}

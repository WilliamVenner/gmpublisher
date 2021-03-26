#![cfg_attr(
	all(not(debug_assertions), target_os = "windows"),
	windows_subsystem = "windows"
)]

use addon_size_analyzer::AddonSizeAnalyzer;
use downloader::WorkshopDownloader;
use tauri::{AppBuilder, Webview};
extern crate webview_official;

mod util;
pub(crate) use util::*;

mod settings;
mod show;

mod appdata;
use appdata::AppData;
use appdata::AppDataPlugin;

mod commands;

mod workshop;
use transactions::Transactions;
use workshop::Workshop;

mod base64_image;
pub(crate) use base64_image::Base64Image;

mod game_addons;
use game_addons::GameAddons;

mod addon_size_analyzer;

mod downloader;

mod publish;

#[macro_use]
mod transactions;

pub(crate) mod gma;

use lazy_static::lazy_static;
lazy_static! {
	pub(crate) static ref WORKSHOP: RwLockDebug<Workshop> =
		RwLockDebug::new(match Workshop::init() {
			Ok(workshop) => workshop,
			Err(error) => {
				show::panic(format!(
					"Couldn't initialize the Steam API! Is Steam running?\nError: {:#?}",
					error
				));
				panic!();
			}
		});
	pub(crate) static ref WORKSHOP_DOWNLOADER: RwLockDebug<WorkshopDownloader> =
		RwLockDebug::new(WorkshopDownloader::init());
	pub(crate) static ref APP_DATA: RwLockDebug<AppData> =
		RwLockDebug::new(match AppData::init(WORKSHOP.read().unwrap().get_user()) {
			Ok(app_data) => app_data,
			Err(error) => {
				show::panic(format!("{:#?}", error));
				panic!();
			}
		});
	pub(crate) static ref GAME_ADDONS: RwLockDebug<GameAddons> =
		RwLockDebug::new(GameAddons::init());
	pub(crate) static ref TRANSACTIONS: RwLockDebug<Transactions> =
		RwLockDebug::new(Transactions::init());
	pub(crate) static ref ADDON_SIZE_ANALYZER: AddonSizeAnalyzer = AddonSizeAnalyzer::init();

	pub(crate) static ref NUM_CPUS: usize = num_cpus::get();
}

fn main() {
	// TODO use steam api to get gmod dir instead of steamlocate

	#[cfg(debug_assertions)]
	{
		println!("Num CPUs: {}", *NUM_CPUS);
		println!("Rayon Threads: {}", rayon::current_num_threads());
	}

	let window_size = APP_DATA.read().unwrap().settings.window_size.clone();
	let mut first_setup = true;
	let setup = move |webview: &mut Webview, _: String| {
		webview.set_title(&format!("gmpublisher v{}", env!("CARGO_PKG_VERSION")));

		if first_setup {
			webview.set_size(500, 500, webview_official::SizeHint::MIN);
			webview.set_size(
				std::cmp::max(window_size.0, 500),
				std::cmp::max(window_size.1, 500),
				webview_official::SizeHint::NONE,
			);

			drop(window_size);
			first_setup = false;
		}
	};

	AppBuilder::new()
		.setup(setup)
		.plugin(AppDataPlugin::init())
		.invoke_handler(commands::invoke_handler())
		.build()
		.run();
}

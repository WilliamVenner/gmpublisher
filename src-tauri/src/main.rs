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
pub mod webview;

mod commands;

fn main() {
	globals::init_globals();

	std::thread::spawn(move || {
		steam!().client_wait();
		downloads!().download(vec![
			steamworks::PublishedFileId(2439258443),
			steamworks::PublishedFileId(2439806441),
			steamworks::PublishedFileId(2440241937),
		]);
	});

	std::thread::spawn(move || {
		steam!().client_wait();
		lazy_static::initialize(&GAME_ADDONS);
		let _now = std::time::Instant::now();
		game_addons!().discover_addons();
	});

	std::thread::spawn(move || {
		std::thread::sleep(std::time::Duration::from_secs(2));
		let now = std::time::Instant::now();

		let src_path = std::path::PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\jerma_sus_playermodel_npc_2349955985"#);
		let dest_path = std::path::PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\publish_test\sneed.gma"#);

		println!("=================================================================");

		GMAFile::write(
			src_path,
			dest_path.clone(),
			&GMAMetadata::Standard {
				title: "gmpublisher".to_string(),
				addon_type: "addon".to_string(),
				tags: vec!["gmpublisher".to_string()],
				ignore: vec![],
			},
		)
		.unwrap();

		println!("{:?}ms", now.elapsed().as_millis());

		let now = std::time::Instant::now();

		let transaction = transaction!();
		GMAFile::open(&dest_path)
			.unwrap()
			.extract(gma::ExtractDestination::Temp, transaction)
			.unwrap();

		println!("{:?}ms", now.elapsed().as_millis());

		//println!("{:#?}", steam!().publish(dest_path.with_file_name(""), "gmpublisher".to_string(), std::path::PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\publish_test\imdeadbru.gif"#)));
		/*println!("{:#?}", steam!().update(WorkshopUpdateType::Update(WorkshopUpdateDetails {
			id: steamworks::PublishedFileId(2446913281),
			path: dest_path.with_file_name(""),
			preview: None,
			changes: None,
		})));*/
	});

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

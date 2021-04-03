#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

pub const GMOD_APP_ID: AppId = AppId(4000);

use std::{cell::RefCell, fs::File, hash::Hash, mem::MaybeUninit, path::PathBuf, sync::atomic::AtomicBool};
use gma::{extract::ExtractDestination, write::GMACreationData};
use tauri::{ApplicationExt, WebviewBuilderExt, WebviewDispatcher, WebviewManager};

use lazy_static::lazy_static;
use steamworks::{AppId, PublishedFileId};

#[macro_use] extern crate turbonone;

pub mod transactions;

#[macro_use]
pub mod util;
pub use util::*;

pub mod gma;
pub use gma::*;

pub mod base64_image;
pub use base64_image::Base64Image;

pub mod octopus;
pub use octopus::steamworks::WorkshopItem;
lazy_static! {
	pub static ref STEAMWORKS: octopus::Steamworks = octopus::Steamworks::init();
	pub static ref GMA: octopus::GMA = octopus::GMA::init();
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

pub struct WrappedWebview<Application: ApplicationExt + 'static> {
	pub setup: AtomicBool,
	pub inner: RefCell<MaybeUninit<WebviewDispatcher<Application::Dispatcher>>>,
}
unsafe impl<Application: ApplicationExt + 'static> Send for WrappedWebview<Application> {}
unsafe impl<Application: ApplicationExt + 'static> Sync for WrappedWebview<Application> {}
impl<Application: ApplicationExt + 'static> WrappedWebview<Application> {
	fn pending() -> Self {
		Self {
			setup: AtomicBool::new(false),
			inner: RefCell::new(MaybeUninit::uninit()),
		}
	}

	fn init(&self, webview: WebviewManager<Application>) {
		if !self.setup.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
			unsafe {
				self.inner.borrow_mut().as_mut_ptr().write(webview.current_webview().unwrap());
			}
		}
	}
}
lazy_static! {
	pub static ref WEBVIEW: WrappedWebview<tauri::flavors::Wry> = WrappedWebview::pending();
}
#[macro_export]
macro_rules! webview {
	() => {
		unsafe { &*crate::WEBVIEW.inner.borrow().as_ptr() }
	};
}
#[macro_export]
macro_rules! webview_emit_safe {
	( $event:expr, $data:expr ) => {
		if crate::WEBVIEW.setup.load(std::sync::atomic::Ordering::Acquire) {
			ignore! { webview_emit!($event, $data) };
		}
	};

	( $event:expr ) => {
		if crate::WEBVIEW.setup.load(std::sync::atomic::Ordering::Acquire) {
			ignore! { webview_emit!($event) };
		}
	};
}
#[macro_export]
macro_rules! webview_emit {
	( $event:expr, $data:expr ) => {
		webview!().emit($event, Some($data))
	};

	( $event:expr ) => {
		webview!().emit($event, turbonone!())
	};
}

fn main() {
	lazy_static::initialize(&APP_DATA);
	lazy_static::initialize(&STEAMWORKS);
	lazy_static::initialize(&GMA);

	std::thread::spawn(move || {
		std::thread::sleep(std::time::Duration::from_secs(2));
		let now = std::time::Instant::now();

		let src_path = PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\lw_bmw_pack"#);
		let dest_path = PathBuf::from(r#"C:\Users\billy\AppData\Local\Temp\gmpublisher\lw_bmw_pack_sneed.gma"#);

		println!("=================================================================");

		let mut gma = GMAFile::write(src_path, dest_path.clone(), GMACreationData {
		    title: "LW BMW Pack Test".to_string(),
		    addon_type: "addon".to_string(),
		    tags: vec!["gmpublisher".to_string()],
		    ignore: vec!["test".to_string()],
		});

		println!("{:?}ms", now.elapsed().as_millis());

		let now = std::time::Instant::now();

		GMAFile::open(dest_path).unwrap().extract(ExtractDestination::Temp, &transaction!()).unwrap();

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

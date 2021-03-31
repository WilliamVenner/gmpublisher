#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use std::{cell::RefCell, mem::MaybeUninit, sync::atomic::AtomicBool};

use lazy_static::lazy_static;
use tauri::{ApplicationExt, WebviewBuilderExt, WebviewDispatcher, WebviewManager};

//pub(crate) mod transactions;

pub(crate) mod util;
pub(crate) use util::*;

pub(crate) mod gma;
pub(crate) use gma::*;

pub(crate) mod base64_image;
pub(crate) use base64_image::Base64Image;

pub(crate) mod octopus;
pub(crate) use octopus::steamworks::WorkshopItem;
lazy_static! {
	pub(crate) static ref STEAMWORKS: octopus::Steamworks = octopus::Steamworks::init();
	pub(crate) static ref GMA: octopus::GMA = octopus::GMA::init();
}
#[macro_export]
macro_rules! steamworks {
	() => {
		&*crate::STEAMWORKS
	};
}
#[macro_export]
macro_rules! gma {
	() => {
		&*crate::GMA
	};
}

pub(crate) mod appdata;
pub(crate) use appdata::AppData;
lazy_static! {
	pub(crate) static ref APP_DATA: AppData = AppData::init();
}
#[macro_export]
macro_rules! app_data {
	() => {
		&*crate::APP_DATA
	};
}

pub(crate) struct WrappedWebview<Application: ApplicationExt + 'static> {
	setup: AtomicBool,
	pub(crate) inner: RefCell<MaybeUninit<WebviewDispatcher<Application::Dispatcher>>>,
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
		use std::sync::atomic::Ordering;
		if !self.setup.load(Ordering::Acquire) {
			self.setup.store(true, Ordering::Release);
			unsafe {
				self.inner.borrow_mut().as_mut_ptr().write(webview.current_webview().unwrap());
			}
		}
	}
}
lazy_static! {
	pub(crate) static ref WEBVIEW: WrappedWebview<tauri::flavors::Wry> = WrappedWebview::pending();
}
#[macro_export]
macro_rules! webview {
	() => {
		unsafe { &*crate::WEBVIEW.inner.borrow().as_ptr() }
	};
}

fn main() {
	lazy_static::initialize(&APP_DATA);
	lazy_static::initialize(&STEAMWORKS);
	lazy_static::initialize(&GMA);

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
		.invoke_handler(tauri::generate_handler![])
		.build(tauri::generate_context!())
		.run();
}

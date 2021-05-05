use std::{cell::RefCell, sync::atomic::AtomicBool};

use crossbeam::channel::Sender;
use serde::Serialize;
use tauri::{
	api::assets::EmbeddedAssets,
	runtime::{flavors::wry::Wry, Args},
	Window,
};

use crate::{GMAFile, WorkshopItem};

type Params = Args<String, String, EmbeddedAssets, Wry>;

pub struct WrappedWebview {
	pub window: RefCell<Option<Window<Params>>>,
	pending: AtomicBool,
	tx: Sender<Window<Params>>,
}
unsafe impl Send for WrappedWebview {}
unsafe impl Sync for WrappedWebview {}
impl WrappedWebview {
	pub fn pending() -> Self {
		Self {
			window: RefCell::new(None),
			tx: WrappedWebview::channel(),
			pending: AtomicBool::new(true),
		}
	}

	fn channel() -> Sender<Window<Params>> {
		let (tx, rx) = crossbeam::channel::bounded(1);

		std::thread::spawn(move || {
			let window = rx.recv().unwrap();
			*webview!().window.borrow_mut() = Some(window);
			webview!().pending.store(false, std::sync::atomic::Ordering::Release);
		});

		tx
	}

	pub fn init(&self, window: Window<Params>) {
		ignore! { self.tx.send(window) };
	}

	pub fn emit<D: Serialize + Send + 'static>(&self, event: &'static str, payload: Option<D>) {
		while self.pending.load(std::sync::atomic::Ordering::Acquire) {
			sleep_ms!(50);
		}
		ignore! { self.window.borrow().as_ref().unwrap().emit(&event.to_string(), payload) };
	}
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Addon {
	Installed(GMAFile),
	Workshop(WorkshopItem),
}
impl Addon {
	#[inline(always)]
	pub fn installed(&self) -> &GMAFile {
		match self {
			Addon::Installed(addon) => addon,
			Addon::Workshop(_) => unreachable!(),
		}
	}
	#[inline(always)]
	pub fn workshop(&self) -> &WorkshopItem {
		match self {
			Addon::Installed(_) => unreachable!(),
			Addon::Workshop(addon) => addon,
		}
	}
}
impl Into<GMAFile> for Addon {
	fn into(self) -> GMAFile {
		match self {
			Addon::Installed(addon) => addon,
			Addon::Workshop(_) => unreachable!(),
		}
	}
}
impl Into<WorkshopItem> for Addon {
	fn into(self) -> WorkshopItem {
		match self {
			Addon::Installed(_) => unreachable!(),
			Addon::Workshop(addon) => addon,
		}
	}
}
impl From<GMAFile> for Addon {
	fn from(installed: GMAFile) -> Self {
		Addon::Installed(installed)
	}
}
impl From<WorkshopItem> for Addon {
	fn from(item: WorkshopItem) -> Self {
		Addon::Workshop(item)
	}
}
impl PartialOrd for Addon {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match self {
			Addon::Installed(addon) => match other {
				Addon::Installed(other) => addon.partial_cmp(other),
				_ => unreachable!(),
			},
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.partial_cmp(other),
				_ => unreachable!(),
			},
		}
	}
}
impl Ord for Addon {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match self {
			Addon::Installed(addon) => match other {
				Addon::Installed(other) => addon.cmp(other),
				_ => unreachable!(),
			},
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.cmp(other),
				_ => unreachable!(),
			},
		}
	}
}
impl PartialEq for Addon {
	fn eq(&self, other: &Self) -> bool {
		match self {
			Addon::Installed(addon) => match other {
				Addon::Installed(other) => addon.eq(other),
				_ => unreachable!(),
			},
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.eq(other),
				_ => unreachable!(),
			},
		}
	}
}
impl Eq for Addon {}

pub struct ErrorReporter;
impl<M: tauri::Params + 'static> tauri::plugin::Plugin<M> for ErrorReporter {
	fn initialization_script(&self) -> Option<String> {
		Some(include_str!("../../app/plugins/ErrorReporter.js").replacen(
			"{$_DEBUG_MODE_$}",
			if cfg!(debug_assertions) { "true" } else { "false" },
			1,
		))
	}

	fn name(&self) -> &'static str {
		"ErrorReporter"
	}
}

#[tauri::command]
pub fn js_error(message: String, stack: String) {
	eprintln!("\n=== JavaScript Error! ===");
	eprintln!("{}", message);
	eprintln!("{}\n", stack);
}

#[tauri::command]
pub fn error(message: String) {
	eprintln!("[WebView] [ERROR] {}", message);
}

#[tauri::command]
pub fn info(message: String) {
	eprintln!("[WebView] [INFO] {}", message);
}

#[tauri::command]
pub fn warn(message: String) {
	eprintln!("[WebView] [WARN] {}", message);
}

static mut RELOADED: AtomicBool = AtomicBool::new(false);
#[tauri::command]
pub fn reloaded() {
	if unsafe { RELOADED.fetch_or(true, std::sync::atomic::Ordering::SeqCst) } {
		crate::commands::free_caches();
	}
}

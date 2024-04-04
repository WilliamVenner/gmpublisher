use std::{cell::RefCell, mem::MaybeUninit, sync::atomic::AtomicBool};

use crossbeam::channel::Sender;
use serde::Serialize;
use tauri::{Window, Wry};

use crate::{GMAFile, WorkshopItem};

pub struct WrappedWebview {
	pub window: RefCell<MaybeUninit<Window<Wry>>>,
	pending: AtomicBool,
	tx: Sender<Window<Wry>>,
}
unsafe impl Send for WrappedWebview {}
unsafe impl Sync for WrappedWebview {}
impl WrappedWebview {
	pub fn pending() -> Self {
		Self {
			window: RefCell::new(MaybeUninit::uninit()),
			tx: WrappedWebview::channel(),
			pending: AtomicBool::new(true),
		}
	}

	fn channel() -> Sender<Window<Wry>> {
		let (tx, rx) = crossbeam::channel::bounded(1);

		std::thread::spawn(move || {
			let window = rx.recv().unwrap();
			unsafe { webview!().window.borrow_mut().as_mut_ptr().write(window) };
			webview!().pending.store(false, std::sync::atomic::Ordering::Release);
		});

		tx
	}

	pub fn init(&self, window: Window<Wry>) {
		self.tx.send(window).unwrap();
	}

	pub fn emit<D: Serialize + Send + 'static>(&self, event: &'static str, payload: Option<D>) {
		ignore! { self.window().emit(event, &payload) };
	}

	pub fn window(&self) -> &Window<Wry> {
		while self.pending.load(std::sync::atomic::Ordering::Relaxed) {
			sleep_ms!(50);
		}
		unsafe { &*self.window.borrow().as_ptr() }
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
impl From<Addon> for GMAFile {
	fn from(val: Addon) -> Self {
		match val {
			Addon::Installed(addon) => addon,
			Addon::Workshop(_) => unreachable!(),
		}
	}
}
impl From<Addon> for WorkshopItem {
	fn from(val: Addon) -> Self {
		match val {
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
impl<R: tauri::Runtime> tauri::plugin::Plugin<R> for ErrorReporter {
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
	println!("[WebView] [INFO] {}", message);
}

#[tauri::command]
pub fn warn(message: String) {
	println!("[WebView] [WARN] {}", message);
}

static mut RELOADED: AtomicBool = AtomicBool::new(false);
#[tauri::command]
pub fn reloaded() {
	if unsafe { RELOADED.fetch_or(true, std::sync::atomic::Ordering::SeqCst) } {
		crate::commands::free_caches();
	}
}

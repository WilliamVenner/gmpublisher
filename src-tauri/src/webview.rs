use std::{cell::RefCell, mem::MaybeUninit, sync::{atomic::AtomicBool, mpsc::{self, Receiver, SendError, Sender}}};

use atomic_refcell::AtomicRefCell;
use lazy_static::lazy_static;
use serde::Serialize;
use tauri::{ApplicationExt, WebviewDispatcher, WebviewManager};


lazy_static! {
	pub static ref WEBVIEW: WrappedWebview<tauri::flavors::Wry> = WrappedWebview::pending();
}

pub type WebviewEmit = (&'static str, Option<Box<dyn erased_serde::Serialize + Send>>);

pub struct WrappedWebview<Application: ApplicationExt + 'static> {
	pub tx: Sender<WebviewEmit>,
	tx_webview: Sender<WebviewDispatcher<Application::Dispatcher>>
}
unsafe impl<Application: ApplicationExt + 'static> Send for WrappedWebview<Application> {}
unsafe impl<Application: ApplicationExt + 'static> Sync for WrappedWebview<Application> {}
impl<Application: ApplicationExt + 'static> WrappedWebview<Application> {
	pub fn pending() -> Self {
		let (tx_webview, tx) = WrappedWebview::<Application>::channel();
		Self { tx_webview, tx }
	}

	fn channel() -> (Sender<WebviewDispatcher<Application::Dispatcher>>, Sender<WebviewEmit>) {
		let (tx, rx): (Sender<WebviewEmit>, Receiver<WebviewEmit>) = mpsc::channel();
		let (tx_webview, rx_webview) = mpsc::channel();
		std::thread::spawn(move || {
			let webview: WebviewDispatcher<Application::Dispatcher> = rx_webview.recv().unwrap();
			loop {
				let (event, payload) = rx.recv().unwrap();
				webview.emit(event, payload);
			}
		});

		(tx_webview, tx)
	}

	pub fn init(&self, webview: WebviewManager<Application>) {
		ignore! { self.tx_webview.send(webview.current_webview().unwrap()) };
	}

	pub fn emit<D: Serialize + Send + 'static>(&self, event: &'static str, payload: Option<D>) -> Result<(), SendError<(&'static str, Option<Box<dyn erased_serde::Serialize + Send>>)>> {
		self.tx.send((event, match payload {
			Some(payload) => Some(Box::new(payload)),
			None => None
		}))
	}
}
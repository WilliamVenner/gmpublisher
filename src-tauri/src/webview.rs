use crossbeam::channel::{SendError, Sender};
use serde::Serialize;
use tauri::{Params, Window, runtime::Manager};

pub type WebviewEmit = (&'static str, Option<Box<dyn erased_serde::Serialize + Send>>);

pub struct WrappedWebview<M: Params<Event = String, Label = String> + Send + 'static> {
	pub tx_emit: Sender<WebviewEmit>,
	tx_window: Sender<Window<M>>,
}
unsafe impl<M: Params<Event = String, Label = String> + Send + 'static> Send for WrappedWebview<M> {}
unsafe impl<M: Params<Event = String, Label = String> + Send + 'static> Sync for WrappedWebview<M> {}
impl<M: Params<Event = String, Label = String> + Send + 'static> WrappedWebview<M> {
	pub fn pending() -> Self {
		let (tx_window, tx_emit) = WrappedWebview::<M>::channel();
		Self { tx_window, tx_emit }
	}

	fn channel() -> (Sender<Window<M>>, Sender<WebviewEmit>) {
		let (tx, rx) = crossbeam::channel::bounded::<WebviewEmit>(1);
		let (tx_window, rx_window) = crossbeam::channel::unbounded::<Window<M>>();
		std::thread::spawn(move || {
			let window = rx_window.recv().unwrap();
			loop {
				let (event, payload) = rx.recv().unwrap();
				ignore! { window.emit(&event.to_string(), payload) };
			}
		});

		(tx_window, tx)
	}

	pub fn init(&self, window: Window<M>) {
		ignore! { self.tx_window.send(window) };
	}

	pub fn emit<D: Serialize + Send + 'static>(
		&self,
		event: &'static str,
		payload: Option<D>,
	) -> Result<(), SendError<WebviewEmit>> {
		self.tx_emit.send((
			event,
			match payload {
				Some(payload) => Some(Box::new(payload)),
				None => None,
			},
		))
	}
}

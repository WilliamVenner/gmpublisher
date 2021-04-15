use crossbeam::channel::{SendError, Sender};
use serde::Serialize;
use tauri::{Params, Window};

use crate::{GMAFile, WorkshopItem};

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
            Addon::Workshop(_) => unreachable!()
        }
	}
	#[inline(always)]
	pub fn workshop(&self) -> &WorkshopItem {
		match self {
            Addon::Installed(_) => unreachable!(),
            Addon::Workshop(addon) => addon
        }
	}
}
impl Into<GMAFile> for Addon {
    fn into(self) -> GMAFile {
        match self {
            Addon::Installed(addon) => addon,
            Addon::Workshop(_) => unreachable!()
        }
    }
}
impl Into<WorkshopItem> for Addon {
    fn into(self) -> WorkshopItem {
        match self {
            Addon::Installed(_) => unreachable!(),
            Addon::Workshop(addon) => addon
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
				_ => unreachable!()
			}
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.partial_cmp(other),
				_ => unreachable!()
			}
		}
    }
}
impl Ord for Addon {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self {
			Addon::Installed(addon) => match other {
				Addon::Installed(other) => addon.cmp(other),
				_ => unreachable!()
			}
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.cmp(other),
				_ => unreachable!()
			}
		}
    }
}
impl PartialEq for Addon {
    fn eq(&self, other: &Self) -> bool {
        match self {
			Addon::Installed(addon) => match other {
				Addon::Installed(other) => addon.eq(other),
				_ => unreachable!()
			}
			Addon::Workshop(addon) => match other {
				Addon::Workshop(other) => addon.eq(other),
				_ => unreachable!()
			}
		}
    }
}
impl Eq for Addon {}

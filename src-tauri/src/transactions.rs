use std::{cell::Cell, collections::{HashMap, hash_map::RandomState}, sync::{Arc, Mutex, Weak, atomic::{AtomicUsize, Ordering}}};
use tauri::{Webview, WebviewMut};

static TRANSACTION_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Transaction {
	pub(crate) id: usize,
	progress: f32,
	cancel_callbacks: Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
	webview: WebviewMut
}

unsafe impl Sync for Transaction {}
unsafe impl Send for Transaction {}

impl Transaction {
	pub(crate) fn connect_cancel(&mut self, f: Box<dyn FnOnce() + Send + Sync + 'static>) {
		self.cancel_callbacks.push(f);
	}

	pub(crate) fn cancel(mut self) {
		crate::TRANSACTIONS.write().unwrap().map.remove(&self.id);

		tauri::event::emit(&mut self.webview, "transactionCancelled", Some(self.id)).ok();

		for callback in self.cancel_callbacks { (callback)(); }
	}

	pub(crate) fn progress(&mut self, progress: f32) {
		self.progress = progress;
		tauri::event::emit(&mut self.webview, "transactionProgress", Some((self.id, progress))).ok();
	}

	pub(crate) fn progress_incr(&mut self, incr: f32) {
		self.progress(self.progress + incr);
	}
}

pub(crate) struct Transactions {
	map: HashMap<usize, Transaction>
}
impl Transactions {
	pub(crate) fn init() -> Transactions {
		Transactions { map: HashMap::with_hasher(RandomState::new()) }
	}

	pub(crate) fn get(&mut self, id: usize) -> Option<&mut Transaction> {
		self.map.get_mut(&id)
	}

	pub(crate) fn new(&mut self, webview: WebviewMut) -> &mut Transaction {
		let transaction = Transaction {
			id: TRANSACTION_ID.fetch_add(1, Ordering::Relaxed),
			progress: 0.,
			cancel_callbacks: Vec::new(),
		    webview
		};
		self.map.insert(transaction.id, transaction);
		self.get(self.map.len()-1).unwrap()
	}
}

use std::{collections::HashMap, sync::{Arc, atomic::{AtomicU16, AtomicUsize, Ordering}, mpsc::{self, Sender}}};
use tauri::{WebviewMut};

static TRANSACTION_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) enum TransactionMessage {
	Progress(f32),
	IncrementProgress(f32),
	Cancel(Arc<Vec<Box<dyn Fn() + Send + Sync + 'static>>>),
	Finish(Box<dyn erased_serde::Serialize + Send + Sync>)
}

pub(crate) struct Transaction {
	pub(crate) id: usize,
	progress: Arc<AtomicU16>,
	cancel_callbacks: Arc<Vec<Box<dyn Fn() + Send + Sync + 'static>>>,
	tx: Sender<TransactionMessage>
}

impl Drop for Transaction {
    fn drop(&mut self) {
        self.cancel();
    }
}

unsafe impl Sync for Transaction {}
unsafe impl Send for Transaction {}

impl Transaction {
	fn new(mut webview: WebviewMut) -> Transaction {
		let (tx, rx) = mpsc::channel();

		let progress = Arc::new(AtomicU16::new(0));
		let id = TRANSACTION_ID.fetch_add(1, Ordering::Relaxed);

		let transaction = Transaction {
			id,
			progress: progress.clone(),
			cancel_callbacks: Arc::new(Vec::new()),
			tx
		};

		std::thread::spawn(move || {
			use TransactionMessage::*;
			loop {
				let msg: TransactionMessage = rx.recv().unwrap();
				match msg {
					Progress(new_progress) => {
						progress.store(((new_progress * 100.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						tauri::event::emit(&mut webview, "transactionProgress", Some((id, (progress.load(Ordering::SeqCst) as f32) / 100.))).ok();
					},
					IncrementProgress(incr) => {
						progress.fetch_add(((incr * 100.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						tauri::event::emit(&mut webview, "transactionProgress", Some((id, (progress.load(Ordering::SeqCst) as f32) / 100.))).ok();
					},
					Cancel(callbacks) => {
						for callback in &*callbacks { (callback)(); }
						break;
					},
					Finish(data) => {
						tauri::event::emit(&mut webview, "transactionFinished", Some((id, data))).ok();
						break;
					}
				}
			}
		});

		transaction
	}

	pub(crate) fn connect_cancel(&mut self, f: Box<dyn Fn() + Send + Sync + 'static>) {
		match Arc::get_mut(&mut self.cancel_callbacks) {
			Some(cancel_callbacks) => cancel_callbacks.push(f),
			None => {}
		}
	}

	pub(crate) fn cancel(&self) {
		let res = self.tx.send(TransactionMessage::Cancel(self.cancel_callbacks.clone()));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress(&self, progress: f32) {
		let res = self.tx.send(TransactionMessage::Progress(progress));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress_incr(&self, incr: f32) {
		let res = self.tx.send(TransactionMessage::IncrementProgress(incr));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn finish<T: erased_serde::Serialize + Send + Sync + 'static>(&self, data: T) {
		let res = self.tx.send(TransactionMessage::Finish(Box::new(data)));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}
}

pub(crate) struct Transactions {
	map: HashMap<usize, Arc<Transaction>>
}
impl Transactions {
	pub(crate) fn init() -> Transactions {
		Transactions { map: HashMap::new() }
	}

	pub(crate) fn get(&self, id: usize) -> Option<Arc<Transaction>> {
		self.map.get(&id).map(|x| x.clone())
	}

	pub(crate) fn take(&mut self, id: usize) -> Option<Arc<Transaction>> {
		self.map.remove(&id)
	}

	pub(crate) fn new(&mut self, webview: WebviewMut) -> Arc<Transaction> {
		let transaction = Transaction::new(webview);
		let transaction_ref = Arc::new(transaction);
		self.map.insert(transaction_ref.id, transaction_ref.clone());
		transaction_ref
	}
}

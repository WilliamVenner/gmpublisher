use std::{cell::Cell, collections::{HashMap, hash_map::RandomState}, sync::{Arc, Mutex, Weak, atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering}, mpsc::{self, Receiver, Sender}}};
use tauri::{Webview, WebviewMut};

static TRANSACTION_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) enum TransactionMessage {
	Progress(f32),
	IncrementProgress(f32),
	Cancel(Vec<Box<dyn FnOnce() + Send + Sync + 'static>>)
}

pub(crate) struct Transaction {
	pub(crate) id: usize,
	progress: Arc<AtomicU16>,
	cancel_callbacks: Vec<Box<dyn FnOnce() + Send + Sync + 'static>>,
	tx: Sender<TransactionMessage>
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
			cancel_callbacks: Vec::new(),
			tx
		};

		std::thread::spawn(move || {
			use TransactionMessage::*;
			loop {
				let msg: TransactionMessage = rx.recv().unwrap();
				match msg {
					Progress(new_progress) => {
						progress.store(((new_progress * 100.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						println!("{} = {}", new_progress, progress.load(Ordering::SeqCst));
						tauri::event::emit(&mut webview, "transactionProgress", Some((id, (progress.load(Ordering::SeqCst) as f32) / 100.))).ok();
					},
					IncrementProgress(incr) => {
						progress.fetch_add(((incr * 100.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						tauri::event::emit(&mut webview, "transactionProgress", Some((id, (progress.load(Ordering::SeqCst) as f32) / 100.))).ok();
					},
					Cancel(callbacks) => {
						for callback in callbacks { (callback)(); }
						break;
					},
				}
			}
		});

		transaction
	}

	pub(crate) fn connect_cancel(&mut self, f: Box<dyn FnOnce() + Send + Sync + 'static>) {
		self.cancel_callbacks.push(f);
	}

	pub(crate) fn cancel(self) {
		crate::TRANSACTIONS.write().unwrap().map.remove(&self.id);
		let res = self.tx.send(TransactionMessage::Cancel(self.cancel_callbacks));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress(&self, progress: f32) {
		let res = self.tx.send(TransactionMessage::Progress(progress));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress_incr(&mut self, incr: f32) {
		let res = self.tx.send(TransactionMessage::IncrementProgress(incr));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}
}

pub(crate) struct Transactions {
	map: HashMap<usize, Arc<Transaction>>
}
impl Transactions {
	pub(crate) fn init() -> Transactions {
		Transactions { map: HashMap::with_hasher(RandomState::new()) }
	}

	pub(crate) fn get(&mut self, id: usize) -> Option<Arc<Transaction>> {
		self.map.get(&id).map(|x| x.clone())
	}

	pub(crate) fn new(&mut self, webview: WebviewMut) -> Arc<Transaction> {
		let transaction = Transaction::new(webview);
		let transaction_ref = Arc::new(transaction);
		self.map.insert(transaction_ref.id, transaction_ref.clone());
		transaction_ref
	}
}

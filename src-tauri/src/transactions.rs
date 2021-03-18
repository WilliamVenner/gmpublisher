use std::{collections::HashMap, rc::Rc, sync::{Arc, Mutex, RwLock, RwLockReadGuard, atomic::{AtomicU16, AtomicUsize, Ordering}, mpsc::{self, Sender}}};
use tauri::{WebviewMut};

static TRANSACTION_ID: AtomicUsize = AtomicUsize::new(0);

pub(crate) type GenericJSON = Box<dyn erased_serde::Serialize + Send + Sync>;
pub(crate) type ErrorData = (String, GenericJSON);
pub(crate) type FinishedData = GenericJSON;

pub(crate) type CancelCallback = dyn Fn(&Option<Arc<ErrorData>>) + Send + Sync + 'static;
type CancelCallbacks = Arc<Vec<Box<CancelCallback>>>;

pub(crate) type FinishCallback = dyn Fn(&Arc<FinishedData>) + Send + Sync + 'static;
type FinishCallbacks = Arc<Vec<Box<FinishCallback>>>;

fn call_cancel_callbacks(callbacks: CancelCallbacks, error_data: &Option<Arc<ErrorData>>) {
	for callback in &*callbacks { (callback)(error_data); }
}

pub(crate) enum TransactionMessage {
	Progress(f32),
	IncrementProgress(f32),
	Cancel(CancelCallbacks),
	Error(Arc<ErrorData>, CancelCallbacks), // TODO change double Box to a lifetime?
	Finish(Arc<FinishedData>, FinishCallbacks),
}

pub(crate) struct Transaction {
	pub(crate) id: usize,
	progress: Arc<AtomicU16>,
	tx: Sender<TransactionMessage>,

	cancel_callbacks: CancelCallbacks,
	finish_callbacks: FinishCallbacks,

	cancelled: RwLock<Option<Option<Arc<ErrorData>>>>,
	finished: RwLock<Option<Arc<FinishedData>>>,
	error: RwLock<Option<Arc<ErrorData>>>,
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
			finish_callbacks: Arc::new(Vec::new()),
			cancelled: RwLock::new(None),
			finished: RwLock::new(None),
			error: RwLock::new(None),
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
						call_cancel_callbacks(callbacks, &None);
						break;
					},
					Error(data, callbacks) => {
						call_cancel_callbacks(callbacks, &Some(data.clone()));
						tauri::event::emit(&mut webview, "transactionError", Some((id, data))).ok();
						break;
					},
					Finish(data, callbacks) => {
						for callback in &*callbacks { (callback)(&data); }
						tauri::event::emit(&mut webview, "transactionFinished", Some((id, data))).ok();
						break;
					},
				}
			}
		});

		transaction
	}

	pub(crate) fn connect_cancel(&mut self, f: Box<CancelCallback>) {
		match *self.cancelled.read().unwrap() {
			Some(ref data) => (&*f)(data),
			None => match Arc::get_mut(&mut self.cancel_callbacks) {
				Some(cancel_callbacks) => cancel_callbacks.push(f),
				None => {}
			}
		}
	}

	pub(crate) fn connect_finish(&mut self, f: Box<FinishCallback>) {
		match *self.finished.read().unwrap() {
			Some(ref data) => (&*f)(data),
			None => match Arc::get_mut(&mut self.finish_callbacks) {
				Some(finish_callbacks) => finish_callbacks.push(f),
				None => {}
			}
		}
	}

	pub(crate) fn cancel(&self) {
		let mut cancelled = self.cancelled.write().unwrap();
		if cancelled.is_some() { return }
		*cancelled = Some(None);

		let res = self.tx.send(TransactionMessage::Cancel(self.cancel_callbacks.clone()));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn error<T: erased_serde::Serialize + Send + Sync + 'static>(&self, msg: &str, data: T) {
		let mut error = self.error.write().unwrap();
		//let mut cancelled = self.cancelled.write().unwrap();

		// FIXME

		#[cfg(debug_assertions)]
		debug_assert!(error.is_none(), "Possible data race; transaction already errored");

		let data: Arc<(String, Box<dyn erased_serde::Serialize + Send + Sync + 'static>)> = Arc::new((msg.to_string(), Box::new(data)));
		*error = Some(data.clone());

		//*cancelled = Some(Some(data.clone()));

		let res = self.tx.send(TransactionMessage::Error(data, self.cancel_callbacks.clone()));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn finish<T: erased_serde::Serialize + Send + Sync + 'static>(&self, data: T) {
		let mut finished = self.finished.write().unwrap();

		#[cfg(debug_assertions)]
		debug_assert!(finished.is_none(), "Possible data race; transaction already errored");

		let data: Arc<Box<dyn erased_serde::Serialize + Send + Sync + 'static>> = Arc::new(Box::new(data));

		*finished = Some(data.clone());

		let res = self.tx.send(TransactionMessage::Finish(data, self.finish_callbacks.clone()));
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
}

pub(crate) struct Transactions {
	id: usize,
	map: HashMap<usize, Arc<Transaction>>,
}
impl Transactions {
	pub(crate) fn init() -> Transactions {
		Transactions { id: 0, map: HashMap::new() }
	}

	pub(crate) fn get(&self, id: usize) -> Option<Arc<Transaction>> {
		self.map.get(&id).map(|x| x.clone())
	}

	pub(crate) fn take(&mut self, id: usize) -> Option<Arc<Transaction>> {
		self.map.remove(&id)
	}

	pub(crate) fn new(&mut self, webview: WebviewMut) -> TransactionBuilder {
		self.id = self.id + 1;
		TransactionBuilder {
			id: self.id,
			inner: Transaction::new(webview)
		}
	}
}

pub(crate) struct TransactionBuilder {
	pub(crate) id: usize,
	inner: Transaction,
}
impl TransactionBuilder {
	pub(crate) fn connect_cancel(mut self, f: Box<CancelCallback>) -> Self {
		self.inner.connect_cancel(f);
		self
	}

	pub(crate) fn connect_finish(mut self, f: Box<FinishCallback>) -> Self {
		self.inner.connect_finish(f);
		self
	}

	pub(crate) fn build(self) -> Arc<Transaction> {
		let transaction_ref = Arc::new(self.inner);
		crate::TRANSACTIONS.write().unwrap().map.insert(self.id, transaction_ref.clone());
		transaction_ref
	}
}

use std::{cell::UnsafeCell, collections::HashMap, fmt::Debug, panic::{self, AssertUnwindSafe}, rc::Rc, sync::{Arc, Mutex, RwLock, RwLockReadGuard, atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering}, mpsc::{self, Sender, SyncSender}}, thread::{JoinHandle, Thread, ThreadId}};
use tauri::{WebviewMut};

pub(crate) type GenericJSON = Box<dyn erased_serde::Serialize + Send + Sync>;

pub(crate) type ErrorData = Option<(String, GenericJSON)>;
pub(crate) type FinishedData = GenericJSON;

pub(crate) type AbortCallback = Box<dyn Fn(&TransactionStatus) + Send + Sync + 'static>;

pub(crate) enum TransactionMessage {
	Progress(f32),
	IncrementProgress(f32),

	Cancel,
	Error(ErrorData),
	Finish(FinishedData),
}
impl Debug for TransactionMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&match self {
		    TransactionMessage::Progress(progress) => format!("Progress({})", progress),
		    TransactionMessage::IncrementProgress(progress) => format!("Progress({})", progress),
		    TransactionMessage::Cancel => "Cancel".to_string(),
		    TransactionMessage::Error(_) => "Error".to_string(),
		    TransactionMessage::Finish(_) => "Finish".to_string(),
		})
    }
}

pub(crate) enum TransactionStatus {
	Pending,
	Cancelled,
	Finished(FinishedData),
	Error(ErrorData)
}
impl Default for TransactionStatus {
    fn default() -> Self {
        Self::Pending
    }
}

pub(crate) struct Transaction {
	pub(crate) id: usize,
	progress: Arc<AtomicU16>,
	tx: Option<SyncSender<TransactionMessage>>,

	callbacks: Vec<AbortCallback>,

	status: RwLock<TransactionStatus>,
	aborted: Arc<AtomicBool>,

	#[cfg(debug_assertions)]
	thread: Option<ThreadId>
}

impl Drop for Transaction {
    fn drop(&mut self) {
        match &*self.status.read().unwrap() {
			TransactionStatus::Pending => panic!("Transaction dropped while still in progress!"),
			_ => {}
		}
    }
}

impl Transaction {
	fn new(id: usize) -> Transaction {
		Transaction {
			id,

			progress: Arc::new(AtomicU16::new(0)),
			aborted: Arc::new(AtomicBool::new(false)),
			status: RwLock::new(TransactionStatus::default()),

			callbacks: Vec::new(),

			tx: None,

			#[cfg(debug_assertions)]
			thread: None,
		}
	}

	fn listen(mut self, mut webview: WebviewMut) -> Arc<Transaction> {
		let rx = {
			#[cfg(debug_assertions)]
			{
				assert!(self.tx.is_none(), "This transaction is already listening.");
				self.thread = Some(std::thread::current().id());
			}

			let (tx, rx) = mpsc::sync_channel(101);
			self.tx = Some(tx);
			rx
		};

		let transaction_ref = Arc::new(self);
		let transaction = transaction_ref.clone();
		std::thread::spawn(move || {
			let abort = || {
				let status = transaction.status.read().unwrap();
				for callback in &*transaction.callbacks { (callback)(&*status); }
				crate::TRANSACTIONS.write().unwrap().map.remove(&transaction.id);
			};

			use TransactionMessage::*;
			loop {
				let msg: TransactionMessage = rx.recv().unwrap();
				match msg {
					Finish(data) => {
						tauri::event::emit(&mut webview, "transactionFinished", Some((transaction.id, &data))).ok();
						*transaction.status.write().unwrap() = TransactionStatus::Finished(data);
						abort();
						break;
					},
					Error(data) => {
						tauri::event::emit(&mut webview, "transactionError", Some((transaction.id, &data))).ok();
						*transaction.status.write().unwrap() = TransactionStatus::Error(data);
						abort();
						break;
					},
					Cancel => {
						*transaction.status.write().unwrap() = TransactionStatus::Cancelled;
						abort();
						break;
					},
					Progress(new_progress) => {
						transaction.progress.store(((new_progress * 100.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						tauri::event::emit(&mut webview, "transactionProgress", Some((transaction.id, new_progress))).ok();
					},
					IncrementProgress(incr) => {
						let incr = ((incr * 100.).round() as u16).min(10000).max(0);
						let progress = transaction.progress.fetch_add(incr, Ordering::SeqCst) + incr;
						tauri::event::emit(&mut webview, "transactionProgress", Some((transaction.id, (progress as f32) / 100.))).ok();
					},
				}
			}
		});

		transaction_ref
	}

	pub(crate) fn channel(&self) -> TransactionChannel {
		#[cfg(debug_assertions)]
		{
			assert!(self.thread.expect("You can't create a new channel for a thread that isn't listening yet.") == std::thread::current().id(), "You can only create a new transaction channel on the thread the transaction began listening on.");
		}

		TransactionChannel
		{
			inner: self.tx.as_ref().unwrap().clone(),
			aborted: self.aborted.clone()
		}
	}

	pub(crate) fn progress(&self) -> u16 {
		self.progress.load(Ordering::Acquire)
	}

	pub(crate) fn aborted(&self) -> bool {
		self.aborted.load(Ordering::Acquire)
	}

	pub(crate) fn status(&self) -> RwLockReadGuard<'_, TransactionStatus> {
		self.status.read().unwrap()
	}

	pub(crate) unsafe fn tx(&self) -> &SyncSender<TransactionMessage> {
		&self.tx.as_ref().unwrap()
	}
}

pub(crate) struct TransactionChannel {
	inner: SyncSender<TransactionMessage>,
	aborted: Arc<AtomicBool>,
}
impl std::ops::Deref for TransactionChannel {
    type Target = SyncSender<TransactionMessage>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl TransactionChannel {
	pub(crate) fn cancel(&self) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to cancel an already aborted transaction."); return; }

		let res = self.send(TransactionMessage::Cancel);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn error<T: erased_serde::Serialize + Send + Sync + 'static>(&self, msg: &str, data: T) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to error an already aborted transaction."); return; }

		let res = self.send(
			TransactionMessage::Error(
				Some((
					msg.to_string(),
					Box::new(data)
				))
			)
		);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn finish<T: erased_serde::Serialize + Send + Sync + 'static>(&self, data: T) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to finish an already aborted transaction."); return; }

		let res = self.send(
			TransactionMessage::Finish(
				Box::new(data)
			)
		);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress(&self, progress: f32) {
		if self.aborted.load(Ordering::Acquire) { debug_assert!(false, "Tried to progress an aborted transaction."); return; }

		let res = self.send(TransactionMessage::Progress(progress));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver")
	}

	pub(crate) fn progress_incr(&self, incr: f32) {
		if self.aborted.load(Ordering::Acquire) { debug_assert!(false, "Tried to progress an aborted transaction."); return; }

		let res = self.send(TransactionMessage::IncrementProgress(incr));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver")
	}
}

pub(crate) struct PendingTransaction {
	tx: SyncSender<()>,
	recv: JoinHandle<()>
}
impl PendingTransaction {
	pub(crate) fn new(transaction_ref: Arc<Transaction>) -> Self {
		let channel = transaction_ref.channel();
		let (tx, rx) = mpsc::sync_channel(1);
		Self {
			tx,
			recv: std::thread::spawn(move || {
				if let Ok(()) = rx.recv() {
					channel.cancel();
				}
			})
		}
	}

	pub(crate) fn cancel(self) {
		self.tx.send(()).ok();
	}
}

pub(crate) struct Transactions {
	id: usize,
	map: HashMap<usize, PendingTransaction>,
}
impl Transactions {
	pub(crate) fn init() -> Transactions {
		Transactions { id: 0, map: HashMap::new() }
	}

	pub(crate) fn get(&self, id: usize) -> Option<&PendingTransaction> {
		self.map.get(&id)
	}

	pub(crate) fn cancel(&mut self, id: usize) {
		if let Some(pending) = self.map.remove(&id) {
			pending.cancel();
		}
	}

	pub(crate) fn new(&mut self, webview: WebviewMut) -> TransactionBuilder {
		self.id = self.id + 1;
		TransactionBuilder {
			id: self.id,
			inner: Transaction::new(self.id),
			webview
		}
	}
}

pub(crate) struct TransactionBuilder {
	pub(crate) id: usize,
	inner: Transaction,
	webview: WebviewMut
}
impl TransactionBuilder {
	pub(crate) fn connect_abort<F>(mut self, f: F) -> Self
	where
		F: Fn(&TransactionStatus) + Send + Sync + 'static
	{
		self.inner.callbacks.push(Box::new(f));
		self
	}

	pub(crate) fn build(self) -> Arc<Transaction> {
		let transaction_ref = self.inner.listen(self.webview);

		crate::TRANSACTIONS.write().unwrap().map.insert(
			self.id,
			PendingTransaction::new(transaction_ref.clone())
		);

		transaction_ref
	}
}

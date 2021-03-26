use std::{cell::UnsafeCell, collections::HashMap, fmt::Debug, panic::{self, AssertUnwindSafe}, rc::Rc, sync::{Arc, Mutex, RwLock, RwLockReadGuard, atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering}, mpsc::{self, Sender, SyncSender}}, thread::{JoinHandle, Thread, ThreadId}};
use tauri::{WebviewMut};

pub(crate) type AbortCallback = Box<dyn Fn(&TransactionStatus) + Send + Sync + 'static>;

type ErrorData = Option<(String, TransactionDataBoxed)>;
type FinishedData = TransactionDataBoxed;
type GenericData = TransactionDataBoxed;

pub(crate) enum TransactionMessage {
	Progress(f64),
	ProgressMessage(String),
	Data(GenericData),

	Cancel,
	Error(ErrorData),
	Finish(FinishedData),
}
impl Debug for TransactionMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(&match self {
			TransactionMessage::Progress(progress) => format!("Progress({})", progress),
			TransactionMessage::ProgressMessage(msg) => format!("ProgressMessage({})", msg),
			TransactionMessage::Cancel => "Cancel".to_string(),
			TransactionMessage::Error(_) => "Error".to_string(),
			TransactionMessage::Finish(_) => "Finish".to_string(),
			TransactionMessage::Data(_) => "Data".to_string(),
		})
	}
}

pub(crate) enum TransactionStatus {
	Pending,

	Cancelled,
	Error(ErrorData),
	Finished(FinishedData),
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
impl serde::Serialize for Transaction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
        serializer.serialize_u64(self.id as u64)
    }
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
						transaction.progress.store(((new_progress * 10000.).round() as u16).min(10000).max(0), Ordering::SeqCst);
						tauri::event::emit(&mut webview, "transactionProgress", Some((transaction.id, new_progress))).ok();
					},
					ProgressMessage(msg) => {
						tauri::event::emit(&mut webview, "transactionProgressMsg", Some((transaction.id, msg))).ok();
					},
					Data(data) => {
						tauri::event::emit(&mut webview, "transactionData", Some((transaction.id, data))).ok();
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

	pub(crate) fn progress(&self) -> f64 {
		(self.progress.load(Ordering::Acquire) as f64) / 10000.
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

pub(crate) type TransactionDataBoxed = Box<dyn erased_serde::Serialize + Sync + Send + 'static>;
pub(crate) trait TransactionDataToBox {
	fn into_box(self) -> TransactionDataBoxed;
}

pub(crate) struct TransactionData(pub(crate) TransactionDataBoxed);
impl TransactionDataToBox for TransactionData {
	fn into_box(self) -> TransactionDataBoxed {
		Box::new(self.0)
	}
}
#[macro_export]
macro_rules! transaction_data {
	( $x:expr ) => {
		crate::transactions::TransactionData(Box::new($x))
	};
}

pub(crate) type TransactionDataFutureFn = dyn FnOnce() -> TransactionDataBoxed + Sync + Send + 'static;
pub(crate) struct TransactionDataFuture(pub(crate) Box<TransactionDataFutureFn>);
impl TransactionDataToBox for TransactionDataFuture {
	fn into_box(self) -> TransactionDataBoxed {
		(self.0)()
	}
}
#[macro_export]
macro_rules! transaction_data_fn {
	( $x:expr ) => {
		crate::transactions::TransactionDataFuture(Box::new(move || {$x}))
	};
}

pub(crate) struct TransactionDataRaw(pub(crate) serde_json::Value);
impl TransactionDataToBox for TransactionDataRaw {
	fn into_box(self) -> TransactionDataBoxed {
		Box::new(self.0)
	}
}
#[macro_export]
macro_rules! transaction_data_raw {
	( $x:expr ) => {
		crate::transactions::TransactionDataRaw(serde_json::to_value($x).unwrap())
	};
}

#[derive(Clone)]
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
	pub(crate) fn aborted(&self) -> bool {
		self.aborted.load(Ordering::Acquire)
	}

	pub(crate) fn cancel(&self) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to cancel an already aborted transaction."); return; }

		#[cfg(debug_assertions)]
		{
			println!("Transaction cancelled!");
		}

		let res = self.send(TransactionMessage::Cancel);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn error<T: TransactionDataToBox>(&self, msg: &str, data: T) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to error an already aborted transaction."); return; }

		let res = self.send(
			TransactionMessage::Error(
				Some((
					msg.to_string(),
					data.into_box()
				))
			)
		);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn finish<T: TransactionDataToBox>(&self, data: T) {
		if self.aborted.fetch_or(true, Ordering::AcqRel) { debug_assert!(false, "Tried to finish an already aborted transaction."); return; }

		let res = self.send(
			TransactionMessage::Finish(
				data.into_box()
			)
		);
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver");
	}

	pub(crate) fn progress(&self, progress: f64) {
		if self.aborted.load(Ordering::Acquire) {
			#[cfg(debug_assertions)]
			println!("Tried to progress an aborted transaction.");
			return;
		}

		let res = self.send(TransactionMessage::Progress(progress));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver")
	}

	pub(crate) fn progress_msg(&self, msg: &str) {
		if self.aborted.load(Ordering::Acquire) {
			#[cfg(debug_assertions)]
			println!("Tried to progress an aborted transaction.");
			return;
		}

		let res = self.send(TransactionMessage::ProgressMessage(msg.to_string()));
		debug_assert!(res.is_ok(), "Failed to send message to transaction receiver")
	}

	pub(crate) fn data<T: TransactionDataToBox>(&self, data: T) {
		if self.aborted.load(Ordering::Acquire) {
			#[cfg(debug_assertions)]
			println!("Tried to send data to an aborted transaction.");
			return;
		}

		let res = self.send(TransactionMessage::Data(data.into_box()));
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

	pub(crate) fn new(webview: WebviewMut) -> TransactionBuilder {
		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		transactions.id = transactions.id + 1;
		TransactionBuilder {
			id: transactions.id,
			inner: Transaction::new(transactions.id),
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

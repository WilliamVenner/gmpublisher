mod websocket;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use rayon::ThreadPool;
use serde::Serialize;
use std::sync::{
	atomic::{AtomicBool, AtomicU32, Ordering},
	Arc, Weak,
};

use crate::dprintln;

use self::websocket::{TransactionMessage, TransactionServer};

lazy_static! {
	static ref TRANSACTIONS: Transactions = Transactions::init();
	static ref TRANSACTIONS_SLAVE: ThreadPool = thread_pool!(1);
}

pub struct Transactions {
	inner: RwLock<Vec<TransactionRef>>,
	id: AtomicU32,
	websocket: Option<TransactionServer>,
}
impl std::ops::Deref for Transactions {
	type Target = RwLock<Vec<TransactionRef>>;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
impl Transactions {
	fn init() -> Transactions {
		Transactions {
			inner: RwLock::new(Vec::new()),
			id: AtomicU32::new(0),
			websocket: if *crate::cli::CLI_MODE { None } else { TransactionServer::init().ok() },
		}
	}

	pub fn find(&self, transaction_id: u32) -> Option<Transaction> {
		let transactions = self.inner.read();
		if let Ok(pos) = transactions.binary_search_by_key(&transaction_id, |transaction| transaction.id) {
			let transaction = transactions.get(pos).unwrap().upgrade();
			if transaction.is_some() {
				return transaction;
			} else {
				#[cfg(debug_assertions)]
				panic!("Stale transaction found in transactions list");
			}
		}

		None
	}
}

pub struct TransactionRef {
	pub id: u32,
	ptr: Weak<TransactionInner>,
}
impl std::ops::Deref for TransactionRef {
	type Target = Weak<TransactionInner>;
	fn deref(&self) -> &Self::Target {
		&self.ptr
	}
}
impl PartialOrd for TransactionRef {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id.partial_cmp(&other.id)
	}
}
impl Ord for TransactionRef {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id.cmp(&other.id)
	}
}
impl PartialEq for TransactionRef {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}
impl Eq for TransactionRef {}

#[inline(always)]
fn progress_as_int(progress: f64) -> u16 {
	u16::min((progress * 10000.) as u16, 10000)
}

pub type Transaction = Arc<TransactionInner>;
#[derive(Debug)]
pub struct TransactionInner {
	pub id: u32,
	aborted: AtomicBool,
}
impl TransactionInner {
	fn emit(&self, message: TransactionMessage) {
		if *crate::cli::CLI_MODE { return; }

		if let Some(ref websocket) = TRANSACTIONS.websocket {
			websocket.send(message);
		} else {
			TransactionServer::send_tauri_event(message);
		}
	}

	fn abort(&self) {
		self.aborted.store(true, Ordering::Release);

		let id = self.id;
		TRANSACTIONS_SLAVE.spawn(move || {
			let mut transactions = TRANSACTIONS.write();
			if let Ok(pos) = transactions.binary_search_by_key(&id, |transaction| transaction.id) {
				transactions.remove(pos);
			}
		});
	}

	pub fn data<D: Serialize + Send + 'static>(&self, data: D) {
		self.emit(TransactionMessage::Data(self.id, json!(data)));
	}

	pub fn status<S: Into<String>>(&self, status: S) {
		self.emit(TransactionMessage::Status(self.id, status.into()))
	}

	pub fn progress(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit(TransactionMessage::Progress(self.id, progress_as_int(progress)));
		}
	}

	pub fn progress_incr(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit(TransactionMessage::IncrProgress(self.id, progress_as_int(progress)));
		}
	}

	pub fn progress_reset(&self) {
		if self.aborted() {
			dprintln!("Tried to reset the progress of an aborted transaction!");
		} else {
			self.emit(TransactionMessage::ResetProgress(self.id));
		}
	}

	pub fn error<S: Into<String>, D: Serialize + Send + 'static>(&self, msg: S, data: D) {
		self.abort();
		self.emit(TransactionMessage::Error(self.id, msg.into(), json!(data)));
	}

	pub fn finished<D: Serialize + Send + 'static>(&self, data: D) {
		debug_assert!(!self.aborted(), "Tried to finish an aborted transaction!");
		self.abort();
		self.emit(TransactionMessage::Finished(self.id, json!(data)));
	}

	pub fn cancel(&self) {
		self.abort();
	}

	pub fn aborted(&self) -> bool {
		self.aborted.load(Ordering::Acquire)
	}
}
impl Drop for TransactionInner {
	fn drop(&mut self) {
		if !self.aborted() {
			self.error("ERR_UNKNOWN", turbonone!());
			self.abort();

			#[cfg(debug_assertions)]
			println!("{:#?}", backtrace::Backtrace::new());
		}
	}
}
impl serde::Serialize for TransactionInner {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_u64(self.id as u64)
	}
}

pub fn init() {
	lazy_static::initialize(&TRANSACTIONS);
}

pub fn new() -> Transaction {
	let transaction = Arc::new(TransactionInner {
		id: TRANSACTIONS.id.fetch_add(1, Ordering::SeqCst),
		aborted: AtomicBool::new(false),
	});

	{
		let mut transactions = TRANSACTIONS.write();
		transactions.push(TransactionRef {
			id: transaction.id,
			ptr: Arc::downgrade(&transaction),
		});
		transactions.reserve(1);
	}

	transaction
}

#[macro_export]
macro_rules! transaction {
	() => {
		crate::transactions::new()
	};
}

#[tauri::command]
fn cancel_transaction(id: u32) {
	if let Some(transaction) = TRANSACTIONS.find(id) {
		transaction.cancel();
	}
}

#[tauri::command]
fn websocket() -> Option<u16> {
	TRANSACTIONS.websocket.as_ref().map(|socket| socket.port)
}

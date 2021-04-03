use std::sync::{Arc, Weak, atomic::{AtomicBool, AtomicUsize, Ordering}};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::Serialize;

use crate::{dprintln, ignore, main_thread_forbidden, webview, webview_emit};

lazy_static! {
	static ref TRANSACTIONS: Transactions = Transactions::init();
	static ref TRANSACTIONS_SLAVE: ThreadPool = ThreadPoolBuilder::new().num_threads(1).build().unwrap();
}

pub struct Transactions {
	inner: RwLock<Vec<TransactionRef>>,
	id: AtomicUsize
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
			id: AtomicUsize::new(0)
		}
	}

	pub fn find(&self, transaction_id: usize) -> Option<Transaction> {
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
	pub id: usize,
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
	pub id: usize,
	aborted: AtomicBool
}
impl TransactionInner {
	fn emit<D: Serialize + Send + 'static>(&self, event: &'static str, data: D) {
		webview_emit!(event, (self.id, data));
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
		self.emit("TransactionData", data);
	}

	pub fn status<S: AsRef<str> + Serialize + Send + 'static>(&self, status: S) {
		self.emit("TransactionStatus", status)
	}

	pub fn progress(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit("TransactionProgress", progress_as_int(progress));
		}
	}

	pub fn progress_incr(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit("TransactionIncrProgress", progress_as_int(progress));
		}
	}

	pub fn error<D: Serialize + Send + 'static>(&self, error: D) {
		self.emit("TransactionError", error);
	}

	pub fn finished<D: Serialize + Send + 'static>(&self, data: Option<D>) {
		debug_assert!(!self.aborted(), "Tried to finish an aborted transaction!");
		self.abort();
		self.emit("TransactionFinished", data);
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
			self.error("ERR_UNKNOWN");
			self.abort();
		}
	}
}
impl serde::Serialize for TransactionInner {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer
	{
		serializer.serialize_u64(self.id as u64)
	}
}

pub fn new() -> Transaction {
	main_thread_forbidden!();

	let transaction = Arc::new(TransactionInner {
		id: TRANSACTIONS.id.fetch_add(1, Ordering::SeqCst),
		aborted: AtomicBool::new(false)
	});

	{
		let mut transactions = TRANSACTIONS.write();
		transactions.push(TransactionRef {
			id: transaction.id,
			ptr: Arc::downgrade(&transaction)
		});
		transactions.reserve(1);
	}
	
	transaction
}

#[tauri::command]
fn cancel_transaction(id: usize) {
	if let Some(transaction) = TRANSACTIONS.find(id) {
		transaction.cancel();
	}
}

#[macro_export]
macro_rules! transaction {
	() => { crate::transactions::new() };
}
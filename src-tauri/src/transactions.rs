use std::sync::{Arc, Weak, atomic::{AtomicBool, Ordering}};
use atomic_refcell::AtomicRefCell;
use lazy_static::lazy_static;
use serde::Serialize;

use crate::{dprintln, ignore, main_thread_forbidden, webview, webview_emit};

lazy_static! {
	static ref TRANSACTIONS: Arc<AtomicRefCell<Vec<Weak<TransactionInner>>>> = Arc::new(AtomicRefCell::new(Vec::new()));
	// FIXME "memory leak" when lots of transactions are made over time... maybe implement a GC to turn this Weak<>s into Option<Weaks<>>
	// or just use something other than Vec<>
}

#[inline(always)]
fn progress_as_int(progress: f64) -> u16 {
	u16::min((progress * 10000.) as u16, 10000)
}

#[derive(Debug)]
pub(crate) struct TransactionInner {
	pub(crate) id: usize,
	aborted: AtomicBool
}
impl TransactionInner {
	fn emit<S: AsRef<str>, D: Serialize>(&self, event: S, data: D) {
		ignore! { webview_emit!(event.as_ref(), (self.id, data)) }
	}

	fn abort(&self) {
		self.aborted.store(true, Ordering::Release);
	}

	pub(crate) fn data<D: Serialize>(&self, data: D) {
		self.emit("TransactionData", data);
	}

	pub(crate) fn status<S: AsRef<str> + Serialize>(&self, status: S) {
		self.emit("TransactionStatus", status)
	}

	pub(crate) fn progress(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit("TransactionProgress", progress_as_int(progress));
		}
	}

	pub(crate) fn progress_incr(&self, progress: f64) {
		if self.aborted() {
			dprintln!("Tried to progress an aborted transaction!");
		} else {
			self.emit("TransactionIncrProgress", progress_as_int(progress));
		}
	}

	pub(crate) fn error<S: AsRef<str> + Serialize>(&self, error: S) {
		self.emit("TransactionError", error);
	}

	pub(crate) fn finished<D: Serialize>(&self, data: Option<D>) {
		debug_assert!(!self.aborted(), "Tried to finish an aborted transaction!");
		self.abort();
		self.emit("TransactionFinished", data);
	}

	pub(crate) fn cancel(&self) {
		self.abort();
	}

	pub(crate) fn aborted(&self) -> bool {
		self.aborted.load(Ordering::Acquire)
	}
}
impl Drop for TransactionInner {
    fn drop(&mut self) {
        if !self.aborted() {
			self.error("ERR_UNKNOWN");
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

pub(crate) type Transaction = Arc<TransactionInner>;
pub(crate) fn new() -> Transaction {
	main_thread_forbidden!();

	let mut transaction = TransactionInner {
		id: 0,
		aborted: AtomicBool::new(false)
	};

	loop {
		if let Ok(mut transactions) = TRANSACTIONS.try_borrow_mut() {
			transaction.id = transactions.len();
			let transaction = Arc::new(transaction);

			transactions.push(Arc::downgrade(&transaction));
			transactions.reserve(1);
			
			break transaction
		}
	}
}

#[tauri::command]
fn cancel_transaction(id: usize) {
	loop {
		if let Ok(transactions) = TRANSACTIONS.try_borrow() {
			if let Some(transaction) = transactions.get(id) {
				if let Some(transaction) = transaction.upgrade() {
					transaction.cancel();
					break;
				}
			}
		}
	}
}

#[macro_export]
macro_rules! transaction {
	() => { crate::transactions::new() };
}
// Utility library for shared concurrency between JS and Rust.

use std::{any::Any, collections::{HashMap, LinkedList, VecDeque}, fmt::Debug, hash::Hash, sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc}};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut, BorrowMutError};
use crossbeam::channel::{Receiver, Sender};

use parking_lot::{Mutex, RwLock, RwLockWriteGuard};

use std::collections::hash_map::Entry::{Occupied, Vacant};

pub enum VariableSingleton<T> {
	Singleton(T),
	Variable(Vec<T>),
}

pub type PromiseHashCache<K, V> = PromiseCache<HashMap<K, V>, K, V>;
pub type PromiseHashNullableCache<K, V> = PromiseCache<HashMap<K, V>, K, Option<V>>;

pub struct PromiseCache<Cache: Default + Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> {
	cache: RelaxedRwLock<Cache>,
	promises: RwLock<HashMap<K, VariableSingleton<Box<dyn FnOnce(&Args) + Send + 'static>>>>,
}
impl<Cache: Default + Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> std::ops::Deref for PromiseCache<Cache, K, Args> {
	type Target = RelaxedRwLock<Cache>;
	fn deref(&self) -> &Self::Target {
		&self.cache
	}
}
impl<Cache: Default + Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> std::ops::DerefMut for PromiseCache<Cache, K, Args> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.cache
	}
}
impl<Cache: Default + Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> PromiseCache<Cache, K, Args> {
	pub fn new(cache: Cache) -> PromiseCache<Cache, K, Args> {
		PromiseCache {
			cache: RelaxedRwLock::new(cache),
			promises: RwLock::new(HashMap::new()),
		}
	}

	pub fn task<F>(&self, k: K, f: F) -> bool
	where
		F: FnOnce(&Args) + Send + 'static,
	{
		use VariableSingleton::*;
		match self.promises.write().entry(k.clone()) {
			Occupied(mut o) => {
				match o.get_mut() {
					Variable(ref mut vec) => vec.push(Box::new(f)),
					_ => unreachable!(),
				}
				false
			}
			Vacant(v) => {
				v.insert(Singleton(Box::new(f)));
				true
			}
		}
	}

	pub fn promises(&self, k: &K) -> Option<VariableSingleton<Box<dyn FnOnce(&Args) + Send + 'static>>> {
		self.promises.write().remove(k)
	}

	pub fn execute(&self, k: &K, v: Args) {
		use rayon::iter::{IntoParallelIterator, ParallelIterator};
		use VariableSingleton::*;
		if let Some(promises) = self.promises(k) {
			match promises {
				Singleton(f) => f(&v),
				Variable(vec) => {
					vec.into_par_iter().for_each_with(v, |v, f| f(v as &Args));
				}
			}
		}
	}
}

pub struct AtomicRefSome<'a, V> {
	inner: AtomicRef<'a, Option<V>>,
}
impl<V> std::ops::Deref for AtomicRefSome<'_, V> {
	type Target = V;
	fn deref(&self) -> &Self::Target {
		debug_assert!(self.inner.as_ref().is_some(), "Steam has not connected yet");
		self.inner.as_ref().unwrap()
	}
}
impl<'a, V> From<AtomicRef<'a, Option<V>>> for AtomicRefSome<'a, V> {
	fn from(inner: AtomicRef<'a, Option<V>>) -> Self {
		AtomicRefSome { inner }
	}
}
pub struct AtomicRefMutSome<'a, V> {
	pub inner: AtomicRefMut<'a, Option<V>>,
}
impl<V> std::ops::Deref for AtomicRefMutSome<'_, V> {
	type Target = V;
	fn deref(&self) -> &Self::Target {
		self.inner.as_ref().unwrap()
	}
}
impl<V> std::ops::DerefMut for AtomicRefMutSome<'_, V> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.inner.as_mut().unwrap()
	}
}
impl<'a, V> From<AtomicRefMut<'a, Option<V>>> for AtomicRefMutSome<'a, V> {
	fn from(inner: AtomicRefMut<'a, Option<V>>) -> Self {
		AtomicRefMutSome { inner }
	}
}

pub type RelaxedRwLockFn<V> = dyn FnOnce(&mut RwLockWriteGuard<'_, V>) + 'static + Send + Sync;

/*
#[derive(derive_more::Deref)]
struct RelaxedRwLocks(Mutex<LinkedList<*const RelaxedRwLock<dyn Any + Send + Sync + 'static>>>);
unsafe impl Sync for RelaxedRwLocks {}
lazy_static! {
	static ref RELAXED_RW_LOCKS: RelaxedRwLocks = RelaxedRwLocks(Mutex::new(LinkedList::new()));
}
*/

#[derive(derive_more::Deref, Clone)]
pub struct RelaxedRwLock<V: Send + Sync + 'static> {
	#[deref]
	inner: Arc<RwLock<V>>,
	queue: Arc<Mutex<VecDeque<Box<RelaxedRwLockFn<V>>>>>,
}
impl<V: Send + Sync + 'static> RelaxedRwLock<V> {
	pub fn new(inner: V) -> Self {
		let inner = Arc::new(RwLock::new(inner));
		let queue: Arc<Mutex<VecDeque<Box<RelaxedRwLockFn<V>>>>> = Arc::new(Mutex::new(VecDeque::new()));

		{
			let inner = inner.clone();
			let queue = queue.clone();
			std::thread::spawn(move || loop {
				if let Some(mut queue) = queue.try_lock() {
					if !queue.is_empty() {
						if let Some(mut inner) = inner.try_write() {
							for f in queue.drain(..) {
								f(&mut inner);
							}
						}
					}
				}
				sleep_ms!(1000);
			});
		}

		Self {
			inner,
			queue,
		}
	}

	pub fn write<F>(&'static self, f: F)
	where
		F: FnOnce(&mut RwLockWriteGuard<'_, V>) + 'static + Send + Sync
	{
		if let Some(mut lock) = self.inner.try_write() {
			f(&mut lock);
		} else {
			self.queue.lock().push_back(Box::new(f));
		}
	}
}
impl<V: Send + Sync + 'static> Drop for RelaxedRwLock<V> {
    fn drop(&mut self) {
        debug_assert!(false, "A RelaxedRwLock should never be dropped");
    }
}

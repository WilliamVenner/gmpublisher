// Utility library for shared concurrency between JS and Rust.

use std::{
	collections::{HashMap, VecDeque},
	hash::Hash,
	sync::Arc,
};

use atomic_refcell::{AtomicRef, AtomicRefMut};

use parking_lot::{Condvar, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use std::collections::hash_map::Entry::{Occupied, Vacant};

pub enum VariableSingleton<T> {
	Singleton(T),
	Variable(Vec<T>),
}

pub type PromiseHashCache<K, V> = PromiseCache<HashMap<K, V>, K, V>;
pub type PromiseHashNullableCache<K, V> = PromiseCache<HashMap<K, V>, K, Option<V>>;

#[derive(derive_more::Deref)]
pub struct PromiseCache<Cache: Default + Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> {
	#[deref]
	cache: RelaxedRwLock<Cache>,
	promises: RwLock<HashMap<K, VariableSingleton<Box<dyn FnOnce(&Args) + Send + 'static>>>>,
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

#[derive(derive_more::Deref, Clone)]
pub struct RelaxedRwLock<V: Send + Sync + 'static> {
	#[deref]
	inner: Arc<RwLock<V>>,
	queue: Arc<(Mutex<VecDeque<Box<RelaxedRwLockFn<V>>>>, Condvar)>,
}
impl<V: Send + Sync + 'static> RelaxedRwLock<V> {
	pub fn new(inner: V) -> Self {
		let inner = Arc::new(RwLock::new(inner));
		let queue: Arc<(Mutex<VecDeque<Box<RelaxedRwLockFn<V>>>>, Condvar)> = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));

		{
			let inner = inner.clone();
			let queue = queue.clone();
			std::thread::spawn(move || loop {
				let (mutex, cvar) = &*queue;
				{
					let mut guard = mutex.lock();
					if guard.is_empty() {
						cvar.wait(&mut guard);
					}
				}

				loop {
					{
						let mut queue = mutex.lock();
						if !queue.is_empty() {
							let mut inner = inner.write();
							for f in std::mem::take(&mut *queue) {
								f(&mut inner);
							}
						} else {
							break;
						}
					}
					sleep_ms!(25);
				}
			});
		}

		Self { inner, queue }
	}

	pub fn write<F>(&'static self, f: F)
	where
		F: FnOnce(&mut RwLockWriteGuard<'_, V>) + 'static + Send + Sync,
	{
		if let Some(mut lock) = self.inner.try_write() {
			f(&mut lock);
		} else {
			self.queue.0.lock().push_back(Box::new(f));
			self.queue.1.notify_all();
		}
	}

	pub fn write_sync(&'static self) -> RwLockWriteGuard<'_, V> {
		self.inner.write()
	}

	pub fn read_sync(&'static self) -> RwLockReadGuard<'_, V> {
		self.inner.read()
	}
}
impl<V: Send + Sync + 'static> Drop for RelaxedRwLock<V> {
	fn drop(&mut self) {
		debug_assert!(false, "A RelaxedRwLock should never be dropped");
	}
}

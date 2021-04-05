// Utility library for shared concurrency between JS and Rust.

use std::{
	collections::HashMap,
	hash::Hash,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut, BorrowMutError};
use crossbeam::channel::{Receiver, Sender};

use parking_lot::RwLock;

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

pub struct RelaxedRwLock<V: Default + Send + Sync + 'static> {
	r: Arc<AtomicRefCell<V>>,
	w: Arc<AtomicRefCell<Option<V>>>,
	tx: Sender<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
	begin: Arc<AtomicBool>,
}

unsafe impl<V: Default + Send + Sync + 'static> Send for RelaxedRwLock<V> {}
unsafe impl<V: Default + Send + Sync + 'static> Sync for RelaxedRwLock<V> {}

impl<V: Default + Send + Sync + 'static> RelaxedRwLock<V> {
	pub fn new(v: V) -> RelaxedRwLock<V> {
		let (tx, rx): (
			Sender<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
			Receiver<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
		) = crossbeam::channel::unbounded();
		let begin = Arc::new(AtomicBool::new(false));
		let r = Arc::new(AtomicRefCell::new(v));
		let w = Arc::new(AtomicRefCell::new(None));
		{
			let begin_ref = begin.clone();
			let w_ref = w.clone();
			let r_ref = r.clone();
			std::thread::spawn(move || loop {
				// TODO use a static vec of weak rc relaxedrwlocks and a single worker thread
				let f = match rx.try_recv() {
					Ok(f) => f,
					Err(error) => match error {
						crossbeam::channel::TryRecvError::Empty => {
							if !begin_ref.load(Ordering::Acquire) && AtomicRefCell::borrow(&w_ref).is_some() {
								if let Ok(mut r) = r_ref.try_borrow_mut() {
									drop(std::mem::replace(&mut *r, w_ref.borrow_mut().take().unwrap()));
								}
							}
							std::thread::sleep(std::time::Duration::from_millis(10));
							continue;
						}
						crossbeam::channel::TryRecvError::Disconnected => break,
					},
				};

				if AtomicRefCell::borrow(&w_ref).is_none() {
					loop {
						match r_ref.try_borrow_mut() {
							Ok(_r) => unsafe { *w_ref.borrow_mut() = Some(std::ptr::read(r_ref.as_ptr())) },
							Err(_) => continue,
						}
						break;
					}
				}
				f(w_ref.borrow_mut().into());
			});
		}

		RelaxedRwLock { r, w, tx, begin }
	}

	pub fn read(&self) -> AtomicRef<V> {
		loop {
			if let Ok(r) = self.r.try_borrow() {
				return r;
			}
		}
	}

	pub fn take(&self) -> V {
		loop {
			if let Ok(_r) = self.r.try_borrow_mut() {
				break unsafe { std::mem::take(&mut *self.r.as_ptr()) };
			}
		}
	}

	pub fn try_take(&self) -> Result<V, BorrowMutError> {
		let _r = self.r.try_borrow_mut()?;
		Ok(std::mem::take(unsafe { &mut *self.r.as_ptr() }))
	}

	pub fn try_take_inner<F, U: Default>(&self, map: F) -> Result<U, BorrowMutError>
	where
		F: FnOnce(&mut AtomicRefMut<'_, V>) -> *mut U + 'static,
	{
		let mut borrow = self.r.try_borrow_mut()?;
		let inner = map(&mut borrow);
		Ok(std::mem::take(unsafe { &mut *inner }))
	}

	pub fn write<F>(&self, callback: F) -> &Self
	where
		F: FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync,
	{
		self.tx.send(Box::new(callback)).unwrap();
		self
	}

	pub fn begin(&self) {
		self.begin.store(true, Ordering::Release);
	}
	pub fn commit(&self) {
		self.begin.store(false, Ordering::Release);
	}
}

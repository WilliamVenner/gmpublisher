// Utility library for shared concurrency between JS and Rust.

// TODO https://pkolaczk.github.io/multiple-threadpools-rust/

pub mod gma;
pub mod steamworks;

use std::{
	collections::HashMap,
	hash::Hash,
	sync::{
		atomic::{AtomicBool, Ordering},
		mpsc::{self, Receiver, Sender},
		Arc,
	},
};

use atomic_refcell::{AtomicRef, AtomicRefCell, AtomicRefMut};
use lazy_static::lazy_static;
use parking_lot::RwLock;
use rayon::{ThreadPool, ThreadPoolBuilder};

use std::collections::hash_map::Entry::{Occupied, Vacant};

pub use self::{gma::GMA, steamworks::Steamworks};

lazy_static! {
	pub static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

pub enum VariableSingleton<T> {
	Singleton(T),
	Variable(Vec<T>),
}

pub type PromiseHashCache<K, V> = PromiseCache<HashMap<K, V>, K, V>;
pub type PromiseHashNullableCache<K, V> = PromiseCache<HashMap<K, V>, K, Option<V>>;

pub struct PromiseCache<Cache: Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> {
	cache: RelaxedRwLock<Cache>,
	promises: RwLock<HashMap<K, VariableSingleton<Box<dyn FnOnce(&Args) + Send + 'static>>>>,
}
impl<Cache: Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> std::ops::Deref for PromiseCache<Cache, K, Args> {
	type Target = RelaxedRwLock<Cache>;
	fn deref(&self) -> &Self::Target {
		&self.cache
	}
}
impl<Cache: Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> std::ops::DerefMut for PromiseCache<Cache, K, Args> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.cache
	}
}
impl<Cache: Send + Sync + 'static, K: Hash + Eq + Clone, Args: Clone + Sync + Send> PromiseCache<Cache, K, Args> {
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

	pub fn execute(&self, k: &K, mut v: Args) {
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
		debug_assert!(self.inner.as_ref().is_some(), "Steamworks has not connected yet");
		self.inner.as_ref().unwrap()
	}
}
impl<'a, V> From<AtomicRef<'a, Option<V>>> for AtomicRefSome<'a, V> {
	fn from(inner: AtomicRef<'a, Option<V>>) -> Self {
		AtomicRefSome { inner }
	}
}
pub struct AtomicRefMutSome<'a, V> {
	inner: AtomicRefMut<'a, Option<V>>,
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

pub struct RelaxedRwLock<V: Send + Sync + 'static> {
	r: Arc<AtomicRefCell<V>>,
	w: Arc<AtomicRefCell<Option<V>>>,
	tx: Sender<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
	begin: Arc<AtomicBool>,
}

unsafe impl<V: Send + Sync + 'static> Send for RelaxedRwLock<V> {}
unsafe impl<V: Send + Sync + 'static> Sync for RelaxedRwLock<V> {}

impl<V: Send + Sync + 'static> RelaxedRwLock<V> {
	pub fn new(v: V) -> RelaxedRwLock<V> {
		let (tx, rx): (
			Sender<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
			Receiver<Box<dyn FnOnce(AtomicRefMutSome<V>) + 'static + Send + Sync>>,
		) = mpsc::channel();
		let begin = Arc::new(AtomicBool::new(false));
		let r = Arc::new(AtomicRefCell::new(v));
		let w = Arc::new(AtomicRefCell::new(None));
		{
			let begin_ref = begin.clone();
			let w_ref = w.clone();
			let r_ref = r.clone();
			std::thread::spawn(move || loop {
				let f = match rx.try_recv() {
					Ok(f) => f,
					Err(err) => {
						if let mpsc::TryRecvError::Empty = err {
							if !begin_ref.load(Ordering::Acquire) && AtomicRefCell::borrow(&w_ref).is_some() {
								if let Ok(mut r) = r_ref.try_borrow_mut() {
									drop(std::mem::replace(&mut *r, w_ref.borrow_mut().take().unwrap()));
								}
							}
						}
						std::thread::sleep(std::time::Duration::from_millis(10));
						continue;
					}
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

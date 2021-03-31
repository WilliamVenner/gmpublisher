use std::{collections::HashMap, fmt::Display, path::{Path, PathBuf}, sync::Arc};

use rayon::{ThreadPool, ThreadPoolBuilder};
use steamworks::PublishedFileId;

use crate::{gma::{GMAFile, GMAReadError}, main_thread_forbidden};

use super::PromiseCache;

pub struct GMA {
	thread_pool: ThreadPool,
	cache: PromiseCache<HashMap<PathBuf, GMAFile>, PathBuf, Result<GMAFile, GMAReadError>>
}

unsafe impl Sync for GMA {}
unsafe impl Send for GMA {}

impl GMA {
	pub fn init() -> GMA {
		Self {
			thread_pool: ThreadPoolBuilder::new().build().unwrap(),
			cache: PromiseCache::new(HashMap::new())
		}
	}

	pub fn get<P: AsRef<Path>>(&'static self, path: P, id: Option<PublishedFileId>) -> Result<GMAFile, GMAReadError> {
		main_thread_forbidden!();

		let path = path.as_ref();

		let mut gma = GMAFile::new(path, id)?;
		gma.metadata()?;
		gma.entries()?;

		{
			let path = path.to_path_buf();
			let gma = gma.clone();
			self.cache.write(move |mut cache| {
				cache.insert(path, gma);
			});
		}

		Ok(gma)
	}

	pub fn get_async<P: AsRef<Path>, F>(&'static self, path: P, id: Option<PublishedFileId>, f: F)
	where
		F: FnOnce(&Result<GMAFile, GMAReadError>) + 'static + Send
	{
		let path = path.as_ref();
		match self.cache.read().get(path) {
		    Some(gma) => {
				let mut gma = gma.clone();
				gma.id = id;
				f(&Ok(gma));
			},
		    None => {
				let path = path.to_path_buf();
				if self.cache.task(path.clone(), f) {
					self.thread_pool.spawn(move || {
						let v = self.get(&path, id);
						crate::GMA.cache.execute(&path, v);
					});
				}
			}
		}
	}
}

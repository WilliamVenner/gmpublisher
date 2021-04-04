use std::{ffi::OsString, fs::DirEntry, path::{Path, PathBuf}, sync::mpsc, time::SystemTime};

use indexmap::IndexMap;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use rayon::{ThreadPool, ThreadPoolBuilder};
use steamworks::PublishedFileId;

use crate::{app_data, GMAFile};

lazy_static! {
	static ref DISCOVERY_POOL: ThreadPool = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
	static ref GMA_FILE_EXTENSION: OsString = OsString::from("gma");
}

#[derive(Debug)]
pub struct GameAddons {
	addons: RwLock<IndexMap<PathBuf, GMAFile>>,
}

unsafe impl Sync for GameAddons {}
unsafe impl Send for GameAddons {}

impl GameAddons {
	pub fn init() -> Self {
		let game_addons = Self {
		    addons: RwLock::new(IndexMap::new()),
		};
		game_addons.discover_addons();
		game_addons
	}

	fn gma_check(entry: Result<DirEntry, std::io::Error>) -> Option<(PathBuf, String)> {
		let path = entry.ok()?.path();
		if !path.is_file() { return None; }
		
		let mut extension = path.extension()?.to_owned();
		extension.make_ascii_lowercase();
		if extension != *GMA_FILE_EXTENSION { return None; }

		let file_name = path.file_name()?.to_string_lossy().to_string();

		Some((path, (&file_name[..(file_name.len()-4)]).to_owned()))
	}

	pub fn discover_addons(&self) {
		let mut gmod = if let Some(gmod) = app_data!().gmod() { gmod } else {
			*self.addons.write() = IndexMap::new();
			return;
		};

		DISCOVERY_POOL.scope(move |scope| {
			let (tx, rx) = mpsc::channel();

			let addons_dir = gmod.join("GarrysMod/addons");

			gmod.push("GarrysMod/cache/workshop");
			let cache_dir = gmod;

			let tx_addons = tx.clone();
			scope.spawn(move |_| {
				let addons = match addons_dir.read_dir() {
					Ok(addons) => addons,
					Err(_) => return
				};

				'paths: for (path, file_name) in addons.filter_map(GameAddons::gma_check) {
					let mut id = 0u64;

					for char in file_name
						.chars()
						.rev() // Reverse iterator so we're looking at the suffix (the PublishedFileId)
						.take_while(|c| c.is_digit(10))
						.collect::<Vec<char>>()
						.into_iter()
						.rev()
					{
						match id.checked_add(char::to_digit(char, 10).unwrap() as u64) {
							None => continue 'paths,
							Some(id_op) => match 10_u64.checked_mul(id_op) {
								None => continue 'paths,
								Some(id_op) => id = id_op
							},
						}
					}

					tx_addons.send((
						path,
						if id == 0 { None } else {
							Some(PublishedFileId(id / 10))
						}
					)).unwrap();
				}
			});
	
			let tx_cache = tx;
			scope.spawn(move |_| {
				let cache = match cache_dir.read_dir() {
					Ok(cache) => cache,
					Err(_) => return
				};

				for (path, file_name) in cache.filter_map(GameAddons::gma_check) {
					let id = match str::parse::<u64>(&file_name) {
						Ok(id) => Some(PublishedFileId(id)),
						Err(_ok) => None,
					};

					tx_cache.send((path, id)).unwrap();
				}
			});
			
			let mut addons: Vec<(GMAFile, Option<SystemTime>)> = vec![];
			while let Ok((path, id)) = rx.recv() {
				let mut gma = match GMAFile::open(&path) {
					Ok(gma) => gma,
					Err(_) => continue
				};

				if let Some(id) = id { gma.set_ws_id(id); }

				let modified = path.metadata().and_then(|metadata| metadata.modified().map(|x| Some(x))).unwrap_or(None);
				let pos = addons.binary_search_by_key(&modified, |x| x.1).unwrap_or_else(|pos| pos);
				addons.insert(pos, (gma, modified));
			}

			let mut addons_map: IndexMap<PathBuf, GMAFile> = IndexMap::with_capacity(addons.len());
			for (addon, _) in addons.into_iter() {
				addons_map.insert(addon.path.to_owned(), addon);
			}

			*self.addons.write() = addons_map;
		});
	}

	pub fn addon<P: AsRef<Path>>(&self, path: P) -> Option<GMAFile> {
		self.addons.read().get(path.as_ref()).cloned()
	}
}

/*
#[tauri::command]
fn installed_addons(page: u16) -> &'static [GMAFile] {

}
*/
use std::{
	collections::{BinaryHeap, HashMap},
	fs::DirEntry,
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicBool, Ordering},
		mpsc, Arc,
	},
};

use lazy_static::lazy_static;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::ser::SerializeTuple;
use steamworks::PublishedFileId;

use crate::{game_addons, webview::Addon, GMAFile};

lazy_static! {
	static ref DISCOVERY_POOL: ThreadPool = ThreadPoolBuilder::new().num_threads(3).build().unwrap();
}

#[derive(Debug)]
pub struct GameAddons {
	discovered: AtomicBool,
	paths: RwLock<HashMap<PathBuf, Arc<Addon>>>,
	pages: RwLock<Vec<Arc<Addon>>>,
}

impl GameAddons {
	pub fn init() -> GameAddons {
		GameAddons {
			discovered: AtomicBool::new(false),
			paths: RwLock::new(HashMap::new()),
			pages: RwLock::new(Vec::new()),
		}
	}

	fn gma_check(entry: Result<DirEntry, std::io::Error>) -> Option<(PathBuf, String)> {
		let path = entry.ok()?.path();
		if !path.is_file() {
			return None;
		}

		let extension = path.extension()?.to_string_lossy().to_lowercase();
		if extension != "gma" {
			return None;
		}

		let file_name = path.file_name()?.to_string_lossy().to_string();

		Some((path, (&file_name[..(file_name.len() - 4)]).to_owned()))
	}

	pub fn refresh(&self) {
		let mut gmod = if let Some(gmod) = app_data!().gmod_dir() {
			gmod
		} else {
			*self.paths.write() = HashMap::new();
			*self.pages.write() = Vec::new();
			return;
		};

		let addons_dir = gmod.join("GarrysMod/addons");

		gmod.push("GarrysMod/cache/workshop");
		let cache_dir = gmod;

		let (tx_metadata, rx_metadata) = mpsc::channel();
		let (tx, rx) = mpsc::channel();

		let tx_addons_metadata = tx_metadata.clone();
		DISCOVERY_POOL.spawn(move || {
			let addons = match addons_dir.read_dir() {
				Ok(addons) => addons,
				Err(_) => return,
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
							Some(id_op) => id = id_op,
						},
					}
				}

				tx_addons_metadata
					.send((path, if id == 0 { None } else { Some(PublishedFileId(id / 10)) }))
					.unwrap();
			}
		});

		let tx_cache_metadata = tx_metadata;
		DISCOVERY_POOL.spawn(move || {
			let cache = match cache_dir.read_dir() {
				Ok(cache) => cache,
				Err(_) => return,
			};

			for (path, file_name) in cache.filter_map(GameAddons::gma_check) {
				let id = match str::parse::<u64>(&file_name) {
					Ok(id) => Some(PublishedFileId(id)),
					Err(_ok) => None,
				};

				tx_cache_metadata.send((path, id)).unwrap();
			}
		});

		DISCOVERY_POOL.spawn(move || {
			while let Ok((path, id)) = rx_metadata.recv() {
				let mut gma = match GMAFile::open(&path) {
					Ok(gma) => gma,
					Err(_) => continue,
				};

				if let Some(id) = id {
					gma.set_ws_id(id);
				}

				ignore! { gma.metadata() };

				tx.send(gma).unwrap();
			}
		});

		{
			let mut pages = self.pages.write();
			*pages = Vec::new();

			let mut paths = self.paths.write();
			*paths = HashMap::new();

			let mut pages_heap = BinaryHeap::new();

			while let Ok(mut gma) = rx.recv() {
				let modified = gma
					.path
					.metadata()
					.and_then(|metadata| metadata.modified().map(|x| Some(x)))
					.unwrap_or(None);
				gma.modified = modified;

				let path = gma.path.to_owned();
				let gma: Arc<Addon> = Arc::new(gma.into());

				pages_heap.push(gma.clone());
				paths.insert(path, gma);
			}

			*pages = pages_heap.into_sorted_vec();

			println!("Discovered {} addons", paths.len());
		}

		self.discovered.store(true, Ordering::Release);

		// Download the first page from Steam
		browse_installed_addons(1);
	}

	pub fn discover_addons(&self) {
		if !self.discovered.load(Ordering::Acquire) {
			self.refresh();
		}
	}

	pub fn from_path<P: AsRef<Path>>(&self, path: P) -> Option<Arc<Addon>> {
		self.discover_addons();
		self.paths.read().get(path.as_ref()).cloned()
	}

	pub fn get_addons(&self) -> RwLockReadGuard<Vec<Arc<Addon>>> {
		self.discover_addons();
		self.pages.read()
	}
}

#[derive(Debug)]
pub struct InstalledAddonsPage(usize, MappedRwLockReadGuard<'static, [Arc<Addon>]>);
impl serde::Serialize for InstalledAddonsPage {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut tup = serializer.serialize_tuple(2)?;
		tup.serialize_element(&self.0)?;
		tup.serialize_element(self.1.as_ref())?;
		tup.end()
	}
}

#[tauri::command]
pub fn browse_installed_addons(page: u32) -> InstalledAddonsPage {
	game_addons!().discover_addons();

	let start = ((page.max(1) - 1) as usize) * crate::steam::RESULTS_PER_PAGE;
	InstalledAddonsPage(
		game_addons!().paths.read().len(),
		RwLockReadGuard::map(game_addons!().pages.read(), |read| {
			steam!().fetch_workshop_items(
				read.iter()
					.skip(start)
					.take(crate::steam::RESULTS_PER_PAGE)
					.filter_map(|x| x.installed().id)
					.collect(),
			);
			&read[start..(start + crate::steam::RESULTS_PER_PAGE).min(read.len())]
		}),
	)
}

#[tauri::command]
pub fn get_installed_addon(path: PathBuf) -> Option<Arc<Addon>> {
	game_addons!().paths.read().get(&path).cloned()
}

#[tauri::command]
pub fn downloader_extract_gmas(paths: Vec<PathBuf>) {
	let destination = &app_data!().settings.read().extract_destination;
	for path in paths.into_iter() {
		if path.is_file()
			&& match path.extension() {
				Some(extension) => extension.to_string_lossy().eq_ignore_ascii_case("gma"),
				None => false,
			} {
			if let Ok(mut gma) = GMAFile::open(&path) {
				let transaction = transaction!();
				webview_emit!(
					"ExtractionStarted",
					(transaction.id, path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap(), gma.id)
				);
				transaction.data(gma.size);
				ignore! { gma.extract(destination.clone(), transaction, false) };
			}
		}
	}
}

pub fn free_caches() {
	let mut paths = crate::game_addons!().paths.write();
	let mut pages = crate::game_addons!().pages.write();
	*paths = HashMap::new();
	*pages = Vec::new();
	crate::game_addons!().discovered.store(false, Ordering::Release);
}

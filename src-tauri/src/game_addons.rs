use std::{
	collections::{BinaryHeap, HashMap},
	fs::DirEntry,
	path::{Path, PathBuf},
	sync::{
		atomic::{AtomicU8, Ordering},
		mpsc, Arc,
	},
	time::SystemTime,
};

use lazy_static::lazy_static;
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use rayon::ThreadPool;
use serde::ser::SerializeTuple;
use steamworks::PublishedFileId;

use crate::{game_addons, gma::extract::ExtractGMAMut, webview::Addon, GMAFile};

lazy_static! {
	static ref DISCOVERY_POOL: ThreadPool = thread_pool!(4);
}

#[repr(u8)]
enum Discovered {
	No = 0,
	Discovering = 1,
	Yes = 2,
}
impl From<u8> for Discovered {
	#[inline(always)]
	fn from(n: u8) -> Self {
		unsafe { std::mem::transmute(n) }
	}
}
impl Into<u8> for Discovered {
	#[inline(always)]
	fn into(self) -> u8 {
		unsafe { std::mem::transmute(self) }
	}
}

#[derive(Debug)]
pub struct GameAddons {
	discovered: AtomicU8,
	paths: RwLock<HashMap<PathBuf, Arc<Addon>>>,
	pages: RwLock<Vec<Arc<Addon>>>,
	external: RwLock<HashMap<PathBuf, Option<Arc<Addon>>>>,
}

impl GameAddons {
	pub fn init() -> GameAddons {
		GameAddons {
			discovered: AtomicU8::new(Discovered::No.into()),
			paths: RwLock::new(HashMap::new()),
			pages: RwLock::new(Vec::new()),
			external: RwLock::new(HashMap::new()),
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

	pub fn get_ws_id<S: AsRef<str>>(_file_name: S) -> Option<PublishedFileId> {
		let _file_name = _file_name.as_ref();
		let file_name = _file_name.strip_prefix("ds_").unwrap_or(&_file_name);

		if let Ok(id) = str::parse::<u64>(file_name) {
			return Some(PublishedFileId(id));
		}

		let id = GameAddons::extract_suffix_ws_id(file_name);
		if id == 0 {
			None
		} else {
			Some(PublishedFileId(id))
		}
	}

	fn extract_suffix_ws_id<S: AsRef<str>>(file_name: S) -> u64 {
		let mut id = 0u64;
		for char in file_name.as_ref()
			.chars()
			.rev() // Reverse iterator so we're looking at the suffix (the PublishedFileId)
			.take_while(|c| c.is_digit(10))
			.collect::<Vec<char>>()
			.into_iter()
			.rev()
		{
			match id.checked_add(char::to_digit(char, 10).unwrap() as u64) {
				None => return 0,
				Some(id_op) => match 10_u64.checked_mul(id_op) {
					None => return 0,
					Some(id_op) => id = id_op,
				},
			}
		}
		id
	}

	fn get_workshop_content_dir<P: AsRef<Path>>(gmod: P) -> Option<PathBuf> {
		Some(gmod.as_ref().parent()?.parent()?.join("workshop/content/4000"))
	}

	pub fn refresh(&self) {
		self.discovered.store(Discovered::Discovering.into(), Ordering::Release);

		let mut gmod = if let Some(gmod) = app_data!().gmod_dir() {
			gmod
		} else {
			*self.paths.write() = HashMap::new();
			*self.pages.write() = Vec::new();

			self.discovered.store(Discovered::No.into(), Ordering::Release);
			return;
		};

		let workshop_content_dir = GameAddons::get_workshop_content_dir(&gmod);

		let addons_dir = gmod.join("GarrysMod/addons");

		gmod.push("GarrysMod/cache/workshop");
		let cache_dir = gmod;

		let (tx_metadata, rx_metadata) = mpsc::channel();
		let (tx, rx) = mpsc::channel();

		if let Some(workshop_content_dir) = workshop_content_dir {
			let tx_workshop_content_metadata = tx_metadata.clone();
			DISCOVERY_POOL.spawn(move || {
				let addons = match workshop_content_dir.read_dir() {
					Ok(addons) => addons,
					Err(_) => return,
				};

				for (id, mut read_dir) in addons.filter_map(|entry| {
					entry.ok().and_then(|entry| if entry.file_type().ok()?.is_dir() {
						let read_dir = entry.path().read_dir().ok()?;
						let id = entry.file_name().to_string_lossy().parse::<u64>().ok()?;
						Some((id, read_dir))
					} else {
						None
					})
				}) {
					if let Some(gma_path) = {
						read_dir.find_map(|entry| {
							entry.ok().and_then(|entry| {
								if entry.file_type().ok()?.is_file() && entry.path().extension().and_then(|x| x.to_str())? == "gma" {
									Some(entry.path().to_path_buf())
								} else {
									None
								}
							})
						})
					} {
						tx_workshop_content_metadata.send((gma_path, if id == 0 { None } else { Some(PublishedFileId(id)) })).unwrap();
					}
				}
			});
		}

		let tx_addons_metadata = tx_metadata.clone();
		DISCOVERY_POOL.spawn(move || {
			let addons = match addons_dir.read_dir() {
				Ok(addons) => addons,
				Err(_) => return,
			};

			for (path, _file_name) in addons.filter_map(GameAddons::gma_check) {
				let file_name = _file_name.strip_prefix("ds_").unwrap_or(&_file_name);
				let id = GameAddons::extract_suffix_ws_id(file_name);

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

		let ids = {
			let mut ids = Vec::new();

			let mut pages = self.pages.write();
			*pages = Vec::new();

			let mut paths = self.paths.write();
			*paths = HashMap::new();

			let mut pages_heap = BinaryHeap::new();

			while let Ok(mut gma) = rx.recv() {
				let modified = gma
					.path
					.metadata()
					.and_then(|metadata| {
						metadata
							.modified()
							.map(|x| Some(x.duration_since(SystemTime::UNIX_EPOCH).map(|dur| dur.as_secs()).unwrap_or(0)))
					})
					.unwrap_or(None);
				gma.modified = modified;

				if let Some(id) = gma.id {
					ids.push(id);
				}

				let path = gma.path.to_owned();
				let gma: Arc<Addon> = Arc::new(gma.into());

				pages_heap.push(gma.clone());
				paths.insert(path, gma);
			}

			*pages = pages_heap.into_sorted_vec();

			search!().add_bulk(&pages);

			println!("Discovered {} addons", paths.len());

			ids
		};

		self.discovered.store(Discovered::Yes.into(), Ordering::Release);

		browse_installed_addons(1); // Download the first page first
		steam!().fetch_workshop_items(ids); // Download the rest in the background too
	}

	pub fn discover_addons(&self) {
		main_thread_forbidden!();

		match self.discovered.load(Ordering::Acquire).into() {
			Discovered::Yes => {}
			Discovered::No => self.refresh(),
			Discovered::Discovering => loop {
				sleep_ms!(25);
				game_addons!().discover_addons();
			},
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
	game_addons!().discover_addons();

	if let Some(cached) = game_addons!().external.read().get(&path) {
		return cached.clone();
	}

	if path.is_absolute() && path.is_file() && crate::path::has_extension(&path, "gma") {
		match GMAFile::open(&path) {
			Ok(mut gma) => {
				if let Some(id) = GameAddons::get_ws_id(path.file_name().unwrap().to_string_lossy()) {
					gma.set_ws_id(id);
				}

				ignore! { gma.metadata() };

				let gma = Arc::new(Addon::Installed(gma));
				game_addons!().external.write().insert(path, Some(gma.clone()));
				Some(gma)
			}
			Err(_) => {
				game_addons!().external.write().insert(path, None);
				None
			}
		}
	} else {
		None
	}
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
					(
						transaction.id,
						Some(path.clone()),
						path.file_name().map(|x| x.to_string_lossy().to_string()).unwrap(),
						gma.id
					)
				);
				transaction.data((turbonone!(), path.metadata().map(|metadata| metadata.len()).unwrap_or(0)));
				ignore! { gma.extract(destination.clone(), &transaction, false, true) };
			}
		}
	}
}

pub fn free_caches() {
	let mut paths = crate::game_addons!().paths.write();
	let mut pages = crate::game_addons!().pages.write();
	*paths = HashMap::new();
	*pages = Vec::new();
	crate::game_addons!().discovered.store(Discovered::No.into(), Ordering::Release);
}

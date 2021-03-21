use std::{borrow::Borrow, collections::{HashMap, hash_map::Entry}, fs::{DirEntry, File}, io::BufReader, mem::MaybeUninit, path::PathBuf, sync::{Arc, Mutex, MutexGuard, RwLock, mpsc::{self, Receiver, Sender}}};
use anyhow::{anyhow, Error};
use serde::Serialize;

use gma::GMAMetadata;
use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{util, gma::{self, ExtractDestination, GMAFile, GMAReadError}, transactions::Transactions, workshop::{WorkshopItem}};
use super::show;

pub(crate) struct GameAddons {
	pub(crate) total: u32,
	pub(crate) previewing: Option<PathBuf>,
	pub(crate) gma_cache: Option<GMACache>,
}

#[derive(Default)]
pub(crate) struct GMACache {
	pub(crate) installed_gmas: Vec<(PathBuf, Option<PublishedFileId>)>,
	pub(crate) metadata: RwLock<HashMap<PathBuf, GMAFile>>,
	pub(crate) ws_metadata: RwLock<HashMap<PublishedFileId, Option<WorkshopItem>>>,
}
impl GMACache {
	pub(crate) fn gma_metadata(&self, path: &PathBuf, id: Option<PublishedFileId>) -> Result<GMAFile, GMAReadError> {
		let r_lock = self.metadata.read().unwrap();
		if let Some(cached) = r_lock.get(path) {
			Ok(cached.clone())
		} else {
			drop(r_lock);

			let mut gma = GMAFile::new(path, id)?;
			gma.entries()?;
			gma.metadata()?;

			let mut metadata_cache = self.metadata.write().unwrap();
			metadata_cache.insert(path.clone(), gma.clone());

			Ok(gma)
		}
	}

	pub(crate) fn ws_metadata(&self, id: PublishedFileId, fetch_owner: bool) -> Option<WorkshopItem> {
		let r_lock = self.ws_metadata.read().unwrap();

		if let Some(cached) = r_lock.get(&id).cloned() {

			if !fetch_owner || cached.is_none() { return cached }

			let item = cached.as_ref().unwrap();
			if item.owner.is_none() && item.steamid.is_some() {

				drop(item); drop(r_lock);

				let workshop = crate::WORKSHOP.write().unwrap();
				let steam_user = workshop.query_user(item.steamid.unwrap());

				let mut ws_metadata_cache = self.ws_metadata.write().unwrap();
				let mut item = ws_metadata_cache.get_mut(&id).unwrap().as_mut().unwrap();
				item.owner = Some(steam_user);
				
				Some(item.clone())

			} else {
				cached
			}

		} else {

			drop(r_lock);

			let workshop = crate::WORKSHOP.write().unwrap();
			let mut ws_addon = workshop.get_item(id).unwrap().ok()?;
			
			if fetch_owner {
				if let Some(ws_addon) = &mut ws_addon {
					if let Some(steamid) = ws_addon.steamid {
						ws_addon.owner = Some(workshop.query_user(steamid));
					}
				}
			}

			let mut ws_metadata_cache = self.ws_metadata.write().unwrap();
			ws_metadata_cache.insert(id, ws_addon.clone());

			ws_addon

		}
	}
}

impl GameAddons {
	pub(crate) fn init() -> GameAddons {
		GameAddons {
			total: 0,
			gma_cache: None,
			previewing: None
		}
	}
}

pub(crate) fn cache_addon_paths() -> bool {
	let app_data = crate::APP_DATA.read().unwrap();
	let dir = app_data.gmod.as_ref().unwrap();

	let mut game_addons = crate::GAME_ADDONS.write().unwrap();
	if let None = game_addons.gma_cache {
		let mut gma_cache = GMACache::default();

		let mut local_files: Vec<(u64, PathBuf, Option<PublishedFileId>)> = Vec::new();

		match dir.join("GarrysMod/addons").read_dir() {
			Ok(addons_gmas) => 'paths:

			for (modified, file_name, path) in addons_gmas.filter_map(|r| {
				let r = r.ok()?;
				if r.path().is_file() {
					let file_name = r.file_name().to_string_lossy().strip_suffix(".gma").map(ToOwned::to_owned)?;

					let mut canonicalized = r.path();
					canonicalized = dunce::canonicalize(canonicalized.clone()).unwrap_or(canonicalized);

					Some(
						(util::get_modified_time(&r).unwrap_or(0), file_name, canonicalized)
					)
				} else {
					None
				}
			})

			{

				let mut id = 0u64;

				for char in file_name.chars()
					.rev() // Reverse iterator so we're looking at the suffix (the PublishedFileId)
					.take_while(|c| c.is_digit(10)) // Only capture digits
					.collect::<Vec<char>>()
					.into_iter()
					.rev() // Reverse again
				{
					match id.checked_add(char::to_digit(char, 10).unwrap() as u64) { None => continue 'paths, Some(id_op) => {
						match 10_u64.checked_mul(id_op) { None => continue 'paths, Some(id_op) => {
							id = id_op;
						}}
					}}
				}

				let id = if id == 0 { None } else { Some(PublishedFileId(id / 10)) };
				if
					id.is_none() ||
					local_files.binary_search_by_key(&id, |t| t.2).is_err()
				{
					local_files.push((modified, path, id))
				}

			},

			Err(error) => show::panic(format!("Failed to scan game directory for workshop addons!\n{:#?}", error))
		}
	
		match dir.join("GarrysMod/cache/workshop").read_dir() {
			Ok(entries) => for entry in entries.filter_map(|r| r.ok()) {
				let path = entry.path();
				if path.extension().unwrap_or_default() != "gma" { continue }
			
				match str::parse::<u64>(match entry.file_name().to_str() {
					Some(id) => id,
					None => continue
				}) {
					Ok(id) => {
						let id = Some(PublishedFileId(id));
						match local_files.binary_search_by_key(&id, |t| t.2) {
							Ok(_) => {},
							Err(_) => local_files.push((util::get_modified_time(&entry).unwrap_or(0), path, id))
						}
					},
					Err(_) => {}
				}
			},

			Err(error) => show::panic(format!("Failed to scan game directory for workshop addons!\n{:#?}", error))
		}

		local_files.sort_unstable_by_key(|k| k.0);
		gma_cache.installed_gmas = local_files.into_iter().map(|k| (k.1, k.2)).collect();
		game_addons.total = gma_cache.installed_gmas.len() as u32;

		game_addons.gma_cache = Some(gma_cache);

		false
	} else {
		true
	}
}

pub(crate) fn browse(resolve: String, reject: String, webview: &mut Webview<'_>, page: u32) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		cache_addon_paths();
		
		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let gma_cache = game_addons.gma_cache.as_ref().unwrap();
		
		let page_items: Vec<(&PathBuf, &PublishedFileId)> = gma_cache.installed_gmas.iter()
			.skip(((page - 1) * 50) as usize).take(50)
			.filter(|entry| entry.1.is_some())
			.map(|(path, id)| (path, id.as_ref().unwrap()))
		.collect();

		Ok(match crate::WORKSHOP.read().unwrap().get_items(
			page_items.iter().map(|entry| entry.1).cloned().collect()
		).unwrap() {
			Ok(data) => (
				game_addons.total,
				data.1.into_iter().enumerate().map(|(i, item)| (page_items[i].0.clone(), item)).collect::<Vec<(PathBuf, WorkshopItem)>>()
			),
			Err(_) => {
				// TODO spawn a thread which reads metadata
				(
					game_addons.total,
					page_items.into_iter().map(|(path, id)| (path.clone(), WorkshopItem::from(*id))).collect::<Vec<(PathBuf, WorkshopItem)>>()
				)
			},
		})
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_gma_metadata(resolve: String, reject: String, webview: &mut Webview<'_>, path: PathBuf, id: Option<PublishedFileId>) -> Result<(), String> {
	tauri::execute_promise(webview, move || {
		
		crate::GAME_ADDONS.read().unwrap()
			.gma_cache.as_ref().unwrap()
			.gma_metadata(&path, id)
			.map(|t| t.clone())
			.map_err(|_| {
				anyhow!(
					path.file_name()
						.and_then(|s| Some(s.to_string_lossy().to_string()))
						.unwrap_or_else(|| {
							id
								.and_then(|id| Some(id.0.to_string()))
								.unwrap_or_else(|| path.to_string_lossy().to_string())
						})
				)
			})
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_gma_ws_owner(resolve: String, reject: String, webview: &mut Webview<'_>, id: PublishedFileId) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		Ok(
			crate::GAME_ADDONS.read().unwrap()
				.gma_cache.as_ref().unwrap()
				.ws_metadata(id, true)
		)

	}, resolve, reject);

	Ok(())
}

pub(crate) fn preview_gma(resolve: String, reject: String, webview: &mut Webview<'_>, path: PathBuf, id: Option<PublishedFileId>) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let lock_r = crate::GAME_ADDONS.read().unwrap();

		let result = lock_r
			.gma_cache.as_ref().unwrap()
			.gma_metadata(&path, id)
			.map_err(|_| anyhow!(""))
			.map(|f| f.clone());

		drop(lock_r);

		if result.is_ok() {
			crate::GAME_ADDONS.write().unwrap().previewing = Some(path.clone());
		}
		
		result
		
	}, resolve, reject);

	Ok(())
}

pub(crate) fn open_gma_preview_entry(resolve: String, reject: String, webview: &mut Webview<'_>, entry_path: String) -> Result<(), String> {
	if crate::GAME_ADDONS.read().unwrap().previewing.is_none() { return Ok(()) }

	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		let transaction = Transactions::new(webview_mut);
		let id = transaction.id;

		std::thread::spawn(move || {
			let progress_transaction = transaction.build();
			let channel = progress_transaction.channel();

			let game_addons = crate::GAME_ADDONS.read().unwrap();
			let preview_path = game_addons.previewing.as_ref().unwrap();

			match game_addons.gma_cache.as_ref().unwrap().gma_metadata(preview_path, None) {
				Err(error) => channel.error(&format!("{}", error), ()),

				Ok(preview_gma) => {
					let extract_dest = preview_gma.extract_entry(
						entry_path,
						ExtractDestination::Temp
					).unwrap();
		
					channel.finish(());
		
					show::open(extract_dest.to_str().unwrap());
				},
			}
		});
		
		Ok(id)
		
	}, resolve, reject);

	Ok(())
}

// TODO change all args to resolve, reject, webview, ...

pub(crate) fn extract_gma_preview(resolve: String, reject: String, webview: &mut Webview<'_>, path: Option<PathBuf>, named_dir: bool, tmp: bool, downloads: bool, addons: bool) -> Result<(), String> {
	if crate::GAME_ADDONS.read().unwrap().previewing.is_none() { return Ok(()) }

	let save_destination_path = path.is_some();
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {
		
		let transaction = Transactions::new(webview_mut.clone());
		let id = transaction.id;

		std::thread::spawn(move || {
			let transaction = transaction.build();
			let channel = transaction.channel();

			let mut use_named_dir = named_dir;

			let dest = match tmp {
				true => ExtractDestination::Temp,
				false => {
					let mut check_exists = true;
					let mut discriminated_path = MaybeUninit::<PathBuf>::uninit();
					unsafe {
						if addons {
							use_named_dir = true;
							*discriminated_path.as_mut_ptr() = crate::APP_DATA.read().unwrap().gmod.as_ref().unwrap().join("garrysmod/addons");
						} else if downloads {
							use_named_dir = true;
							*discriminated_path.as_mut_ptr() = dirs::download_dir().unwrap();
						} else {
							check_exists = false;
							*discriminated_path.as_mut_ptr() = path.unwrap();
						}
					}
	
					let discriminated_path = unsafe { discriminated_path.assume_init() };
					if discriminated_path.is_absolute() && (!check_exists || discriminated_path.exists()) {
						match use_named_dir {
							true => ExtractDestination::NamedDirectory(discriminated_path),
							false => ExtractDestination::Directory(discriminated_path)
						}
					} else {
						channel.error("ERR_EXTRACT_INVALID_DEST", ()); // TODO internationalize
						return;
					}
				}
			};

			match 
				{
					let game_addons = crate::GAME_ADDONS.read().unwrap();
					let preview_path = game_addons.previewing.as_ref().unwrap();

					game_addons.gma_cache.as_ref().unwrap().gma_metadata(preview_path, None)
						.and_then(|preview_gma| {
							let channel = transaction.channel();
							preview_gma.extract(dest, Some(Box::new(move |progress| channel.progress(progress))))
						})
				}
			{
				Ok(mut path) => {
					show::open(&path.to_string_lossy().to_string());

					channel.finish(());
					
					if save_destination_path {
						if use_named_dir { path.pop(); }
						let mut app_data = crate::APP_DATA.write().unwrap();
						let settings = &mut app_data.settings;
						if let Err(_) = settings.destinations.binary_search(&path) {
							settings.destinations.push(path);
							if let Ok(_) = settings.save(None) {
								settings.send(webview_mut);
							} else {
								settings.destinations.pop();
							}
						}
					}
				},
				Err(err) => channel.error("ERR_EXTRACT_IO_ERROR", format!("{}", err))
			}
		});

		Ok(id)

	}, resolve, reject);

	Ok(())
}
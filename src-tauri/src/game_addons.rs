use anyhow::anyhow;
use std::{collections::HashMap, fs::DirEntry, path::PathBuf, sync::{Arc, RwLock, mpsc::{self, Receiver, Sender}}};

use steamworks::PublishedFileId;
use tauri::Webview;

use super::show;
use crate::{gma::{ExtractDestination, GMAFile, GMAReadError}, transactions::Transactions, util::{self, RwLockDebug}, workshop::WorkshopItem};

use crate::transaction_data;

pub(crate) struct GameAddons {
	pub(crate) total: u32,
	pub(crate) previewing: Option<PathBuf>,
	pub(crate) gma_cache: Option<GMACache>,
}

#[derive(Default)]
pub(crate) struct GMACache {
	pub(crate) installed_gmas: Vec<(PathBuf, Option<PublishedFileId>)>,
	pub(crate) installed_ids: Vec<(PathBuf, PublishedFileId)>,
	pub(crate) metadata: RwLockDebug<HashMap<PathBuf, GMAFile>>,
	pub(crate) ws_metadata: RwLockDebug<HashMap<PublishedFileId, Option<WorkshopItem>>>,
}
impl GMACache {
	pub(crate) fn gma_metadata(
		&self,
		path: &PathBuf,
		id: Option<PublishedFileId>,
	) -> Result<GMAFile, GMAReadError> {
		let r_lock = self.metadata.read();
		if let Some(cached) = r_lock.get(path) {
			Ok(cached.clone())
		} else {
			drop(r_lock);

			let mut gma = GMAFile::new(path, id)?;
			gma.entries()?;
			gma.metadata()?;

			let mut metadata_cache = self.metadata.write();
			metadata_cache.insert(path.clone(), gma.clone());

			Ok(gma)
		}
	}

	pub(crate) fn ws_metadata(
		&self,
		id: PublishedFileId,
		fetch_owner: bool,
	) -> Option<WorkshopItem> {
		let r_lock = self.ws_metadata.read();

		if let Some(cached) = r_lock.get(&id).cloned() {
			if !fetch_owner || cached.is_none() {
				return cached;
			}

			let item = cached.as_ref().unwrap();
			if item.owner.is_none() && item.steamid.is_some() {
				drop(item);
				drop(r_lock);

				let workshop = crate::WORKSHOP.write();
				let steam_user = workshop.query_user(item.steamid.unwrap());

				let mut ws_metadata_cache = self.ws_metadata.write();
				let mut item = ws_metadata_cache.get_mut(&id).unwrap().as_mut().unwrap();
				item.owner = Some(steam_user);

				Some(item.clone())
			} else {
				cached
			}
		} else {
			drop(r_lock);

			let workshop = crate::WORKSHOP.write();
			let mut ws_addon = workshop.get_item(id).unwrap().ok()?;

			if fetch_owner {
				if let Some(ws_addon) = &mut ws_addon {
					if let Some(steamid) = ws_addon.steamid {
						ws_addon.owner = Some(workshop.query_user(steamid));
					}
				}
			}

			let mut ws_metadata_cache = self.ws_metadata.write();
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
			previewing: None,
		}
	}
}

pub(crate) fn cache_addon_paths() -> bool {
	let app_data = crate::APP_DATA.read();
	let dir = app_data.gmod.as_ref().unwrap().to_owned();

	let game_addons = crate::GAME_ADDONS.read();
	if let None = game_addons.gma_cache {
		drop(game_addons);

		let (tx_modified_path_id, rx): (
			Sender<(u64, PathBuf, Option<PublishedFileId>)>,
			Receiver<(u64, PathBuf, Option<PublishedFileId>)>,
		) = mpsc::channel();
		let local_files = std::thread::spawn(move || {
			let mut modified_times: (Vec<u64>, Vec<u64>) = (Vec::new(), Vec::new());
			let mut local_files: Vec<(PathBuf, Option<PublishedFileId>)> = Vec::new();
			let mut installed_ids: Vec<(PathBuf, PublishedFileId)> = Vec::new();
			loop {
				let (modified, path, id) = match rx.recv() {
					Ok(data) => data,
					Err(_) => break,
				};

				if let Some(id) = id {
					let pos = match modified_times.1.binary_search(&modified) {
						Ok(pos) => pos,
						Err(pos) => pos,
					};

					modified_times.1.insert(pos, modified);
					installed_ids.insert(pos, (path.clone(), id));
				}

				let pos = match modified_times.0.binary_search(&modified) {
					Ok(pos) => pos,
					Err(pos) => pos,
				};

				modified_times.0.insert(pos, modified);
				local_files.insert(pos, (path, id));
			}
			(local_files, installed_ids)
		});

		let (tx_entry_id, rx): (
			Sender<(DirEntry, Option<PublishedFileId>)>,
			Receiver<(DirEntry, Option<PublishedFileId>)>,
		) = mpsc::channel();
		std::thread::spawn(move || loop {
			let (entry, id) = match rx.recv() {
				Ok(data) => data,
				Err(_) => break,
			};

			let mut canonicalized = entry.path();
			canonicalized = dunce::canonicalize(canonicalized.clone()).unwrap_or(canonicalized);

			tx_modified_path_id
				.send((
					util::get_modified_time(&entry).unwrap_or(0),
					canonicalized,
					id,
				))
				.unwrap();
		});

		{
			let dir = dir.clone();
			let tx_entry_id = tx_entry_id.clone();
			std::thread::spawn(move || {
				match dir.join("GarrysMod/addons").read_dir() {
					Ok(addons_gmas) => {
						'paths: for (entry, file_name) in addons_gmas.filter_map(|r| {
							let entry = r.ok()?;
							if entry.path().is_file() {
								let file_name = entry
									.file_name()
									.to_string_lossy()
									.strip_suffix(".gma")
									.map(ToOwned::to_owned)?;
								Some((entry, file_name))
							} else {
								None
							}
						}) {
							let mut id = 0u64;

							for char in file_name
								.chars()
								.rev() // Reverse iterator so we're looking at the suffix (the PublishedFileId)
								.take_while(|c| c.is_digit(10)) // Only capture digits
								.collect::<Vec<char>>()
								.into_iter()
								.rev()
							// Reverse again
							{
								match id.checked_add(char::to_digit(char, 10).unwrap() as u64) {
									None => continue 'paths,
									Some(id_op) => match 10_u64.checked_mul(id_op) {
										None => continue 'paths,
										Some(id_op) => {
											id = id_op;
										}
									},
								}
							}

							if id == 0 {
								tx_entry_id.send((entry, None)).unwrap();
							} else {
								tx_entry_id
									.send((entry, Some(PublishedFileId(id / 10))))
									.unwrap();
							}
						}
					}

					Err(error) => show::panic(format!(
						"Failed to scan game directory for workshop addons!\n{:#?}",
						error
					)),
				}
			});
		}

		std::thread::spawn(
			move || match dir.join("GarrysMod/cache/workshop").read_dir() {
				Ok(entries) => {
					for r in entries {
						let entry = if r.is_ok() { r.unwrap() } else { continue };
						if entry.path().extension().unwrap_or_default() != "gma" {
							continue;
						}

						match str::parse::<u64>(match entry.file_name().to_str() {
							Some(id) => id.strip_suffix(".gma").unwrap_or(id),
							None => continue,
						}) {
							Ok(id) => tx_entry_id
								.send((entry, Some(PublishedFileId(id))))
								.unwrap(),
							Err(_ok) => tx_entry_id.send((entry, None)).unwrap(),
						};
					}
				}

				Err(error) => show::panic(format!(
					"Failed to scan game directory for workshop addons!\n{:#?}",
					error
				)),
			},
		);

		let mut gma_cache = GMACache::default();
		let (installed_gmas, installed_ids) = local_files.join().unwrap();
		gma_cache.installed_gmas = installed_gmas;
		gma_cache.installed_ids = installed_ids;

		let mut game_addons = crate::GAME_ADDONS.write();
		game_addons.total = gma_cache.installed_gmas.len() as u32;
		game_addons.gma_cache = Some(gma_cache);

		false
	} else {
		true
	}
}

pub(crate) fn browse(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	page: u32,
) -> Result<(), String> {
	tauri::execute_promise(
		webview,
		move || {
			cache_addon_paths();

			let game_addons = crate::GAME_ADDONS.read();
			let gma_cache = game_addons.gma_cache.as_ref().unwrap();

			let page_items: Vec<(PathBuf, PublishedFileId)> = gma_cache
				.installed_ids
				.iter()
				.skip(((page - 1) * 50) as usize)
				.take(50)
				.cloned()
				.collect();

			// FIXME: Duplicate Ids cause issues with this

			Ok(
				match crate::WORKSHOP
					.read()
					.get_items(page_items.iter().map(|entry| entry.1).collect())
					.unwrap()
				{
					Ok(data) => (
						game_addons.total,
						data.1
							.into_iter()
							.enumerate()
							.map(|(i, item)| (page_items[i].0.clone(), item))
							.collect::<Vec<(PathBuf, WorkshopItem)>>(),
					),
					Err(_) => {
						// TODO spawn a thread which reads metadata
						(
							game_addons.total,
							page_items
								.into_iter()
								.map(|(path, id)| (path, WorkshopItem::from(id)))
								.collect::<Vec<(PathBuf, WorkshopItem)>>(),
						)
					}
				},
			)
		},
		resolve,
		reject,
	);
	Ok(())
}

pub(crate) fn get_gma_metadata(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	path: PathBuf,
	id: Option<PublishedFileId>,
) -> Result<(), String> {
	cache_addon_paths();

	tauri::execute_promise(
		webview,
		move || {
			crate::GAME_ADDONS
				.read()
				.gma_cache
				.as_ref()
				.unwrap()
				.gma_metadata(&path, id)
				.map(|t| t.clone())
				.map_err(|_| {
					anyhow!(path
						.file_name()
						.and_then(|s| Some(s.to_string_lossy().to_string()))
						.unwrap_or_else(|| {
							id.and_then(|id| Some(id.0.to_string()))
								.unwrap_or_else(|| path.to_string_lossy().to_string())
						}))
				})
		},
		resolve,
		reject,
	);
	Ok(())
}

pub(crate) fn get_gma_ws_uploader(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	id: PublishedFileId,
) -> Result<(), String> {
	cache_addon_paths();

	tauri::execute_promise(
		webview,
		move || {
			Ok(crate::GAME_ADDONS
				.read()
				.gma_cache
				.as_ref()
				.unwrap()
				.ws_metadata(id, true)
				.unwrap_or_else(|| id.into())
				.owner)
		},
		resolve,
		reject,
	);

	Ok(())
}

pub(crate) fn get_gma_ws_metadata(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	id: PublishedFileId,
) -> Result<(), String> {
	cache_addon_paths();

	tauri::execute_promise(
		webview,
		move || {
			Ok(crate::GAME_ADDONS
				.read()
				.gma_cache
				.as_ref()
				.unwrap()
				.ws_metadata(id, false)
				.unwrap_or_else(|| id.into()))
		},
		resolve,
		reject,
	);

	Ok(())
}

pub(crate) fn preview_gma(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	path: PathBuf,
	id: Option<PublishedFileId>,
) -> Result<(), String> {
	cache_addon_paths();

	tauri::execute_promise(
		webview,
		move || {
			let result = {
				crate::GAME_ADDONS
					.read()
					.gma_cache
					.as_ref()
					.unwrap()
					.gma_metadata(&path, id)
					.map_err(|_| anyhow!(""))
					.map(|f| f.clone())
			};

			if result.is_ok() {
				crate::GAME_ADDONS.write().previewing = Some(path.clone());
			}

			result
		},
		resolve,
		reject,
	);

	Ok(())
}

pub(crate) fn open_gma_preview_entry(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	entry_path: String,
) -> Result<(), String> {
	if crate::GAME_ADDONS.read().previewing.is_none() {
		return Ok(());
	}

	let webview_mut = webview.as_mut();

	tauri::execute_promise(
		webview,
		move || {
			let transaction = Transactions::new(webview_mut);
			let id = transaction.id;

			std::thread::spawn(move || {
				let progress_transaction = transaction.build();
				let channel = progress_transaction.channel();

				let game_addons = crate::GAME_ADDONS.read();
				let preview_path = game_addons.previewing.as_ref().unwrap();

				match game_addons
					.gma_cache
					.as_ref()
					.unwrap()
					.gma_metadata(preview_path, None)
				{
					Err(error) => channel.error(&format!("{}", error), transaction_data!(())),

					Ok(preview_gma) => {
						let extract_dest = preview_gma
							.extract_entry(entry_path, ExtractDestination::Temp)
							.unwrap();

						channel.finish(transaction_data!(()));

						show::open(extract_dest.to_str().unwrap());
					}
				}
			});

			Ok(id)
		},
		resolve,
		reject,
	);

	Ok(())
}

// TODO change all args to resolve, reject, webview, ...

pub(crate) fn extract_gma_preview(
	resolve: String,
	reject: String,
	webview: &mut Webview<'_>,
	tmp: bool,
	path: Option<PathBuf>,
	named_dir: bool,
	downloads: bool,
	addons: bool,
) -> Result<(), String> {
	if crate::GAME_ADDONS.read().previewing.is_none() {
		return Ok(());
	}

	let save_destination_path = path.is_some();
	let webview_mut = webview.as_mut();

	tauri::execute_promise(
		webview,
		move || {
			let transaction = Transactions::new(webview_mut.clone());
			let id = transaction.id;

			std::thread::spawn(move || {
				let transaction = transaction.build();
				let channel = transaction.channel();

				let (using_named_dir, dest) =
					match ExtractDestination::build(tmp, path, named_dir, downloads, addons) {
						Ok(dest) => (
							match &dest {
								&ExtractDestination::NamedDirectory(_) => true,
								_ => false,
							},
							dest,
						),
						Err(_) => {
							channel.error("ERR_EXTRACT_INVALID_DEST", transaction_data!(())); // TODO internationalize
							return;
						}
					};

				match {
					let game_addons = crate::GAME_ADDONS.read();
					let preview_path = game_addons.previewing.as_ref().unwrap();

					game_addons
						.gma_cache
						.as_ref()
						.unwrap()
						.gma_metadata(preview_path, None)
						.and_then(|preview_gma| {
							let channel = transaction.channel();
							preview_gma.extract(
								dest,
								Some(Box::new(move |progress| channel.progress(progress))),
							)
						})
				} {
					Ok(mut path) => {
						show::open(&path.to_string_lossy().to_string());

						channel.finish(transaction_data!(()));

						if save_destination_path {
							if using_named_dir {
								path.pop();
							}
							let mut app_data = crate::APP_DATA.write();
							let settings = &mut app_data.settings;
							if let Err(_) = settings.destinations.binary_search(&path) {
								settings.destinations.push(path);
								if let Ok(_) = settings.save(None) {
									app_data.send(webview_mut);
								} else {
									settings.destinations.pop();
								}
							}
						}
					}
					Err(err) => channel.error(
						"ERR_EXTRACT_IO_ERROR",
						transaction_data!(format!("{}", err)),
					),
				}
			});

			Ok(id)
		},
		resolve,
		reject,
	);

	Ok(())
}

use std::{borrow::Borrow, collections::{HashMap, hash_map::Entry}, fs::{DirEntry, File}, io::BufReader, mem::MaybeUninit, path::PathBuf, sync::{Arc, Mutex, MutexGuard}};
use anyhow::{anyhow, Error};

use gma::GMAMetadata;
use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{gma::{self, ExtractDestination, GMAFile}, workshop::{WorkshopItem}};
use super::show;

pub(crate) struct GameAddons {
	total: u32,
	previewing: Option<GMAFile>,
	gma_cache: Option<GMACache>,
}

#[derive(Default)]
struct GMACache {
	installed_ws_ids: Vec<PublishedFileId>,
	paths: HashMap<PublishedFileId, PathBuf>,
	gma_metadata: Arc<Mutex<HashMap<PathBuf, GMAFile>>>,
	gma_ws_metadata: Arc<Mutex<HashMap<PublishedFileId, Option<WorkshopItem>>>>,
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

fn get_modified_time(entry: &DirEntry) -> Result<u64, Error> {
	Ok(entry.metadata()?.modified()?.elapsed()?.as_secs())
}

fn cache_addon_paths() -> bool {
	let app_data = crate::APP_DATA.read().unwrap();
	let dir = app_data.gmod.as_ref().unwrap();

	let mut game_addons = crate::GAME_ADDONS.write().unwrap();
	if let None = game_addons.gma_cache {
		let mut gma_cache = GMACache::default();

		let mut local_files: Vec<(u64, PublishedFileId)> = Vec::new();

		match dir.join("GarrysMod/addons").read_dir() {
			Ok(addons_gmas) => {
				'paths: for (modified, file_name, path) in addons_gmas.filter_map(|r| {
					match r {
						Ok(r) => {
							if r.path().is_file() {
								match r.file_name().to_string_lossy().strip_suffix(".gma").map(ToOwned::to_owned) {
									Some(file_name) => {
										let mut canonicalized = r.path();
										canonicalized = dunce::canonicalize(canonicalized.clone()).unwrap_or(canonicalized);
										return Some(
											(get_modified_time(&r).unwrap_or(0), file_name, canonicalized)
										)
									},
									None => {}
								}
							}
						}
						Err(_) => {}
					}
					None
				}) {

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

					let id = PublishedFileId(id / 10);
					gma_cache.paths.insert(id, path);
					match local_files.binary_search_by_key(&id, |t| t.1) {
						Ok(_) => {},
						Err(pos) => local_files.insert(pos, (modified, id))
					}

				}
			},
			Err(error) => show::panic(format!("Failed to scan game directory for workshop addons!\n{:#?}", error))
		}
	
		match dir.join("GarrysMod/cache/workshop").read_dir() {
			Err(error) => show::panic(format!("Failed to scan game directory for workshop addons!\n{:#?}", error)),
			Ok(entries) => {
				for entry in entries.filter_map(|r| r.ok()) {
					if entry.path().extension().unwrap_or_default() != "gma" { continue }
				
					match str::parse::<u64>(match entry.file_name().to_str() {
						Some(id) => id,
						None => continue
					}) {
						Ok(id) => {
							let id = PublishedFileId(id);
							gma_cache.paths.insert(id, entry.path());
							match local_files.binary_search_by_key(&id, |t| t.1) {
								Ok(_) => {},
								Err(pos) => local_files.insert(pos, (get_modified_time(&entry).unwrap_or(0), id))
							}
						},
						Err(_) => {}
					}
				}
			}
		}

		local_files.sort_unstable_by_key(|k| k.0);
		gma_cache.installed_ws_ids = local_files.into_iter().map(|k| k.1).collect();
		game_addons.total = gma_cache.installed_ws_ids.len() as u32;

		game_addons.gma_cache = Some(gma_cache);

		false
	} else {
		true
	}
}

pub(crate) fn browse(resolve: String, reject: String, webview: &mut Webview<'_>, page: u32) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let was_cached = cache_addon_paths();
		
		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let gma_cache = game_addons.gma_cache.as_ref().unwrap();
		
		let page_items: Vec<PublishedFileId> = gma_cache.installed_ws_ids.iter().skip(((page - 1) * 50) as usize).take(50).cloned().collect();
		Ok(match crate::WORKSHOP.read().unwrap().get_items(page_items.clone()).unwrap() {
			Ok(data) => (
				game_addons.total,
				data.1,
				if !was_cached { Some(gma_cache.paths.clone()) } else { None }
			),
			Err(_) => (
				game_addons.total,
				page_items.into_iter().map(|id| id.into()).collect::<Vec<WorkshopItem>>(),
				if !was_cached { Some(gma_cache.paths.clone()) } else { None }
			)
		})
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_gma_paths(resolve: String, reject: String, webview: &mut Webview<'_>) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		cache_addon_paths();
		Ok(crate::GAME_ADDONS.read().unwrap().gma_cache.as_ref().unwrap().paths.clone())

	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_addon_metadata(resolve: String, reject: String, webview: &mut Webview<'_>, id: PublishedFileId) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		cache_addon_paths();
		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let gma_cache = game_addons.gma_cache.as_ref().unwrap();

		let path = match gma_cache.paths.get(&id) {
			Some(path) => path,
			None => return Ok(None)
		};

		let generate_error = |_| {
			anyhow!(
				path.file_name()
					.and_then(|s| Some(s.to_string_lossy().to_string()))
					.unwrap_or(id.0.to_string())
			)
		};

		let mut gma_metadata_cache = gma_cache.gma_metadata.lock().unwrap();
		match gma_metadata_cache.entry(path.clone()) {
			Entry::Occupied(mut o) => {
				let gma = o.get_mut();
				gma.metadata().map_err(generate_error)?;
				gma.close();
				Ok(Some(gma.clone()))
			},
			Entry::Vacant(v) => {
				let mut gma = GMAFile::new(path, Some(id)).map_err(generate_error)?;
				gma.metadata().map_err(generate_error)?;
				gma.close();
				v.insert(gma.clone());
				Ok(Some(gma))
			}
		}
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn preview_gma(resolve: String, reject: String, webview: &mut Webview<'_>, path: PathBuf, id: Option<PublishedFileId>) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let mut game_addons = crate::GAME_ADDONS.write().unwrap();
		let gma_cache = game_addons.gma_cache.as_ref().unwrap();

		let metadata_cache = gma_cache.gma_metadata.clone();
		let get_metadata = move || {
			match metadata_cache.lock().unwrap().entry(path.clone()) {
				Entry::Occupied(mut o) => {
					let gma = o.get_mut();
					gma.entries().unwrap();
					Ok(gma.clone())
				},
				Entry::Vacant(v) => {
					match GMAFile::new(&path, id) {
						Ok(mut gma) => {
							gma.metadata().unwrap();
							gma.entries().unwrap();
							gma.close();
							v.insert(gma.clone());
							Ok(gma)
						},
						Err(error) => Err(anyhow!(format!("{}", error)))
					}
				}
			}
		};
		
		let (mut gma, ws_metadata) = match &id {
			Some(_) => {
				let ws_metadata_cache = gma_cache.gma_ws_metadata.clone();
				let ws_metadata = std::thread::spawn(move || {
					match ws_metadata_cache.lock().unwrap().entry(id.unwrap()) {
						Entry::Occupied(o) => {
							Ok(o.get().clone())
						},
						Entry::Vacant(v) => {
							let workshop = crate::WORKSHOP.write().unwrap();
							workshop.get_item(id.unwrap()).unwrap().and_then(|ws_addon| {
								v.insert(ws_addon.clone());
								Ok(ws_addon.and_then(|mut ws_addon| {
									if let Some(steamid) = ws_addon.steamid {
										ws_addon.owner = Some(workshop.query_user(steamid));
									}
									Some(ws_addon)
								}))
							})
						}
					}
				});

				let metadata = std::thread::spawn(get_metadata);
		
				(metadata.join().unwrap()?, ws_metadata.join().unwrap().unwrap_or_default())
			},
			None => {
				(get_metadata()?, None)
			}
		};

		gma.close();
		game_addons.previewing = Some(gma.clone());

		Ok((gma, ws_metadata))
		
	}, resolve, reject);

	Ok(())
}

pub(crate) fn open_gma_preview_entry(resolve: String, reject: String, webview: &mut Webview<'_>, entry_path: String) -> Result<(), String> {
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		let transaction = transactions.new(webview_mut);
		let id = transaction.id;

		std::thread::spawn(move || {
			let progress_transaction = transaction.build();
			let channel = progress_transaction.channel();

			let mut game_addons = crate::GAME_ADDONS.write().unwrap();
			let preview_gma = game_addons.previewing.as_mut().unwrap();

			let extract_dest = preview_gma.extract_entry(
				entry_path,
				ExtractDestination::Temp
			).unwrap();

			channel.finish(());

			show::open(extract_dest.to_str().unwrap());
		});
		
		Ok(id)
		
	}, resolve, reject);

	Ok(())
}

// TODO change all args to resolve, reject, webview, ...

pub(crate) fn extract_gma_preview(resolve: String, reject: String, webview: &mut Webview<'_>, path: Option<PathBuf>, named_dir: bool, tmp: bool, downloads: bool, addons: bool) -> Result<(), String> {
	let save_destination_path = path.is_some();
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {
		
		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		let transaction = transactions.new(webview_mut.clone());
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
					let channel = transaction.channel();
					let mut gma = crate::GAME_ADDONS.read().unwrap().previewing.as_ref().unwrap().to_owned();
					gma.open().and_then(|_| {
						gma.extract(dest, Some(Box::new(move |progress| channel.progress(progress))))
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

pub(crate) fn analyze_addon_sizes(resolve: String, reject: String, webview: &mut Webview<'_>) -> Result<(), String> {
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		/*
		cache_addon_paths();
		
		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		let transaction = transactions.new(webview_mut);
		let id = transaction.id;

		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let total_paths = gma_cache.paths.len();

		if total_paths == 0 {
			Err(anyhow!("No addons found on filesystem."))
		} else {
			let thread_count = (num_cpus::get() * 2).min(total_paths);
			let chunk_len = ((total_paths as f32) / (thread_count as f32).floor()) as usize;
			let paths = gma_cache.paths.values().cloned().collect::<Vec<PathBuf>>().chunks(chunk_len).map(|c| c.to_owned()).collect::<Vec<Vec<PathBuf>>>();

			/*
			let (tx, rx) = mpsc::channel();

			// Sorting thread
			std::thread::spawn(move || {
				let mut total_size: usize = 0;
				let mut sorted_gmas: Vec<GMAMetadata> = Vec::with_capacity(total_paths);
				let mut i = 0;
				while i < total_paths {
					i = i + 1;

					let gma: GMAMetadata = match rx.recv() {
						Ok(gma) => gma,
						Err(_) => break
					};

					total_size = total_size + gma.file_size;

					let pos = match sorted_gmas.binary_search_by_key(&gma.file_size, |gma| gma.file_size) {
						Ok(pos) => pos,
						Err(pos) => pos
					};
					sorted_gmas.insert(pos, gma);

					transaction.progress((i as f32) / (total_paths as f32));
				}
				transaction.finish((total_size, sorted_gmas));
			});

			// Discovery thread
			for chunk in paths {
				let tx = tx.clone();
				std::thread::spawn(move || {
					for path in chunk {
						match gma::read_gma(&path, false, None) {
							Ok(gma) => tx.send(GMAMetadata::from(gma)).unwrap(),
							Err(_) => continue
						};
					}
				});
			}
			*/

			Ok(id)

		}*/
		
		Ok(())

	}, resolve, reject);

	Ok(())
}
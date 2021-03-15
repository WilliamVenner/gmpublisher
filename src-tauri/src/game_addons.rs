use std::{collections::HashMap, fs::DirEntry, path::PathBuf, sync::mpsc};
use anyhow::{anyhow, Error};

use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{gma::{self, GMAFile, metadata}, workshop::{WorkshopItem}};
use super::show;

pub(crate) struct GameAddons {
	cached: bool,
	total: u32,
	ws_addon_cache: Vec<PublishedFileId>,
	paths: HashMap<PublishedFileId, PathBuf>,
}

impl GameAddons {
	pub(crate) fn init() -> GameAddons {
		GameAddons {
			cached: false,
			total: 0,
			ws_addon_cache: Vec::new(),
			paths: HashMap::new()
		}
	}
}

fn get_modified_time(entry: &DirEntry) -> Result<u64, Error> {
	Ok(entry.metadata()?.modified()?.elapsed()?.as_secs())
}

fn cache_addon_paths() {
	let app_data = crate::APP_DATA.read().unwrap();
	let dir = match app_data.gmod {
		Some(ref gmod) => gmod,
		None => return
	};

	let mut game_addons = crate::GAME_ADDONS.write().unwrap();
	if !game_addons.cached {
		game_addons.cached = true;

		let mut local_files: Vec<(u64, PublishedFileId)> = Vec::new();

		match dir.join("GarrysMod/addons").read_dir() {
			Ok(addons_gmas) => {
				'paths: for (modified, file_name, path) in addons_gmas.filter_map(|r| {
					match r {
						Ok(r) => {
							if r.path().is_file() {
								match r.file_name().to_string_lossy().strip_suffix(".gma").map(ToOwned::to_owned) {
									Some(file_name) => return Some(
										(get_modified_time(&r).unwrap_or(0), file_name, r.path())
									),
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
					game_addons.paths.insert(id, path);
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
							game_addons.paths.insert(id, entry.path());
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
		game_addons.ws_addon_cache = local_files.into_iter().map(|k| k.1).collect();
		game_addons.total = game_addons.ws_addon_cache.len() as u32;
	}
}

pub(crate) fn browse(resolve: String, reject: String, webview: &mut Webview<'_>, page: u32) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		cache_addon_paths();
		
		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let was_cached = game_addons.cached;
		
		let page_items: Vec<PublishedFileId> = game_addons.ws_addon_cache.iter().skip(((page - 1) * 50) as usize).take(50).cloned().collect();
		Ok(match crate::WORKSHOP.read().unwrap().get_items(page_items.clone()).unwrap() {
			Ok(data) => (
				game_addons.total,
				data.1,
				if !was_cached { Some(game_addons.paths.clone()) } else { None }
			),
			Err(_) => (
				game_addons.total,
				page_items.into_iter().map(|id| id.into()).collect::<Vec<WorkshopItem>>(),
				if !was_cached { Some(game_addons.paths.clone()) } else { None }
			)
		})
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_gma_paths(resolve: String, reject: String, webview: &mut Webview<'_>) -> Result<(), String> {
	tauri::execute_promise(webview, move || {
		Ok(crate::GAME_ADDONS.read().unwrap().paths.clone())
	}, resolve, reject);
	Ok(())
}

pub(crate) fn get_gma_metadata(resolve: String, reject: String, webview: &mut Webview<'_>, id: PublishedFileId) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let game_addons = crate::GAME_ADDONS.read().unwrap();

		match gma::metadata(match game_addons.paths.get(&id) {
			Some(path) => path,
			None => return Ok(None)
		}) {
			Ok(mut gma) => {
				gma.id = Some(id);
				gma
			},
			Err(err) => Err(anyhow!(format!("{:?}", err)))
		}
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn open_addon(resolve: String, reject: String, webview: &mut Webview<'_>, path: String) -> Result<(), String> {
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		let transaction = transactions.new(webview_mut);
		let id = transaction.id;

		std::thread::spawn(move || {
			let progress_transaction = transaction.clone();
			match gma::read_gma(
				&PathBuf::from(path),
				true,
				Some(Box::new(move |progress: f32| {
					progress_transaction.progress(progress);
				}))
			) {
				Ok(gma) => gma,
				Err(_) => {
					// TODO transaction errors
					panic!();
				}
			}
		});
		
		Ok(id)
		
	}, resolve, reject);

	Ok(())
}

pub(crate) fn analyze_addon_sizes(resolve: String, reject: String, webview: &mut Webview<'_>) -> Result<(), String> {
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		cache_addon_paths();
		
		let mut transactions = crate::TRANSACTIONS.write().unwrap();
		let transaction = transactions.new(webview_mut);
		let id = transaction.id;

		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let total_paths = game_addons.paths.len();

		if total_paths == 0 {
			Err(anyhow!("No addons found on filesystem."))
		} else {
			let thread_count = (num_cpus::get() * 2).min(total_paths);
			let chunk_len = ((total_paths as f32) / (thread_count as f32).floor()) as usize;
			let paths = game_addons.paths.values().cloned().collect::<Vec<PathBuf>>().chunks(chunk_len).map(|c| c.to_owned()).collect::<Vec<Vec<PathBuf>>>();

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

			Ok(id)
		}
		
	}, resolve, reject);

	Ok(())
}
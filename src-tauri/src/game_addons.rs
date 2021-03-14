use std::{collections::HashMap, fs::{DirEntry, File}, io::BufReader, path::PathBuf, sync::{Arc, Mutex}};
use anyhow::{anyhow, Error};

use steamworks::PublishedFileId;
use tauri::Webview;
use serde::Serialize;

use crate::{lib::gma::{self, GMAFile}, transactions::{Transaction, Transactions}, workshop::{Workshop, WorkshopItem}};
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

pub(crate) fn browse(resolve: String, reject: String, webview: &mut Webview<'_>, dir: PathBuf, page: u32) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let mut game_addons = crate::GAME_ADDONS.write().unwrap();

		let was_cached = game_addons.cached;
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

#[derive(Serialize)]
struct GMAMetadata {
    name: String,
    description: String,
    author: String
}
impl GMAMetadata {
	fn get(id: PublishedFileId, path: &PathBuf) -> Result<Option<Self>, Error> {
		let handle = match File::open(path) {
			Ok(handle) => handle,
			Err(_) => return Ok(None)
		};

		Ok(Some(GMAMetadata::from(match gma::read_gma(BufReader::new(handle), false) {
			Ok(gma) => gma,
			Err(_) => return Err(anyhow!(match path.file_name() {
				Some(file_name) => file_name.to_string_lossy().to_string(),
				None => id.0.to_string()
			}))
		})))
	}
}
impl From<GMAFile> for GMAMetadata {
    fn from(gma: GMAFile) -> Self {
		GMAMetadata {
			name: gma.name,
			description: gma.description,
			author: gma.author
		}
    }
}

pub(crate) fn get_gma_metadata(resolve: String, reject: String, webview: &mut Webview<'_>, id: PublishedFileId) -> Result<(), String> {
	tauri::execute_promise(webview, move || {

		let game_addons = crate::GAME_ADDONS.read().unwrap();

		let path = match game_addons.paths.get(&id) {
			Some(path) => path,
			None => return Ok(None)
		};

		GMAMetadata::get(id, path)
		
	}, resolve, reject);
	Ok(())
}

pub(crate) fn open_addon(resolve: String, reject: String, webview: &mut Webview<'_>, path: String) -> Result<(), String> {
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {
		
		//let transaction = transactions.new(webview_mut);

		let path = PathBuf::from(path);
		
		//transaction.progress(50.);

		Ok(())
		
	}, resolve, reject);

	Ok(())
}
use std::path::PathBuf;

use tauri::{InvokeMessage, Params};

pub fn invoke_handler<M>() -> impl Fn(InvokeMessage<M>) + Send + Sync + 'static
where
	M: Params,
{
	tauri::generate_handler![
		free_caches,
		check_dir,
		check_file,
		open,
		open_file_location,
		file_size,
		crate::transactions::websocket,
		crate::transactions::cancel_transaction,
		crate::appdata::update_settings,
		crate::appdata::validate_gmod,
		crate::appdata::window_resized,
		crate::game_addons::browse_installed_addons,
		crate::game_addons::get_installed_addon,
		crate::game_addons::downloader_extract_gmas,
		crate::steam::is_steam_connected,
		crate::steam::get_current_user,
		crate::steam::users::get_steam_user,
		crate::steam::workshop::fetch_workshop_items,
		crate::steam::workshop::fetch_workshop_item,
		crate::steam::workshop::browse_my_workshop,
		crate::steam::downloads::workshop_download,
		crate::steam::publishing::verify_whitelist,
		crate::addon_size_analyzer::addon_size_analyzer,
		crate::content_generator::get_content_generator_manifests,
		crate::content_generator::update_content_generator_manifest,
		crate::gma::preview::preview_gma,
		crate::gma::preview::extract_preview_entry,
		crate::gma::preview::extract_preview_gma,
	]
}

#[tauri::command]
pub fn free_caches() {
	crate::game_addons::free_caches();
	crate::steam::workshop::free_caches();
}

#[tauri::command]
pub fn check_file(path: PathBuf, extension: Option<String>) -> bool {
	path.is_absolute()
		&& path.is_file()
		&& match extension {
			Some(extension) => {
				if let Some(picked_extension) = path.extension() {
					extension.eq_ignore_ascii_case(&picked_extension.to_string_lossy())
				} else {
					false
				}
			}
			None => true,
		}
}

#[tauri::command]
pub fn check_dir(path: PathBuf) -> bool {
	path.is_absolute() && path.is_dir()
}

#[tauri::command]
fn open(path: PathBuf) {
	crate::path::open(path);
}

#[tauri::command]
fn open_file_location(path: PathBuf) {
	crate::path::open_file_location(path);
}

#[tauri::command]
fn file_size(path: PathBuf) -> Option<u64> {
	path.metadata().ok().map(|metadata| metadata.len())
}

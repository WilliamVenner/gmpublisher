use std::path::PathBuf;

pub fn invoke_handler() -> impl Fn(tauri::Invoke<crate::webview::Params>) + Send + Sync + 'static {
	tauri::generate_handler![
		check_dir,
		check_file,
		open,
		open_file_location,
		file_size,
		crate::webview::reloaded,
		crate::webview::js_error,
		crate::webview::error,
		crate::webview::info,
		crate::webview::warn,
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
		crate::steam::workshop::workshop_item_channel,
		crate::steam::downloads::workshop_download,
		crate::steam::publishing::verify_whitelist,
		crate::steam::publishing::publish,
		crate::steam::publishing::verify_icon,
		crate::steam::publishing::publish_icon,
		crate::steam::subscriptions::browse_subscribed_addons,
		crate::addon_size_analyzer::addon_size_analyzer,
		crate::content_generator::get_content_generator_manifests,
		crate::content_generator::update_content_generator_manifest,
		crate::gma::preview::preview_gma,
		crate::gma::preview::extract_preview_entry,
		crate::gma::preview::extract_preview_gma,
		crate::gma::extract::extract_gma,
		crate::search::search,
		crate::search::search_channel,
		crate::search::full_search,
	]
}

pub fn free_caches() {
	crate::game_addons::free_caches();
	crate::steam::workshop::free_caches();
	search!().clear();
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
pub fn open(path: PathBuf) {
	crate::path::open(path);
}

#[tauri::command]
pub fn open_file_location(path: PathBuf) {
	crate::path::open_file_location(path);
}

#[tauri::command]
pub fn file_size(path: PathBuf) -> Option<u64> {
	path.metadata().ok().map(|metadata| metadata.len())
}

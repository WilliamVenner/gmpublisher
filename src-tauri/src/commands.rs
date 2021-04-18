use tauri::{InvokeMessage, Params};

pub fn invoke_handler<M>() -> impl Fn(InvokeMessage<M>) + Send + Sync + 'static
where
	M: Params
{
	tauri::generate_handler![
		free_caches,

		crate::transactions::cancel_transaction,

		crate::appdata::update_settings,
		crate::appdata::clean_app_data,
		crate::appdata::verify_directory,

		crate::game_addons::browse_installed_addons,
		crate::game_addons::get_installed_addon,

		crate::steam::is_steam_connected,
		crate::steam::get_steam_user,
		crate::steam::workshop::browse_my_workshop,

		crate::addon_size_analyzer::addon_size_analyzer,

		crate::content_generator::get_content_generator_manifests,
		crate::content_generator::update_content_generator_manifest,
	]
}

#[tauri::command]
pub fn free_caches() {
	crate::game_addons::free_caches();
	crate::steam::workshop::free_caches();
}

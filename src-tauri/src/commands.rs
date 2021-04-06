use tauri::{InvokeMessage, Params, api::assets::Assets, runtime::{Runtime, Tag}};

pub fn invoke_handler<M>() -> impl Fn(InvokeMessage<M>) + Send + Sync + 'static
where
	M: Params
{
	tauri::generate_handler![
		crate::transactions::cancel_transaction,

		crate::appdata::update_settings,
		crate::appdata::clean_app_data,

		crate::game_addons::browse_game_addons,

		crate::steam::is_steam_connected,
		crate::steam::get_steam_user,

		crate::addon_size_analyzer::free_addon_size_analyzer,
	]
}

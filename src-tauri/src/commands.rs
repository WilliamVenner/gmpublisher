use tauri::{InvokeMessage, WebviewManager, ApplicationExt};

pub fn invoke_handler<A>() -> impl Fn(WebviewManager<A>, InvokeMessage<A>) + Send + 'static
where
	A: ApplicationExt + 'static
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

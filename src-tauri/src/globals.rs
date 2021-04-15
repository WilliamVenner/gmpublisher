use tauri::runtime::Args;

pub const GMOD_APP_ID: steamworks::AppId = steamworks::AppId(4000);

lazy_static! {
	pub static ref STEAMWORKS: crate::steam::Steam = crate::steam::Steam::init();
	pub static ref GMA_CACHE: crate::gma::cache::GMACache = crate::gma::cache::GMACache::init();
	pub static ref GAME_ADDONS: crate::game_addons::GameAddons = crate::game_addons::GameAddons::init();
	pub static ref ADDON_SIZE_ANALYZER: crate::addon_size_analyzer::AddonSizeAnalyzer = crate::addon_size_analyzer::AddonSizeAnalyzer::init();
	pub static ref APP_DATA: crate::appdata::AppData = crate::appdata::AppData::init();
	pub static ref WEBVIEW: crate::webview::WrappedWebview<Args<String, String, tauri::api::assets::EmbeddedAssets, tauri::runtime::flavors::wry::Wry>> = crate::webview::WrappedWebview::pending();
}

#[macro_export]
macro_rules! steam {
	() => {
		&crate::STEAMWORKS
	};
}
#[macro_export]
macro_rules! downloads {
	() => {
		&crate::steam::DOWNLOADS
	};
}

#[macro_export]
macro_rules! gma {
	() => {
		&crate::GMA
	};
}
#[macro_export]
macro_rules! gma_cache {
	() => {
		&crate::GMA_CACHE
	};
}

#[macro_export]
macro_rules! game_addons {
	() => {
		&crate::GAME_ADDONS
	};
}

#[macro_export]
macro_rules! app_data {
	() => {
		&crate::APP_DATA
	};
}

#[macro_export]
macro_rules! webview {
	() => {
		&crate::WEBVIEW
	};
}
#[macro_export]
macro_rules! webview_emit {
	( $event:expr, $data:expr ) => {
		crate::webview!().emit($event, Some($data)).unwrap()
	};

	( $event:expr ) => {
		crate::webview!().emit($event, turbonone!()).unwrap()
	};
}

pub(super) fn init_globals() {
	lazy_static::initialize(&STEAMWORKS);
}

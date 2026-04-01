pub const GMOD_APP_ID: steamworks::AppId = steamworks::AppId(4000);

lazy_static! {
	pub static ref STEAMWORKS: crate::steam::Steam = crate::steam::Steam::init();
	pub static ref GAME_ADDONS: crate::game_addons::GameAddons = crate::game_addons::GameAddons::init();
	pub static ref ADDON_SIZE_ANALYZER: crate::addon_size_analyzer::AddonSizeAnalyzer = crate::addon_size_analyzer::AddonSizeAnalyzer::init();
	pub static ref APP_DATA: crate::appdata::AppData = crate::appdata::AppData::init();
	pub static ref WEBVIEW: crate::webview::WrappedWebview = crate::webview::WrappedWebview::pending();
	pub static ref SEARCH: crate::search::Search = crate::search::Search::init();
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
macro_rules! search {
	() => {
		&crate::SEARCH
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
		crate::webview!().emit($event, Some($data))
	};

	( $event:expr ) => {
		crate::webview!().emit($event, turbonone!())
	};
}

pub(super) fn init_globals() {
	println!("Initializing Steamworks...");
	lazy_static::initialize(&STEAMWORKS);

	println!("Initializing AppData...");
	lazy_static::initialize(&APP_DATA);

	println!("Initializing Transactions...");
	crate::transactions::init();

	rayon::spawn(|| {
		println!("Initializing Game Addons...");
		lazy_static::initialize(&GAME_ADDONS);
		GAME_ADDONS.discover_addons();
	});

	rayon::spawn(|| {
		&*crate::gma::whitelist::ADDON_WHITELIST;
	});
}

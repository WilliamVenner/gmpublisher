use std::sync::{Mutex, Arc};

use serde::Deserialize;
use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{appdata::AppData, game_addons::GameAddons, show, transactions::Transactions, workshop::Workshop};
use crate::workshop;
use crate::settings;
use crate::game_addons;

#[derive(Deserialize)]
#[serde(tag="cmd", rename_all="camelCase")]
enum Command {
	CancelTransaction {
		id: usize
	},

	UpdateSettings {
		settings: String
	},
	WorkshopBrowser {
		page: u32,
		callback: String,
		error: String
	},
	GameAddonsBrowser {
		page: u32,
		callback: String,
		error: String
	},
	GmaMetadata {
		id: PublishedFileId,
		callback: String,
		error: String
	},
	GetGmaPaths {
		callback: String,
		error: String
	},
	OpenAddon {
		path: String,
		callback: String,
		error: String
	},

	LoadAsset
}

pub(crate) fn invoke_handler<'a>() -> impl FnMut(&mut Webview<'_>, &str) -> Result<(), String> + 'static {
	move |webview: &mut Webview, arg: &str| {
		#[cfg(debug_assertions)]
		println!("{}", arg);
		
		match serde_json::from_str(arg) {
			Err(error) => {
				show::error(format!("Invoke handler ERROR:\n{:#?}", error));
				Err(error.to_string())
			},
			Ok(cmd) => {
				use Command::*;
				match match cmd {
					CancelTransaction { id } => {
						// TODO
						Ok(())
					},

					GameAddonsBrowser { page, callback, error } => {
						match &crate::APP_DATA.read().unwrap().gmod {
							Some(gmod) => game_addons::browse(callback, error, webview, gmod.to_owned(), page),
							None => { Ok(()) }
						}
					},

					WorkshopBrowser { page, callback, error } => {
						workshop::browse(callback, error, webview, page)
					},

					UpdateSettings { settings } => {
						settings::invoke_handler(webview, settings)
					},

					GmaMetadata { id, callback, error } => {
						game_addons::get_gma_metadata(callback, error, webview, id)
					},

					GetGmaPaths { callback, error } => {
						game_addons::get_gma_paths(callback, error, webview)
					},

					OpenAddon { callback, error, path } => {
						game_addons::open_addon(callback, error, webview, path)
					},

					LoadAsset => { Ok(()) },

					#[allow(unreachable_patterns)]
					_ => Err("Unknown command".to_string())
				} {
					Ok(_) => Ok(()),
					Err(error) => { show::error(format!("{:#?}", error)); Ok(()) }
				}
			}
		}
	}
}
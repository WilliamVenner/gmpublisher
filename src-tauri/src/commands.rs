use std::sync::{Mutex, Arc};

use serde::Deserialize;
use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{appdata::AppData, game_addons::GameAddons, show, workshop::Workshop};
use crate::workshop;
use crate::settings;
use crate::game_addons;

#[derive(Deserialize)]
#[serde(tag="cmd", rename_all="camelCase")]
enum Command {
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

	LoadAsset
}

pub(crate) fn invoke_handler<'a>(app_data: AppData, workshop: Workshop, game_addons: GameAddons) -> impl FnMut(&mut Webview<'_>, &str) -> Result<(), String> + 'static {
	let app_data_ptr = Arc::new(Mutex::new(app_data));
	let workshop_ptr = Arc::new(workshop);
	let game_addons_ptr = Arc::new(Mutex::new(game_addons));

	Box::new(move |webview: &mut Webview, arg: &str| {
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
					GameAddonsBrowser { page, callback, error } => {
						let app_data = app_data_ptr.lock().unwrap();
						match &app_data.gmod {
							Some(gmod) => game_addons::browse(callback, error, webview, game_addons_ptr.clone(), workshop_ptr.clone(), gmod.clone(), page),
							None => { Ok(()) }
						}
					},

					WorkshopBrowser { page, callback, error } => {
						workshop::browse(callback, error, webview, workshop_ptr.clone(), page)
					},

					UpdateSettings { settings } => {
						settings::invoke_handler(webview, app_data_ptr.clone(), settings)
					},

					GmaMetadata { id, callback, error } => {
						game_addons::get_gma_metadata(callback, error, webview, game_addons_ptr.clone(), id)
					},

					GetGmaPaths { callback, error } => {
						game_addons::get_gma_paths(callback, error, webview, game_addons_ptr.clone())
					},

					LoadAsset => { Ok(()) },
					_ => Err("Unknown command".to_string())
				} {
					Ok(_) => Ok(()),
					Err(error) => { show::error(format!("{:#?}", error)); Ok(()) }
				}
			}
		}
	})
}
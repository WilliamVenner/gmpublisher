use std::path::PathBuf;
use serde::Deserialize;
use steamworks::PublishedFileId;
use tauri::Webview;

use crate::show;
use crate::workshop;
use crate::settings;
use crate::game_addons;
use crate::util;

#[derive(Deserialize)]
#[serde(tag="cmd", rename_all="camelCase")]
enum Command {
	CancelTransaction {
		id: usize
	},
	
	OpenFile { // TODO rename to OpenURL
		path: String
	},
	OpenFolder {
		path: PathBuf
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

	PreviewGma {
		path: PathBuf,
		id: Option<PublishedFileId>,
		callback: String,
		error: String
	},
	OpenGmaPreviewEntry {
		entry_path: String,
		callback: String,
		error: String
	},
	ExtractGma {
		path: Option<PathBuf>,
		named_dir: bool,
		tmp: bool,
		downloads: bool,
		addons: bool,
		callback: String,
		error: String
	},

	PromptPathDialog {
		multiple: bool,
		directory: bool,
		save: bool,
		default_path: Option<String>,
		filter: Option<String>,
		callback: String,
		error: String
	},
	
	AnalyzeAddonSizes {
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
						crate::TRANSACTIONS.write().unwrap().cancel(id);
						Ok(())
					},

					OpenFile { path } => {
						show::open(&path);
						Ok(())
					},

					OpenFolder { path } => {
						match show::open_file_location(path.as_os_str().to_string_lossy().to_string()) {
							Ok(_) => Ok(()),
							Err(error) => Err(format!("{:?}", error))
						}
					},

					PromptPathDialog { callback, error, multiple, directory, save, default_path, filter } => {
						util::prompt_path_dialog(callback, error, webview, multiple, directory, save, default_path, filter)
					},

					WorkshopBrowser { page, callback, error } => {
						workshop::browse(callback, error, webview, page)
					},

					UpdateSettings { settings } => {
						settings::invoke_handler(webview, settings)
					},

					GameAddonsBrowser { page, callback, error } => {
						if crate::APP_DATA.read().unwrap().gmod.is_some() {
							game_addons::browse(callback, error, webview, page)
						} else {
							Err("Garry's Mod not found".to_string()) // TODO
						}
					},
					GmaMetadata { id, callback, error } => {
						game_addons::get_addon_metadata(callback, error, webview, id)
					},
					GetGmaPaths { callback, error } => {
						game_addons::get_gma_paths(callback, error, webview)
					},
					
					PreviewGma { callback, error, path, id } => {
						game_addons::preview_gma(callback, error, webview, path, id)
					},
					OpenGmaPreviewEntry { callback, error, entry_path } => {
						game_addons::open_gma_preview_entry(callback, error, webview, entry_path)
					},
					ExtractGma { callback, error, path, named_dir, tmp, downloads, addons } => {
						game_addons::extract_gma_preview(callback, error, webview, path, named_dir, tmp, downloads, addons)
					},

					AnalyzeAddonSizes { callback, error } => {
						game_addons::analyze_addon_sizes(callback, error, webview)
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
use crate::{GMOD_APP_ID, Transaction, gma::{GMAFile, GMAEntry, GMAMetadata, GMAFilePointers}};
use image::{ImageError, ImageFormat};
use parking_lot::Mutex;
use path_slash::PathBufExt;
use walkdir::WalkDir;
use std::{fs::File, io::BufReader, mem::MaybeUninit, path::PathBuf, sync::Arc, time::SystemTime};
use steamworks::{PublishedFileId, SteamError};

#[cfg(not(target_os = "windows"))]
use std::collections::HashSet;

pub enum PublishError {
	NotWhitelisted(Vec<String>),
	NoEntries,
	DuplicateEntries,
	InvalidContentPath,
	MultipleGMAs,
	IconTooLarge,
	IconTooSmall,
	IconInvalidFormat,
	IOError,
	SteamError(SteamError),
	ImageError(ImageError),
}
impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
			PublishError::NotWhitelisted(_) => write!(f, "ERR_WHITELIST"),
			PublishError::NoEntries => write!(f, "ERR_NO_ENTRIES"),
			PublishError::DuplicateEntries => write!(f, "ERR_DUPLICATE_ENTRIES"),
            PublishError::InvalidContentPath => write!(f, "ERR_INVALID_CONTENT_PATH"),
            PublishError::MultipleGMAs => write!(f, "ERR_MULTIPLE_GMAS"),
            PublishError::IconTooLarge => write!(f, "ERR_ICON_TOO_LARGE"),
            PublishError::IconTooSmall => write!(f, "ERR_ICON_TOO_SMALL"),
            PublishError::IconInvalidFormat => write!(f, "ERR_ICON_INVALID_FORMAT"),
            PublishError::IOError => write!(f, "ERR_IO_ERROR"),
            PublishError::SteamError(_) => write!(f, "ERR_STEAM_ERROR"),
            PublishError::ImageError(_) => write!(f, "ERR_IMAGE_ERROR"),
        }
    }
}
impl From<SteamError> for PublishError {
	fn from(error: SteamError) -> PublishError {
		PublishError::SteamError(error)
	}
}
impl From<ImageError> for PublishError {
	fn from(error: ImageError) -> PublishError {
		PublishError::ImageError(error)
	}
}
impl From<std::io::Error> for PublishError {
	fn from(_: std::io::Error) -> PublishError {
		PublishError::IOError
	}
}

use super::Steam;
pub struct ContentPath(PathBuf);
impl std::ops::Deref for ContentPath {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl Into<PathBuf> for ContentPath {
	fn into(self) -> PathBuf {
		self.0
	}
}
impl ContentPath {
	pub fn new(path: PathBuf) -> Result<ContentPath, PublishError> {
		if !path.is_dir() {
			return Err(PublishError::InvalidContentPath);
		}

		let mut gma_path: MaybeUninit<PathBuf> = MaybeUninit::uninit();
		for (i, path) in path
			.read_dir()?
			.filter_map(|entry| {
				entry.ok().and_then(|entry| {
					let path = entry.path();
					let extension = path.extension()?;
					if extension == "gma" {
						Some(path)
					} else {
						None
					}
				})
			})
			.enumerate()
		{
			if i > 0 {
				return Err(PublishError::MultipleGMAs);
			}
			unsafe {
				gma_path.as_mut_ptr().write(path);
			}
		}

		Ok(ContentPath(unsafe { gma_path.assume_init() }))
	}
}

const WORKSHOP_ICON_MAX_SIZE: u64 = 1000000;
const WORKSHOP_ICON_MIN_SIZE: u64 = 16;
pub enum WorkshopIcon {
	Path(PathBuf),
	Default
}
impl Into<PathBuf> for WorkshopIcon {
	fn into(self) -> PathBuf {
		match self {
			WorkshopIcon::Path(path) => path,
			WorkshopIcon::Default => {
				let mut path = app_data!().temp_dir().to_owned();
				path.push("gmpublisher_default_icon.png");
				if !path.is_file() {
					std::fs::write(&path, include_bytes!("../../../public/img/gmpublisher_default_icon.png")).expect("Failed to write default icon to temp directory!");
				}
				path
			},
		}
	}
}
impl WorkshopIcon {
	fn try_format(tried_best_guess: bool, file_type: ImageFormat, path: &PathBuf, mut file_types: Vec<ImageFormat>) -> Result<(), PublishError> {
		if let Err(error) = image::load(BufReader::new(File::open(path)?), file_type) {
			if let ImageError::Decoding(decoding_error) = error {
				if !tried_best_guess {
					if let image::error::ImageFormatHint::Exact(best_guess) = decoding_error.format_hint() {
						let mut i = 0;
						while i != file_types.len() {
							if file_types[i] == best_guess {
								return WorkshopIcon::try_format(true, file_types.remove(i), path, file_types);
							} else {
								i += 1;
							}
						}
					}
				}

				if file_types.is_empty() {
					Err(PublishError::IconInvalidFormat)
				} else {
					WorkshopIcon::try_format(tried_best_guess, file_types.remove(0), path, file_types)
				}
			} else {
				Err(PublishError::ImageError(error))
			}
		} else {
			Ok(())
		}
	}

	pub fn new(path: PathBuf) -> Result<WorkshopIcon, PublishError> {
		// FIXME remove the guessing, it probably won't work with Steam

		let len = path.metadata()?.len();
		if len > WORKSHOP_ICON_MAX_SIZE {
			return Err(PublishError::IconTooLarge);
		} else if len < WORKSHOP_ICON_MIN_SIZE {
			return Err(PublishError::IconTooSmall);
		}

		let file_extension = path.extension().and_then(|x| x.to_str()).unwrap_or("jpg").to_ascii_lowercase();
		let mut file_types = match file_extension.as_str() {
			"png" => vec![ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif],
			"gif" => vec![ImageFormat::Gif, ImageFormat::Jpeg, ImageFormat::Png],
			_ => vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Gif],
		};

		WorkshopIcon::try_format(false, file_types.remove(0), &path, file_types)?;

		Ok(WorkshopIcon::Path(path))
	}
}

pub struct WorkshopCreationDetails {
	pub id: PublishedFileId,
	pub title: String,
	pub path: ContentPath,
	pub preview: WorkshopIcon,
}
pub struct WorkshopUpdateDetails {
	pub id: PublishedFileId,
	pub path: PathBuf,
	pub preview: Option<PathBuf>,
	pub changes: Option<String>,
}
pub enum WorkshopUpdateType {
	Creation(WorkshopCreationDetails),
	Update(WorkshopUpdateDetails),
}

impl Steam {
	pub fn update(&self, details: WorkshopUpdateType, transaction: Transaction) -> Result<(PublishedFileId, bool), PublishError> {
		use WorkshopUpdateType::*;

		let result = Arc::new(Mutex::new(None));
		let result_ref = result.clone();
		let update_handle = match details {
			Creation(details) => {
				self.client()
					.ugc()
					.start_item_update(GMOD_APP_ID, details.id)
					.content_path(&details.path)
					.title(&details.title)
					.preview_path(&Into::<PathBuf>::into(details.preview))
					.submit(None, move |result| {
						*result_ref.lock() = Some(result);
					})
			}

			Update(details) => {
				let path = ContentPath::new(details.path)?;
				let preview = match details.preview {
					Some(preview) => Some(WorkshopIcon::new(preview)?),
					None => None,
				};

				let update = self.client().ugc().start_item_update(GMOD_APP_ID, details.id);
				match preview {
					Some(preview) => update.preview_path(&Into::<PathBuf>::into(preview)),
					None => update,
				}
				.content_path(&path)
				.submit(details.changes.as_deref(), move |result| {
					*result_ref.lock() = Some(result);
				})
			}
		};

		let (_, _, total) = update_handle.progress();
		transaction.data(total);

		let result = loop {
			let (processed, progress, total) = update_handle.progress();
			if !matches!(processed, steamworks::UpdateStatus::Invalid) {
				transaction.status(match processed {
					steamworks::UpdateStatus::Invalid => unreachable!(),
					steamworks::UpdateStatus::PreparingConfig => "PUBLISH_PREPARING_CONFIG",
					steamworks::UpdateStatus::PreparingContent => "PUBLISH_PREPARING_CONTENT",
					steamworks::UpdateStatus::UploadingContent => "PUBLISH_UPLOADING_CONTENT",
					steamworks::UpdateStatus::UploadingPreviewFile => "PUBLISH_UPLOADING_PREVIEW_FILE",
					steamworks::UpdateStatus::CommittingChanges => "PUBLISH_COMMITTING_CHANGES",
				});
			}
			transaction.progress(progress as f64 / total as f64);

			if !result.is_locked() && result.lock().is_some() {
				break Arc::try_unwrap(result).unwrap().into_inner().unwrap().map_err(|error| PublishError::SteamError(error));
			} else {
				self.run_callbacks();
			}
		};

		if result.is_ok() { transaction.progress(1.); }
		result
	}

	pub fn publish(&self, path: ContentPath, preview: WorkshopIcon, title: String, transaction: Transaction) -> Result<(PublishedFileId, bool), PublishError> {
		let published = Arc::new(Mutex::new(None));
		let published_ref = published.clone();
		self.client()
			.ugc()
			.create_item(GMOD_APP_ID, steamworks::FileType::Community, move |result| {
				match result {
					Ok((id, _accepted_legal_agreement)) => {
						// TODO test accepted_legal_agreement
						*published_ref.lock() = Some(Ok(id));
					}
					Err(err) => {
						*published_ref.lock() = Some(Err(err));
					}
				}
			});

		loop {
			if let Some(published_ref) = published.try_lock() {
				if published_ref.is_some() {
					break;
				}
			}
			self.run_callbacks();
		}

		let id = Arc::try_unwrap(published)
			.unwrap()
			.into_inner()
			.unwrap()?;

		self.update(WorkshopUpdateType::Creation(WorkshopCreationDetails { id, title, preview, path }), transaction)
	}
}

#[tauri::command]
fn verify_whitelist(path: PathBuf, ignore: Vec<String>) -> Result<(Vec<GMAEntry>, u64), (String, Option<Vec<String>>)> {
	if !path.is_dir() || !path.is_absolute() { return Err((PublishError::InvalidContentPath.to_string(), None)); }

	let root_path_strip_len = path.to_slash_lossy().len() + 1;

	let mut size = 0;
	let mut failed_extra = false;
	let mut failed = Vec::with_capacity(10);
	let mut files = Vec::new();
	#[cfg(not(target_os = "windows"))]
	let mut dedup = HashSet::new();

	for (path, relative_path) in WalkDir::new(&path).contents_first(true).into_iter().filter_map(|entry| {

		let path = match entry {
			Ok(entry) => entry.into_path(),
			Err(err) => match err.path() {
				Some(path) => path.to_path_buf(),
				None => return None,
			}
		};

		if path.is_dir() { return None; }

		let relative_path = {
			let mut relative_path = path.to_slash_lossy();
			if relative_path.len() < root_path_strip_len { return None; }
			relative_path.split_off(root_path_strip_len).to_lowercase()
		};

		Some((path, relative_path))

	}).filter(|(_, relative_path)| crate::gma::whitelist::filter_default_ignored(relative_path)).filter(|(_, relative_path)| !crate::gma::whitelist::is_ignored(relative_path, &ignore)) {
		if !crate::gma::whitelist::check(&relative_path) {
			if failed.len() == 9 {
				failed_extra = true;
				break;
			} else {
				failed.push(relative_path);
			}
		} else if failed.is_empty() {
			let entry_size = path.metadata().map(|metadata| metadata.len()).unwrap_or(0);
			size += entry_size;
			files.push(GMAEntry {
				path: relative_path,
				size: entry_size,
				crc: 0,
				index: 0
			});
		}
	}

	if failed.is_empty() {
		if files.is_empty() {
			Err((PublishError::NoEntries.to_string(), None))
		} else {
			Ok((files, size))
		}
	} else {
		failed.sort_unstable();

		if failed_extra {
			failed.push("...".to_string());
		}

		Err((PublishError::NotWhitelisted(vec![]).to_string(), Some(failed)))
	}
}

#[tauri::command]
pub fn publish(content_path: PathBuf, icon_path: Option<PathBuf>, title: String, tags: Vec<String>, addon_type: String) -> u32 {
	let transaction = transaction!();
	let id = transaction.id;

	let icon_path = match icon_path {
		Some(icon_path) => match WorkshopIcon::new(icon_path) {
			Ok(icon) => icon,
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				return id;
			}
		},
		None => WorkshopIcon::Default
	};

	rayon::spawn(move || {
		transaction.status("PUBLISH_PACKING");

		let mut path = app_data!().temp_dir().to_owned();
		path.pop();
		path.push("gmpublisher_publishing");

		if let Err(_) = std::fs::create_dir_all(&path) {
			transaction.error("ERR_IO_ERROR", turbonone!());
			return;
		}

		path.push("gmpublisher.gma");

		let gma = GMAFile { // TODO convert to GMAFile::new()
		    path: path.clone(),
		    size: 0,
		    id: None,
		    metadata: Some(GMAMetadata::Standard {
		        title: title.clone(),
		        addon_type,
		        tags,
		        ignore: app_data!().settings.read().ignore_globs.to_owned(),
			}),
		    entries: None,
		    pointers: GMAFilePointers::default(),
		    version: 3,
		    extracted_name: String::new(),
		    modified: None,
		    membuffer: None,
		};

		if let Err(error) = gma.create(content_path, transaction.clone()) {
			if !transaction.aborted() {
				transaction.error(error.to_string(), turbonone!());
			}
			return;
		}

		path.pop();
		let content_path = match ContentPath::new(path) {
			Ok(content_path) => content_path,
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				return;
			}
		};

		transaction.status("PUBLISH_STARTING");

		let (id, accepted_legal_agreement) = match steam!().publish(content_path, icon_path, title, transaction.clone()) {
			Ok(data) => data,
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				return;
			}
		};

		transaction.finished((id, accepted_legal_agreement));

		// TODO remove packed addon when done
	});
	// https://partner.steamgames.com/doc/api/ISteamUGC#GetItemUpdateProgress

	id
}

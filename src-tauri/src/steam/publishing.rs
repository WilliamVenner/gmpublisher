use crate::{
	gma::{GMAEntry, GMAFile, GMAFilePointers, GMAMetadata},
	Transaction, GMOD_APP_ID,
};
use image::{DynamicImage, GenericImageView, ImageError, ImageFormat};
use parking_lot::Mutex;
use path_slash::PathBufExt;
use std::{
	fs::File,
	io::BufReader,
	mem::MaybeUninit,
	path::{Path, PathBuf},
	sync::Arc,
};
use steamworks::{PublishedFileId, SteamError};
use walkdir::WalkDir;

#[cfg(not(target_os = "windows"))]
use std::collections::HashSet;

pub enum PublishError {
	NotWhitelisted(Vec<String>),
	NoEntries,
	DuplicateEntry(String),
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
			PublishError::NotWhitelisted(whitelisted) => write!(f, "ERR_WHITELIST:{}", whitelisted.join("\n")),
			PublishError::NoEntries => write!(f, "ERR_NO_ENTRIES"),
			PublishError::DuplicateEntry(path) => write!(f, "ERR_DUPLICATE_ENTRIES:{}", path),
			PublishError::InvalidContentPath => write!(f, "ERR_INVALID_CONTENT_PATH"),
			PublishError::MultipleGMAs => write!(f, "ERR_MULTIPLE_GMAS"),
			PublishError::IconTooLarge => write!(f, "ERR_ICON_TOO_LARGE"),
			PublishError::IconTooSmall => write!(f, "ERR_ICON_TOO_SMALL"),
			PublishError::IconInvalidFormat => write!(f, "ERR_ICON_INVALID_FORMAT"),
			PublishError::IOError => write!(f, "ERR_IO_ERROR"),
			PublishError::SteamError(error) => write!(f, "ERR_STEAM_ERROR:{}", error.to_string()),
			PublishError::ImageError(error) => write!(f, "ERR_IMAGE_ERROR:{}", error.to_string()),
		}
	}
}
impl serde::Serialize for PublishError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.to_string())
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

const WORKSHOP_ICON_MAX_SIZE: u64 = 1048576;
const WORKSHOP_ICON_MIN_SIZE: u64 = 16;
const WORKSHOP_DEFAULT_ICON: &'static [u8] = include_bytes!("../../../public/img/gmpublisher_default_icon.png");

pub enum WorkshopIcon {
	Custom {
		image: DynamicImage,
		path: PathBuf,
		format: ImageFormat,
		width: u32,
		height: u32,
		upscale: bool,
	},
	Default,
}
impl WorkshopIcon {
	pub fn can_upscale(width: u32, height: u32, format: ImageFormat) -> bool {
		!matches!(format, ImageFormat::Gif) && ((width < 512 || height < 512) || (width != height))
	}
}
impl Into<PathBuf> for WorkshopIcon {
	fn into(self) -> PathBuf {
		match self {
			WorkshopIcon::Custom {
				path,
				image,
				width,
				height,
				upscale,
				format,
			} => {
				if upscale && WorkshopIcon::can_upscale(width, height, format) {
					let format_extension = match format {
						ImageFormat::Png => "png",
						ImageFormat::Jpeg => "jpg",
						_ => unreachable!(),
					};

					let mut temp_img = app_data!().temp_dir().to_owned();
					temp_img.push(format!("gmpublisher_upscaled_icon.{}", format_extension));

					let image = image.resize_exact(512, 512, image::imageops::FilterType::CatmullRom);
					match image.save_with_format(&temp_img, format) {
						Ok(_) => temp_img,
						Err(_) => path,
					}
				} else {
					path
				}
			}
			WorkshopIcon::Default => {
				let mut path = app_data!().temp_dir().to_owned();
				path.push("gmpublisher_default_icon.png");
				if !path.is_file() && path.metadata().map(|metadata| metadata.len()).unwrap_or(0) != WORKSHOP_DEFAULT_ICON.len() as u64 {
					std::fs::write(&path, WORKSHOP_DEFAULT_ICON).expect("Failed to write default icon to temp directory!");
				}
				path
			}
		}
	}
}
impl WorkshopIcon {
	pub fn new<P: AsRef<Path>>(path: P, upscale: bool) -> Result<WorkshopIcon, PublishError> {
		let path = path.as_ref();

		let len = path.metadata()?.len();
		if len > WORKSHOP_ICON_MAX_SIZE {
			return Err(PublishError::IconTooLarge);
		} else if len < WORKSHOP_ICON_MIN_SIZE {
			return Err(PublishError::IconTooSmall);
		}

		let file_extension = path.extension().and_then(|x| x.to_str()).unwrap_or("jpg").to_ascii_lowercase();
		let image_format = match file_extension.as_str() {
			"png" => ImageFormat::Png,
			"gif" => ImageFormat::Gif,
			"jpeg" | "jpg" => ImageFormat::Jpeg,
			_ => return Err(PublishError::IconInvalidFormat),
		};

		let image = image::load(BufReader::new(File::open(path)?), image_format)?;
		Ok(WorkshopIcon::Custom {
			path: path.to_path_buf(),
			width: image.width(),
			height: image.height(),
			format: image_format,
			upscale,
			image,
		})
	}
}

pub enum WorkshopUpdateType {
	Creation {
		title: String,
		path: ContentPath,
		tags: Vec<String>,
		addon_type: String,
		preview: WorkshopIcon,
	},
	Update {
		title: String,
		path: ContentPath,
		tags: Vec<String>,
		addon_type: String,
		preview: Option<WorkshopIcon>,
		changes: Option<String>,
	},
}

impl Steam {
	pub fn update(&self, id: PublishedFileId, details: WorkshopUpdateType, transaction: &Transaction) -> Result<bool, PublishError> {
		use WorkshopUpdateType::*;

		let result = Arc::new(Mutex::new(None));
		let result_ref = result.clone();
		let update_handle = match details {
			Creation {
				title,
				path,
				mut tags,
				addon_type,
				preview,
			} => {
				tags.reserve(tags.len() + 2);
				tags.push("Addon".to_string());
				tags.push(addon_type);

				self.client()
					.ugc()
					.start_item_update(GMOD_APP_ID, id)
					.content_path(&path)
					.title(&title)
					.preview_path(&Into::<PathBuf>::into(preview))
					.tags(tags)
					.submit(None, move |result| {
						*result_ref.lock() = Some(result);
					})
			}

			Update {
				title,
				path,
				tags,
				addon_type,
				preview,
				changes,
			} => {
				let mut tags = tags;
				tags.reserve(tags.len() + 2);
				tags.push("Addon".to_string());
				tags.push(addon_type);

				let preview_path: Option<PathBuf> = match preview {
					Some(preview) => Some(preview.into()),
					None => None,
				};

				let update = self.client().ugc().start_item_update(GMOD_APP_ID, id);
				match preview_path {
					Some(preview_path) => update.preview_path(&preview_path),
					None => update,
				}
				.content_path(&path)
				.tags(tags)
				.title(&title)
				.submit(changes.as_deref(), move |result| {
					*result_ref.lock() = Some(result);
				})
			}
		};

		let mut last_processed;
		let result = loop {
			let (processed, progress, total) = update_handle.progress();
			last_processed = processed;
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
			if total == 0 || last_processed != processed {
				transaction.progress_reset();
			} else {
				transaction.data(total);
				transaction.progress(progress as f64 / total as f64);
			}

			if !result.is_locked() && result.lock().is_some() {
				break Arc::try_unwrap(result).unwrap().into_inner().unwrap();
			} else {
				self.run_callbacks();
			}
		};

		match result {
			Ok((_, legal_agreement)) => {
				transaction.progress(1.);
				Ok(legal_agreement)
			}
			Err(error) => Err(PublishError::SteamError(error)),
		}
	}

	pub fn publish(&self, details: WorkshopUpdateType, transaction: &Transaction) -> (Option<PublishedFileId>, Result<bool, PublishError>) {
		debug_assert!(matches!(details, WorkshopUpdateType::Creation { .. }));

		let published = Arc::new(Mutex::new(None));
		let published_ref = published.clone();
		self.client()
			.ugc()
			.create_item(GMOD_APP_ID, steamworks::FileType::Community, move |result| {
				*published_ref.lock() = Some(result);
			});

		loop {
			if let Some(published_ref) = published.try_lock() {
				if published_ref.is_some() {
					break;
				}
			}
			self.run_callbacks();
		}

		let id = match Arc::try_unwrap(published).unwrap().into_inner().unwrap() {
			Ok((id, _)) => id,
			Err(error) => return (None, Err(PublishError::SteamError(error))),
		};

		(Some(id), self.update(id, details, transaction))
	}

	pub fn update_icon(&self, addon_id: PublishedFileId, icon: WorkshopIcon, transaction: &Transaction) -> Result<bool, PublishError> {
		let result = Arc::new(Mutex::new(None));
		let result_ref = result.clone();
		let update_handle = self.client()
			.ugc()
			.start_item_update(GMOD_APP_ID, addon_id)
			.preview_path(&Into::<PathBuf>::into(icon))
			.submit(None, move |result| {
				*result_ref.lock() = Some(result);
			});

		let mut last_processed;
		let result = loop {
			let (processed, progress, total) = update_handle.progress();
			last_processed = processed;
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
			if total == 0 || last_processed != processed {
				transaction.progress_reset();
			} else {
				transaction.data(total);
				transaction.progress(progress as f64 / total as f64);
			}

			if !result.is_locked() && result.lock().is_some() {
				break Arc::try_unwrap(result).unwrap().into_inner().unwrap();
			} else {
				self.run_callbacks();
			}
		};

		match result {
			Ok((_, legal_agreement)) => {
				transaction.progress(1.);
				Ok(legal_agreement)
			}
			Err(error) => Err(PublishError::SteamError(error)),
		}
	}
}

#[tauri::command]
pub fn verify_whitelist(path: PathBuf) -> Result<(Vec<GMAEntry>, u64), PublishError> {
	if !path.is_dir() || !path.is_absolute() {
		return Err(PublishError::InvalidContentPath);
	}

	let root_path_strip_len = path.to_slash_lossy().len() + 1;

	let ignore: Vec<String> = app_data!()
		.settings
		.read()
		.ignore_globs
		.iter()
		.map(|x| {
			let mut x = x.to_string();
			x.push('\0');
			x
		})
		.collect();

	let mut size = 0;
	let mut failed_extra = false;
	let mut failed = Vec::with_capacity(10);
	let mut files = Vec::new();

	#[cfg(not(target_os = "windows"))]
	let mut dedup: HashSet<String> = HashSet::new();

	for (path, relative_path) in WalkDir::new(&path)
		.follow_links(true)
		.contents_first(true)
		.into_iter()
		.filter_map(|entry| {
			let path = match entry {
				Ok(entry) => entry.into_path(),
				Err(err) => match err.path() {
					Some(path) => path.to_path_buf(),
					None => return None,
				},
			};

			if path.is_dir() {
				return None;
			}

			let relative_path = {
				let mut relative_path = path.to_slash_lossy();
				if relative_path.len() < root_path_strip_len {
					return None;
				}
				relative_path.split_off(root_path_strip_len).to_lowercase()
			};

			Some((path, relative_path))
		})
		.filter(|(_, relative_path)| crate::gma::whitelist::filter_default_ignored(relative_path))
		.filter(|(_, relative_path)| !crate::gma::whitelist::is_ignored(relative_path, &ignore))
	{
		#[cfg(not(target_os = "windows"))]
		{
			if !dedup.insert(relative_path.to_owned()) {
				return Err(PublishError::DuplicateEntry(relative_path));
			}
		}

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
				index: 0,
			});
		}
	}

	// TODO some tasks shouldnt be cancelable (i.e. showing the cross button)

	if failed.is_empty() {
		if files.is_empty() {
			Err(PublishError::NoEntries)
		} else {
			Ok((files, size))
		}
	} else {
		failed.sort_unstable();

		if failed_extra {
			failed.push("...".to_string());
		}

		Err(PublishError::NotWhitelisted(failed))
	}
}

#[tauri::command]
pub fn publish_icon(
	icon_path: PathBuf,
	upscale: bool,
	addon_id: PublishedFileId
) -> u32 {
	let transaction = transaction!();
	let id = transaction.id;

	rayon::spawn(move || {
		let preview = match WorkshopIcon::new(icon_path, upscale) {
			Ok(icon) => icon,
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				return;
			}
		};

		let result = steam!().update_icon(addon_id, preview, &transaction);

		match result {
			Ok(legal_agreement) => {
				if legal_agreement {
					crate::path::open("https://steamcommunity.com/workshop/workshoplegalagreement");
				}
				transaction.finished(turbonone!());
			}
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
			}
		};
	});

	id
}

#[tauri::command]
pub fn publish(
	content_path_src: PathBuf,
	icon_path: Option<PathBuf>,
	title: String,
	tags: Vec<String>,
	addon_type: String,
	upscale: bool,
	update_id: Option<PublishedFileId>,
	changes: Option<String>,
) -> u32 {
	let transaction = transaction!();
	let id = transaction.id;

	let is_updating = update_id.is_some();

	rayon::spawn(move || {
		let preview = match icon_path {
			Some(icon_path) => {
				transaction.status("PUBLISH_PROCESSING_ICON");

				match WorkshopIcon::new(icon_path, upscale) {
					Ok(icon) => Some(icon),
					Err(error) => {
						transaction.error(error.to_string(), turbonone!());
						return;
					}
				}
			}
			None => {
				if !is_updating {
					Some(WorkshopIcon::Default)
				} else {
					None
				}
			}
		};

		transaction.status("PUBLISH_PACKING");

		let mut path = app_data!().temp_dir().to_owned();
		path.pop();
		path.push("gmpublisher_publishing");

		if let Err(_) = std::fs::create_dir_all(&path) {
			transaction.error("ERR_IO_ERROR", turbonone!());
			return;
		}

		path.push("gmpublisher.gma");

		{
			let gma = GMAFile {
				// TODO convert to GMAFile::new()
				path: path.clone(),
				size: 0,
				id: None,
				metadata: Some(GMAMetadata::Standard {
					title: title.clone(),
					addon_type: addon_type.clone(),
					tags: tags.clone(),
					ignore: app_data!().settings.read().ignore_globs.clone(),
				}),
				entries: None,
				pointers: GMAFilePointers::default(),
				version: 3,
				extracted_name: String::new(),
				modified: None,
				membuffer: None,
			};

			if let Err(error) = gma.create(&content_path_src, transaction.clone()) {
				if !transaction.aborted() {
					transaction.error(error.to_string(), turbonone!());
				}
				return;
			}
		}

		let mut content_path = path.clone();
		content_path.pop();

		let content_path = match ContentPath::new(content_path) {
			Ok(content_path) => content_path,
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				return;
			}
		};

		transaction.status("PUBLISH_STARTING");

		let (id, result) = if let Some(id) = update_id {
			(
				update_id,
				steam!().update(
					id,
					WorkshopUpdateType::Update {
						title,
						path: content_path,
						tags,
						addon_type,
						preview,
						changes,
					},
					&transaction,
				),
			)
		} else {
			steam!().publish(
				WorkshopUpdateType::Creation {
					title,
					path: content_path,
					tags,
					addon_type,
					preview: preview.unwrap(),
				},
				&transaction,
			)
		};

		ignore! { std::fs::remove_file(path) };

		match result {
			Ok(legal_agreement) => {
				if legal_agreement {
					crate::path::open("https://steamcommunity.com/workshop/workshoplegalagreement");
				}

				let id = id.unwrap();

				crate::path::open(format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id.0));

				transaction.finished(turbonone!());

				app_data!().settings.write().my_workshop_local_paths.insert(id, content_path_src);
				ignore! { app_data!().settings.read().save() };
				app_data!().send();
			}
			Err(error) => {
				transaction.error(error.to_string(), turbonone!());
				if !is_updating {
					if let Some(id) = id {
						steam!().client().ugc().delete_item(id, |_| {});
					}
				}
			}
		};
	});

	id
}

#[tauri::command]
pub fn verify_icon(path: PathBuf) -> Result<(String, bool), Transaction> {
	WorkshopIcon::new(&path, false)
		.and_then(|icon| {
			let (prefix, can_upscale) = match icon {
				WorkshopIcon::Custom { format, width, height, .. } => (
					format!(
						"data:image/{};base64,",
						match format {
							ImageFormat::Png => "png",
							ImageFormat::Jpeg => "jpeg",
							ImageFormat::Gif => "gif",
							_ => unreachable!(),
						}
					),
					WorkshopIcon::can_upscale(width, height, format),
				),
				_ => unreachable!(),
			};
			let base64 = base64::encode(std::fs::read(path)?);
			Ok((prefix + &base64, can_upscale))
		})
		.map_err(|error| {
			let transaction = transaction!();
			transaction.error(error.to_string(), turbonone!());
			transaction
		})
}

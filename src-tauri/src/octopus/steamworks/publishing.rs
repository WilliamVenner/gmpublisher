use crate::GMOD_APP_ID;
use anyhow::anyhow;
use image::{ImageError, ImageFormat};
use parking_lot::RwLock;
use std::{fs::File, io::BufReader, mem::MaybeUninit, path::PathBuf, sync::Arc};
use steamworks::PublishedFileId;

use super::Steamworks;
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
	pub fn new(path: PathBuf) -> Result<ContentPath, anyhow::Error> {
		if !path.is_dir() {
			return Err(anyhow!("ERR_CONTENT_PATH_NOT_DIR"));
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
				return Err(anyhow!("ERR_CONTENT_PATH_MULTIPLE_GMA"));
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
pub struct WorkshopIcon(PathBuf);
impl std::ops::Deref for WorkshopIcon {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl Into<PathBuf> for WorkshopIcon {
	fn into(self) -> PathBuf {
		self.0
	}
}
impl WorkshopIcon {
	fn try_format(tried_best_guess: bool, file_type: ImageFormat, path: &PathBuf, mut file_types: Vec<ImageFormat>) -> Result<(), anyhow::Error> {
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
					Err(anyhow!("ERR_ICON_INVALID_FORMAT"))
				} else {
					WorkshopIcon::try_format(tried_best_guess, file_types.remove(0), path, file_types)
				}
			} else {
				Err(anyhow!(error))
			}
		} else {
			Ok(())
		}
	}

	pub fn new(path: PathBuf) -> Result<WorkshopIcon, anyhow::Error> {
		let len = path.metadata()?.len();
		if len > WORKSHOP_ICON_MAX_SIZE {
			return Err(anyhow!("ERR_ICON_TOO_LARGE"));
		} else if len < WORKSHOP_ICON_MIN_SIZE {
			return Err(anyhow!("ERR_ICON_TOO_SMALL"));
		}

		/*let (w, h) = image::image_dimensions(&path)?;
		if w != 512 || h != 512 {
			return Err(anyhow!("ERR_ICON_INCORRECT_DIMENSIONS"));
		}*/

		let file_extension = path.extension().and_then(|x| x.to_str()).unwrap_or("jpg").to_ascii_lowercase();
		let mut file_types = match file_extension.as_str() {
			"png" => vec![ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif],
			"gif" => vec![ImageFormat::Gif, ImageFormat::Jpeg, ImageFormat::Png],
			_ => vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Gif],
		};

		WorkshopIcon::try_format(false, file_types.remove(0), &path, file_types)?;

		Ok(WorkshopIcon(path))
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

impl Steamworks {
	pub fn update(&self, details: WorkshopUpdateType) -> Result<(PublishedFileId, bool), anyhow::Error> {
		use WorkshopUpdateType::*;

		let result = Arc::new(RwLock::new(None));
		let result_ref = result.clone();
		match details {
			Creation(details) => {
				self
					.client()
					.ugc()
					.start_item_update(GMOD_APP_ID, details.id)
					.content_path(&details.path)
					.title(&details.title)
					.preview_path(&details.preview)
					.submit(None, move |result| {
						*result_ref.write() = Some(result);
					});
			}

			Update(details) => {
				let path = ContentPath::new(details.path)?;
				let preview = match details.preview {
					Some(preview) => Some(WorkshopIcon::new(preview)?),
					None => None,
				};

				let update = self.client().ugc().start_item_update(GMOD_APP_ID, details.id);
				match preview {
					Some(preview) => update.preview_path(&*preview),
					None => update,
				}
				.content_path(&path)
				.submit(details.changes.as_deref(), move |result| {
					*result_ref.write() = Some(result);
				});
			}
		}

		loop {
			if !result.is_locked() && result.read().is_some() {
				break Arc::try_unwrap(result).unwrap().into_inner().unwrap().map_err(|error| anyhow!(error));
			} else {
				self.run_callbacks();
			}
		}
	}

	pub fn publish(&self, path: PathBuf, title: String, preview: PathBuf) -> Result<(PublishedFileId, bool), anyhow::Error> {
		let path = ContentPath::new(path)?;
		let preview = WorkshopIcon::new(preview)?;

		let published = Arc::new(RwLock::new(None));
		let published_ref = published.clone();
		self
			.client()
			.ugc()
			.create_item(GMOD_APP_ID, steamworks::FileType::Community, move |result| {
				match result {
					Ok((id, _accepted_legal_agreement)) => {
						// TODO test accepted_legal_agreement
						*published_ref.write() = Some(Some(id));
					}
					Err(_) => {
						*published_ref.write() = Some(None);
					}
				}
			});

		loop {
			if let Some(published_ref) = published.try_read() {
				if published_ref.is_some() {
					break;
				}
			}
			self.run_callbacks();
		}

		let id = Arc::try_unwrap(published).unwrap().into_inner().unwrap().ok_or(anyhow!("ERR_PUBLISH_FAILED"))?;

		self.update(WorkshopUpdateType::Creation(WorkshopCreationDetails { id, title, preview, path }))
	}
}
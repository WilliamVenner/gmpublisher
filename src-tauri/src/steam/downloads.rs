use parking_lot::{Condvar, Mutex, MutexGuard};
use rayon::{ThreadPool, ThreadPoolBuilder};

use std::{
	path::PathBuf,
	sync::{atomic::AtomicBool, Arc},
};

use steamworks::{ClientManager, ItemState, PublishedFileId, QueryResults, UGC};

use crate::{gma::{ExtractGMAMut, ExtractDestination}, transaction, transactions::Transaction, webview_emit, GMAFile, GMOD_APP_ID};

lazy_static! {
	pub static ref DOWNLOADS: Downloads = Downloads::init();
	static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

#[derive(Debug)]
pub struct DownloadInner {
	item: PublishedFileId,
	transaction: Transaction,
	sent_total: AtomicBool,
	extract_destination: ExtractDestination,
}
impl std::hash::Hash for DownloadInner {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.item.hash(state);
	}
}
impl Eq for DownloadInner {}
impl PartialEq for DownloadInner {
	fn eq(&self, other: &Self) -> bool {
		self.item == other.item
	}
}
impl std::ops::Deref for DownloadInner {
	type Target = PublishedFileId;
	fn deref(&self) -> &Self::Target {
		&self.item
	}
}
pub type Download = Arc<DownloadInner>;
pub struct Downloads {
	pending: Mutex<Vec<Download>>, // TODO consider using VecDeque?
	downloading: Mutex<Vec<Download>>,
	watchdog: Condvar,
}
pub struct IDList {
	inner: Vec<PublishedFileId>,
}
impl Into<Vec<PublishedFileId>> for IDList {
	fn into(self) -> Vec<PublishedFileId> {
		self.inner
	}
}
impl From<PublishedFileId> for IDList {
	fn from(id: PublishedFileId) -> Self {
		IDList { inner: vec![id] }
	}
}
impl From<Vec<PublishedFileId>> for IDList {
	fn from(ids: Vec<PublishedFileId>) -> Self {
		IDList { inner: ids }
	}
}
impl Downloads {
	fn init() -> Self {
		Self {
			pending: Mutex::new(Vec::new()),
			downloading: Mutex::new(Vec::new()),
			watchdog: Condvar::new(),
		}
	}

	fn extract(folder: PathBuf, item: PublishedFileId, extract_destination: ExtractDestination) {
		THREAD_POOL.spawn(move || {
			let transaction = transaction!();

			webview_emit!(
				"ExtractionStarted",
				(
					transaction.id,
					turbonone!(),
					turbonone!(),
					Some(item)
				)
			);

			let mut gma = if folder.is_dir() {
				let mut gma_path = None;

				if let Ok(read_dir) = folder.read_dir() {
					for entry in read_dir {
						if let Ok(entry) = entry {
							if !crate::path::has_extension(entry.path(), "gma") {
								continue;
							}
							if gma_path.is_some() {
								// TODO better handling here - just include the extra files in the addon
								gma_path = None;
								break;
							} else {
								gma_path = Some(entry.path());
							}
						}
					}
				}

				if let Some(path) = gma_path {
					match GMAFile::open(path) {
						Ok(gma) => gma,
						Err(err) => return transaction.error(err.to_string(), turbonone!()),
					}
				} else {
					return transaction.error("ERR_DOWNLOAD_MISSING", turbonone!());
				}
			} else if folder.is_file() && crate::path::has_extension(&folder, "bin") {

				match GMAFile::open(&folder) {
					Ok(gma) => gma,
					Err(_) => match GMAFile::decompress(folder) {
						Ok(gma) => gma,
						Err(err) => return transaction.error(err.to_string(), turbonone!()),
					}
				}

			} else {
				return transaction.error("ERR_DOWNLOAD_MISSING", turbonone!());
			};

			gma.id = Some(item);

			transaction.data((Some(gma.metadata.as_ref().map(|metadata| metadata.title().to_owned())), gma.size));

			if let Err(err) = gma.extract(extract_destination, &transaction, false) {
				transaction.error(err.to_string(), turbonone!());
			}
		});
	}

	fn push_download(
		ugc: &UGC<ClientManager>,
		pending: &mut MutexGuard<Vec<Arc<DownloadInner>>>,
		extract_destination: &Arc<ExtractDestination>,
		item: PublishedFileId,
	) {
		let state = ugc.item_state(item);
		if state.intersects(ItemState::INSTALLED) && !state.intersects(ItemState::NEEDS_UPDATE) {
			if let Some(info) = ugc.item_install_info(item) {
				Downloads::extract(PathBuf::from(info.folder), item, (&**extract_destination).clone());
			} else {
				let transaction = transaction!();
				webview_emit!("DownloadStarted", transaction.id);
				transaction.data((0, item));
				transaction.error("ERR_DOWNLOAD_MISSING", turbonone!());
			}
		} else {
			let download = Arc::new(DownloadInner {
				item,
				sent_total: AtomicBool::new(false),
				transaction: transaction!(),
				extract_destination: (&**extract_destination).clone(),
			});

			webview_emit!("DownloadStarted", download.transaction.id);
			download.transaction.data((0, item));

			pending.push(download);
		}
	}

	pub fn download<IDs: Into<IDList>>(&self, ids: IDs) {
		let mut ids: Vec<PublishedFileId> = ids.into().into();
		let extract_destination = Arc::new(app_data!().settings.read().extract_destination.to_owned());
		let possible_collections: Vec<PublishedFileId> = {
			let workshop_cache = &steam!().workshop.read().0;
			let mut possible_collections = Vec::with_capacity(ids.len());
			ids = ids
				.into_iter()
				.filter(|id| {
					if workshop_cache.contains(id) {
						true
					} else {
						possible_collections.push(*id);
						false
					}
				})
				.collect();
			possible_collections
		};

		if !possible_collections.is_empty() {
			let possible_collections_len = possible_collections.len();
			let extract_destination = extract_destination.clone();
			if steam!().connected() {
				let done = Arc::new(());

				let done_ref = done.clone();
				steam!()
					.client()
					.ugc()
					.query_items(possible_collections.clone())
					.unwrap()
					.include_children(true)
					.fetch(move |results: Result<QueryResults<'_>, steamworks::SteamError>| {
						if let Ok(results) = results {
							let mut pending = downloads!().pending.lock();
							pending.reserve(results.returned_results() as usize);

							let mut not_collections = Vec::with_capacity(possible_collections_len);

							let ugc = steam!().client().ugc();
							for (i, item) in results.iter().enumerate() {
								if let Some(item) = item {
									if item.file_type == steamworks::FileType::Collection {
										let children = results.get_children(i as u32).unwrap();
										steam!().fetch_workshop_items(children.clone());
										for item in children {
											Downloads::push_download(&ugc, &mut pending, &extract_destination, item);
										}
									} else {
										not_collections.push(item.published_file_id);
										Downloads::push_download(&ugc, &mut pending, &extract_destination, item.published_file_id);
									}
								} else {
									let transaction = transaction!();
									webview_emit!("DownloadStarted", transaction.id);
									transaction.data((0, possible_collections[i]));
									transaction.error("ERR_ITEM_NOT_FOUND", turbonone!());
								}
							}

							if !not_collections.is_empty() {
								steam!().fetch_workshop_items(not_collections);
							}
						}

						drop(done_ref);
					});

				while Arc::strong_count(&done) > 1 {
					sleep_ms!(25);
				}
			}
		}

		let mut pending = self.pending.lock();
		pending.reserve(ids.len());

		let ugc = steam!().client().ugc();
		for item in ids {
			Downloads::push_download(&ugc, &mut pending, &extract_destination, item);
		}

		if !pending.is_empty() {
			drop(pending);
			self.start();
		}
	}

	pub fn start(&self) {
		let mut downloading = self.downloading.lock();
		downloading.append(&mut self.pending.lock());

		self.watchdog.notify_one();
	}

	pub(super) fn watchdog() {
		let in_progress: Arc<Mutex<Vec<Arc<DownloadInner>>>> = Arc::new(Mutex::new(vec![]));
		let in_progress_ref = in_progress.clone();
		let _cb = steam!().register_callback(move |result: steamworks::DownloadItemResult| {
			if result.app_id == GMOD_APP_ID {
				let mut in_progress = in_progress_ref.lock();
				if let Ok(pos) = in_progress.binary_search_by_key(&result.published_file_id.0, |download| download.0) {
					let download = in_progress.remove(pos);
					if let Some(error) = result.error {
						dprintln!("ISteamUGC Download ERROR: {:?}", download.item);
						download.transaction.error("ERR_STEAM_ERROR", error);
					} else if let Some(info) = steam!().client().ugc().item_install_info(result.published_file_id) {
						dprintln!("ISteamUGC Download SUCCESS: {:?}", download.item);
						download.transaction.finished(turbonone!());
						Downloads::extract(
							PathBuf::from(info.folder),
							download.item,
							Arc::try_unwrap(download).unwrap().extract_destination,
						);
					} else {
						dprintln!("ISteamUGC Download MISSING: {:?}", download.item);
						download.transaction.error("ERR_DOWNLOAD_MISSING", turbonone!());
					}
				} else {
					dprintln!("ISteamUGC Download ???: {:?}", result.published_file_id);
				}
			}
		});

		loop {
			let downloading = std::mem::take(&mut *DOWNLOADS.downloading.lock());
			if downloading.is_empty() {
				DOWNLOADS.watchdog.wait(&mut DOWNLOADS.downloading.lock());
				continue;
			}

			let ugc = steam!().client().ugc();

			{
				let mut in_progress = in_progress.lock();
				in_progress.reserve(downloading.len());

				for download in downloading {
					let pos = match in_progress.binary_search_by_key(&download.item, |x| x.item) {
						Ok(_) => continue,
						Err(pos) => pos,
					};

					if !ugc.download_item(download.item, true) {
						download.transaction.error("ERR_DOWNLOAD_FAILED", turbonone!());
						continue;
					} else {
						dprintln!("Starting ISteamUGC Download for {:?}", download.item);
					}

					in_progress.insert(pos, download);
				}
			}

			loop {
				if let Some(mut in_progress) = in_progress.try_lock() {
					if in_progress.is_empty() {
						break;
					} else {
						let mut i = 0;
						while i < in_progress.len() {
							let download = &in_progress[i];
							if download.transaction.aborted() {
								in_progress.remove(i);
							} else if let Some((current, total)) = ugc.item_download_info(download.item) {
								if total > 0 {
									if !download.sent_total.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
										download.transaction.data((1, total));
									}
									download.transaction.progress(current as f64 / total as f64);
								}
							}
							i += 1;
						}
					}
				}
				steam!().run_callbacks();
			}
		}
	}
}

#[tauri::command]
pub fn workshop_download(ids: Vec<PublishedFileId>) {
	downloads!().download(ids);
}

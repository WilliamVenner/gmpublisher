use lazy_static::lazy_static;
use parking_lot::{Condvar, Mutex, RwLock};
use rayon::{
	iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
};
use serde::Serialize;
use std::{collections::HashMap, hash::Hash, mem::MaybeUninit, path::PathBuf, sync::{atomic::AtomicBool, Arc}};

use steamworks::{AccountId, AppId, Callback, CallbackHandle, Client, ClientManager, Friend, ItemState, PublishedFileId, QueryResult, QueryResults, SingleClient, SteamError, SteamId, SteamServerConnectFailure, SteamServersConnected, SteamServersDisconnected};

use atomic_refcell::AtomicRefCell;

use super::{THREAD_POOL, AtomicRefSome, PromiseCache, PromiseHashCache};

use crate::{main_thread_forbidden, transaction, GMOD_APP_ID, webview_emit, steamworks, transactions::Transaction};

lazy_static! {
	pub static ref DOWNLOADS: Downloads = Downloads::init();
	static ref ITEM_STATE_SKIP_DOWNLOAD: ItemState = ItemState::DOWNLOAD_PENDING | ItemState::DOWNLOADING;
}
#[derive(Debug)]
pub struct DownloadInner {
	id: PublishedFileId,
	transaction: Transaction,
	sent_total: AtomicBool
}
impl std::hash::Hash for DownloadInner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl Eq for DownloadInner {}
impl PartialEq for DownloadInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl std::ops::Deref for DownloadInner {
    type Target = PublishedFileId;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
pub type Download = Arc<DownloadInner>;
pub struct Downloads {
	pending: Mutex<Vec<Download>>,
	downloading: Mutex<Vec<Download>>,
	watchdog: Condvar,
}
pub struct IDList {
	inner: Vec<PublishedFileId>
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

	pub fn download<IDs: Into<IDList>>(&self, ids: IDs) {
		let ids: Vec<PublishedFileId> = ids.into().into();

		let mut pending = self.pending.lock();
		pending.reserve(ids.len());

		for id in ids {
			let download = Arc::new(DownloadInner {
				id,
				sent_total: AtomicBool::new(false),
				transaction: transaction!()
			});

			pending.push(download.clone());

			webview_emit!("DownloadStarted", download.transaction.id);
		}
	}

	pub fn start(&self) {
		let mut downloading = self.downloading.lock();
		downloading.append(&mut self.pending.lock());
		
		self.watchdog.notify_one();
	}

	pub(super) fn watchdog() {
		loop {
			DOWNLOADS.watchdog.wait(&mut DOWNLOADS.downloading.lock());

			let ugc = steamworks!().client().ugc();

			loop {
				let mut downloading = std::mem::take(&mut *DOWNLOADS.downloading.lock());
				if downloading.is_empty() { break; }

				downloading.sort_unstable_by_key(|download| download.0);
				downloading.dedup_by_key(|download| download.0);

				let downloading = Arc::new(Mutex::new(downloading));

				let downloading_ref = downloading.clone();
				let _cb = steamworks!().register_callback(move |result: steamworks::DownloadItemResult| {
					if result.app_id == GMOD_APP_ID {
						let mut downloading = downloading_ref.lock();
						if let Ok(pos) = downloading.binary_search_by_key(&result.published_file_id.0, |download| download.0) {
							let download = downloading.remove(pos);
							if let Some(error) = result.error {
								download.transaction.error(("ERR_STEAM_ERROR", format!("{}", error)));
							} else if let Some(info) = steamworks!().client().ugc().item_install_info(result.published_file_id) {
								download.transaction.finished(Some(info.folder));
							} else {
								download.transaction.error("ERR_DOWNLOAD_FAILED");
							}
						}
					}
				});
				
				let started = std::time::Instant::now();

				loop {
					let mut downloading = match downloading.try_lock() {
						Some(lock) => lock,
						None => continue,
					};

					if downloading.is_empty() { break; }

					let mut i = 0;
					while i != downloading.len() {
						let download = &mut downloading[i];

						let state = ugc.item_state(download.id);
						if state.intersects(ItemState::INSTALLED) && !state.intersects(ItemState::NEEDS_UPDATE) {
							if let Some(info) = ugc.item_install_info(download.id) {
								download.transaction.finished(Some(info.folder));
								downloading.remove(i);
								continue;
							}
						} else if !state.intersects(*ITEM_STATE_SKIP_DOWNLOAD) {
							if !ugc.download_item(download.id, true) {
								download.transaction.error("ERR_DOWNLOAD_FAILED");
								downloading.remove(i);
								continue;
							} else {
								dprintln!("Starting ISteamUGC Download for {:?}", download.id);
							}
						}

						if let Some((downloaded, total)) = ugc.item_download_info(download.id) {
							if total != 0 {
								if !download.sent_total.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
									download.transaction.data(total);
								}

								download.transaction.progress((downloaded as f64) / (total as f64));

								i += 1;
								continue;
							}
						}

						if started.elapsed().as_secs() >= 10 {
							download.transaction.error("ERR_DOWNLOAD_TIMEOUT");
							downloading.remove(i);
						} else {
							i += 1;
						}
					}

					sleep_ms!(50);
				}
			}
		}
	}
}
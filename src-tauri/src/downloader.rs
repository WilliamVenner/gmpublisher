use std::{path::PathBuf, sync::{Arc, Mutex, atomic::AtomicBool}, thread::JoinHandle, time::Duration};

use steamworks::PublishedFileId;
use tauri::Webview;

use crate::{transaction_data, transactions::{Transaction, TransactionChannel, Transactions}};

struct ActiveDownload {
	transaction: Arc<Transaction>,
	channel: TransactionChannel,
	id: PublishedFileId,
	sent_data: bool
}

pub(crate) struct WorkshopDownloader {
	downloads: Arc<Mutex<Vec<ActiveDownload>>>,
	thread: Option<JoinHandle<()>>,
	kill: Arc<AtomicBool>,
}
impl WorkshopDownloader {
	pub(crate) fn init() -> Self {
		Self {
			thread: None,
			downloads: Arc::new(Mutex::new(Vec::new())),
			kill: Arc::new(AtomicBool::new(false))
		}
	}

	pub(crate) fn kill(&mut self) {
		if self.thread.is_none() { return; }
		self.kill.store(true, std::sync::atomic::Ordering::Release);
		self.thread.take().unwrap().join().ok();
	}

	pub(crate) fn listen(&mut self) {
		if self.thread.is_some() { return; }

		self.kill = Arc::new(AtomicBool::new(false));

		let active_downloads = self.downloads.clone();
		let kill = self.kill.clone();

		self.thread = Some(std::thread::spawn(move || {
			// TODO calculate download speed

			let mut killed = false;
			loop {
				if kill.load(std::sync::atomic::Ordering::Acquire) { killed = true; break; }

				let mut active_downloads = active_downloads.lock().unwrap();

				let mut finished = active_downloads.is_empty();

				if !finished {
					finished = true;
					let ugc = crate::WORKSHOP.read().unwrap().client.ugc();
					let mut i = 0;
					while i < active_downloads.len() {
						let mut download = active_downloads.get_mut(i).unwrap();
						i = i + 1;

						match ugc.item_download_info(download.id) {
							Some((downloaded, total)) => {
								if total == 0 { continue; }
								if downloaded != total {
									if !download.sent_data {
										download.sent_data = true;
										download.channel.data(transaction_data!(total));
									}

									finished = false;
									let progress = (downloaded as f64) / (total as f64);
									if download.transaction.progress() != progress {
										download.channel.progress(progress);
									}
									continue;
								}
							},
							None => {}
						}

						download.channel.finish(transaction_data!(()));
						active_downloads.remove(i - 1);
					}
				}

				if finished { break; }

				std::thread::sleep(Duration::from_millis(50));
			}

			if !killed {
				match crate::WORKSHOP_DOWNLOADER.try_write() {
					Ok(mut write) => write.thread = None,
					Err(_) => {
						#[cfg(debug_assertions)]
						println!("[WorkshopDownloader] Failed to delete JoinHandle for listener thread.")
					}
				}
			}
		}));
	}
}

pub(crate) fn download(callback: String, reject: String, webview: &mut Webview<'_>, mut ids: Vec<String>, path: Option<PathBuf>, named_dir: bool, tmp: bool, downloads: bool, addons: bool) -> Result<(), String> {
	let ids = {
		let input_ids = ids.len();
		let ids: Vec<PublishedFileId> = ids.into_iter().filter_map(|x| x.parse::<u64>().ok().map(|x| PublishedFileId(x))).collect();
		if ids.len() != input_ids { return Err("Failed to parse PublishedFileId".to_string()); }
		ids
	};

	let mut webview_muts = Vec::with_capacity(ids.len());
	for _ in 0..ids.len() { webview_muts.push(webview.as_mut()); }
	let mut webview_muts = webview_muts.into_iter();

	tauri::execute_promise(webview, move || {

		let mut transaction_ids: Vec<(usize, PublishedFileId)> = Vec::with_capacity(ids.len());
		let mut failed: Vec<PublishedFileId> = Vec::with_capacity(ids.len());

		let mut downloader = crate::WORKSHOP_DOWNLOADER.write().unwrap();
		let workshop = crate::WORKSHOP.read().unwrap();

		let ugc = workshop.client.ugc();

		let mut downloads = {
			downloader.kill();
			downloader.downloads.lock().unwrap()
		};

		for id in ids {
			if !ugc.download_item(id, true) {
				failed.push(id);
				continue
			}
			
			let transaction = Transactions::new(webview_muts.next().unwrap()).build();
			transaction_ids.push((transaction.id, id));

			downloads.push(ActiveDownload {
				channel: transaction.channel(),
				transaction,
				id,
				sent_data: false
			});
		}

		if !downloads.is_empty() {
			drop(downloads);
			downloader.listen();
		}

		Ok((transaction_ids, failed))

	}, callback, reject);

	Ok(())
}
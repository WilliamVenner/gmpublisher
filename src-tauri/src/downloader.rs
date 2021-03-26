use std::{collections::VecDeque, path::PathBuf, sync::{Arc, RwLock, atomic::{AtomicBool, AtomicU16}}, thread::JoinHandle, time::Duration};

use gma::GMAReadError;
use lazy_static::lazy_static;
use steamworks::{CallbackHandle, InstallInfo, ItemState, PublishedFileId, SteamError};
use sysinfo::SystemExt;
use tauri::{Webview, WebviewMut};
use serde::Serialize;
use anyhow::anyhow;

use crate::{gma::{self, ExtractDestination, GMAFile}, transaction_data, transactions::{Transaction, TransactionChannel, TransactionStatus, Transactions}, util::ThreadWatchdog};

lazy_static! {
	static ref ITEM_STATE_SKIP_DOWNLOAD: ItemState = ItemState::DOWNLOAD_PENDING | ItemState::DOWNLOADING;
}

#[derive(Serialize)]
struct ActiveDownload {
	transaction: Arc<Transaction>,
	id: PublishedFileId,

	#[serde(skip)]
	channel: TransactionChannel,
	#[serde(skip)]
	sent_data: bool,
	#[serde(skip)]
	_cb: CallbackHandle
}
impl ActiveDownload {
	fn new(webview: WebviewMut, id: PublishedFileId, transaction: Arc<Transaction>, dest: ExtractDestination) -> Self {
		let cb = {
			let mut dest = Some(dest);
			let transaction = transaction.clone();
			let channel = transaction.channel();
			let workshop = crate::WORKSHOP.read().unwrap();
			println!("registered callback");
			workshop.client.register_callback(move |downloaded: steamworks::DownloadItemResult| {
				if downloaded.published_file_id != id || dest.is_none() { return; }
				match downloaded.error {
					Some(error) => channel.error(&format!("{}", error), transaction_data!(())),
					None => {
						match download_finished(
							webview.clone(),
							{
								let workshop = crate::WORKSHOP.read().unwrap();
								workshop.client.ugc().item_install_info(id)
							},
							id,
							dest.take().unwrap()
						)
						{
							Ok(transaction_id) => channel.finish(transaction_data!(transaction_id)),
							Err(_) => channel.error(&format!("{}", SteamError::Expired), transaction_data!(()))
						}
					},
				}
			})
		};

		Self {
			id,
			channel: transaction.channel(),
			transaction,
			sent_data: false,
			_cb: cb
		}
	}
}

pub(crate) struct WorkshopDownloader {
	downloads: Arc<RwLock<Vec<ActiveDownload>>>,
	thread: Option<JoinHandle<()>>,
	kill: Arc<AtomicBool>,

	extraction_queue: VecDeque<(Option<PublishedFileId>, PathBuf, bool, ExtractDestination, TransactionChannel, TransactionChannel, TransactionChannel)>,
	extraction_pool: AtomicU16,
}
impl WorkshopDownloader {
	pub(crate) fn init() -> Self {
		Self {
			thread: None,
			downloads: Arc::new(RwLock::new(Vec::new())),
			kill: Arc::new(AtomicBool::new(false)),
			extraction_queue: VecDeque::new(),
			extraction_pool: AtomicU16::new(0)
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
			let mut killed = false;
			loop {
				if kill.load(std::sync::atomic::Ordering::Acquire) { killed = true; break; }

				let mut active_downloads = match active_downloads.try_write() {
					Ok(w) => w,
					Err(err) => match err {
					    std::sync::TryLockError::Poisoned(_) => break,
					    std::sync::TryLockError::WouldBlock => continue
					}
				};

				let mut finished = active_downloads.is_empty();

				if !finished {
					finished = true;
					let ugc = crate::WORKSHOP.read().unwrap().client.ugc();
					let mut i = 0;
					while i < active_downloads.len() {
						let mut download = active_downloads.get_mut(i).unwrap();
						i = i + 1;

						match *download.transaction.status() {
							TransactionStatus::Pending => {
								match ugc.item_download_info(download.id) {
									Some((downloaded, total)) => {
										if !download.transaction.aborted() {
											finished = false;
											if total == 0 { continue; }
											if downloaded != total {
												if !download.sent_data {
													download.sent_data = true;
													download.channel.data(transaction_data!(total));
												}
			
												let progress = (downloaded as f64) / (total as f64);
												if download.transaction.progress() != progress {
													download.channel.progress(progress);
												}
											}
										}
									},
									None => {}
								}
								continue;
							}
							_ => {}
						};

						active_downloads.remove(i - 1);
					}
				}

				if finished { break; }

				if let Ok(workshop) = crate::WORKSHOP.try_read() {
					if let Ok(single) = workshop.single.try_lock() {
						single.run_callbacks();
					}
				}

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

	fn extraction_thread_die() {
		crate::WORKSHOP_DOWNLOADER.write().unwrap()
			.extraction_pool.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
	}

	fn extraction_worker() {
		let _watchdog = ThreadWatchdog::new(WorkshopDownloader::extraction_thread_die);

		loop {
			let mut w = match crate::WORKSHOP_DOWNLOADER.try_write() {
				Ok(w) => w,
				Err(err) => match err {
				    std::sync::TryLockError::Poisoned(_) => break,
				    std::sync::TryLockError::WouldBlock => continue
				}
			};

			if let Some((id, path, compressed, dest, channel, progress_channel, compression_channel)) = w.extraction_queue.pop_front() {
				match (|| -> Result<PathBuf, GMAReadError> {

					let mut gma = if compressed {
						
						let output = {
							let input = std::fs::read(&path).map_err(|_| GMAReadError::IOError)?;
							let mut output = Vec::with_capacity(input.len());

							let total_bytes = input.len();

							let available_memory = ({
								let mut sys = sysinfo::System::new();
								sys.refresh_memory();
								sys.get_available_memory()
							} * 1000) - 1000000000;

							// TODO somehow, in some really unsafe and stupid way, monitor the progress of decompression

							xz2::stream::Stream::new_lzma_decoder(available_memory).map_err(|_| GMAReadError::IOError)?
								.process_vec(&input, &mut output, xz2::stream::Action::Run)
								.map_err(|_| GMAReadError::IOError)?;

							output
						};

						// TODO replace this with a generic GMAFile which can read BufReader::new(Cursor::new(output)), id)

						let output_path = std::env::temp_dir().join(PathBuf::from("gmpublisher_decompress_gma"));

						std::fs::write(
							&output_path,
							output
						).map_err(|_| GMAReadError::IOError)?;

						GMAFile::new(&output_path, id)?
					} else {
						GMAFile::new(&path, id)?
					};

					gma.metadata()?;
					gma.entries()?;

					channel.data(transaction_data!((gma.size, gma.metadata.as_ref().and_then(|m| Some(m.name.clone())))));
					
					gma.extract(dest, Some(Box::new(
						move |progress| progress_channel.progress(progress)
					)))

					/*
					match compressed {
						true => {
							gma.extract(dest, Some(Box::new(
								move |progress| progress_channel.progress(progress / 2.0)
							)))
						},
						false => {
							gma.extract(dest, Some(Box::new(
								move |progress| progress_channel.progress(progress)
							)))
						}
					}
					*/

				})() {
					Ok(path) => channel.finish(transaction_data!(path)),
					Err(err) => channel.error(&format!("{}", err), transaction_data!(()))
				}
			}
			
			break;
		}
	}

	pub(crate) fn extract(id: Option<PublishedFileId>, path: PathBuf, compressed: bool, dest: ExtractDestination, webview_mut: WebviewMut) -> usize {
		let mut downloader = crate::WORKSHOP_DOWNLOADER.write().unwrap();

		let transaction = Transactions::new(webview_mut).build();
		downloader.extraction_queue.push_back((id, path, compressed, dest, transaction.channel(), transaction.channel(), transaction.channel()));

		if downloader.extraction_queue.len() == 1 || (downloader.extraction_pool.load(std::sync::atomic::Ordering::Acquire) as usize) < num_cpus::get() {
			downloader.extraction_pool.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
			std::thread::spawn(WorkshopDownloader::extraction_worker);
		}

		transaction.id
	}
}

fn download_finished(webview_mut: WebviewMut, info: Option<InstallInfo>, id: PublishedFileId, dest: ExtractDestination) -> Result<usize, anyhow::Error> {
	let info = info.ok_or(anyhow!(""))?;

	let path = PathBuf::from(info.folder);

	if path.is_file() {
		match path.extension() {
			Some(extension) => match extension.to_str().unwrap_or("") {
				"bin" => return Ok(WorkshopDownloader::extract(
					Some(id),
					path,
					true,
					dest,
					webview_mut,
				)),

				"gma" => return Ok(WorkshopDownloader::extract(
					Some(id),
					path,
					false, // TODO check if this is actually true?
					dest,
					webview_mut,
				)),

				_ => {}
			},
			_ => {},
		};

		return Err(anyhow!(""));
	}

	for f in path.read_dir()? {
		if let Ok(f) = f {
			let path = f.path();
			if
				match path.extension() {
					Some(extension) => extension == "gma",
					None => false
				}
			{
				return Ok(WorkshopDownloader::extract(
					Some(id),
					path,
					false,
					dest,
					webview_mut,
				))
			}
		}
	}

	Err(anyhow!(""))
}

pub(crate) fn download(callback: String, reject: String, webview: &mut Webview<'_>, ids: Vec<String>, tmp: bool, path: Option<PathBuf>, named_dir: bool, downloads: bool, addons: bool) -> Result<(), String> {
	let ids = {
		let input_ids = ids.len();
		let ids: Vec<PublishedFileId> = ids.into_iter().filter_map(|x| x.parse::<u64>().ok().map(|x| PublishedFileId(x))).collect();
		if ids.len() != input_ids { return Err("Failed to parse PublishedFileId".to_string()); }
		ids
	};
	
	let webview_mut = webview.as_mut();

	tauri::execute_promise(webview, move || {

		let mut transaction_ids: Vec<(usize, PublishedFileId)> = Vec::with_capacity(ids.len());
		let mut installed_transaction_ids: Vec<(usize, PublishedFileId)> = Vec::with_capacity(ids.len());
		let mut failed: Vec<PublishedFileId> = Vec::with_capacity(ids.len());

		crate::WORKSHOP_DOWNLOADER.write().unwrap().kill();

		let workshop = crate::WORKSHOP.read().unwrap();
		let ugc = workshop.client.ugc();

		let dest = ExtractDestination::build(tmp, path.clone(), named_dir, downloads, addons).unwrap_or(ExtractDestination::Temp);

		for id in ids {
			let state = ugc.item_state(id);
			if state.intersects(ItemState::INSTALLED) && !state.intersects(ItemState::NEEDS_UPDATE) {

				match download_finished(
					webview_mut.clone(),
					ugc.item_install_info(id),
					id,
					dest.clone(),
				)
				{
					Err(_) => failed.push(id),
					Ok(transaction_id) => installed_transaction_ids.push((transaction_id, id))
				}

			} else {
				let transaction = Transactions::new(webview_mut.clone()).build();
				transaction_ids.push((transaction.id, id));

				let active_download = ActiveDownload::new(
					webview_mut.clone(),
					id,
					transaction,
					dest.clone(),
				);

				if !state.intersects(*ITEM_STATE_SKIP_DOWNLOAD) {
					if !ugc.download_item(id, true) {
						failed.push(id);
						continue;
					} else {
						#[cfg(debug_assertions)]
						println!("Starting ISteamUGC Download for {:?}", id);
					}
				} else {
					// FIXME handle duplicates
				}

				crate::WORKSHOP_DOWNLOADER.read().unwrap()
				.downloads.write().unwrap()
				.push(active_download);
			}
		}

		if !transaction_ids.is_empty() {
			crate::WORKSHOP_DOWNLOADER.write().unwrap().listen();
		}

		Ok((transaction_ids, installed_transaction_ids, failed))

	}, callback, reject);

	Ok(())
}
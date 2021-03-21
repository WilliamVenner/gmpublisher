use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex, mpsc::{self, Receiver, Sender, SyncSender}}, thread::JoinHandle};

use steamworks::PublishedFileId;
use tauri::Webview;

use serde::Serialize;

use crate::{gma::GMAFile, transactions::{TransactionChannel, Transactions}, workshop::{self, WorkshopItem}};
use crate::game_addons;

#[derive(Debug, Serialize, Clone)]
pub(crate) struct AnalyzedAddon {
	gma: GMAFile,
	preview_url: Option<String>
}
impl From<GMAFile> for AnalyzedAddon {
    fn from(gma: GMAFile) -> Self {
		Self { gma, preview_url: None }
    }
}

// https://www.win.tue.nl/~vanwijk/stm.pdf
use treemap::{TaggedTreeMapData, TreeMap, TreeMapData};

use self::treemap::Square;
mod treemap {
	use serde::Serialize;
	pub(super) type TreeMapData = Box<dyn erased_serde::Serialize + Sync + Send + 'static>;
	pub(super) type TaggedTreeMapData = Option<(Box<dyn erased_serde::Serialize + Sync + Send + 'static>, Option<String>)>;

	/*
	maybe use Struct(a, b)
	
	#[serde(Serialize)]
	pub(super) struct TreeMapData { 
		tag: Option<String>,
		data: Option<Box<dyn erased_serde::Serialize + Sync + Send + 'static>>
	}
	impl TreeMapData {
		fn new(tag: Option<String>, data: Option<Box<dyn erased_serde::Serialize + Sync + Send + 'static>>) -> Self {
			Self { tag, data }
		}
	}
	 */

	#[derive(Serialize)]
	pub(super) struct Square {
		pub(super) data: TaggedTreeMapData,
		pub(super) x: f64,
		pub(super) y: f64,
		pub(super) w: f64,
		pub(super) h: f64
	}

	pub(super) struct TreeMap {
		pub(super) data: Vec<TaggedTreeMapData>,
		pub(super) squares: Vec<Square>,
		pub(super) x: f64,
		pub(super) y: f64,
		pub(super) w: f64,
		pub(super) h: f64
	}
	impl TreeMap {
		pub(super) fn new(w: f64, h: f64) -> Self {
			Self {
				data: Vec::new(),
				squares: Vec::new(),
				x: 0.,
				y: 0.,
				w,
				h,
			}
		}

		pub(super) fn process(&mut self, data_sizes: Vec<f64>, total_size: f64) {
			let mut scaled: Vec<f64> = data_sizes.into_iter().map(|x| (x * self.h * self.w) / total_size).collect();
			//let mut scaled: Vec<f64> = self.data.iter().map(|x| ((x.as_ref().unwrap().size() as f64) * self.h * self.w) / total_size).collect();
			self.squarify(&mut scaled, &mut Vec::new(), self.min_width().0);
		}

		fn squarify(&mut self, squares: &mut Vec<f64>, row: &mut Vec<f64>, w: f64) {
			if squares.len() == 1 {
				return self.layout_last_row(squares, row, w);
			}

			let mut row_with_child: Vec<f64> = row.clone();
			row_with_child.push(squares[0]);

			if row.is_empty() || self.worst_ratio(row, w) >= self.worst_ratio(&row_with_child, w) {
				squares.remove(0);
				return self.squarify(squares, &mut row_with_child, w);
			}

			self.layout_row(row, w, self.min_width().1);
			return self.squarify(squares, &mut Vec::new(), self.min_width().0);
		}

		fn worst_ratio(&self, row: &Vec<f64>, w: f64) -> f64 {
			let mut sum: f64 = 0.;
			let mut max: f64 = 0.;
			let mut min: f64 = f64::MAX;
			for row in row {
				sum = sum + *row;
				max = max.max(*row);
				min = min.min(*row);
			}

			let sumsum = sum.powi(2);
			let ww = w.powi(2);

			f64::max(
				(ww * max) / sumsum,
				sumsum / (ww * min)
			)
		}

		fn min_width(&self) -> (f64, bool) {
			if self.h.powi(2) > self.w.powi(2) {
				(self.w, false)
			} else {
				(self.h, true)
			}
		}

		fn layout_row(&mut self, row: &mut Vec<f64>, w: f64, vertical: bool) {
			let row_height = row.iter().sum::<f64>() / w;

			for row in row {
				let row_width = *row / row_height;
				self.squares.push(if vertical {
					let data = Square {
						x: self.x,
						y: self.y,
						w: row_height,
						h: row_width,
						data: self.data[self.squares.len()].take(),
					};
					self.y = self.y + row_width;
					data
				} else {
					let data = Square {
						x: self.x,
						y: self.y,
						w: row_width,
						h: row_height,
						data: self.data[self.squares.len()].take(),
					};
					self.x = self.x + row_width;
					data
				});
			}

			if vertical {
				self.x = self.x + row_height;
				self.y = self.y - w;
				self.w = self.w - row_height;
			} else {
				self.x = self.x - w;
				self.y = self.y + row_height;
				self.h = self.h - row_height;
			}
		}

		fn layout_last_row(&mut self, squares: &mut Vec<f64>, row: &mut Vec<f64>, w: f64) {
			let vertical = self.min_width().1;
			self.layout_row(row, w, vertical);
			self.layout_row(squares, w, vertical);
		}
	}
}

pub(super) struct AddonSizeAnalyzer {
	analyzed: Mutex<(Vec<AnalyzedAddon>, u64)>,
}
impl AddonSizeAnalyzer {
	pub(super) fn init() -> Self {
		Self { analyzed: Mutex::new((Vec::new(), 0)) }
	}

	pub(super) fn free(&self) {
		*self.analyzed.lock().unwrap() = (Vec::new(), 0);
	}

	pub(super) fn compute(&'static self, resolve: String, reject: String, webview: &mut Webview<'_>, w: f64, h: f64) -> Result<(), String> {
		let webview_mut = webview.as_mut();

		tauri::execute_promise(webview, move || {
	
			let transaction = Transactions::new(webview_mut).build();
			let id = transaction.id;
			let channel = transaction.channel();
			let ws_channel = transaction.channel();
			let taggify_channel = transaction.channel();
			channel.progress_msg("FS_ANALYZER_COMPUTING");

			std::thread::spawn(move || {
				let (gma_files, total_size) = match self.analyze(channel.clone()) {
					Some(thread) => {
						let (gma_files, gma_ids, total_size) = thread.join().unwrap();
						channel.progress_msg("FS_ANALYZER_STEAM");
						
						self.download_steam(ws_channel, gma_files, gma_ids, total_size)
					},
					None => {
						let gma_files = self.analyzed.lock().unwrap();
						(gma_files.0.clone(), gma_files.1)
					}
				};

				channel.progress_msg("FS_ANALYZER_TAGGIFYING");

				let treemap = self.taggify(gma_files, w, h, total_size, taggify_channel);

				channel.progress_msg("FS_ANALYZER_SERIALIZING");

				channel.finish(treemap.squares);
			});

			Ok(id)

		}, resolve, reject);

		Ok(())
	}

	fn taggify(&self, gma_files: Vec<AnalyzedAddon>, w: f64, h: f64, total_size: u64, channel: TransactionChannel) -> TreeMap {
		use std::collections::hash_map::Entry::{Occupied, Vacant};

		let mut master_treemap = TreeMap::new(w, h);

		let total_files = gma_files.len() as f64;

		let mut tag_total_sizes: HashMap<String, f64> = HashMap::new();
		let mut tag_sizes: HashMap<String, Vec<f64>> = HashMap::new();
		let mut tag_data: HashMap<String, Vec<TaggedTreeMapData>> = HashMap::new();
		for (i, gma_file) in gma_files.into_iter().enumerate() {
			let tag = gma_file.gma.metadata.as_ref().unwrap()
				.addon_type.clone()
				.or_else(|| {
					if let Some(tags) = &gma_file.gma.metadata.as_ref().unwrap().tags {
						if !tags.is_empty() { return Some(tags.get(1).cloned().unwrap()) }
					}
					None
				})
				.unwrap_or("addon".to_string())
			.to_lowercase();
			
			match tag_data.entry(tag.clone()) {
			    Occupied(mut o) => {
					let total_tag_sizes = tag_total_sizes.get_mut(&tag).unwrap();
					*total_tag_sizes = *total_tag_sizes + (gma_file.gma.size as f64);

					tag_sizes.get_mut(&tag).unwrap().push(gma_file.gma.size as f64);

					let gma_files = o.get_mut();
					gma_files.push(Some((Box::new(gma_file), None)));
				},
			    Vacant(v) => {
					tag_total_sizes.insert(tag.clone(), gma_file.gma.size as f64);
					
					tag_sizes.insert(tag, vec![gma_file.gma.size as f64]);

					v.insert(vec![Some((Box::new(gma_file), None))]);
				}
			}

			channel.progress(0.5 + (((i as f64) / total_files) / 4.));
		}

		for tag in tag_total_sizes.keys() {
			master_treemap.data.push(Some((Box::new(()), Some(tag.clone()))));
		}

		master_treemap.process(tag_total_sizes.values().cloned().collect(), total_size as f64);

		/*
		let mut treemap_threads: Vec<JoinHandle<()>> = Vec::new();
		let total_squares = master_treemap.squares.len() as f64; // TODO is this = to something?
		for (i, square) in master_treemap.squares.iter_mut().enumerate() {
			let (_, tag) = square.data.as_ref().unwrap();
			let tag = tag.as_ref().unwrap().clone();

			let tag_total_size = tag_total_sizes.remove(&tag).unwrap();
			let tag_sizes = tag_sizes.remove(&tag).unwrap();
			let tag_data = tag_data.remove(&tag).unwrap();

			treemap_threads.push(std::thread::spawn(move || {
				let mut treemap = TreeMap::new(square.w, square.h);

				treemap.data = tag_data;
				treemap.process(tag_sizes, tag_total_size);
	
				square.data = Some((Box::new(treemap.squares), Some(tag.clone())));
	
				//channel.progress(0.75 + (((i as f64) / total_squares) / 4.));
			}));
		}

		for thread in treemap_threads { thread.join().unwrap(); }
		*/

		master_treemap
	}

	pub(super) fn analyze(&'static self, channel: TransactionChannel) -> Option<JoinHandle<(Vec<AnalyzedAddon>, Vec<Option<PublishedFileId>>, u64)>> {
		if !self.analyzed.lock().unwrap().0.is_empty() { return None; }

		game_addons::cache_addon_paths();

		let game_addons = crate::GAME_ADDONS.read().unwrap();
		let paths = &game_addons.gma_cache.as_ref().unwrap().installed_gmas;
		let total_paths = paths.len();

		if total_paths == 0 {
			channel.error("ERROR_NO_ADDONS_FOUND", ());
			None
		} else {
			let thread_count = (num_cpus::get() - 1).min(total_paths);
			let chunk_len = ((total_paths as f64) / (thread_count as f64).floor()) as usize;
			let paths = paths.iter().cloned().collect::<Vec<(PathBuf, Option<PublishedFileId>)>>().chunks(chunk_len).map(|c| c.to_owned()).collect::<Vec<Vec<(PathBuf, Option<PublishedFileId>)>>>();

			let (tx, rx) = mpsc::channel();

			for chunk in paths {
				let tx = tx.clone();
				std::thread::spawn(move || {
					for (path, id) in chunk {
						let cached_gma = crate::GAME_ADDONS.read().unwrap()
							.gma_cache.as_ref().unwrap()
							.metadata.read().unwrap()
							.get(&path)
							.cloned();
						
						if tx.send({
							
							let mut gma = match cached_gma {

								Some(cached_gma) => {
									if cached_gma.size == 0 { continue }
									cached_gma
								},
								None => match GMAFile::new(&path, None) {
									Ok(mut gma) => {
										if gma.metadata().is_err() { continue }
										if gma.size == 0 { continue }
										gma
									},
									Err(_) => continue
								}

							};
							
							if id.is_some() && gma.id.is_none() {
								gma.id = id;
							}
							gma

						}).is_err() { break; }
					}
				});
			}

			Some(std::thread::spawn(move || {
				let mut total_size: u64 = 0;
				let mut sorted_gmas: Vec<AnalyzedAddon> = Vec::with_capacity(total_paths);
				let mut sorted_ids: Vec<Option<PublishedFileId>> = Vec::with_capacity(total_paths);
				
				let mut i = 0;
				while i < total_paths {
					i = i + 1;

					match rx.recv() {
						Ok(gma) => {
							total_size = total_size + gma.size;

							let pos = match sorted_gmas.binary_search_by(|gma_cmp| gma.size.cmp(&gma_cmp.gma.size)) {
								Ok(pos) => pos,
								Err(pos) => pos
							};

							sorted_ids.insert(pos, gma.id.clone());
							sorted_gmas.insert(pos, gma.into());

							if channel.aborted() { break; }
							channel.progress(((i as f64) / (total_paths as f64)) / 4.);
						},
						Err(_) => break
					}
				}
				
				(sorted_gmas, sorted_ids, total_size)
			}))
		}
	}

	pub(crate) fn download_steam(&self, ws_channel: TransactionChannel, mut gma_files: Vec<AnalyzedAddon>, gma_ids: Vec<Option<PublishedFileId>>, total_size: u64) -> (Vec<AnalyzedAddon>, u64) {
		let workshop = crate::WORKSHOP.write().unwrap();
		let id_chunks: Vec<&[Option<PublishedFileId>]> = gma_ids.chunks(workshop::kNumUGCResultsPerPage as usize).collect();
		let chunks_num = id_chunks.len() as f64;

		let (tx, rx): (SyncSender<Vec<WorkshopItem>>, Receiver<Vec<WorkshopItem>>) = mpsc::sync_channel(chunks_num as usize);

		let gma_files = std::thread::spawn(move || {
			let mut gma_chunks = gma_files.chunks_mut(workshop::kNumUGCResultsPerPage as usize);
			let mut progress: u32 = 0;
			loop {
				let workshop_data = match rx.recv() {
					Ok(results) => results,
					Err(_) => break
				};

				progress = progress + 1;
				ws_channel.progress(0.25 + (((progress as f64) / chunks_num) / 4.));
				
				let mut gma_chunk: Vec<&mut AnalyzedAddon> = gma_chunks.next().unwrap().into_iter().collect();
				let mut i = 0;
				for item in workshop_data {
					let mut proceeding_checked = false;
					loop {
						let mut gma = gma_chunk.get_mut(i).unwrap();
						if let Some(id) = gma.gma.id {
							if id == item.id {

								gma.preview_url = item.preview_url;

								#[cfg(debug_assertions)]
								if proceeding_checked {
									println!("Resolved");
								}

								i = i + 1;

								break;

							} else {
								#[cfg(debug_assertions)]
								{
									println!("Id Mismatch: {:?} / {:?}", id, item.id);
									/*
									Id Mismatch: PublishedFileId(504095596) / PublishedFileId(284266415)
									Id Mismatch: PublishedFileId(284266415) / PublishedFileId(1649131424)
									Id Mismatch: PublishedFileId(526098847) / PublishedFileId(714720335)
									Id Mismatch: PublishedFileId(714720335) / PublishedFileId(349281554)
									Id Mismatch: PublishedFileId(349281554) / PublishedFileId(1921113973)
									Id Mismatch: PublishedFileId(1921113973) / PublishedFileId(328735857)
									*/
								}
							}
						} else {
							#[cfg(debug_assertions)]
							{
								println!("Missing id");
								println!("{:#?}", gma);
							}
						}

						if proceeding_checked {

							#[cfg(debug_assertions)]
							println!("Discarded");

							break;

						} else {
							i = i + 1;
							proceeding_checked = true;
						}
					}
				}
			}
			gma_files
		});
		
		{
			let tx = tx;
			for chunk in id_chunks {
				let (_, data) = match workshop.get_items(chunk.into_iter().filter_map(|id| *id).collect()).unwrap() {
					Ok(data) => data,
					Err(_) => break,
				};

				if tx.send(data).is_err() { break; }
			}
		}

		let gma_files = gma_files.join().unwrap();

		let mut gma_files_mv = self.analyzed.lock().unwrap();
		*gma_files_mv = (gma_files.clone(), total_size);

		(gma_files, total_size)
	}
}
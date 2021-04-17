use std::{
	collections::BinaryHeap,
	sync::Arc,
};

use indexmap::IndexMap;
use lazy_static::lazy_static;
use rayon::{ThreadPool, ThreadPoolBuilder};
use steamworks::PublishedFileId;

use serde::Serialize;

use crate::{game_addons, transaction, transactions::Transaction, webview::Addon};

lazy_static! {
	static ref THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().num_threads(4).build().unwrap();
	static ref ANALYZER_THREAD_POOL: ThreadPool = ThreadPoolBuilder::new().build().unwrap();
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq, derive_more::Deref)]
struct AnalyzedAddon(Arc<Addon>);
impl PartialOrd for AnalyzedAddon {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.0.installed().size.partial_cmp(&other.0.installed().size).map(|x| x.reverse())
	}
}
impl Ord for AnalyzedAddon {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.0.installed().id.cmp(&other.0.installed().id).reverse()
	}
}

// https://www.win.tue.nl/~vanwijk/stm.pdf
use treemap::{TaggedTreeMapData, TreeMap};
mod treemap {
	use serde::Serialize;
	pub(super) type TaggedTreeMapData = Option<(Option<Box<dyn erased_serde::Serialize + Sync + Send + 'static>>, Option<String>)>;

	#[derive(Serialize)]
	pub(super) struct Square {
		pub(super) data: TaggedTreeMapData,
		pub(super) x: f64,
		pub(super) y: f64,
		pub(super) w: f64,
		pub(super) h: f64,
	}

	pub(super) struct TreeMap {
		pub(super) squares: Vec<Square>,
		pub(super) data: Vec<TaggedTreeMapData>,
		pub(super) x: f64,
		pub(super) y: f64,
		pub(super) w: f64,
		pub(super) h: f64,
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
			if data_sizes.is_empty() {
				return;
			}

			let mut scaled: Vec<f64> = data_sizes.into_iter().map(|x| (x * self.h * self.w) / total_size).collect();
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

			f64::max((ww * max) / sumsum, sumsum / (ww * min))
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

pub struct AddonSizeAnalyzer;
impl AddonSizeAnalyzer {
	pub fn init() -> Self {
		Self {}
	}

	pub fn compute(&'static self, w: f64, h: f64) -> Transaction {
		let transaction = transaction!();
		let transaction_ref = transaction.clone();

		THREAD_POOL.spawn(move || {
			transaction.status("FS_ANALYZER_DISCOVERING");

			let addons = game_addons!().get_addons().clone();
			if addons.is_empty() {
				transaction.error("ERR_NO_ADDONS_FOUND");
				return;
			}

			transaction.status("FS_ANALYZER_COMPUTING");

			let (addons, total_size) = self.count(addons, &transaction);

			if transaction.aborted() {
				return;
			}

			transaction.status("FS_ANALYZER_TAGGIFYING");

			let treemap = self.taggify(addons, w, h, total_size, &transaction);

			if transaction.aborted() {
				return;
			}

			transaction.status("FS_ANALYZER_SERIALIZING");
			transaction.progress(1.);

			transaction.finished(Some(treemap.squares));
		});

		transaction_ref
	}

	fn count(&'static self, addons: Vec<Arc<Addon>>, transaction: &Transaction) -> (Vec<AnalyzedAddon>, u64) {
		let mut ids: Vec<PublishedFileId> = Vec::with_capacity(addons.len());
		let mut sorted_addons = BinaryHeap::with_capacity(addons.len());

		let mut total_size = 0;
		let total = addons.len() as f64;
		for (i, gma) in addons.into_iter().enumerate() {
			total_size += gma.installed().size;

			if let Some(ref id) = gma.installed().id {
				ids.push(*id);
			}

			sorted_addons.push(AnalyzedAddon(gma));
			transaction.progress(((i as f64) / total) / 3.);
		}

		steam!().fetch_workshop_items(ids);

		(sorted_addons.into_sorted_vec(), total_size)
	}

	fn taggify(&self, gma_files: Vec<AnalyzedAddon>, w: f64, h: f64, total_size: u64, transaction: &Transaction) -> TreeMap {
		use indexmap::map::Entry::{Occupied, Vacant};

		let mut master_treemap = TreeMap::new(w, h);

		let total_files = gma_files.len() as f64;

		let mut tag_total_sizes: IndexMap<String, f64> = IndexMap::new();
		let mut tag_sizes: IndexMap<String, Vec<f64>> = IndexMap::new();
		let mut tag_data: IndexMap<String, Vec<TaggedTreeMapData>> = IndexMap::new();
		for (i, gma) in gma_files.into_iter().enumerate() {
			let metadata = gma.installed().metadata.as_ref().unwrap();
			let tag = metadata
				.addon_type()
				.or_else(|| {
					if let Some(tags) = metadata.tags() {
						if !tags.is_empty() {
							return Some(tags.get(1).unwrap());
						}
					}
					None
				})
				.unwrap_or("addon")
				.to_lowercase();

			match tag_data.entry(tag.clone()) {
				Occupied(mut o) => {
					let total_tag_sizes = tag_total_sizes.get_mut(&tag).unwrap();
					*total_tag_sizes = *total_tag_sizes + (gma.installed().size as f64);

					tag_sizes.get_mut(&tag).unwrap().push(gma.installed().size as f64);

					let gma_files = o.get_mut();
					gma_files.push(Some((Some(Box::new(gma)), None)));
				}
				Vacant(v) => {
					tag_total_sizes.insert(tag.clone(), gma.installed().size as f64);

					tag_sizes.insert(tag, vec![gma.installed().size as f64]);

					v.insert(vec![Some((Some(Box::new(gma)), None))]);
				}
			}

			transaction.progress(0.33 + (((i as f64) / total_files) / 3.));
		}

		for tag in tag_total_sizes.keys() {
			master_treemap.data.push(Some((None, Some(tag.clone()))));
		}

		master_treemap.process(tag_total_sizes.values().cloned().collect(), total_size as f64);

		rayon::scope(|scope| {
			let total_squares_i = master_treemap.squares.len(); // TODO is this = to something?
			let total_squares_f = total_squares_i as f64; // TODO is this = to something?

			for (i, square) in master_treemap.squares.chunks_exact_mut(1).enumerate() {
				if transaction.aborted() {
					break;
				}

				let square = square.get_mut(0).unwrap();

				let (_, tag) = square.data.as_ref().unwrap();
				let tag = tag.as_ref().unwrap().clone();

				let tag_total_size = tag_total_sizes.remove(&tag).unwrap();
				let tag_sizes = tag_sizes.remove(&tag).unwrap();
				let tag_data = tag_data.remove(&tag).unwrap();

				let transaction = transaction.clone();
				scope.spawn(move |_| {
					if transaction.aborted() {
						return;
					}

					let padding = (f64::min(square.w, square.h) * 0.05).ceil();
					let mut treemap = TreeMap::new(square.w.floor() - padding, square.h.floor() - padding);

					treemap.data = tag_data;
					treemap.process(tag_sizes, tag_total_size);

					square.data = Some((Some(Box::new(treemap.squares)), Some(tag.clone())));

					transaction.progress(0.66 + (((i as f64) / total_squares_f) / 3.));
				});
			}
		});

		master_treemap
	}
}

#[tauri::command]
fn addon_size_analyzer(w: f64, h: f64) -> Transaction {
	crate::ADDON_SIZE_ANALYZER.compute(w, h)
}

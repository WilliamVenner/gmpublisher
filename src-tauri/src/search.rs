use std::{
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, AtomicU32, AtomicU8},
		Arc,
	},
};

use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{ser::SerializeTuple, Serialize};
use steamworks::PublishedFileId;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::{GMAFile, Transaction, WorkshopItem};

const MAX_QUICK_RESULTS: u8 = 10;

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "source", content = "association")]
pub enum SearchItemSource {
	InstalledAddons(PathBuf, Option<PublishedFileId>),
	MyWorkshop(PublishedFileId),
	WorkshopItem(PublishedFileId),
}

#[derive(Debug)]
pub struct SearchItem {
	pub label: String,
	pub terms: Vec<String>,
	pub timestamp: u64,
	pub len: usize,
	pub source: SearchItemSource,
}
impl PartialOrd for SearchItem {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		let cmp1 = self.timestamp.partial_cmp(&other.timestamp).map(|ord| ord.reverse());
		let cmp2 = self.len.partial_cmp(&other.len).map(|ord| ord.reverse());
		cmp1.partial_cmp(&cmp2)
	}
}
impl Ord for SearchItem {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		let cmp1 = self.timestamp.cmp(&other.timestamp).reverse();
		let cmp2 = self.len.cmp(&other.len).reverse();
		cmp1.cmp(&cmp2)
	}
}
impl PartialEq for SearchItem {
	fn eq(&self, other: &Self) -> bool {
		match &self.source {
			SearchItemSource::InstalledAddons(a, _) => match &other.source {
				SearchItemSource::InstalledAddons(b, _) => a == b,
				_ => false,
			},
			SearchItemSource::MyWorkshop(a) => match &other.source {
				SearchItemSource::MyWorkshop(b) => a == b,
				_ => false,
			},
			_ => unreachable!(),
		}
	}
}
impl Eq for SearchItem {}
impl SearchItem {
	pub fn new<D: Into<u64>>(source: SearchItemSource, label: String, mut terms: Vec<String>, timestamp: D) -> SearchItem {
		terms.shrink_to_fit();
		terms.sort_by(|a, b| a.len().cmp(&b.len()));

		SearchItem {
			len: terms.iter().map(|x| x.len()).reduce(|a, b| a.max(b)).unwrap_or(0).max(label.len()),
			label,
			terms,
			timestamp: timestamp.into(),
			source,
		}
	}
}
impl Serialize for SearchItem {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let mut tup = serializer.serialize_tuple(2)?;
		tup.serialize_element(&self.label)?;
		tup.serialize_element(&self.source)?;
		tup.end()
	}
}

pub trait Searchable {
	fn search_item(&self) -> Option<SearchItem>;
}
impl Searchable for WorkshopItem {
	fn search_item(&self) -> Option<SearchItem> {
		let mut terms = self.tags.clone();

		if let Some(steamid) = &self.steamid {
			terms.push(steamid.raw().to_string());
			terms.push(steamid.steamid32());
		}

		Some(SearchItem::new(
			SearchItemSource::MyWorkshop(self.id),
			self.title.to_owned(),
			terms,
			self.time_updated,
		))
	}
}
impl Searchable for GMAFile {
	fn search_item(&self) -> Option<SearchItem> {
		let (label, terms) = match &self.metadata {
			Some(metadata) => {
				let mut terms = metadata.tags().cloned().unwrap_or_default();
				if let Some(addon_type) = metadata.addon_type() {
					terms.push(addon_type.to_string());
				}
				(metadata.title().to_owned(), terms)
			}
			None => {
				if !self.extracted_name.is_empty() {
					(self.extracted_name.to_owned(), vec![])
				} else {
					return None;
				}
			}
		};

		Some(SearchItem::new(
			SearchItemSource::InstalledAddons(
				dunce::canonicalize(&self.path).unwrap_or_else(|_| self.path.to_owned()),
				self.id.to_owned(),
			),
			label,
			terms,
			self.modified.unwrap_or(0),
		))
	}
}
impl Searchable for std::sync::Arc<crate::webview::Addon> {
	fn search_item(&self) -> Option<SearchItem> {
		match &**self {
			crate::webview::Addon::Installed(installed) => installed.search_item(),
			crate::webview::Addon::Workshop(workshop) => workshop.search_item(),
		}
	}
}

#[derive(Clone, Copy)]
struct ResultsPtr(*mut Vec<Option<(i64, Arc<SearchItem>)>>);
unsafe impl Send for ResultsPtr {}
unsafe impl Sync for ResultsPtr {}

pub struct Search {
	channel: Transaction,
	dirty: AtomicBool,
	items: RwLock<Vec<Arc<SearchItem>>>,
	matcher: SkimMatcherV2,

	pub installed_addons: RwLock<Vec<Arc<SearchItem>>>,
}
impl Search {
	pub fn init() -> Search {
		Self {
			channel: transaction!(),
			items: RwLock::new(Vec::new()),
			matcher: SkimMatcherV2::default().ignore_case().use_cache(true),
			dirty: AtomicBool::new(false),

			installed_addons: RwLock::new(Vec::new()),
		}
	}

	pub fn dirty(&self) {
		if !self.dirty.load(std::sync::atomic::Ordering::Acquire) {
			return;
		}

		{
			let mut items = self.items.write();
			items.par_sort();
			items.dedup();
		}

		{
			let mut installed_addons = self.installed_addons.write();
			installed_addons.par_sort_by(|a, b| match &a.source {
				SearchItemSource::InstalledAddons(_, a_id) => match &b.source {
					SearchItemSource::InstalledAddons(_, b_id) => a_id.cmp(b_id),
					_ => unreachable!(),
				},
				_ => unreachable!(),
			});
			installed_addons.dedup_by(|a, b| match &a.source {
				SearchItemSource::InstalledAddons(_, a_id) => match &b.source {
					SearchItemSource::InstalledAddons(_, b_id) => a_id.eq(b_id),
					_ => unreachable!(),
				},
				_ => unreachable!(),
			});
		}
	}

	pub fn add<V: Searchable>(&self, item: &V) {
		if let Some(search_item) = item.search_item() {
			let search_item = Arc::new(search_item);

			if self.dirty.load(std::sync::atomic::Ordering::Acquire) {
				if let SearchItemSource::InstalledAddons(_, id) = &search_item.source {
					if id.is_some() {
						self.installed_addons.write().push(search_item.clone());
					}
				}

				self.items.write().push(search_item);
			} else {
				if let SearchItemSource::InstalledAddons(_, id) = &search_item.source {
					if let Some(id) = id {
						let mut installed_addons = self.installed_addons.write();
						match installed_addons.binary_search_by(|cmp| match &cmp.source {
							SearchItemSource::InstalledAddons(_, cmp_id) => cmp_id.as_ref().unwrap().cmp(id),
							_ => unreachable!(),
						}) {
							Ok(pos) => {
								installed_addons[pos] = search_item.clone();
							}
							Err(pos) => {
								installed_addons.insert(pos, search_item.clone());
							}
						}
					}
				}

				let mut items = self.items.write();
				match items.binary_search(&search_item) {
					Ok(pos) => {
						items[pos] = search_item;
					}
					Err(pos) => {
						items.insert(pos, search_item);
					}
				}
			}
		}
	}

	pub fn reserve(&self, amount: usize) {
		self.items.write().reserve(amount);
	}

	pub fn add_bulk<V: Searchable>(&self, items: &Vec<V>) {
		self.dirty.store(true, std::sync::atomic::Ordering::Release);

		let mut installed_addons = once_cell::unsync::OnceCell::new();

		let mut store = self.items.write();
		store.reserve(items.len());
		store.extend(items.into_iter().filter_map(|v| {
			v.search_item().map(|search_item| {
				let search_item = Arc::new(search_item);
				if let SearchItemSource::InstalledAddons(_, id) = &search_item.source {
					if id.is_some() {
						installed_addons.get_or_init(|| self.installed_addons.write());
						installed_addons.get_mut().unwrap().push(search_item.clone());
					}
				}
				search_item
			})
		}));
	}

	pub fn quick(&self, query: String) -> (Vec<Arc<SearchItem>>, bool) {
		game_addons!().discover_addons();
		//steam!().discover_my_workshop_addons();
		self.dirty();

		let i = AtomicU8::new(0);
		let has_more = AtomicBool::new(false);
		let mut results: Vec<Option<(i64, Arc<SearchItem>)>> = vec![None; MAX_QUICK_RESULTS as usize];

		self.items
			.read()
			.par_iter()
			.try_for_each_with(ResultsPtr(&mut results as *mut _), |results, search_item| {
				if i.load(std::sync::atomic::Ordering::Acquire) >= MAX_QUICK_RESULTS {
					has_more.store(true, std::sync::atomic::Ordering::Release);
					return Err(());
				}

				if search_item.len < query.len() {
					return Ok(());
				}

				let mut winner = None;

				if search_item.label.len() >= query.len() {
					if let Some(score) = self.matcher.fuzzy_match(&search_item.label, &query) {
						winner = Some(score);
					}
				}

				for term in search_item.terms.iter() {
					if term.len() < query.len() {
						continue;
					}
					if let Some(score) = self.matcher.fuzzy_match(term, &query) {
						if winner.is_none() || winner.unwrap() < score {
							winner = Some(score);
						}
					}
				}

				if let Some(score) = winner {
					let i = i.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
					if i >= MAX_QUICK_RESULTS {
						has_more.store(true, std::sync::atomic::Ordering::Release);
						return Err(());
					} else {
						(unsafe { &mut *results.0 })[i as usize] = Some((score, search_item.clone()));
					}
				}

				Ok(())
			})
			.ok();

		let i = i.into_inner();

		if i == 0 {
			(vec![], false)
		} else if i == 1 {
			(vec![results[0].take().unwrap().1], false)
		} else {
			let has_more = has_more.load(std::sync::atomic::Ordering::Acquire);

			results.sort_by(|a, b| {
				if let Some(a) = a {
					if let Some(b) = b {
						return a.0.cmp(&b.0).reverse();
					} else {
						return std::cmp::Ordering::Less;
					}
				} else if b.is_some() {
					return std::cmp::Ordering::Greater;
				}
				return std::cmp::Ordering::Equal;
			});

			(results.into_iter().filter_map(|x| x.map(|x| x.1)).collect(), has_more)
		}
	}

	pub fn full(&'static self, query: String) -> u32 {
		game_addons!().discover_addons();
		//steam!().discover_my_workshop_addons();
		self.dirty();

		let transaction = transaction!();
		let id = transaction.id;

		rayon::spawn(move || {
			let items = self.items.read();
			let items_n_f = items.len() as f64;
			let i = Arc::new(AtomicU32::new(0));

			items
				.par_iter()
				.try_for_each_with(i, |i, search_item| {
					if transaction.aborted() {
						return Err(());
					} else {
						transaction.progress(i.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as f64 / items_n_f);
					}

					if search_item.len < query.len() {
						return Ok(());
					}

					let mut winner = None;

					if search_item.label.len() >= query.len() {
						if let Some(score) = self.matcher.fuzzy_match(&search_item.label, &query) {
							winner = Some(score);
						}
					}

					for term in search_item.terms.iter() {
						if term.len() < query.len() {
							continue;
						}
						if let Some(score) = self.matcher.fuzzy_match(term, &query) {
							if winner.is_none() || winner.unwrap() < score {
								winner = Some(score);
							}
						}
					}

					if let Some(score) = winner {
						transaction.data((score, search_item.clone()));
					}

					Ok(())
				})
				.unwrap();

			transaction.finished(turbonone!());
		});

		id
	}

	pub fn clear(&self) {
		*self.items.write() = Vec::new();
	}
}

#[tauri::command]
fn search(salt: u32, query: String) {
	search!().channel.data((salt, search!().quick(query)));
}

#[tauri::command]
fn full_search(query: String) -> u32 {
	search!().full(query)
}

#[tauri::command]
fn search_channel() -> u32 {
	search!().channel.id
}

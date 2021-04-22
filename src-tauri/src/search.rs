use std::{cell::UnsafeCell, cmp::Ordering, path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicU8}}};

use rayon::prelude::*;
use steamworks::PublishedFileId;
use serde::{Serialize, ser::SerializeTuple};
use parking_lot::RwLock;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::{GMAFile, Transaction, WorkshopItem};

#[derive(Clone, Serialize, Debug)]
pub enum SearchAssociation {
	PublishedFileId(PublishedFileId),
	Path(PathBuf)
}
impl PartialOrd for SearchAssociation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		match self {
			SearchAssociation::PublishedFileId(a) => match other {
				SearchAssociation::PublishedFileId(b) => a.partial_cmp(b),
				_ => None
			},
			SearchAssociation::Path(a) => match other {
				SearchAssociation::Path(b) => a.partial_cmp(b),
				_ => None
			},
		}
    }
}
impl Ord for SearchAssociation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		match self {
			SearchAssociation::PublishedFileId(a) => match other {
				SearchAssociation::PublishedFileId(b) => a.cmp(b),
				_ => Ordering::Less
			},
			SearchAssociation::Path(a) => match other {
				SearchAssociation::Path(b) => a.cmp(b),
				_ => Ordering::Less
			},
		}
    }
}
impl PartialEq for SearchAssociation {
    fn eq(&self, other: &Self) -> bool {
		match self {
			SearchAssociation::PublishedFileId(a) => match other {
				SearchAssociation::PublishedFileId(b) => a == b,
				_ => false
			},
			SearchAssociation::Path(a) => match other {
				SearchAssociation::Path(b) => a == b,
				_ => false
			},
		}
    }
}
impl Eq for SearchAssociation {}

#[derive(Debug)]
pub struct SearchItem {
	label: String,
	terms: Vec<String>,
	associations: Vec<SearchAssociation>,
	timestamp: u64,
	len: usize
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
		if self.associations.len() == other.associations.len() {
			if self.associations.len() == 1 {
				self.associations[0] == other.associations[1]
			} else {
				let mut eq = true;
				let mut i = 0;
				while i < self.associations.len() {
					if self.associations[i] != other.associations[i] {
						eq = false;
					}
					i += 1;
				}
				eq
			}
		} else {
			false
		}
    }
}
impl Eq for SearchItem {}
impl SearchItem {
	pub fn new<D: Into<u64>>(label: String, mut terms: Vec<String>, mut associations: Vec<SearchAssociation>, timestamp: D) -> SearchItem {
		terms.shrink_to_fit();

		associations.shrink_to_fit();
		assert!(!associations.is_empty());
		associations.sort();

		terms.sort_by(|a, b| a.len().cmp(&b.len()));

		SearchItem {
			len: terms.iter().map(|x| x.len()).reduce(|a, b| a.max(b)).unwrap_or(0).max(label.len()),
			label,
			terms,
			associations,
			timestamp: timestamp.into(),
		}
	}
}
impl Serialize for SearchItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
	{
        let mut tup = serializer.serialize_tuple(2)?;
		tup.serialize_element(&self.label)?;
		tup.serialize_element(&self.associations)?;
		tup.end()
    }
}

pub trait Searchable {
	fn search_item(&self) -> Option<SearchItem>;
}
impl Searchable for WorkshopItem {
	fn search_item(&self) -> Option<SearchItem> {
		let mut terms = self.tags.clone();

		if let Some(steamid64) = &self.steamid64 {
			terms.push(steamid64.to_owned());
			// TODO add SteamID32
		}

		Some(SearchItem::new(
			self.title.to_owned(),
			terms,
			vec![SearchAssociation::PublishedFileId(self.id)],
			self.time_updated
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
			},
		    None => {
				if !self.extracted_name.is_empty() {
					(self.extracted_name.to_owned(), vec![])
				} else {
					return None;
				}
			}
		};

		let mut associations = vec![SearchAssociation::Path(self.path.to_owned())];
		if let Some(id) = self.id {
			associations.push(SearchAssociation::PublishedFileId(id));
		}

		Some(SearchItem::new(
			label,
			terms,
			associations,
			self.modified.and_then(|x| x.duration_since(std::time::SystemTime::UNIX_EPOCH).ok().map(|dur| dur.as_secs())).unwrap_or(0)
		))
	}
}
impl Searchable for std::sync::Arc<crate::webview::Addon> {
    fn search_item(&self) -> Option<SearchItem> {
        match &**self {
            crate::webview::Addon::Installed(installed) => installed.search_item(),
            crate::webview::Addon::Workshop(workshop) => workshop.search_item()
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
	items: RwLock<Vec<Arc<SearchItem>>>, // TODO store Arc<SearchItem> for less cloning overhead
	matcher: SkimMatcherV2,
}
impl Search {
	pub fn init() -> Search {
		Self {
			channel: transaction!(),
			items: RwLock::new(Vec::new()),
			matcher: SkimMatcherV2::default().ignore_case().use_cache(true),
			dirty: AtomicBool::new(false)
		}
	}

	pub fn dirty(&self) {
		if !self.dirty.load(std::sync::atomic::Ordering::Acquire) { return; }
		let mut items = self.items.write();
		items.sort();
		items.dedup();
	}

	pub fn add<V: Searchable>(&self, item: &V) {
		if let Some(search_item) = item.search_item() {
			let search_item = Arc::new(search_item);
			let mut items = self.items.write();
			let pos = match items.binary_search(&search_item) {
				Ok(_) => return,
				Err(pos) => pos
			};
			items.insert(pos, search_item);
		}
	}

	pub fn add_bulk<V: Searchable>(&self, items: &Vec<V>) {
		self.dirty.store(true, std::sync::atomic::Ordering::Release);

		let mut store = self.items.write();
		store.reserve(items.len());
		store.extend(items.into_iter().filter_map(|v| v.search_item().map(|search_item| Arc::new(search_item))));
	}

	pub fn quick(&self, query: String) -> Vec<Arc<SearchItem>> {
		game_addons!().discover_addons();

		let i = AtomicU8::new(0);
		let mut results: Vec<Option<(i64, Arc<SearchItem>)>> = vec![None; 15];

		self.items.read().par_iter().try_for_each_with(ResultsPtr(&mut results as *mut _), |results, search_item| {
			if search_item.len < query.len() { return Ok(()); }

			let mut winner = None;

			if search_item.label.len() >= query.len() {
				if let Some(score) = self.matcher.fuzzy_match(&search_item.label, &query) {
					winner = Some(score);
				}
			}

			for term in search_item.terms.iter() {
				if term.len() < query.len() { continue; }
				if let Some(score) = self.matcher.fuzzy_match(term, &query) {
					if winner.is_none() || winner.unwrap() < score {
						winner = Some(score);
					}
				}
			}

			if let Some(score) = winner {
				let i = i.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
				if i >= 15 {
					return Err(());
				} else {
					(unsafe { &mut *results.0 })[i as usize] = Some((score, search_item.clone()));
				}
			}

			Ok(())

		}).ok();

		let i = i.into_inner();

		if i == 0 {
			vec![]
		} else if i == 1 {
			vec![results[0].take().unwrap().1]
		} else {
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

			results.into_iter().filter_map(|x| x.map(|x| x.1)).collect()
		}
	}

	pub fn full(&'static self, query: String) -> u32 {
		game_addons!().discover_addons();

		let transaction = transaction!();
		let id = transaction.id;

		rayon::spawn(move || {
			self.items.read().par_iter().for_each(|search_item| {
				if search_item.len < query.len() { return; }

				let mut winner = None;

				if search_item.label.len() >= query.len() {
					if let Some(score) = self.matcher.fuzzy_match(&search_item.label, &query) {
						winner = Some(score);
					}
				}

				for term in search_item.terms.iter() {
					if term.len() < query.len() { continue; }
					if let Some(score) = self.matcher.fuzzy_match(term, &query) {
						if winner.is_none() || winner.unwrap() < score {
							winner = Some(score);
						}
					}
				}

				if let Some(score) = winner {
					transaction.data((score, search_item.clone()));
				}
			});

			transaction.finished(turbonone!());
		});

		id
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

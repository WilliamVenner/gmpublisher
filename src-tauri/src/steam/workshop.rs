use serde::Serialize;
use std::{cell::RefCell, collections::VecDeque, ops::DerefMut, path::PathBuf, sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	}};
use std::collections::{HashMap, HashSet};

use steamworks::{PublishedFileId, QueryResult, QueryResults, SteamError, SteamId};

use parking_lot::Mutex;

use super::{users::SteamUser, Steam};

use crate::{GMOD_APP_ID, main_thread_forbidden, webview::Addon, webview_emit};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItem {
	pub id: PublishedFileId,
	pub title: String,
	pub owner: Option<SteamUser>,
	pub time_created: u32,
	pub time_updated: u32,
	pub description: Option<String>,
	pub score: f32,
	pub tags: Vec<String>,
	pub preview_url: Option<String>,
	pub subscriptions: u64,
	pub local_file: Option<PathBuf>,
	pub search_title: String,

	#[serde(skip)]
	pub steamid: Option<SteamId>,
	pub steamid64: Option<String>,

	pub dead: bool,
}
impl From<QueryResult> for WorkshopItem {
	fn from(result: QueryResult) -> Self {
		WorkshopItem {
			id: result.published_file_id,
			title: result.title.clone(),
			steamid: Some(result.owner),
			steamid64: Some(result.owner.raw().to_string()),
			owner: None,
			time_created: result.time_created,
			time_updated: result.time_updated,
			description: Some(result.description), // TODO parse or strip bbcode?
			score: result.score,
			tags: result.tags,
			preview_url: None,
			subscriptions: 0,
			local_file: None,
			search_title: result.title.to_lowercase(),

			dead: false,
		}
	}
}
impl From<PublishedFileId> for WorkshopItem {
	fn from(id: PublishedFileId) -> Self {
		WorkshopItem {
			id,
			title: id.0.to_string(),
			steamid: None,
			steamid64: None,
			owner: None,
			time_created: 0,
			time_updated: 0,
			description: None,
			score: 0.,
			tags: Vec::with_capacity(0),
			preview_url: None,
			subscriptions: 0,
			local_file: None,
			search_title: id.0.to_string(),
			dead: true,
		}
	}
}
impl PartialEq for WorkshopItem {
    fn eq(&self, other: &Self) -> bool {
		if self.time_created == 0 {
			if self.time_updated == 0 {
				self.id.eq(&other.id)
			} else {
				self.time_updated.eq(&other.time_updated)
			}
		} else {
			if other.time_created == 0 {
				self.id.eq(&other.id)
			} else {
				self.time_created.eq(&other.time_created)
			}
		}
    }
}
impl Eq for WorkshopItem {}
impl PartialOrd for WorkshopItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		if self.time_created == 0 {
			if self.time_updated == 0 {
				self.id.partial_cmp(&other.id)
			} else {
				self.time_updated.partial_cmp(&other.time_updated)
			}
		} else {
			if other.time_created == 0 {
				self.id.partial_cmp(&other.id)
			} else {
				self.time_created.partial_cmp(&other.time_created)
			}
		}
    }
}
impl Ord for WorkshopItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		if self.time_created == 0 {
			if self.time_updated == 0 {
				self.id.cmp(&other.id)
			} else {
				self.time_updated.cmp(&other.time_updated)
			}
		} else {
			if other.time_created == 0 {
				self.id.cmp(&other.id)
			} else {
				self.time_created.cmp(&other.time_created)
			}
		}
    }
}

#[derive(derive_more::Deref)]
struct FetcherBacklog(RefCell<VecDeque<PublishedFileId>>);
unsafe impl Sync for FetcherBacklog {}
lazy_static! {
	static ref FETCHER_BACKLOG: FetcherBacklog = FetcherBacklog(RefCell::new(VecDeque::new()));
	static ref FETCHER_NEXT: AtomicBool = AtomicBool::new(false);
}
impl Steam {
	pub fn workshop_fetcher() { loop {
		steam!().workshop.write(|workshop| {
			if workshop.1.is_empty() {
				FETCHER_NEXT.store(true, Ordering::Release);
				return;
			} else {
				FETCHER_NEXT.store(false, Ordering::Release);
			}

			let mut backlog = FETCHER_BACKLOG.borrow_mut();

			backlog.reserve(workshop.1.len());
			for id in workshop.1.drain(..).into_iter() { backlog.push_back(id); }

			drop(workshop);

			while !backlog.is_empty() {
				let backlog_len = backlog.len();
				let mut queue = backlog.split_off((steamworks::RESULTS_PER_PAGE as usize).min(backlog_len));
				std::mem::swap(&mut queue, &mut *backlog);

				let queue: Vec<PublishedFileId> = queue.into();

				let next = Arc::new(AtomicBool::new(false));
				let next_ref = next.clone();

				steam!().client().ugc().query_items(queue.to_owned()).unwrap().allow_cached_response(600).fetch(
					move |results: Result<QueryResults<'_>, SteamError>| {
						if let Ok(results) = results {
							let mut i = 0;
							for item in results.iter() {
								webview_emit!(
									"WorkshopItem",
									Addon::from(
										if let Some(item) = item {
											let mut item: WorkshopItem = item.into();
											item.preview_url = results.preview_url(i);
											item.subscriptions = results.statistic(i, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
											item
										} else {
											WorkshopItem::from(queue[i as usize])
										}
									)
								);
								i += 1;
							}
						} else {
							steam!().workshop.write(move |workshop| {
								for id in queue.into_iter() {
									workshop.0.remove(&id);
								}
							});
						}
						next_ref.store(true, Ordering::Release);
					},
				);

				while !next.load(Ordering::Acquire) {
					steam!().run_callbacks();
				}
			}

			FETCHER_NEXT.store(true, Ordering::Release);
		});

		while !FETCHER_NEXT.load(Ordering::Acquire) { sleep_ms!(50); }
	}}

	pub fn fetch_workshop_items(&'static self, ids: Vec<PublishedFileId>) {
		self.workshop.write(|workshop| {
			let (cache, queue) = workshop.deref_mut();
			queue.reserve(ids.len());
			for id in ids.into_iter().filter(|id| cache.insert(*id)) {
				queue.push(id);
			}
			queue.shrink_to_fit();
		});
	}

	/*
	// Workshop //

	pub fn fetch_workshop_item(&'static self, id: PublishedFileId) -> WorkshopItem {
		main_thread_forbidden!();

		let workshop = Arc::new(self.workshop.read());
		if let Some(item) = workshop.get(&id) {
			item.clone()
		} else {
			let item_response = Arc::new(AtomicRefCell::new(None));
			let response_received = Arc::new(AtomicBool::new(false));
			{
				let item_response = item_response.clone();
				let response_received = response_received.clone();
				self.client()
					.ugc()
					.query_item(id.clone())
					.unwrap()
					.fetch(move |query: Result<QueryResults<'_>, SteamError>| {
						if let Ok(results) = query {
							let item = if let Some(item) = results.get(0) {
								let mut item: WorkshopItem = item.into();
								item.preview_url = results.preview_url(0);
								item.subscriptions = results.statistic(0, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
								item
							} else {
								WorkshopItem::from(id)
							};

							*(*item_response).borrow_mut() = Some(item.clone());
							self.workshop.write(move |mut workshop| {
								workshop.insert(id, item);
							});
						}
						response_received.store(true, std::sync::atomic::Ordering::Release);
					});
			}
			while !response_received.load(std::sync::atomic::Ordering::Acquire) {
				self.run_callbacks();
			}

			Arc::try_unwrap(item_response).unwrap().into_inner().unwrap()
		}
	}

	pub fn fetch_workshop_items(&'static self, items: Vec<PublishedFileId>, include_cached: bool) -> Vec<WorkshopItem> {
		main_thread_forbidden!();
		debug_assert!(!items.is_empty());

		let items_response = Arc::new(AtomicRefCell::new(Vec::with_capacity(items.len())));
		let uncached: Vec<PublishedFileId> = {
			let workshop = Arc::new(self.workshop.read());
			let mut items_response = (*items_response).borrow_mut();

			items
				.into_iter()
				.filter_map(move |id| {
					if let Some(item) = workshop.get(&id) {
						if include_cached {
							items_response.push(match item.dead {
								false => item.clone(),
								true => WorkshopItem::from(id.to_owned()),
							});
						}
						None
					} else {
						Some(id)
					}
				})
				.collect()
		};

		if !uncached.is_empty() {
			let items_response = items_response.clone();
			let response_received = Arc::new(AtomicBool::new(false));
			{
				let response_received = response_received.clone();
				self.client()
					.ugc()
					.query_items(uncached.to_owned())
					.unwrap()
					.fetch(move |query: Result<QueryResults<'_>, SteamError>| {
						let mut items_response = (*items_response).borrow_mut();
						if let Ok(results) = query {
							self.workshop.begin();
							for (i, item) in results.iter().enumerate() {
								let id = uncached[i];
								let item = if let Some(item) = item {
									let mut item: WorkshopItem = item.into();
									item.preview_url = results.preview_url(i as u32);
									item.subscriptions = results.statistic(i as u32, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
									item
								} else {
									WorkshopItem::from(id)
								};

								items_response.push(item.clone());
								self.workshop.write(move |mut workshop| {
									workshop.insert(id, item);
								});
							}
							self.workshop.commit();
						}
						response_received.store(true, std::sync::atomic::Ordering::Release);
					});
			}
			while !response_received.load(std::sync::atomic::Ordering::Acquire) {
				self.run_callbacks();
			}
		}

		Arc::try_unwrap(items_response).unwrap().into_inner()
	}

	pub fn fetch_workshop_item_with_uploader(&'static self, id: PublishedFileId) -> WorkshopItem {
		let mut item = self.fetch_workshop_item(id);
		if !item.dead {
			if let None = item.owner {
				if let Some(steamid) = item.steamid {
					let owner = self.fetch_user(steamid);
					if !owner.dead {
						item.owner = Some(owner);
						let item = item.clone();
						steam!().workshop.write(move |mut workshop| {
							workshop.insert(id, item);
						});
					}
				}
			}
		}
		item
	}

	pub fn fetch_workshop_item_async<F>(&'static self, item: PublishedFileId, f: F)
	where
		F: FnOnce(&WorkshopItem) + 'static + Send,
	{
		match self.workshop.read().get(&item) {
			Some(item) => f(&item),
			None => {
				if self.workshop.task(item, f) {
					THREAD_POOL.spawn(move || {
						steam!().workshop.execute(&item, steam!().fetch_workshop_item(item));
					});
				}
			}
		}
	}

	pub fn fetch_workshop_items_async<F>(&'static self, items: Vec<PublishedFileId>, f: F, include_cached: bool)
	where
		F: FnOnce(Vec<WorkshopItem>) + 'static + Send,
	{
		THREAD_POOL.spawn(move || f(steam!().fetch_workshop_items(items, include_cached)));
	}

	pub fn fetch_workshop_item_with_uploader_async<F>(&'static self, item: PublishedFileId, f: F)
	where
		F: FnOnce(&WorkshopItem) + 'static + Send,
	{
		match self.workshop.read().get(&item) {
			Some(item) => {
				if let Some(_) = item.owner {
					f(&item.clone());
				}
			}
			None => {
				THREAD_POOL.spawn(move || {
					steam!()
						.workshop
						.execute(&item, steam!().fetch_workshop_item_with_uploader(item));
				});
			}
		}
	}
	*/

	// Collections //

	pub fn fetch_collection_items(&'static self, collection: PublishedFileId) -> Option<Vec<PublishedFileId>> {
		main_thread_forbidden!();

		let response = Arc::new(Mutex::new(None));
		{
			let response = response.clone();
			self.client()
				.ugc()
				.query_item(collection)
				.unwrap()
				.include_children(true)
				.fetch(move |query: Result<QueryResults<'_>, SteamError>| {
					if let Ok(results) = query {
						if let Some(result) = results.get(0) {
							if matches!(result.file_type, steamworks::FileType::Collection) {
								if let Some(children) = results.get_children(0) {
									*response.lock() = Some(Some(children));
									return;
								}
							}
						}
					}
					*response.lock() = Some(None);
				});
		}

		mutex_wait!(response, {
			self.run_callbacks();
		});

		Arc::try_unwrap(response).unwrap().into_inner().unwrap()
	}

	pub fn fetch_collection_items_async<F>(&'static self, collection: PublishedFileId, f: F)
	where
		F: FnOnce(&Option<Vec<PublishedFileId>>) + 'static + Send,
	{
		rayon::spawn(move || f(&self.fetch_collection_items(collection)));
	}

	pub fn browse_my_workshop(&'static self, page: u32) -> Option<(u32, Vec<Addon>)> {
		let results = Arc::new(Mutex::new(None));

		let results_ref = results.clone();
		let client = self.client(); client
		.ugc()
		.query_user(
			client.steam_id.account_id(),
			steamworks::UserList::Published,
			steamworks::UGCType::ItemsReadyToUse,
			steamworks::UserListOrder::LastUpdatedDesc,
			steamworks::AppIDs::ConsumerAppId(GMOD_APP_ID),
			page,
		)
		.ok()?
		.exclude_tag("dupe")
		.allow_cached_response(600)
		.fetch(move |result: Result<QueryResults<'_>, SteamError>| {
			if let Ok(data) = result {
				*results_ref.lock() = Some(Some((
					data.total_results(),
					data.iter()
						.enumerate()
						.map(|(i, x)| {
							let mut item: WorkshopItem = x.unwrap().into();
							item.preview_url = data.preview_url(i as u32);
							item.subscriptions = data
								.statistic(
									i as u32,
									steamworks::UGCStatisticType::Subscriptions,
								)
								.unwrap_or(0);
							item.into()
						})
						.collect::<Vec<Addon>>(),
				)));
			} else {
				*results_ref.lock() = Some(None);
			}
		});

		mutex_wait!(results, {
			self.run_callbacks();
		});

		Arc::try_unwrap(results).unwrap().into_inner().unwrap()
	}
}

#[tauri::command]
pub fn browse_my_workshop(page: u32) -> Option<(u32, Vec<Addon>)> {
	steam!().client_wait();
	rayon::scope(|_| steam!().browse_my_workshop(page))
}

pub fn free_caches() {
	*steam!().users.write_sync() = HashMap::new();
	*steam!().workshop.write_sync() = (HashSet::new(), Vec::new());
}

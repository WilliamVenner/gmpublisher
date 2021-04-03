use serde::Serialize;
use std::{
	path::PathBuf,
	sync::{atomic::AtomicBool, Arc},
};

use steamworks::{PublishedFileId, QueryResult, QueryResults, SteamError, SteamId};

use atomic_refcell::AtomicRefCell;

use super::{users::SteamUser, Steamworks, THREAD_POOL};

use crate::{main_thread_forbidden, steamworks};

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

impl Steamworks {
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

	pub fn fetch_workshop_items(&'static self, items: Vec<PublishedFileId>) -> Vec<WorkshopItem> {
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
						items_response.push(match item.dead {
							false => item.clone(),
							true => WorkshopItem::from(id.to_owned()),
						});
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
						steamworks!().workshop.write(move |mut workshop| {
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
						steamworks!().workshop.execute(&item, steamworks!().fetch_workshop_item(item));
					});
				}
			}
		}
	}

	pub fn fetch_workshop_items_async<F>(&'static self, items: Vec<PublishedFileId>, f: F)
	where
		F: FnOnce(Vec<WorkshopItem>) + 'static + Send,
	{
		THREAD_POOL.spawn(move || f(steamworks!().fetch_workshop_items(items)));
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
					steamworks!()
						.workshop
						.execute(&item, steamworks!().fetch_workshop_item_with_uploader(item));
				});
			}
		}
	}

	// Collections //

	pub fn fetch_collection_items(&'static self, collection: PublishedFileId) -> Option<Vec<PublishedFileId>> {
		main_thread_forbidden!();

		let response = Arc::new(AtomicRefCell::new(None));
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
									*response.borrow_mut() = Some(Some(children));
									return;
								}
							}
						}
					}
					*response.borrow_mut() = Some(None);
				});
		}

		loop {
			if let Ok(response) = response.try_borrow() {
				if response.is_some() {
					break;
				}
			}
			self.run_callbacks();
		}

		let children = Arc::try_unwrap(response).unwrap().into_inner().unwrap();

		if children.is_some() {
			let children = Some(children.clone().unwrap());
			self.collections.write(move |mut collections| {
				collections.insert(collection, children);
			});
		}

		children
	}

	pub fn fetch_collection_items_async<F>(&'static self, collection: PublishedFileId, f: F)
	where
		F: FnOnce(&Option<Vec<PublishedFileId>>) + 'static + Send,
	{
		match self.collections.read().get(&collection) {
			Some(cached) => f(cached),
			None => {
				if self.collections.task(collection, f) {
					THREAD_POOL.spawn(move || {
						steamworks!().collections.execute(&collection, self.fetch_collection_items(collection));
					});
				}
			}
		}
	}
}

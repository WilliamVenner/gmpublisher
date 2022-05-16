use serde::Serialize;
use std::{
	cell::RefCell,
	collections::{HashMap, HashSet, VecDeque},
	ops::DerefMut,
	path::PathBuf,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc,
	},
};

use steamworks::{PublishedFileId, QueryResult, QueryResults, SteamError, SteamId};

use parking_lot::Mutex;

use super::{users::SteamUser, Steam};

use crate::{main_thread_forbidden, webview::Addon, GMOD_APP_ID};

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
	//pub search_title: String,
	#[serde(serialize_with = "super::serialize_opt_steamid", rename = "steamid64")]
	pub steamid: Option<SteamId>,

	pub dead: bool,
}
impl From<QueryResult> for WorkshopItem {
	fn from(result: QueryResult) -> Self {
		WorkshopItem {
			id: result.published_file_id,
			title: result.title.clone(),
			steamid: Some(result.owner),
			owner: None,
			time_created: result.time_created,
			time_updated: result.time_updated,
			description: Some(result.description), // TODO parse or strip bbcode?
			score: result.score,
			tags: result.tags,
			preview_url: None,
			subscriptions: 0,
			local_file: None,
			//search_title: result.title.to_lowercase(),
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
			owner: None,
			time_created: 0,
			time_updated: 0,
			description: None,
			score: 0.,
			tags: Vec::with_capacity(0),
			preview_url: None,
			subscriptions: 0,
			local_file: None,
			//search_title: id.0.to_string(),
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
	pub fn workshop_fetcher() {
		loop {
			steam!().workshop.write(|workshop| {
				if workshop.1.is_empty() {
					FETCHER_NEXT.store(true, Ordering::Release);
					return;
				} else {
					FETCHER_NEXT.store(false, Ordering::Release);
				}

				let mut backlog = FETCHER_BACKLOG.borrow_mut();

				backlog.reserve(workshop.1.len());
				for data in std::mem::take(&mut workshop.1) {
					backlog.push_back(data);
				}

				while !backlog.is_empty() {
					let backlog_len = backlog.len();
					let mut queue = backlog.split_off((steamworks::RESULTS_PER_PAGE as usize).min(backlog_len));
					std::mem::swap(&mut queue, &mut *backlog);

					let queue: Vec<PublishedFileId> = queue.into();

					let next = Arc::new(AtomicBool::new(false));
					let next_ref = next.clone();

					search!().reserve(queue.len());

					steam!()
						.client()
						.ugc()
						.query_items(queue.to_owned())
						.unwrap()
						.allow_cached_response(600)
						.fetch(move |results: Result<QueryResults<'_>, SteamError>| {
							if let Ok(results) = results {
								search!().dirty();
								let mut search_installed_addons = search!().installed_addons.write();

								let mut i = 0;
								for item in results.iter() {
									let item = Addon::from(if let Some(item) = item {
										let mut item: WorkshopItem = item.into();
										item.preview_url = results.preview_url(i);
										item.subscriptions = results.statistic(i, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);

										if let Ok(pos) = search_installed_addons.binary_search_by(|x| match &x.source {
											crate::search::SearchItemSource::InstalledAddons(_, id) => id.as_ref().unwrap().cmp(&item.id),
											_ => unreachable!(),
										}) {
											let search_item = unsafe { Arc::get_mut_unchecked(&mut search_installed_addons[pos]) };
											if search_item.label != item.title {
												search_item.terms.push(std::mem::take(&mut search_item.label));
												search_item.label = item.title.to_owned();
											}
										}

										item
									} else {
										WorkshopItem::from(queue[i as usize])
									});

									steam!().workshop_channel.data(item);

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
						});

					while !next.load(Ordering::Acquire) {
						steam!().run_callbacks();
					}
				}

				FETCHER_NEXT.store(true, Ordering::Release);
			});

			sleep_ms!(50);
			while !FETCHER_NEXT.load(Ordering::Acquire) {
				sleep_ms!(50);
			}
		}
	}

	pub fn fetch_workshop_items(&'static self, ids: Vec<PublishedFileId>) {
		self.workshop.write(move |workshop| {
			let (cache, queue) = workshop.deref_mut();
			queue.reserve(ids.len());
			for id in ids.into_iter().filter(|id| cache.insert(*id)) {
				queue.push(id);
			}
		});
	}

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
									if !children.is_empty() {
										*response.lock() = Some(Some(children));
										return;
									}
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
		let client = self.client();
		client
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
			.require_tag("addon")
			.fetch(move |result: Result<QueryResults<'_>, SteamError>| {
				if let Ok(data) = result {
					*results_ref.lock() = Some(Some((
						data.total_results(),
						data.iter()
							.enumerate()
							.map(|(i, x)| {
								let mut item: WorkshopItem = x.unwrap().into();
								item.preview_url = data.preview_url(i as u32);
								item.subscriptions = data.statistic(i as u32, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
								search!().add(&item);
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

#[tauri::command]
pub fn fetch_workshop_items(items: Vec<PublishedFileId>) {
	steam!().fetch_workshop_items(items);
}

#[tauri::command]
pub fn fetch_workshop_item(item: PublishedFileId) {
	steam!().fetch_workshop_items(vec![item]);
}

#[tauri::command]
pub fn workshop_item_channel() -> u32 {
	steam!().workshop_channel.id
}

pub fn free_caches() {
	*steam!().users.write_sync() = HashMap::new();
	*steam!().workshop.write_sync() = (HashSet::new(), Vec::new());
}

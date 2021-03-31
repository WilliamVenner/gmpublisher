use rayon::{
	prelude::*,
	iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
	ThreadPool, ThreadPoolBuilder,
};
use serde::Serialize;
use std::{collections::HashMap, mem::MaybeUninit, path::PathBuf, sync::{Arc, atomic::{AtomicBool, AtomicUsize}}};

use steamworks::{AccountId, Callback, CallbackHandle, Client, ClientManager, Friend, PublishedFileId, QueryResult, QueryResults, SingleClient, SteamError, SteamId};

use atomic_refcell::AtomicRefCell;

use super::{AtomicRefSome, PromiseCache, PromiseHashCache, PromiseHashNullableCache};

use crate::main_thread_forbidden;

lazy_static::lazy_static! {
	static ref PERSONACHANGE_USER_INFO: steamworks::PersonaChange = steamworks::PersonaChange::NAME | steamworks::PersonaChange::AVATAR;
}

mod serde_steamid64 {
	use serde::{
		de::{self, Visitor},
		Deserializer, Serializer,
	};
	use steamworks::SteamId;

	pub(super) fn serialize<S>(steamid: &SteamId, serialize: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serialize.serialize_str(&steamid.raw().to_string())
	}

	struct SteamID64Visitor;
	impl<'de> Visitor<'de> for SteamID64Visitor {
		type Value = SteamId;

		fn visit_string<E>(self, str: String) -> Result<Self::Value, E>
		where
			E: de::Error,
		{
			Ok(SteamId::from_raw(str.parse::<u64>().unwrap_or(0)))
		}

		fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
			formatter.write_str("Expected a SteamID64")
		}
	}

	pub(super) fn deserialize<'de, D>(deserialize: D) -> Result<SteamId, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserialize.deserialize_string(SteamID64Visitor)
	}
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SteamUser {
	#[serde(with = "serde_steamid64")]
	steamid: SteamId,
	name: String,
	avatar: Option<crate::Base64Image>,

	dead: bool,
}
impl<Manager: steamworks::Manager> From<Friend<Manager>> for SteamUser {
	fn from(friend: Friend<Manager>) -> Self {
		SteamUser {
			steamid: friend.id(),
			name: friend.name(),
			avatar: friend.medium_avatar().map(|buf| crate::base64_image::Base64Image::new(buf, 64, 64)),
			dead: false, // TODO
		}
	}
}

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

pub struct Interface {
	client: Client,
	single: SingleClient,
	account_id: AccountId,
}
impl std::ops::Deref for Interface {
	type Target = Client;
	fn deref(&self) -> &Self::Target {
		&self.client
	}
}
impl From<(Client, SingleClient)> for Interface {
	fn from((client, single): (Client, SingleClient)) -> Self {
		let user = client.user();

		client.friends().request_user_information(user.steam_id(), false);

		Interface {
			account_id: user.steam_id().account_id(),
			client,
			single,
		}
	}
}

pub struct Steamworks {
	interface: AtomicRefCell<Option<Interface>>,
	thread_pool: ThreadPool,

	users: PromiseHashCache<SteamId, SteamUser>,
	workshop: PromiseHashCache<PublishedFileId, WorkshopItem>,
	collections: PromiseHashCache<PublishedFileId, Option<Vec<PublishedFileId>>>,
}

unsafe impl Sync for Steamworks {}
unsafe impl Send for Steamworks {}

impl Steamworks {
	// Callbacks //
	pub fn callback_once_with_data<'a, C: 'static, EqF>(&'static self, eq_f: EqF, timeout: u8) -> Option<C>
	where
		C: Callback,
		EqF: Fn(&C) -> bool + 'static + Send,
	{
		struct MultithreadedCallbackData<C> {
			inner: C,
		}
		unsafe impl<C> Send for MultithreadedCallbackData<C> {}
		unsafe impl<C> Sync for MultithreadedCallbackData<C> {}

		let data: Arc<AtomicRefCell<MaybeUninit<MultithreadedCallbackData<C>>>> = Arc::new(AtomicRefCell::new(MaybeUninit::uninit()));
		let _cb = {
			let mut data = Some(data.clone());
			self.register_callback(move |c: C| {
				if eq_f(&c) {
					if let Some(mut data) = data.take() {
						unsafe {
							*Arc::get_mut(&mut data).unwrap().get_mut().as_mut_ptr() = MultithreadedCallbackData { inner: c };
						}
					}
				}
			})
		};

		if timeout == 0 {
			while Arc::strong_count(&data) > 1 {
				std::thread::sleep(std::time::Duration::from_millis(25));
			}
		} else {
			let timeout = timeout as u64;
			let started = std::time::Instant::now();
			while Arc::strong_count(&data) > 1 {
				if timeout > 0 && started.elapsed().as_secs() >= timeout {
					return None;
				}
				std::thread::sleep(std::time::Duration::from_millis(25));
			}
		}

		Some(unsafe { Arc::try_unwrap(data).unwrap().into_inner().assume_init() }.inner)
	}

	pub fn callback_once<C, EqF>(&'static self, eq_f: EqF, timeout: u8) -> bool
	where
		C: Callback,
		EqF: Fn(&C) -> bool + 'static + Send,
	{
		let received = Arc::new(AtomicBool::new(false));
		let _cb = {
			let received = received.clone();
			self.register_callback(move |c: C| {
				if eq_f(&c) {
					received.store(true, std::sync::atomic::Ordering::Release);
				}
			})
		};

		if timeout == 0 {
			while !received.load(std::sync::atomic::Ordering::Acquire) {
				std::thread::sleep(std::time::Duration::from_millis(25));
			}
		} else {
			let timeout = timeout as u64;
			let started = std::time::Instant::now();
			while !received.load(std::sync::atomic::Ordering::Acquire) {
				if started.elapsed().as_secs() >= timeout {
					return false;
				}
				std::thread::sleep(std::time::Duration::from_millis(25));
			}
		}

		true
	}

	pub fn register_callback<C, F>(&'static self, f: F) -> CallbackHandle<ClientManager>
	where
		C: Callback,
		F: FnMut(C) + 'static + Send,
	{
		self.client().register_callback(f)
	}

	pub fn run_callbacks(&self) {
		self.client().single.run_callbacks();
		std::thread::sleep(std::time::Duration::from_millis(50));
	}

	// Users //

	pub fn fetch_user(&'static self, steamid: SteamId) -> SteamUser {
		main_thread_forbidden!();

		if self.client().friends().request_user_information(steamid, false) {
			self.callback_once(
				move |p: &steamworks::PersonaStateChange| p.flags & *PERSONACHANGE_USER_INFO == *PERSONACHANGE_USER_INFO && p.steam_id == steamid,
				10,
			);
		}

		let user = SteamUser::from(self.client().friends().get_friend(steamid));

		{
			let user = user.clone();
			crate::STEAMWORKS.users.write(move |mut users| {
				users.insert(user.steamid, user);
			});
		}

		user
	}

	pub fn fetch_users(&'static self, steamids: Vec<SteamId>) -> Vec<SteamUser> {
		self.users.begin();
		let mut users = Vec::with_capacity(steamids.len());
		steamids.into_par_iter().map(|steamid| self.fetch_user(steamid)).collect_into_vec(&mut users);
		self.users.commit();
		users
	}

	pub fn fetch_user_async<F>(&'static self, steamid: SteamId, f: F)
	where
		F: FnOnce(&SteamUser) + 'static + Send,
	{
		match self.users.read().get(&steamid) {
			Some(user) => f(user),
			None => {
				if self.users.task(steamid, f) {
					if self.client().friends().request_user_information(steamid, false) {
						self.thread_pool.spawn(move || {
							crate::STEAMWORKS.users.execute(&steamid, crate::STEAMWORKS.fetch_user(steamid));
						});
					} else {
						crate::STEAMWORKS.users.execute(&steamid, crate::STEAMWORKS.fetch_user(steamid));
					}
				}
			}
		}
	}

	pub fn fetch_users_async<F>(&'static self, steamids: Vec<SteamId>, f: F)
	where
		F: FnOnce(Vec<SteamUser>) + 'static + Send,
	{
		self.thread_pool.spawn(move || f(self.fetch_users(steamids)));
	}

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
				self.client().ugc().query_item(id.clone()).unwrap().fetch(move |query: Result<QueryResults<'_>, SteamError>| {
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
				self.client().ugc().query_items(uncached.to_owned()).unwrap().fetch(move |query: Result<QueryResults<'_>, SteamError>| {
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
						crate::STEAMWORKS.workshop.write(move |mut workshop| {
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
					self.thread_pool.spawn(move || {
						crate::STEAMWORKS.workshop.execute(&item, crate::STEAMWORKS.fetch_workshop_item(item));
					});
				}
			}
		}
	}

	pub fn fetch_workshop_items_async<F>(&'static self, items: Vec<PublishedFileId>, f: F)
	where
		F: FnOnce(Vec<WorkshopItem>) + 'static + Send,
	{
		self.thread_pool.spawn(move || f(crate::STEAMWORKS.fetch_workshop_items(items)));
	}

	pub fn fetch_workshop_item_with_uploader_async<F>(&'static self, item: PublishedFileId, f: F)
	where
		F: FnOnce(&WorkshopItem) + 'static + Send,
	{
		match self.workshop.read().get(&item) {
			Some(item) => if let Some(_) = item.owner {
				f(&item.clone());
			},
			None => {
				self.thread_pool.spawn(move || {
					crate::STEAMWORKS.workshop.execute(&item, crate::STEAMWORKS.fetch_workshop_item_with_uploader(item));
				});
			}
		}
	}

	pub fn fetch_collection_items(&'static self, collection: PublishedFileId) -> Option<Vec<PublishedFileId>> {
		main_thread_forbidden!();

		let response = Arc::new(AtomicRefCell::new(None));
		{
			let response = response.clone();
			self.client().ugc().query_item(collection).unwrap().include_children(true).fetch(move |query: Result<QueryResults<'_>, SteamError>| {
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
		    None => if self.collections.task(collection, f) {
				self.thread_pool.spawn(move || {
					crate::STEAMWORKS.collections.execute(&collection, self.fetch_collection_items(collection));
				});
			}
		}
	}

	// Static Steamworks //

	pub fn init() -> Steamworks {
		std::thread::spawn(Steamworks::connect);
		Steamworks {
			interface: AtomicRefCell::new(None),
			thread_pool: ThreadPoolBuilder::new().build().unwrap(),
			users: PromiseCache::new(HashMap::new()),
			workshop: PromiseCache::new(HashMap::new()),
			collections: PromiseCache::new(HashMap::new()),
		}
	}

	pub fn connect() {
		loop {
			if let Ok(connection) = Client::init() {
				println!("[Steamworks] Connected!");

				loop {
					if let Ok(mut interface) = crate::STEAMWORKS.interface.try_borrow_mut() {
						*interface = Some(connection.into());
						break;
					}
				}

				break;
			}

			std::thread::sleep(std::time::Duration::from_millis(50));
		}
	}

	pub fn connected(&self) -> bool {
		match self.interface.try_borrow() {
			Ok(interface) => interface.is_some(),
			Err(_) => true,
		}
	}

	pub fn client(&self) -> AtomicRefSome<Interface> {
		self.interface.borrow().into()
	}

	pub fn client_wait(&self) -> AtomicRefSome<Interface> {
		loop {
			if self.connected() {
				if let Ok(interface) = self.interface.try_borrow() {
					return interface.into();
				}
			}
		}
	}
}

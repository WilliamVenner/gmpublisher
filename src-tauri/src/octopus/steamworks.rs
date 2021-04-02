use lazy_static::lazy_static;
use parking_lot::{Condvar, Mutex, RwLock};
use rayon::{
	iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
};
use serde::Serialize;
use std::{collections::HashMap, hash::Hash, mem::MaybeUninit, path::PathBuf, sync::{atomic::AtomicBool, Arc}};

use steamworks::{AccountId, AppId, Callback, CallbackHandle, Client, ClientManager, Friend, ItemState, PublishedFileId, QueryResult, QueryResults, SingleClient, SteamError, SteamId, SteamServerConnectFailure, SteamServersConnected, SteamServersDisconnected};

use atomic_refcell::AtomicRefCell;

use super::{THREAD_POOL, AtomicRefSome, PromiseCache, PromiseHashCache};

use crate::{main_thread_forbidden, transaction, webview, webview_emit, webview_emit_safe, steamworks, transactions::Transaction};

lazy_static! {
	static ref PERSONACHANGE_USER_INFO: steamworks::PersonaChange = steamworks::PersonaChange::NAME | steamworks::PersonaChange::AVATAR;
	static ref ITEM_STATE_SKIP_DOWNLOAD: ItemState = ItemState::DOWNLOAD_PENDING | ItemState::DOWNLOADING;
}

lazy_static! {
	pub static ref DOWNLOADS: Downloads = Downloads::init();
}
#[derive(Debug)]
pub struct DownloadInner {
	id: PublishedFileId,
	transaction: Transaction,
	sent_total: AtomicBool
}
impl std::hash::Hash for DownloadInner {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl Eq for DownloadInner {}
impl PartialEq for DownloadInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl std::ops::Deref for DownloadInner {
    type Target = PublishedFileId;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
pub type Download = Arc<DownloadInner>;
pub struct Downloads {
	pending: Mutex<Vec<Download>>,
	downloading: Mutex<Vec<Download>>,
	watchdog: Condvar,
}
pub struct IDList {
	inner: Vec<PublishedFileId>
}
impl Into<Vec<PublishedFileId>> for IDList {
	fn into(self) -> Vec<PublishedFileId> {
		self.inner
	}
}
impl From<PublishedFileId> for IDList {
	fn from(id: PublishedFileId) -> Self {
		IDList { inner: vec![id] }
	}
}
impl From<Vec<PublishedFileId>> for IDList {
	fn from(ids: Vec<PublishedFileId>) -> Self {
		IDList { inner: ids }
	}
}
impl Downloads {
	fn init() -> Self {
		Self {
			pending: Mutex::new(Vec::new()),
			downloading: Mutex::new(Vec::new()),
			watchdog: Condvar::new(),
		}
	}

	pub fn download<IDs: Into<IDList>>(&self, ids: IDs) {
		let ids: Vec<PublishedFileId> = ids.into().into();

		let mut pending = self.pending.lock();
		pending.reserve(ids.len());

		for id in ids {
			let download = Arc::new(DownloadInner {
				id,
				sent_total: AtomicBool::new(false),
				transaction: transaction!()
			});

			pending.push(download.clone());

			webview_emit!("DownloadStarted", download.transaction.id).unwrap();
		}
	}

	pub fn start(&self) {
		let mut downloading = self.downloading.lock();
		downloading.append(&mut self.pending.lock());
		
		self.watchdog.notify_one();
	}

	fn watchdog() {
		loop {
			DOWNLOADS.watchdog.wait(&mut DOWNLOADS.downloading.lock());

			let ugc = steamworks!().client().ugc();

			loop {
				let mut downloading = std::mem::take(&mut *DOWNLOADS.downloading.lock());
				if downloading.is_empty() { break; }

				downloading = crate::dedup_unsorted(downloading);

				/* TODO
				steamworks!().register_callback(|download: steamworks::DownloadItemResult| {
					Remember to check the app id
				});
				*/
			
				while !downloading.is_empty() {
					let started = std::time::Instant::now();

					let mut i = 0;
					while i != downloading.len() {
						let download = &mut downloading[i];

						let state = ugc.item_state(download.id);
						if state.intersects(ItemState::INSTALLED) && !state.intersects(ItemState::NEEDS_UPDATE) {
							if let Some(info) = ugc.item_install_info(download.id) {
								download.transaction.finished(Some(info.folder));
								downloading.remove(i);
								continue;
							}
						} else if !state.intersects(*ITEM_STATE_SKIP_DOWNLOAD) {
							if !ugc.download_item(download.id, true) {
								download.transaction.error("ERR_DOWNLOAD_FAILED");
								downloading.remove(i);
								continue;
							} else {
								dprintln!("Starting ISteamUGC Download for {:?}", download.id);
							}
						}

						if let Some((downloaded, total)) = ugc.item_download_info(download.id) {
							if total != 0 {
								if !download.sent_total.fetch_or(true, std::sync::atomic::Ordering::SeqCst) {
									download.transaction.data(total);
								}

								download.transaction.progress((downloaded as f64) / (total as f64));

								i += 1;
								continue;
							}
						}

						if started.elapsed().as_secs() >= 10 {
							download.transaction.error("ERR_DOWNLOAD_TIMEOUT");
							downloading.remove(i);
						} else {
							i += 1;
						}
					}

					sleep_ms!(50);
				}
			}
		}
	}
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
	connected: AtomicBool,

	interface: AtomicRefCell<Option<Interface>>,

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
			steamworks!().users.write(move |mut users| {
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
						THREAD_POOL.spawn(move || {
							steamworks!().users.execute(&steamid, steamworks!().fetch_user(steamid));
						});
					} else {
						steamworks!().users.execute(&steamid, steamworks!().fetch_user(steamid));
					}
				}
			}
		}
	}

	pub fn fetch_users_async<F>(&'static self, steamids: Vec<SteamId>, f: F)
	where
		F: FnOnce(Vec<SteamUser>) + 'static + Send,
	{
		THREAD_POOL.spawn(move || f(self.fetch_users(steamids)));
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
					steamworks!().workshop.execute(&item, steamworks!().fetch_workshop_item_with_uploader(item));
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

	// Static Steamworks //

	pub fn init() -> Steamworks {
		std::thread::spawn(Steamworks::connect);
		Steamworks {
			connected: AtomicBool::new(false),
			interface: AtomicRefCell::new(None),
			users: PromiseCache::new(HashMap::new()),
			workshop: PromiseCache::new(HashMap::new()),
			collections: PromiseCache::new(HashMap::new()),
		}
	}

	fn watchdog() {
		#[cfg(debug_assertions)]
		std::mem::forget(steamworks!().register_callback(|c: SteamServerConnectFailure| {
			println!("[Steamworks] SteamServerConnectFailure {:#?}", c);
		}));

		std::mem::forget(steamworks!().register_callback(|_: SteamServersConnected| {
			steamworks!().set_connected(true);
			println!("[Steamworks] Connected");
		}));

		std::mem::forget(steamworks!().register_callback(|c: SteamServersDisconnected| {
			steamworks!().set_connected(false);
			println!("[Steamworks] SteamServersDisconnected {:#?}", c);
		}));

		loop { steamworks!().run_callbacks(); }
	}

	fn on_initialized() {
		std::thread::spawn(Steamworks::watchdog);

		lazy_static::initialize(&DOWNLOADS);
		std::thread::spawn(Downloads::watchdog);
	}

	pub fn connect() {
		loop {
			if let Ok(connection) = Client::init() {
				println!("[Steamworks] Client initialized");

				loop {
					if let Ok(mut interface) = steamworks!().interface.try_borrow_mut() {
						*interface = Some(connection.into());
						break;
					}
				}

				steamworks!().set_connected(true);

				Steamworks::on_initialized();

				break;
			}

			std::thread::sleep(std::time::Duration::from_millis(50));
		}
	}

	pub fn connected(&self) -> bool {
		self.connected.load(std::sync::atomic::Ordering::Acquire)
	}

	fn set_connected(&self, connected: bool) {
		self.connected.store(connected, std::sync::atomic::Ordering::Release);
		webview_emit_safe!(if connected { "SteamConnected" } else { "SteamDisconnected" });
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

#[tauri::command]
fn is_steam_connected() -> bool {
	steamworks!().connected()
}

pub mod publishing {
	use crate::GMOD_APP_ID;
	use std::{io::BufReader, path::PathBuf, sync::Arc};
	use anyhow::anyhow;
    use parking_lot::RwLock;
    use steamworks::PublishedFileId;
	use std::fs::File;
	use image::{ImageError, ImageFormat};
	use std::mem::MaybeUninit;

    use super::Steamworks;
	pub struct ContentPath(PathBuf);
	impl std::ops::Deref for ContentPath {
    	type Target = PathBuf;
    	fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl Into<PathBuf> for ContentPath {
    	fn into(self) -> PathBuf {
			self.0
		}
	}
	impl ContentPath {
		pub fn new(path: PathBuf) -> Result<ContentPath, anyhow::Error> {
			if !path.is_dir() {
				return Err(anyhow!("ERR_CONTENT_PATH_NOT_DIR"));
			}

			let mut gma_path: MaybeUninit<PathBuf> = MaybeUninit::uninit();
			for (i, path) in path.read_dir()?.filter_map(|entry| entry.ok().and_then(|entry| {
				let path = entry.path();
				let extension = path.extension()?;
				if extension == "gma" {
					Some(path)
				} else {
					None
				}
			})).enumerate() {
				if i > 0 {
					return Err(anyhow!("ERR_CONTENT_PATH_MULTIPLE_GMA"));
				}
				unsafe { gma_path.as_mut_ptr().write(path); }
			}

			Ok(ContentPath(unsafe { gma_path.assume_init() }))
		}
	}

	const WORKSHOP_ICON_MAX_SIZE: u64 = 1000000;
	const WORKSHOP_ICON_MIN_SIZE: u64 = 16;
	pub struct WorkshopIcon(PathBuf);
	impl std::ops::Deref for WorkshopIcon {
    	type Target = PathBuf;
    	fn deref(&self) -> &Self::Target {
			&self.0
		}
	}
	impl Into<PathBuf> for WorkshopIcon {
    	fn into(self) -> PathBuf {
			self.0
		}
	}
	impl WorkshopIcon {
		fn try_format(tried_best_guess: bool, file_type: ImageFormat, path: &PathBuf, mut file_types: Vec<ImageFormat>) -> Result<(), anyhow::Error> {
			if let Err(error) = image::load(BufReader::new(File::open(path)?), file_type) {
				if let ImageError::Decoding(decoding_error) = error {
					if !tried_best_guess {
						if let image::error::ImageFormatHint::Exact(best_guess) = decoding_error.format_hint() {
							let mut i = 0;
							while i != file_types.len() {
								if file_types[i] == best_guess {
									return WorkshopIcon::try_format(true, file_types.remove(i), path, file_types);
								} else {
									i += 1;
								}
							}
						}
					}

					if file_types.is_empty() {
						Err(anyhow!("ERR_ICON_INVALID_FORMAT"))
					} else {
						WorkshopIcon::try_format(tried_best_guess, file_types.remove(0), path, file_types)
					}

				} else {
					Err(anyhow!(error))
				}
			} else {
				Ok(())
			}
		}

		pub fn new(path: PathBuf) -> Result<WorkshopIcon, anyhow::Error> {
			let len = path.metadata()?.len();
			if len > WORKSHOP_ICON_MAX_SIZE {
				return Err(anyhow!("ERR_ICON_TOO_LARGE"));
			} else if len < WORKSHOP_ICON_MIN_SIZE {
				return Err(anyhow!("ERR_ICON_TOO_SMALL"));
			}

			let (w, h) = image::image_dimensions(&path)?;
			if w != 512 || h != 512 {
				return Err(anyhow!("ERR_ICON_INCORRECT_DIMENSIONS"));
			}

			let file_extension = path.extension().and_then(|x| x.to_str()).unwrap_or("jpg").to_ascii_lowercase();
			let mut file_types = match file_extension.as_str() {
				"png" => vec![ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Gif],
				"gif" => vec![ImageFormat::Gif, ImageFormat::Jpeg, ImageFormat::Png],
				_ => vec![ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Gif],
			};

			WorkshopIcon::try_format(false, file_types.remove(0), &path, file_types)?;

			Ok(WorkshopIcon(path))
		}
	}

	pub struct WorkshopCreationDetails {
		id: PublishedFileId,
		title: String,
		path: ContentPath,
		preview: WorkshopIcon,
	}
	pub struct WorkshopUpdateDetails {
		id: PublishedFileId,
		path: PathBuf,
		preview: Option<PathBuf>,
		changes: Option<String>,
	}
	pub enum WorkshopUpdateType {
		Creation(WorkshopCreationDetails),
		Update(WorkshopUpdateDetails)
	}

	pub fn update(steamworks: &'static Steamworks, details: WorkshopUpdateType) -> Result<(PublishedFileId, bool), anyhow::Error> {
		use WorkshopUpdateType::*;

		let result = Arc::new(RwLock::new(None));
		let result_ref = result.clone();
		match details {
			Creation(details) => {
				steamworks.client().ugc().start_item_update(GMOD_APP_ID, details.id)
				.content_path(&details.path)
				.title(&details.title)
				.preview_path(&details.preview)
				.submit(None, move |result| {
					*result_ref.write() = Some(result);
				});
			},
			
			Update(details) => {
				let path = ContentPath::new(details.path)?;
				let preview = match details.preview { Some(preview) => Some(WorkshopIcon::new(preview)?), None => None };

				let update = steamworks.client().ugc().start_item_update(GMOD_APP_ID, details.id);
				match preview {
					Some(preview) => update.preview_path(&*preview),
					None => update
				}
				.content_path(&path)
				.submit(details.changes.as_deref(), move |result| {
					*result_ref.write() = Some(result);
				});
			}
		}

		loop {
			if !result.is_locked() && result.read().is_some() {
				break Arc::try_unwrap(result).unwrap().into_inner().unwrap().map_err(|error| anyhow!(error));
			} else {
				steamworks.run_callbacks();
			}
		}
	}

	pub fn publish(steamworks: &'static Steamworks, path: PathBuf, title: String, preview: PathBuf) -> Result<(PublishedFileId, bool), anyhow::Error> {
		let path = ContentPath::new(path)?;
		let preview = WorkshopIcon::new(preview)?;

		// FIXME audit the AtomicRefCells to make sure they can't mutably borrow when the loop is going
		// maybe use a RwLock instead
		let published = Arc::new(RwLock::new(None));
		let published_ref = published.clone();
		steamworks.client().ugc().create_item(GMOD_APP_ID, steamworks::FileType::Community, move |result| {
			match result {
				Ok((id, accepted_legal_agreement)) => {
					// TODO test accepted_legal_agreement
					*published_ref.write() = Some(Some(id));
				},
				Err(_) => {
					*published_ref.write() = Some(None);
				},
			}
		});

		loop {
			if let Some(published_ref) = published.try_read() {
				if published_ref.is_some() {
					break;
				}
			}
			steamworks.run_callbacks();
		}

		let id = match Arc::try_unwrap(published).unwrap().into_inner().unwrap() {
			None => return Err(anyhow!("ERR_PUBLISH_FAILED")),
			Some(id) => id,
		};

		let details = WorkshopUpdateType::Creation(WorkshopCreationDetails {
			id,
			title,
			preview,
			path
		});

		self::update(steamworks, details)
	}
}
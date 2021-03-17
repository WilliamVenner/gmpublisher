use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex, atomic::AtomicBool}};
use anyhow::{anyhow, Error};
use lazy_static::lazy_static;
use steamworks::{PublishedFileId, AccountId, AppId, Client, CreateQueryError, QueryResult, QueryResults, SingleClient, SteamError, SteamId};
use serde::Serialize;
use tauri::Webview;

static APP_GMOD: AppId = AppId(4000);

use super::Base64Image;

lazy_static! {
	static ref PERSONACHANGE_USER_INFO: steamworks::PersonaChange = steamworks::PersonaChange::NAME | steamworks::PersonaChange::AVATAR;
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct SteamUser {
	#[serde(skip)]
	steamid: SteamId,
	steamid64: String,
	name: String,
	avatar: Option<Base64Image>
}

pub(crate) struct Workshop {
	client: Client,
	single: Arc<Mutex<SingleClient>>,
	account_id: AccountId,
	cache: Arc<Mutex<HashMap<PublishedFileId, Option<WorkshopItem>>>>,
	users: Arc<Mutex<HashMap<SteamId, SteamUser>>>
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all="camelCase")]
pub(crate) struct WorkshopItem {
	pub(crate) id: PublishedFileId,
	pub(crate) title: String,
	pub(crate) owner: Option<SteamUser>,
	pub(crate) time_created: u32,
	pub(crate) time_updated: u32,
	pub(crate) description: Option<String>,
	pub(crate) score: f32,
	pub(crate) tags: Vec<String>,
	pub(crate) preview_url: Option<String>,
	pub(crate) subscriptions: u64,
	pub(crate) local_file: Option<PathBuf>,
	pub(crate) search_title: String,
	
	#[serde(skip)]
	pub(crate) steamid: Option<SteamId>,
	pub(crate) steamid64: Option<String>,
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
			search_title: result.title.to_lowercase()
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
			search_title: id.0.to_string()
		}
    }
}

impl Workshop {
	pub(crate) fn init() -> Result<Workshop, Error> {
		let (client, single) = Client::init()?;
		client.friends().request_user_information(client.user().steam_id(), false);
		Ok(Workshop {
			single: Arc::new(Mutex::new(single)),
			account_id: client.user().steam_id().account_id(),
			client,
			cache: Arc::new(Mutex::new(HashMap::new())),
			users: Arc::new(Mutex::new(HashMap::new())),
		})
	}
	
	pub(crate) fn get_gmod(&self) -> Option<String> {
		let apps = self.client.apps();
		if !apps.is_app_installed(APP_GMOD) { return None }
		Some(apps.app_install_dir(APP_GMOD))
	}

	pub(crate) fn get_user(&self) -> SteamUser {
		let friends = self.client.friends();
		let steamid = self.client.user().steam_id();

		SteamUser {
			steamid,
			steamid64: steamid.raw().to_string(),
			name: friends.name(),
			avatar: friends.get_friend(steamid).medium_avatar().map(|buf| Base64Image::new(buf, 64, 64))
		}
	}

	pub(crate) fn query_user(&self, steamid: SteamId) -> SteamUser {
		use std::sync::atomic::Ordering::Relaxed;

		let mut users = self.users.lock().unwrap();
		match users.get(&steamid) {
			None => {
				let friends = self.client.friends();

				if friends.request_user_information(steamid, false) {
					let sync = Arc::new(AtomicBool::new(false));
					let _cb = {
						let c_sync = sync.clone();
						self.client.register_callback(move |p: steamworks::PersonaStateChange| {
							if p.flags & *PERSONACHANGE_USER_INFO == *PERSONACHANGE_USER_INFO && p.steam_id == steamid {
								c_sync.store(true, Relaxed);
							}
						})
					};

					let single = self.single.lock().unwrap();
					while !sync.load(Relaxed) {
						single.run_callbacks();
						std::thread::sleep(std::time::Duration::from_millis(50));
					}
				}

				let user = friends.get_friend(steamid);
				let steam_user = SteamUser {
					steamid,
					steamid64: steamid.raw().to_string(),
					name: user.name(),
					avatar: user.medium_avatar().map(|buf| Base64Image::new(buf, 64, 64))
				};

				users.insert(steamid, steam_user.clone());
				steam_user
			},

			Some(user) => user.clone()
		}
	}

	pub(crate) fn get_item(&self, id: PublishedFileId) -> Result<Result<Option<WorkshopItem>, SteamError>, CreateQueryError> {
		let sync = Arc::new(Mutex::new(None));

		{
			let c_cache = self.cache.clone();
			let c_sync = sync.clone();
			self.client.ugc().query_item(id)?.fetch(move |result: Result<QueryResults<'_>, SteamError>| {
				let mut lock = c_sync.lock().unwrap();
				match result {

					Ok(data) => {
						let mut cache = c_cache.lock().unwrap();
						if data.total_results() == 0 {
							cache.insert(id, None);
							*lock = Some(Ok(None));
						} else {
							let mut item: WorkshopItem = match data.get(0) {
								Some(item) => item.into(),
								None => {
									*lock = Some(Err(SteamError::BadResponse));
									return;
								}
							};
							item.preview_url = data.preview_url(0);
							item.subscriptions = data.statistic(0, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
							cache.insert(item.id, Some(item.clone()));

							*lock = Some(Ok(Some(item)));
						}
					},

					Err(error) => *lock = Some(Err(error))

				};
			});
		}

		let single = self.single.lock().unwrap();
		while sync.lock().unwrap().is_none() {
			single.run_callbacks();
			::std::thread::sleep(::std::time::Duration::from_millis(50));
		}
		
		let data = sync.lock().unwrap().take().unwrap();
		Ok(data)
	}

	pub(crate) fn get_items(&self, ids: Vec<PublishedFileId>) -> Result<Result<(u32, Vec<WorkshopItem>), SteamError>, CreateQueryError> {
		let sync = Arc::new(Mutex::new(None));

		{
			let mut ids_ref = ids.clone().into_iter();
			let c_cache = self.cache.clone();
			let c_sync = sync.clone();
			self.client.ugc().query_items(ids)?.fetch(move |result: Result<QueryResults<'_>, SteamError>| {
				let mut lock = c_sync.lock().unwrap();
				match result {

					Ok(data) => {
						let mut cache = c_cache.lock().unwrap();
						*lock = Some(Ok(
							(
								data.total_results(),
								data.iter_maybe().enumerate().map(|(i, x)| {
									match x {
										Some(x) => {
											ids_ref.nth(0);
											
											let mut item: WorkshopItem = x.into();
											item.preview_url = data.preview_url(i as u32);
											item.subscriptions = data.statistic(i as u32, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
											cache.insert(item.id, Some(item.clone()));
											item
										}
										None => ids_ref.nth(0).unwrap().into()
									}
								}).collect::<Vec<WorkshopItem>>(),
							)
						));
					},

					Err(error) => *lock = Some(Err(error))

				};
			});
		}

		let single = self.single.lock().unwrap();
		while sync.lock().unwrap().is_none() {
			single.run_callbacks();
			::std::thread::sleep(::std::time::Duration::from_millis(50));
		}
		
		let data = sync.lock().unwrap().take().unwrap();
		Ok(data)
	}

	pub(crate) fn query(&self, page: u32) -> Result<Result<(u32, Vec<WorkshopItem>), SteamError>, CreateQueryError> {
		let sync = Arc::new(Mutex::new(None));

		{
			let c_cache = self.cache.clone();
			let c_sync = sync.clone();
			self.client.ugc().query_user(
				self.account_id,
				steamworks::UserList::Published,
				steamworks::UGCType::ItemsReadyToUse,
				steamworks::UserListOrder::LastUpdatedDesc,
				steamworks::AppIDs::ConsumerAppId(APP_GMOD),
				page
			)?.exclude_tag("dupe").fetch(move |result: Result<QueryResults<'_>, SteamError>| {
				let mut lock = c_sync.lock().unwrap();
				match result {

					Ok(data) => {
						let mut cache = c_cache.lock().unwrap();
						*lock = Some(Ok(
							(
								data.total_results(),
								data.iter().enumerate().map(|(i, x)| {
									let mut item: WorkshopItem = x.into();
									item.preview_url = data.preview_url(i as u32);
									item.subscriptions = data.statistic(i as u32, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
									cache.insert(item.id, Some(item.clone()));
									item
								}).collect::<Vec<WorkshopItem>>()
							)
						));
					},

					Err(error) => *lock = Some(Err(error))

				};
			});
		}

		let single = self.single.lock().unwrap();
		while sync.lock().unwrap().is_none() {
			single.run_callbacks();
			::std::thread::sleep(::std::time::Duration::from_millis(50));
		}
		
		let data = sync.lock().unwrap().take().unwrap();
		Ok(data)
	}
}

pub(crate) fn browse(resolve: String, reject: String, webview: &mut Webview<'_>, page: u32) -> Result<(), String> {
	tauri::execute_promise(webview, move || {
		match crate::WORKSHOP.read().unwrap().query(page).unwrap() {
			Ok(items) => Ok(items),
			Err(error) => Err(anyhow!(error))
		}
	}, resolve, reject);

	Ok(())
}
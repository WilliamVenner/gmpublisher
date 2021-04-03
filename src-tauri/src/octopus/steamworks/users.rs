use lazy_static::lazy_static;
use parking_lot::{Condvar, Mutex, RwLock};
use rayon::{
	iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator},
};
use serde::Serialize;
use std::{collections::HashMap, hash::Hash, mem::MaybeUninit, path::PathBuf, sync::{atomic::AtomicBool, Arc}};

use steamworks::{AccountId, AppId, Callback, CallbackHandle, Client, ClientManager, Friend, ItemState, PublishedFileId, QueryResult, QueryResults, SingleClient, SteamError, SteamId, SteamServerConnectFailure, SteamServersConnected, SteamServersDisconnected};

use atomic_refcell::AtomicRefCell;

use super::{AtomicRefSome, PromiseCache, PromiseHashCache, Steamworks, THREAD_POOL};

use crate::{main_thread_forbidden, transaction, GMOD_APP_ID, webview_emit, steamworks, transactions::Transaction};

lazy_static! {
	static ref PERSONACHANGE_USER_INFO: steamworks::PersonaChange = steamworks::PersonaChange::NAME | steamworks::PersonaChange::AVATAR;
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SteamUser {
	#[serde(with = "super::serde_steamid64")]
	pub steamid: SteamId,
	pub name: String,
	pub avatar: Option<crate::Base64Image>,
	
	pub dead: bool,
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

impl Steamworks {
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
}
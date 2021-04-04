use std::{collections::{HashMap, HashSet}, mem::MaybeUninit, sync::{atomic::AtomicBool, Arc}};

use parking_lot::RwLock;
use steamworks::{
	AccountId, Callback, CallbackHandle, Client, ClientManager, PublishedFileId, SingleClient, SteamId, SteamServerConnectFailure,
	SteamServersConnected, SteamServersDisconnected,
};

use atomic_refcell::AtomicRefCell;

use self::{downloads::Downloads, users::SteamUser, workshop::WorkshopItem};

use super::{AtomicRefSome, PromiseCache, PromiseHashCache, RelaxedRwLock, THREAD_POOL};

use crate::{Base64Image, steamworks, webview_emit};

pub mod downloads;
pub mod publishing;
pub mod users;
pub mod workshop;

pub use downloads::DOWNLOADS;

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

pub struct Interface {
	client: Client,
	single: SingleClient,
	steam_id: SteamId,
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
			steam_id: user.steam_id(),
			client,
			single,
		}
	}
}

pub struct Steamworks {
	connected: AtomicBool,

	interface: AtomicRefCell<Option<Interface>>,

	users: PromiseHashCache<SteamId, SteamUser>,
	collections: PromiseHashCache<PublishedFileId, Option<Vec<PublishedFileId>>>,

	workshop: RelaxedRwLock<(HashSet<PublishedFileId>, Vec<PublishedFileId>)>,
}

unsafe impl Sync for Steamworks {}
unsafe impl Send for Steamworks {}

impl Steamworks {
	pub fn init() -> Steamworks {
		std::thread::spawn(Steamworks::connect);
		Steamworks {
			connected: AtomicBool::new(false),
			interface: AtomicRefCell::new(None),
			users: PromiseCache::new(HashMap::new()),
			collections: PromiseCache::new(HashMap::new()),

			workshop: RelaxedRwLock::new((HashSet::new(), Vec::new())),
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

		loop {
			steamworks!().run_callbacks();
		}
	}

	fn on_initialized() {
		std::thread::spawn(Steamworks::watchdog);
		std::thread::spawn(Steamworks::workshop_fetcher);

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
		webview_emit!(if connected { "SteamConnected" } else { "SteamDisconnected" });
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
}

#[tauri::command]
fn is_steam_connected() -> bool {
	steamworks!().connected()
}

#[tauri::command]
fn get_user_info() -> (String, Option<Base64Image>) {
	steamworks!().client_wait();
	let user = steamworks!().fetch_user(steamworks!().client().steam_id);
	(user.name, user.avatar)
}

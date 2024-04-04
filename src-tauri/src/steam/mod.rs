use std::{
	collections::{HashMap, HashSet},
	mem::MaybeUninit,
	sync::{atomic::AtomicBool, Arc},
};

use steamworks::{
	Callback, CallbackHandle, Client, ClientManager, PublishedFileId, SingleClient, SteamId, SteamServerConnectFailure, SteamServersConnected,
	SteamServersDisconnected,
};

use atomic_refcell::AtomicRefCell;

use self::{downloads::Downloads, users::SteamUser};

use crate::{
	octopus::{AtomicRefSome, PromiseCache, PromiseHashCache, RelaxedRwLock},
	Transaction,
};

use crate::webview_emit;

pub mod downloads;
pub mod publishing;
pub mod subscriptions;
pub mod users;
pub mod workshop;

pub use downloads::DOWNLOADS;

pub const RESULTS_PER_PAGE: usize = steamworks::RESULTS_PER_PAGE as usize;

pub fn serialize_opt_steamid<S>(steamid: &Option<SteamId>, serialize: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	match steamid {
		Some(steamid) => serialize.serialize_some(&steamid.raw().to_string()),
		None => serialize.serialize_none(),
	}
}

pub fn serialize_steamid<S>(steamid: &SteamId, serialize: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	serialize.serialize_str(&steamid.raw().to_string())
}

pub struct Interface {
	client: Client,
	single: SingleClient,
	pub steam_id: SteamId,
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

pub struct Steam {
	connected: AtomicBool,

	interface: AtomicRefCell<Option<Interface>>,

	users: PromiseHashCache<SteamId, SteamUser>,

	workshop: RelaxedRwLock<(HashSet<PublishedFileId>, Vec<PublishedFileId>)>,
	workshop_channel: Transaction,
}

unsafe impl Sync for Steam {}
unsafe impl Send for Steam {}

impl Steam {
	pub fn init() -> Steam {
		std::thread::spawn(Steam::connect);
		Steam {
			connected: AtomicBool::new(false),
			interface: AtomicRefCell::new(None),
			users: PromiseCache::new(HashMap::new()),

			workshop: RelaxedRwLock::new((HashSet::new(), Vec::new())),
			workshop_channel: transaction!(),
		}
	}

	fn watchdog() {
		#[cfg(debug_assertions)]
		std::mem::forget(steam!().register_callback(|c: SteamServerConnectFailure| {
			steam!().set_connected(false);
			println!("[Steam] SteamServerConnectFailure {:#?}", c);
		}));

		std::mem::forget(steam!().register_callback(|_: SteamServersConnected| {
			steam!().set_connected(true);
			println!("[Steam] Connected");
		}));

		std::mem::forget(steam!().register_callback(|c: SteamServersDisconnected| {
			steam!().set_connected(false);
			println!("[Steam] SteamServersDisconnected {:#?}", c);
		}));

		loop {
			steam!().run_callbacks();
		}
	}

	fn on_initialized() {
		std::thread::spawn(Steam::watchdog);
		std::thread::spawn(Steam::workshop_fetcher);

		lazy_static::initialize(&DOWNLOADS);
		std::thread::spawn(Downloads::watchdog);

		if app_data!().settings.read().gmod.is_none() {
			app_data!().send();
		}
	}

	pub fn connect() {
		loop {
			if let Ok(connection) = Client::init_app(4000) {
				println!("[Steam] Client initialized");

				loop {
					if let Ok(mut interface) = steam!().interface.try_borrow_mut() {
						*interface = Some(connection.into());
						break;
					}
				}

				steam!().set_connected(true);

				Steam::on_initialized();

				break;
			}

			sleep_ms!(50);
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
			sleep_ms!(50);
		}
	}

	// Callbacks //
	pub fn callback_once_with_data<C: 'static, EqF>(&'static self, eq_f: EqF, timeout: u8) -> Option<C>
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
				self.run_callbacks();
			}
		} else {
			let timeout = timeout as u64;
			let started = std::time::Instant::now();
			while Arc::strong_count(&data) > 1 {
				if timeout > 0 && started.elapsed().as_secs() >= timeout {
					return None;
				}
				self.run_callbacks();
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
				self.run_callbacks();
			}
		} else {
			let timeout = timeout as u64;
			let started = std::time::Instant::now();
			while !received.load(std::sync::atomic::Ordering::Acquire) {
				if started.elapsed().as_secs() >= timeout {
					return false;
				}
				self.run_callbacks();
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
		sleep_ms!(50);
	}
}

#[tauri::command]
pub fn is_steam_connected() -> bool {
	steam!().connected()
}

#[tauri::command]
pub fn get_current_user() -> (String, Option<crate::Base64Image>) {
	steam!().client_wait();
	let user = steam!().fetch_user(steam!().client().steam_id);
	(user.name, user.avatar)
}

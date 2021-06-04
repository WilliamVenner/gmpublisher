import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event';
import { Transaction } from './transactions';

class DeferredPromise {
	constructor(wrapPromise) {
		this._promise = new Promise((resolve, reject) => {
			this.resolve = resolve;
			this.reject = reject;
		});

		this.then = this._promise.then.bind(this._promise);
		this.catch = this._promise.catch.bind(this._promise);

		this[Symbol.toStringTag] = 'Promise';

		if (wrapPromise) wrapPromise.then(this.resolve, this.reject);
	}

	static resolve(data) {
		const promise = new DeferredPromise();
		promise.resolve(data);
		return promise;
	}

	static reject(data) {
		const promise = new DeferredPromise();
		promise.reject(data);
		return promise;
	}
}

class Steam {
	constructor() {
		this.Addons = {};
		this.Workshop = {};
		this.SteamUsers = {};

		this.MyWorkshop = [];
		this.InstalledAddons = [];

		listen("WorkshopItem", ({ payload: { workshop: workshopItem } }) => {
			this.workshopItemReceived(workshopItem);
		});
	}

	workshopItemReceived(workshopItem) {
		if (workshopItem.id in this.Workshop) {
			this.Workshop[workshopItem.id].resolve(workshopItem);
		} else {
			this.Workshop[workshopItem.id] = DeferredPromise.resolve(workshopItem);
		}
	}

	getMyWorkshop(page) {
		if (this.MyWorkshop[page] == null) {
			this.MyWorkshop[page] = new DeferredPromise(invoke("browse_my_workshop", { page }));
		}
		return this.MyWorkshop[page];
	}

	getInstalledAddons(page) {
		if (this.InstalledAddons[page] == null) {
			this.InstalledAddons[page] = new DeferredPromise(invoke("browse_installed_addons", { page }));
		}
		return this.InstalledAddons[page];
	}

	getSubscribedAddons(page) {
		return new DeferredPromise(invoke("browse_subscribed_addons", { page }))
	}

	getAddon(path) {
		if (this.Addons[path] == null) {
			this.Addons[path] = new DeferredPromise(invoke("get_installed_addon", { path }));
		}
		return this.Addons[path];
	}

	getWorkshopAddon(id, deadCallback) {
		if (this.Workshop[id] == null) {
			invoke("fetch_workshop_item", { item: id });
			this.Workshop[id] = new DeferredPromise();
		}
		if (deadCallback) {
			this.Workshop[id].then(item => {
				if (item.dead) deadCallback(item);
				return item;
			});
		}
		return this.Workshop[id];
	}

	async lockWorkshopAddons(lock) {
		await invoke("lock_fetch_workshop_items", { lock });
	}

	getSteamUser(steamid64) {
		if (this.SteamUsers[steamid64] == null) {
			this.SteamUsers[steamid64] = new DeferredPromise(invoke("get_steam_user", { steamid64 }));
		}
		return this.SteamUsers[steamid64];
	}
}

function trimPath(path) {
	let n = 0;
	for (let i = path.length-1; i >= 0; i--) {
		if (path[i] === '/' || path[i] === '\\') {
			n++;
		} else {
			break;
		}
	}
	if (n > 0) {
		return path.substr(0, path.length - n);
	} else {
		return path;
	}
}

function getFileIcon(extension) {
	switch(extension) {
		case 'lua':
			return 'script_code.png';

		case 'mp3':
		case 'ogg':
		case 'wav':
			return 'sound.png';

		case 'png':
		case 'jpg':
		case 'jpeg':
			return 'photo.png';

		case 'bsp':
		case 'nav':
		case 'ain':
		case 'fgd':
			return 'map.png';

		case 'pcf':
			return 'wand.png';

		case 'vcd':
			return 'comments.png';

		case 'ttf':
			return 'font.png';

		case 'txt':
			return 'page_white_text.png';

		case 'properties':
			return 'page_white_wrench.png';

		case 'vmt':
		case 'vtf':
			return 'picture_link.png';

		case 'mdl':
		case 'vtx':
		case 'phy':
		case 'ani':
		case 'vvd':
			return 'bricks.png';

		default:
			return 'page_white.png';
	}
	// TODO remove unused
}

function getFileType(extension) {
	switch(extension) {
		case 'mp3':
		case 'ogg':
		case 'wav':
			return 'audio';

		case 'png':
		case 'jpg':
		case 'jpeg':
			return 'image';

		case 'bsp':
			return 'map';

		case 'vtf':
		case 'vmt':
		case 'map':
		case 'ain':
		case 'nav':
		case 'ttf':
		case 'vcd':
		case 'fgd':
		case 'pcf':
		case 'lua':
		case 'mdl':
		case 'vtx':
		case 'phy':
		case 'ani':
		case 'vvd':
		case 'txt':
		case 'properties':
			return extension;

		default:
			return 'unknown';
	}
}

const RE_FILE_EXTENSION = /^.*(?:\.(.*?))$/;
function getFileTypeInfo(path) {
	const extension = path.match(RE_FILE_EXTENSION)?.[1].toLowerCase();
	return [getFileIcon(extension), getFileType(extension), extension];
}

const steam = new Steam();

listen('InstalledAddonsRefreshed', () => {
	steam.InstalledAddons = [];
});

invoke('workshop_item_channel').then(transaction_id => {
	const transaction = new Transaction(transaction_id);
	transaction.listen(event => {
		if (event.stream) {
			steam.workshopItemReceived(event.data.workshop);
		}
	});
});

export { steam as Steam, getFileTypeInfo, trimPath }

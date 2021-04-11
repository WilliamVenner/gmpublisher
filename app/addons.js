import { invoke } from '@tauri-apps/api/tauri'
import { listen } from '@tauri-apps/api/event';

class Addons {
	constructor() {
		this.Addons = {};
		this.Workshop = {};

		this.MyWorkshop = [];
		this.InstalledAddons = [];

		listen("WorkshopItem", ({ payload: { workshop: workshopItem } }) => {
			this.Workshop[workshopItem.id] = Promise.resolve(workshopItem);
		});
	}

	getMyWorkshop(page) {
		if (this.MyWorkshop[page] == null) {
			this.MyWorkshop[page] = invoke("browse_my_workshop", { page });
		}
		return this.MyWorkshop[page];
	}

	getInstalledAddons(page) {
		if (this.InstalledAddons[page] == null) {
			this.InstalledAddons[page] = invoke("browse_installed_addons", { page });
		}
		return this.InstalledAddons[page];
	}

	getAddon(path) {
		if (this.Addons[path] == null) {
			this.Addons[path] = invoke("get_installed_addon", { path });
		}
		return this.Addons[path];
	}

	getWorkshopAddon(id) {
		if (this.Workshop[id] == null) {
			console.log('get_workshop_addon');
			this.Workshop[id] = invoke("get_workshop_addon", { id });
		}
		console.log('shit nugget', this.Workshop[id]);
		return this.Workshop[id];
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

const addons = new Addons();
export { addons as Addons, getFileTypeInfo, trimPath }

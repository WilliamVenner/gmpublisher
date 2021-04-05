import { invoke } from '@tauri-apps/api/tauri'
import { Transaction } from './transactions.js'

class Addons {
	constructor() {
		this.workshopCache = [];
		this.gameAddonsCache = [];

		this.gmaMetadataCache = {};
		this.gmaMetadataQueue = {
			queue: [],
			waiting: {}
		};

		this.gmaPreviewCache = {};

		this.workshopMetadataCache = {};
		this.workshopUploaderCache = {};
	}

	clearCache(cacheName) {
		this[cacheName + 'Cache'] = [];
	}

	browseWorkshop(page) {
		if (page > 4294967295 || page <= 0) throw 'Page out of bounds';

		if (page in this.workshopCache) {
			return Promise.resolve(this.workshopCache[page]);
		} else {
			return invoke('workshopBrowser', { page }).then(data => {
				this.workshopCache[page] = data;
				return data;
			});
		}
	}

	browseGame(page) {
		if (page > 4294967295 || page <= 0) throw 'Page out of bounds';

		if (page in this.gameAddonsCache) {
			return Promise.resolve(this.gameAddonsCache[page]);
		} else {
			return invoke('gameAddonsBrowser', { page }).then(data => {
				this.gameAddonsCache[page] = data;
				return data;
			});
		}
	}

	checkGMAQueue() {
		const next = this.gmaMetadataQueue.queue[0];
		if (next) {
			const [[path, id], resolve, reject] = next;

			invoke('gmaMetadata', { path, id })

				.then(metadata => {

					this.gmaMetadataCache[path] = [true, metadata];

					delete this.gmaMetadataQueue.waiting[path];
					this.gmaMetadataQueue.queue.splice(0, 1);

					window.setTimeout(this.checkGMAQueue.bind(this), 0); // prevents stack overflow

					return metadata;

				}, error => {
					this.gmaMetadataCache[path] = [false, error];
					return error;
				})

				.then(resolve, reject);
		}
	}

	getGMAMetadata(path, id) {
		if (path in this.gmaMetadataCache) {

			return this.gmaMetadataCache[path][0] ?
				Promise.resolve(this.gmaMetadataCache[path][1])
				:
				Promise.reject(this.gmaMetadataCache[path][1])

		} else {

			if (!(path in this.gmaMetadataQueue.waiting)) {
				this.gmaMetadataQueue.waiting[path] = new Promise((resolve, reject) => {
					if (this.gmaMetadataQueue.queue.push([[path, id], resolve, reject]) === 1) {
						this.checkGMAQueue();
					}
				});
			}

			return this.gmaMetadataQueue.waiting[path];

		}
	}

	getWorkshopMetadata(id) {
		if (!(id in this.workshopMetadataCache)) {
			this.workshopMetadataCache[id] = invoke({
				cmd: 'getWorkshopMetadata',
				id
			});
		}
		return this.workshopMetadataCache[id];
	}

	getWorkshopUploader(id) {
		if (!(id in this.workshopUploaderCache)) {
			this.workshopUploaderCache[id] = invoke({
				cmd: 'getWorkshopUploader',
				id
			});
		}
		return this.workshopUploaderCache[id];
	}

	previewGMA(path, id) {
		if (!(path in this.gmaPreviewCache))
			this.gmaPreviewCache[path] = invoke('previewGma', { path, id });

		return this.gmaPreviewCache[path];
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

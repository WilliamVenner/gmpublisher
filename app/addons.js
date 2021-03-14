import { promisified } from 'tauri/api/tauri'
import Transaction from './transactions.js'

class Addons {
	constructor() {
		this.pathsCached = false;
		this.paths = {};

		this.workshopCache = [];
		this.gameAddonsCache = [];

		this.gmaMetadataCache = {};
		this.gmaMetadataQueue = {
			queue: [],
			waiting: {}
		};
	}

	clearCache(cacheName) {
		this[cacheName + 'Cache'] = [];
	}

	browseWorkshop(page) {
		if (page > 4294967295 || page <= 0) throw 'Page out of bounds';
		
		if (page in this.workshopCache) {
			return Promise.resolve(this.workshopCache[page]);
		} else {
			return promisified({ cmd: 'workshopBrowser', page }).then(data => {
				this.workshopCache[page] = data;
				return data;
			});
		}
	}

	getGMAPaths() {
		if (this.pathsCached) return Promise.resolve();
		return new Promise(resolve => {
			promisified({ cmd: 'getGmaPaths' }).then(data => {
				this.pathsCached = true;
				this.paths = data;
				resolve();
			}, resolve);
		});
	}

	browseGame(page) {
		if (page > 4294967295 || page <= 0) throw 'Page out of bounds';

		if (page in this.gameAddonsCache) {
			return Promise.resolve(this.gameAddonsCache[page]);
		} else {
			return new Promise((resolve, reject) => {
				this.getGMAPaths().then(() => {
					promisified({ cmd: 'gameAddonsBrowser', page }).then(data => {
						if (data[2]) {
							this.paths = data[2]; // Intercept paths and save them
							this.pathsCached = true;
						}
						this.gameAddonsCache[page] = data;
						return data;
					}).then(resolve, reject);
				});
			});
		}
	}

	getAddonPath(id) {
		return this.paths[id];
	}

	checkGMAQueue() {
		const next = this.gmaMetadataQueue.queue[0];
		if (next) {
			const [id, resolve, reject] = next;

			promisified({ cmd: 'gmaMetadata', id })

				.then(metadata => {
					
					this.gmaMetadataCache[id] = [true, metadata];

					delete this.gmaMetadataQueue.waiting[id];
					this.gmaMetadataQueue.queue.splice(0, 1);

					window.setTimeout(this.checkGMAQueue.bind(this), 0); // prevents stack overflow

					return metadata;

				}, error => {
					this.gmaMetadataCache[id] = [false, error];
					return error;
				})
				
				.then(resolve, reject);
		}
	}

	getGMAMetadata(id) {
		if (!(id in this.paths)) return Promise.reject(id);

		if (id in this.gmaMetadataCache) {
			
			return this.gmaMetadataCache[id][0] ?
				Promise.resolve(this.gmaMetadataCache[id][1])
				:
				Promise.reject(this.gmaMetadataCache[id][1])

		} else {

			if (!(id in this.gmaMetadataQueue.waiting)) {
				this.gmaMetadataQueue.waiting[id] = new Promise((resolve, reject) => {
					if (this.gmaMetadataQueue.queue.push([id, resolve, reject]) === 1) {
						this.checkGMAQueue();
					}
				});
			}

			return this.gmaMetadataQueue.waiting[id];

		}
	}

	openGMA(path) {
		return promisified({ cmd: 'openAddon', path }).then(transactionId => new Transaction(transactionId));
	}
}

const addons = new Addons();
export default addons;
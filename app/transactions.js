import { invoke } from '@tauri-apps/api/tauri';
import { listen } from '@tauri-apps/api/event';
import { writable } from 'svelte/store';

let transactions = {};

const tasks = writable([]);
const tasksNum = writable(0);
const taskHeight = 49;
const tasks1080p = 4;
const ratio = (taskHeight * tasks1080p) / 1027;
const tasksMax = writable(4);
function resize() {
	tasksMax.set(Math.max(
		Math.round(
			(window.outerHeight * ratio) / taskHeight
		),
		2
	));
}
window.addEventListener('resize', resize);
resize();

let orphanQueue = [];
let orphanedTransactions = {};
function checkOrphanQueue(transaction, id) {
	let i = 0;
	while (i < orphanQueue.length) {
		const orphan = orphanQueue[i];
		if (orphan[1][0] === id) {
			orphanQueue.splice(i, 1);
			orphan[1][0] = transaction;
			fireTransactionEvent(orphan[0], orphan[1]);
		} else {
			i++;
		}
	}
}

const dedup = {};

class Transaction {
	constructor(id, TASK_statusTextFn) {
		if (id === null || id == undefined) return;

		if (id in dedup) {
			console.log('DUPLICATE TRANSACTION ID: ' + id);
		} else {
			dedup[id] = true;
		}

		this.id = id;
		this.callbacks = [];
		this.progress = 0;
		this.finished = false;
		this.cancelled = false;
		this.unconsumedEvents = [];

		transactions[id] = this;

		if (TASK_statusTextFn)
			tasks.update(tasks => { tasks.push([this, TASK_statusTextFn, null]); return tasks });

		if (id in orphanedTransactions) {
			delete orphanedTransactions[id];
			checkOrphanQueue(this, id);
		}
	}

	static get(id) {
		return transactions[id];
	}

	listen(callback) {
		this.callbacks.push(callback);

		if (this.callbacks.length === 1) {
			for (let i = 0; i < this.unconsumedEvents.length; i++) {
				callback(this.unconsumedEvents[i]);
			}
		}

		return this;
	}

	emit(event) {
		if (this.callbacks.length === 0) {
			this.unconsumedEvents.push(event);
		} else {
			for (let i = 0; i < this.callbacks.length; i++) {
				this.callbacks[i](event);
			}
		}
		return this;
	}

	cancel(fromBackend) {
		if (this.cancelled || this.finished) return;

		this.cancelled = true;
		this.emit({ cancelled: true });
		delete transactions[this.id];

		if (!fromBackend) {
			invoke('cancel_transaction', {
				id: this.id
			});
		}

		return this;
	}

	setFinished(data) {
		this.finished = true;
		if (this.progress < 100) {
			this.progress = 100;
			this.emit({ progress: 100 });
		}
		this.emit({ finished: true, data });
		delete transactions[this.id];

		return this;
	}

	setError(msg, data) {
		this.error = [msg, data];
		this.emit({ error: msg, data });
		delete transactions[this.id];

		return this;
	}

	setData(data) {
		this.emit({ stream: true, data });
		return this;
	};

	setStatus(msg) {
		this.status = msg;
		this.emit({ msg });

		return this;
	}

	setProgress(progress) {
		if (progress !== this.progress) {
			this.emit({ progress });
			this.progressInt = progress;
			this.progress = progress / 100;
		}

		return this;
	}
}

let transactionEvents = {};
function fireTransactionEvent(event, data) {
	transactionEvents[event](data);
}
function receiveTransactionEvent(event, data) {
	const transaction = Transaction.get(data[0]);
	if (transaction) {
		data[0] = transaction;
		fireTransactionEvent(event, data);
	} else {
		orphanedTransactions[data[0]] = true;
		orphanQueue.push([event, data]);
	}
}
function transactionEvent(event, callback) {
	transactionEvents[event] = callback;
	listen('Transaction' + event, ({ payload: data }) => {
		receiveTransactionEvent(event, data);
	});
}

transactionEvent('Progress', ([ transaction, progress ]) => {
	if (progress > (transaction.progressInt ?? 0)) transaction.setProgress(progress);
});

transactionEvent('Cancelled', ([ transaction ]) => {
	//console.log('transactionCancelled', transaction);
	transaction.cancel(true);
});

transactionEvent('Finished', ([ transaction, data ]) => {
	//console.log('transactionFinished', transaction, data);
	transaction.setFinished(data);
});

transactionEvent('Error', ([ transaction, [ msg, data ] ]) => {
	//console.log('transactionError', transaction, msg, data);
	transaction.setError(msg, data);
});

transactionEvent('Status', ([ transaction, msg ]) => {
	//console.log('transactionStatus', transaction, msg);
	transaction.setStatus(msg);
});

transactionEvent('Data', ([ transaction, data ]) => {
	//console.log('transactionData', data);
	transaction.setData(data);
});

invoke('websocket').then(port => {
	const decoder = new TextDecoder('utf-8');
	const read_nt_string = (byteOffset, view) => {
		const buffer = [];
		let i = byteOffset + 1;
		for (i; i < view.byteLength; i++) {
			const byte = view.getUint8(i);
			if (byte === 0) {
				i--;
				break;
			} else {
				buffer.push(byte);
			}
		}
		return [decoder.decode(new Uint8Array(buffer)), i];
	};
	const read_json = (byteOffset, view) => {
		const data = read_nt_string(byteOffset, view);
		data[0] = JSON.parse(data[0]);
		return data;
	};

	const socket = new WebSocket('ws://localhost:' + port, 'gmpublisher');
	socket.binaryType = 'arraybuffer';
	socket.addEventListener('message', event => {
        const view = new DataView(event.data);
		const message = view.getUint8(0);
        const id = view.getUint32(1);

		switch(message) {
			case 0:
			{
				const [data, _] = read_json(5, view);
				receiveTransactionEvent('Finished', [id, data]);
			}
			break;

			case 1:
			{
				const [msg, i] = read_nt_string(5, view);
				const [data, _] = read_json(i, view);
				receiveTransactionEvent('Error', [id, msg, data]);
			}
			break;

			case 2:
			{
				const [data, _] = read_json(5, view);
				receiveTransactionEvent('Data', [id, data]);
			}
			break;

			case 3:
			{
				const [status, _] = read_nt_string(5, view);
				receiveTransactionEvent('Status', [id, status]);
			}
			break;

			case 4:
			receiveTransactionEvent('Progress', [id, view.getUint16(5)]);
			break;

			case 5:
			receiveTransactionEvent('IncrProgress', [id, view.getUint16(5)]);
			break;
		}
	});
});

export { Transaction, tasks, taskHeight, tasksMax, tasksNum }

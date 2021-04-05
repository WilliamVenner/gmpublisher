import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { writable } from 'svelte/store';

let transactions = [];

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
function checkOrphanQueue(id) {
	let i = 0;
	while (i < orphanQueue.length) {
		const orphan = orphanQueue[i];
		if (orphan[1][0] === id) {
			orphanQueue.splice(i, 1);
			fireTransactionEvent(orphan[0], orphan[1]);
		} else {
			i++;
		}
	}
}

class Transaction {
	constructor(id, TASK_statusTextFn) {
		if (id === null || id == undefined) return;

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
			checkOrphanQueue(id);
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
			invoke('cancelTransaction', {
				id: this.id
			});
		}

		return this;
	}

	finish(data) {
		this.finished = true;
		if (this.progress < 100) {
			this.progress = 100;
			this.emit({ progress: 100 });
		}
		this.emit({ finished: true, data });
		delete transactions[this.id];

		return this;
	}

	error(msg, data) {
		this.error = [msg, data];
		this.emit({ error: msg, data });
		delete transactions[this.id];

		return this;
	}

	data(data) {
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
			this.progress = progress;
		}

		return this;
	}
}

let transactionEvents = {};
function fireTransactionEvent(event, data) {
	transactionEvents[event](data);
}
function transactionEvent(event, callback) {
	transactionEvents[event] = callback;
	
	listen('Transaction' + event, (data) => {
		console.log(data);
		const id = data[0];
		const transaction = Transaction.get(id);
		if (transaction) {
			callback(data);
		} else {
			orphanedTransactions[data[0]] = true;
			orphanQueue.push([event, data]);
		}
	});
}

transactionEvent('Progress', ([ id, progress ]) => {
	const transaction = Transaction.get(id);
	var progress = Math.floor((progress + Number.EPSILON) * 10000) / 100;
	if (transaction && progress !== transaction.progress) transaction.setProgress(progress);
});

transactionEvent('Cancelled', id => {
	const transaction = Transaction.get(id);
	console.log('transactionCancelled', transaction);
	if (transaction) transaction.cancel(true);
});

transactionEvent('Finished', ([ id, data ]) => {
	const transaction = Transaction.get(id);
	console.log('transactionFinished', transaction, data);
	if (transaction) transaction.finish(data);
});

transactionEvent('Error', ([ id, [ msg, data ] ]) => {
	const transaction = Transaction.get(id);
	console.log('transactionError', transaction, data);
	if (transaction) transaction.error(msg, data);
});

transactionEvent('ProgressMsg', ([ id, msg ]) => {
	const transaction = Transaction.get(id);
	console.log('transactionProgressMsg', transaction, msg);
	if (transaction) transaction.setStatus(msg);
});

transactionEvent('Data', ([ id, data ]) => {
	const transaction = Transaction.get(id);
	console.log('transactionData', data);
	if (transaction) transaction.data(data);
});

export { Transaction, tasks, taskHeight, tasksMax, tasksNum }

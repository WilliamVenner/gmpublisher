import { invoke } from 'tauri/api/tauri'
import { listen } from 'tauri/api/event'
import { writable } from 'svelte/store';

let transactions = [];

const tasks = writable([]);

class Transaction {
	constructor(id, asTask) {
		this.id = id;
		this.callbacks = [];
		this.progress = 0;
		this.finished = false;
		this.cancelled = false;

		transactions[id] = this;
		$tasks = [this, ...$tasks];
	}

	static get(id) {
		return transactions[id];
	}

	listen(callback) {
		this.callbacks.push(callback);
		return this;
	}

	emit(event) {
		for (let i = 0; i < this.callbacks.length; i++)
			this.callbacks[i](event);
		return this;
	}

	cancel(fromBackend) {
		if (this.cancelled) return;

		this.cancelled = true;
		this.emit({ cancelled: true });
		delete transactions[this.id];

		if (!fromBackend) {
			invoke({
				cmd: 'cancelTransaction',
				id: this.id
			});
		}

		return this;
	}

	finish(data) {
		this.finished = true;
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

	setProgress(progress) {
		this.progress = progress;
		this.emit({ progress });

		return this;
	}
}

listen("transactionProgress", ({ payload: [ id, progress ] }) => {
	const transaction = Transaction.get(id);
	var progress = Math.floor((progress + Number.EPSILON) * 100) / 100;
	if (transaction && progress !== transaction.progress) transaction.setProgress(progress);
});

listen("transactionCancelled", ({ payload: id }) => {
	const transaction = Transaction.get(id);
	if (transaction) transaction.cancel(true);
});

listen("transactionFinished", ({ payload: [ id, data ] }) => {
	const transaction = Transaction.get(id);
	if (transaction) transaction.finish(data);
});

listen("transactionError", ({ payload: [ id, [ msg, data ] ] }) => {
	const transaction = Transaction.get(id);
	if (transaction) transaction.error(msg, data);
});

export { Transaction, tasks }

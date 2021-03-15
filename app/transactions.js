import { invoke } from 'tauri/api/tauri'
import { listen } from 'tauri/api/event'

let transactions = [];

class Transaction {
	constructor(id) {
		this.id = id;
		this.callbacks = [];
		this.progress = 0;
		this.cancelled = false;

		transactions[id] = this;
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
		this.emit({ finished: true, data });
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

export default Transaction;

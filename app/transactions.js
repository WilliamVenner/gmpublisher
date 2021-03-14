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
	}

	emit(event) {
		for (let i = 0; i < this.callbacks.length; i++)
			this.callbacks[i](event);
	}

	cancel(fromBackend) {
		this.cancelled = true;
		this.emit({
			progress: this.progress,
			cancelled: true,
			finished: this.progress >= 100
		});
		delete transactions[this.id];

		if (!fromBackend) {
			invoke({
				cmd: 'cancelTransaction',
				id: this.id
			});
		}
	}

	setProgress(progress) {
		this.progress = progress;
		this.emit({
			progress,
			cancelled: this.cancelled,
			finished: progress >= 100
		});
	}
}

listen("transactionProgress", ({ payload: [ id, progress ] }) => {
	const transaction = Transaction.get(id);
	if (transaction) transaction.setProgress(progress);
});

listen("transactionCancelled", id => {
	const transaction = Transaction.get(id);
	if (transaction) transaction.cancel(true);
});

export default Transaction;

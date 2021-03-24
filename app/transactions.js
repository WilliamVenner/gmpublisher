import { invoke } from 'tauri/api/tauri'
import { listen } from 'tauri/api/event'
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

class Transaction {
	constructor(id, TASK_statusTextFn) {
		if (id === null || id == undefined) return;

		this.id = id;
		this.callbacks = [];
		this.progress = 0;
		this.finished = false;
		this.cancelled = false;

		transactions[id] = this;

		if (TASK_statusTextFn)
			tasks.update(tasks => { tasks.push([this, TASK_statusTextFn, null]); return tasks });
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
		if (this.cancelled || this.finished || this.error) return;

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

listen("transactionProgress", ({ payload: [ id, progress ] }) => {
	const transaction = Transaction.get(id);
	var progress = Math.floor((progress + Number.EPSILON) * 100000) / 1000;
	if (transaction && progress !== transaction.progress) transaction.setProgress(progress);
});

listen("transactionCancelled", ({ payload: id }) => {
	const transaction = Transaction.get(id);
	console.log('transactionCancelled', transaction);
	if (transaction) transaction.cancel(true);
});

listen("transactionFinished", ({ payload: [ id, data ] }) => {
	const transaction = Transaction.get(id);
	console.log('transactionFinished', transaction, data);
	if (transaction) transaction.finish(data);
});

listen("transactionError", ({ payload: [ id, [ msg, data ] ] }) => {
	const transaction = Transaction.get(id);
	console.log('transactionError', transaction, data);
	if (transaction) transaction.error(msg, data);
});

listen("transactionProgressMsg", ({ payload: [ id, msg ] }) => {
	const transaction = Transaction.get(id);
	console.log('transactionProgressMsg', transaction, msg);
	if (transaction) transaction.setStatus(msg);
});

export { Transaction, tasks, taskHeight, tasksMax, tasksNum }

import { writable } from 'svelte/store';

const _modals = writable([]);

export const modals = _modals;
export const clearModals = e => {
	if (e.target.id === "modals") {
		e.stopPropagation();
		_modals.set([]);
	}
};
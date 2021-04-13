import { writable } from 'svelte/store';

export const notifications = writable([]);

export const NOTIFICATION_ALERT = 0;
export const NOTIFICATION_SUCCESS = 1;
export const NOTIFICATION_ERROR = 2;

export function pushNotification(notification) {
	switch(notification.type ?? 0) {
		case NOTIFICATION_ALERT:
			document.querySelector('#sound-alert').play();
			break;

		case NOTIFICATION_SUCCESS:
			document.querySelector('#sound-success').play();
			break;

		case NOTIFICATION_ERROR:
			document.querySelector('#sound-error').play();
			break;
	}

	$notifications.push(notification);
}

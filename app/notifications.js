import { writable } from 'svelte/store';

export const notifications = writable([]);

export const NOTIFICATION_SILENT = -1;
export const NOTIFICATION_ALERT = 0;
export const NOTIFICATION_SUCCESS = 1;
export const NOTIFICATION_ERROR = 2;

function stop(audio) {
	audio.pause();
	audio.currentTime = 0;
}

export function pushNotification(notification) {
	switch(notification.type ?? -1) {
		case NOTIFICATION_SILENT: break;

		case NOTIFICATION_ALERT:
			document.querySelector('#sound-alert').play();
			break;

		case NOTIFICATION_SUCCESS:
			stop(document.querySelector('#sound-alert'));
			document.querySelector('#sound-success').play();
			break;

		case NOTIFICATION_ERROR:
			stop(document.querySelector('#sound-alert'));
			stop(document.querySelector('#sound-success'));
			document.querySelector('#sound-error').play();
			break;
	}

	notifications.update(notifications => {
		notifications.push(notification);
		return notifications;
	});
}

import { writable } from 'svelte/store';
import { playSound, stopSound } from './sounds';
import * as notification from '@tauri-apps/api/notification';

export const NOTIFICATION_SILENT = -1;
export const NOTIFICATION_ALERT = 0;
export const NOTIFICATION_SUCCESS = 1;
export const NOTIFICATION_ERROR = 2;

export const notifications = writable([]);
export const enabled = writable(null);

let enabled_ = null;
enabled.subscribe(val => enabled_ = val);

export function pushNotification(options) {
	if (AppSettings && AppSettings.notification_sounds) {
		switch(options.type ?? -1) {
			case NOTIFICATION_SILENT: break;

			case NOTIFICATION_ALERT:
				playSound('alert');
				break;

			case NOTIFICATION_SUCCESS:
				stopSound('alert');
				playSound('success');
				break;

			case NOTIFICATION_ERROR:
				stopSound('alert');
				stopSound('success');
				playSound('error');
				break;
		}
	}

	notifications.update(notifications => {
		notifications.push(options);
		return notifications;
	});

	if (enabled_ && options.desktop != false && document.visibilityState !== 'visible') {
		notification.sendNotification({
			title: options.title,
			body: options.body,
			icon: options.icon
		});
	}
}

export function appSettingsCallback() {
	if (AppSettings.desktop_notifications) {
		notification.isPermissionGranted().then(granted => {
			enabled.set(granted);
		});
	}
}

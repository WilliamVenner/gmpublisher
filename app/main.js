import { appSettingsCallback } from './notifications';

__GMPUBLISHER__(() => {
	appSettingsCallback();
});

import { default as i18n } from './i18n.js'; i18n();

import App from './App.svelte';
var app = new App({
	target: document.body
});

export default app;

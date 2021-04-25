import { default as i18n, switchLanguage } from './i18n.js';
import App from './App.svelte';

__GMPUBLISHER__(() => {
	if (AppSettings.language) switchLanguage(AppSettings.language);
});

i18n();

var app = new App({
	target: document.body
});

export default app;

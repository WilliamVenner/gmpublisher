__GMPUBLISHER__();

import gmpublisher from './gmpublisher.js';
new gmpublisher();

import { default as i18n } from './i18n.js'; i18n();
import './dragndrop.js';

import App from './App.svelte';
const app = new App({
	target: document.body,
	props: {
		AppData
	}
});

export default app;
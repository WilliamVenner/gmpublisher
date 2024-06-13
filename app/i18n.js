import { _, addMessages, init, getLocaleFromNavigator, locale } from 'svelte-i18n';
import { get } from 'svelte/store';
import en from '../i18n/en.json';

window.APP_LANGUAGES = JSON.parse(__GMPUBLISHER_APP_LANGUAGES__);

{
	for (let file in window.APP_LANGUAGES) {
		if (file === 'en') continue;
		addMessages(file, window.APP_LANGUAGES[file]);
	}
	addMessages('en', en);

	const navigatorLocale = getLocaleFromNavigator() ?? 'en';
	init({
		fallbackLocale: 'en',
		initialLocale: navigatorLocale,
	});

	console.report('info', `Initial locale: ${navigatorLocale}`);
}

const RE_SPLIT_ERROR = /^(.*?)(?::([\s\S]*))?$/;
export function translateError(error, data) {
	if (data != null) {
		return get(_)(error, { values: { data: data.toString() } });
	} else {
		const match = error.match(RE_SPLIT_ERROR);
		if (!match) {
			return get(_)(error);
		} else if (match[2]) {
			return get(_)(match[1], { values: { data: match[2] } });
		} else {
			return get(_)(match[1]);
		}
	}
}

export function switchLanguage(switchLocale) {
	const newLocale = switchLocale in window.APP_LANGUAGES ? switchLocale : (getLocaleFromNavigator() ?? 'en');
	locale.set(newLocale);
	console.report('info', `Switched to locale: ${newLocale}`);
}

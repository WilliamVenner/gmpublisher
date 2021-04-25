import { _, addMessages, init as i18n_init, getLocaleFromNavigator, register, locale } from 'svelte-i18n';
import { get } from 'svelte/store';
import en from '../i18n/en.json';

for (let file in APP_LANGUAGES) {
	register(file, () => import(`../i18n/${file}.json`));
}

export default () => {
	addMessages('en', en);

	i18n_init({
		fallbackLocale: 'en',
		initialLocale: getLocaleFromNavigator(),
	});
};

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

export function switchLanguage(newLocale) {
	if (newLocale in APP_LANGUAGES) {
		locale.set(newLocale);
	} else {
		locale.set(getLocaleFromNavigator());
	}
}

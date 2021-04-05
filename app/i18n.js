import { addMessages, init as i18n_init, getLocaleFromNavigator } from 'svelte-i18n';
import en from '../i18n/en.json';

export default () => {
	addMessages('en', en);

	i18n_init({
		fallbackLocale: 'en',
		initialLocale: getLocaleFromNavigator(),
	});
};
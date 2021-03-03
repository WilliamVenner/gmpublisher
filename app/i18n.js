import { derived } from 'svelte/store';
import { addMessages, register, init as i18n_init, getLocaleFromNavigator, _ as interpolate } from 'svelte-i18n';
import en from '../i18n/en.json';

export default () => {
	addMessages('en', en);

	//register('en-US', () => import('../i18n/en-US.json'));

	i18n_init({
		fallbackLocale: 'en',
		initialLocale: getLocaleFromNavigator(),
	});
};
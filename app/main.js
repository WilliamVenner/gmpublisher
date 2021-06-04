import { switchLanguage } from './i18n.js';
import App from './App.svelte';

function rgbToHsl(r, g, b) {
	r /= 255, g /= 255, b /= 255;

	var max = Math.max(r, g, b), min = Math.min(r, g, b);
	var h, s, l = (max + min) / 2;

	if (max == min) {
		h = s = 0; // achromatic
	} else {
		var d = max - min;
		s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

		switch (max) {
			case r: h = (g - b) / d + (g < b ? 6 : 0); break;
			case g: h = (b - r) / d + 2; break;
			case b: h = (r - g) / d + 4; break;
		}

		h /= 6;
	}

	return [ h, s, l ];
}

window.updateCustomColor = function(name, colorInt) {
	const root = document.documentElement;
	const varName = `--${name}-custom`;

	let h, s, l;
	{
		colorInt >>>= 0;
		const b = colorInt & 0xFF,
			  g = (colorInt & 0xFF00) >>> 8,
			  r = (colorInt & 0xFF0000) >>> 16;

		[h, s, l] = rgbToHsl(r, g, b);
	}

	root.style.setProperty(varName, `${h * 360}, ${s * 100}%`);
	root.style.setProperty(varName + '-l', (l * 100) + '%');
}

__GMPUBLISHER__(() => {
	if (AppSettings.language) switchLanguage(AppSettings.language);

	updateCustomColor('neutral', AppSettings.color_neutral);
	updateCustomColor('success', AppSettings.color_success);
	updateCustomColor('error', AppSettings.color_error);
});

var app = new App({
	target: document.body
});

export default app;

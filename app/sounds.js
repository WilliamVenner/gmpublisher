const audio = {
	'alert': new Audio('/sound/alert.wav'),
	'success': new Audio('/sound/success.wav'),
	'error': new Audio('/sound/error.wav'),
	'btn-on': new Audio('/sound/btn_on.ogg'),
	'btn-off': new Audio('/sound/btn_off.ogg'),
};

for (let sound in audio) {
	audio[sound].load();
}

export function playSound(sound) {
	if (AppSettings && AppSettings.sounds) {
		audio[sound].play();
	}
}

export function stopSound(sound) {
	audio[sound].pause();
	audio[sound].currentTime = 0;
}

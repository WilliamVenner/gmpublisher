const updateSettings = () => {
	__TAURI__.tauri.invoke({
		cmd: 'updateSettings',
		settings: JSON.stringify(window.AppSettings)
	}); // TODO

	console.log('updateSettings');
};

const AppSettingsProxy = {
	set(data, key, val) {
		data[key] = val;
		if (window.__SETTINGS_TRANSACTION__) return true;

		updateSettings();

		return true;
	}
};

class AppSettings {
	static init(data) {
		return new Proxy(new AppSettings(data), AppSettingsProxy);
	}

	constructor(data) {
		for (const [key, val] of Object.entries(data)) {
			if (val && (val.constructor === Array || val.constructor === Object)) {
				this[key] = new Proxy(val, AppSettingsProxy);
			} else {
				this[key] = val;
			}
		}
	}

	begin() {
		window.__SETTINGS_TRANSACTION__ = true;
	}

	commit() {
		delete window.__SETTINGS_TRANSACTION__;
		updateSettings();
	}
}

window.__GMPUBLISHER__ = () => {
	__TAURI__.tauri.invoke('free_caches');

	{
		const AppDataPtr = {};
		window.AppData = new Proxy(AppDataPtr, {
			get: function(_, key) { return _._[key]; }
		});

		function updateAppData(newAppData) {
			console.log('UpdateAppData');
			console.log(newAppData);

			const settings = AppSettings.init(newAppData.settings);
			window.AppSettings = settings;

			delete newAppData.settings;
			AppDataPtr._ = Object.freeze(newAppData);
		}

		updateAppData(JSON.parse('{$_APP_DATA_$}'));
		__TAURI__.event.listen('UpdateAppData', ({ payload }) => updateAppData(payload));
	}

	{
		__TAURI__.event.listen('tauri://file-drop', ({ payload: path }) => {
			document.body.classList.remove('file-drop');
			console.log('File Drop', path);
		});

		__TAURI__.event.listen('tauri://file-drop-hover', ({ payload: path }) => {
			document.body.classList.add('file-drop');
			console.log('File Drop Hover', path);
		});

		__TAURI__.event.listen('tauri://file-drop-cancelled', ({ payload: path }) => {
			document.body.classList.remove('file-drop');
			console.log('File Drop Cancelled', path);
		});
	}
};

window.__WS_DEAD__ = JSON.parse('{$_WS_DEAD_$}');
window.__WS_DEAD__.dead = true;
delete window.__WS_DEAD__.id;
delete window.__WS_DEAD__.title;
delete window.__WS_DEAD__.searchTitle;
delete window.__WS_DEAD__.localFile;

window.PATH_SEPARATOR = {$_PATH_SEPARATOR_$};

let resizeTimeout;

function isMaximized() {
	return window.innerWidth - window.outerWidth === 0 && window.innerHeight - window.outerHeight === 0;
}
function resized(e) {
	console.log(e);
	/*window.__TAURI__.invoke("windowResize", {
		maximized: isMaximized(),
		width: window.outerWidth,
		height: window.outerHeight
	});*/
}
window.addEventListener('resize', e => {
	clearTimeout(resizeTimeout);
	resizeTimeout = setTimeout(resized, 100, e);
});

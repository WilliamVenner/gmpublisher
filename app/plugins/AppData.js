const updateSettings = () => {
	__TAURI__.tauri.invoke({
		cmd: 'updateSettings',
		settings: JSON.stringify(window.AppSettings)
	});

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
	const AppDataPtr = {};
	window.AppData = new Proxy(AppDataPtr, {
		get: function(_, key) { return _._[key]; }
	});

	function updateAppData(AppData) {
		const settings = AppSettings.init(AppData.settings);
		window.AppSettings = settings;

		delete AppData.settings;
		AppDataPtr._ = Object.freeze(AppData);

		window.PATH_SEPARATOR = AppData.path_separator;
	}

	updateAppData(JSON.parse(String.raw`{$_SETTINGS_$}`));
	__TAURI__.event.listen('updateAppData', ({ payload }) => updateAppData(payload)); // FIXME - why doesn't it work?
};
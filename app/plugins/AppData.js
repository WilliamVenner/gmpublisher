window.__GMPUBLISHER__ = appDataCallback => {
	__TAURI__.tauri.invoke('reloaded');

	// TODO
	/*{
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

		window.addEventListener('drop', e => {
			console.log('JS File Drop (drop)', e);
		});

		window.addEventListener('dragstart', e => {
			console.log('JS File Drop Hover (dragstart)', e);
		});

		window.addEventListener('dragend', e => {
			console.log('JS File Drop Cancelled (dragend)', e);
		});
	}*/

	{
		const AppDataPtr = {};
		window.AppData = new Proxy(AppDataPtr, {
			get: function(_, key) { return _._[key]; }
		});

		function updateAppData(newAppData) {
			console.log('UpdateAppData');
			console.log(newAppData);
			console.log(newAppData.settings);

			window.AppSettings = newAppData.settings;

			delete newAppData.settings;
			AppDataPtr._ = Object.freeze(newAppData);

			if (appDataCallback) appDataCallback();
		}

		updateAppData(JSON.parse('{$_APP_DATA_$}'));
		__TAURI__.event.listen('UpdateAppData', ({ payload }) => updateAppData(payload));
	}
};

window.__WS_DEAD__ = JSON.parse('{$_WS_DEAD_$}');
window.__WS_DEAD__.dead = true;
delete window.__WS_DEAD__.id;
delete window.__WS_DEAD__.title;
delete window.__WS_DEAD__.searchTitle;
delete window.__WS_DEAD__.localFile;

window.PATH_SEPARATOR = {$_PATH_SEPARATOR_$};

window.DEFAULT_IGNORE_GLOBS = JSON.parse('{$_DEFAULT_IGNORE_GLOBS_$}');

let resizeTimeout;
function resized() {
	window.__TAURI__.invoke("window_resized", {
		width: window.innerWidth,
		height: window.innerHeight
	});
}
window.addEventListener('resize', e => {
	clearTimeout(resizeTimeout);
	resizeTimeout = setTimeout(resized, 100, e);
});

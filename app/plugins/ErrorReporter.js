window.DEBUG_MODE = {$_DEBUG_MODE_$};

console.log('Listening for errors');

(function() {

	const eprintln = window.console.error;

	function report(message, data) {
		const invoke = __TAURI__?.tauri?.invoke;

		if (invoke) invoke(message, data);
		else eprintln('Failed to report log to Tauri backend!');
	}

	function stringify(...data) {
		let stringified = '';
		for (let i = 0; i < data.length - 1; i++) {
			stringified += JSON.stringify(data[i], null, 4) + '\t';
		}
		stringified += JSON.stringify(data[data.length - 1], null, 4);
		return stringified;
	}

	if (DEBUG_MODE) {
		window.console.warn = (...data) => {
			report('warn', { message: stringify(...data) });
		};

		window.console.log = (...data) => {
			report('info', { message: stringify(...data) });
		};
	}

	window.console.error = (...data) => {
		report('error', { message: stringify(...data) });
	};

	window.addEventListener('error', e => {
		report('js_error', {
			message: e.error.message,
			stack: e.error.stack
		});
	});

})();

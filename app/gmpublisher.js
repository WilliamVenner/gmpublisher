class gmpublisher {
	constructor() {
		this.initWindowResize();
	}

	initWindowResize() {
		let timeout;

		const update = () => {
			if (AppSettings.window_size[0] === window.outerWidth && AppSettings.window_size[1] === window.outerHeight) return;
			
			AppSettings.begin();
			AppSettings.window_size[0] = window.outerWidth;
			AppSettings.window_size[1] = window.outerHeight;
			AppSettings.commit();
		};

		window.addEventListener('resize', () => {
			if (timeout || timeout === 0) window.clearInterval(timeout);
			timeout = window.setTimeout(update, 500);
		});
	}
}

export default gmpublisher;
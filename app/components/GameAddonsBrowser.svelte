<script>
	import AddonsBrowser from '../components/AddonsBrowser.svelte';
	import Addons from '../addons.js';

	let page = 1;
	let loadingPage = null;

	function firstPage() {
		page = 1;
		return Addons.browseGame(1);
	}

	function advancePage() {
		if (loadingPage !== null) return;

		page += 1;
		loadingPage = page;

		return new Promise((resolve, reject) => {
			Addons.browseGame(page).then(resolve, reject).finally(() => loadingPage = null);
		});
	}
</script>

<AddonsBrowser firstPage={firstPage} advancePage={advancePage} cacheName="gameAddons"/>
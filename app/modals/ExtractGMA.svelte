<script>
	import Addons from '../addons.js';

	export let path;

	let progress = 0;
	let gma = new Promise((resolve, reject) => {
		Addons.previewGMA(path).then(transaction => {
			transaction.listen(event => {
				if ("progress" in event) {
					progress = event.progress;
				} else if (event.finished) {
					resolve(event.data);
				} else if (event.error) {
					reject(event.error);
				}
			});
		});
	});
</script>

<div id="gma-preview" class="modal">
	{#await gma}
		<div id="loading">
			<img src="/img/logo.svg" id="logo" alt="Logo"/>
			Progress {progress}%
		</div>
	{:then gma}
		{#each gma.entries as entry}
			{entry}
		{/each}
	{:catch error}
		<div id="error">{error}</div>
	{/await}
</div>

<style>
	
</style>
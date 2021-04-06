<script context="module">

	export const pages = [
		{
			name: 'my_workshop',
			component: MyWorkshop,
		},
		{
			name: 'game',
			component: GameAddons,
		},
		{
			name: 'size_analyzer',
			component: AddonSizeAnalyzer,
		},
		{
			name: 'downloader',
			component: Downloader,
			persist: true,
		},
	];

	export let activePage = pages[0];

</script>

<script>
	import { _ } from 'svelte-i18n';

	import AddonSizeAnalyzer from '../pages/AddonSizeAnalyzer.svelte';
	import Downloader from '../pages/Downloader.svelte';
	import GameAddons from '../pages/GameAddons.svelte';
	import MyWorkshop from '../pages/MyWorkshop.svelte';

	function selectPage(e) {
		const page = e.target.dataset.page;
		pages[page].created = true;
		activePage = pages[page];
	}
</script>

<main>
	<div>
		{#each pages as choice, i}
			<div class:active={ activePage.name === choice.name } on:click={selectPage} data-page={i}>{$_(choice.name)}</div>
		{/each}
	</div>

	<div id="credits">
		<img src="/img/logo.svg" alt="Logo" id="logo" /><br>
		gmpublisher v{AppData.version} by Billy<br>
		<a href="https://github.com/WilliamVenner/gmpublisher/stargazers" target="_blank">{$_('github_star')}</a>&nbsp;🤩
	</div>
</main>

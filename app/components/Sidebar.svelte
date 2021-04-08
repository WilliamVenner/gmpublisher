<script context="module">

	export const pages = [
		{
			name: 'my_workshop',
			component: MyWorkshop,
		},
		{
			name: 'installed_addons',
			component: InstalledAddons,
		},
		{
			name: 'downloader',
			component: Downloader,
			persist: true,
		},
		{
			name: 'size_analyzer',
			component: AddonSizeAnalyzer,
		},
		{
			name: 'content_generator',
			component: ContentGenerator,
		},
	];

	export let activePage = writable(pages[0]);

</script>

<script>
	import { writable } from 'svelte/store';
	import { _ } from 'svelte-i18n';

	import AddonSizeAnalyzer from '../pages/AddonSizeAnalyzer.svelte';
	import Downloader from '../pages/Downloader.svelte';
	import InstalledAddons from '../pages/InstalledAddons.svelte';
	import MyWorkshop from '../pages/MyWorkshop.svelte';
	import ContentGenerator from '../pages/ContentGenerator.svelte';

	function selectPage(e) {
		const page = e.target.dataset.page;
		pages[page].created = true;
		$activePage = pages[page];
	}
</script>

<sidebar>
	<div>
		{#each pages as choice, i}
			<div class:active={ $activePage.name === choice.name } on:click={selectPage} data-page={i}>{$_(choice.name)}</div>
		{/each}
	</div>

	<div id="credits">
		<img src="/img/logo.svg" alt="Logo" id="logo" /><br>
		gmpublisher v{AppData.version} by Billy<br>
		<a href="https://github.com/WilliamVenner/gmpublisher/stargazers" target="_blank">{$_('github_star')}</a>&nbsp;🤩
	</div>
</sidebar>

<style>
	sidebar {
		position: fixed;
		padding: 1.5rem;
		padding-right: 0;
		width: 26.04%;
		max-width: 250px;
		height: calc(100% - 70px);
		top: 70px;
		left: 0;
		display: flex;
		flex-direction: column;
	}
	sidebar > div:first-child {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
	}
	sidebar > div:first-child > div {
		padding: .5rem;
		padding-left: .7rem;
		padding-right: .7rem;
		cursor: pointer;
		border-radius: 4px;
	}
	sidebar > div:first-child > div.active {
		background-color: #2A2A2A;
	}
	sidebar > div:first-child > div:not(:last-child) {
		margin-bottom: .5rem;
	}

	#credits {
		text-align: center;
		font-size: .8rem;
		padding-top: 1rem;
		padding-bottom: 1rem;
		line-height: 1.8em;
	}
	#credits a {
		color: #636363;
		transition: color .25s;
	}
	#credits a:hover {
		color: #fff;
	}

	#logo {
		margin-bottom: .5rem;
		width: min(calc(100% - 1rem), 50px);
	}
</style>

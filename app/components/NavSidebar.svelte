<script context="module">

	const pagesManifest = [
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
			background: true
		},
		{
			name: 'size_analyzer',
			component: AddonSizeAnalyzer,
		},
		/*
		{
			name: 'subscriptions',
			component: Subscriptions,
		},
		{
			name: 'content_generator',
			component: ContentGenerator,
		},
		*/
	];

	export const pages = writable(pagesManifest);

	export const activeItem = writable(0);

</script>

<script>
	import { writable } from 'svelte/store';
	import { _ } from 'svelte-i18n';
	import Sidebar from './Sidebar.svelte';

	import AddonSizeAnalyzer from '../pages/AddonSizeAnalyzer.svelte';
	import Downloader from '../pages/Downloader.svelte';
	import InstalledAddons from '../pages/InstalledAddons.svelte';
	import MyWorkshop from '../pages/MyWorkshop.svelte';
	import ContentGenerator from '../pages/ContentGenerator.svelte';
	import SidebarItem from './SidebarItem.svelte';
import Subscriptions from '../pages/Subscriptions.svelte';

	function selectPage(page) {
		$pages[page].created = true;
	}
</script>

<Sidebar id="nav-sidebar">
	{#each $pages as choice, i}
		<SidebarItem {activeItem} id={i} click={selectPage}>{$_(choice.name)}</SidebarItem>
	{/each}

	<div id="credits" slot="footer">
		<img src="/img/logo.svg" alt="Logo" id="logo" /><br>
		gmpublisher v{AppData.version} by Billy<br>
		<a tabindex="-1" href="https://github.com/WilliamVenner/gmpublisher/stargazers" target="_blank">{$_('github_star')}</a>&nbsp;🤩
	</div>
</Sidebar>

<style>
	:global(#nav-sidebar) {
		position: fixed;
		width: 26.04%;
		padding-right: 0;
		height: calc(100% - 70px);
		max-width: 250px;
		top: 70px;
		left: 0;
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

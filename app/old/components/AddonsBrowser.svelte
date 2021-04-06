<script>
	import { writable } from 'svelte/store';
	import { onDestroy } from 'svelte';
	import { _ } from 'svelte-i18n';
	import { ArrowCycle, Search, Cross, TriangleAlert } from 'akar-icons-svelte';
	import { Addons } from '../addons.js';
	import WorkshopAddon from './WorkshopAddon.svelte';
	import Dead from '../../public/img/dead.svg';

	export let cacheName;
	export let hasPaths = false;

	let refreshing = false;

	const STATUS_DEAD = -1;
	const STATUS_LOADING = 0;
	const STATUS_SUCCESS = 1;
	const STATUS_ERROR = 2;

	let totalAddons = 0;
	let status = STATUS_LOADING;
	let addons = [];
	
	export let advancePage;
	export let firstPage;
	const addonsLoading = writable(firstPage());

	let scroller; let scrollFade;
	function infiniScroll() {
		if (!scroller) scroller = document.querySelector('#addons-container');
		if (!scrollFade) scrollFade = document.querySelector('#scroll-fade');

		scrollFade.classList.toggle('active', scroller.scrollTop > 14);

		if (scroller.offsetHeight + scroller.scrollTop >= scroller.scrollHeight * .9) {
			if (addons.length < totalAddons) {
				let didAdvance = advancePage();
				if (didAdvance) addonsLoading.set(didAdvance);
			}
		}
	}

	let searchActive = false;
	let searching = false;
	let searchFailed = false;
	let searchTimeout;
	let searchAmount = null;

	function performSearch() {
		searchActive = true;
		searchAmount = 0;

		const term = document.querySelector('#search').value.trim().toLowerCase();
		let found = false;
		for (let i = 0; i < addons.length; i++) {
			const addon = hasPaths ? addons[i][1] : addons[i];
			const elems = document.querySelectorAll('.ws-' + addon.id);
			if (!!!elems) continue;

			for (let k = 0; k < elems.length; k++) {
				const elem = elems[k];

				const foundIndex = addon.searchTitle.indexOf(term);
				if (foundIndex !== -1) {

					elem.classList.remove('searching');
					found = true;

					const title = elem.querySelector('#title');
					title.textContent = addon.title.substr(0, foundIndex);

					const highlight = document.createElement('span');
					highlight.classList.add('highlight');
					highlight.textContent = addon.title.substr(foundIndex, term.length);
					title.appendChild(highlight);

					const text = document.createTextNode(addon.title.substr(foundIndex + term.length));
					title.appendChild(text);

					searchAmount++;

				} else {

					elem.classList.add('searching');

					const highlight = elem.querySelector('.highlight');
					if (highlight) {
						highlight.remove();
						elem.querySelector('#title').textContent = addon.title;
					}

				}
			}
		}
		searchFailed = !found;
	}

	function clearSearch() {
		clearTimeout(searchTimeout);

		searchAmount = null;
		searchFailed = false;
		searching = false;
		searchActive = false;
		
		const search = document.querySelector('#search');
		search.value = '';

		document.querySelectorAll('#workshop-addon').forEach(elem => elem.classList.remove('searching'));
		document.querySelectorAll('#workshop-addon .highlight').forEach(elem => {
			elem.insertAdjacentText('beforebegin', elem.textContent);
			elem.remove();
		});
	}

	function updateSearch(e, _term) {
		const term = _term || e.target.value.trim();
		if (term.length === 0) {
			clearTimeout(searchTimeout);
			searchActive = false;
			clearSearch();
		} else {
			if (!_term) {
				clearTimeout(searchTimeout);
				searchTimeout = window.setTimeout(updateSearch, 50, null, term);
			} else {
				performSearch();
			}
		}
	}
	function preloadSearch() {
		searching = false;
		const search = document.querySelector('#search');

		if (addons.length >= totalAddons || (!searchActive && document.activeElement !== search) || status === STATUS_DEAD) return;

		let didAdvance = advancePage();
		if (didAdvance) {
			searching = true;
			addonsLoading.set(didAdvance);
			didAdvance.then(data => {
				window.setTimeout(preloadSearch, 0);
				return data;
			}).finally(() => {
				if (status !== STATUS_DEAD && searchActive) updateSearch(null, search.value);
			});
		}
	}
	onDestroy(() => clearTimeout(searchTimeout));

	onDestroy(addonsLoading.subscribe(promise => {
		if (!promise) return;
		status = STATUS_LOADING;
		promise.then(([total, results]) => {
			refreshing = false;
			addons = addons.concat(results);
			totalAddons = total;
			status = STATUS_SUCCESS;
		}, error => {
			console.error(JSON.stringify(error));
			status = STATUS_ERROR;
		});
	}));

	function refresh() {
		refreshing = true;

		addons = [];
		totalAddons = 0;
		status = STATUS_LOADING;
		clearSearch();

		Addons.clearCache(cacheName);
		addonsLoading.set(firstPage());
	}

</script>

<main>
	<div id="top-controls">
		<div id="refresh" class:active={refreshing} on:click={refresh}>
			<ArrowCycle size="1.2rem"/>
		</div>
		<div id="search-container">
			<input type="text" id="search" placeholder={$_('search')} on:input={updateSearch} on:click={advancePage ? preloadSearch : null}/>
			{#if searchActive}
				<span id="cancel-search" on:click={clearSearch}><Cross size="1rem"/></span>
			{:else}
				<Search size="1rem"/>
			{/if}
		</div>
	</div>

	{#if (searchAmount && searchAmount !== totalAddons) || (addons.length !== 0 && totalAddons !== 0 && addons.length !== totalAddons)}
		<h2 id="amount">
			{#if searching}
				<img src="/img/loading.svg" alt="Loading" id="search-loading"/>&nbsp;
			{/if}
			<span>{$_('showing_num_addons', { values: { num: (searchAmount !== null ? searchAmount : addons.length).toLocaleString(), total: totalAddons.toLocaleString() }})}</span>
		</h2>
	{/if}
	<div id="addons-container" class:searching={searching} class="hide-scroll" on:scroll={advancePage ? infiniScroll : null}>
		<div id="scroll-fade"></div>

		{#if addons != false}
			<div id="addons" class:unpadded={searchActive && searchFailed}>
				{#each addons as addon}
					{#if hasPaths}
						<WorkshopAddon {...addon[1]} localFile={addon[0]}/>
					{:else}
						<WorkshopAddon {...addon}/>
					{/if}
				{/each}
			</div>
		{/if}

		{#if status === STATUS_LOADING}
			<div id="loading"><img src="/img/loading.svg" alt="Loading"/></div>
		{:else if status === STATUS_ERROR}
			<div id="error"><TriangleAlert color="red"/></div>
		{:else if searchActive && searchFailed}
			<div id="search-failed"><Dead/></div>
		{/if}
	</div>
</main>

<style>
	:global(#cancel-search) {
		cursor: pointer;
	}
	:global(#cancel-search .icon) {
		opacity: 1 !important;
	}

	#amount {
		margin-top: 0;
		margin-bottom: 1rem;
	}
	#amount > span, #search-loading {
		vertical-align: middle;
	}
	#search-loading {
		width: 24px;
	}

	#search-container {
		position: relative;
	}
	#search-container :global(.icon) {
		position: absolute;
		top: calc(50% - .5rem);
		left: 1rem;
		opacity: .3;
		vertical-align: initial !important;
	}
	#search {
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgba(0, 0, 0, .4);
		width: 100%;
		padding: 1rem;
		padding-left: 2.5rem;
		color: #fff;
	}
	#search:focus {
		outline: none;
		box-shadow: inset 0 0 0px 1.5px #127cff;
	}
	#search:focus + :global(.icon) {
		opacity: 1;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	#addons-container {
		flex: 1;
		overflow: auto;
		position: relative;
	}
	#addons-container #scroll-fade {
		position: -webkit-sticky;
		position: sticky;
		width: 100%;
		height: 0;
		top: 0;
		left: 0;
		box-shadow: 0 0 6px 8px #1a1a1a;
		display: block;
		z-index: 3;
		opacity: 0;
		transition: opacity .1s;
	}
	:global(#addons-container #scroll-fade.active) { opacity: 1 !important }
	#addons {
		position: relative;
		padding-bottom: 1.5rem;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		grid-auto-flow: dense;
		align-items: center;
		grid-gap: 1rem;

		filter: grayscale(0) blur(0);
		opacity: 1;

		transition: opacity .25s, filter .25s;
	}
	:global(#addons.unpadded) {
		padding-top: 0 !important;
		padding-bottom: 0 !important;
	}

	:global(#workshop-addon.searching) {
		display: none !important;
	}

	#loading, #error, #search-failed {
		text-align: center;
		margin-top: 1rem;
	}
	#loading img {
		opacity: .2;
	}

	#search-failed :global(svg) {
		color: #fff;
		opacity: .3;
	}

	#top-controls {
		display: grid;
		grid-template-columns: auto 1fr;
		grid-template-rows: 1fr;
		margin-bottom: 1.5rem;
	}
	#refresh {
		cursor: pointer;
		margin-right: 1rem;
		background: rgba(255, 255, 255, .1);
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .9rem;
	}
	#refresh :global(.icon) {
		opacity: .3;
		transition: opacity .25s;
	}
	:global(#refresh.active .icon), #refresh:hover :global(.icon) {
		opacity: 1;
	}
	:global(#refresh.active .icon) {
		animation: spin 3s linear infinite;
	}
	@keyframes spin {
		from {transform: rotate(0deg);}
		to {transform: rotate(360deg);}
	}
</style>
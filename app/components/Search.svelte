<script>
	import { _ } from 'svelte-i18n';
	import { Search, Cross } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Transaction } from '../transactions';
	import Loading from './Loading.svelte';

	function getRandomInt(min, max) {
		return Math.floor(Math.random() * (max - min + 1)) + min;
	}

	let prevSalt;
	function getSalt() {
		while (true) {
			const salt = getRandomInt(0, 0xffffffff);
			if (salt != prevSalt) {
				prevSalt = salt;
				return salt;
			}
		}
	}

	let searchResults = [];
	let hasMoreResults = false;
	let isLoading = false;
	let isSearching = false;
	let searchInput;
	function updateSearch() {
		const query = this.value.trim();
		if (query.length === 0) {
			searchResults = [];
			isLoading = false;
			isSearching = false;
		} else {
			invoke('search', { salt: getSalt(), query: this.value });
			if (!isSearching) isLoading = true;
			isSearching = true;
		}
	}

	function clearSearch() {
		searchInput.value = '';
		searchInput.focus();
		searchResults = [];
		isLoading = false;
		isSearching = false;
	}

	invoke('search_channel').then(transaction_id => {
		const transaction = new Transaction(transaction_id);
		transaction.listen(event => {
			if (event.stream) {
				const [salt, [results, hasMore]] = event.data;
				if (salt !== prevSalt) return;
				hasMoreResults = hasMore;
				searchResults = results;
				isLoading = false;
			}
		});
	});
</script>

<main class:results={searchResults.length > 0}>
	<div id="input-container">
		<input type="text" id="search" placeholder={$_('search')} on:input={updateSearch} bind:this={searchInput}/>
		{#if isSearching}
			<span id="cancel-search" on:click={clearSearch}><Cross size="1rem"/></span>
		{:else}
			<Search id="search-icon" size="1rem"/>
		{/if}
	</div>
	<div id="search-results" class="hide-scroll" class:loading={isLoading}>
		{#if isLoading}
			<Loading size="1.5rem"/>
		{:else if searchResults.length > 0}
			{#each searchResults as result}
				<div class="result">
					<div>
						{result[0]}
						{#if result[1] && result[1].source === 'my_workshop'}
							<a class="association" target="_blank" href="https://steamcommunity.com/sharedfiles/filedetails/?id={result[1].association}">https://steamcommunity.com/sharedfiles/filedetails/?id={result[1].association}</a>
						{:else}
							<div class="association">{result[1].association}</div>
						{/if}
					</div>
					{#if result[1]}
						<div>{$_(result[1].source)}</div>
					{/if}
				</div>
			{/each}
			{#if hasMoreResults}
				<div class="show-more">{$_('show_more')}</div>
			{/if}
		{/if}
	</div>
</main>

<style>
	main {
		position: relative;
		flex: 1;
		margin: 1rem;

		display: flex;
		align-items: center;
		justify-content: center;
	}
	main > div {
		position: relative;
		width: 40%;
		min-width: min(500px, 100%);
		max-width: 100%;

		transition: width .25s;
	}
	#search {
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		width: 100%;
		padding: .8rem;
		padding-left: 2.5rem;
		color: #fff;
	}
	#search:focus {
		outline: none;
		box-shadow: inset 0 0 0px 1.5px #127cff;
	}
	main :global(.icon) {
		position: absolute;
		top: calc(50% - .5rem);
		opacity: .3;
		vertical-align: initial !important;
		transition: margin-right .25s;
		left: .8rem;
	}

	main:focus-within > div {
		width: 60%;
	}
	#search:focus + :global(.icon), #search + #cancel-search > :global(.icon) {
		opacity: 1;
	}
	#search + :global(#search-icon) {
		pointer-events: none;
	}
	#cancel-search > :global(.icon) {
		cursor: pointer;
	}
	#input-container {
		z-index: 2;
	}

	main.results #search:focus {
		border-bottom-left-radius: 0;
		border-bottom-right-radius: 0;
	}
	main:not(:focus-within) > #search-results {
		opacity: 0;
		pointer-events: none;
	}
	main > #search-results {
		position: absolute;
		top: 100%;
		background: #474747;
		box-shadow: 0 0 10px #0000009e;
		z-index: 1;
		max-height: calc(100vh - 70px - 1rem);
	}
	main > #search-results .result {
		display: flex;
		padding: .7rem;
		cursor: pointer;
	}
	main > #search-results .result, #search-results .show-more {
		transition: background-color .1s;
	}
	main > #search-results .result:hover, #search-results .show-more:hover {
		background-color: rgb(0,0,0,.2);
	}
	main > #search-results.show-more .result:last-child {
		padding-bottom: 0;
	}
	main > #search-results .result > div:first-child {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	main > #search-results .result > div:last-child {
		margin-left: .5rem;
		opacity: .5;
	}

	#search-results .show-more {
		padding: 1.25rem;
		text-align: center;
		color: #00adff;
		font-weight: bold;
		font-size: 1em;
		cursor: pointer;
	}

	#search-results.loading {
		padding: 1rem;
	}

	#search-results .association {
		display: block;
		opacity: .5;
		font-size: .8em;
		margin-top: .5rem;
	}
	#search-results a.association {
		transition: opacity .25s, color .25s;
	}
	#search-results a.association:hover {
		opacity: 1;
		color: #46b0ff;
	}
</style>

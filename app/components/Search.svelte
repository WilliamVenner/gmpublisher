<script>
	import { _ } from 'svelte-i18n';
	import { Search, Cross } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Transaction } from '../transactions';

	let searchActive = false;

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
	function updateSearch() {
		const query = this.value.trim();
		if (query.length === 0) {
			searchResults = [];
		} else {
			invoke('search', { salt: getSalt(), query: this.value });
		}
	}

	function clearSearch() {

	}

	invoke('search_channel').then(transaction_id => {
		const transaction = new Transaction(transaction_id);
		transaction.listen(event => {
			if (event.stream) {
				const [salt, results] = event.data;
				if (salt !== prevSalt) return;
				searchResults = results;
			}
		});
	});
</script>

<main class:results={searchResults.length > 0}>
	<div id="input-container">
		<input type="text" id="search" placeholder={$_('search')} on:input={updateSearch}/>
		{#if searchActive}
			<span id="cancel-search" on:click={clearSearch}><Cross size="1rem"/></span>
		{:else}
			<Search size="1rem"/>
		{/if}
	</div>
	{#if searchResults.length > 0}
		<div id="search-results">
			{#each searchResults as result}
				<div class="result">{result[0]}</div>
			{/each}
		</div>
	{/if}
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
	#search:focus + :global(.icon) {
		opacity: 1;
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
		padding: 1rem;
		background: #474747;
	}
	main > #search-results .result:not(:last-child) {
		margin-bottom: .5rem;
	}
</style>

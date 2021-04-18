<script>
	import { _ } from 'svelte-i18n';
	import { Search, Cross } from 'akar-icons-svelte';

	let searchActive = false;

	function updateSearch() {

	}

	function clearSearch() {

	}
</script>

<main><div>
	<input type="text" id="search" placeholder={$_('search')} on:input={updateSearch}/>
	{#if searchActive}
		<span id="cancel-search" on:click={clearSearch}><Cross size="1rem"/></span>
	{:else}
		<Search size="1rem"/>
	{/if}
</div></main>

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
		font-size: .85em;
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
</style>

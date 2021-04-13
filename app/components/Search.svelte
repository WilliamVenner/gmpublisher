<script>
	import { _ } from 'svelte-i18n';
	import { Search, Cross } from 'akar-icons-svelte';

	let searchActive = false;

	function updateSearch() {

	}

	function clearSearch() {

	}
</script>

<main><div id="padding">
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
		justify-content: flex-end;
		align-items: center;
		min-width: 250px;
	}
	main :global(.icon) {
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
		padding: .8rem;
		padding-left: 2.5rem;
		color: #fff;
		font-size: .85em;
	}
	#search:focus {
		outline: none;
		box-shadow: inset 0 0 0px 1.5px #127cff;
	}
	#search:focus + :global(.icon) {
		opacity: 1;
	}
	main > #padding {
		position: relative;
		width: max(27%, 250px);
		min-width: min(400px, max(250px, 100%));
		max-width: max(100%, 250px);
	}
</style>

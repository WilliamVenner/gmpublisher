<script>
	import Addon from "./Addon.svelte";
	import { _ } from "svelte-i18n";
	import Dead from "./Dead.svelte";
	import Loading from "./Loading.svelte";

	export let next;

	let loading = true;
	let total = 0;
	let addons = [];
	let firstPage;
	function fetchNext() {
		loading = true;

		let promise = next().then(([fetchedTotal, fetchedAddons]) => {

			loading = false;

			addons = addons.concat(fetchedAddons);
			total = fetchedTotal;

		}, () => loading = false);

		if (!firstPage) firstPage = promise;
	}
	fetchNext();

	let scroller;
	function infiniScroll() {
		if (!scroller || loading) return;
		if (scroller.offsetHeight + scroller.scrollTop >= scroller.scrollHeight * .9 && addons.length < total) {
			fetchNext();
		}
	}
</script>

<main class="hide-scroll" on:scroll={infiniScroll} bind:this={scroller}>
	{#if addons.length !== 0 && total !== 0 && addons.length !== total}
		<h2 id="amount">
			<span>{$_('showing_num_addons', { values: { num: addons.length.toLocaleString(), total: total.toLocaleString() }})}</span>
		</h2>
	{/if}

	{#await firstPage}
		<Loading size="2rem"/>
	{:then}
		<div id="grid">
			{#each addons as addon}
				<Addon workshopData={addon.workshop ? Promise.resolve(addon.workshop) : null} installedData={addon.installed ? Promise.resolve(addon.installed) : null}/>
			{/each}
		</div>
	{:catch reason}
		<Dead size="2rem"/>
		<div id="error">{JSON.stringify(reason)}</div>
	{/await}
</main>

<style>
	main {
		flex: 1;
		overflow: auto;
		position: relative;
		padding: 1.5rem;
		height: 100%;
	}
	main > h2 {
		margin-top: 0;
	}
	main #grid {
		position: relative;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		grid-auto-flow: dense;
		align-items: center;
		grid-gap: 1rem;
	}
	main > :global(svg) {
		position: absolute;
		top: 1.5rem !important;
		bottom: auto !important;
	}
	main > :global(.loading) {
		opacity: .2;
	}
	#error {
		position: absolute;
		margin: auto;
		top: 4.5rem;
		text-align: center;
		padding: 1.5rem;
		left: 0;
		right: 0;
		padding-top: 0;
		padding-bottom: 0;
	}
</style>

<script>
	import Addon from "./Addon.svelte";
	import { _ } from "svelte-i18n";
	import Dead from "./Dead.svelte";
	import Loading from "./Loading.svelte";

	export let next;

	let total = 0;
	let addons = [];
	let firstPage;
	function fetchNext() {
		let promise = next().then(([fetchedTotal, fetchedAddons]) => {
			addons = addons.concat(fetchedAddons);
			total = fetchedTotal;
		});

		if (!firstPage) firstPage = promise;
	}
	fetchNext();

	let scroller; let scrollFade;
	function infiniScroll() {
		scrollFade.classList.toggle('active', scroller.scrollTop > 14);

		if (scroller.offsetHeight + scroller.scrollTop >= scroller.scrollHeight * .9) {
			if (addons.length < total) {
				let didAdvance = next();
				if (didAdvance) addonsLoading.set(didAdvance);
			}
		}
	}
</script>

<main class="hide-scroll" on:scroll={infiniScroll}>
	<div id="scroll-fade" bind:this={scrollFade}></div>

	{#if addons.length !== 0 && total !== 0 && addons.length !== total}
		<h2 id="amount">
			<span>{$_('showing_num_addons', { values: { num: addons.length.toLocaleString(), total: total.toLocaleString() }})}</span>
		</h2>
	{/if}

	<div id="grid" bind:this={scroller}>
		{#await firstPage}
			<Loading/>
		{:then}
			{#each addons as addon}
				<Addon workshop={addon.workshop ? Promise.resolve(addon.workshop) : null} installed={addon.installed ? Promise.resolve(addon.installed) : null}/>
			{/each}
		{:catch reason}
			<Dead/>
			{JSON.stringify(reason)}
		{/await}
	</div>
</main>

<style>
	main {
		flex: 1;
		overflow: auto;
		position: relative;
		padding: 1.5rem;
	}
	main > h2 {
		margin-top: 0;
	}
	main #scroll-fade {
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
		grid-column: none;
		grid-row: none;
	}
	:global(main #scroll-fade.active) {
		opacity: 1 !important
	}
	main #grid {
		position: relative;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		grid-auto-flow: dense;
		align-items: center;
		grid-gap: 1rem;
	}
</style>

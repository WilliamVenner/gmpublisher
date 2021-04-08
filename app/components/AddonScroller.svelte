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
			console.log(addons);
			total = fetchedTotal;
		});

		if (!firstPage) firstPage = promise;
	}
	fetchNext();

	let searchAmount;
	let searching = false;
</script>

<main>
	{#if (searchAmount && searchAmount !== total) || (addons.length !== 0 && total !== 0 && addons.length !== total)}
		<h2 id="amount">
			{#if searching}
				<Loading/>&nbsp;
			{/if}
			<span>{$_('showing_num_addons', { values: { num: (searchAmount != null ? searchAmount : addons.length).toLocaleString(), total: total.toLocaleString() }})}</span>
		</h2>
	{/if}

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
</main>

<script>
	import { Steam } from '../steam';
	import AddonScroller from '../components/AddonScroller.svelte';
	import { _ } from 'svelte-i18n';
	import { writable } from 'svelte/store';
	import PreparePublish from '../components/PreparePublish.svelte';

	let page = 0;
	function next() {
		return Steam.getMyWorkshop(++page);
	}

	let updatingAddon = writable(null);
	let preparePublish = writable(false);

	function togglePreparePublish() {
		$updatingAddon = null;
		$preparePublish = !$preparePublish;
	}

	async function editPublishedAddon(_, addon) {
		const addonAwaited = await addon;
		if (addonAwaited != $updatingAddon) {
			$updatingAddon = addonAwaited;
		}
		$preparePublish = !$preparePublish;
	}

	let remountAddonScroller = writable(false);
</script>

{#if $remountAddonScroller || !$remountAddonScroller}
	<AddonScroller {next} onClick={editPublishedAddon} onNewAddonClick={togglePreparePublish}/>
{/if}

<PreparePublish {preparePublish} {updatingAddon} {remountAddonScroller}/>

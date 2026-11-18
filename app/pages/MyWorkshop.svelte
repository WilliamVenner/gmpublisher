<script>
	import { Steam } from '../steam';
	import AddonScroller from '../components/AddonScroller.svelte';
	import { _ } from 'svelte-i18n';
	import { writable } from 'svelte/store';
	import PreparePublish, { remountAddonScroller } from '../components/PreparePublish.svelte';
	import { afterUpdate } from 'svelte';

	function getPage(page) {
		return Steam.getMyWorkshop(page);
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

	afterUpdate(() => {
		if ($remountAddonScroller) {
			$remountAddonScroller = false;
		}
	});
</script>

{#if !$remountAddonScroller}
	<AddonScroller {getPage}onClick={editPublishedAddon} onNewAddonClick={togglePreparePublish}/>
{/if}

<PreparePublish {preparePublish} {updatingAddon}/>

<script context="module">
	export const preparePublish = writable(false);
</script>

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

	function togglePreparePublish() {
		$preparePublish = !$preparePublish;
	}

	function editPublishedAddon() {

	}
</script>

<AddonScroller next={next} onClick={editPublishedAddon} onNewAddonClick={togglePreparePublish}/>

<PreparePublish/>

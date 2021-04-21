<script context="module">
	export const preparePublish = writable(false);
</script>

<script>
	import { Addons } from '../addons';
	import AddonScroller from '../components/AddonScroller.svelte';
	import { _ } from 'svelte-i18n';
	import { writable } from 'svelte/store';
	import PreparePublish from '../components/PreparePublish.svelte';

	let page = 0;
	function next() {
		return Addons.getMyWorkshop(++page);
	}

	function togglePreparePublish() {
		$preparePublish = !$preparePublish;
	}
</script>

<AddonScroller next={next} newAddon={togglePreparePublish}/>

<PreparePublish/>

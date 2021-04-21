<script>
import { writable } from 'svelte/store';

	import { Steam } from '../steam';
	import AddonScroller from '../components/AddonScroller.svelte';
	import PreviewGMA from '../components/PreviewGMA.svelte';

	let page = 0;
	function next() {
		return Steam.getInstalledAddons(++page);
	}

	let previewingGMA = false;
	const promises = writable([new Promise(() => {}), new Promise(() => {})]);
	function onClick(e, workshop, installed) {
		previewingGMA = true;
		$promises = [workshop, installed];
	}
</script>

<AddonScroller next={next} {onClick}/>

<PreviewGMA active={previewingGMA} {promises} cancel={() => previewingGMA = false}/>

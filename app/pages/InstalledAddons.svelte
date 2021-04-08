<script>
	import { Addons } from '../addons';
	import AddonScroller from '../components/AddonScroller.svelte';

	let page = 0;
	function next() {
		return Addons.getInstalledAddons(++page);
	}
</script>

<main>
	<AddonScroller next={next}/>
</main>

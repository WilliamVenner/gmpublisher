<script>
	import { Steam } from '../steam';
	import AddonScroller from '../components/AddonScroller.svelte';

	let page = 0;
	function next() {
		return Steam.getSubscribedAddons(++page);
	}

	function onClick(_, workshop, installed) {
		console.log(_, workshop, installed);
	}
</script>

<main>
	<AddonScroller next={next} {onClick}/>
</main>

<style>

</style>

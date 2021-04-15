<script>
	import { Addons } from '../addons';
	import AddonScroller from '../components/AddonScroller.svelte';

	let page = 0;
	function next() {
		return Addons.getMyWorkshop(++page);
	}
</script>

<AddonScroller next={next} newAddon={true}/>

<script>
	import { onDestroy } from 'svelte';
	import { _ } from 'svelte-i18n';
	import { ArrowCycle } from 'akar-icons-svelte';
	import Addons from '../addons.js';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';

	let _transaction;
	onDestroy(() => _transaction?.cancel());

	Addons.analyzeAddonSizes().then(transaction => {
		_transaction = transaction.listen(event => {
			if (event.finished) {

			} else if ("progress" in event) {

			}
		});
	});
</script>
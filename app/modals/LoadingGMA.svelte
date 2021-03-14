<script>
	import Addons from '../addons.js';

	export let path;
	export let promise;

	Addons.openGMA(path).then(transaction => {
		transaction.listen(event => {
			console.log(event);
		});
	});
</script>

<div id="loading" class="modal">
	Progress %
</div>

<style>
	
</style>
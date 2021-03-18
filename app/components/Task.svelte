<script>
import { writable } from "svelte/store";


	export let transaction;

	const progress = writable(0);
	const error = writable(null);
	const finished = writable(false);

	transaction.listen(event => {

	});
</script>

<div id="task">
	<img src="/img/loading.svg" alt="Loading"/>
	
</div>
<script>
	import ContextMenu, { currentContext } from './ContextMenu.svelte';

	let container;
	currentContext.subscribe(context => {
		if (context) {
			new ContextMenu({
				target: container,
				props: context
			});
		}
	});
</script>

<main bind:this={container}></main>

<style>
	main {
		position: absolute;
		width: 0;
		height: 0;
		max-width: 0;
		max-height: 0;
		top: 0;
		left: 0;
		z-index: 9999;
	}
</style>

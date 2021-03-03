<script>
	import Landing from './pages/Landing.svelte';

	const pages = [
		{ component: Landing, props: {} },
	];

	let selected = pages[0];
</script>

<main>
	<div id="content">
		<svelte:component this={selected.component} {...selected.props}/>
	</div>
</main>

<style>
	main, #content {
		width: 100%;
		height: 100%;
	}
</style>
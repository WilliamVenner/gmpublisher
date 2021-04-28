<script>
	export let click = null;
</script>

<main on:click={click}>
	<slot name="icon"></slot>
	<slot name="label"></slot>
</main>

<style>
	main {
		display: flex;
		align-items: center;
		padding: .7rem;
		cursor: pointer;
		background-color: rgba(255,255,255,0);
		transition: background-color .1s;
	}
	main > :global(*:not(:last-child)) {
		margin-right: .5rem;
	}
	main:hover {
		background-color: rgba(255,255,255,.1);
	}
</style>

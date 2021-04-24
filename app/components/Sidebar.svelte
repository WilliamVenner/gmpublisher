<script>
	export let id = null;
</script>

<sidebar {id}>
	<div><slot></slot></div>
	<slot name="footer"></slot>
</sidebar>

<style>
	sidebar {
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
	}
	sidebar > div:first-child {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
</style>

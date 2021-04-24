<script>
	export let id;
	export let activeItem;
	export let click = null;
</script>

<item class:active={$activeItem == id} on:click={() => { $activeItem = id; if (click) click(id) }}><slot></slot></item>

<style>
	item {
		padding: .5rem;
		padding-left: .7rem;
		padding-right: .7rem;
		cursor: pointer;
		border-radius: 4px;
	}
	item.active {
		background-color: #2A2A2A;
	}
	item:not(:last-child) {
		margin-bottom: .5rem;
	}
</style>

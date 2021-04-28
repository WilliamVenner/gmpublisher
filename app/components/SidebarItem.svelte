<script>
	export let id;
	export let activeItem;
	export let click = null;
	export let jobs = null;
</script>

<item class:active={$activeItem == id} on:click={() => { $activeItem = id; if (click) click(id) }}>
	<span><slot></slot></span>
	{#if jobs && $jobs > 0}
		<div class="jobs">{$jobs}</div>
	{/if}
</item>

<style>
	item {
		display: flex;
		padding: .5rem;
		padding-left: .7rem;
		padding-right: .7rem;
		cursor: pointer;
		border-radius: 4px;
		align-items: center;
	}
	item > span {
		flex: 1;
	}
	item > .jobs {
		padding: .25em .4em;
		background-color: #007eff;
		border-radius: .35rem;
		font-size: .75em;
		text-align: center;
		white-space: nowrap;
		vertical-align: baseline;
		margin-right: -.4em;
	}
	item.active {
		background-color: #2A2A2A;
	}
	item:not(:last-child) {
		margin-bottom: .5rem;
	}
</style>

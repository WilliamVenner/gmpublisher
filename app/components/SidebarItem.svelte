<script>
	export let id;
	export let activeItem;
	export let click = null;
	export let jobs = null;
</script>

<item class:active={$activeItem == id} on:click={() => { $activeItem = id; if (click) click(id) }}>
	<span><slot></slot></span>
	{#if jobs}
		<div class="jobs" class:active={$jobs > 0}>{$jobs}</div>
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
		height: calc(1.181em + .5rem + .5rem);
		max-height: calc(1.181em + .5rem + .5rem);
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

		transform: scale(0);
		opacity: 0;
		transition: transform .1s, opacity .1s;
	}
	item > .jobs.active {
		transform: scale(1);
		opacity: 1;
	}
	item.active {
		background-color: #2A2A2A;
	}
	item:not(:last-child) {
		margin-bottom: .5rem;
	}
</style>

<script>
	import fileSize from "filesize";
	import { Steam } from "../steam";
	import Dead from "./Dead.svelte";
	import Loading from "./Loading.svelte";

	export let item;
</script>

<div class="item">
	{#await Steam.getWorkshopAddon(item)}
		<div class="image">
			<Loading size="2rem"/>
		</div>
		<div class="info">
			<div class="name hide-scroll">...</div>
			<div class="size">{fileSize(0)}</div>
		</div>
	{:then workshop}
		{#if workshop.dead}
			<div class="image">
				<Dead size="2rem"/>
			</div>
			<div class="info">
				<div class="name hide-scroll">¯\_(ツ)_/¯</div>
				<div class="size">{fileSize(0)}</div>
			</div>
		{:else}
			<div class="image">
				<img src={workshop.previewUrl}/>
			</div>
			<div class="info">
				<div class="name hide-scroll">{workshop.title}</div>
				<div class="size">{fileSize(workshop.size)}</div>
			</div>
		{/if}
	{:catch}
		<div class="image">
			<Dead size="2rem"/>
		</div>
		<div class="info">
			<div class="name hide-scroll">¯\_(ツ)_/¯</div>
			<div class="size">{fileSize(0)}</div>
		</div>
	{/await}
</div>

<style>
	.item {
		display: grid;
		grid-template-columns: min-content 1fr;
		grid-template-rows: 1fr;
		padding: 1rem;
		background-color: #151517;
		box-shadow: inset 0 0 8px #000000c7;
		cursor: pointer;
	}
	.item .image {
		position: relative;
		width: 5vw;
		height: 5vw;
		margin-right: 1rem;
		background-color: #0a0a0a;
	}
	.item > .image > :global(.icon), .item > .image > :global(.dead), .item > .image > :global(.loading) {
		position: absolute;
	}
	.item > .image > * {
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
	}
	.item > .image > img {
		display: block;
		width: 100%;
		height: 100%;
	}
	.item .info {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.item .info .name {
		flex: 1;
		margin-bottom: .5rem;
		font-size: 1.2em;
	}
</style>

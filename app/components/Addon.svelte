<script>
	import { Addons } from '../addons';
	import { tippyFollow } from '../tippy';
	import Loading from './Loading.svelte';
	import Dead from './Dead.svelte';

	export let workshop = null;
	export let installed = null;

	if (!installed && workshop) {
		if (workshop.localFile) {
			installed = Addons.getAddon(workshop.localFile);
		} else {
			installed = Promise.reject().catch(() => {});
		}
	} else if (installed && !workshop) {
		workshop = new Promise((resolve, reject) => {
			installed.then(installed => {
				if (installed.id) {
					console.log('ye');
					Addons.getWorkshopAddon(installed.id).then(item => {
						console.log('2');
						item.dead ? reject() : resolve(item);
					});
				} else {
					reject();
				}
				return installed;
			}, reject);
		});
	}
</script>

<main>
	<div id="card">
		<div id="stats">
			<span id="subscriptions">
				<img src="/img/download.png"/>
				{#await workshop}
					0
				{:then workshop}
					{workshop.subscriptions ?? 0}
				{/await}
				&nbsp;
			</span>
			{#await workshop}
				<img use:tippyFollow={'0.00%'} id="score" src="/img/0-star.png"/>
			{:then workshop}
				<img use:tippyFollow={(Math.round((((workshop.score ?? 0) * 100) + Number.EPSILON) * 100) / 100) + '%'} id="score" src="/img/{Math.round(((workshop.score ?? 0) * 10) / 2)}-star.png"/>
			{/await}
		</div>

		{#await workshop}
			<div id="preview" class="dead"><Loading size="2rem"/></div>
		{:then workshop}
			{#if workshop.previewUrl}
				<img id="preview" src={workshop.previewUrl} alt="Preview"/>
			{:else}
				<div id="preview" class="dead"><Dead/></div>
			{/if}
		{:catch}
			<div id="preview" class="dead"><Dead/></div>
		{/await}

		{#await workshop}
			{#await installed}
				<div id="title">¯\_(ツ)_/¯</div>
			{:then installed}
				<div id="title">{installed.name ?? installed.extractedName}</div>
			{:catch}
				<div id="title">¯\_(ツ)_/¯</div>
			{/await}
		{:then workshop}
			<div id="title">{workshop.title}</div>
		{:catch}
			{#await installed}
				<div id="title">¯\_(ツ)_/¯</div>
			{:then installed}
				<div id="title">{installed.name ?? installed.extractedName}</div>
			{:catch}
				<div id="title">¯\_(ツ)_/¯</div>
			{/await}
		{/await}
	</div>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	main #card {
		padding: .8rem;
		transition: background-color .1s, box-shadow .1s;
	}
	main:not(.previewing) {
		cursor: pointer;
	}
	main:not(.previewing):hover #card {
		background-color: rgba(45, 45, 45, 1);
		box-shadow: 0px 0px 4px rgb(0 0 0 / 40%);
	}
	main #title {
		margin-top: .8rem;
		flex: 1;
		text-align: center;
	}
	main #stats {
		display: flex;
		flex-direction: row;
		text-align: center;
		align-items: center;
	}
	main #stats * {
		vertical-align: middle;
	}
	main #subscriptions {
		flex: 1;
		text-align: left;
	}
	main #subscriptions img {
		margin-right: .1rem;
	}
	main #preview {
		width: 100%;
		flex: 0;
		margin-top: .8rem;
		box-shadow: 0 0 2px 1px rgba(0, 0, 0, .5);
	}
	main #preview.dead {
		position: relative;
		background-color: #0c0c0c;
	}
	main #preview.dead::after {
		content: "";
		display: block;
		padding-bottom: 100%;
	}
	main #preview.dead :global(svg) {
		position: absolute;
		margin: auto;
		left: 0;
		top: 0;
		right: 0;
		bottom: 0;
		color: #212121;
	}
	main :global(.highlight) {
		background-color: rgba(255, 255, 0, .5);
		border-radius: 4px;
		box-shadow: 0 0 2px rgba(255, 255, 0, .5);
		color: black;
		padding: 2px;
		padding-left: 4px;
		padding-right: 4px;
		margin: -4px;
	}
</style>

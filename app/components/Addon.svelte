<script>
	import { _ } from 'svelte-i18n';
	import { CirclePlus } from 'akar-icons-svelte';
	import { Steam } from '../steam';
	import { tippyFollow } from '../tippy';
	import Loading from './Loading.svelte';
	import Dead from './Dead.svelte';

	// TODO merge workshop and gma promises?

	export let newAddon = false;
	export let workshopData = null;
	export let installedData = null;
	export let onClick = null;
	export let previewing = false;

	let workshop = workshopData;
	let installed = installedData;

	if (!newAddon) {
		if (!installed && workshop) {
			if (workshop.localFile) {
				installed = Steam.getAddon(workshop.localFile);
			} else {
				installed = Promise.reject().catch(() => {});
			}
		} else if (installed && !workshop) {
			workshop = new Promise((resolve, reject) => {
				installed.then(installed => {
					if (installed.id) {
						Steam.getWorkshopAddon(installed.id).then(item => {
							item.dead ? reject() : resolve(item);
							return item;
						});
					} else {
						reject();
					}
					return installed;
				}, reject);
			});
		}

		if (!workshop && !installed) {
			throw new Error("workshop && installed == null");
		}
	}
</script>

<main class="addon" class:previewing={previewing} on:click={e => onClick(e, workshop, installed)}>
	<div id="card">
		<div id="stats">
			{#if newAddon}
				<span id="subscriptions">
					<img src="/img/download.png"/>
					0
					&nbsp;
				</span>
				<img use:tippyFollow={'0.00%'} id="score" src="/img/0-star.png"/>
			{:else}
				{#await workshop}
					<span id="subscriptions">
						<img src="/img/download.png"/>
						0
						&nbsp;
					</span>
					<img use:tippyFollow={'0.00%'} id="score" src="/img/0-star.png"/>
				{:then workshop}
					<span id="subscriptions">
						<img src="/img/download.png"/>
						{workshop.subscriptions}
						&nbsp;
					</span>
					<img use:tippyFollow={(Math.round((((workshop.score ?? 0) * 100) + Number.EPSILON) * 100) / 100) + '%'} id="score" src="/img/{Math.round(((workshop.score ?? 0) * 10) / 2)}-star.png"/>
				{:catch}
					<span id="subscriptions">
						<img src="/img/download.png"/>
						0
						&nbsp;
					</span>
					<img use:tippyFollow={'0.00%'} id="score" src="/img/0-star.png"/>
				{/await}
			{/if}
		</div>

		{#if newAddon}
			<div id="preview" class="new">
				<CirclePlus size="4rem"/>
			</div>
			<div id="title">{$_('publish_new')}</div>
		{:else}
			{#await workshop}
				<div id="preview" class="dead"><Loading size="2rem"/></div>
			{:then workshop}
				{#if workshop.previewUrl}
					<div id="preview" class="loading">
						<Loading size="2rem"/>
						<img src={workshop.previewUrl} alt="Preview" onload="this.parentElement.classList.remove('loading')"/>
					</div>
				{:else}
					<div id="preview" class="dead"><Dead size="4rem"/></div>
				{/if}
			{:catch}
				<div id="preview" class="dead"><Dead size="4rem"/></div>
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
		{/if}
	</div>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	main:not(.previewing) #card {
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
		position: relative;
		width: 100%;
		flex: 0;
		margin-top: .8rem;
		box-shadow: 0 0 2px 1px rgba(0, 0, 0, .5);
		background-color: #0c0c0c;
	}
	main #preview > img {
		width: 100%;
		display: block;
	}
	main #preview.loading > img {
		position: absolute;
		opacity: 0;
	}
	main #preview:not(.dead):not(.loading):not(.new) > :global(svg) {
		display: none;
	}
	main #preview.dead::after, main #preview.loading::after, main #preview.new::after {
		content: "";
		display: block;
		padding-bottom: 100%;
	}
	main #preview.dead :global(svg), main #preview.loading :global(svg), main #preview.new :global(svg) {
		position: absolute;
		margin: auto;
		left: 0;
		top: 0;
		right: 0;
		bottom: 0;
	}
	main #preview.dead :global(svg), main #preview.loading :global(svg) {
		color: #212121;
	}
	main #preview.new :global(svg) {
		color: #424242;
		transition: color .1s;
	}
	main:hover #preview.new :global(svg) {
		color: #fff;
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

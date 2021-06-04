<script context="module">
	import ContextMenuItem from './ContextMenuItem.svelte';

	export const currentContext = writable(null);

	const contextRegistry = new WeakMap();

	new MutationObserver(mutations => {
		for (let x = 0; x < mutations.length; x++) {
			const mutation = mutations[x];
			if (mutation.removedNodes) {
				for (let i = 0; i < mutation.removedNodes; i++) {
					const node = mutation[i];
					if (contextRegistry.has(node)) {
						contextRegistry.delete(node);
					}
				}
			}
		}
	}).observe(document.body, { subtree: true, childList: true });

	export function registerContext(node, ...data) {
		contextRegistry.set(node, data);
	}

	function findTarget(e, target) {
		if (contextRegistry.has(target)) {

			var register = contextRegistry.get(target);
			let workshop;
			let gma;
			let disableDownload;

			if (typeof register[0] === 'function') {
				register = register[0](e);
				if (!register) return;
			}

			[workshop, gma, disableDownload] = register;

			currentContext.set({
				x: e.pageX,
				y: e.pageY,
				workshop,
				gma,
				disableDownload
			});

			e.preventDefault();
			e.stopPropagation();

		} else if (target.parentNode) {
			return findTarget(e, target.parentNode);
		}
	}

	document.addEventListener('contextmenu', function(e) {
		if (e.target) {
			findTarget(e, e.target);
		}
	}, false);

	window.addEventListener('scroll', () => {
		const current = get(currentContext);
		if (current && current.element) {
			current.element.remove();
		}
		currentContext.set(null);
	}, true);
</script>

<script>
	import { _ } from 'svelte-i18n';
	import { Folder, FolderAdd, LinkChain, LinkOut, Image, CloudDownload } from 'akar-icons-svelte';
	import { get, writable } from 'svelte/store';
	import Loading from './Loading.svelte';
	import { onDestroy, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import DestinationSelect from './DestinationSelect.svelte';
	import fileSize from 'filesize';
	import { taskMessage, Transaction } from '../transactions';
import { playSound } from '../sounds';

	export let workshop = null;
	export let gma = null;
	export let x;
	export let y;
	export let disableDownload = false;

	let subscription;

	const me = get(currentContext);
	if (!me) throw new Error('currentContext invalid');

	if (!workshop || !gma) {
		throw new Error('Tried to create a context menu with no Workshop or GMA promise');
	}

	let loading = new Promise(resolve => {
		if (workshop) {
			workshop.then(workshop => {
				resolve();
				return workshop;
			});
		}
		if (gma) {
			gma.then(gma => {
				resolve();
				return gma;
			});
		}
	});

	let destroyed = false;
	function destroy() {
		destroyed = true;

		setTimeout(() => {
			if (contextMenu) {
				contextMenu.remove();
			}
		}, 250);

		onSoftDestroy();

		if ($currentContext == me) {
			$currentContext = null;
		}
	}

	let contextMenu;
	function updatePosition() {
		if (!contextMenu) return;

		contextMenu.style.left = Math.max(Math.min(x, window.innerWidth - contextMenu.clientWidth), 0) + 'px';

		const top = Math.max(Math.min(y, window.innerHeight - contextMenu.clientHeight), 0);
		contextMenu.style.top = top + 'px';
		contextMenu.style.transformOrigin = y >= top ? 'top' : 'bottom';
	}

	onMount(() => {
		me.element = contextMenu;
		new ResizeObserver(updatePosition).observe(contextMenu);
		updatePosition();
	});
	$: { updatePosition(); }

	function onClickOff(e) {
		if (!contextMenu) return;
		destroy();
	}
	onMount(() => {
		document.addEventListener('click', onClickOff);
	});

	function onSoftDestroy() {
		document.removeEventListener('click', onClickOff);

		if (subscription) {
			subscription();
			subscription = null;
		}
	}
	onDestroy(onSoftDestroy);

	subscription = currentContext.subscribe(val => {
		if (val == me) return;
		destroy();
	});

	async function openAddonLocation() {
		invoke('open_file_location', { path: (await gma).path });
	}

	async function copyLink() {
		navigator.clipboard.writeText('https://steamcommunity.com/sharedfiles/filedetails/?id=' + (await workshop).id);
	}

	async function copyImageLink() {
		navigator.clipboard.writeText((await workshop).previewUrl);
	}

	let choosingExtractDestination = false;
	let extractedName;
	let gmaSize;
	async function chooseExtractDestination() {
		let awaitedGma = await gma;
		gmaSize = awaitedGma.size;
		extractedName = awaitedGma.extractedName;
		choosingExtractDestination = true;
	}
	async function extractGMA(dest) {
		choosingExtractDestination = false;

		invoke('extract_gma', { gmaPath: (await gma).path, dest })
			.then(transactionId => new Transaction(transactionId, transaction => {
				return $_('extracting_progress', { values: {
					pct: transaction.progress,
					data: fileSize((transaction.progress / 100) * gmaSize),
					dataTotal: fileSize(gmaSize)
				}});
			}));
	}

	async function download() {
		await invoke('workshop_download', { ids: [(await workshop).id] });

		playSound('success');
		taskMessage($_('sent_to_downloader'));
	}
</script>

<DestinationSelect active={choosingExtractDestination} cancel={() => choosingExtractDestination = false} callback={extractGMA} text={$_('extract')} extractedName={extractedName}/>

<main class="context-menu" bind:this={contextMenu} class:destroyed={destroyed} on:resize={updatePosition}>
	{#await loading}
		<Loading size="2rem"/>
	{:then}
		{#if gma}
			{#await gma then gma}
				{#if gma}
					{#if gma.path}
						<ContextMenuItem click={chooseExtractDestination}>
							<span slot="icon"><FolderAdd size="1rem"/></span>
							<span slot="label">{$_('extract')}</span>
						</ContextMenuItem>
						<ContextMenuItem click={openAddonLocation}>
							<span slot="icon"><Folder size="1rem"/></span>
							<span slot="label">{$_('open_addon_location')}</span>
						</ContextMenuItem>
					{/if}
				{/if}
			{/await}
		{/if}
		{#await Promise.allSettled([workshop, gma]) then [workshop, gma]}
			{#if workshop?.value && gma?.value}
				<div class="divider"></div>
			{/if}
		{/await}
		{#if workshop}
			{#await workshop then workshop}
				{#if workshop && !workshop.dead}
					<a class="nostyle" href="https://steamcommunity.com/sharedfiles/filedetails/?id={workshop.id}" target="_blank">
						<ContextMenuItem>
							<span slot="icon"><LinkOut size="1rem"/></span>
							<span slot="label">{$_('steam_workshop')}</span>
						</ContextMenuItem>
					</a>
					<ContextMenuItem click={copyLink}>
						<span slot="icon"><LinkChain size="1rem"/></span>
						<span slot="label">{$_('copy_link')}</span>
					</ContextMenuItem>
					{#if !disableDownload}
						<ContextMenuItem click={download}>
							<span slot="icon"><CloudDownload size="1rem"/></span>
							<span slot="label">{$_('download')}</span>
						</ContextMenuItem>
					{/if}
					{#if workshop.previewUrl}
						<div class="divider"></div>
						<a class="nostyle" href={workshop.previewUrl} target="_blank">
							<ContextMenuItem>
								<span slot="icon"><LinkOut size="1rem"/></span>
								<span slot="label">{$_('open_image')}</span>
							</ContextMenuItem>
						</a>
						<ContextMenuItem click={copyImageLink}>
							<span slot="icon"><Image size="1rem"/></span>
							<span slot="label">{$_('copy_image_link')}</span>
						</ContextMenuItem>
					{/if}
				{/if}
			{/await}
		{/if}
	{/await}
</main>

<style>
	.context-menu {
		position: absolute;
		animation: context-menu .25s forwards;
		z-index: 9999;
		background-color: #4A4A4A;
		color: #fff;
		font-size: .9em;
		border-radius: .2rem;
		overflow: hidden;
		width: max-content;
		transform-origin: top;
	}
	.context-menu.destroyed {
		animation: context-menu-reverse .25s forwards;
		pointer-events: none;
	}

	.divider {
		background-color: #636363;
		height: 1px;
	}

	@keyframes context-menu {
		from {
			opacity: 0;
			transform: scaleY(0);
		}
		to {
			opacity: 1;
			transform: scaleY(1);
		}
	}
	@keyframes context-menu-reverse {
		from {
			opacity: 1;
			transform: scaleY(1);
		}
		to {
			opacity: 0;
			transform: scaleY(0);
		}
	}
</style>

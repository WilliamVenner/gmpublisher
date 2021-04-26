<script>
	import { invoke } from "@tauri-apps/api/tauri";
	import { Download, Folder, Plus } from "akar-icons-svelte";
	import { _ } from 'svelte-i18n';
	import * as dialog from '@tauri-apps/api/dialog';
	import Dead from "../components/Dead.svelte";
	import Modal from "../components/Modal.svelte";
	import { tippy } from '../tippy';
	import { Transaction } from "../transactions";
import { Steam } from "../steam";
import fileSize from "filesize";
import Loading from "../components/Loading.svelte";
import BundleItem from "../components/BundleItem.svelte";

	const RE_COLLECTION_ID = /^(?:https?:\/\/(?:.*?\.)?steamcommunity\.com\/sharedfiles\/filedetails\/?.*?(?:\?|&)id=(\d+)|(\d+))$/i;

	let selectedBundle;
	let indexedBundles = {};
	let bundles = invoke('get_bundles').then(bundles => {
		selectedBundle = bundles[0];
		for (let i = 0; i < bundles.length; i++) {
			indexedBundles[bundles[i].id] = bundles[i];
			sortBundle(bundle);
		}
		return bundles;
	});

	let showHelp = false;

	let newCollectionInput;
	let newBundleNameInput;
	let newBundleCollectionImg;
	let newBundleNameCreateBtn;

	let newBundleCollection;
	let newBundleCollectionName = null;
	let newBundleCollectionError = false;
	let newBundleValid = false;

	let newBundle = false;
	function showNewBundle() {
		newBundle = true;
		newBundleCollectionImg.classList.remove('valid');
		newBundleValid = false;
		newBundleCollection = null;
		newBundleCollectionError = false;
		newCollectionInput.value = '';
		newBundleNameInput.value = '';
		newBundleCollectionImg.setAttribute('src', '/img/steam_anonymous.jpg');
	}
	function checkNewBundle() {
		newBundleValid = newBundleNameInput.value.trim().length > 0;
	}
	async function checkCollectionLink() {
		const input = newCollectionInput.value.trim();
		if (input.length > 0) {
			const match = input.match(RE_COLLECTION_ID);
			const collection = parseInt(match?.[1] ?? match?.[2] ?? 0);
			if (!!collection) {
				let data;
				try { data = await invoke('check_bundle_collection', { collection }) } catch (e) {}
				if (data) {
					newBundleCollection = collection;
					newBundleCollectionError = false;
					newBundleCollectionImg.setAttribute('src', data.preview_url ?? '/img/steam_anonymous.jpg');
					newBundleCollectionName = data.title;
					return;
				}
			}
			newBundleCollectionError = true;
		} else {
			newBundleCollectionError = false;
		}
		newBundleCollectionImg.setAttribute('src', '/img/steam_anonymous.jpg');
		newBundleCollectionName = null;
	}
	async function createNewBundle() {
		if (this.classList.contains('disabled')) return;
		newBundle = false;

		const bundle = await invoke('new_bundle', { name: newBundleNameInput.value.trim(), basedOnCollection: newBundleCollection });
		indexedBundles[bundle.id] = bundle;
		(await bundles).push(bundle);
		bundles = bundles;

		await sortBundle(bundle);
		await selectBundle(bundle.id);
	}

	let importBundle = false;
	function showImportBundle() {
		importBundle = true;
	}
	function importBundleBrowse() {
		dialog.open({
			filters: [{
				name: $_('bundle_file'),
				extensions: ['lua']
			}]
		}).then(path => {
			if (path) {
				invoke('import_bundle', { path }).then(transactionId => {
					const transaction = new Transaction(transactionId, () => $_('importing_bundle'));
					transaction.listen(event => {
						if (event.finished) {
							console.log(event.data);
							// TODO
						}
					});
				});
			}
		});
	}

	function pasteBundle(e) {
		if (document.activeElement.id !== 'import-bundle') return;
		if (!e.clipboardData) return;

		const pasted = e.clipboardData.getData('text');
		if (!pasted) return;

		invoke('paste_bundle', { pasted }).then(transactionId => {
			const transaction = new Transaction(transactionId, () => $_('importing_bundle'));
			transaction.listen(event => {
				if (event.finished) {
					console.log(event.data);
					// TODO
				}
			});
		});
	}

	let bundleSelector;
	async function selectBundle(id) {
		id = parseInt(id ?? bundleSelector.value);
		if (!id) return;
		selectedBundle = indexedBundles[id] ?? selectedBundle;
		console.log('selectBundle', selectedBundle);
	}

	async function sortItems(items) {
		items.sort(async (a, b) => {
			const aWorkshop = await Steam.getWorkshopAddon(a);
			const bWorkshop = await Steam.getWorkshopAddon(b);
			if ((aWorkshop.dead && bWorkshop.dead) || (aWorkshop.size === bWorkshop.size)) {
				return 0;
			} else if (aWorkshop.size > bWorkshop.size) {
				return 1;
			} else {
				return -1;
			}
		});
	}
	async function sortBundle(bundle) {
		if (bundle.collection) {
			await Promise.all([
				sortItems(bundle.items),
				sortItems(bundle.collection.include),
				sortItems(bundle.collection.exclude)
			]);
		} else {
			await sortItems(bundle.items);
		}
	}
</script>

<svelte:body on:paste={pasteBundle}/>

<main>
	<Modal id="new-bundle" active={newBundle} cancel={() => newBundle = false}>
		<h2>{$_('new_bundle')}</h2>
		<input type="text" placeholder={$_('bundle_name')} id="bundle-name" bind:this={newBundleNameInput} on:input={checkNewBundle} on:change={checkNewBundle}/>
		<div id="collection-link">
			<img src="/img/steam_anonymous.jpg" bind:this={newBundleCollectionImg}/>
			<span>
				<span>
					{#if !!newBundleCollectionName}
						{$_('based_on_collection_name', { values: { name: newBundleCollectionName } })}
					{:else}
						{$_('based_on_collection')}
					{/if}
				</span>
				<span>{$_('optional')}</span>
			</span>
			<input type="text" class:error={newBundleCollectionError} placeholder={$_('collection_link')} bind:this={newCollectionInput} on:input={checkCollectionLink} on:change={checkCollectionLink}/>
		</div>
		<div class="btn validate" on:click={createNewBundle} class:disabled={!newBundleValid} bind:this={newBundleNameCreateBtn}><Plus size="1rem"/>{$_('create_exclamation')}</div>
	</Modal>

	<Modal id="import-bundle" active={importBundle} cancel={() => importBundle = false}>
		<h2>{$_('import_bundle')}</h2>
		<p>{$_('paste_bundle')}</p>
		<div class="btn" on:click={importBundleBrowse}><Folder size="1rem"/>{$_('browse')}</div>
	</Modal>

	{#await bundles}
		{$_('loading')}
	{:then bundles}
		{#if bundles.length === 0}
			<Modal id="bundles-help" active={showHelp} cancel={() => showHelp = false}>
				<h2>{$_('bundles_what')}</h2>
				<p>{$_('bundles_help')}</p>
			</Modal>
			<div id="no-bundles"><div>
				<Dead inline={true} size="4rem"/>
				<h2>{$_('no_bundles')}</h2>
				<a class="color" href="#" on:click={() => showHelp = true}>{$_('bundles_what')}</a><br>
				<div class="btn" on:click={showNewBundle}><Plus size="1rem"/>{$_('new_bundle')}</div>
				<div class="btn" on:click={showImportBundle}><Download size="1rem"/>{$_('import_bundle')}</div>
			</div></div>
		{:else}
			<div id="bundles-nav">
				<div class="icon-button" on:click={showImportBundle} use:tippy={$_('import_bundle')}><Download size="1.2rem"/></div>
				<div class="icon-button" on:click={showNewBundle} use:tippy={$_('new_bundle')}><Plus size="1.2rem"/></div>

				<select bind:this={bundleSelector} on:change={()=>selectBundle()} on:blur={()=>selectBundle()}>
					{#each bundles as bundle}
						<option value={bundle.id}>{bundle.name}</option>
					{/each}
				</select>
			</div>

			{#if selectedBundle}
				<div id="items-grid" class:split={!!selectedBundle.collection} class="hide-scroll">
					<div>
						{#each selectedBundle.items as item}
							<BundleItem {item}/>
						{/each}
					</div>
					{#if selectedBundle.collection}
						<div>
							{#each selectedBundle.collection.include as item}
								<BundleItem {item}/>
							{/each}
							{#each selectedBundle.collection.exclude as item}
								<BundleItem {item}/>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{/if}
	{:catch err}
		<Dead size="2rem"/>&nbsp;{err}
	{/await}
</main>

<style>
	main {
		height: 100%;
		padding: 1.5rem;
		padding-bottom: 0;
		display: flex;
		flex-direction: column;
	}
	#no-bundles {
		height: 100%;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
	}

	:global(#bundles-help > div) {
		padding: 1.5rem;
		width: 28rem;
	}
	:global(#bundles-help h2) {
		margin-top: 0;
	}
	:global(#bundles-help p) {
		margin-bottom: 0;
	}
	p {
		white-space: pre-line;
		line-height: 1.6;
	}

	#no-bundles > div > a {
		margin-bottom: calc(2rem - 4px);
		display: inline-block;
	}
	#no-bundles > div {
		text-align: center;
	}
	#no-bundles > div > .btn {
		display: inline-flex;
		width: 9rem;
	}
	#no-bundles > div > .btn:not(:first-child) {
		margin-left: .5rem;
	}

	.btn {
		cursor: pointer;
		background-color: #313131;
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .7rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.btn.validate {
		transition: background-color .25s;
	}
	.btn.validate:not(.disabled) {
		background-color: #007eff;
	}
	.btn:not(.validate):active {
		transition: none;
		background-color: #252525;
	}
	.btn > :global(.icon) {
		margin-right: .5rem;
	}

	input[type='text'] {
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgba(0, 0, 0, .4);
		padding: .7rem;
		color: #fff;
		font-size: .85em;
		width: 100%;
	}
	input[type='text']:focus {
		box-shadow: inset 0 0 0px 1.5px #127cff;
		outline: none;
	}
	input[type='text'].error {
		outline: none !important;
		box-shadow: inset 0 0 0px 1.5px #a90000 !important;
	}

	:global(#new-bundle > div) {
		padding: 1.5rem;
		width: 30rem;
	}
	:global(#new-bundle > div > .btn) {
		margin-top: 1.5rem;
	}
	:global(#new-bundle > div > h2) {
		margin-top: 0;
		margin-bottom: 1.5rem;
		text-align: center;
	}
	#collection-link {
		display: grid;
		grid-template-columns: min-content 1fr;
		grid-template-rows: 1fr min-content;
		grid-gap: 1rem;
		margin-top: 1.5rem;
	}
	#collection-link > img {
		grid-row: 1 / 3;
		grid-column: 1;
		object-fit: cover;
		width: 4.4rem;
		height: 4.4rem;
	}
	#collection-link > span {
		grid-row: 1;
		grid-column: 2;
	}
	#collection-link > input {
		grid-row: 2;
		grid-column: 2;
	}
	#collection-link > span {
		display: flex;
	}
	#collection-link > span > span:first-child {
		flex: 1;
		margin-right: .5rem;
	}

	:global(#import-bundle > div) {
		padding: 1.5rem;
		width: 25rem;
		text-align: center;
	}
	:global(#import-bundle > div p) {
		margin-top: 1.5rem;
		margin-bottom: 1.5rem;
	}
	:global(#import-bundle h2) {
		margin-top: 0;
	}

	select {
		-webkit-appearance: none;
		-moz-appearance: none;
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		padding: .7rem;
		padding-left: 1rem;
		padding-right: 1rem;
		color: #fff;
		font-size: 1em;
		width: 100%;
		cursor: pointer;
	}
	select:focus {
		box-shadow: inset 0 0 0px 1.5px #127cff;
		outline: none;
	}
	option {
		background: #313131;
		color: #fff;
	}
	option:hover {
		background: #CECECE;
		color: #313131;
	}

	#bundles-nav {
		display: flex;
	}
	#bundles-nav > *:not(:last-child) {
		margin-right: .8rem;
	}
	#bundles-nav > select {
		flex: 1;
	}

	#items-grid {
		margin-top: 1.5rem;
		padding-bottom: 1.5rem;
	}
	#items-grid > div {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
		grid-gap: 1rem;
	}
</style>

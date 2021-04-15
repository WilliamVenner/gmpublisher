<script>
	import { trimPath } from '../addons.js';
	import { _ } from 'svelte-i18n';
	import { tippy } from '../tippy.js';
	import { Folder, Download, FolderAdd } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';

	export let active;
	export let text;
	export let callback;
	export let cancel = null;
	export let gma = null;
	export let forceCreateFolder = false;

	let extractPath = [null, null, AppSettings.create_folder_on_extract];
	let extractPathInput;

	function computeExtractPath(click) {
		if (!click) {
			if (extractPathInput.value.length !== 0) return;
			if (extractPath[0]) return;
		}

		const dest = click ? this.dataset.dest : null;
		switch(this.dataset.dest) {
			case 'tmp':
				extractPath = [dest, trimPath(AppSettings.temp) + PATH_SEPARATOR + 'gmpublisher', AppSettings.create_folder_on_extract];
				break;

			case 'addons':
				extractPath = [dest, trimPath(AppSettings.gmod) + PATH_SEPARATOR + 'garrysmod' + PATH_SEPARATOR + 'addons', AppSettings.create_folder_on_extract];
				break;

			case 'downloads':
				extractPath = [dest, trimPath(AppData.downloads_dir), AppSettings.create_folder_on_extract];
				break;

			default:
				extractPath = [null, null, AppSettings.create_folder_on_extract];
		}

		extractPathInput.value = '';
	}
	function extractDestHover() {
		if (this === extractPathInput) {
			if (this.value.length === 0) {
				extractPath = [null, null, AppSettings.create_folder_on_extract];
			} else {
				extractPath = ['browse', trimPath(this.value), AppSettings.create_folder_on_extract];
			}
		} else {
			computeExtractPath.call(this, false);
		}
	}
	function extractDestInputted() {
		if (this.value === "" || (extractPath[0] !== null && extractPath[0] !== 'browse')) return;
		extractPath = ['browse', trimPath(this.value), AppSettings.create_folder_on_extract];
		this.value = '';
	}
	function extractDestFocused() {
		if (!!extractPath[0]) {
			this.value = extractPath[1];
		}
	}
	function extractDestLostFocus() {
		if (this.value.length > 0 && !!extractPath[1]) {
			this.value = '';
		}
	}
	function extractDestHoverLeave() {
		if (extractPathInput.value.length !== 0) return;
		if (extractPath[0] === null) {
			extractPath = [null, null, AppSettings.create_folder_on_extract];
		}
	}
	function updateExtractDest() {
		if (this.dataset.dest === extractPath[0]) {
			extractPath = [null, null, AppSettings.create_folder_on_extract];
		} else {
			computeExtractPath.call(this, true);
		}
	}
	function extractDestBrowse() {
		if ('browse' === extractPath[0]) {
			extractPath = [null, null, AppSettings.create_folder_on_extract];
		} else {
			invoke('prompt_path_dialog', {

				directory: true,
				multiple: false,
				save: false,
				defaultPath: AppSettings.destinations[0],

			}).then(path => {
				if (!!path)
					extractPath = ['browse', trimPath(path[0]), AppSettings.create_folder_on_extract]
			});
		}
	}
	function extractableHistoryPath() {
		extractPath = ['browse', trimPath(this.textContent), AppSettings.create_folder_on_extract];
	}
	function createFolderUpdated() {
		AppSettings.create_folder_on_extract = this.checked;
		extractPath = [extractPath[0], extractPath[1], this.checked];
	}

	function doCallback() {
		if (!this.classList.contains('disabled'))
			callback(extractPath);
	}

	let destinationModal;
	function doCancel(e) {
		if (cancel && e.target === destinationModal)
			cancel();
	}
</script>

<main on:click={doCancel} bind:this={destinationModal} class:active={active}><div>
	<h1>{$_('extract_where_to')}</h1>
	<h4>{$_('extract_overwrite_warning')}</h4>

	<input type="text" name="path" on:input={extractDestHover} on:focus={extractDestFocused} on:blur={extractDestLostFocus} on:change={extractDestInputted} bind:this={extractPathInput} placeholder={extractPath[0] ? (extractPath[1] + ((forceCreateFolder || extractPath[2]) ? (PATH_SEPARATOR + (gma?.extracted_name ?? 'addon_name')) : '')) : (gma?.extracted_name ?? 'addon_name')}/>

	{#if extractPath[0] === 'browse' && !forceCreateFolder}
		<div id="checkbox">
			<label>
				<input type="checkbox" id="named" name="named" on:change={createFolderUpdated} checked={AppSettings.create_folder_on_extract}>
				<span>{$_('create_folder')}</span>
			</label>
		</div>
	{/if}

	<div id="destinations">
		<div class="destination" class:active={extractPath[0] === 'browse'} on:hover={extractDestHover} on:click={extractDestBrowse} data-dest="browse">
			<Folder/>
			<div>{$_('browse')}</div>
		</div>

		<div class="destination" class:disabled={!!!AppSettings.temp} class:active={extractPath[0] === 'tmp'} use:tippy={$_('extract_open_tip')} on:mouseover={extractDestHover} on:click={updateExtractDest} on:mouseleave={extractDestHoverLeave} data-dest="tmp">
			<FolderAdd/>
			<div>{$_('open')}</div>
		</div>

		<div class="destination" class:disabled={!!!AppSettings.gmod} class:active={extractPath[0] === 'addons'} on:mouseover={extractDestHover} on:mouseleave={extractDestHoverLeave} on:click={updateExtractDest} data-dest="addons">
			<img src="/img/gmod.svg"/>
			<div>{$_('addons_folder')}</div>
		</div>

		<div class="destination" class:disabled={!!!AppData.downloads_dir} class:active={extractPath[0] === 'downloads'} on:mouseover={extractDestHover} on:mouseleave={extractDestHoverLeave} on:click={updateExtractDest} data-dest="downloads">
			<Download/>
			<div>{$_('downloads_folder')}</div>
		</div>
	</div>

	{#if AppSettings.destinations.length > 0}
		<div id="history" class="hide-scroll">
			{#each AppSettings.destinations as path}
				<div on:click={extractableHistoryPath} class:active={extractPath[0] === 'browse' && extractPath[1] === path}>{path}</div>
			{/each}
		</div>
	{/if}

	<div class="extract-btn" on:click={doCallback} class:disabled={!extractPath[0]}>{text}</div>
</div></main>

<style>
	main {
		pointer-events: none;

		transition: backdrop-filter .25s, background-color .25s;
		background-color: rgba(0,0,0,0);

		z-index: 4;
		position: relative;
		width: 100%;
		height: 100%;
	}
	main.active {
		pointer-events: all;

		backdrop-filter: grayscale(1) blur(1px);
		background-color: rgba(0,0,0,.4);
	}
	main.active > div {
		transform: scale(1, 1);
	}
	main > div {
		transition: transform .25s;
		transform: scale(0, 0);

		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		margin: auto;

		text-align: center;
		padding: 1.5rem;
		background-color: #1a1a1a;
		border-radius: .3rem;
		box-shadow: 0 0 10px rgba(0, 0, 0, .25);

		width: min-content;
		height: min-content;
		max-width: 90%;
		max-height: 90%;

		display: flex;
		flex-direction: column;
	}
	main > div > h1 {
		margin-top: 0;
		margin-bottom: 0;
	}
	main > div > h4 {
		margin-top: .8rem;
		margin-bottom: 1.5rem;
	}

	#destinations {
		display: grid;
		grid-template-rows: 7rem;
		grid-template-columns: 7rem 7rem 7rem 7rem;
		grid-gap: 1rem;
	}
	#destinations .destination {
		border-radius: .4rem;
		background-color: #292929;
		box-shadow: 0 0 6px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		cursor: pointer;
		height: 7rem;
		width: 7rem;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
	}
	#destinations .destination.disabled {
		cursor: default !important;
		pointer-events: none !important;
		filter: brightness(0.5) grayscale(1);
	}
	#destinations .destination:active,
	#destinations .destination.active {
		background-color: #0e0e0e;
	}
	#destinations .destination img, #destinations .destination :global(.icon) {
		height: 2.5rem;
		margin-bottom: .6rem;
	}
	#destinations .destination > div {
		white-space: nowrap;
	}
	main input[type='text'] {
		appearance: none;
		border: none;
		border-radius: 0;
		text-align: left;
		display: block;
		margin-bottom: .8rem;
		padding: .8rem;
		background-color: #0e0e0e;
		width: 100%;
		font: inherit;
		color: #fff;
		font-size: .9em;
	}
	main input[type='text']:focus {
		outline: none;
	}
	main input[type='text']:placeholder-shown {
		text-align: center;
	}
	#history {
		flex: 1;
		overflow: auto;
		margin-top: 1.5rem;
		background-color: #292929;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		border-radius: .4rem;
	}
	#history > div {
		padding: .6rem;
		font-size: .9em;
		text-align: left;
		cursor: pointer;
		transition: background-color .1s;
		word-break: break-all;
	}
	#history > div:nth-child(2n-1) {
		background-color: rgb(0, 0, 0, .12);
	}
	#history > div.active {
		background-color: #0e0e0e;
	}
	main #checkbox {
		margin-bottom: 1rem;
		display: inline-flex;
		justify-content: center;
		align-items: center;
	}
	main #checkbox label {
		cursor: pointer;
	}
	main #checkbox label > * {
		vertical-align: middle;
	}
	main .extract-btn {
		margin-top: 1rem;
	}

	.extract-btn {
		padding: .7rem;
		text-align: center;
		background-color: #006cc7;
		z-index: 3;
		box-shadow: 0 0 5px rgba(0, 0, 0, .1);
		cursor: pointer;
		text-shadow: 0px 1px 0px rgba(0, 0, 0, .6);
		line-height: 1;
		transition: background-color .5s;
	}
	.extract-btn.disabled {
		background-color: rgb(59, 59, 59);
	}
</style>

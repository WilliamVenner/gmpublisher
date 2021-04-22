<script>
	import { preparePublish } from '../pages/MyWorkshop.svelte';
	import { _ } from 'svelte-i18n';
	import Modal from '../components/Modal.svelte';
	import { Cross, Folder } from 'akar-icons-svelte';
	import { tippyFollow } from '../tippy';
	import * as dialog from '@tauri-apps/api/dialog';
	import { invoke } from '@tauri-apps/api/tauri';
	import { playSound } from '../sounds';
	import FileBrowser from './FileBrowser.svelte';
	import filesize from 'filesize';

	function togglePreparePublish() {
		$preparePublish = !$preparePublish;
	}

	let gmaIcon;
	let gmaIconPath = null;
	let gmaEntries = [];
	let gmaSize;
	let ignoreGlobs = [];

	let pathInput;
	let pathInputContainer;
	let pathValue = '';
	let pathFailMessage = null;
	function browseAddon() {
		dialog.open({ directory: true }).then(path => {
			if (path && path.length > 0) {
				checkPath(path);
			}
		});
	}

	function browseIcon() {
		dialog.open({

			filters: [{
				extensions: ['jpg', 'jpeg', 'png', 'gif'],
				name: $_('icon_file_pick')
			}]

		}).then(async path => {
			if (path) {
				const size = await invoke('file_size', { path });
				if (size === null) {
					playSound('error');
					alert($_('ERR_IO_ERROR'));
				} else if (size > 1000000) {
					playSound('error');
					alert($_('ERR_ICON_TOO_LARGE'));
				} else if (size < 16) {
					playSound('error');
					alert($_('ERR_ICON_TOO_SMALL'));
				} else {
					playSound('success');
					gmaIconPath = path;
				}
			}
		});
	}
	function removeIcon() {
		gmaIconPath = null;
	}

	function checkPath(path) {
		invoke('verify_whitelist', { path, ignore: ignoreGlobs }).then(([entries, size]) => {

			console.log(entries);

			gmaEntries = entries;
			gmaSize = size;

			pathFailMessage = null;
			tippyFollow(pathInputContainer, pathFailMessage);
			playSound('success');

			pathValue = path;

		}, ([err, failed_paths]) => {

			gmaEntries = [];
			pathValue = pathInput.value;

			if (err === "ERR_WHITELIST") {
				pathFailMessage = $_('ERR_WHITELIST') + '\n\n' + failed_paths.join('\n');
			} else {
				pathFailMessage = $_(err);
			}

			tippyFollow(pathInputContainer, pathFailMessage);
			playSound('error');

			pathValue = path;

		});

	}

	function onPathChanged() {
		if (this.value.length > 0) {
			checkPath(this.value);
		} else {
			pathFailMessage = null;
			tippyFollow(pathInputContainer, pathFailMessage);
			pathValue = '';
		}
	}

	let chosenAddonTags = [null, null, null];
	let addonTags = ['fun', 'roleplay', 'scenic', 'movie', 'realism', 'cartoon', 'water', 'comic', 'build'];
	function tagChosen() {
		const chosen = [];
		document.querySelectorAll('.tag-choice').forEach((choice, i) => {
			if (choice.value !== 'default') {
				if (chosen.findIndex(choice => choice === choice.value) !== -1) {
					choice.value = 'default';
					chosen[i] = null;
				} else {
					chosen[i] = choice.value;
				}
			}
		});
		chosenAddonTags = chosen;
		console.log(chosenAddonTags);
	}

	function removeIgnore() {
		// TODO
	}

	function openEntry(path) {
		invoke('open', { path: pathValue + PATH_SEPARATOR + path });
	}

	function openAddon() {
		invoke('open', { path: pathValue });
	}
</script>

<Modal id="prepare-publish" active={$preparePublish} cancel={togglePreparePublish}>
	<div id="icon-container">
		{#if gmaIconPath}
			<img id="icon" src="{gmaIconPath}" bind:this={gmaIcon}/>
			<div class="btn" on:click={removeIcon}><Cross size="1rem"/>{$_('remove_icon')}</div>
		{:else}
			<img id="icon" src="/img/gmpublisher_default_icon.png" bind:this={gmaIcon}/>
			<div class="btn" on:click={browseIcon}><Folder size="1rem"/>{$_('browse')}</div>
		{/if}
		<p>{$_('icon_instructions')}</p>
	</div>
	<div id="details-container">
		<div class="path-container" bind:this={pathInputContainer}>
			<input type="text" class:error={pathFailMessage?.length > 0} bind:this={pathInput} id="path" placeholder={$_('addon_path')} required on:change={onPathChanged} value={pathValue}/>
			<div class="browse icon-button" on:click={browseAddon}><Folder size="1rem"/></div>
		</div>

		<input type="text" id="title" placeholder={$_('addon_title')} required/>

		<select id="addon-type">
			<option value="default" selected hidden disabled>{$_('addon_type')}</option>
			<option value="ServerContent">{$_('addon_types.ServerContent')}</option>
			<option value="gamemode">{$_('addon_types.gamemode')}</option>
			<option value="map">{$_('addon_types.map')}</option>
			<option value="weapon">{$_('addon_types.weapon')}</option>
			<option value="vehicle">{$_('addon_types.vehicle')}</option>
			<option value="npc">{$_('addon_types.npc')}</option>
			<option value="tool">{$_('addon_types.tool')}</option>
			<option value="effects">{$_('addon_types.effects')}</option>
			<option value="model">{$_('addon_types.model')}</option>
		</select>

		<div id="addon-tags">
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[0] ?? 'default'}>
				<option value="default" hidden disabled>{$_('tag_1')}</option>
				{#each addonTags as tag}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[1] ?? 'default'}>
				<option value="default" hidden disabled>{$_('tag_2')}</option>
				{#each addonTags as tag}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[2] ?? 'default'}>
				<option value="default" hidden disabled>{$_('tag_3')}</option>
				{#each addonTags as tag, i}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
		</div>

		{#if pathValue.length > 0}
			<FileBrowser browsePath={pathValue.length > 0 ? pathValue : null} entriesList={gmaEntries} {openEntry} open={openAddon} size={gmaSize}/>
		{/if}
	</div>
	<div id="ignore">
		<div class="title">{$_('ignored_file_patterns')}</div>
		<input type="text" placeholder={$_('add_ellipsis')}/>
		<div class="hide-scroll">
			{#each ignoreGlobs as ignore}
				<div on:click={removeIgnore}>{ignore}</div>
			{/each}
			{#each window.DEFAULT_IGNORE_GLOBS as ignore}
				<div class="default">{ignore}</div>
			{/each}
		</div>
	</div>
</Modal>

<style>
	:global(#prepare-publish > div) {
		display: flex;
		width: 55rem;
		padding: 1.5rem;
	}

	#icon {
		background-image: url('/img/transparency.svg');
		width: 16rem;
		height: 16rem;
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
	.path-container {
		display: flex;
	}
	.path-container > input {
		flex: 1;
	}
	.browse {
		margin-left: .75rem;
		padding: .7rem;
		width: 2.4rem;
		height: 2.4rem;
		display: flex;
	}
	#icon-container > .btn {
		cursor: pointer;
		background: #313131;
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .7rem;
		margin-top: 1rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	#icon-container > .btn:active {
		background: #252525;
	}
	#icon-container > .btn > :global(.icon) {
		margin-right: .5rem;
	}

	p {
		white-space: pre-line;
		line-height: 1.6;
		margin-top: 1rem;
		margin-bottom: 0;
	}

	#addon-type {
		display: block;
	}

	#addon-tags {
		display: flex;
	}
	#addon-tags select {
		flex: 1;
		flex-basis: 0;
	}

	#details-container {
		flex: 1;
		margin-left: 1.5rem;
		margin-right: 1.5rem;
	}
	#details-container > *:not(:last-child) {
		margin-bottom: 1rem;
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
		color: #fff;
		font-size: .85em;
		width: 100%;
		cursor: pointer;
		text-align: center;
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
	select:nth-child(2) {
		margin-left: 1rem;
		margin-right: 1rem;
	}

	#ignore > .title {
		text-align: center;
		margin-bottom: 1rem;
	}
	#ignore > .hide-scroll {
		flex: 1;
		overflow: auto;
		margin-top: 1rem;
		background-color: #292929;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		border-radius: .4rem;
	}
	#ignore > .hide-scroll > div {
		padding: .6rem;
		font-size: .9em;
		text-align: left;
		transition: background-color .1s;
		word-break: break-all;
	}
	#ignore > .hide-scroll > div:not(.default) {
		cursor: pointer;
	}
	#ignore > .hide-scroll > div:nth-child(2n-1) {
		background-color: rgb(0, 0, 0, .12);
	}
</style>

<script>
	import { preparePublish } from '../pages/MyWorkshop.svelte';
	import { _ } from 'svelte-i18n';
	import Modal from '../components/Modal.svelte';
	import { CloudUpload, Cross, Folder } from 'akar-icons-svelte';
	import { tippyFollow } from '../tippy';
	import * as dialog from '@tauri-apps/api/dialog';
	import { invoke } from '@tauri-apps/api/tauri';
	import { playSound } from '../sounds';
	import FileBrowser from './FileBrowser.svelte';
	import { writable } from 'svelte/store';
	import { Transaction } from '../transactions';
	import filesize from 'filesize';

	function togglePreparePublish() {
		$preparePublish = !$preparePublish;
	}

	let gmaIcon;
	let gmaIconPath = null;
	let gmaEntries = writable([]);
	let gmaSize;
	let readyForPublish = false;
	let ignoreGlobs = AppSettings.ignore_globs;

	let titleInput;
	let addonTypeInput;

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
					gmaIconPath = path;
				}
			}
		});
	}
	function removeIcon() {
		gmaIconPath = null;
	}

	function checkPath(path, successSound) {
		invoke('verify_whitelist', { path, ignore: ignoreGlobs }).then(([entries, size]) => {

			$gmaEntries = entries;
			gmaSize = size;

			pathFailMessage = null;
			tippyFollow(pathInputContainer, pathFailMessage);

			pathValue = path;

			if (successSound) playSound('success');

		}, ([err, failed_paths]) => {

			$gmaEntries = [];
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

	function onPathChanged(path) {
		if (path.length > 0) {
			checkPath(path, true);
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
	}

	function openEntry(path) {
		invoke('open', { path: pathValue + PATH_SEPARATOR + path });
	}

	function openAddon() {
		invoke('open', { path: pathValue });
	}

	function ignoreKeyPress(e) {
		if (e.which === 13 || e.keyCode === 13 || e.key === 'Enter') {
			e.preventDefault();
			const ignore = this.value.trim();
			if (ignore.length > 0 && ignoreGlobs.findIndex(s => s === ignore) === -1) {
				ignoreGlobs.push(ignore);
				ignoreGlobs = ignoreGlobs;
				invoke('update_settings', { settings: AppSettings });
				if (pathValue.length > 0) checkPath(pathValue);
			}
			this.value = '';
		}
	}
	function removeIgnore() {
		const ignore = this.innerText;
		const index = ignoreGlobs.findIndex(s => s === ignore);
		if (index !== -1) {
			ignoreGlobs.splice(index, 1);
			ignoreGlobs = ignoreGlobs;
			invoke('update_settings', { settings: AppSettings });
			if (pathValue.length > 0) checkPath(pathValue);
		}
	}

	function publish() {
		if (!readyForPublish) return;
		playSound('success');
		invoke('publish', {
			contentPath: pathValue,
			title: titleInput.value.trim(),
			tags: chosenAddonTags.filter(tag => !!tag),
			addonType: addonTypeInput.value
		}).then(transactionId => {
			const transaction = new Transaction(transactionId, transaction => {
				return $_(transaction.status ?? 'PUBLISH_PACKING', { values: {
					pct: transaction.progress,
					data: filesize((transaction.progress / 100) * gmaSize),
					dataTotal: filesize(gmaSize)
				}});
			});

			transaction.listen(event => {
				if (event.finished) {
					const [id, not_accepted_legal_agreement] = event.data;

					if (not_accepted_legal_agreement) {
						invoke('open', { path: 'https://steamcommunity.com/workshop/workshoplegalagreement/' });
					}

					invoke('open', { path: 'https://steamcommunity.com/sharedfiles/filedetails/?id=' + id });
				}
			});
		});
	}

	function isFormValid() {
		if (pathValue.length === 0 || pathFailMessage !== null) return false;

		let chosenAddonTag = false;
		for (let i = 0; i < chosenAddonTags.length; i++) {
			if (chosenAddonTags[i] !== null) {
				chosenAddonTag = true;
				break;
			}
		}
		if (!chosenAddonTag) return false;

		if (titleInput.value.trim().length === 0) return false;

		if (addonTypeInput.value === 'default') return false;

		return true;
	}

	function checkForm() {
		const isValid = isFormValid();
		if (isValid !== readyForPublish) {
			readyForPublish = isValid;
			if (readyForPublish) {
				playSound('btn-on');
			} else {
				playSound('btn-off');
			}
		}
	}
</script>

<Modal id="prepare-publish" active={$preparePublish} cancel={togglePreparePublish}>
	<div id="details-container">
		{#if gmaIconPath}
			<div id="icon-container" on:click={browseIcon}>
				<div id="addon-icon-background" style="background-image: url('{gmaIconPath}')"></div>
				<div id="addon-icon"><img src={gmaIconPath} bind:this={gmaIcon}/></div>
			</div>
			<div id="icon-browse" on:click={removeIcon}><Cross size="1rem"/>{$_('remove_icon')}</div>
		{:else}
			<div id="icon-container" on:click={browseIcon}>
				<div id="addon-icon-background" style="background-image: url('/img/gmpublisher_default_icon.png')"></div>
				<div id="addon-icon"><img src="/img/gmpublisher_default_icon.png" bind:this={gmaIcon}/></div>
			</div>
			<div id="icon-browse" on:click={browseIcon}><Folder size="1rem"/>{$_('browse')}</div>
		{/if}
		<p>{$_('icon_instructions')}</p>

		<div class="path-container" bind:this={pathInputContainer}>
			<input type="text" class:error={pathFailMessage?.length > 0} bind:this={pathInput} id="path" placeholder={$_('addon_path')} required on:change={() => onPathChanged(this.value)} value={pathValue}/>
			<div class="browse icon-button" on:click={browseAddon}><Folder size="1rem"/></div>
		</div>

		<input type="text" id="title" placeholder={$_('addon_title')} bind:this={titleInput} on:input={checkForm} on:change={checkForm}/>

		<select id="addon-type" bind:this={addonTypeInput} on:blur={checkForm} on:change={checkForm}>
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

		<div id="addon-tags" on:blur={checkForm} on:change={checkForm}>
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

		<div id="publish-btn" on:click={publish} class:disabled={!readyForPublish}><CloudUpload size="1.1rem"/>{$_('publish_exclamation')}</div>
	</div>

	<div id="file-browser-container">
		<FileBrowser fileSelect={path => onPathChanged.call(pathInput, path)} background={true} browsePath={pathValue.length > 0 ? pathValue : null} entriesList={gmaEntries} {openEntry} open={openAddon} size={gmaSize}/>
	</div>

	<div id="ignore">
		<div class="title">{$_('ignored_file_patterns')}</div>
		<input type="text" placeholder={$_('add_ellipsis')} on:keypress={ignoreKeyPress}/>
		<div class="hide-scroll">
			{#each ignoreGlobs as ignore}
				<div on:click={removeIgnore}>{ignore}</div>
			{/each}
			{#each window.DEFAULT_IGNORE_GLOBS as ignore}
				<div class="default" use:tippyFollow={$_('ignored_for_convenience')}>{ignore}</div>
			{/each}
		</div>
	</div>
</Modal>

<style>
	:global(#prepare-publish > div) {
		display: grid;
		width: 70rem;
		min-height: 0;
		height: 42rem;
		padding: 1.5rem;
		grid-template-rows: 1fr;
		grid-template-columns: 18rem 1fr 14rem;
		grid-gap: 1.5rem;
	}

	#file-browser-container > :global(#file-browser) {
		height: 100%;
		border-radius: .4rem;
		overflow: hidden;
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
	#icon-container {
		position: relative;
		overflow: hidden;
		cursor: pointer;
	}
	#icon-browse {
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
	#icon-browse:active {
		background: #252525;
	}
	#icon-browse > :global(.icon) {
		margin-right: .5rem;
	}
	#addon-icon {
		text-align: center;
	}
	#addon-icon > img {
		background-image: url('/img/transparency.svg');
		width: 15rem;
		height: 15rem;
	}
	#addon-icon-background {
		position: absolute;
		width: calc(100% + 10px);
		height: calc(100% + 10px);
		background-size: cover;
		filter: blur(5px);
		left: -5px;
		top: -5px;
		z-index: -1;
	}

	p {
		white-space: pre-line;
		line-height: 1.6;
		margin-top: 1rem;
		margin-bottom: 0;
		text-align: center;
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
		text-align-last: center;
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

	#ignore {
		display: flex;
		flex-direction: column;
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
	#ignore > .hide-scroll > div.default {
		color: rgba(255,255,255,.5);
	}
	#ignore > .hide-scroll > div:nth-child(2n-1) {
		background-color: rgb(0, 0, 0, .12);
	}

	#publish-btn {
		padding: .7rem;
		text-align: center;
		background-color: #006cc7;
		z-index: 3;
		box-shadow: 0 0 5px rgba(0, 0, 0, .1);
		cursor: pointer;
		text-shadow: 0px 1px 0px rgba(0, 0, 0, .6);
		line-height: 1;
		border-radius: 4px;
		transition: background-color .5s;
	}
	#publish-btn.disabled {
		background-color: #313131;
	}
	#publish-btn > :global(.icon) {
		margin-right: .25rem;
	}
</style>

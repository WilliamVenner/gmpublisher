<script context="module">
	export const isPublishing = writable(false);
</script>

<script>
	import { _ } from 'svelte-i18n';
	import Modal from '../components/Modal.svelte';
	import { CloudUpload, Cross, Folder, LinkOut } from 'akar-icons-svelte';
	import { tippyFollow } from '../tippy';
	import * as dialog from '@tauri-apps/api/dialog';
	import { invoke } from '@tauri-apps/api/tauri';
	import { playSound } from '../sounds';
	import FileBrowser from './FileBrowser.svelte';
	import { writable } from 'svelte/store';
	import { Transaction } from '../transactions';
	import filesize from 'filesize';
	import Loading from './Loading.svelte';
	import { translateError } from '../i18n';
	import { onMount } from 'svelte';

	export let updatingAddon = null;
	export let remountAddonScroller;
	export let preparePublish;

	function togglePreparePublish() {
		$preparePublish = !$preparePublish;
	}

	let gmaIcon;
	let gmaIconPath = null;
	let gmaIconBase64;
	let gmaEntries = writable([]);
	let gmaSize;
	let readyForPublish = false;
	let ignoreGlobs = AppSettings.ignore_globs;

	let titleInput;
	let addonTypeInput;
	let changeLog;

	let upscale;
	let canUpscale = false;

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

		}).then(path => {
			if (path) {
				invoke('verify_icon', { path }).then(([base64, can_upscale]) => {
					canUpscale = can_upscale;
					gmaIconPath = path;
					gmaIconBase64 = base64;
				}, transactionId => new Transaction(transactionId, () => ''));
			}
		});
	}
	function removeIcon() {
		gmaIconPath = null;
		gmaIconBase64 = null;
		canUpscale = false;
	}

	function checkPath(path, successSound) {
		return invoke('verify_whitelist', { path, ignore: ignoreGlobs }).then(([entries, size]) => {

			$gmaEntries = entries;
			gmaSize = size;

			pathFailMessage = null;
			tippyFollow(pathInputContainer, pathFailMessage);

			pathValue = path;

			if (successSound) playSound('success');

		}, (err) => {

			$gmaEntries = [];
			pathValue = pathInput.value;
			pathFailMessage = translateError(err);

			tippyFollow(pathInputContainer, pathFailMessage);
			playSound('error');

			pathValue = path;

		});
	}

	async function onPathChanged(path, playSound) {
		if (path.length > 0) {
			await checkPath(path, playSound);
		} else {
			pathFailMessage = null;
			tippyFollow(pathInputContainer, pathFailMessage);
			pathValue = '';
		}
	}

	let chosenAddonTags = [null, null, null];
	const addonTags = ['fun', 'roleplay', 'scenic', 'movie', 'realism', 'cartoon', 'water', 'comic', 'build'];
	const addonTypes = ['ServerContent', 'gamemode', 'map', 'weapon', 'vehicle', 'npc', 'tool', 'effects', 'model'];
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

	async function publish() {
		if (!readyForPublish || $isPublishing) return;
		$isPublishing = true;
		playSound('success');

		invoke('publish', {

			contentPathSrc: pathValue,

			title: titleInput.value.trim(),
			tags: chosenAddonTags.filter(tag => !!tag),
			addonType: addonTypeInput.value,

			iconPath: gmaIconPath,
			upscale: canUpscale && upscale.checked,

			updateId: $updatingAddon ? ((await $updatingAddon).id) : undefined,
			changes: changeLog ? changeLog.value : null,

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
					$remountAddonScroller = !$remountAddonScroller;
				}

				if (event.finished || event.error) {
					$isPublishing = false;
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

	function checkForm(sound) {
		const isValid = isFormValid();
		if (isValid !== readyForPublish) {
			readyForPublish = isValid;
			if (sound !== false) {
				if (readyForPublish) {
					playSound('btn-on');
				} else {
					playSound('btn-off');
				}
			}
		}
	}

	const tagSearchMax = Math.max(addonTypes.length, addonTags.length);
	let updatingAddonSubscription;
	onMount(() => updatingAddonSubscription = updatingAddon.subscribe(async updatingAddon => {
		if (changeLog) changeLog.value = '';

		if (!updatingAddon) {
			gmaIconPath = null;
			gmaIconBase64 = null;
			$gmaEntries = [];
			gmaSize = null;
			readyForPublish = false;
			titleInput.value = '';
			addonTypeInput.value = 'default';
			upscale.checked = AppSettings.upscale_addon_icon;
			upscale = upscale;
			canUpscale = false;
			pathInput.value = '';
			pathValue = '';
			pathFailMessage = null;
			return;
		}

		canUpscale = false;

		gmaIconPath = null;
		gmaIconBase64 = updatingAddon.previewUrl;

		if (updatingAddon.id in AppSettings.my_workshop_local_paths) {
			await onPathChanged(AppSettings.my_workshop_local_paths[updatingAddon.id]);
		} else {
			pathValue = '';
			pathFailMessage = null;
		}

		addonTypeInput.value = 'default';
		addonTypeInput = addonTypeInput;

		chosenAddonTags = [null, null, null];
		let chosen = 0;
		for (let i = 0; i < updatingAddon.tags.length; i++) {
			if (updatingAddon.tags[i] === 'ServerContent') {
				addonTypeInput.value = 'ServerContent';
				addonTypeInput = addonTypeInput;
				continue;
			}
			if (updatingAddon.tags[i] === 'Addon') continue;

			const tag = updatingAddon.tags[i].toLowerCase();
			let search = -1;
			while (++search < tagSearchMax) {
				if (addonTags[search] === tag) {
					chosenAddonTags[chosen++] = tag;
				} else if (addonTypes[search] === tag) {
					addonTypeInput.value = tag;
					addonTypeInput = addonTypeInput;
				}
			}
		}
		chosenAddonTags = chosenAddonTags;

		titleInput.value = updatingAddon.title;
		titleInput = titleInput;

		checkForm(false);
	}));
</script>

<Modal id="prepare-publish" active={$preparePublish} cancel={togglePreparePublish}>
	<div id="details-container">
		{#if $updatingAddon}
			<div id="ws-link"><a class="color" href="https://steamcommunity.com/sharedfiles/filedetails/?id={$updatingAddon.id}" target="_blank">Steam Workshop<LinkOut size=".8rem"/></a></div>
		{/if}

		{#if gmaIconBase64}
			<div id="icon-container" on:click={browseIcon}>
				<div id="addon-icon-background" style="background-image: url('{gmaIconBase64}')"></div>
				{#if canUpscale && upscale.checked}
					<div id="addon-icon" class="upscale">
						<img src={gmaIconBase64} bind:this={gmaIcon}/>
						<img src="/img/gmpublisher_default_icon.png"/>
					</div>
				{:else}
					<div id="addon-icon">
						<img src={gmaIconBase64} bind:this={gmaIcon}/>
					</div>
				{/if}
			</div>
		{:else}
			<div id="icon-container" on:click={browseIcon}>
				<div id="addon-icon-background" style="background-image: url('/img/gmpublisher_default_icon.png')"></div>
				<div id="addon-icon"><img src="/img/gmpublisher_default_icon.png" bind:this={gmaIcon}/></div>
			</div>
		{/if}
		{#if gmaIconBase64 && !$updatingAddon}
			<div id="icon-browse" on:click={removeIcon}><Cross size="1rem"/>{$_('remove_icon')}</div>
		{:else}
			<div id="icon-browse" on:click={browseIcon}><Folder size="1rem"/>{$_('browse')}</div>
		{/if}
		<p>{$_('icon_instructions')}</p>
		<div id="upscale-container">
			<label class:disabled={!canUpscale}>
				<input type="checkbox" id="upscale" bind:this={upscale} checked={AppSettings.upscale_addon_icon} disabled={!canUpscale} on:change={() => upscale = upscale}/>
				{$_('upscale_addon_icon')}
			</label>
		</div>

		<div class="path-container" bind:this={pathInputContainer}>
			<input type="text" class:error={pathFailMessage?.length > 0} bind:this={pathInput} id="path" placeholder={$_('addon_path')} required on:change={() => onPathChanged(pathInput.value, true)} value={pathValue}/>
			<div class="browse icon-button" on:click={browseAddon}><Folder size="1rem"/></div>
		</div>

		<input type="text" id="title" placeholder={$_('addon_title')} bind:this={titleInput} on:input={checkForm} on:change={checkForm}/>

		<select id="addon-type" bind:this={addonTypeInput} on:blur={checkForm} on:change={checkForm}>
			<option value="default" selected hidden disabled>{$_('addon_type')}</option>
			{#each addonTypes as addonType}
				<option value={addonType}>{$_('addon_types.' + addonType)}</option>
			{/each}
		</select>

		<div id="addon-tags" on:blur={checkForm} on:change={checkForm}>
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[0] ?? 'default'}>
				<option value="default">{$_('tag_1')}</option>
				{#each addonTags as tag}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[1] ?? 'default'}>
				<option value="default">{$_('tag_2')}</option>
				{#each addonTags as tag}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
			<select on:change={tagChosen} class="tag-choice" value={chosenAddonTags[2] ?? 'default'}>
				<option value="default">{$_('tag_3')}</option>
				{#each addonTags as tag}
					{#if chosenAddonTags.findIndex(choice => choice === tag) === -1}
						<option value={tag}>{$_('addon_tags.' + tag)}</option>
					{:else}
						<option value={tag} disabled>{$_('addon_tags.' + tag)}</option>
					{/if}
				{/each}
			</select>
		</div>

		{#if $updatingAddon}
			<div id="publish-btn" on:click={publish} class:disabled={!readyForPublish || $isPublishing} use:tippyFollow={$_('update_warning', { values: { title: $updatingAddon.title, id: $updatingAddon.id } })}>
				{#if $isPublishing}
					<Loading size="1.1rem"/>
				{:else}
					<CloudUpload size="1.1rem"/>{$_('update_exclamation')}
				{/if}
			</div>
		{:else}
			<div id="publish-btn" on:click={publish} class:disabled={!readyForPublish || $isPublishing}>
				{#if $isPublishing}
					<Loading size="1.1rem"/>
				{:else}
					<CloudUpload size="1.1rem"/>{$_('publish_exclamation')}
				{/if}
			</div>
		{/if}
	</div>

	<div id="middle-column">
		<FileBrowser fileSelect={path => onPathChanged(path)} background={true} browsePath={pathValue.length > 0 ? pathValue : null} entriesList={gmaEntries} {openEntry} open={openAddon} size={gmaSize}/>

		{#if $updatingAddon}
			<div id="changes-container">
				<textarea id="changes" bind:this={changeLog} required></textarea>
				<div>{$_('changelog')}</div>
			</div>
		{/if}
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
		display: flex;
		width: 70rem;
		min-height: 0;
		height: min-content;
		padding: 1.5rem;
	}
	#details-container {
		width: 18rem;
	}
	#middle-column {
		flex: 1;
		margin-left: 1.5rem;
		margin-right: 1.5rem;
		display: flex;
		flex-direction: column;
	}

	#middle-column > :global(#file-browser) {
		border-radius: .4rem;
		overflow: hidden;
		flex: 1;
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
		background-color: #101010;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		border-radius: .4rem;
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
		width: 100%;
		height: 15rem;
		position: relative;
	}
	#addon-icon img {
		background-image: url('/img/transparency.svg');
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		max-width: 100%;
		max-height: 100%;
		display: block;
	}
	#addon-icon:not(.upscale) img {
		position: absolute;
	}
	#addon-icon.upscale {
		width: max-content;
		height: 15rem;
		position: relative;
		display: block;
		margin: auto;
	}
	#addon-icon.upscale img {
		height: 100%;
	}
	#addon-icon.upscale > img:first-child {
		position: absolute;
		width: 100%;
	}
	#addon-icon.upscale > img:last-child {
		opacity: 0;
	}
	#addon-icon-background {
		position: absolute;
		width: calc(100% + 10px);
		height: calc(100% + 10px);
		background-size: cover;
		background-position: 50% 50%;
		filter: blur(5px);
		left: -5px;
		top: -5px;
		z-index: 0;
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
		width: 14rem;
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

	#upscale-container > label.disabled {
		opacity: .5;
	}
	#upscale-container > label:not(.disabled) {
		cursor: pointer;
	}
	#upscale-container > label {
		display: flex;
		justify-content: center;
	}
	#upscale-container input {
		margin-right: .5rem;
	}

	#changes-container {
		position: relative;
	}
	#changes-container > div {
		position: absolute;
		top: 1.5rem;
		left: 0;
		right: 0;
		bottom: 0;
		margin: auto;
		font-size: 1.3em;
		text-shadow: 0px 1px 0px rgb(0 0 0 / 60%);
		opacity: .5;
		pointer-events: none;
		width: min-content;
		height: min-content;
		z-index: 2;
	}
	#changes-container:focus-within > div, #changes:not(:invalid) + div {
		opacity: 0;
	}
	#changes {
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
		min-height: 12.35rem;
		height: 12.35rem;
		max-height: 12.35rem;
		margin-top: 1.5rem;
		resize: none;
		z-index: 1;
		display: block;
	}
	#changes:focus {
		box-shadow: inset 0 0 0px 1.5px #127cff;
		outline: none;
	}

	#ws-link {
		margin-bottom: 1.2rem;
		text-align: center;
	}
	#ws-link :global(.icon) {
		margin-left: .2rem;
	}
</style>

<script>
	import { getFileTypeInfo } from '../steam.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow } from '../tippy.js';
	import { ChevronUp, Copy, Folder, FolderAdd } from 'akar-icons-svelte';
	import { afterUpdate, onDestroy } from 'svelte';
	import Dead from './Dead.svelte';
	import * as dialog from '@tauri-apps/api/dialog';

	export let browsePath;
	export let entriesList = null;
	export let openEntry;
	export let open;
	export let size;
	export let background = false;
	export let fileSelect = null;

	let browsing;
	let total_files = 0;
	let entries = {
		dirs: {},
		files: [],
		path: '',
	};

	function initBrowser() {
		browsing = undefined;
		total_files = 0;
		entries = {
			dirs: {},
			files: [],
			path: '',
		};

		for (let i = 0; i < $entriesList.length; i++) {
			const entry = $entriesList[i];
			const components = entry.path.split('/');
			let path = entries;
			let path_str = '';
			for (let k = 0; k < components.length-1; k++) {
				const component = components[k];
				path_str += (k > 0 ? '/' : '') + component;

				if (!(component in path.dirs))
					path.dirs[component] = {
						dirs: { '../': path },
						files: [],
						path: path_str
					};

				path = path.dirs[component];
			}

			const name = components[components.length-1];
			const [icon, type, extension] = getFileTypeInfo(name);

			path.files.push({
				path: entry.path,
				name,
				icon,
				type,
				extension,
				size: entry.size,
				typeTip: $_('file_types.' + type, { values: { extension } }),
			});

			total_files++;
		}

		browsing = createDirShortcuts(entries, '');
	}

	function createDirShortcuts(entries, path) {
		let i = 0; let _dir;
		for (let dir in entries.dirs) {
			if (dir === '../') continue;
			i++; _dir = dir;
			entries.dirs[dir] = createDirShortcuts(entries.dirs[dir], path + (path.length > 0 ? '/' : '') + dir);
			entries.dirs[dir].dirs['../'] = entries;
		}
		if (entries.files.length === 0 && i === 1 && path !== '') {
			let dir = entries.dirs[_dir];
			if (!("shortcut" in dir)) {
				dir.shortcut = path;
				dir.shortcut_dest = _dir;
			}
			return dir;
		}
		return entries;
	}

	let pathContainer;
	function scrollPath() {
		if (!pathContainer) return;
		pathContainer.scrollTo({
			left: pathContainer.scrollWidth,
			behavior: 'smooth'
		});
	}
	afterUpdate(scrollPath);

	function browseDirectory() {
		const path = this.dataset.path;
		browsing = browsing.dirs[path];
	}

	function goUp() {
		if ("../" in browsing.dirs) {
			browsing = browsing.dirs['../'];
		}
	}

	function selectFile() {
		dialog.open({ directory: true }).then(path => {
			if (path && path.length > 0) {
				fileSelect(path);
			}
		});
	}

	function copy() {
		let path = pathContainer.innerText.trim();
		if (path.length > 0) {
			if (PATH_SEPARATOR !== '/') {
				path = path.replace(/\//g, PATH_SEPARATOR);
			}
			navigator.clipboard.writeText(path);
		}
	}

	function countDirs(dirs) {
		let count = 0;
		for (let dir in dirs) {
			if (dir === '../') continue;
			count++;
		}
		return count;
	}

	if (browsePath) initBrowser();
	if (entriesList && entriesList.subscribe) {
		onDestroy(entriesList.subscribe(initBrowser));
	}
</script>

<main id="file-browser">
	<div id="nav">
		{#if browsePath}
			<div id="up" class="control" on:click={goUp}><ChevronUp size="1rem"/></div>
			<div id="path" class="select hide-scroll" bind:this={pathContainer}>
				{#if browsing}
					{browsePath.replace(/\\/g, '/')}{browsing.path.length > 0 ? '/' : ''}{browsing.path}
				{:else}
					{browsePath.replace(/\\/g, '/')}
				{/if}
			</div>
			<div id="copy" class="control" on:click={copy} use:tippyFollow={$_('copy_path')}><Copy size="1rem"/></div>
			<div id="open" class="control" on:click={open} use:tippyFollow={$_('open_addon_location')}><Folder size="1rem"/></div>
		{:else}
			<div id="path" class="select hide-scroll" bind:this={pathContainer}>
				<div style="text-align:center">{$_('file_browser')}</div>
			</div>
		{/if}
	</div>

	<div id="entries" class="hide-scroll" class:background={background}>
		{#if !browsePath && fileSelect}
			<div id="file-select" on:click={selectFile}>
				<div>
					<FolderAdd size="4rem" stroke-width="1"/>
					<div>{$_('file_browser_select')}</div>
				</div>
			</div>
		{:else if total_files === 0}
			<div id="no-files">
				<div>
					<Dead size="4rem"/>
					<div>{$_('no_files_found')}</div>
				</div>
			</div>
		{:else}
			<table>
				<tbody>
					{#each Object.entries(browsing.dirs) as [dir, entries]}
						{#if dir !== "../"}
							<tr on:click={browseDirectory} data-path={dir}>
								<td><img use:tippyFollow={$_('file_types.folder')} src="/img/silkicons/folder.png" alt=""/></td>
								<td colspan="3">
									{#if entries.shortcut}
										<span class="shortcut">{entries.shortcut}/</span><span>{entries.shortcut_dest}</span>
									{:else}
										<span>{dir}</span>
									{/if}
								</td>
							</tr>
						{/if}
					{/each}
					{#each browsing.files as entry}
						<tr on:click={() => openEntry(entry.path)}>
							<td><img class="icon" use:tippyFollow={entry.typeTip} src="/img/silkicons/{entry.icon}" alt=""/></td>
							<td><span>{entry.name}</span></td>
							<td><span>{entry.typeTip}</span></td>
							<td><span>{filesize(entry.size)}</span></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>

	<div id="ribbon">
		{total_files === 1 ? $_('items_one') : $_('items_num', { values: { n: total_files } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{$_('items_shown', { values: { n: browsing.files.length + countDirs(browsing.dirs) } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{filesize(size ?? 0)}
	</div>
</main>

<style>
	#entries > table th, #entries > table td, #entries > table td > span, #entries > table td > img {
		vertical-align: middle;
	}
	#entries > table tr {
		cursor: pointer;
	}

	#entries > table {
		border-collapse: collapse;
		width: 100%;
		font-size: .9em;
	}
	#entries > table td {
		padding: .6rem;
		vertical-align: top !important;
	}
	#entries > table td:nth-child(3) {
		text-align: right;
	}
	#entries > table td:nth-child(4) {
		text-align: center;
	}
	#entries > table td:nth-child(1), #entries > table td:nth-child(4) {
		width: 1px;
		white-space: nowrap;
	}
	#entries > table td:nth-child(2) {
		padding-left: 0 !important;
		word-break: break-all;
	}
	#entries > table tr {
		transition: background-color .1s;
	}
	#entries > table tr:hover {
		background-color: #212121;
	}
	#entries td:first-child img {
		width: 16px;
		height: 16px;
	}

	#file-browser {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	#entries {
		flex: 1;
		height: 0;
	}
	#entries.background {
		background-color: #292929;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
	}
	#entries .shortcut {
		opacity: .5;
	}
	#nav {
		background-color: #0a0a0a;
		font-size: .8em;
		display: flex;
	}
	#nav .control {
		padding: .6rem;
		cursor: pointer;
		position: relative;
		box-sizing: content-box;
		width: 1rem;
	}
	#nav .control:not(:last-child) {
		padding-right: 0;
	}
	#nav .control :global(.icon) {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		margin: auto;
	}
	#path {
		padding: .6rem;
		padding-left: 0;
		padding-right: 0;
		flex: 1;
		white-space: nowrap;
		width: 0;
		-ms-scroll-translation: vertical-to-horizontal;
		-webkit-scroll-translation: vertical-to-horizontal;
		-moz-scroll-translation: vertical-to-horizontal;
		scroll-translation: vertical-to-horizontal;
	}
	#ribbon {
		position: relative;
		font-size: .8em;
		text-align: center;
		padding: .6rem;
		background-color: #0a0a0a;
		transition: background-color .25s;
		display: grid;
		grid-template-rows: 1fr;
		grid-template-columns: 1fr;
	}

	#file-select, #no-files {
		display: flex;
		width: 100%;
		height: 100%;
		align-items: center;
		justify-content: center;
		padding: 1rem;
		opacity: .25;
	}
	#file-select {
		cursor: pointer;
		transition: opacity .25s;
	}
	#file-select:hover {
		opacity: 1;
	}
	#file-select > div, #no-files > div {
		text-align: center;
	}
	#file-select > div > div, #no-files > div > div {
		font-size: 1.3em;
	}
	#file-select > div > :global(.icon), #no-files > div > :global(.dead) {
		margin-bottom: 1rem;
	}
</style>

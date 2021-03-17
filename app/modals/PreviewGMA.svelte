<script>
	import Addons from '../addons.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow } from '../tippy.js';
	import { writable } from 'svelte/store';
	import Dead from '../../public/img/dead.svg';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';
	import SteamID from 'steamid';
	import { ChevronUp, Folder, LinkOut } from 'akar-icons-svelte';
	import { invoke, promisified } from 'tauri/api/tauri';
	import Timestamp from '../components/Timestamp.svelte';
	import { afterUpdate, onDestroy } from 'svelte';
	import Transaction from '../transactions.js';

	export let path;
	export let addon;

	function getFileIcon(extension) {
		switch(extension) {
			case 'lua':
				return 'script_code.png';

			case 'mp3':
			case 'ogg':
			case 'wav':
				return 'sound.png';

			case 'png':
			case 'jpg':
			case 'jpeg':
				return 'photo.png';

			case 'bsp':
			case 'nav':
			case 'ain':
			case 'fgd':
				return 'map.png';

			case 'pcf':
				return 'wand.png';
			
			case 'vcd':
				return 'comments.png';

			case 'ttf':
				return 'font.png';

			case 'txt':
				return 'page_white_text.png';

			case 'properties':
				return 'page_white_wrench.png';

			case 'vmt':
			case 'vtf':
				return 'picture_link.png';

			case 'mdl':
			case 'vtx':
			case 'phy':
			case 'ani':
			case 'vvd':
				return 'bricks.png';

			default:
				return 'page_white.png';
		}
		// TODO remove unused
	}

	function getFileType(extension) {
		switch(extension) {
			case 'mp3':
			case 'ogg':
			case 'wav':
				return 'audio';

			case 'png':
			case 'jpg':
			case 'jpeg':
				return 'image';

			case 'vtf':
			case 'vmt':
			case 'map':
			case 'ain':
			case 'nav':
			case 'ttf':
			case 'vcd':
			case 'fgd':
			case 'pcf':
			case 'lua':
			case 'mdl':
			case 'vtx':
			case 'phy':
			case 'ani':
			case 'vvd':
			case 'txt':
			case 'properties':
				return extension;

			default:
				return 'unknown';
		}
	}

	const RE_FILE_EXTENSION = /^.*(?:\.(.*?))$/;
	function getFileTypeInfo(path) {
		const extension = path.match(RE_FILE_EXTENSION)?.[1].toLowerCase();
		return [getFileIcon(extension), getFileType(extension), extension];
	}

	let entries;
	let browsing = writable({});
	let total_files = 0;

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

	let metadata = Addons.previewGMA(path, addon.id).then(data => {
		let [metadata, ws_metadata] = data;
		
		entries = {
			dirs: {},
			files: [],
			path: '',
		};

		for (let i = 0; i < metadata.entries.length; i++) {
			const entry = metadata.entries[i];
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
			});

			total_files++;
		}

		$browsing = createDirShortcuts(entries, '');

		return data;
	});

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
		$browsing = $browsing.dirs[path];
	}

	function goUp() {
		if ("../" in $browsing.dirs) {
			$browsing = $browsing.dirs['../'];
		}
	}

	function open() {
		invoke({
			cmd: 'openFolder',
			path
		});
	}

	let openGMAEntryRequest = writable(false);
	let extractGMARequest = writable(null);
	function openGMAEntry() {
		this.classList.add('extracting');

		promisified({
			cmd: 'openGmaPreviewEntry',
			entry_path: this.dataset.path,
		})
			.then(transactionId => new Transaction(transactionId))
			.then(transaction => {
				$openGMAEntryRequest = true;
				transaction.listen(event => {
					if (event.finished || event.cancelled) {
						$openGMAEntryRequest = false;
						this.classList.remove('extracting');
					}
				});
				onDestroy(() => transaction.cancel()); // TODO check cancel doesn't fire twice
			}); // TODO handle error
	}

	// TODO use Loading component for loading svg
</script>

<div id="gma-preview" class="modal" class:loaded={!!entries}>
	{#await metadata}
		<img src="/img/loading.svg" alt="Loading"/>
	{:then [metadata, ws_metadata]}
		<div id="content">
			<div id="addon" class="hide-scroll">
				<div><WorkshopAddon {...addon} isPreviewing={true}/></div>
				<div id="tags">
					{#each addon.tags as tag}
						<div class="tag">{tag}</div>
					{/each}
				</div>
				<table id="addon-info">
					<tbody>
						<tr>
							<th>{$_('size')}</th>
							<td>{filesize(metadata.size)}</td>
						</tr>
						{#if metadata.type}
							<tr>
								<th>{$_('addon_type')}</th>
								<td>{metadata.type}</td>
							</tr>
						{/if}
						{#if ws_metadata}
							<tr>
								<th>{$_('author')}</th>
								<td>
									{#if ws_metadata.owner}
										<a target="_blank" href="https://steamcommunity.com/profiles/{ws_metadata.steamid64}" style="text-decoration:none">
											<img id="avatar" src="data:image/png;base64,{ws_metadata.owner.avatar}"/>
											<span>{ws_metadata.owner.name}</span>
										</a>
									{:else}
										<a target="_blank" class="color" href="https://steamcommunity.com/profiles/{ws_metadata.steamid64}">
											{new SteamID(ws_metadata.steamid64).getSteam2RenderedID(true)}
										</a>
									{/if}
								</td>
							</tr>
							<tr>
								<th>{$_('created')}</th>
								<td><Timestamp unix={ws_metadata.timeCreated}/></td>
							</tr>
							{#if ws_metadata.timeUpdated && ws_metadata.timeUpdated != ws_metadata.timeCreated}
								<tr>
									<th>{$_('updated')}</th>
									<td><Timestamp unix={ws_metadata.timeUpdated}/></td>
								</tr>
							{/if}
						{/if}
					</tbody>
				</table>
				{#if ws_metadata}
					<div id="ws-link"><a class="color" href="https://steamcommunity.com/sharedfiles/filedetails/?id={ws_metadata.id}" target="_blank">Steam Workshop<LinkOut size=".8rem"/></a></div>
					{#if ws_metadata.description}
						<p id="description" class="select">{ws_metadata.description}</p>
					{/if}
				{/if}
			</div>

			<div id="browser">
				<div id="nav">
					<div id="up" class="control" on:click={goUp}><ChevronUp size="1rem"/></div>
					<div id="path" class="select hide-scroll" bind:this={pathContainer}>
						{metadata.path}{$browsing.path.length > 0 ? '/' : ''}{$browsing.path}
					</div>
					<div id="open" class="control" on:click={open} use:tippyFollow={$_('open_addon_location')}><Folder size="1rem"/></div>
				</div>

				<div id="entries" class="hide-scroll">
					<table>
						<tbody>
							{#each Object.entries($browsing.dirs) as [dir, entries]}
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
							{#each $browsing.files as entry}
								<tr on:click={openGMAEntry} data-path={entry.path}>
									<td>
										<img class="loading" width="16px" height="16px" src="/img/loading.svg"/>
										<img class="icon" use:tippyFollow={$_('file_types.' + entry.type, { values: { extension: entry.extension } })} src="/img/silkicons/{entry.icon}" alt=""/>
									</td>
									<td><span>{entry.name}</span></td>
									<td><span>{$_('file_types.' + entry.type, { values: { extension: entry.extension } })}</span></td>
									<td><span>{filesize(entry.size)}</span></td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div id="ribbon" class:extracting={$openGMAEntryRequest || $extractGMARequest}>
					{#if $openGMAEntryRequest}
						<img width="1rem" src="/img/loading.svg" alt="Loading"/><span>&nbsp;Extracting...</span>
					{:else if $extractGMARequest}
						<!-- TODO show extraction progress -->
					{:else}
						{total_files === 1 ? $_('items_one') : $_('items_num', { values: { n: total_files } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{$_('items_shown', { values: { n: $browsing.files.length + Object.keys($browsing.dirs).length } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{filesize(metadata.size)}
					{/if}
				</div>
			</div>
		</div>
	{:catch error}
		<div id="error"><Dead/><br>{error}</div>
	{/await}
</div>

<style>
	#gma-preview {
		position: relative;
		max-width: 100%;
   		max-height: 100%;
		width: 1000px;
		height: 700px;
		pointer-events: none;
	}
	#gma-preview.loaded {
		pointer-events: initial !important;
	}
	#gma-preview > * {
		position: absolute;
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
	}
	@media (max-width: 1000px), (max-height: 700px) {
		#gma-preview {
			width: 100%;
			height: 100%;
		}
	}
	#gma-preview > #content {
		display: flex;
		background-color: #131313;
		height: 100%;
		box-shadow: rgba(0, 0, 0, .24) 0px 3px 8px;
		animation: modal .25s;
	}

	#addon {
		width: 17rem;
		padding: 1.5rem;
		box-shadow: 0 0 10px 5px rgba(0, 0, 0, .25);
		background-color: #212121;
		z-index: 2;
	}
	#addon :global(#workshop-addon) {
		margin-bottom: 1.2rem;
	}
	#addon :global(#workshop-addon #card) {
		padding: 0;
	}

	#addon-info {
		text-align: left;
		border-spacing: 1rem;
	}

	#addon #tags {
		margin-bottom: .6rem;
		line-height: 1.7rem;
		margin-top: -.51rem;
	}
	#addon #tags .tag {
		position: relative;
		color: black;
		padding: 2px;
		padding-right: 13px;
		padding-left: 5px;
		font-size: 11px;
		z-index: 1;
		display: inline-block;
		line-height: initial;
		text-transform: lowercase;
	}
	#addon #tags .tag::after {
		content: '';
		position: absolute;
		top: 0;
		right: 0;
		width: 0;
		height: 0;
		border-style: solid;
		border-width: 9px 0 9px 9px;
		border-color: transparent transparent transparent #fff;
		-webkit-transform: rotate(360deg);
	}
	#addon #tags .tag::before {
		content: '';
		position: absolute;
		top: 0;
		right: 9px;
		width: calc(100% - 9px);
		height: 100%;
		background-color: #fff;
		z-index: -1;
	}
	#addon #tags .tag:not(:last-child) {
		margin-right: .4rem;
	}
	#entries > table th, #entries > table td, #entries > table td > span, #entries > table td > img {
		vertical-align: middle;
	}
	#entries > table tr {
		cursor: pointer;
	}
	#entries > table :global(tr:not(.extracting) .loading),
	#entries > table :global(tr.extracting .icon) {
		display: none;
	}

	#entries > table {
		border-collapse: collapse;
		width: 100%;
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
	#addon-info {
		border-spacing: .5rem;
		margin: -.5rem -.5rem -.5rem -.5rem;
	}

	#addon #description {
		margin: 0;
		margin-top: .8rem;
    	white-space: pre-line;
	}

	#addon #avatar, #addon #avatar + span {
		vertical-align: middle;
		width: 1.5rem;
		border-radius: 50%;
	}
	#addon #avatar {
		margin-right: .2rem;
	}

	#gma-preview > #error {
		line-height: 1.8rem;
    	text-align: center;
		width: max-content;
		height: max-content;
	}

	#browser {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	#entries {
		flex: 1;
		height: 0;
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
		padding: .6rem;
		background-color: #0a0a0a;
		font-size: .8em;
		text-align: center;
		transition: background-color .25s;
	}
	#ribbon.extracting {
		background-color: #0058a2;
	}
	#ribbon.extracting > img {
		width: .8rem;
	}
	#ribbon > * {
    	vertical-align: middle;
	}

	#ws-link {
		margin-top: 1rem;
		margin-bottom: 1rem;
		text-align: center;
	}
	#ws-link :global(.icon) {
		margin-left: .2rem;
	}
</style>
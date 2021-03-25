<script>
	import { Addons, trimPath, getFileTypeInfo } from '../addons.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow, tippy } from '../tippy.js';
	import { writable } from 'svelte/store';
	import Dead from '../../public/img/dead.svg';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';
	import SteamID from 'steamid';
	import { ChevronUp, Folder, LinkOut, Download, FolderAdd } from 'akar-icons-svelte';
	import { invoke, promisified } from 'tauri/api/tauri';
	import Timestamp from '../components/Timestamp.svelte';
	import { afterUpdate, onDestroy } from 'svelte';
	import { Transaction } from '../transactions.js';
	import Loading from '../components/Loading.svelte';

	// TODO clean up dumb workshop promise logic

	export let path;
	export let dead;
	export let workshop = dead ? Promise.reject() : new Promise();

	let metadata;
	let size = workshop?.size != null ? filesize(workshop.size) : null;

	let entries;
	let browsing;
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

	let owner = new Promise(() => {});

	const loading = Addons.previewGMA(path, workshop.id).then(gma => {
		owner = Addons.getWorkshopMetadata(workshop.id);

		entries = {
			dirs: {},
			files: [],
			path: '',
		};

		for (let i = 0; i < gma.entries.length; i++) {
			const entry = gma.entries[i];
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

		size = filesize(gma.size);

		browsing = createDirShortcuts(entries, '');
		metadata = gma;
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
		browsing = browsing.dirs[path];
	}

	function goUp() {
		if ("../" in browsing.dirs) {
			browsing = browsing.dirs['../'];
		}
	}

	function open() {
		invoke({
			cmd: 'openFolder',
			path
		});
	}

	function openGMAEntry() {
		promisified({
			cmd: 'openGmaPreviewEntry',
			entry_path: this.dataset.path,
		})
			.then(transactionId => new Transaction(transactionId, transaction => {
				return $_('extracting_progress', { values: {
					pct: transaction.progress,
					data: filesize((transaction.progress / 100) * metadata.size),
					dataTotal: size
				}});
			}));
	}

	const chooseDestination = writable(false);
	let extractModal;
	function extract() {
		$chooseDestination = true;
	}
	function cancelExtract(e) {
		if (e.target !== extractModal) return;
		$chooseDestination = false;
	}

	const extractPath = writable([null, null, AppSettings.create_folder_on_extract]);
	let extractPathInput;

	function computeExtractPath(click) {
		if (!click) {
			if (extractPathInput.value.length !== 0) return;
			if ($extractPath[0]) return;
		}

		const dest = click ? this.dataset.dest : null;
		switch(this.dataset.dest) {
			case 'tmp':
				$extractPath = [dest, trimPath(AppData.tmp_dir) + PATH_SEPARATOR + 'gmpublisher', AppSettings.create_folder_on_extract];
				break;

			case 'addons':
				$extractPath = [dest, trimPath(AppData.gmod) + PATH_SEPARATOR + 'garrysmod' + PATH_SEPARATOR + 'addons', AppSettings.create_folder_on_extract];
				break;

			case 'downloads':
				$extractPath = [dest, trimPath(AppData.downloads_dir), AppSettings.create_folder_on_extract];
				break;
			
			default:
				$extractPath = [null, null, AppSettings.create_folder_on_extract];
		}

		extractPathInput.value = '';
	}
	function extractDestHover() {
		if (this === extractPathInput) {
			if (this.value.length === 0) {
				$extractPath = [null, null, AppSettings.create_folder_on_extract];
			} else {
				$extractPath = ['browse', trimPath(this.value), AppSettings.create_folder_on_extract];
			}
		} else {
			computeExtractPath.call(this, false);
		}
	}
	function extractDestInputted() {
		if (this.value === "" || ($extractPath[0] !== null && $extractPath[0] !== 'browse')) return;
		$extractPath = ['browse', trimPath(this.value), AppSettings.create_folder_on_extract];
		this.value = '';
	}
	function extractDestFocused() {
		if (!!$extractPath[0]) {
			this.value = $extractPath[1];
		}
	}
	function extractDestLostFocus() {
		if (this.value.length > 0 && !!$extractPath[1]) {
			this.value = '';
		}
	}
	function extractDestHoverLeave() {
		if (extractPathInput.value.length !== 0) return;
		if ($extractPath[0] === null) {
			$extractPath = [null, null, AppSettings.create_folder_on_extract];
		}
	}
	function updateExtractDest() {
		if (this.dataset.dest === $extractPath[0]) {
			$extractPath = [null, null, AppSettings.create_folder_on_extract];
		} else {
			computeExtractPath.call(this, true);
		}
	}
	function extractDestBrowse() {
		if ('browse' === $extractPath[0]) {
			$extractPath = [null, null, AppSettings.create_folder_on_extract];
		} else {
			promisified({

				cmd: 'promptPathDialog',
				directory: true,
				multiple: false,
				save: false,
				filter: '',
				defaultPath: AppSettings.destinations[0],

			}).then(path => {
				if (!!path)
					$extractPath = ['browse', trimPath(path[0]), AppSettings.create_folder_on_extract]
			});
		}
	}
	function extractableHistoryPath() {
		$extractPath = ['browse', trimPath(this.textContent), AppSettings.create_folder_on_extract];
	}
	function createFolderUpdated() {
		AppSettings.create_folder_on_extract = this.checked;
		$extractPath = [$extractPath[0], $extractPath[1], this.checked];
	}
	function doExtract() {
		const dest = $extractPath[0];

		promisified({

			cmd: 'extractGma',
			named_dir: AppSettings.create_folder_on_extract,
			path: dest === 'browse' ? $extractPath[1] : null,
			tmp: dest === 'tmp',
			addons: dest === 'addons',
			downloads: dest === 'downloads',

		}).then(id => new Transaction(id, transaction => {
			return $_('extracting_progress', { values: {
				pct: transaction.progress,
				data: filesize((transaction.progress / 100) * metadata.size),
				dataTotal: size
			}});
		}));

		$chooseDestination = false;
	}

	// TODO use Loading component for loading svg
</script>

<div id="gma-preview" class="modal" class:loaded={!!entries}>
	<div id="content">
		<div id="sidebar">
			<div class="extract-btn" on:click={extract}>{$_('extract')}</div>
			<div id="addon" class="hide-scroll">
				{#if workshop}
					<div><WorkshopAddon {...workshop} isPreviewing={true}/></div>
					<div id="tags">
						{#if metadata && metadata.type && workshop.tags.indexOf(metadata.type.toLowerCase()) !== -1}
							<div class="tag {metadata.type.toLowerCase()}">{metadata.type}</div>
						{/if}
						{#each workshop.tags as tag}
							<div class="tag {tag.toLowerCase()}">{tag}</div>
						{/each}
					</div>
				{:else}
					
				{/if}
				<table id="addon-info">
					<tbody>
						{#if size}
							<tr>
								<th>{$_('size')}</th>
								<td>{size}</td>
							</tr>
						{/if}
						{#if !dead}
							<tr>
								<th>{$_('author')}</th>
								<td>
									{#await owner}
										<Loading/>
									{:then owner}
										<a target="_blank" href="https://steamcommunity.com/profiles/{owner.steamid64}" style="text-decoration:none">
											<img id="avatar" src="data:image/png;base64,{owner.owner.avatar}"/>
											<span>{owner.owner.name}</span>
										</a>
									{:catch}
										<a target="_blank" class="color" href="https://steamcommunity.com/profiles/{workshop.steamid64}">
											{new SteamID(workshop.steamid64).getSteam2RenderedID(true)}
										</a>
									{/await}
								</td>
							</tr>
						{/if}
						{#if workshop.timeCreated}
							<tr>
								<th>{$_('created')}</th>
								<td><Timestamp unix={workshop.timeCreated}/></td>
							</tr>
						{/if}
						{#if workshop.timeUpdated && workshop.timeUpdated != workshop.timeCreated}
							<tr>
								<th>{$_('updated')}</th>
								<td><Timestamp unix={workshop.timeUpdated}/></td>
							</tr>
						{/if}
					</tbody>
				</table>
				{#if workshop}
					<div id="ws-link"><a class="color" href="https://steamcommunity.com/sharedfiles/filedetails/?id={workshop.id}" target="_blank">Steam Workshop<LinkOut size=".8rem"/></a></div>
					{#if workshop.description}
						<p id="description" class="select">{workshop.description}</p>
					{/if}
				{/if}
			</div>
		</div>

		<div id="browser">
			<div id="nav">
				<div id="up" class="control" on:click={goUp}><ChevronUp size="1rem"/></div>
				<div id="path" class="select hide-scroll" bind:this={pathContainer}>
					{#if metadata}
						{metadata.path}{browsing.path.length > 0 ? '/' : ''}{browsing.path}
					{:else if workshop && !!workshop.localFile}
						{workshop.localFile}{browsing.path.length > 0 ? '/' : ''}{browsing.path}
					{:else}
						{browsing.path.length > 0 ? '/' : ''}{browsing.path}
					{/if}
				</div>
				<div id="open" class="control" on:click={open} use:tippyFollow={$_('open_addon_location')}><Folder size="1rem"/></div>
			</div>

			<div id="entries" class="hide-scroll">
				{#await loading}
					<Loading/>
				{:then}
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
								<tr on:click={openGMAEntry} data-path={entry.path}>
									<td><img class="icon" use:tippyFollow={$_('file_types.' + entry.type, { values: { extension: entry.extension } })} src="/img/silkicons/{entry.icon}" alt=""/></td>
									<td><span>{entry.name}</span></td>
									<td><span>{$_('file_types.' + entry.type, { values: { extension: entry.extension } })}</span></td>
									<td><span>{filesize(entry.size)}</span></td>
								</tr>
							{/each}
						</tbody>
					</table>
				{:catch error}
					<div id="error"><Dead/><br>{error}</div>
				{/await}
			</div>

			<div id="ribbon">
				{#await loading}
					<Loading/>
				{:then}
					{total_files === 1 ? $_('items_one') : $_('items_num', { values: { n: total_files } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{$_('items_shown', { values: { n: browsing.files.length + Object.keys(browsing.dirs).length } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{size}
				{:catch}
					{$_('items_num', { values: { n: 0 } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{$_('items_shown', { values: { n: 0 } })}{#if size}&nbsp;&nbsp;∣&nbsp;&nbsp;{size}{/if}
				{/await}
			</div>
		</div>
	</div>

	{#if metadata}
		<div id="destination" class:active={$chooseDestination} on:click={cancelExtract} bind:this={extractModal}><div>
			<h1>{$_('extract_where_to')}</h1>
			<h4>{$_('extract_overwrite_warning')}</h4>

			<input type="text" name="path" on:input={extractDestHover} on:focus={extractDestFocused} on:blur={extractDestLostFocus} on:change={extractDestInputted} bind:this={extractPathInput} placeholder={$extractPath[0] ? ($extractPath[1] + ($extractPath[2] ? (PATH_SEPARATOR + metadata.extracted_name) : '')) : metadata.extracted_name}/>

			{#if $extractPath[0] === 'browse'}
				<div id="checkbox">
					<label>
						<input type="checkbox" id="named" name="named" on:change={createFolderUpdated} checked={AppSettings.create_folder_on_extract}>
						<span>{$_('create_folder')}</span>
					</label>
				</div>
			{/if}

			<div id="destinations">
				<div class="destination" class:active={$extractPath[0] === 'browse'} on:hover={extractDestHover} on:click={extractDestBrowse} data-dest="browse">
					<Folder/>
					<div>{$_('browse')}</div>
				</div>

				{#if !!AppData.tmp_dir}
					<div class="destination" class:active={$extractPath[0] === 'tmp'} use:tippy={$_('extract_open_tip')} on:mouseover={extractDestHover} on:click={updateExtractDest} on:mouseleave={extractDestHoverLeave} data-dest="tmp">
						<FolderAdd/>
						<div>{$_('open')}</div>
					</div>
				{/if}

				{#if !!AppData.gmod}
					<div class="destination" class:active={$extractPath[0] === 'addons'} on:mouseover={extractDestHover} on:mouseleave={extractDestHoverLeave} on:click={updateExtractDest} data-dest="addons">
						<img src="/img/gmod.svg"/>
						<div>{$_('addons_folder')}</div>
					</div>
				{/if}

				{#if !!AppData.downloads_dir}
					<div class="destination" class:active={$extractPath[0] === 'downloads'} on:mouseover={extractDestHover} on:mouseleave={extractDestHoverLeave} on:click={updateExtractDest} data-dest="downloads">
						<Download/>
						<div>{$_('downloads_folder')}</div>
					</div>
				{/if}
			</div>

			{#if AppSettings.destinations.length > 0}
				<div id="history" class="hide-scroll">
					{#each AppSettings.destinations as path}
						<div on:click={extractableHistoryPath} class:active={$extractPath[0] === 'browse' && $extractPath[1] === path}>{path}</div>
					{/each}
				</div>
			{/if}

			<div class="extract-btn" on:click={doExtract} class:disabled={!$extractPath[0]}>{$_('extract')}</div>
		</div></div>
	{/if}
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
		flex: 1;
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
	#entries > table th, #entries > table td, #entries > table td > span, #entries > table td > img {
		vertical-align: middle;
	}
	#entries > table tr {
		cursor: pointer;
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
		color: #888;
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

	#ws-link {
		margin-top: 1rem;
		margin-bottom: 1rem;
		text-align: center;
	}
	#ws-link :global(.icon) {
		margin-left: .2rem;
	}

	#gma-preview #sidebar {
		display: flex;
		flex-direction: column;
		overflow: hidden;
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

	#destination {
		pointer-events: none;

		transition: backdrop-filter .25s, background-color .25s;
		background-color: rgba(0,0,0,0);

		z-index: 4;
		position: relative;
		width: 100%;
		height: 100%;
	}
	#destination.active {
		pointer-events: all;

		backdrop-filter: grayscale(1) blur(1px);
		background-color: rgba(0,0,0,.4);
	}
	#destination.active > div {
		transform: scale(1, 1);
	}
	#destination > div {
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
	#destination > div > h1 {
		margin-top: 0;
		margin-bottom: 0;
	}
	#destination > div > h4 {
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
	#destination input[type='text'] {
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
	#destination input[type='text']:focus {
		outline: none;
	}
	#destination input[type='text']:placeholder-shown {
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
	#destination #checkbox {
		margin-bottom: 1rem;
		display: inline-flex;
		justify-content: center;
		align-items: center;
	}
	#destination #checkbox label {
		cursor: pointer;
	}
	#destination #checkbox label > * {
		vertical-align: middle;
	}
	#destination .extract-btn {
		margin-top: 1rem;
	}
</style>
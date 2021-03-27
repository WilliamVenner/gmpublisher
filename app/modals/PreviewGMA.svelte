<script>
	import { Addons, trimPath, getFileTypeInfo } from '../addons.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow, tippy } from '../tippy.js';
	import { writable } from 'svelte/store';
	import Dead from '../../public/img/dead.svg';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';
	import SteamID from 'steamid';
	import { ChevronUp, Folder, LinkOut, Download, FolderAdd, Tag } from 'akar-icons-svelte';
	import { invoke, promisified } from 'tauri/api/tauri';
	import Timestamp from '../components/Timestamp.svelte';
	import { afterUpdate, onDestroy } from 'svelte';
	import { Transaction } from '../transactions.js';
	import Loading from '../components/Loading.svelte';
	import Destination from '../modals/Destination.svelte';

	export let id = null;
	export let gmaData = null;
	export let workshopData = null;

	let gma = gmaData;

	let workshop = workshopData;
	let ws_id = id;
	if (!ws_id) {
		if (workshop && workshop.id)
			ws_id = workshop.id;
		else
			ws_id = gma.id;
	}

	let path;
	if (gma && gma.path) {
		path = gma.path;
	} else if (workshop && workshop.localFile) {
		path = workshop.localFile;
	} else {
		console.error('Tried to preview a GMA with no resolvable path!');
		console.error(gma, workshop);
	}

	let size = gma?.size ?? workshop?.size ?? null;
	if (size) size = filesize(size);

	let dead = workshop?.dead ?? false;

	let gmaLoading = gma ? Promise.resolve(gma) : new Promise(() => {});
	let workshopLoading = workshop ? Promise.resolve(workshop) : new Promise(() => {});
	let workshopUploaderLoading = workshop?.owner ? Promise.resolve(workshop.owner) : new Promise(() => {});

	const deadWorkshopAddon = Object.assign({ id: ws_id, title: gma ? (gma.name ?? gma.extracted_name) : ws_id }, window.__WS_DEAD__);
	
	let subscriptions = [];
	onDestroy(() => subscriptions.forEach(subscription => subscription()));

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

	function initBrowser() {
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

		browsing = createDirShortcuts(entries, '');
	}

	function getWorkshopMetadata() {
		workshopLoading = Addons.getWorkshopMetadata(ws_id).then(_workshop => {
			workshop = _workshop;
			dead = workshop.dead;

			workshopUploaderLoading = dead ? Promise.reject() : new Promise((resolve, reject) => Addons.getWorkshopUploader(ws_id).then(owner => {
				if (workshop.dead) return reject();
				workshop.owner = owner;
				resolve(owner);
			}));
		});
	}

	if (!gma || !gma.entries) {

		gmaLoading = Addons.previewGMA(path, ws_id).then(_gma => {
			gma = _gma;
			
			workshopLoading = (!workshop || !workshop.owner) && ws_id ? getWorkshopMetadata() : Promise.reject();

			size = filesize(gma.size);
			initBrowser();
		});

	} else {

		initBrowser();
		workshopLoading = (!workshop || !workshop.owner) && ws_id ? getWorkshopMetadata() : Promise.reject();

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
					data: filesize((transaction.progress / 100) * gma.size),
					dataTotal: size
				}});
			}));
	}

	let chooseDestination = false;
	function extract() {
		chooseDestination = true;
	}
	function cancelExtract(e) {
		chooseDestination = false;
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
	function doExtract(extractPath) {
		chooseDestination = false;
		const dest = extractPath[0];

		promisified({

			cmd: 'extractGma',
			named_dir: AppSettings.create_folder_on_extract,
			path: dest === 'browse' ? extractPath[1] : null,
			tmp: dest === 'tmp',
			addons: dest === 'addons',
			downloads: dest === 'downloads',

		}).then(id => new Transaction(id, transaction => {
			return $_('extracting_progress', { values: {
				pct: transaction.progress,
				data: filesize((transaction.progress / 100) * gma.size),
				dataTotal: size
			}});
		}));
	}
</script>

<div id="gma-preview" class="modal" class:loaded={!!(workshop || gma)}>
	{#if workshop || gma}
		<div id="content">
			<div id="sidebar">
				<div class="extract-btn" on:click={extract}>{$_('extract')}</div>
				<div id="addon" class="hide-scroll">
					{#if workshop && !dead}
						<div><WorkshopAddon {...workshop} dead={dead} isPreviewing={true}/></div>
						<div id="tags">
							{#if workshop.tags}
								{#if gma && gma.type && workshop.tags.indexOf(gma.type.toLowerCase()) !== -1}
									<div class="tag {gma.type.toLowerCase()}">{gma.type}</div>
								{/if}
								{#each workshop.tags as tag}
									<div class="tag {tag.toLowerCase()}">{tag}</div>
								{/each}
							{/if}
							{#if gma && gma.tags}
								{#each gma.tags as tag}
									{#if !workshop.tags || workshop.tags.indexOf(tag) !== -1}
										<div class="tag {tag.toLowerCase()}">{tag}</div>
									{/if}
								{/each}
							{/if}
						</div>
					{:else if gma}
						{#if !workshop || dead}
							<div><WorkshopAddon {...deadWorkshopAddon} loading={!dead} isPreviewing={true}/></div>
						{/if}

						{#if gma.name}
							<div id="workshop-addon">{gma.name}</div>
						{:else if gma.extracted_name}
							<div id="workshop-addon">{gma.extracted_name}</div>
						{/if}
						{#if gma.tags}
							<div id="tags">
								{#if gma.type && gma.tags.indexOf(gma.type.toLowerCase()) !== -1}
									<div class="tag {gma.type.toLowerCase()}">{gma.type}</div>
								{/if}
								{#each gma.tags as tag}
									<div class="tag {tag.toLowerCase()}">{tag}</div>
								{/each}
							</div>
						{/if}
					{/if}
					<table id="addon-info">
						<tbody>
							{#if size}
								<tr>
									<th>{$_('size')}</th>
									<td>{size}</td>
								</tr>
							{/if}
							{#if workshop}
								{#if workshop.owner}
									<tr>
										<th>{$_('author')}</th>
										<td>
											<a target="_blank" href="https://steamcommunity.com/profiles/{workshop.owner.steamid64}" style="text-decoration:none">
												<img id="avatar" src="data:image/png;base64,{workshop.owner.avatar}"/>
												<span>{workshop.owner.name}</span>
											</a>
										</td>
									</tr>
								{:else if workshop.steamid64}
									<tr>
										<th>{$_('author')}</th>
										<td>
											<div id="author-loading">
												<a target="_blank" class="color" href="https://steamcommunity.com/profiles/{workshop.steamid64}">
													{new SteamID(workshop.steamid64).getSteam2RenderedID(true)}
												</a>{#await workshopUploaderLoading}&nbsp;<Loading inline="true"/>{:catch} {/await}
											</div>
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
							{/if}
						</tbody>
					</table>
					{#if ws_id}
						<div id="ws-link"><a class="color" href="https://steamcommunity.com/sharedfiles/filedetails/?id={ws_id}" target="_blank">Steam Workshop<LinkOut size=".8rem"/></a></div>
					{/if}
					{#if workshop && workshop.description}
						<p id="description" class="select">{workshop.description}</p>
					{/if}
				</div>
			</div>

			<div id="browser">
				<div id="nav">
					<div id="up" class="control" on:click={goUp}><ChevronUp size="1rem"/></div>
					<div id="path" class="select hide-scroll" bind:this={pathContainer}>
						{#if browsing}
							{#if gma}
								{gma.path}{browsing.path.length > 0 ? '/' : ''}{browsing.path}
							{:else if workshop && !!workshop.localFile}
								{workshop.localFile}{browsing.path.length > 0 ? '/' : ''}{browsing.path}
							{:else}
								{browsing.path.length > 0 ? '/' : ''}{browsing.path}
							{/if}
						{:else}
							{#if gma}
								{gma.path}
							{:else if workshop && !!workshop.localFile}
								{workshop.localFile}
							{:else}
								<Loading/>
							{/if}
						{/if}
					</div>
					<div id="open" class="control" on:click={open} use:tippyFollow={$_('open_addon_location')}><Folder size="1rem"/></div>
				</div>

				<div id="entries" class="hide-scroll">
					{#await gmaLoading}
						<Loading size="1.5rem"/>
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
					{#await gmaLoading}
						<Loading/>
					{:then}
						{total_files === 1 ? $_('items_one') : $_('items_num', { values: { n: total_files } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{$_('items_shown', { values: { n: browsing.files.length + Object.keys(browsing.dirs).length } })}&nbsp;&nbsp;∣&nbsp;&nbsp;{size}
					{:catch}
						<Dead size="1rem"/>
					{/await}
				</div>
			</div>
		</div>

		{#if gma}
			<Destination active={chooseDestination} cancel={cancelExtract} callback={doExtract} text={$_('extract')} gma={gma}/>
		{/if}
	{:else}
		<Loading size="2rem"/>
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

	:global(#addon > .loading:first-child) {
		margin-bottom: .8rem !important;
	}
	#workshop-addon {
		text-align: center;
	}

	#author-loading {
		display: flex;
	}
	#author-loading > a {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
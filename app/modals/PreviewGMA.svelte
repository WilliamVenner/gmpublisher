<script>
	import Addons from '../addons.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow } from '../tippy.js';
	import { writable } from 'svelte/store';
	import Dead from '../../public/img/dead.svg';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';
	import SteamID from 'steamid';

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

	const RE_FILE_EXTENSION = /^.*?(?:\.(.*?))?$/;
	function getFileTypeInfo(path) {
		const extension = path.match(RE_FILE_EXTENSION)?.[1].toLowerCase();
		return [getFileIcon(extension), getFileType(extension), extension];
	}

	let entries;
	let browsing = writable({});

	let metadata = Addons.previewGMA(path, addon.id).then(data => {
		let [metadata, ws_metadata] = data;

		entries = {
			dirs: {},
			files: []
		};

		for (let i = 0; i < metadata.entries.length; i++) {
			const entry = metadata.entries[i];
			const components = entry.path.split('/');
			let path = entries;
			for (let k = 0; k < components.length-1; k++) {
				const component = components[k];
				if (!(component in path.dirs))
					path.dirs[component] = {
						dirs: { '../': path },
						files: []
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
		}

		$browsing = entries;

		return data;
	});

	function browseDirectory() {
		const path = this.dataset.path;
		$browsing = $browsing.dirs[path];
	}

	function previewFile() {
		console.log(this);
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
							<th>Size</th>
							<td>{filesize(metadata.size)}</td>
						</tr>
						{#if metadata.type}
							<tr>
								<th>Type</th>
								<td>{metadata.type}</td>
							</tr>
						{/if}
						{#if ws_metadata}
							<tr>
								<th>Author</th>
								<td>
									<a target="_blank" href="https://steamcommunity.com/profiles/{ws_metadata.steamid64}" style="text-decoration:none">
										{#if ws_metadata.owner}
											<img id="avatar" src="data:image/png;base64,{ws_metadata.owner.avatar}"/>
											<span>{ws_metadata.owner.name}</span>
										{:else}
											{new SteamID(String(ws_metadata.steamid64)).getSteam2RenderedID(true)}
										{/if}
									</a>
								</td>
							</tr>
							<tr>
								<th>Created</th>
								<td></td>
							</tr>
							<tr>
								<th>Updated</th>
								<td></td>
							</tr>
						{/if}
					</tbody>
				</table>
				{#if ws_metadata}
					{#if ws_metadata.description}
						<p id="description" class="select">{ws_metadata.description}</p>
					{/if}
				{/if}
			</div>

			<table id="entries">
				<thead>
					<tr>
						<th></th>
						<th>{$_('name')}</th>
						{#if $browsing.files.length > 0}
							<th>{$_('size')}</th>
						{/if}
					</tr>
				</thead>
				<tbody>
					{#each Object.keys($browsing.dirs) as dir}
						<tr on:click={browseDirectory} data-path={dir}>
							<td><img use:tippyFollow={$_('file_types.folder')} src="/img/silkicons/folder.png" alt=""/></td>
							<td colspan="2"><span>{dir}</span></td>
						</tr>
					{/each}
					{#each $browsing.files as entry}
						<tr on:click={previewFile} data-path={entry.path}>
							<td><img use:tippyFollow={$_('file_types.' + entry.type, { values: { extension: entry.extension } })} src="/img/silkicons/{entry.icon}" alt=""/></td>
							<td><span>{entry.name}</span></td>
							<td><span>{filesize(entry.size)}</span></td>
						</tr>
					{/each}
				</tbody>
			</table>
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
		padding: 1rem;
		background-color: #1a1a1a;
		height: 100%;
	}

	#addon {
		width: 15rem;
		margin-right: 1rem;
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
	#entries th, #entries td, #entries td > span, #entries td > img {
		vertical-align: middle;
	}
	#entries tr {
		cursor: pointer;
	}

	#entries {
		border-collapse: collapse;
	}
	#entries td, #entries th {
		padding: .4rem;
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

	#gma-preview > #error {
		line-height: 1.8rem;
    	text-align: center;
	}
</style>
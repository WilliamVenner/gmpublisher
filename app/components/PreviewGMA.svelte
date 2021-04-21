<script>
	import { Steam, getFileTypeInfo } from '../steam.js';
	import { _ } from 'svelte-i18n';
	import filesize from 'filesize';
	import { tippyFollow } from '../tippy.js';
	import Dead from './Dead.svelte';
	import SteamID from 'steamid';
	import { ChevronUp, Folder, LinkOut } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import Timestamp from './Timestamp.svelte';
	import { afterUpdate, onDestroy } from 'svelte';
	import { Transaction } from '../transactions.js';
	import Loading from './Loading.svelte';
	import Modal from './Modal.svelte';
	import Addon from './Addon.svelte';
	import FileBrowser from './FileBrowser.svelte';
	import DestinationSelect from './DestinationSelect.svelte';

	export let active = false;
	export let promises;
	export let cancel;

	let subscriptions = [];
	onDestroy(() => subscriptions.forEach(subscription => subscription()));

	function openGMAEntry() {
		invoke('open_gma_entry', { path: this.dataset.path })
			.then(transactionId => new Transaction(transactionId, transaction => {
				return $_('extracting_progress', { values: {
					pct: transaction.progress,
					data: filesize((transaction.progress / 100) * gma.size),
					dataTotal: size
				}});
			}));
	}

	let destinationSelect = false;
	function extract() {
		destinationSelect = true;
	}
	function doExtract() {
		destinationSelect = false;
	}

	let addon;
	function updatePromises(promises) {
		addon = Promise.allSettled(promises).then(([workshop, gma]) => {
			console.log(workshop, gma);
			return [
				workshop.status === 'fulfilled' ? (!workshop.value.dead ? workshop.value : null) : null,
				gma.status === 'fulfilled' ? gma.value : null
			];
		});
	}
	updatePromises($promises);
	onDestroy(promises.subscribe(updatePromises));

	function open(path) {
		invoke('open_file_location', { path });
	}
	function openEntry(gma_path, entry_path) {
		invoke('extract_gma_entry', { gma_path, entry_path });
	}
</script>

<Modal id="gma-preview" {active} {cancel}>
	{#await addon}
		<Loading size="2rem"/>
	{:then [workshop, gma]}
		{#if !workshop && !gma}
			<Dead size="2rem"/>
		{:else}
			<div id="content">
				<div id="sidebar">
					<div class="extract-btn" on:click={extract}>{$_('extract')}</div>
					<div id="addon" class="hide-scroll">
						<div><Addon workshopData={$promises[0]} installedData={$promises[1]}/></div>
						{#if workshop}
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
								{#if gma && gma.size > 0}
									<tr>
										<th>{$_('size')}</th>
										<td>{filesize(gma.size)}</td>
									</tr>
								{:else if workshop && workshop.size > 0}
									<tr>
										<th>{$_('size')}</th>
										<td>{filesize(workshop.size)}</td>
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
												{#await Steam.getSteamUser(workshop.steamid64)}
													<div id="author-loading">
														<a target="_blank" class="color" href="https://steamcommunity.com/profiles/{workshop.steamid64}">
															{new SteamID(workshop.steamid64).getSteam2RenderedID(true)}
														</a>
													</div>
													&nbsp;<Loading inline="true"/>
												{:then owner}
													<a target="_blank" href="https://steamcommunity.com/profiles/{owner.steamid64}" style="text-decoration:none">
														<img id="avatar" src="data:image/png;base64,{owner.avatar}"/>
														<span>{owner.name}</span>
													</a>
												{:catch}
													<div id="author-loading">
														<a target="_blank" class="color" href="https://steamcommunity.com/profiles/{workshop.steamid64}">
															{new SteamID(workshop.steamid64).getSteam2RenderedID(true)}
														</a>
													</div>
													&nbsp;<Dead inline="true"/>
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
								{/if}
							</tbody>
						</table>
						{#if (gma && gma.id) || workshop}
							<div id="ws-link"><a class="color" href="https://steamcommunity.com/sharedfiles/filedetails/?id={gma?.id ?? workshop.id}" target="_blank">Steam Workshop<LinkOut size=".8rem"/></a></div>
						{/if}
						{#if workshop && workshop.description}
							<p id="description" class="select">{workshop.description}</p>
						{/if}
					</div>
				</div>

				{#if gma || workshop.localFile}
					{#await Steam.getGMAEntries(gma?.path ?? workshop.localFile)}
						<Loading size="2rem"/>
					{:then entriesList}
						<FileBrowser browsePath={gma?.path ?? workshop.localFile} {entriesList} {open} {openEntry} size={gma.size ?? workshop.size}/>
					{:catch gmaError}
						<Dead size="2rem"/>{$_(gmaError)}
					{/await}
				{:else}
					<Dead size="2rem"/>
				{/if}
			</div>

			<DestinationSelect active={destinationSelect} cancel={() => destinationSelect = false} callback={doExtract} text={$_('extract_where_to')} extractedName={gma?.name} />
		{/if}
	{/await}
</Modal>

<style>
	:global(#gma-preview) > div {
		max-width: 100%;
   		max-height: 100%;
		width: 1000px;
		height: 700px;
	}
	:global(#gma-preview) #content {
		display: flex;
		background-color: #131313;
		height: 100%;
		box-shadow: rgba(0, 0, 0, .24) 0px 3px 8px;
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

	:global(#gma-preview) > #error {
		line-height: 1.8rem;
		text-align: center;
		width: max-content;
		height: max-content;
	}

	#ws-link {
		margin-top: 1rem;
		margin-bottom: 1rem;
		text-align: center;
	}
	#ws-link :global(.icon) {
		margin-left: .2rem;
	}

	:global(#gma-preview) #sidebar {
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
